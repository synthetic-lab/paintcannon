/// Capabilities confirmed by an environment signal. False means unconfirmed,
/// so the glyph protocol can still establish support.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Capabilities {
    pub(super) nerd_font: bool,
    pub(super) symbols_for_legacy_computing: bool,
}
