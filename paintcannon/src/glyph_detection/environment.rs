use super::{
    capabilities::{Capabilities, HyperlinkCapability},
    environment_markers::MARKERS,
    ssh,
    terminals::{self, HyperlinkSupport},
    tmux,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ClientPlatform {
    #[default]
    Unknown,
    MacOs,
    Linux,
}

impl ClientPlatform {
    pub(super) fn local() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }
}

pub(super) fn resolve_hyperlink_support(
    support: HyperlinkSupport,
    platform: ClientPlatform,
) -> HyperlinkCapability {
    let open_hint = match (support, platform) {
        (HyperlinkSupport::Unconfirmed, _) => return HyperlinkCapability::Unconfirmed,
        (HyperlinkSupport::Supported { open_hint }, _) => open_hint,
        (HyperlinkSupport::PlatformSpecific { macos, .. }, ClientPlatform::MacOs) => macos,
        (HyperlinkSupport::PlatformSpecific { linux, .. }, ClientPlatform::Linux) => linux,
        (HyperlinkSupport::PlatformSpecific { unknown, .. }, ClientPlatform::Unknown) => unknown,
    };
    HyperlinkCapability::Confirmed { open_hint }
}

pub(super) struct Environment {
    values: HashMap<String, String>,
    clients: Option<Vec<Capabilities>>,
    platform: ClientPlatform,
}

impl Environment {
    pub(super) fn current() -> Self {
        let mut environment = Self {
            values: std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
                .collect(),
            clients: None,
            platform: ClientPlatform::local(),
        };
        if !environment.get("TMUX").is_empty() {
            environment.clients = Some(tmux::client_capabilities(environment.get("TMUX_PANE")));
        }
        environment
    }

    pub(super) fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            values: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            clients: None,
            platform: ClientPlatform::Unknown,
        }
    }

    pub(super) fn with_platform(mut self, platform: ClientPlatform) -> Self {
        self.platform = platform;
        self
    }

    fn client_platform(&self) -> ClientPlatform {
        // SSH exposes the server OS, not the terminal's OS. Locale spellings
        // cannot establish that identity, either. Apply this per live tmux client.
        if ssh::from_environment(|key| self.values.get(key).map(String::as_bytes)) {
            ClientPlatform::Unknown
        } else {
            self.platform
        }
    }

    #[cfg(test)]
    pub(super) fn with_clients(mut self, clients: Vec<Capabilities>) -> Self {
        self.clients = Some(clients);
        self
    }

    pub(super) fn can_probe(&self) -> bool {
        // Passthrough replies carry no client identity. One positive reply cannot
        // prove support in every attached client; no clients cannot answer at all.
        self.clients
            .as_ref()
            .is_none_or(|clients| clients.len() == 1)
            && self.capabilities().supports_apc
    }

    fn get(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or("")
    }

    pub(super) fn nerd_font_override(&self) -> Option<bool> {
        self.values.get("NERD_FONT").map(|value| value != "0")
    }

    pub(super) fn capabilities(&self) -> Capabilities {
        if let Some(clients) = &self.clients {
            // Never use the pane's inherited terminal identifiers or markers:
            // they describe the terminal that started it, not today's clients.
            return Capabilities {
                supports_apc: !clients.is_empty()
                    && clients.iter().all(|client| client.supports_apc),
                osc8: match clients.as_slice() {
                    [] => HyperlinkCapability::Unconfirmed,
                    [client] => client.osc8,
                    clients
                        if clients
                            .iter()
                            .any(|client| client.osc8 == HyperlinkCapability::Unconfirmed) =>
                    {
                        HyperlinkCapability::Unconfirmed
                    }
                    _ => HyperlinkCapability::ConfirmedButUnhintable,
                },
                nerd_font: !clients.is_empty() && clients.iter().all(|client| client.nerd_font),
                symbols_for_legacy_computing: !clients.is_empty()
                    && clients
                        .iter()
                        .all(|client| client.symbols_for_legacy_computing),
            };
        }
        // A recognized terminal is authoritative, even for unconfirmed capabilities.
        // SSH may only forward TERM, so check both identifiers before other signals.
        if let Some(terminal) = terminals::from_term_program(self.get("TERM_PROGRAM"))
            .or_else(|| terminals::from_term(self.get("TERM")))
        {
            return Capabilities {
                osc8: resolve_hyperlink_support(terminal.capabilities.osc8, self.client_platform()),
                supports_apc: terminal.capabilities.supports_apc,
                nerd_font: terminal.capabilities.nerd_font,
                symbols_for_legacy_computing: terminal.capabilities.symbols_for_legacy_computing,
            };
        }

        MARKERS
            .iter()
            .find_map(|marker| (marker.detect)(self.get(marker.name)))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_queries_require_apc_support_locally_and_through_tmux() {
        assert!(!Environment::from_pairs(&[]).can_probe());
        for identity in [
            ("TERM_PROGRAM", ""),
            ("TERM_PROGRAM", "unknown"),
            ("TERM_PROGRAM", "not-rio"),
            ("TERM_PROGRAM", "secure shellfish"),
            ("TERM_PROGRAM", "absolutetelnet"),
            ("TERM_PROGRAM", "Apple_Terminal"),
            ("TERM_PROGRAM", "apple_terminal"),
            ("TERM_PROGRAM", "APPLE_TERMINAL"),
            ("TERM_PROGRAM", "terminology"),
            ("TERM_PROGRAM", "rxvt"),
            ("TERM_PROGRAM", "rxvt-unicode"),
            ("TERM_PROGRAM", "urxvt"),
            ("TERM", "rxvt"),
            ("TERM", "rxvt-color"),
            ("TERM", "rxvt-256color"),
            ("TERM", "rxvt-unicode"),
            ("TERM", "rxvt-unicode-256color"),
            ("TERM", "xterm"),
            ("TERM", "xterm-256color"),
            ("TERM", "screen-256color"),
            ("TERM", "tmux-256color"),
            ("TERM", "dumb"),
        ] {
            let local = Environment::from_pairs(&[identity]);
            assert!(!local.can_probe(), "{identity:?}");
            assert!(!Environment::from_pairs(&[identity, ("SSH_TTY", "/dev/pts/1")]).can_probe());
            assert!(!Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")])
                .with_clients(vec![local.capabilities()])
                .can_probe());
        }
        for identity in [
            ("TERM_PROGRAM", "domterm"),
            ("TERM_PROGRAM", "hterm"),
            ("TERM_PROGRAM", "hyper"),
            ("TERM_PROGRAM", "mintty"),
            ("TERM_PROGRAM", "windows_terminal"),
            ("TERM_PROGRAM", "WindowsTerminal"),
            ("TERM_PROGRAM", "ghostty"),
            ("TERM_PROGRAM", "WezTerm"),
            ("TERM_PROGRAM", "rio"),
            ("TERM_PROGRAM", "kitty"),
            ("TERM_PROGRAM", "alacritty"),
            ("TERM_PROGRAM", "foot"),
            ("TERM_PROGRAM", "iTerm.app"),
            ("TERM_PROGRAM", "ITERM2"),
            ("TERM_PROGRAM", "konsole"),
            ("TERM_PROGRAM", "gnome-terminal"),
            ("TERM_PROGRAM", "xfce4-terminal"),
            ("TERM_PROGRAM", "terminator"),
            ("TERM_PROGRAM", "gnome-console"),
            ("TERM_PROGRAM", "tilix"),
            ("TERM_PROGRAM", "ptyxis"),
            ("TERM_PROGRAM", "guake"),
            ("TERM_PROGRAM", "mate-terminal"),
            ("TERM_PROGRAM", "roxterm"),
            ("TERM_PROGRAM", "xterm.js"),
            ("TERM_PROGRAM", "xtermjs"),
            ("TERM_PROGRAM", "vscode"),
            ("TERM_PROGRAM", "xterm"),
            ("TERM", "domterm"),
            ("TERM", "xterm-ghostty"),
            ("TERM", "wezterm"),
            ("TERM", "rio"),
            ("TERM", "xterm-kitty"),
            ("TERM", "alacritty"),
            ("TERM", "alacritty-direct"),
            ("TERM", "foot"),
            ("TERM", "foot-extra"),
            ("TERM", "foot-direct"),
        ] {
            let local = Environment::from_pairs(&[identity]);
            assert!(local.can_probe(), "{identity:?}");
            assert!(Environment::from_pairs(&[identity, ("SSH_TTY", "/dev/pts/1")]).can_probe());
            assert!(
                Environment::from_pairs(&[("TERM_PROGRAM", "Apple_Terminal")])
                    .with_clients(vec![local.capabilities()])
                    .can_probe()
            );
        }
    }

    #[test]
    fn fallback_markers_identify_apc_support_without_overriding_known_terminals() {
        for marker in [
            ("KITTY_WINDOW_ID", "1"),
            ("WT_SESSION", "session"),
            ("VTE_VERSION", "7600"),
            ("KONSOLE_DBUS_SESSION", "/Sessions/1"),
            ("GNOME_TERMINAL_SCREEN", "/screen/1"),
            ("TERMINATOR_UUID", "session"),
            ("COLORTERM", "xfce4-terminal"),
        ] {
            let local = Environment::from_pairs(&[("TERM", "xterm-256color"), marker]);
            assert!(local.can_probe(), "{marker:?}");
            assert!(
                Environment::from_pairs(&[("TERM_PROGRAM", "Apple_Terminal")])
                    .with_clients(vec![local.capabilities()])
                    .can_probe()
            );
            assert!(
                !Environment::from_pairs(&[("TERM_PROGRAM", "Apple_Terminal"), marker]).can_probe()
            );
        }
    }

    #[test]
    fn foot_and_vte_terminals_have_opening_hints() {
        for (program, expected) in [
            ("foot", "Ctrl-Shift-O, then type the link label to open"),
            ("gnome-terminal", "Ctrl-Click to open"),
            ("xfce4-terminal", "Ctrl-Click to open"),
            ("terminator", "Ctrl-Click to open"),
            ("gnome-console", "Ctrl-Click to open"),
            ("tilix", "Ctrl-Click to open"),
            ("ptyxis", "Ctrl-Click to open"),
            ("guake", "Ctrl-Click to open"),
            ("mate-terminal", "Ctrl-Click to open"),
            ("roxterm", "Ctrl-Click to open"),
        ] {
            let local = Environment::from_pairs(&[("TERM_PROGRAM", program)]).capabilities();
            let expected = HyperlinkCapability::Confirmed {
                open_hint: expected,
            };
            assert_eq!(local.osc8, expected, "{program}");
            assert_eq!(
                Environment::from_pairs(&[])
                    .with_clients(vec![local])
                    .capabilities()
                    .osc8,
                expected,
                "tmux: {program}"
            );
        }
        assert_eq!(
            Environment::from_pairs(&[("VTE_VERSION", "7600")])
                .capabilities()
                .osc8,
            HyperlinkCapability::Confirmed {
                open_hint: "Ctrl-Click to open"
            }
        );
    }

    #[test]
    fn platform_specific_definition_resolves_to_a_concrete_capability() {
        let definition = HyperlinkSupport::PlatformSpecific {
            macos: "Cmd-Shift-Click to open",
            linux: "Ctrl-Shift-Click to open",
            unknown: "Consult this terminal's link bindings",
        };
        for (platform, expected) in [
            (ClientPlatform::MacOs, "Cmd-Shift-Click to open"),
            (ClientPlatform::Linux, "Ctrl-Shift-Click to open"),
            (
                ClientPlatform::Unknown,
                "Consult this terminal's link bindings",
            ),
        ] {
            assert_eq!(
                resolve_hyperlink_support(definition, platform),
                HyperlinkCapability::Confirmed {
                    open_hint: expected
                }
            );
        }
    }

    #[test]
    fn tmux_does_not_resolve_confirmed_client_hints_again_using_the_pane_platform() {
        let client = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")])
            .with_platform(ClientPlatform::MacOs)
            .capabilities();
        let pane = Environment::from_pairs(&[("SSH_TTY", "/dev/pts/1")])
            .with_platform(ClientPlatform::Linux)
            .with_clients(vec![client]);
        assert_eq!(pane.capabilities(), client);
        assert_eq!(
            pane.capabilities().osc8,
            HyperlinkCapability::Confirmed {
                open_hint: "Cmd-Shift-Click to open"
            }
        );
    }

    #[test]
    fn direct_terminal_hints_use_local_platform_and_terminal_defaults() {
        for (program, macos, linux, unknown) in [
            (
                "ghostty",
                "Cmd-Shift-Click to open",
                "Ctrl-Shift-Click to open",
                "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
            ),
            (
                "rio",
                "Cmd-Click to open",
                "Alt-Click to open",
                "Alt-Click to open (Cmd-Click on macOS)",
            ),
            (
                "WezTerm",
                "Shift-Click to open",
                "Shift-Click to open",
                "Shift-Click to open",
            ),
            (
                "kitty",
                "Ctrl-Shift-Click to open",
                "Ctrl-Shift-Click to open",
                "Ctrl-Shift-Click to open",
            ),
            (
                "alacritty",
                "Shift-Click to open",
                "Shift-Click to open",
                "Shift-Click to open",
            ),
        ] {
            for (platform, expected) in [
                (ClientPlatform::MacOs, macos),
                (ClientPlatform::Linux, linux),
                (ClientPlatform::Unknown, unknown),
            ] {
                let env =
                    Environment::from_pairs(&[("TERM_PROGRAM", program)]).with_platform(platform);
                assert_eq!(
                    env.capabilities().osc8,
                    HyperlinkCapability::Confirmed {
                        open_hint: expected
                    },
                    "{program}, {platform:?}"
                );
            }
        }
        for (program, platform, expected) in [
            (
                "iterm.app",
                ClientPlatform::MacOs,
                "Cmd-Option-Click to open",
            ),
            (
                "windows_terminal",
                ClientPlatform::Unknown,
                "Ctrl-Click to open",
            ),
        ] {
            let env = Environment::from_pairs(&[("TERM_PROGRAM", program)]).with_platform(platform);
            assert_eq!(
                env.capabilities().osc8,
                HyperlinkCapability::Confirmed {
                    open_hint: expected
                }
            );
        }
    }

    #[test]
    fn ssh_preserves_terminal_specific_hints_with_unknown_client_os() {
        for key in ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"] {
            for platform in [
                ClientPlatform::MacOs,
                ClientPlatform::Linux,
                ClientPlatform::Unknown,
            ] {
                for (program, expected) in [
                    (
                        "ghostty",
                        "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
                    ),
                    ("kitty", "Ctrl-Shift-Click to open"),
                    ("rio", "Alt-Click to open (Cmd-Click on macOS)"),
                    ("WezTerm", "Shift-Click to open"),
                    ("iterm.app", "Cmd-Option-Click to open"),
                ] {
                    let env = Environment::from_pairs(&[
                        ("TERM_PROGRAM", program),
                        (key, "remote"),
                        ("LC_CTYPE", "UTF-8"),
                    ])
                    .with_platform(platform);
                    assert_eq!(
                        env.capabilities().osc8,
                        HyperlinkCapability::Confirmed {
                            open_hint: expected
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn multiple_tmux_clients_keep_links_but_never_show_a_hint() {
        let local = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")])
            .with_platform(ClientPlatform::Linux)
            .capabilities();
        let mac = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")])
            .with_platform(ClientPlatform::MacOs)
            .capabilities();
        let wezterm = Environment::from_pairs(&[("TERM_PROGRAM", "WezTerm")])
            .with_platform(ClientPlatform::Linux)
            .capabilities();
        let remote =
            Environment::from_pairs(&[("TERM_PROGRAM", "ghostty"), ("SSH_CONNECTION", "remote")])
                .with_platform(ClientPlatform::Linux)
                .capabilities();
        for clients in [
            vec![local, local],
            vec![local, mac],
            vec![local, wezterm],
            vec![local, remote],
            vec![remote, local],
            vec![remote, remote],
        ] {
            let env = Environment::from_pairs(&[]).with_clients(clients);
            assert_eq!(
                env.capabilities().osc8,
                HyperlinkCapability::ConfirmedButUnhintable
            );
        }
    }

    #[test]
    fn osc8_support_uses_known_terminals_then_markers_not_nerd_font_overrides() {
        for (program, supported) in [
            ("ghostty", true),
            ("rio", true),
            ("WezTerm", true),
            ("alacritty", true),
            ("kitty", true),
            ("foot", true),
            ("iTerm.app", true),
            ("vscode", true),
            ("gnome-terminal", true),
            ("Hyper", true),
            ("Apple_Terminal", false),
            ("konsole", false),
            ("unrecognized", false),
        ] {
            for override_value in ["0", "1"] {
                let env = Environment::from_pairs(&[
                    ("TERM_PROGRAM", program),
                    ("NERD_FONT", override_value),
                ]);
                assert_eq!(
                    env.capabilities().osc8 != HyperlinkCapability::Unconfirmed,
                    supported,
                    "{program}"
                );
            }
        }
        assert_eq!(
            Environment::from_pairs(&[("VTE_VERSION", "4900")])
                .capabilities()
                .osc8,
            HyperlinkCapability::Unconfirmed
        );
        assert_eq!(
            Environment::from_pairs(&[("VTE_VERSION", "5004")])
                .capabilities()
                .osc8,
            HyperlinkCapability::Confirmed {
                open_hint: "Ctrl-Click to open"
            }
        );
        assert!(matches!(
            Environment::from_pairs(&[("WT_SESSION", "session")])
                .capabilities()
                .osc8,
            HyperlinkCapability::Confirmed { .. }
        ));
    }

    #[test]
    fn ssh_uses_term_without_a_recognized_term_program() {
        for (term, supports_apc, nerd_font, symbols_for_legacy_computing) in [
            ("xterm-ghostty", true, true, true),
            ("xterm-kitty", true, false, true),
            ("wezterm", true, true, false),
            ("rio", true, true, false),
            ("alacritty", true, false, false),
            ("alacritty-direct", true, false, false),
            ("foot", true, false, true),
            ("foot-extra", true, false, true),
            ("foot-direct", true, false, true),
            ("rxvt-unicode-256color", false, false, false),
        ] {
            for program in [None, Some(""), Some("tmux"), Some("unrecognized")] {
                let mut pairs = vec![("TERM", term), ("SSH_TTY", "/dev/pts/1")];
                if let Some(program) = program {
                    pairs.push(("TERM_PROGRAM", program));
                }
                assert_eq!(
                    Environment::from_pairs(&pairs)
                        .with_platform(ClientPlatform::Linux)
                        .capabilities(),
                    Capabilities {
                        supports_apc,
                        nerd_font,
                        symbols_for_legacy_computing,
                        osc8: match term {
                            "rxvt-unicode-256color" => HyperlinkCapability::Unconfirmed,
                            "rio" => HyperlinkCapability::Confirmed {
                                open_hint: "Alt-Click to open (Cmd-Click on macOS)"
                            },
                            "foot" | "foot-extra" | "foot-direct" =>
                                HyperlinkCapability::Confirmed {
                                    open_hint: "Ctrl-Shift-O, then type the link label to open"
                                },
                            "xterm-ghostty" => HyperlinkCapability::Confirmed {
                                open_hint: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
                            },
                            "xterm-kitty" => HyperlinkCapability::Confirmed {
                                open_hint: "Ctrl-Shift-Click to open"
                            },
                            _ => HyperlinkCapability::Confirmed {
                                open_hint: "Shift-Click to open"
                            },
                        },
                    },
                    "{term}, {program:?}"
                );
            }
        }
    }

    #[test]
    fn ssh_generic_term_does_not_claim_capabilities() {
        for term in [
            "xterm",
            "xterm-256color",
            "screen",
            "screen-256color",
            "tmux-256color",
            "dumb",
        ] {
            let env = Environment::from_pairs(&[("TERM", term), ("SSH_TTY", "/dev/pts/1")]);
            assert_eq!(env.capabilities(), Capabilities::default(), "{term}");
        }
    }

    #[test]
    fn term_program_provides_capabilities_when_term_is_generic() {
        for term in ["xterm-256color", "screen-256color", "tmux-256color"] {
            let env = Environment::from_pairs(&[("TERM", term), ("TERM_PROGRAM", "ghostty")]);
            assert_eq!(
                env.capabilities(),
                Capabilities {
                    supports_apc: true,
                    nerd_font: true,
                    symbols_for_legacy_computing: true,
                    osc8: HyperlinkCapability::Confirmed {
                        open_hint: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
                    },
                }
            );
        }
    }

    #[test]
    fn windows_terminal_and_wt_session_confirm_legacy_mosaic_corners() {
        for identity in [
            ("TERM_PROGRAM", "windows_terminal"),
            ("TERM_PROGRAM", "windowsterminal"),
            ("WT_SESSION", "session-id"),
        ] {
            let capabilities =
                Environment::from_pairs(&[("TERM", "xterm-256color"), identity]).capabilities();
            assert!(capabilities.symbols_for_legacy_computing, "{identity:?}");
            assert!(!capabilities.nerd_font);
            assert_eq!(
                capabilities.osc8,
                HyperlinkCapability::Confirmed {
                    open_hint: "Ctrl-Click to open"
                }
            );
        }
        // An unrelated inherited marker must not override a recognized terminal.
        let apple = Environment::from_pairs(&[
            ("TERM_PROGRAM", "Apple_Terminal"),
            ("WT_SESSION", "inherited-session"),
        ]);
        assert_eq!(apple.capabilities(), Capabilities::default());
    }

    #[test]
    fn markers_provide_legacy_capabilities_for_unidentified_terminals() {
        for marker in [
            ("KITTY_WINDOW_ID", "1"),
            ("KONSOLE_DBUS_SESSION", "/Sessions/1"),
            ("VTE_VERSION", "7600"),
            ("GNOME_TERMINAL_SCREEN", "/org/gnome/Terminal/screen/1"),
            ("TERMINATOR_UUID", "session"),
            ("COLORTERM", "xfce4-terminal"),
        ] {
            for term in ["xterm-256color", "tmux-256color"] {
                let env = Environment::from_pairs(&[("TERM", term), marker]);
                assert_eq!(
                    env.capabilities(),
                    Capabilities {
                        supports_apc: true,
                        nerd_font: false,
                        symbols_for_legacy_computing: true,
                        osc8: match marker.0 {
                            "KITTY_WINDOW_ID" => HyperlinkCapability::Confirmed {
                                open_hint: "Ctrl-Shift-Click to open"
                            },
                            "VTE_VERSION" => HyperlinkCapability::Confirmed {
                                open_hint: "Ctrl-Click to open"
                            },
                            _ => HyperlinkCapability::Unconfirmed,
                        },
                    },
                    "{term}, {marker:?}"
                );
            }
        }
    }

    #[test]
    fn empty_or_unrelated_markers_do_not_claim_capabilities() {
        for marker in [
            ("KITTY_WINDOW_ID", ""),
            ("KONSOLE_DBUS_SESSION", ""),
            ("VTE_VERSION", ""),
            ("GNOME_TERMINAL_SCREEN", ""),
            ("TERMINATOR_UUID", ""),
            ("COLORTERM", "truecolor"),
            ("XTERM_VERSION", "XTerm(390)"),
        ] {
            let env = Environment::from_pairs(&[("TERM", "xterm-256color"), marker]);
            assert_eq!(env.capabilities(), Capabilities::default(), "{marker:?}");
        }
    }

    #[test]
    fn tmux_uses_outer_term_when_term_program_is_missing() {
        for (term, nerd_font, symbols_for_legacy_computing) in [
            ("xterm-ghostty", true, true),
            ("xterm-kitty", false, true),
            ("wezterm", true, false),
            ("rio", true, false),
            ("alacritty-direct", false, false),
            ("foot-direct", false, true),
        ] {
            let env = Environment::from_pairs(&[
                ("TMUX", "/tmp/tmux-test/default,123,0"),
                ("TERM_PROGRAM", "tmux"),
                ("TERM", "tmux-256color"),
            ])
            .with_clients(vec![
                Environment::from_pairs(&[("TERM", term)]).capabilities()
            ]);
            assert_eq!(
                env.capabilities(),
                Capabilities {
                    supports_apc: true,
                    nerd_font,
                    symbols_for_legacy_computing,
                    osc8: match term {
                        "rio" => HyperlinkCapability::Confirmed {
                            open_hint: "Alt-Click to open (Cmd-Click on macOS)"
                        },
                        "foot-direct" => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Shift-O, then type the link label to open"
                        },
                        "xterm-ghostty" => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
                        },
                        "xterm-kitty" => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Shift-Click to open"
                        },
                        _ => HyperlinkCapability::Confirmed {
                            open_hint: "Shift-Click to open"
                        },
                    },
                },
                "{term}"
            );
        }
    }

    #[test]
    fn tmux_checks_program_and_term_like_local_detection() {
        for (program, term, supports_apc, nerd_font, symbols_for_legacy_computing) in [
            ("WezTerm", "xterm-256color", true, true, false),
            ("ghostty", "xterm-ghostty", true, true, true),
            ("unrecognized", "xterm-ghostty", true, true, true),
            ("Apple_Terminal", "xterm-256color", false, false, false),
            ("Apple_Terminal", "xterm-ghostty", false, false, false),
            ("tmux", "screen-256color", false, false, false),
        ] {
            let env =
                Environment::from_pairs(&[("TERM_PROGRAM", "tmux"), ("TERM", "tmux-256color")])
                    .with_clients(vec![Environment::from_pairs(&[
                        ("TERM_PROGRAM", program),
                        ("TERM", term),
                    ])
                    .capabilities()]);
            assert_eq!(
                env.capabilities(),
                Capabilities {
                    supports_apc,
                    nerd_font,
                    symbols_for_legacy_computing,
                    osc8: match program {
                        "WezTerm" => HyperlinkCapability::Confirmed {
                            open_hint: "Shift-Click to open"
                        },
                        "ghostty" | "unrecognized" => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
                        },
                        _ => HyperlinkCapability::Unconfirmed,
                    },
                },
                "{program}, {term}"
            );
            assert_eq!(
                env.capabilities(),
                Environment::from_pairs(&[("TERM_PROGRAM", program), ("TERM", term)])
                    .capabilities(),
            );
        }
    }

    #[test]
    fn tmux_outer_program_provides_capabilities() {
        for (program, supports_apc, nerd_font, symbols_for_legacy_computing) in [
            ("ghostty", true, true, true),
            ("WezTerm", true, true, false),
            ("Apple_Terminal", false, false, false),
            ("gnome-terminal", true, false, true),
            ("vscode", true, false, true),
        ] {
            let env =
                Environment::from_pairs(&[("TERM_PROGRAM", "tmux"), ("TERM", "tmux-256color")])
                    .with_clients(vec![
                        Environment::from_pairs(&[("TERM_PROGRAM", program)]).capabilities()
                    ]);
            assert_eq!(
                env.capabilities(),
                Capabilities {
                    supports_apc,
                    nerd_font,
                    symbols_for_legacy_computing,
                    osc8: match program {
                        "Apple_Terminal" => HyperlinkCapability::Unconfirmed,
                        "gnome-terminal" => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Click to open"
                        },
                        "ghostty" => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
                        },
                        "WezTerm" => HyperlinkCapability::Confirmed {
                            open_hint: "Shift-Click to open"
                        },
                        _ => HyperlinkCapability::Confirmed {
                            open_hint: "Ctrl-Click to open (Cmd-Click on macOS)"
                        },
                    },
                }
            );
        }
    }

    #[test]
    fn every_attached_client_must_support_a_capability() {
        let ghostty = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")]).capabilities();
        let wezterm = Environment::from_pairs(&[("TERM_PROGRAM", "WezTerm")]).capabilities();
        let apple = Environment::from_pairs(&[("TERM_PROGRAM", "Apple_Terminal")]).capabilities();
        for (clients, expected, can_probe) in [
            (vec![ghostty], ghostty, true),
            (
                vec![ghostty, ghostty],
                Capabilities {
                    osc8: HyperlinkCapability::ConfirmedButUnhintable,
                    ..ghostty
                },
                false,
            ),
            (
                vec![ghostty, wezterm],
                Capabilities {
                    osc8: HyperlinkCapability::ConfirmedButUnhintable,
                    ..wezterm
                },
                false,
            ),
            (vec![ghostty, apple], apple, false),
            (
                vec![Capabilities::default()],
                Capabilities::default(),
                false,
            ),
            (
                vec![ghostty, Capabilities::default()],
                Capabilities::default(),
                false,
            ),
            (vec![], Capabilities::default(), false),
        ] {
            // A pane inherited Ghostty at startup; clients may since have changed.
            let env = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")]).with_clients(clients);
            assert_eq!(env.capabilities(), expected);
            assert_eq!(env.can_probe(), can_probe);
        }
    }
}
