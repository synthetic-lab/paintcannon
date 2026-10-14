/// Capabilities confirmed by an environment signal. False means unconfirmed.
/// Glyph capabilities can also be established by the glyph protocol; OSC 8
/// has no universal query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Capabilities {
    pub(super) osc8: bool,
    pub(super) nerd_font: bool,
    pub(super) symbols_for_legacy_computing: bool,
}
