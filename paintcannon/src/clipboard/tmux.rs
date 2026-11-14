use std::process::Command;
use std::time::Duration;

use crate::subprocess::command_output_with_timeout;

pub(super) fn prepare_rich_paste() -> bool {
    let Some(pane) = std::env::var_os("TMUX_PANE") else {
        return false;
    };
    let mut command = Command::new("tmux");
    // focus-events is a server option. Use the server selected by our TMUX
    // environment; do not change the user's setting.
    // https://man.openbsd.org/tmux#focus-events
    command.args(["show-options", "-sv", "focus-events"]);
    if !query_focus_events(command) {
        return false;
    }

    // A blur can arrive after zooming another pane or switching windows hides us.
    // `on` drops the ensuing reset, leaving rich paste enabled in unrelated panes.
    // Require pane-local `all` before enabling the outer mode so cleanup still passes.
    // https://man.openbsd.org/tmux#allow-passthrough
    let mut command = Command::new("tmux");
    command
        .args(["set-option", "-p", "-t"])
        .arg(pane)
        .args(["allow-passthrough", "all"]);
    enable_hidden_passthrough(command)
}

fn enable_hidden_passthrough(command: Command) -> bool {
    command_output_with_timeout(command, Duration::from_millis(250))
        .ok()
        .flatten()
        .is_some_and(|output| output.status.success())
}

fn query_focus_events(command: Command) -> bool {
    command_output_with_timeout(command, Duration::from_millis(250))
        .ok()
        .flatten()
        .is_some_and(|output| output.status.success() && output.stdout.trim_ascii() == b"on")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn focus_events_requires_success_and_an_unambiguous_on_value() {
        assert!(query_focus_events(shell("printf 'on\\n'")));
        assert!(query_focus_events(shell("printf 'on\\r\\n'")));
        for script in [
            "printf 'off\\n'",
            "exit 0",
            "printf 'focus-events on\\n'",
            "printf 'on\\noff\\n'",
            "printf 'on\\n'; exit 1",
            "printf 'on\\n' >&2",
        ] {
            assert!(!query_focus_events(shell(script)), "{script}");
        }
    }

    #[test]
    fn focus_events_query_errors_and_timeouts_fail_closed() {
        assert!(!query_focus_events(Command::new(
            "/nonexistent-paintcannon-test-tmux"
        )));
        // Even an 'on' value is not trustworthy until the command exits successfully.
        assert!(!query_focus_events(shell("printf 'on\\n'; exec sleep 30")));
    }

    #[test]
    fn hidden_passthrough_requires_success_and_fails_closed_on_errors_or_timeouts() {
        assert!(enable_hidden_passthrough(shell("exit 0")));
        assert!(!enable_hidden_passthrough(shell("exit 1")));
        assert!(!enable_hidden_passthrough(Command::new(
            "/nonexistent-paintcannon-test-tmux"
        )));
        assert!(!enable_hidden_passthrough(shell("exec sleep 30")));
    }
}
