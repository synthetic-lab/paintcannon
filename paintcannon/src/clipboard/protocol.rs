//! Kitty's paste-event extension: https://sw.kovidgoyal.net/kitty/clipboard/
use base64::{engine::general_purpose::STANDARD, Engine};

pub(super) const QUERY: &[u8] = b"\x1b[?5522$p";
pub(super) const ENABLE: &[u8] = b"\x1b[?5522h";
pub(super) const DISABLE: &[u8] = b"\x1b[?5522l";
pub(crate) const MAX_PACKET: usize = 16 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Packet {
    pub id: Option<String>,
    pub status: String,
    pub mime: Option<String>,
    pub password: Option<String>,
    pub primary: bool,
    // Invalid data is retained as an error, not silently skipped: doing so
    // would deliver a corrupt image when a later DONE packet arrives.
    pub data: Option<Vec<u8>>,
}

pub(crate) fn parse_packet(bytes: &[u8]) -> Option<Packet> {
    let body = std::str::from_utf8(bytes).ok()?.strip_prefix("5522;")?;
    let (metadata, payload) = body.split_once(';').unwrap_or((body, ""));
    let field = |key| {
        metadata.split(':').find_map(|part| {
            let (name, value) = part.split_once('=')?;
            (name == key).then_some(value)
        })
    };
    if field("type") != Some("read") {
        return None;
    }
    let password = field("pw");
    let mut data = STANDARD
        .decode(payload)
        .ok()
        .filter(|data| data.len() <= 4096);
    if password.is_some_and(|password| STANDARD.decode(password).is_err()) {
        data = None;
    }
    Some(Packet {
        id: field("id").map(str::to_owned),
        status: field("status")?.to_owned(),
        mime: field("mime").and_then(|mime| String::from_utf8(STANDARD.decode(mime).ok()?).ok()),
        password: password.map(str::to_owned),
        primary: field("loc") == Some("primary"),
        data,
    })
}

pub(crate) fn parse_mode(bytes: &[u8]) -> Option<u8> {
    let mode = bytes.strip_prefix(b"\x1b[?5522;")?.strip_suffix(b"$y")?;
    match mode {
        [value @ b'0'..=b'4'] => Some(value - b'0'),
        _ => None,
    }
}

pub(super) fn read_request(id: &str, mime: &str, primary: bool, password: Option<&str>) -> Vec<u8> {
    let mut metadata = format!("type=read:id={id}");
    if primary {
        metadata.push_str(":loc=primary");
    }
    if let Some(password) = password {
        metadata.push_str(":pw=");
        metadata.push_str(password);
        metadata.push_str(":name=UGFzdGUgZXZlbnQ="); // "Paste event"
    }
    format!("\x1b]5522;{metadata};{}\x1b\\", STANDARD.encode(mime)).into_bytes()
}
