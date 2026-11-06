/// Resolved OSC 8 support with instructions for the connected client(s).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum HyperlinkCapability {
    #[default]
    Unconfirmed,
    ConfirmedButUnhintable,
    Confirmed {
        open_hint: &'static str,
    },
}

/// Capabilities confirmed by an environment signal. False means unconfirmed.
/// Glyph capabilities can also be established by the glyph protocol; OSC 8
/// has no universal query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Capabilities {
    pub(crate) osc8: HyperlinkCapability,
    pub(crate) notifications: NotificationProtocols,
    /// Local desktop commands must never target an SSH server's desktop.
    pub(crate) local: bool,
    pub(crate) supports_apc: bool,
    pub(crate) nerd_font: bool,
    pub(crate) symbols_for_legacy_computing: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NotificationProtocols {
    pub(crate) osc9: bool,
    pub(crate) osc777: bool,
}

impl NotificationProtocols {
    pub(crate) const NONE: Self = Self {
        osc9: false,
        osc777: false,
    };
    pub(crate) const SIMPLE: Self = Self {
        osc9: true,
        osc777: true,
    };
    pub(crate) const ITERM: Self = Self {
        osc9: true,
        osc777: false,
    };
    pub(crate) const URXVT: Self = Self {
        osc9: false,
        osc777: true,
    };

    pub(crate) fn intersect(self, other: Self) -> Self {
        Self {
            osc9: self.osc9 && other.osc9,
            osc777: self.osc777 && other.osc777,
        }
    }
}
