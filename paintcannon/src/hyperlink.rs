use std::sync::Arc;

use taffy::NodeId;

pub(crate) struct Anchor {
    pub(crate) href: Arc<str>,
    pub(crate) fallback: Option<[NodeId; 2]>,
}

// OSC 8 requires a printable URI. Encode UTF-8 bytes outside printable ASCII,
// including ESC, BEL and ST, so even direct NAPI callers cannot inject controls.
// https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
pub(crate) fn encode_href(href: &str) -> Arc<str> {
    use std::fmt::Write;
    let mut encoded = String::with_capacity(href.len());
    for byte in href.bytes() {
        if (0x21..=0x7e).contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            write!(&mut encoded, "%{byte:02X}").expect("writing to String");
        }
    }
    encoded.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_terminal_controls_spaces_and_unicode_without_double_encoding() {
        assert_eq!(
            &*encode_href("https://example.com/a b/%20?x=\u{e9}\x1b\x07\u{9c}"),
            "https://example.com/a%20b/%20?x=%C3%A9%1B%07%C2%9C"
        );
    }
}
