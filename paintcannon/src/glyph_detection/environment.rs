use super::{capabilities::Capabilities, terminals, tmux};
use std::collections::HashMap;

pub(super) struct Environment {
    values: HashMap<String, String>,
    clients: Option<Vec<Capabilities>>,
}

impl Environment {
    pub(super) fn current() -> Self {
        let mut environment = Self {
            values: std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
                .collect(),
            clients: None,
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
            return terminal.capabilities;
        }

        // These markers establish legacy glyph support without claiming a product identity.
        if !self.get("KITTY_WINDOW_ID").is_empty()
            || !self.get("KONSOLE_DBUS_SESSION").is_empty()
            || !self.get("VTE_VERSION").is_empty()
            || !self.get("GNOME_TERMINAL_SCREEN").is_empty()
            || !self.get("TERMINATOR_UUID").is_empty()
            || self.get("COLORTERM") == "xfce4-terminal"
        {
            return Capabilities {
                nerd_font: false,
                symbols_for_legacy_computing: true,
            };
        }
        Capabilities::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_uses_term_without_a_recognized_term_program() {
        for (term, nerd_font, symbols_for_legacy_computing) in [
            ("xterm-ghostty", true, true),
            ("xterm-kitty", false, true),
            ("wezterm", true, false),
            ("rio", true, false),
            ("alacritty", false, false),
            ("alacritty-direct", false, false),
            ("foot", false, true),
            ("foot-extra", false, true),
            ("foot-direct", false, true),
            ("rxvt-unicode-256color", false, false),
        ] {
            for program in [None, Some(""), Some("tmux"), Some("unrecognized")] {
                let mut pairs = vec![("TERM", term), ("SSH_TTY", "/dev/pts/1")];
                if let Some(program) = program {
                    pairs.push(("TERM_PROGRAM", program));
                }
                assert_eq!(
                    Environment::from_pairs(&pairs).capabilities(),
                    Capabilities {
                        nerd_font,
                        symbols_for_legacy_computing
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
                    nerd_font: true,
                    symbols_for_legacy_computing: true,
                }
            );
        }
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
                        nerd_font: false,
                        symbols_for_legacy_computing: true,
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
                    nerd_font,
                    symbols_for_legacy_computing,
                },
                "{term}"
            );
        }
    }

    #[test]
    fn tmux_checks_program_and_term_like_local_detection() {
        for (program, term, nerd_font, symbols_for_legacy_computing) in [
            ("WezTerm", "xterm-256color", true, false),
            ("ghostty", "xterm-ghostty", true, true),
            ("unrecognized", "xterm-ghostty", true, true),
            ("Apple_Terminal", "xterm-256color", false, false),
            ("tmux", "screen-256color", false, false),
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
                    nerd_font,
                    symbols_for_legacy_computing,
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
        for (program, nerd_font, symbols_for_legacy_computing) in [
            ("ghostty", true, true),
            ("WezTerm", true, false),
            ("Apple_Terminal", false, false),
            ("gnome-terminal", false, true),
            ("vscode", false, true),
        ] {
            let env =
                Environment::from_pairs(&[("TERM_PROGRAM", "tmux"), ("TERM", "tmux-256color")])
                    .with_clients(vec![
                        Environment::from_pairs(&[("TERM_PROGRAM", program)]).capabilities()
                    ]);
            assert_eq!(
                env.capabilities(),
                Capabilities {
                    nerd_font,
                    symbols_for_legacy_computing,
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
            (vec![ghostty, ghostty], ghostty, false),
            (vec![ghostty, wezterm], wezterm, false),
            (vec![ghostty, apple], Capabilities::default(), false),
            (vec![], Capabilities::default(), false),
        ] {
            // A pane inherited Ghostty at startup; clients may since have changed.
            let env = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")]).with_clients(clients);
            assert_eq!(env.capabilities(), expected);
            assert_eq!(env.can_probe(), can_probe);
        }
    }
}
