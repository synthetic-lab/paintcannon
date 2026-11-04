//! Local fallback only: never run these on behalf of a remote SSH client.
use super::NotificationOptions;
use crate::subprocess::command_output_with_timeout;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Backend {
    Osascript(PathBuf),
    NotifySend(PathBuf),
}

pub(super) fn detect() -> Option<Backend> {
    if cfg!(target_os = "macos") {
        executable("osascript").map(Backend::Osascript)
    } else if cfg!(target_os = "linux")
        && std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        && (std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some())
    {
        executable("notify-send").map(Backend::NotifySend)
    } else {
        None
    }
}

fn executable(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|path| path.join(name))
        .find(|path| {
            path.metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

fn command(backend: &Backend, options: &NotificationOptions, previous: Option<u32>) -> Command {
    match backend {
        Backend::Osascript(path) => {
            let mut cmd = Command::new(path);
            // Arguments, not interpolated AppleScript: quotes/backslashes/newlines
            // remain data. https://developer.apple.com/library/archive/documentation/LanguagesUtilities/Conceptual/MacAutomationScriptingGuide/DisplayNotifications.html
            cmd.args(["-e", "on run argv\ndisplay notification (item 2 of argv) with title (item 1 of argv)\nend run", "--", &options.title, &options.body]);
            cmd
        }
        Backend::NotifySend(path) => {
            let mut cmd = Command::new(path);
            cmd.args(["--app-name=PaintCannon", "--print-id"]);
            if let Some(id) = previous {
                cmd.arg(format!("--replace-id={id}"));
            }
            if options.silent == Some(true) {
                cmd.arg("--hint=boolean:suppress-sound:true");
            }
            if options.require_interaction {
                cmd.arg("--expire-time=0");
            }
            // libnotify bodies may be markup. Preserve plain text semantics.
            cmd.arg("--").arg(&options.title).arg(
                options
                    .body
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            );
            cmd
        }
    }
}

pub(super) fn show(
    backend: &Backend,
    options: &NotificationOptions,
    previous: Option<u32>,
) -> io::Result<Option<u32>> {
    let output = run(command(backend, options, previous))?;
    match backend {
        Backend::NotifySend(_) => String::from_utf8_lossy(&output)
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(io::Error::other),
        Backend::Osascript(_) => Ok(None),
    }
}

pub(super) fn supports_close(backend: &Backend) -> bool {
    matches!(backend, Backend::NotifySend(_)) && executable("gdbus").is_some()
}

pub(super) fn close(backend: &Backend, id: Option<u32>) -> io::Result<()> {
    if let (Backend::NotifySend(_), Some(id), Some(gdbus)) = (backend, id, executable("gdbus")) {
        let mut cmd = Command::new(gdbus);
        cmd.args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.Notifications",
            "--object-path",
            "/org/freedesktop/Notifications",
            "--method",
            "org.freedesktop.Notifications.CloseNotification",
        ]);
        cmd.arg(id.to_string());
        run(cmd)?;
    }
    Ok(())
}

fn run(cmd: Command) -> io::Result<Vec<u8>> {
    let output = command_output_with_timeout(cmd, Duration::from_secs(2))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "notification command timed out"))?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "notification command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_commands_keep_untrusted_text_out_of_code_and_options() {
        let options = NotificationOptions {
            title: "--help; $(touch /tmp/no)".into(),
            body: "\" & <b>hello</b>\nworld\\".into(),
            tag: String::new(),
            silent: Some(true),
            require_interaction: true,
        };
        let mac = command(&Backend::Osascript("osascript".into()), &options, None);
        let args: Vec<_> = mac.get_args().map(|s| s.to_str().unwrap()).collect();
        assert_eq!(
            &args[2..],
            ["--", options.title.as_str(), options.body.as_str()]
        );
        assert!(!args[1].contains(&options.body));
        let linux = command(
            &Backend::NotifySend("notify-send".into()),
            &options,
            Some(42),
        );
        let args: Vec<_> = linux.get_args().map(|s| s.to_str().unwrap()).collect();
        assert!(args.contains(&"--replace-id=42"));
        assert_eq!(
            &args[args.len() - 3..],
            [
                "--",
                options.title.as_str(),
                "\" &amp; &lt;b&gt;hello&lt;/b&gt;\nworld\\"
            ]
        );
    }
    #[test]
    fn failed_or_missing_native_commands_are_errors_not_panics() {
        assert!(run(Command::new("/paintcannon/nonexistent-notifier")).is_err());
        let mut failed = Command::new("sh");
        failed.args(["-c", "exit 1"]);
        assert!(run(failed).is_err());
    }
}
