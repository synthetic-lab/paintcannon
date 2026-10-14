use super::{protocol::Request, Detection};
use crossbeam_channel::Sender;
use napi::{Env, Result, Task};
use std::sync::Arc;

pub(super) const SAMPLES: &[char] = &[
    '\u{1fb00}',
    '\u{1fb41}',
    '\u{1fb4c}',
    '\u{1fb52}',
    '\u{1fb5d}',
];

pub struct HasSymbolsForLegacyComputing {
    pub(crate) detection: Arc<Detection>,
    pub(crate) queries: Option<Sender<Request>>,
}

#[napi_derive::napi]
impl Task for HasSymbolsForLegacyComputing {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<bool> {
        Ok(self.detection.detect_legacy(self.queries.as_ref()))
    }
    fn resolve(&mut self, _env: Env, output: bool) -> Result<bool> {
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_detection::environment::Environment;
    use std::sync::OnceLock;

    fn resolve(env: Environment) -> bool {
        Detection {
            environment: OnceLock::from(env),
            ..Detection::default()
        }
        .detect_legacy(None)
    }

    #[test]
    fn builtin_legacy_support_is_independent_of_nerd_font_override() {
        for program in [
            "ghostty",
            "kitty",
            "gnome-terminal",
            "xfce4-terminal",
            "xterm.js",
            "vscode",
            "foot",
            "konsole",
        ] {
            for override_value in ["0", "1"] {
                let env = Environment::from_pairs(&[
                    ("TERM_PROGRAM", program),
                    ("NERD_FONT", override_value),
                ]);
                assert!(resolve(env), "{program}");
            }
        }
        let env = Environment::from_pairs(&[("NERD_FONT", "1")]);
        assert!(!resolve(env));
    }

    #[test]
    fn partial_or_unverified_builtin_coverage_fails_closed() {
        for program in ["Apple_Terminal", "alacritty", "rio", "WezTerm"] {
            let env = Environment::from_pairs(&[("TERM_PROGRAM", program)]);
            assert!(!resolve(env));
        }
    }

    #[test]
    fn tmux_outer_terminal_is_used_but_server_fonts_are_not() {
        let env = Environment::from_pairs(&[
            ("TERM_PROGRAM", "tmux"),
            ("TMUX", "session"),
            ("NERD_FONT", "0"),
        ]);
        assert!(!env.capabilities().symbols_for_legacy_computing);
        let env = env.with_clients(vec![Environment::from_pairs(&[(
            "TERM_PROGRAM",
            "ghostty",
        )])
        .capabilities()]);
        assert!(env.capabilities().symbols_for_legacy_computing);
        let env = env.with_clients(vec![Environment::from_pairs(&[(
            "TERM_PROGRAM",
            "Apple_Terminal",
        )])
        .capabilities()]);
        assert!(!resolve(env));
    }
}
