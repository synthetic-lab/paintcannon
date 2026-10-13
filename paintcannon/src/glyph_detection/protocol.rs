//! Rio Glyph Protocol queries. The input thread owns requests and replies;
//! waiting callers never read stdin or compete with keyboard/paste parsing.
use crossbeam_channel::{bounded, Receiver, Sender};
use std::io::{self, Write};
use std::time::{Duration, Instant};

// Safety bound only: a responsive terminal uses its measured RTT plus grace.
pub(crate) const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) struct Request {
    pub characters: Vec<char>,
    pub deadline: Instant,
    pub reply: Sender<Vec<Option<bool>>>,
}

struct Pending {
    request: Request,
    results: Vec<Option<bool>>,
    sent: Instant,
    heartbeat_received: bool,
}

pub(crate) struct Queries {
    incoming: Receiver<Request>,
    pending: Option<Pending>,
}

impl Queries {
    pub(crate) fn has_work(&self) -> bool {
        !self.incoming.is_empty() || self.pending.is_some()
    }

    pub(crate) fn new() -> (Sender<Request>, Self) {
        let (tx, incoming) = bounded(1);
        (
            tx,
            Self {
                incoming,
                pending: None,
            },
        )
    }

    pub(crate) fn poll(&mut self, enabled: bool, out: &mut impl Write, tmux: bool, now: Instant) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| !enabled || now >= p.request.deadline)
        {
            self.finish();
        }
        if self.pending.is_some() {
            return;
        }
        if let Ok(request) = self.incoming.try_recv() {
            let results = vec![None; request.characters.len()];
            if !enabled || now >= request.deadline {
                let _ = request.reply.try_send(results);
                return;
            }
            if write_queries(out, &request.characters, tmux).is_err() {
                let _ = request.reply.try_send(results);
                return;
            }
            self.pending = Some(Pending {
                request,
                results,
                sent: now,
                heartbeat_received: false,
            });
        }
    }

    pub(crate) fn heartbeat(&mut self, now: Instant) {
        if let Some(pending) = &mut self.pending {
            if pending.heartbeat_received {
                return;
            }
            pending.heartbeat_received = true;
            let rtt = now.saturating_duration_since(pending.sent);
            let grace = (rtt / 4).max(Duration::from_millis(50));
            pending.request.deadline = pending.request.deadline.min(now + grace);
        }
    }

    fn finish(&mut self) {
        if let Some(pending) = self.pending.take() {
            let _ = pending.request.reply.try_send(pending.results);
        }
    }

    pub(crate) fn reply(&mut self, body: &[u8]) {
        if let Some((cp, supported)) = parse_reply(body) {
            if let Some(pending) = &mut self.pending {
                if let Some(i) = pending.request.characters.iter().position(|&c| c == cp) {
                    pending.results[i].get_or_insert(supported);
                }
                if pending.results.iter().all(Option::is_some) {
                    self.finish();
                }
            }
        }
    }
}

fn write_queries(out: &mut impl Write, characters: &[char], tmux: bool) -> io::Result<()> {
    let mut query = String::new();
    for &cp in characters {
        use std::fmt::Write;
        let _ = write!(query, "\x1b_25a1;q;cp={:x}\x1b\\", cp as u32);
    }
    // The heartbeat must reach the same endpoint, not be answered by tmux.
    // Keep it in the same passthrough payload as the probes (see standalone test).
    query.push_str("\x1b[5n");
    if tmux {
        crate::terminal::encode_tmux_passthrough(out, query.as_bytes())?;
    } else {
        out.write_all(query.as_bytes())?;
    }
    out.flush()
}

fn parse_reply(body: &[u8]) -> Option<(char, bool)> {
    let body = std::str::from_utf8(body).ok()?.strip_prefix("25a1;q;")?;
    let (mut cp, mut status) = (None, None);
    for field in body.split(';') {
        let (key, value) = field.split_once('=')?;
        match key {
            "cp" if cp.is_none() => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
                cp = Some(char::from_u32(u32::from_str_radix(value, 16).ok()?)?);
            }
            "status" if status.is_none() => status = Some(value),
            "cp" | "status" => return None,
            _ => {}
        }
    }
    let status = status?;
    // Current named coverage plus the original v1 two-bit coverage response.
    let supported = match status {
        "0" | "" => false,
        "1" | "2" | "3" => true,
        _ => status
            .split(',')
            .any(|s| matches!(s, "system" | "glossary")),
    };
    Some((cp?, supported))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_bytes_match_rio_spec() {
        let mut out = Vec::new();
        write_queries(&mut out, &['\u{e0a0}'], false).unwrap();
        assert_eq!(out, b"\x1b_25a1;q;cp=e0a0\x1b\\\x1b[5n");
        out.clear();
        write_queries(&mut out, &['\u{e0a0}'], true).unwrap();
        assert_eq!(
            out,
            b"\x1bPtmux;\x1b\x1b_25a1;q;cp=e0a0\x1b\x1b\\\x1b\x1b[5n\x1b\\"
        );
    }
    #[test]
    fn heartbeat_uses_rtt_plus_grace_and_preserves_partial_results() {
        for (rtt_ms, grace_ms) in [(5, 50), (400, 100)] {
            let (tx, mut queries) = Queries::new();
            let (reply, rx) = bounded(1);
            let start = Instant::now();
            tx.send(Request {
                characters: vec!['a', 'b'],
                deadline: start + QUERY_TIMEOUT,
                reply,
            })
            .unwrap();
            queries.poll(true, &mut Vec::new(), false, start);
            queries.reply(b"25a1;q;cp=61;status=system");
            let heartbeat = start + Duration::from_millis(rtt_ms);
            queries.heartbeat(heartbeat);
            // Unsolicited duplicate heartbeats must not extend the deadline.
            queries.heartbeat(heartbeat + Duration::from_millis(10));
            queries.poll(
                true,
                &mut Vec::new(),
                false,
                heartbeat + Duration::from_millis(grace_ms - 1),
            );
            assert!(rx.is_empty());
            queries.poll(
                true,
                &mut Vec::new(),
                false,
                heartbeat + Duration::from_millis(grace_ms),
            );
            assert_eq!(rx.try_recv().unwrap(), vec![Some(true), None]);
        }
    }

    #[test]
    fn no_heartbeat_has_a_hard_bound_and_all_replies_finish_immediately() {
        let (tx, mut queries) = Queries::new();
        let now = Instant::now();
        for complete in [false, true] {
            let (reply, rx) = bounded(1);
            tx.send(Request {
                characters: vec!['a'],
                deadline: now + QUERY_TIMEOUT,
                reply,
            })
            .unwrap();
            queries.poll(true, &mut Vec::new(), false, now);
            if complete {
                queries.reply(b"25a1;q;cp=61;status=system");
                assert_eq!(rx.try_recv().unwrap(), vec![Some(true)]);
            } else {
                queries.poll(true, &mut Vec::new(), false, now + QUERY_TIMEOUT);
                assert_eq!(rx.try_recv().unwrap(), vec![None]);
            }
            assert!(!queries.has_work());
        }
    }
    #[test]
    fn coverage_and_malformed_replies() {
        for status in [
            "system",
            "glossary",
            "glossary,system",
            "future,system",
            "1",
            "2",
            "3",
        ] {
            assert_eq!(
                parse_reply(format!("25a1;q;cp=e0a0;status={status}").as_bytes()),
                Some(('\u{e0a0}', true))
            );
        }
        for status in ["", "future", "0"] {
            assert_eq!(
                parse_reply(format!("25a1;q;cp=e0a0;status={status}").as_bytes()),
                Some(('\u{e0a0}', false))
            );
        }
        for reply in [
            "25a1;r;cp=e0a0;status=0",
            "25a1;q;status=system",
            "25a1;q;cp=d800;status=system",
            "25a1;q;cp=e0a0",
            "25a1;q;cp=e0a0;status=;status=system",
        ] {
            assert_eq!(parse_reply(reply.as_bytes()), None);
        }
    }
    #[test]
    fn concurrent_queries_correlate_and_timeout_without_poisoning_next_query() {
        let (tx, mut queries) = Queries::new();
        let now = Instant::now();
        let (a, ar) = bounded(1);
        tx.send(Request {
            characters: vec!['\u{e0a0}', '\u{f07b}'],
            deadline: now + QUERY_TIMEOUT,
            reply: a,
        })
        .unwrap();
        queries.poll(true, &mut Vec::new(), false, now);
        queries.reply(b"25a1;q;cp=f07b;status=system");
        assert!(ar.is_empty());
        queries.reply(b"25a1;q;cp=e0a0;status=");
        assert_eq!(ar.try_recv().unwrap(), vec![Some(false), Some(true)]);
        let (c, cr) = bounded(1);
        tx.send(Request {
            characters: vec!['x'],
            deadline: now + QUERY_TIMEOUT,
            reply: c,
        })
        .unwrap();
        queries.poll(true, &mut Vec::new(), false, now + QUERY_TIMEOUT);
        assert_eq!(cr.try_recv().unwrap(), vec![None]);
        let mut out = Vec::new();
        let (d, dr) = bounded(1);
        tx.send(Request {
            characters: vec!['y'],
            deadline: now + QUERY_TIMEOUT,
            reply: d,
        })
        .unwrap();
        queries.poll(false, &mut out, false, now);
        assert!(out.is_empty());
        assert_eq!(dr.try_recv().unwrap(), vec![None]);
    }
}
