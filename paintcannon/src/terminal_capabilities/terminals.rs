//! Capabilities under the terminal's standard renderer/settings, not claims
//! about the installed font. Legacy support includes the smooth mosaic corners
//! U+1FB41/U+1FB4C/U+1FB52/U+1FB5D; sextants alone are insufficient.
use super::NotificationProtocols;
use crate::graphics::GraphicsProtocol;

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
    pub(super) notifications: NotificationProtocols,
    pub(super) supports_apc: bool,
    pub(super) graphics: Option<GraphicsProtocol>,
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
    Terminal {
        // TERM defaults to generic xterm-256color, not an Otty-specific identifier:
        // https://docs.otty.sh/terminal-features/term-value
        term: &[],
        term_program: &["otty"],
        capabilities: TerminalCapabilities {
            // https://docs.otty.sh/terminal-features/images documents Kitty PNG graphics.
            graphics: Some(GraphicsProtocol::Kitty),
            // OSC 9 (body) and 777 (title/body) both create desktop notifications.
            // Shell-controlled notifications default to enabled. OSC 99 is queried.
            // https://docs.otty.sh/terminal-features/notifications
            notifications: NotificationProtocols::SIMPLE,
            // Cmd-click also bypasses app mouse capture. Otty currently ships
            // on macOS; Linux and Windows are listed as in development.
            // https://docs.otty.sh/terminal-features/cursor-and-mouse
            // https://docs.otty.sh/getting-started/installation
            osc8: HyperlinkSupport::Supported {
                open_hint: "Cmd-Click to open",
            },
            // Kitty graphics uses APC framing:
            // https://docs.otty.sh/vt/external-protocols
            supports_apc: true,
            // Bundled fallback covers classic and supplementary Nerd Font icons:
            // https://docs.otty.sh/terminal-features/unicode-and-text-styles#icons-nerd-font
            nerd_font: true,
            // Box Drawing documents Powerline, but not our U+1FBxx mosaic corners:
            // https://docs.otty.sh/terminal-features/box-drawing
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &["domterm"],
        term_program: &["domterm"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // No OSC 9 notification handler; OSC 777 is explicitly ignored.
            // https://github.com/PerBothner/DomTerm/blob/master/hlib/domterm-parser.js (handleOperatingSystemControl)
            notifications: NotificationProtocols::NONE,
            // OSC 8 links are "subtle" links: Cmd on macOS, Ctrl elsewhere.
            // Shift bypasses application mouse reporting.
            // https://github.com/PerBothner/DomTerm/blob/master/hlib/domterm.js (clickLink)
            // https://github.com/PerBothner/DomTerm/blob/master/hlib/terminal.js (_mouseHandler)
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Cmd-Shift-Click to open",
                linux: "Ctrl-Shift-Click to open",
                unknown: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
            },
            // APC payloads are consumed in SEEN_APC_STATE through the string terminator:
            // https://github.com/PerBothner/DomTerm/blob/master/hlib/domterm-parser.js
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["hterm"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // OSC 9 and OSC 777;notify both call hterm.notify (browser permissions apply).
            // https://chromium.googlesource.com/apps/libapps/+/HEAD/hterm/js/hterm_vt.js
            notifications: NotificationProtocols::SIMPLE,
            // OSC 8 spans open on click; Alt overrides application mouse reporting.
            // https://github.com/chromium/hterm/blob/master/js/hterm_text_attributes.js
            // https://github.com/chromium/hterm/blob/master/js/hterm_terminal.js (defeatMouseReports_)
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Option-Click to open",
                linux: "Alt-Click to open",
                unknown: "Alt-Click to open (Option-Click on macOS)",
            },
            // ESC _ uses parseUntilStringTerminator_ to consume APC payloads:
            // https://chromium.googlesource.com/apps/libapps/+/HEAD/hterm/js/hterm_vt.js
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["hyper"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // No built-in OSC 9/777 notification handler in Hyper's xterm.js setup;
            // notification plugins are not evidence of support in a stock install.
            // https://github.com/vercel/hyper/blob/canary/lib/components/term.tsx
            // https://xtermjs.org/docs/api/vtfeatures/#osc
            notifications: NotificationProtocols::NONE,
            // Hyper uses xterm.js's default OSC 8 handler (click, then confirmation).
            // webLinksActivationKey applies to auto-detected URLs, not OSC 8 links.
            // https://github.com/vercel/hyper/blob/canary/lib/components/term.tsx
            // https://github.com/xtermjs/xterm.js/blob/master/src/browser/OscLinkProvider.ts
            osc8: HyperlinkSupport::Supported {
                open_hint: "Click to open",
            },
            // xterm.js recognizes APC strings and silently ignores their payloads:
            // https://xtermjs.org/docs/api/vtfeatures/
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["mintty"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // OSC 9 only handles ConEmu-style taskbar progress (9;4), not messages;
            // do_cmd has no OSC 777 notification handler.
            // https://github.com/mintty/mintty/blob/master/src/termout.c (do_cmd)
            notifications: NotificationProtocols::NONE,
            // Ctrl opens links; Ctrl+Shift overrides application mouse modes.
            // https://mintty.github.io/mintty.1.html#h-Opening-files_-directories-and-URLs
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Shift-Click to open",
            },
            // ESC _ enters IGNORE_STRING for APC payloads:
            // https://github.com/mintty/mintty/blob/master/src/termout.c
            supports_apc: true,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["terminology"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // OSC 777;notify is implemented; OSC 9 messages are not. The
            // notification_escapes setting defaults to true (config.c).
            // https://github.com/borisfaure/terminology/blob/master/src/bin/termptyesc.c (_handle_xterm_777_command)
            // https://github.com/borisfaure/terminology/blob/master/src/bin/config.c
            notifications: NotificationProtocols::URXVT,
            // The OSC 8 link overlay handles primary-button clicks directly.
            // https://github.com/borisfaure/terminology/blob/master/src/bin/termio.c (_cb_link_up)
            osc8: HyperlinkSupport::Supported {
                open_hint: "Click to open",
            },
            // In 1.14.0, _handle_esc has no APC handler and leaves its payload as text:
            // https://github.com/borisfaure/terminology/blob/v1.14.0/src/bin/termptyesc.c#L4948
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["secure shellfish"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // Documents OSC 777 for remote scripts to post local notifications;
            // OSC 9 message support is not documented.
            // https://secureshellfish.app/help/terminal
            notifications: NotificationProtocols::URXVT,
            // Shift keeps clicks local instead of forwarding them to mouse-aware apps.
            // https://secureshellfish.app/help/terminal
            // https://secureshellfish.app/help/cursor-and-selection
            osc8: HyperlinkSupport::Supported {
                open_hint: "Shift-Click to open",
            },
            // APC framing is not documented; unconfirmed.
            // https://secureshellfish.app/help/terminal
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["absolutetelnet"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // OSC 9 and OSC 777 notifications were added in 14.03 (2026-08-20).
            // Older releases lack these formats; OSC 99 is always queried.
            // https://www.celestialsoftware.net/version-history/
            notifications: NotificationProtocols::SIMPLE,
            // 13.16 adds Go To URL and Ctrl-right-click to force the context menu;
            // 14.02 extends URL handling to OSC 8 hyperlinks.
            // https://www.celestialsoftware.net/version-history/
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Right-Click, then Go To URL to open",
            },
            // APC glyph queries arrived in 14.08; safety on older versions is unconfirmed.
            // https://www.celestialsoftware.net/version-history/
            supports_apc: false,
            nerd_font: false,
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["windows_terminal", "windowsterminal"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // OSC 9 handles ConEmu progress/cwd, not desktop messages. OSC 777
            // arrives in 1.26 Preview but requires compatibility.allowOSC777,
            // which defaults to false, so it is not a reliable fallback.
            // https://github.com/microsoft/terminal/blob/main/src/terminal/adapter/adaptDispatch.cpp (DoConEmuAction, DoUrxvtAction)
            // https://github.com/microsoft/terminal/blob/main/src/cascadia/TerminalSettingsModel/MTSMSettings.h
            notifications: NotificationProtocols::NONE,
            // https://learn.microsoft.com/en-us/windows/terminal/tips-and-tricks#hyperlinks
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Click to open",
            },
            // _EventSosPmApcString ignores the entire APC string:
            // https://github.com/microsoft/terminal/blob/main/src/terminal/parser/stateMachine.cpp
            supports_apc: true,
            nerd_font: false,
            // Since 1.25.2733.0, built-in U+1FB00..U+1FB94 includes all four smooth corners:
            // https://github.com/microsoft/terminal/discussions/20758
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &["xterm-ghostty"],
        term_program: &["ghostty"],
        capabilities: TerminalCapabilities {
            // https://ghostty.org/docs/features documents Kitty graphics support.
            graphics: Some(GraphicsProtocol::Kitty),
            // Both OSC 9 and OSC 777 dispatch show_desktop_notification;
            // delivery can be disabled with desktop-notifications=false.
            // https://github.com/ghostty-org/ghostty/blob/main/src/terminal/osc.zig
            // https://ghostty.org/docs/config/reference#desktop-notifications
            notifications: NotificationProtocols::SIMPLE,
            // Hints describe defaults with mouse reporting enabled, not custom bindings.
            // Ghostty needs Shift to bypass capture, then Ctrl on Linux / Cmd on macOS:
            // https://ghostty.org/docs/config/reference#mouse-shift-capture
            // https://ghostty.org/docs/config/reference#link-url
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Cmd-Shift-Click to open",
                linux: "Ctrl-Shift-Click to open",
                unknown: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
            },
            // The parser emits apc_start/apc_put/apc_end instead of printing APC payloads:
            // https://github.com/ghostty-org/ghostty/blob/main/src/terminal/Parser.zig
            supports_apc: true,
            // Bundled Nerd Font:
            // https://ghostty.org/docs/features
            nerd_font: true,
            // Built-in smooth mosaics, including all four corners:
            // https://github.com/ghostty-org/ghostty/blob/v1.0.0/src/font/sprite/Box.zig
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &["wezterm"],
        term_program: &["wezterm"],
        capabilities: TerminalCapabilities {
            // https://wezterm.org/imgcat.html documents iTerm2 inline image support.
            graphics: Some(GraphicsProtocol::Iterm2),
            // Documents OSC 9 toast messages and OSC 777;notify title/body toasts.
            // https://wezterm.org/escape-sequences.html
            notifications: NotificationProtocols::SIMPLE,
            // Shift bypasses mouse reporting and is stripped before default bindings:
            // https://wezterm.org/config/mouse.html
            osc8: HyperlinkSupport::Supported {
                open_hint: "Shift-Click to open",
            },
            // APC payloads are collected and delivered to apc_dispatch at ST:
            // https://github.com/wez/wezterm/blob/main/vtparse/src/lib.rs
            supports_apc: true,
            // https://wezterm.org/config/fonts.html
            nerd_font: true,
            // Legacy mosaics are in the nightly section, not the stable release:
            // https://wezterm.org/changelog.html#continuousnightly
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &["rio"],
        term_program: &["rio"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // Documents both OSC 9 desktop messages and OSC 777;notify.
            // OSC 9;4 progress reporting is a separate supported subcommand.
            // https://rioterm.com/docs/escape-sequence-support
            notifications: NotificationProtocols::SIMPLE,
            // OSC 8 uses Cmd-click on macOS, Alt-click elsewhere, including mouse capture.
            // https://rioterm.com/docs/features/hyperlinks
            // https://github.com/raphamorim/rio/blob/main/rio-backend/src/config/hints.rs (HintMouse::default)
            // https://github.com/raphamorim/rio/blob/main/frontends/rioterm/src/application.rs (hint_click)
            osc8: HyperlinkSupport::PlatformSpecific {
                macos: "Cmd-Click to open",
                linux: "Alt-Click to open",
                unknown: "Alt-Click to open (Cmd-Click on macOS)",
            },
            // The glyph protocol explicitly uses APC framing:
            // https://rioterm.com/docs/features/glyph-protocol
            supports_apc: true,
            // Bundled Nerd Font:
            // https://github.com/raphamorim/rio/tree/main/rio-fonts/resources
            nerd_font: true,
            // Legacy renderer covers sextants/octants, not our corners:
            // https://github.com/raphamorim/rio/blob/main/sugarloaf/src/sprite/legacy.rs
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &["xterm-kitty"],
        term_program: &["kitty"],
        capabilities: TerminalCapabilities {
            // https://sw.kovidgoyal.net/kitty/graphics-protocol/ documents PNG transfers; C=1 requires Kitty 0.20.0+.
            graphics: Some(GraphicsProtocol::Kitty),
            // handle_notification_cmd implements both legacy OSC 9 and 777;
            // OSC 99 features are negotiated separately, never assumed here.
            // https://github.com/kovidgoyal/kitty/blob/master/kitty/notifications.py
            notifications: NotificationProtocols::SIMPLE,
            // ctrl+shift+left is bound in both grabbed and ungrabbed modes:
            // https://sw.kovidgoyal.net/kitty/conf/#mouse-actions
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Shift-Click to open",
            },
            // The graphics protocol uses APC framing:
            // https://sw.kovidgoyal.net/kitty/graphics-protocol/#the-graphics-escape-code
            supports_apc: true,
            nerd_font: false,
            // https://github.com/kovidgoyal/kitty/blob/v0.20.0/kitty/fonts/box_drawing.py
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &["alacritty", "alacritty-direct"],
        term_program: &["alacritty"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // The ANSI dispatch match has no OSC 9 or 777 notification handler;
            // bell handling is not support for titled desktop messages.
            // https://github.com/alacritty/vte/blob/master/src/ansi.rs (osc_dispatch)
            notifications: NotificationProtocols::NONE,
            // Default hyperlinks need no modifier; Shift bypasses mouse reporting:
            // https://alacritty.org/config-alacritty.html#mouse
            osc8: HyperlinkSupport::Supported {
                open_hint: "Shift-Click to open",
            },
            // The VTE parser consumes APC payloads in SosPmApcString:
            // https://github.com/alacritty/vte/blob/master/src/lib.rs
            supports_apc: true,
            nerd_font: false,
            // Only sextants U+1FB00..U+1FB3B, NOT the smooth mosaic corners:
            // https://github.com/alacritty/alacritty/blob/master/extra/man/alacritty.5.scd#font
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &["foot", "foot-extra", "foot-direct"],
        term_program: &["foot"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // Documents both OSC 9 messages and OSC 777;notify, delivered via
            // foot.ini's notification command. OSC 99 is negotiated separately.
            // https://codeberg.org/dnkl/foot/src/branch/master/doc/foot-ctlseqs.7.scd
            notifications: NotificationProtocols::SIMPLE,
            // foot opens links with URL-mode jump labels, not a default mouse binding:
            // https://codeberg.org/dnkl/foot/src/branch/master/doc/foot.ini.5.scd (show-urls-launch)
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Shift-O, then type the link label to open",
            },
            // state_sos_pm_apc_string_switch ignores APC payloads:
            // https://codeberg.org/dnkl/foot/src/branch/master/vt.c
            supports_apc: true,
            nerd_font: false,
            // draw_wedge_triangle handles all four corners:
            // https://codeberg.org/dnkl/foot/src/branch/master/box-drawing.c
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &[],
        term_program: &["konsole"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // OSC 777;notify creates a KNotification. OSC 9 is for ConEmu
            // progress reporting, not iTerm-style desktop messages.
            // https://github.com/KDE/konsole/blob/master/src/Vt102Emulation.cpp (processSessionAttributeRequest)
            notifications: NotificationProtocols::URXVT,
            // OSC 8 remains unconfirmed because it is disabled by default:
            // https://github.com/Alhadis/OSC8-Adoption/
            osc8: HyperlinkSupport::Unconfirmed,
            // APC payloads go through apc_start/apc_put/apc_end:
            // https://github.com/KDE/konsole/blob/master/src/Vt102Emulation.cpp
            supports_apc: true,
            nerd_font: false,
            // drawLegacyCharacter includes U+1FB3C..U+1FB6F:
            // https://github.com/KDE/konsole/blob/master/src/characters/LineBlockCharacters.cpp
            symbols_for_legacy_computing: true,
        },
    },
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
            graphics: None,
            // Upstream VTE has no OSC 9 desktop messages. Its opt-in OSC 777
            // translation only recognizes "Command completed" as shell-postexec,
            // not arbitrary notification titles/bodies. Distribution patches
            // are not a guarantee for stock installs.
            // GNOME Terminal does not enable that translation:
            // https://github.com/GNOME/gnome-terminal/blob/master/src/terminal-screen.cc
            // Xfce likewise has no notification handler or OSC 777 opt-in:
            // https://github.com/xfce-mirror/xfce4-terminal/blob/master/terminal/terminal-screen.c
            // https://github.com/xfce-mirror/xfce4-terminal/blob/master/terminal/terminal-widget.c
            // Terminator's stock VTE setup has no OSC notification handler:
            // https://github.com/gnome-terminator/terminator/blob/master/terminatorlib/terminal.py
            // GNOME Console likewise has no OSC notification integration:
            // https://github.com/GNOME/console/blob/main/src/kgx-terminal.c
            // Tilix only hooks notification-received when a patched VTE exposes
            // that signal; its terminal name alone cannot establish support:
            // https://github.com/gnunn1/tilix/blob/master/source/gx/tilix/terminal/exvte.d (addOnNotificationReceived)
            // Ptyxis observes shell/container termprops, not arbitrary OSC notifications:
            // https://gitlab.gnome.org/GNOME/ptyxis/-/blob/main/src/ptyxis-terminal.c
            // Guake's VTE setup has no OSC notification integration:
            // https://github.com/Guake/guake/blob/master/guake/terminal.py
            // MATE Terminal likewise has no OSC notification handler:
            // https://github.com/mate-desktop/mate-terminal/blob/master/src/terminal-screen.c
            // ROXTerm likewise observes title changes, not OSC notifications:
            // https://github.com/realh/roxterm/blob/master/src/roxterm.c
            // https://gnome.pages.gitlab.gnome.org/vte/gtk3/property.Terminal.enable-legacy-osc777.html
            // https://github.com/GNOME/vte/blob/master/src/vteseq.cc
            notifications: NotificationProtocols::NONE,
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
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Click to open",
            },
            // VTE consumes APC strings in ST_IGNORE:
            // https://github.com/GNOME/vte/blob/master/src/parser.hh
            supports_apc: true,
            nerd_font: false,
            // Built-in smooth mosaics are shared by these VTE-based terminals:
            // https://github.com/GNOME/vte/blob/0.76.0/src/minifont.cc#L1090
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &[],
        term_program: &["xterm.js", "xtermjs"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // Core InputHandler registers neither OSC 9 nor 777. An embedding
            // may add handlers, but xterm.js identification alone proves neither.
            // https://github.com/xtermjs/xterm.js/blob/master/src/common/InputHandler.ts
            notifications: NotificationProtocols::NONE,
            // Default OSC 8 activation is a click followed by a confirmation dialog:
            // https://github.com/xtermjs/xterm.js/blob/master/src/browser/OscLinkProvider.ts
            // https://github.com/xtermjs/xterm.js/blob/master/src/browser/Linkifier.ts
            osc8: HyperlinkSupport::Supported {
                open_hint: "Click to open",
            },
            // APC strings are recognized and silently ignored:
            // https://xtermjs.org/docs/api/vtfeatures/
            supports_apc: true,
            nerd_font: false,
            // WebGL custom-glyph renderer includes all four smooth mosaics.
            // Embeddings that disable custom glyphs/WebGL can differ.
            // https://github.com/xtermjs/xterm.js/blob/master/addons/addon-webgl/src/customGlyphs/CustomGlyphDefinitions.ts
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &[],
        term_program: &["vscode"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // Stock xterm.js integration adds no OSC 9/777 desktop-message
            // handler. Shell integration uses OSC 9;9 for cwd, not notifications;
            // third-party notification extensions cannot be assumed installed.
            // https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/browser/xterm/xtermTerminal.ts
            // https://github.com/microsoft/vscode/blob/main/src/vs/platform/terminal/common/xterm/shellIntegrationAddon.ts
            notifications: NotificationProtocols::NONE,
            // VS Code's default modifier depends on the frontend platform. A remote
            // integrated terminal may be on a different OS even without SSH_* variables,
            // so describe both modifiers.
            // https://code.visualstudio.com/docs/terminal/basics#_links
            osc8: HyperlinkSupport::Supported {
                open_hint: "Ctrl-Click to open (Cmd-Click on macOS)",
            },
            // Its xterm.js parser recognizes APC strings and silently ignores them:
            // https://xtermjs.org/docs/api/vtfeatures/
            supports_apc: true,
            nerd_font: false,
            // Uses xterm.js's custom-glyph renderer:
            // https://code.visualstudio.com/docs/terminal/appearance#_custom-glyphs
            symbols_for_legacy_computing: true,
        },
    },
    Terminal {
        term: &[],
        term_program: &["apple_terminal"],
        capabilities: TerminalCapabilities {
            graphics: None,
            // No documented OSC 9/777 desktop-message support. Apple's alert
            // documentation describes BEL sound/flash/Dock attention instead;
            // leave OSC unconfirmed and allow the local macOS backend.
            // https://support.apple.com/guide/terminal/change-profiles-advanced-settings-trmladvn/mac
            notifications: NotificationProtocols::NONE,
            osc8: HyperlinkSupport::Unconfirmed,
            // APC glyph-query payloads appear as text in Apple Terminal (reproduction):
            // https://github.com/synthetic-lab/paintcannon/pull/47
            supports_apc: false,
            // No verified built-in Nerd Font renderer.
            nerd_font: false,
            // No verified built-in smooth-mosaic renderer.
            symbols_for_legacy_computing: false,
        },
    },
    Terminal {
        term: &[],
        term_program: &["iterm.app", "iterm2"],
        capabilities: TerminalCapabilities {
            // https://iterm2.com/documentation-images.html documents OSC 1337 File on all modern iTerm2 versions.
            graphics: Some(GraphicsProtocol::Iterm2),
            // Documents OSC 9 for posting a notification; the parser maps 9
            // to ITERM_USER_NOTIFICATION and has no OSC 777 notification entry.
            // https://iterm2.com/documentation-escape-codes.html
            // https://github.com/gnachman/iTerm2/blob/master/sources/VT100/VT100XtermParser.m
            notifications: NotificationProtocols::ITERM,
            // Cmd opens the link.
            // https://iterm2.com/documentation-one-page.html (Mouse)
            osc8: HyperlinkSupport::Supported {
                open_hint: "Cmd-Click to open",
            },
            // VT100XtermParser consumes APC strings using kAPCMode:
            // https://github.com/gnachman/iTerm2/blob/master/sources/VT100/VT100XtermParser.m
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
            graphics: None,
            // XTerm's supported OSC commands include neither 9 nor 777;
            // visual/urgency bells are not titled notification messages.
            // https://github.com/ThomasDickey/xterm-snapshots/blob/master/ctlseqs.txt (Operating System Commands)
            notifications: NotificationProtocols::NONE,
            osc8: HyperlinkSupport::Unconfirmed,
            // APC payloads are explicitly ignored:
            // https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Application-Program-Command-functions
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
            graphics: None,
            // No built-in OSC 9 messages. In rxvt-unicode OSC 777 dispatches
            // to Perl extensions, so notifications need a separately installed
            // and enabled notify extension; the terminal name is insufficient.
            // The bundled extensions and default set do not include notify.
            // https://github.com/exg/rxvt-unicode/blob/master/src/command.C (URxvt_perl)
            // https://github.com/exg/rxvt-unicode/blob/master/doc/rxvt.7.man.in (OSC 777)
            // https://github.com/exg/rxvt-unicode/tree/master/src/perl
            // https://man.archlinux.org/man/extra/rxvt-unicode/urxvt.1.en (perl-ext-common)
            notifications: NotificationProtocols::NONE,
            osc8: HyperlinkSupport::Unconfirmed,
            // process_escape_seq has no APC case, leaving its payload as printable text:
            // https://github.com/exg/rxvt-unicode/blob/master/src/command.C
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
    use crate::terminal_capabilities::Capabilities;

    #[test]
    fn every_terminal_name_resolves_to_its_definition() {
        use crate::terminal_capabilities::environment::{
            resolve_hyperlink_support, ClientPlatform, Environment,
        };

        for terminal in TERMINALS {
            let expected = Capabilities {
                notifications: terminal.capabilities.notifications,
                local: false,
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
