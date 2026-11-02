//! Capture command output without blocking on full pipes or leaking reader threads.
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

pub(crate) fn command_output_with_timeout(
    mut command: Command,
    timeout: Duration,
) -> io::Result<Option<Output>> {
    let deadline = Instant::now() + timeout;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let result = collect_output(&mut child, deadline);
    if !matches!(result, Ok(Some(_))) {
        // The private process group also covers shell wrappers whose children
        // might otherwise keep the pipes open after the wrapper exits.
        unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
        let _ = child.kill();
        child.wait()?;
    }
    result
}

fn collect_output(child: &mut Child, deadline: Instant) -> io::Result<Option<Output>> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout pipe"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("missing stderr pipe"))?;
    set_nonblocking(&stdout)?;
    set_nonblocking(&stderr)?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut out_eof = false;
    let mut err_eof = false;
    let mut status = None;
    loop {
        if status.is_none() {
            status = child.try_wait()?;
        }
        if out_eof && err_eof {
            if let Some(status) = status {
                return Ok(Some(Output {
                    status,
                    stdout: out,
                    stderr: err,
                }));
            }
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        let mut descriptors = [
            libc::pollfd {
                fd: if out_eof { -1 } else { stdout.as_raw_fd() },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: if err_eof { -1 } else { stderr.as_raw_fd() },
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // Check child exit at least every 10ms even if it has closed both pipes.
        let millis = remaining.as_nanos().div_ceil(1_000_000).min(10) as i32;
        let ready = unsafe {
            libc::poll(
                descriptors.as_mut_ptr(),
                descriptors.len() as libc::nfds_t,
                millis,
            )
        };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        // Read a bounded chunk from each pipe per iteration so a continuous
        // writer cannot starve the other pipe or the deadline check.
        if !out_eof {
            out_eof = read_chunk(&mut stdout, &mut out)?;
        }
        if !err_eof {
            err_eof = read_chunk(&mut stderr, &mut err)?;
        }
    }
}

fn set_nonblocking(pipe: &impl AsRawFd) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    // These descriptors belong exclusively to our ChildStdout/ChildStderr.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn read_chunk(pipe: &mut impl Read, output: &mut Vec<u8>) -> io::Result<bool> {
    let mut bytes = [0; 16 * 1024];
    match pipe.read(&mut bytes) {
        Ok(0) => Ok(true),
        Ok(count) => {
            output.extend_from_slice(&bytes[..count]);
            Ok(false)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drains_both_pipes_while_running() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf '%262144s' x; printf '%262144s' y >&2"]);
        let output = command_output_with_timeout(command, Duration::from_secs(2))
            .unwrap()
            .expect("output larger than pipe capacity must not time out");
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 262144);
        assert_eq!(output.stderr.len(), 262144);
        assert_eq!(output.stdout.last(), Some(&b'x'));
        assert_eq!(output.stderr.last(), Some(&b'y'));
    }

    #[test]
    fn times_out_and_reaps_stalled_child_even_with_closed_pipes() {
        for close_pipes in [false, true] {
            let path = std::env::temp_dir()
                .join(format!("paintcannon-stalled-{}.pid", std::process::id()));
            let script = format!(
                "printf '%s' \"$$\" > \"$PID_FILE\"; {} exec sleep 60",
                if close_pipes { "exec 1>&- 2>&-;" } else { "" },
            );
            let mut command = Command::new("sh");
            command.env("PID_FILE", &path).args(["-c", &script]);
            let start = Instant::now();
            let result = command_output_with_timeout(command, Duration::from_millis(250));
            let pid: libc::pid_t = std::fs::read_to_string(&path).unwrap().parse().unwrap();
            std::fs::remove_file(path).unwrap();
            assert!(result.unwrap().is_none());
            assert!(start.elapsed() < Duration::from_secs(2));
            // ECHILD proves the subprocess was reaped, not just signalled.
            assert_eq!(
                unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
                -1
            );
            assert_eq!(
                io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
        }
    }

    #[test]
    fn preserves_output_and_nonzero_exit_status() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf out; printf err >&2; exit 7"]);
        let output = command_output_with_timeout(command, Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
    }

    #[test]
    fn missing_command_returns_spawn_error() {
        let command = Command::new("/paintcannon-test/no-such-command");
        assert_eq!(
            command_output_with_timeout(command, Duration::from_secs(2))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
}
