//! OSC 99 negotiation, consumed by the existing terminal input reader.
//! https://sw.kovidgoyal.net/kitty/desktop-notifications/#querying-for-support
use crossbeam_channel::{bounded, select_biased, Receiver, Sender};
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::terminal::encode_tmux_passthrough;
use crate::terminal_capabilities::Detection;

pub(crate) const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Only features advertised by the terminal, not inferred from its product name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Support {
    pub(crate) body: bool,
    pub(crate) close: bool,
    pub(crate) close_events: bool,
    pub(crate) report: bool,
    pub(crate) focus: bool,
    pub(crate) silent: bool,
    pub(crate) expiry: bool,
}

impl Detection {
    pub(crate) fn notification_support(&self) -> Option<Support> {
        *self.notification_support.get_or_init(|| {
            // Multiplexed replies have a request ID, but no client identity.
            // One reply cannot certify support in every attached tmux client.
            if !self.environment().has_single_terminal() {
                return None;
            }
            let queries = self.notification_queries.as_ref()?;
            let (reply, response) = bounded(1);
            let deadline = Instant::now() + QUERY_TIMEOUT;
            let request = Request { deadline, reply };
            select_biased! {
                recv(self.stop) -> _ => return None,
                send(queries, request) -> result => result.ok()?,
                default(QUERY_TIMEOUT) => return None,
            }
            select_biased! {
                recv(self.stop) -> _ => None,
                recv(response) -> result => result.ok().flatten(),
                default(deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(100)) => None,
            }
        })
    }
}

pub(crate) struct Request {
    pub(crate) deadline: Instant,
    pub(crate) reply: Sender<Option<Support>>,
}

struct Pending {
    request: Request,
    id: String,
    sent: Instant,
    heartbeat_received: bool,
}

pub(crate) struct Queries {
    incoming: Receiver<Request>,
    pending: Option<Pending>,
    namespace: String,
    next_id: u64,
}

impl Queries {
    pub(crate) fn new() -> (Sender<Request>, Self) {
        let (tx, incoming) = bounded(1);
        (
            tx,
            Self {
                incoming,
                pending: None,
                namespace: format!(
                    "pc-query-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                ),
                next_id: 0,
            },
        )
    }

    pub(crate) fn has_work(&self) -> bool {
        self.pending.is_some() || !self.incoming.is_empty()
    }

    pub(crate) fn poll(&mut self, enabled: bool, out: &mut impl Write, tmux: bool, now: Instant) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| !enabled || now >= p.request.deadline)
        {
            self.finish(None);
        }
        if self.pending.is_some() {
            return;
        }
        if let Ok(request) = self.incoming.try_recv() {
            if !enabled || now >= request.deadline {
                let _ = request.reply.try_send(None);
                return;
            }
            self.next_id += 1;
            let id = format!("{}-{}", self.namespace, self.next_id);
            if write_query(out, &id, tmux).is_err() {
                let _ = request.reply.try_send(None);
                return;
            }
            self.pending = Some(Pending {
                request,
                id,
                sent: now,
                heartbeat_received: false,
            });
        }
    }

    pub(crate) fn reply(&mut self, body: &[u8]) {
        if let Some(pending) = &self.pending {
            if let Some(support) = parse_reply(body, &pending.id) {
                self.finish(Some(support));
            }
        }
    }

    pub(crate) fn heartbeat(&mut self, now: Instant) {
        if let Some(pending) = &mut self.pending {
            if !pending.heartbeat_received {
                pending.heartbeat_received = true;
                // DA is untagged and can also answer another subsystem's query.
                // Allow the same RTT grace as glyph probes, rather than declaring
                // a missing OSC reply immediately on an unrelated DA response.
                let rtt = now.saturating_duration_since(pending.sent);
                let grace = (rtt / 4).max(Duration::from_millis(50));
                pending.request.deadline = pending.request.deadline.min(now + grace);
            }
        }
    }

    fn finish(&mut self, support: Option<Support>) {
        if let Some(pending) = self.pending.take() {
            let _ = pending.request.reply.try_send(support);
        }
    }
}

fn write_query(out: &mut impl Write, id: &str, tmux: bool) -> io::Result<()> {
    // Both query and heartbeat must reach the outer terminal, not tmux itself.
    let bytes = format!("\x1b]99;i={id}:p=?;\x1b\\\x1b[c");
    if tmux {
        encode_tmux_passthrough(out, bytes.as_bytes())?;
    } else {
        out.write_all(bytes.as_bytes())?;
    }
    out.flush()
}

fn parse_reply(bytes: &[u8], expected_id: &str) -> Option<Support> {
    let text = std::str::from_utf8(bytes).ok()?.strip_prefix("99;")?;
    let (metadata, payload) = text.split_once(';')?;
    let (mut id, mut kind) = (None, None);
    for field in metadata.split(':') {
        let (key, value) = field.split_once('=')?;
        match key {
            "i" if id.is_none() => id = Some(value),
            "p" if kind.is_none() => kind = Some(value),
            "i" | "p" => return None,
            _ => {}
        }
    }
    if id != Some(expected_id) || kind != Some("?") {
        return None;
    }
    // Otty documents a bare acknowledgement. It proves basic delivery only.
    // https://docs.otty.sh/vt/osc/osc-99
    if payload == "ok" {
        return Some(Support::default());
    }
    let mut support = Support::default();
    let mut title = false;
    let mut seen = std::collections::HashSet::new();
    for field in payload.split(':') {
        let (key, value) = field.split_once('=')?;
        if !seen.insert(key) || value.chars().any(char::is_control) {
            return None;
        }
        let contains = |item| value.split(',').any(|v| v == item);
        match key {
            "p" => {
                title = contains("title");
                support.body = contains("body");
                support.close = contains("close");
            }
            "a" => {
                support.report = contains("report");
                support.focus = contains("focus");
            }
            "c" => support.close_events = value == "1",
            "s" => support.silent = contains("silent"),
            "w" => support.expiry = value == "1",
            _ => {}
        }
    }
    title.then_some(support)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_capabilities::environment::Environment;
    use std::sync::{Arc, OnceLock};

    #[test]
    fn concurrent_callers_share_positive_and_negative_results_even_for_unknown_terminals() {
        for result in [
            None,
            Some(Support {
                close: true,
                ..Support::default()
            }),
        ] {
            let (tx, requests) = bounded::<Request>(1);
            let detection = Arc::new(Detection {
                environment: OnceLock::from(Environment::from_pairs(&[("TERM", "xterm-256color")])),
                notification_queries: Some(tx),
                ..Detection::default()
            });
            let callers: Vec<_> = (0..4)
                .map(|_| {
                    let detection = detection.clone();
                    std::thread::spawn(move || detection.notification_support())
                })
                .collect();
            let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
            request.reply.send(result).unwrap();
            for caller in callers {
                assert_eq!(caller.join().unwrap(), result);
            }
            assert_eq!(detection.notification_support(), result);
            assert!(
                requests.is_empty(),
                "one probe per snapshot, including failed probes"
            );
        }
    }

    #[test]
    fn multiple_or_detached_tmux_clients_are_not_certified_by_one_reply() {
        let kitty = Environment::from_pairs(&[("TERM", "xterm-kitty")]).capabilities();
        for clients in [vec![], vec![kitty, kitty]] {
            let (tx, requests) = bounded(1);
            let detection = Detection {
                environment: OnceLock::from(Environment::from_pairs(&[]).with_clients(clients)),
                notification_queries: Some(tx),
                ..Detection::default()
            };
            assert_eq!(detection.notification_support(), None);
            assert!(requests.is_empty());
        }
    }

    #[test]
    fn shutdown_interrupts_an_unanswered_notification_query() {
        let (tx, requests) = bounded(1);
        let (stop, stopping) = bounded(0);
        let detection = Detection {
            environment: OnceLock::from(Environment::from_pairs(&[])),
            notification_queries: Some(tx),
            stop: stopping,
            ..Detection::default()
        };
        let caller = std::thread::spawn(move || detection.notification_support());
        let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
        let before = Instant::now();
        drop(stop);
        assert_eq!(caller.join().unwrap(), None);
        assert!(before.elapsed() < Duration::from_secs(1));
        assert!(request.reply.send(Some(Support::default())).is_err());
    }

    #[test]
    fn query_and_heartbeat_reach_the_same_endpoint() {
        let mut out = Vec::new();
        write_query(&mut out, "pc-test", false).unwrap();
        assert_eq!(out, b"\x1b]99;i=pc-test:p=?;\x1b\\\x1b[c");
        out.clear();
        write_query(&mut out, "pc-test", true).unwrap();
        assert_eq!(
            out,
            b"\x1bPtmux;\x1b\x1b]99;i=pc-test:p=?;\x1b\x1b\\\x1b\x1b[c\x1b\\"
        );
    }

    #[test]
    fn parses_features_independently_and_ignores_future_features() {
        let parsed = parse_reply(b"99;i=test:p=?;p=title,body,close,icon:a=report,focus:c=1:w=1:s=system,silent:future=yes", "test").unwrap();
        assert_eq!(
            parsed,
            Support {
                body: true,
                close: true,
                close_events: true,
                report: true,
                focus: true,
                silent: true,
                expiry: true
            }
        );
        assert_eq!(
            parse_reply(b"99;p=?:i=test;p=title:c=1", "test"),
            Some(Support {
                close_events: true,
                ..Support::default()
            })
        );
        assert_eq!(
            parse_reply(b"99;i=test:p=?;p=title,close", "test"),
            Some(Support {
                close: true,
                ..Support::default()
            })
        );
        assert_eq!(
            parse_reply(b"99;i=test:p=?;ok", "test"),
            Some(Support::default())
        );
        for bytes in [
            "99;i=other:p=?;p=title,close",
            "99;i=test:p=close;",
            "99;i=test:p=?;p=body,close",
            "99;i=test:p=?;p=title,close:p=title",
            "99;i=test:i=test:p=?;p=title",
            "99;i=test:p=?;",
            "99;i=test;p=title",
            "99;i=test:p=?;p=title,close\n",
            "99;i=test:p=?;p=title:c",
        ] {
            assert_eq!(parse_reply(bytes.as_bytes(), "test"), None, "{bytes}");
        }
    }

    fn begin(
        queries: &mut Queries,
        tx: &Sender<Request>,
        now: Instant,
    ) -> (String, Receiver<Option<Support>>) {
        let (reply, rx) = bounded(1);
        tx.send(Request {
            deadline: now + QUERY_TIMEOUT,
            reply,
        })
        .unwrap();
        queries.poll(true, &mut Vec::new(), false, now);
        (queries.pending.as_ref().unwrap().id.clone(), rx)
    }

    #[test]
    fn replies_correlate_and_late_replies_cannot_enable_a_new_probe() {
        let (tx, mut queries) = Queries::new();
        let now = Instant::now();
        let (old, rx) = begin(&mut queries, &tx, now);
        queries.poll(true, &mut Vec::new(), false, now + QUERY_TIMEOUT);
        assert_eq!(rx.try_recv().unwrap(), None);
        let (id, rx) = begin(&mut queries, &tx, now + QUERY_TIMEOUT);
        assert_ne!(id, old);
        queries.reply(format!("99;i={old}:p=?;p=title,close").as_bytes());
        assert!(rx.is_empty());
        queries.reply(format!("99;i={id}:p=?;p=title").as_bytes());
        assert_eq!(rx.try_recv().unwrap(), Some(Support::default()));
        assert!(!queries.has_work());
    }

    #[test]
    fn missing_replies_use_heartbeat_grace_or_a_hard_deadline() {
        for heartbeat in [false, true] {
            let (tx, mut queries) = Queries::new();
            let now = Instant::now();
            let (_, rx) = begin(&mut queries, &tx, now);
            let deadline = if heartbeat {
                queries.heartbeat(now + Duration::from_millis(400));
                queries.heartbeat(now + Duration::from_millis(450));
                now + Duration::from_millis(500)
            } else {
                now + QUERY_TIMEOUT
            };
            queries.poll(
                true,
                &mut Vec::new(),
                false,
                deadline - Duration::from_millis(1),
            );
            assert!(rx.is_empty());
            queries.poll(true, &mut Vec::new(), false, deadline);
            assert_eq!(rx.try_recv().unwrap(), None);
        }
    }

    #[test]
    fn uncaptured_terminal_and_failed_writes_finish_without_support() {
        let (tx, mut queries) = Queries::new();
        let now = Instant::now();
        let (reply, rx) = bounded(1);
        tx.send(Request {
            deadline: now + QUERY_TIMEOUT,
            reply,
        })
        .unwrap();
        let mut out = Vec::new();
        queries.poll(false, &mut out, false, now);
        assert!(out.is_empty());
        assert_eq!(rx.try_recv().unwrap(), None);
        let (reply, rx) = bounded(1);
        tx.send(Request {
            deadline: now + QUERY_TIMEOUT,
            reply,
        })
        .unwrap();
        queries.poll(true, &mut &mut [][..], false, now);
        assert_eq!(rx.try_recv().unwrap(), None);
    }
}
