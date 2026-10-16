//! Live client capabilities, not tmux's saved global/session environment.
use super::{capabilities::Capabilities, environment::Environment};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MARKERS: &[&str] = &[
    "TERM_PROGRAM",
    "KITTY_WINDOW_ID",
    "KONSOLE_DBUS_SESSION",
    "VTE_VERSION",
    "GNOME_TERMINAL_SCREEN",
    "TERMINATOR_UUID",
    "COLORTERM",
];

fn client_format() -> String {
    // W/P loops include linked windows: every client whose session contains this
    // pane matters, even if the pane isn't currently the client's active window.
    let mut format =
        "#{client_control_mode}\t#{W:#{P:#{pane_id} }}\t#{client_termname}\t#{client_termtype}"
            .to_string();
    for marker in MARKERS {
        // I/e reads the attached client's environment (added after tmux 3.7), unlike a bare
        // #{TERM_PROGRAM}, which reads the stale session/global environment.
        // Older tmux leaves these fields empty; client_termname and client_termtype
        // still provide live identity, not stale server environment values.
        // https://man.openbsd.org/tmux#FORMATS
        format.push_str(&format!("\t#{{I/e:{marker}}}"));
    }
    format
}

pub(super) fn client_capabilities(pane: &str) -> Vec<Capabilities> {
    if pane.is_empty() {
        return Vec::new();
    }
    client_output()
        .and_then(|output| parse_clients(&output, pane))
        .unwrap_or_default()
}

fn parse_clients(output: &str, pane: &str) -> Option<Vec<Capabilities>> {
    let mut clients = Vec::new();
    for line in output.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 + MARKERS.len()
            || fields
                .iter()
                .any(|field| field.chars().any(char::is_control))
        {
            return None;
        }
        if fields[0] == "1" || !fields[1].split_whitespace().any(|id| id == pane) {
            continue;
        }
        let mut pairs = vec![("TERM", fields[2])];
        pairs.extend(MARKERS.iter().copied().zip(fields[4..].iter().copied()));
        if fields[4].is_empty() {
            // tmux records XTVERSION replies such as "WezTerm 20240203-..." and
            // "foot(1.18.0)". Match the product through the same terminal table.
            // Unknown product names remain unknown, even if a version is present.
            // https://github.com/tmux/tmux/blob/3.7/tty-keys.c#L1607-L1669
            let program = fields[3].split([' ', '(']).next().unwrap_or("");
            pairs.push(("TERM_PROGRAM", program));
        }
        clients.push(Environment::from_pairs(&pairs).capabilities());
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

    fn client(panes: &str, term: &str, program: &str) -> String {
        format!("0\t{panes}\t{term}\t\t{program}\t\t\t\t\t\t\n")
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
                    nerd_font: true,
                    symbols_for_legacy_computing: true
                },
                Capabilities {
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
        let output = "0\t%1 \txterm-256color\t\t\t\t\t7600\t\t\t\n";
        assert_eq!(
            parse_clients(output, "%1"),
            Some(vec![Capabilities {
                nerd_font: false,
                symbols_for_legacy_computing: true,
            }])
        );
    }

    #[test]
    fn version_reports_identify_live_clients_without_client_environment_support() {
        for (report, nerd, legacy) in [
            ("WezTerm 20240203-110809-5046fc22", true, false),
            ("ghostty 1.0.1", true, true),
            ("foot(1.18.0)", false, true),
            ("unknown 1.0", false, false),
            ("NotWezTerm 1.0", false, false),
        ] {
            let output = format!("0\t%1 \txterm-256color\t{report}\t\t\t\t\t\t\t\n");
            assert_eq!(
                parse_clients(&output, "%1"),
                Some(vec![Capabilities {
                    nerd_font: nerd,
                    symbols_for_legacy_computing: legacy,
                }]),
                "{report}"
            );
        }
    }
}
