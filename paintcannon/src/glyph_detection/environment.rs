use super::{capabilities::Capabilities, terminals};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) struct Environment(HashMap<String, String>);

impl Environment {
    pub(super) fn current() -> Self {
        let mut environment = Self(
            std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
                .collect(),
        );
        if !environment.get("TMUX").is_empty() {
            for key in ["TERM_PROGRAM", "TERM"] {
                if let Some(output) = tmux_environment(key) {
                    environment.apply_tmux_terminal(&output);
                }
            }
        }
        environment
    }

    pub(super) fn apply_tmux_terminal(&mut self, output: &str) {
        if let Some((key @ ("TERM_PROGRAM" | "TERM"), value)) = output.trim().split_once('=') {
            if !value.is_empty() && !value.chars().any(char::is_control) {
                self.0.insert(key.into(), value.into());
            }
        }
    }

    #[cfg(test)]
    pub(super) fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    fn get(&self, key: &str) -> &str {
        self.0.get(key).map(String::as_str).unwrap_or("")
    }

    pub(super) fn nerd_font_override(&self) -> Option<bool> {
        self.0.get("NERD_FONT").map(|value| value != "0")
    }

    pub(super) fn capabilities(&self) -> Capabilities {
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

fn tmux_environment(key: &str) -> Option<String> {
    use std::io::Read;
    let mut child = Command::new("tmux")
        .args(["show-environment", "-g", key])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_millis(300);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let mut text = String::new();
                child
                    .stdout
                    .take()?
                    .take(4096)
                    .read_to_string(&mut text)
                    .ok()?;
                return Some(text);
            }
            Ok(Some(_)) => return None,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
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
            let mut env = Environment::from_pairs(&[
                ("TMUX", "/tmp/tmux-test/default,123,0"),
                ("TERM_PROGRAM", "tmux"),
                ("TERM", "tmux-256color"),
            ]);
            env.apply_tmux_terminal("-TERM_PROGRAM\n");
            env.apply_tmux_terminal(&format!("TERM={term}\n"));
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
            let mut env =
                Environment::from_pairs(&[("TERM_PROGRAM", "tmux"), ("TERM", "tmux-256color")]);
            env.apply_tmux_terminal(&format!("TERM_PROGRAM={program}\n"));
            env.apply_tmux_terminal(&format!("TERM={term}\n"));
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
            let mut env =
                Environment::from_pairs(&[("TERM_PROGRAM", "tmux"), ("TERM", "tmux-256color")]);
            env.apply_tmux_terminal(&format!("TERM_PROGRAM={program}\n"));
            assert_eq!(
                env.capabilities(),
                Capabilities {
                    nerd_font,
                    symbols_for_legacy_computing,
                }
            );
        }
        for output in [
            "-TERM_PROGRAM",
            "",
            "TERM_PROGRAM=",
            "OTHER=ghostty",
            "TERM_PROGRAM=ghostty\nOTHER=1",
        ] {
            let mut env = Environment::from_pairs(&[("TERM_PROGRAM", "tmux")]);
            env.apply_tmux_terminal(output);
            assert_eq!(env.capabilities(), Capabilities::default());
        }
    }
}
