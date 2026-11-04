//! Desktop notification OSC encoders. Text never becomes terminal instructions.
use super::query::Support;
use crate::terminal::{base64_encode, encode_tmux_passthrough};
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Protocol {
    Osc9,
    Osc777,
    Osc99(Support),
}

impl Protocol {
    pub(crate) fn supports_close(self) -> bool {
        matches!(self, Self::Osc99(support) if support.close)
    }
}

pub(crate) fn show(
    out: &mut impl Write,
    protocol: Protocol,
    tmux: bool,
    id: &str,
    options: &super::NotificationOptions,
) -> io::Result<()> {
    match protocol {
        Protocol::Osc9 => {
            let text = if options.body.is_empty() {
                options.title.clone()
            } else {
                format!("{}: {}", options.title, options.body)
            };
            // A numeric first parameter is a ConEmu command, not a notification.
            let text = safe_text(&text);
            let prefix = if text.split_once(';').is_some_and(|(head, _)| {
                !head.is_empty() && head.bytes().all(|b| b.is_ascii_digit())
            }) {
                " "
            } else {
                ""
            };
            sequence(out, tmux, &format!("\x1b]9;{prefix}{text}\x1b\\"))
        }
        Protocol::Osc777 => {
            // OSC 777 has no delimiter escaping. Preserve field boundaries.
            let title = safe_text(&options.title).replace(';', ":");
            let body = safe_text(&options.body).replace(';', ":");
            sequence(out, tmux, &format!("\x1b]777;notify;{title};{body}\x1b\\"))
        }
        Protocol::Osc99(support) => {
            let mut metadata = format!("i={id}");
            if support.report {
                metadata.push_str(":a=report");
            } else if support.focus {
                metadata.push_str(":a=-focus");
            }
            if support.close_events {
                metadata.push_str(":c=1");
            }
            if support.silent && options.silent == Some(true) {
                metadata.push_str(":s=c2lsZW50");
            }
            if support.expiry && options.require_interaction {
                metadata.push_str(":w=0");
            }
            metadata.push_str(":e=1");
            let title = if !support.body && !options.body.is_empty() {
                std::borrow::Cow::Owned(format!("{}: {}", options.title, options.body))
            } else {
                std::borrow::Cow::Borrowed(options.title.as_str())
            };
            let payloads = [("title", title.as_ref()), ("body", options.body.as_str())];
            let payloads = &payloads[..if support.body { 2 } else { 1 }];
            // Encode first, then chunk on base64 groups (kitty allows 4096 encoded
            // bytes per chunk). Keep a title/body pair atomic under the stdout lock.
            for (payload_index, &(payload, text)) in payloads.iter().enumerate() {
                let final_payload = payload_index + 1 == payloads.len();
                let encoded = base64_encode(text.as_bytes());
                let chunks = encoded.as_bytes().chunks(2048).collect::<Vec<_>>();
                if chunks.is_empty() {
                    sequence(
                        out,
                        tmux,
                        &format!(
                            "\x1b]99;{metadata}:p={payload}:d={};\x1b\\",
                            u8::from(final_payload)
                        ),
                    )?;
                }
                for (index, chunk) in chunks.iter().enumerate() {
                    let done = final_payload && index + 1 == chunks.len();
                    sequence(
                        out,
                        tmux,
                        &format!(
                            "\x1b]99;{metadata}:p={payload}:d={};{}\x1b\\",
                            u8::from(done),
                            std::str::from_utf8(chunk).map_err(io::Error::other)?
                        ),
                    )?;
                }
            }
            Ok(())
        }
    }
}

pub(crate) fn close(
    out: &mut impl Write,
    protocol: Protocol,
    tmux: bool,
    id: &str,
) -> io::Result<()> {
    if protocol.supports_close() {
        sequence(out, tmux, &format!("\x1b]99;i={id}:p=close;\x1b\\"))?;
    }
    Ok(())
}

fn sequence(out: &mut impl Write, tmux: bool, text: &str) -> io::Result<()> {
    if tmux {
        encode_tmux_passthrough(out, text.as_bytes())
    } else {
        out.write_all(text.as_bytes())
    }
}

fn safe_text(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) id: String,
    pub(crate) kind: &'static str,
}

pub(crate) fn parse_reply(bytes: &[u8]) -> Option<Reply> {
    let text = std::str::from_utf8(bytes).ok()?.strip_prefix("99;")?;
    let (metadata, payload) = text.split_once(';')?;
    let mut id = None;
    let mut kind = "click";
    for part in metadata.split(':') {
        let (key, value) = part.split_once('=')?;
        match key {
            "i" if id.is_none() => id = Some(value),
            "p" if value == "close" => kind = "close",
            // Queries, alive lists, button actions and untracked closes are not
            // lifecycle reports for the non-persistent Notification API.
            "p" | "i" => return None,
            _ => {}
        }
    }
    let id = id?;
    if !payload.is_empty()
        || id.is_empty()
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return None;
    }
    Some(Reply {
        id: id.to_string(),
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const FULL: Support = Support {
        body: true,
        close: true,
        close_events: true,
        report: true,
        focus: true,
        silent: true,
        expiry: true,
    };
    fn options(title: &str, body: &str) -> super::super::NotificationOptions {
        super::super::NotificationOptions {
            title: title.into(),
            body: body.into(),
            tag: String::new(),
            silent: None,
            require_interaction: false,
        }
    }
    #[test]
    fn legacy_payloads_cannot_inject_escapes_or_conemu_commands() {
        let mut out = Vec::new();
        show(
            &mut out,
            Protocol::Osc9,
            false,
            "id",
            &options("4;1;100\x07\x1b]52;x\x1b\\", ""),
        )
        .unwrap();
        assert_eq!(out, b"\x1b]9; 4;1;100  ]52;x \\\x1b\\");
        out.clear();
        show(
            &mut out,
            Protocol::Osc777,
            false,
            "id",
            &options("title;injected", "body\nline\u{9c}"),
        )
        .unwrap();
        assert_eq!(out, b"\x1b]777;notify;title:injected;body line \x1b\\");
    }
    #[test]
    fn kitty_encodes_text_and_preserves_complete_tmux_frames() {
        let mut out = Vec::new();
        let mut request = options("hello", "world");
        request.silent = Some(true);
        request.require_interaction = true;
        show(&mut out, Protocol::Osc99(FULL), true, "pc-1", &request).unwrap();
        assert_eq!(out, b"\x1bPtmux;\x1b\x1b]99;i=pc-1:a=report:c=1:s=c2lsZW50:w=0:e=1:p=title:d=0;aGVsbG8=\x1b\x1b\\\x1b\\\x1bPtmux;\x1b\x1b]99;i=pc-1:a=report:c=1:s=c2lsZW50:w=0:e=1:p=body:d=1;d29ybGQ=\x1b\x1b\\\x1b\\");
    }
    #[test]
    fn kitty_chunks_large_payloads_and_only_finishes_the_last_chunk() {
        let mut out = Vec::new();
        show(
            &mut out,
            Protocol::Osc99(FULL),
            false,
            "pc-1",
            &options(&"x".repeat(9000), "body"),
        )
        .unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(out.matches("d=1").count(), 1);
        assert!(out.split("\x1b\\").all(|chunk| chunk.len() < 4096));
    }
    #[test]
    fn display_only_reply_does_not_enable_optional_metadata_or_close_payload() {
        let mut request = options("hello", "world");
        request.silent = Some(true);
        request.require_interaction = true;
        let protocol = Protocol::Osc99(Support {
            body: true,
            ..Support::default()
        });
        let mut out = Vec::new();
        show(&mut out, protocol, false, "pc-1", &request).unwrap();
        assert_eq!(out, b"\x1b]99;i=pc-1:e=1:p=title:d=0;aGVsbG8=\x1b\\\x1b]99;i=pc-1:e=1:p=body:d=1;d29ybGQ=\x1b\\");
        out.clear();
        show(&mut out, protocol, true, "pc-1", &request).unwrap();
        assert_eq!(out, b"\x1bPtmux;\x1b\x1b]99;i=pc-1:e=1:p=title:d=0;aGVsbG8=\x1b\x1b\\\x1b\\\x1bPtmux;\x1b\x1b]99;i=pc-1:e=1:p=body:d=1;d29ybGQ=\x1b\x1b\\\x1b\\");
        for tmux in [false, true] {
            out.clear();
            close(&mut out, protocol, tmux, "pc-1").unwrap();
            assert!(out.is_empty());
        }
        assert!(!protocol.supports_close());
    }

    #[test]
    fn close_requires_the_close_payload_not_close_events() {
        for protocol in [
            Protocol::Osc9,
            Protocol::Osc777,
            Protocol::Osc99(Support {
                close_events: true,
                ..Support::default()
            }),
        ] {
            let mut out = Vec::new();
            close(&mut out, protocol, false, "pc-1").unwrap();
            assert!(!protocol.supports_close());
            assert!(out.is_empty());
        }
        let protocol = Protocol::Osc99(Support {
            close: true,
            ..Support::default()
        });
        assert!(protocol.supports_close());
        let mut out = Vec::new();
        close(&mut out, protocol, false, "pc-1").unwrap();
        assert_eq!(out, b"\x1b]99;i=pc-1:p=close;\x1b\\");
        out.clear();
        close(&mut out, protocol, true, "pc-1").unwrap();
        assert_eq!(
            out,
            b"\x1bPtmux;\x1b\x1b]99;i=pc-1:p=close;\x1b\x1b\\\x1b\\"
        );
    }

    #[test]
    fn acknowledgement_only_combines_title_and_body_without_optional_features() {
        let mut out = Vec::new();
        show(
            &mut out,
            Protocol::Osc99(Support::default()),
            false,
            "pc-1",
            &options("hello", "world"),
        )
        .unwrap();
        assert_eq!(
            out,
            b"\x1b]99;i=pc-1:e=1:p=title:d=1;aGVsbG86IHdvcmxk\x1b\\"
        );
        out.clear();
        show(
            &mut out,
            Protocol::Osc99(Support {
                focus: true,
                ..Support::default()
            }),
            false,
            "pc-1",
            &options("hello", ""),
        )
        .unwrap();
        assert_eq!(
            out,
            b"\x1b]99;i=pc-1:a=-focus:e=1:p=title:d=1;aGVsbG8=\x1b\\"
        );
    }

    #[test]
    fn replies_ignore_untracked_closes_and_invalid_identifiers() {
        assert_eq!(
            parse_reply(b"99;i=pc-1;"),
            Some(Reply {
                id: "pc-1".into(),
                kind: "click"
            })
        );
        assert_eq!(parse_reply(b"99;i=pc-1:p=close;").unwrap().kind, "close");
        for bytes in [
            b"99;i=pc-1:p=close;untracked".as_slice(),
            b"99;i=pc-1;pasted",
            b"99;i=pc-1:p=?;",
            b"99;i=pc-1;i=pc-2;",
        ] {
            assert!(parse_reply(bytes).is_none());
        }
    }
}
