//! Built-in glyphs under the terminal's standard renderer/settings, not claims
//! about the installed font. Legacy support includes the smooth mosaic corners
//! U+1FB41/U+1FB4C/U+1FB52/U+1FB5D; sextants alone are insufficient.

// OSC 8 support; Konsole remains unconfirmed because it is disabled by default:
// https://github.com/Alhadis/OSC8-Adoption/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HyperlinkSupport {
    Unconfirmed,
    Supported {
        open_hint: &'static str,
    },
    PlatformSpecific {
        macos: &'static str,
        linux: &'static str,
        unknown: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TerminalCapabilities {
    pub(super) osc8: HyperlinkSupport,
    pub(super) supports_apc: bool,
    pub(super) nerd_font: bool,
    pub(super) symbols_for_legacy_computing: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Terminal {
    pub(super) term: &'static [&'static str],
    pub(super) term_program: &'static [&'static str],
    pub(super) capabilities: TerminalCapabilities,
}

const TERMINALS: &[Terminal] = &[
    // OSC 8 links are "subtle" links: Cmd on macOS, Ctrl elsewhere.
    // Shift bypasses application mouse reporting.
    // https://github.com/PerBothner/DomTerm/blob/master/hlib/domterm.js (clickLink)
    // https://github.com/PerBothner/DomTerm/blob/master/hlib/terminal.js (_mouseHandler)
    Terminal {
        term: &["domterm"],
        term_program: &["domterm"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Cmd-Shift-Click to open",
                linux: "Ctrl-Shift-Click to open",
                unknown: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // OSC 8 spans open on click; Alt overrides application mouse reporting.
    // https://github.com/chromium/hterm/blob/master/js/hterm_text_attributes.js
    // https://github.com/chromium/hterm/blob/master/js/hterm_terminal.js (defeatMouseReports_)
    Terminal {
        term: &[],
        term_program: &["hterm"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Option-Click to open",
                linux: "Alt-Click to open",
                unknown: "Alt-Click to open (Option-Click on macOS)",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // Hyper uses xterm.js's default OSC 8 handler (click, then confirmation).
    // webLinksActivationKey applies to auto-detected URLs, not OSC 8 links.
    // https://github.com/vercel/hyper/blob/canary/lib/components/term.tsx
    // https://github.com/xtermjs/xterm.js/blob/master/src/browser/OscLinkProvider.ts
    Terminal {
        term: &[],
        term_program: &["hyper"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // Ctrl opens links; Ctrl+Shift overrides application mouse modes.
    // https://mintty.github.io/mintty.1.html#h-Opening-files_-directories-and-URLs
    Terminal {
        term: &[],
        term_program: &["mintty"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Shift-Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // The OSC 8 link overlay handles primary-button clicks directly.
    // https://github.com/borisfaure/terminology/blob/master/src/bin/termio.c (_cb_link_up)
    Terminal {
        term: &[],
        term_program: &["terminology"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Click to open",
            },
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // Shift keeps clicks local instead of forwarding them to mouse-aware apps.
    // https://secureshellfish.app/help/terminal
    // https://secureshellfish.app/help/cursor-and-selection
    Terminal {
        term: &[],
        term_program: &["secure shellfish"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Shift-Click to open",
            },
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // 13.16 adds Go To URL and Ctrl-right-click to force the context menu;
    // 14.02 extends URL handling to OSC 8 hyperlinks.
    // https://www.celestialsoftware.net/version-history/
    Terminal {
        term: &[],
        term_program: &["absolutetelnet"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Right-Click, then Go To URL to open",
            },
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // https://learn.microsoft.com/en-us/windows/terminal/tips-and-tricks#hyperlinks
    // Since 1.25.2733.0, built-in U+1FB00..U+1FB94 includes all four smooth corners:
    // https://github.com/microsoft/terminal/discussions/20758
    Terminal {
        term: &[],
        term_program: &["windows_terminal", "windowsterminal"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // Hints describe defaults with mouse reporting enabled, not custom bindings.
    // Ghostty needs Shift to bypass capture, then Ctrl on Linux / Cmd on macOS:
    // https://ghostty.org/docs/config/reference#mouse-shift-capture
    // https://ghostty.org/docs/config/reference#link-url
    // Bundled Nerd Font + built-in smooth mosaics, including all four corners:
    // https://github.com/ghostty-org/ghostty/blob/v1.0.0/src/font/sprite/Box.zig
    // https://ghostty.org/docs/features
    Terminal {
        term: &["xterm-ghostty"],
        term_program: &["ghostty"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Cmd-Shift-Click to open",
                linux: "Ctrl-Shift-Click to open",
                unknown: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
            },
            supports_apc: true,
            nerd_font: true,
            symbols_for_legacy_computing: true,
        },
    },
    // Shift bypasses mouse reporting and is stripped before default bindings:
    // https://wezterm.org/config/mouse.html
    // https://wezterm.org/config/fonts.html
    // Legacy mosaics are in the nightly section, not the stable release:
    // https://wezterm.org/changelog.html#continuousnightly
    Terminal {
        term: &["wezterm"],
        term_program: &["wezterm"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Shift-Click to open",
            },
            supports_apc: true,
            nerd_font: true,
            symbols_for_legacy_computing: false,
        },
    },
    // OSC 8 uses Cmd-click on macOS, Alt-click elsewhere, including mouse capture.
    // https://rioterm.com/docs/features/hyperlinks
    // https://github.com/raphamorim/rio/blob/main/rio-backend/src/config/hints.rs (HintMouse::default)
    // https://github.com/raphamorim/rio/blob/main/frontends/rioterm/src/application.rs (hint_click)
    // Bundled Nerd Font; legacy renderer covers sextants/octants, not our corners:
    // https://github.com/raphamorim/rio/tree/main/rio-fonts/resources
    // https://github.com/raphamorim/rio/blob/main/sugarloaf/src/sprite/legacy.rs
    Terminal {
        term: &["rio"],
        term_program: &["rio"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Cmd-Click to open",
                linux: "Alt-Click to open",
                unknown: "Alt-Click to open (Cmd-Click on macOS)",
            },
            supports_apc: true,
            nerd_font: true,
            symbols_for_legacy_computing: false,
        },
    },
    // ctrl+shift+left is bound in both grabbed and ungrabbed modes:
    // https://sw.kovidgoyal.net/kitty/conf/#mouse-actions
    // https://github.com/kovidgoyal/kitty/blob/v0.20.0/kitty/fonts/box_drawing.py
    Terminal {
        term: &["xterm-kitty"],
        term_program: &["kitty"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Shift-Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // Default hyperlinks need no modifier; Shift bypasses mouse reporting:
    // https://alacritty.org/config-alacritty.html#mouse
    // Only sextants U+1FB00..U+1FB3B, NOT the smooth mosaic corners:
    // https://github.com/alacritty/alacritty/blob/master/extra/man/alacritty.5.scd#font
    Terminal {
        term: &["alacritty", "alacritty-direct"],
        term_program: &["alacritty"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Shift-Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    // foot opens links with URL-mode jump labels, not a default mouse binding:
    // https://codeberg.org/dnkl/foot/src/branch/master/doc/foot.ini.5.scd (show-urls-launch)
    // draw_wedge_triangle handles all four corners:
    // https://codeberg.org/dnkl/foot/src/branch/master/box-drawing.c
    Terminal {
        term: &["foot", "foot-extra", "foot-direct"],
        term_program: &["foot"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Shift-O, then type the link label to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // drawLegacyCharacter includes U+1FB3C..U+1FB6F:
    // https://github.com/KDE/konsole/blob/master/src/characters/LineBlockCharacters.cpp
    Terminal {
        term: &[],
        term_program: &["konsole"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Unconfirmed,
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // These frontends handle Ctrl-click before forwarding events to VTE.
    // Xfce checks for exactly Ctrl, so adding Shift is not interchangeable.
    // https://github.com/GNOME/gnome-terminal/blob/master/src/terminal-screen.cc
    // https://github.com/GNOME/console/blob/main/src/kgx-terminal.c
    // https://gitlab.gnome.org/GNOME/ptyxis/-/blob/main/src/ptyxis-terminal.c
    // https://github.com/xfce-mirror/xfce4-terminal/blob/master/terminal/terminal-widget.c
    // https://github.com/gnome-terminator/terminator/blob/master/terminatorlib/terminal.py
    // https://github.com/gnunn1/tilix/blob/master/source/gx/tilix/terminal/terminal.d
    // https://github.com/Guake/guake/blob/master/guake/terminal.py
    // https://github.com/mate-desktop/mate-terminal/blob/master/src/terminal-screen.c
    // https://github.com/realh/roxterm/blob/master/src/roxterm.c
    // Built-in smooth mosaics are shared by these VTE-based terminals:
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
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // WebGL custom-glyph renderer includes all four smooth mosaics.
    // Embeddings that disable custom glyphs/WebGL can differ.
    // https://github.com/xtermjs/xterm.js/blob/master/addons/addon-webgl/src/customGlyphs/CustomGlyphDefinitions.ts
    // https://code.visualstudio.com/docs/terminal/appearance#_custom-glyphs
    // Default OSC 8 activation is a click followed by a confirmation dialog:
    // https://github.com/xtermjs/xterm.js/blob/master/src/browser/OscLinkProvider.ts
    // https://github.com/xtermjs/xterm.js/blob/master/src/browser/Linkifier.ts
    Terminal {
        term: &[],
        term_program: &["xterm.js", "xtermjs"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // VS Code's default modifier depends on the frontend platform. A remote
    // integrated terminal may be on a different OS even without SSH_* variables,
    // so describe both modifiers (https://code.visualstudio.com/docs/terminal/basics#_links).
    Terminal {
        term: &[],
        term_program: &["vscode"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Click to open (Cmd-Click on macOS)",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: true,
        },
    },
    // No verified built-in smooth-mosaic/Nerd Font renderer.
    Terminal {
        term: &[],
        term_program: &["apple_terminal"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Unconfirmed,
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["iterm.app", "iterm2"],
        capabilities: TerminalCapabilities {
            // Option bypasses mouse reporting; Cmd opens the link.
            // https://iterm2.com/documentation-one-page.html (Mouse)
            osc8: HyperlinkSupport::Supported {
                open_hint: "Cmd-Option-Click to open",
            },
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        // Generic xterm TERM values are used by many other emulators.
        term: &[],
        term_program: &["xterm"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Unconfirmed,
            supports_apc: true,
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
        term_program: &["rxvt", "rxvt-unicode", "urxvt"],
        capabilities: TerminalCapabilities {
            osc8: HyperlinkSupport::Unconfirmed,
            supports_apc: false,
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
    use crate::glyph_detection::capabilities::Capabilities;

    #[test]
    fn every_terminal_name_resolves_to_its_definition() {
        use crate::glyph_detection::environment::{
            resolve_hyperlink_support, ClientPlatform, Environment,
        };

        for terminal in TERMINALS {
            let expected = Capabilities {
                osc8: resolve_hyperlink_support(
                    terminal.capabilities.osc8,
                    ClientPlatform::Unknown,
                ),
                nerd_font: terminal.capabilities.nerd_font,
                supports_apc: terminal.capabilities.supports_apc,
                symbols_for_legacy_computing: terminal.capabilities.symbols_for_legacy_computing,
            };
            assert!(!terminal.term.is_empty() || !terminal.term_program.is_empty());
            for (key, names) in [
                ("TERM", terminal.term),
                ("TERM_PROGRAM", terminal.term_program),
            ] {
                for name in names {
                    let local = Environment::from_pairs(&[(key, name)]);
                    assert_eq!(local.capabilities(), expected, "{key}={name}");
                    let tmux = Environment::from_pairs(&[
                        ("TERM", "tmux-256color"),
                        ("TERM_PROGRAM", "tmux"),
                    ])
                    .with_clients(vec![local.capabilities()]);
                    assert_eq!(tmux.capabilities(), expected, "tmux: {key}={name}");
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
            ("windows_terminal", false, true),
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
