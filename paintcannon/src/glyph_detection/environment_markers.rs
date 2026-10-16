//! Fallback capability signals for terminals without a recognized identity.
use super::capabilities::{Capabilities, HyperlinkCapability};

pub(super) struct EnvironmentMarker {
    pub(super) name: &'static str,
    pub(super) detect: fn(&str) -> Option<Capabilities>,
}

const LEGACY_SYMBOLS: Capabilities = Capabilities {
    osc8: HyperlinkCapability::Unconfirmed,
    supports_apc: true,
    nerd_font: false,
    symbols_for_legacy_computing: true,
};

fn legacy_symbols(value: &str) -> Option<Capabilities> {
    (!value.is_empty()).then_some(LEGACY_SYMBOLS)
}

// First matching marker wins. Keep names and interpretation together so local
// detection and tmux's client-environment queries cannot drift apart.
pub(super) const MARKERS: &[EnvironmentMarker] = &[
    EnvironmentMarker {
        name: "KITTY_WINDOW_ID",
        detect: |value| {
            (!value.is_empty()).then_some(Capabilities {
                osc8: HyperlinkCapability::Confirmed {
                    open_hint: "Ctrl-Shift-Click to open",
                },
                ..LEGACY_SYMBOLS
            })
        },
    },
    EnvironmentMarker {
        name: "WT_SESSION",
        detect: |value| {
            (!value.is_empty()).then_some(Capabilities {
                osc8: HyperlinkCapability::Confirmed {
                    open_hint: "Ctrl-Click to open",
                },
                ..LEGACY_SYMBOLS
            })
        },
    },
    EnvironmentMarker {
        name: "VTE_VERSION",
        detect: |value| {
            let mut capabilities = legacy_symbols(value)?;
            if value.parse::<u32>().is_ok_and(|version| version >= 5000) {
                // Conventional VTE frontend binding; known terminal definitions
                // remain authoritative. See the frontend sources in terminals.rs.
                capabilities.osc8 = HyperlinkCapability::Confirmed {
                    open_hint: "Ctrl-Click to open",
                };
            }
            Some(capabilities)
        },
    },
    EnvironmentMarker {
        name: "KONSOLE_DBUS_SESSION",
        detect: legacy_symbols,
    },
    EnvironmentMarker {
        name: "GNOME_TERMINAL_SCREEN",
        detect: legacy_symbols,
    },
    EnvironmentMarker {
        name: "TERMINATOR_UUID",
        detect: legacy_symbols,
    },
    EnvironmentMarker {
        name: "COLORTERM",
        detect: |value| (value == "xfce4-terminal").then_some(LEGACY_SYMBOLS),
    },
];
