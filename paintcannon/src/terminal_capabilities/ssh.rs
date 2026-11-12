use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub(super) fn from_environment<'a>(mut value: impl FnMut(&str) -> Option<&'a [u8]>) -> bool {
    ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
        .into_iter()
        .any(|key| value(key).is_some_and(|value| !value.is_empty()))
}

pub(super) fn for_process(pid: u32) -> Option<bool> {
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_environ(UpdateKind::Always),
    );
    let environment = system.process(pid)?.environ();
    // sysinfo returns an empty environment when access is denied. That cannot
    // establish that this process is local.
    if environment.is_empty() {
        return None;
    }
    Some(from_environment(|key| {
        environment.iter().find_map(|entry| {
            entry
                .as_encoded_bytes()
                .strip_prefix(key.as_bytes())?
                .strip_prefix(b"=")
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_detection_requires_an_exact_key_and_nonempty_value() {
        assert!(!from_environment(|_| None));
        for key in ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"] {
            for (value, expected) in [(b"".as_slice(), false), (b"remote", true), (b"\xff", true)] {
                assert_eq!(
                    from_environment(|name| (name == key).then_some(value)),
                    expected,
                    "{key}={value:?}"
                );
            }
        }
        for key in ["NOT_SSH_CLIENT", "SSH_CLIENT_EXTRA", "ssh_client"] {
            assert!(!from_environment(|name| (name == key).then_some(b"remote")));
        }
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn reads_ssh_markers_from_processes() {
        use std::io::Read;
        use std::process::{Command, Stdio};

        for (key, value, expected) in [
            ("SSH_CONNECTION", "", false),
            ("SSH_CLIENT", "", false),
            ("SSH_TTY", "", false),
            ("SSH_CONNECTION", "192.0.2.1 1234 192.0.2.2 22", true),
            ("SSH_CLIENT", "192.0.2.1 1234 22", true),
            ("SSH_TTY", "/dev/pts/1", true),
            ("NOT_SSH_CLIENT", "remote", false),
        ] {
            // A newly spawned process can briefly expose an empty environment.
            // Signal readiness, then keep the same process alive until stdin closes.
            let mut child = Command::new("sh")
                .args(["-c", "printf ready; read -r line; exit 0"])
                .env_clear()
                .env("TERM", "xterm-ghostty")
                .env(key, value)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut ready = [0; 5];
            let readiness = child.stdout.take().unwrap().read_exact(&mut ready);
            let result = for_process(child.id());
            // Reap the child before asserting, including on handshake failure.
            drop(child.stdin.take());
            let status = child.wait().unwrap();
            readiness.unwrap();
            assert_eq!(&ready, b"ready");
            assert!(status.success());
            assert_eq!(result, Some(expected), "{key}={value}");
        }
    }
}
