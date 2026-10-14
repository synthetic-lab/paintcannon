//! Built-in glyphs under the terminal's standard renderer/settings, not claims
//! about the installed font. Legacy support includes the smooth mosaic corners
//! U+1FB41/U+1FB4C/U+1FB52/U+1FB5D; sextants alone are insufficient.

// OSC 8 support; Konsole remains unconfirmed because it is disabled by default:
// https://github.com/Alhadis/OSC8-Adoption/
use super::capabilities::Capabilities;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Terminal {
    pub(super) term: &'static [&'static str],
    pub(super) term_program: &'static [&'static str],
    pub(super) capabilities: Capabilities,
}

const TERMINALS: &[Terminal] = &[
    Terminal {
        term: &["domterm"],
        term_program: &[
            "domterm",
            "hterm",
            "hyper",
            "mintty",
            "terminology",
            "windows_terminal",
            "windowsterminal",
            "secure shellfish",
            "absolutetelnet",
        ],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // Bundled Nerd Font + built-in smooth mosaics, including all four corners:
    // https://github.com/ghostty-org/ghostty/blob/v1.0.0/src/font/sprite/Box.zig
    // https://ghostty.org/docs/features
    Terminal {
        term: &["xterm-ghostty"],
        term_program: &["ghostty"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: true,
            symbols_for_legacy_computing: true,
        },
    },
    // https://wezterm.org/config/fonts.html
    // Legacy mosaics are in the nightly section, not the stable release:
    // https://wezterm.org/changelog.html#continuousnightly
    Terminal {
        term: &["wezterm"],
        term_program: &["wezterm"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: true,
            symbols_for_legacy_computing: false,
        },
    },
    // Bundled Nerd Font; legacy renderer covers sextants/octants, not our corners:
    // https://github.com/raphamorim/rio/tree/main/rio-fonts/resources
    // https://github.com/raphamorim/rio/blob/main/sugarloaf/src/sprite/legacy.rs
    Terminal {
        term: &["rio"],
        term_program: &["rio"],
        capabilities: Capabilities {
            osc8: false,
            nerd_font: true,
            symbols_for_legacy_computing: false,
        },
    },
    // https://github.com/kovidgoyal/kitty/blob/v0.20.0/kitty/fonts/box_drawing.py
    Terminal {
        term: &["xterm-kitty"],
        term_program: &["kitty"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // Only sextants U+1FB00..U+1FB3B, NOT the smooth mosaic corners:
    // https://github.com/alacritty/alacritty/blob/master/extra/man/alacritty.5.scd#font
    Terminal {
        term: &["alacritty", "alacritty-direct"],
        term_program: &["alacritty"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // draw_wedge_triangle handles all four corners:
    // https://codeberg.org/dnkl/foot/src/branch/master/box-drawing.c
    Terminal {
        term: &["foot", "foot-extra", "foot-direct"],
        term_program: &["foot"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // drawLegacyCharacter includes U+1FB3C..U+1FB6F:
    // https://github.com/KDE/konsole/blob/master/src/characters/LineBlockCharacters.cpp
    Terminal {
        term: &[],
        term_program: &["konsole"],
        capabilities: Capabilities {
            osc8: false,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // Shared by GNOME Terminal, Xfce Terminal, Terminator, Tilix, etc.
    // https://github.com/GNOME/vte/blob/0.76.0/src/minifont.cc#L1090
    Terminal {
        term: &[],
        term_program: &[
            "gnome-terminal",
            "xfce4-terminal",
            "terminator",
            "gnome-console",
            "tilix",
            "ptyxis",
            "guake",
            "mate-terminal",
            "roxterm",
        ],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // WebGL custom-glyph renderer includes all four smooth mosaics.
    // Embeddings that disable custom glyphs/WebGL can differ.
    // https://github.com/xtermjs/xterm.js/blob/master/addons/addon-webgl/src/customGlyphs/CustomGlyphDefinitions.ts
    // https://code.visualstudio.com/docs/terminal/appearance#_custom-glyphs
    Terminal {
        term: &[],
        term_program: &["xterm.js", "xtermjs", "vscode"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // No verified built-in smooth-mosaic/Nerd Font renderer.
    Terminal {
        term: &[],
        term_program: &["apple_terminal"],
        capabilities: Capabilities {
            osc8: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["iterm.app", "iterm2"],
        capabilities: Capabilities {
            osc8: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        // Generic xterm TERM values are used by many other emulators.
        term: &[],
        term_program: &["xterm"],
        capabilities: Capabilities {
            osc8: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[
            "rxvt",
            "rxvt-color",
            "rxvt-256color",
            "rxvt-unicode",
            "rxvt-unicode-256color",
        ],
        term_program: &["rxvt"],
        capabilities: Capabilities {
            osc8: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
];

pub(super) fn from_term(value: &str) -> Option<&'static Terminal> {
    TERMINALS
        .iter()
        .find(|terminal| terminal.term.contains(&value))
}

pub(super) fn from_term_program(value: &str) -> Option<&'static Terminal> {
    TERMINALS.iter().find(|terminal| {
        terminal
            .term_program
            .iter()
            .any(|program| program.eq_ignore_ascii_case(value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terminal_name_resolves_to_its_definition() {
        use crate::glyph_detection::environment::Environment;

        for terminal in TERMINALS {
            assert!(!terminal.term.is_empty() || !terminal.term_program.is_empty());
            for (key, names) in [
                ("TERM", terminal.term),
                ("TERM_PROGRAM", terminal.term_program),
            ] {
                for name in names {
                    let local = Environment::from_pairs(&[(key, name)]);
                    assert_eq!(local.capabilities(), terminal.capabilities, "{key}={name}");
                    let tmux = Environment::from_pairs(&[
                        ("TERM", "tmux-256color"),
                        ("TERM_PROGRAM", "tmux"),
                    ])
                    .with_clients(vec![local.capabilities()]);
                    assert_eq!(
                        tmux.capabilities(),
                        terminal.capabilities,
                        "tmux: {key}={name}"
                    );
                }
            }
            for name in terminal.term_program {
                assert_eq!(from_term_program(&name.to_uppercase()), Some(terminal));
            }
        }
    }

    #[test]
    fn generic_or_unknown_names_do_not_claim_support() {
        for value in [
            "",
            "unknown",
            "xterm",
            "xterm-256color",
            "tmux-256color",
            "screen-256color",
        ] {
            assert_eq!(from_term(value), None, "{value}");
        }
        for value in ["", "unknown", "tmux", "xterm-ghostty"] {
            assert_eq!(from_term_program(value), None, "{value}");
        }
    }

    #[test]
    fn nerd_fonts_and_legacy_mosaics_are_independent_capabilities() {
        for (program, nerd, legacy) in [
            ("ghostty", true, true),
            ("wezterm", true, false),
            ("rio", true, false),
            ("apple_terminal", false, false),
            ("alacritty", false, false),
            ("kitty", false, true),
            ("gnome-terminal", false, true),
            ("xterm.js", false, true),
            ("vscode", false, true),
            ("foot", false, true),
            ("konsole", false, true),
        ] {
            let terminal = from_term_program(program).unwrap();
            assert_eq!(terminal.capabilities.nerd_font, nerd, "{program}");
            assert_eq!(
                terminal.capabilities.symbols_for_legacy_computing, legacy,
                "{program}"
            );
        }
    }
}
