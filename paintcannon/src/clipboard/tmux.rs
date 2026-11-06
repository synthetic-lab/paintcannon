use std::process::Command;
use std::time::Duration;

use crate::subprocess::command_output_with_timeout;

pub(super) fn focus_events_enabled() -> bool {
    let mut command = Command::new("tmux");
    // focus-events is a server option. Use the server selected by our TMUX
    // environment; do not change the user's setting.
    // https://man.openbsd.org/tmux#focus-events
    command.args(["show-options", "-sv", "focus-events"]);
    query_focus_events(command)
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
}
