//! Live client capabilities, not tmux's saved global/session environment.
use super::{
    capabilities::{Capabilities, HyperlinkCapability},
    environment::{ClientPlatform, Environment},
    environment_markers::MARKERS,
    ssh,
};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn client_format() -> String {
    // W/P loops include linked windows: every client whose session contains this
    // pane matters, even if the pane isn't currently the client's active window.
    let mut format =
        "#{client_control_mode}\t#{W:#{P:#{pane_id} }}\t#{client_termname}\t#{client_termtype}"
            .to_string();
    for name in std::iter::once("TERM_PROGRAM").chain(MARKERS.iter().map(|marker| marker.name)) {
        // I/e reads the attached client's environment (added after tmux 3.7), unlike a bare
        // #{TERM_PROGRAM}, which reads the stale session/global environment.
        // Older tmux leaves these fields empty; client_termname and client_termtype
        // still provide live identity, not stale server environment values.
        // https://man.openbsd.org/tmux#FORMATS
        format.push_str(&format!("\t#{{I/e:{name}}}"));
    }
    format.push_str("\t#{client_termfeatures}\t#{client_pid}");
    format
}

pub(super) fn client_capabilities(pane: &str) -> Vec<Capabilities> {
    if pane.is_empty() {
        return Vec::new();
    }
    // Inspect attached processes, not tmux's saved session environment. This
    // also works on tmux releases without the I/e format modifier.
    client_output()
        .and_then(|output| parse_clients(&output, pane, ssh::for_process))
        .unwrap_or_default()
}

fn parse_clients(
    output: &str,
    pane: &str,
    mut uses_ssh: impl FnMut(u32) -> Option<bool>,
) -> Option<Vec<Capabilities>> {
    let mut clients = Vec::new();
    let features_index = 5 + MARKERS.len();
    for line in output.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != features_index + 2
            || fields
                .iter()
                .any(|field| field.chars().any(char::is_control))
        {
            return None;
        }
        if fields[0] == "1" || !fields[1].split_whitespace().any(|id| id == pane) {
            continue;
        }
        let program = if fields[4].is_empty() {
            // tmux records XTVERSION replies such as "WezTerm 20240203-..." and
            // "foot(1.18.0)". Match the product through the same terminal table.
            // Unknown product names remain unknown, even if a version is present.
            // https://github.com/tmux/tmux/blob/3.7/tty-keys.c#L1607-L1669
            fields[3].split([' ', '(']).next().unwrap_or("")
        } else {
            fields[4]
        };
        let mut pairs = vec![("TERM", fields[2]), ("TERM_PROGRAM", program)];
        let client_environment = MARKERS
            .iter()
            .map(|marker| marker.name)
            .zip(fields[5..features_index].iter().copied());
        pairs.extend(client_environment);
        // tmux only tells us which clients to check for SSH. Local clients use
        // the host OS; neither TERM nor a terminal version report identifies it.
        let pid = fields[features_index + 1].parse().ok()?;
        let platform = match uses_ssh(pid) {
            Some(false) => ClientPlatform::local(),
            Some(true) | None => ClientPlatform::Unknown,
        };
        let mut capabilities = Environment::from_pairs(&pairs)
            .with_platform(platform)
            .capabilities();
        // OSC 8 belongs to tmux's cell metadata, not DCS passthrough (which loses
        // links on pane redraw). Require native forwarding as well as outer support.
        // https://github.com/Alhadis/OSC8-Adoption/#terminal-multiplexers
        if !fields[features_index]
            .split(',')
            .any(|feature| feature == "hyperlinks")
        {
            capabilities.osc8 = HyperlinkCapability::Unconfirmed;
        }
        clients.push(capabilities);
    }
    Some(clients)
}

fn client_output() -> Option<String> {
    const MAX_OUTPUT: u64 = 1024 * 1024;
    let mut child = Command::new("tmux")
        .args(["list-clients", "-F", &client_format()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    // Drain while the command runs, so many clients/windows cannot fill the pipe
    // and deadlock the timeout wait. Bound the output and always reap both workers.
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        stdout.take(MAX_OUTPUT + 1).read_to_string(&mut text).ok()?;
        (text.len() <= MAX_OUTPUT as usize).then_some(text)
    });
    let deadline = Instant::now() + Duration::from_millis(300);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let output = reader.join().ok().flatten();
    if success {
        output
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL_GHOSTTY_HINT: &str = if cfg!(target_os = "macos") {
        "Cmd-Shift-Click to open"
    } else if cfg!(target_os = "linux") {
        "Ctrl-Shift-Click to open"
    } else {
        "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
    };

    fn parse_clients(output: &str, pane: &str) -> Option<Vec<Capabilities>> {
        super::parse_clients(output, pane, |_| Some(false))
    }

    fn client(panes: &str, term: &str, program: &str) -> String {
        client_with_environment(panes, term, &[("TERM_PROGRAM", program)])
    }

    fn client_with_environment(panes: &str, term: &str, environment: &[(&str, &str)]) -> String {
        let mut fields = vec!["0", panes, term, ""];
        for name in std::iter::once("TERM_PROGRAM").chain(MARKERS.iter().map(|marker| marker.name))
        {
            fields.push(
                environment
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map_or("", |(_, value)| *value),
            );
        }
        fields.push("hyperlinks");
        fields.push("42");
        fields.join("\t") + "\n"
    }

    #[test]
    fn local_ghostty_uses_host_platform_without_tmux_environment_support() {
        let local = client("%1 ", "xterm-ghostty", "");
        let expected = match ClientPlatform::local() {
            ClientPlatform::MacOs => "Cmd-Shift-Click to open",
            ClientPlatform::Linux => "Ctrl-Shift-Click to open",
            ClientPlatform::Unknown => "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)",
        };
        assert_eq!(
            parse_clients(&local, "%1").unwrap()[0].osc8,
            HyperlinkCapability::Confirmed {
                open_hint: expected
            }
        );
    }

    #[test]
    fn multiple_clients_hide_the_hint_only_for_sessions_containing_the_pane() {
        let local = client("%1 ", "xterm-ghostty", "");
        let remote = local.replace("\t42\n", "\t43\n");
        for output in [local.clone() + &remote, remote.clone() + &local] {
            let clients = super::parse_clients(&output, "%1", |pid| Some(pid == 43)).unwrap();
            assert_eq!(
                Environment::from_pairs(&[])
                    .with_clients(clients)
                    .capabilities()
                    .osc8,
                HyperlinkCapability::ConfirmedButUnhintable
            );
        }
        // A remote client in a different session must not affect this pane.
        let output = local + &remote.replace("%1 ", "%2 ");
        let clients = super::parse_clients(&output, "%1", |pid| {
            assert_eq!(pid, 42);
            Some(false)
        })
        .unwrap();
        assert_eq!(
            Environment::from_pairs(&[])
                .with_clients(clients)
                .capabilities()
                .osc8,
            HyperlinkCapability::Confirmed {
                open_hint: LOCAL_GHOSTTY_HINT
            }
        );
    }

    #[test]
    fn unreadable_client_environment_cannot_establish_a_local_platform() {
        let local = client("%1 ", "xterm-ghostty", "");
        let clients = super::parse_clients(&local, "%1", |_| None).unwrap();
        assert_eq!(
            Environment::from_pairs(&[])
                .with_clients(clients)
                .capabilities()
                .osc8,
            HyperlinkCapability::Confirmed {
                open_hint: "Ctrl-Shift-Click to open (Cmd-Shift-Click on macOS)"
            }
        );
    }

    #[test]
    fn osc8_requires_both_outer_support_and_tmux_forwarding_for_every_client() {
        let supported = client("%1 ", "xterm-ghostty", "");
        assert!(matches!(
            parse_clients(&supported, "%1").unwrap()[0].osc8,
            HyperlinkCapability::Confirmed { .. }
        ));
        let disabled = supported.replace("hyperlinks", "RGB,clipboard");
        assert_eq!(
            parse_clients(&disabled, "%1").unwrap()[0].osc8,
            HyperlinkCapability::Unconfirmed
        );
        let unknown = client("%1 ", "xterm-256color", "");
        assert_eq!(
            parse_clients(&unknown, "%1").unwrap()[0].osc8,
            HyperlinkCapability::Unconfirmed
        );
        let mixed = parse_clients(&(supported.clone() + &disabled), "%1").unwrap();
        assert_eq!(
            Environment::from_pairs(&[])
                .with_clients(mixed)
                .capabilities()
                .osc8,
            HyperlinkCapability::Unconfirmed
        );
        let modern = parse_clients(&(supported.clone() + &supported), "%1").unwrap();
        assert_eq!(
            Environment::from_pairs(&[])
                .with_clients(modern)
                .capabilities()
                .osc8,
            HyperlinkCapability::ConfirmedButUnhintable
        );
    }

    #[test]
    fn uses_both_live_client_identifiers_and_filters_other_sessions() {
        let output = client("%1 %2 ", "xterm-256color", "ghostty")
            + &client("%1 ", "wezterm", "")
            + &client("%3 ", "xterm-256color", "Apple_Terminal");
        assert_eq!(
            parse_clients(&output, "%1"),
            Some(vec![
                Capabilities {
                    osc8: HyperlinkCapability::Confirmed {
                        open_hint: LOCAL_GHOSTTY_HINT
                    },
                    nerd_font: true,
                    symbols_for_legacy_computing: true
                },
                Capabilities {
                    osc8: HyperlinkCapability::Confirmed {
                        open_hint: "Shift-Click to open"
                    },
                    nerd_font: true,
                    symbols_for_legacy_computing: false
                },
            ])
        );
        assert_eq!(parse_clients(&output, "%9"), Some(vec![]));
    }

    #[test]
    fn old_tmux_unknown_terms_fail_closed_and_control_clients_are_ignored() {
        let output = client("%1 ", "xterm-256color", "")
            + &client("%1 ", "xterm-ghostty", "").replacen("0\t", "1\t", 1);
        assert_eq!(
            parse_clients(&output, "%1"),
            Some(vec![Capabilities::default()])
        );
        assert_eq!(parse_clients("truncated reply", "%1"), None);
    }

    #[test]
    fn unidentified_clients_use_their_own_markers() {
        for (name, value, hint, legacy) in [
            (
                "KITTY_WINDOW_ID",
                "1",
                Some("Ctrl-Shift-Click to open"),
                true,
            ),
            ("WT_SESSION", "session", Some("Ctrl-Click to open"), true),
            ("VTE_VERSION", "7600", Some("Ctrl-Click to open"), true),
            ("VTE_VERSION", "4999", None, true),
            ("KONSOLE_DBUS_SESSION", "/Sessions/1", None, true),
            ("GNOME_TERMINAL_SCREEN", "/screen/1", None, true),
            ("TERMINATOR_UUID", "session", None, true),
            ("COLORTERM", "xfce4-terminal", None, true),
            ("COLORTERM", "truecolor", None, false),
        ] {
            let expected = Capabilities {
                osc8: hint.map_or(HyperlinkCapability::Unconfirmed, |open_hint| {
                    HyperlinkCapability::Confirmed { open_hint }
                }),
                nerd_font: false,
                symbols_for_legacy_computing: legacy,
            };
            let environment = [(name, value)];
            let output = client_with_environment("%1 ", "xterm-256color", &environment);
            assert_eq!(parse_clients(&output, "%1"), Some(vec![expected]), "{name}");
            assert_eq!(
                Environment::from_pairs(&environment).capabilities(),
                expected,
                "local: {name}"
            );
        }
    }

    #[test]
    fn client_format_queries_shared_markers_in_parser_order() {
        let format = client_format();
        let fields: Vec<_> = format.split('\t').collect();
        assert_eq!(fields.len(), 7 + MARKERS.len());
        assert_eq!(fields[4], "#{I/e:TERM_PROGRAM}");
        for (index, marker) in MARKERS.iter().enumerate() {
            assert_eq!(fields[5 + index], format!("#{{I/e:{}}}", marker.name));
        }
        assert_eq!(fields[5 + MARKERS.len()], "#{client_termfeatures}");
        assert_eq!(fields[6 + MARKERS.len()], "#{client_pid}");
    }

    #[test]
    fn version_reports_identify_live_clients_without_client_environment_support() {
        for (report, nerd, legacy, hint) in [
            (
                "WezTerm 20240203-110809-5046fc22",
                true,
                false,
                "Shift-Click to open",
            ),
            ("ghostty 1.0.1", true, true, LOCAL_GHOSTTY_HINT),
            (
                "foot(1.18.0)",
                false,
                true,
                "Ctrl-Shift-O, then type the link label to open",
            ),
            ("unknown 1.0", false, false, ""),
            ("NotWezTerm 1.0", false, false, ""),
        ] {
            let output = client("%1 ", "xterm-256color", "").replacen(
                "xterm-256color\t",
                &format!("xterm-256color\t{report}"),
                1,
            );
            assert_eq!(
                parse_clients(&output, "%1"),
                Some(vec![Capabilities {
                    osc8: if legacy || nerd {
                        HyperlinkCapability::Confirmed { open_hint: hint }
                    } else {
                        HyperlinkCapability::Unconfirmed
                    },
                    nerd_font: nerd,
                    symbols_for_legacy_computing: legacy,
                }]),
                "{report}"
            );
        }
    }
}
