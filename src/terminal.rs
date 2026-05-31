use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::AsRawFd;

use napi_derive::napi;

#[napi(object)]
#[derive(Clone, Copy)]
pub struct TerminalSize {
    pub cols: u32,
    pub rows: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

pub(crate) fn reset_terminal() {
    let mut out = io::stdout().lock();
    let _ = write_synchronized_output_end(&mut out);
    let _ = write_pointer_shape(&mut out, None);
    let _ = write!(out, "\x1b[0m\x1b[?7h\x1b[?25h\n");
    let _ = out.flush();
}

pub(crate) fn copy_text_to_clipboard(text: &str) {
    if text.is_empty() || !stdout_is_terminal() {
        return;
    }

    let payload = base64_encode(text.as_bytes());
    let mut out = io::stdout().lock();
    if inside_tmux() {
        let _ = write_tmux_passthrough(&mut out, format!("\x1b]52;c;{payload}\x07").as_bytes());
    } else {
        let _ = write!(out, "\x1b]52;c;{payload}\x07");
    }
    let _ = out.flush();
}

pub(crate) fn stdout_is_terminal() -> bool {
    io::stdout().is_terminal()
}

pub(crate) fn write_synchronized_output_begin(out: &mut impl Write) -> io::Result<()> {
    if !stdout_is_terminal() {
        return Ok(());
    }

    if inside_tmux() {
        return Ok(());
    }

    write!(out, "\x1b[?2026h")
}

pub(crate) fn write_synchronized_output_end(out: &mut impl Write) -> io::Result<()> {
    if !stdout_is_terminal() {
        return Ok(());
    }

    if inside_tmux() {
        return Ok(());
    }

    write!(out, "\x1b[?2026l")?;
    Ok(())
}

pub(crate) fn reset_pointer_shape() {
    let mut out = io::stdout().lock();
    let _ = write_pointer_shape(&mut out, None);
    let _ = out.flush();
}

pub(crate) fn write_pointer_shape(out: &mut impl Write, shape: Option<&str>) -> io::Result<()> {
    if !stdout_is_terminal() {
        return Ok(());
    }

    let shape = shape.unwrap_or("");
    if inside_tmux() {
        write_tmux_pointer_shape(out, shape)
    } else {
        let sequence = format!("\x1b]22;{shape}\x1b\\");
        out.write_all(sequence.as_bytes())
    }
}

fn write_tmux_pointer_shape(out: &mut impl Write, shape: &str) -> io::Result<()> {
    write_tmux_passthrough(out, format!("\x1b]22;{shape}\x07").as_bytes())
}

fn write_tmux_passthrough(out: &mut impl Write, sequence: &[u8]) -> io::Result<()> {
    let _ = allow_tmux_passthrough();
    encode_tmux_passthrough(out, sequence)
}

fn encode_tmux_passthrough(out: &mut impl Write, sequence: &[u8]) -> io::Result<()> {
    out.write_all(b"\x1bPtmux;")?;
    for byte in sequence {
        if *byte == 0x1b {
            out.write_all(b"\x1b\x1b")?;
        } else {
            out.write_all(&[*byte])?;
        }
    }
    out.write_all(b"\x1b\\")
}

fn allow_tmux_passthrough() -> bool {
    static ALLOWED: OnceLock<bool> = OnceLock::new();
    *ALLOWED.get_or_init(|| {
        if !inside_tmux() {
            return false;
        }

        let mut show_command = Command::new("tmux");
        show_command.args(["show", "-Ap", "allow-passthrough"]);
        let show = command_output_with_timeout(show_command, Duration::from_secs(2));
        if let Ok(output) = show {
            if let Some(output) = output {
                let value = String::from_utf8_lossy(&output.stdout);
                let value = value.trim();
                if value.ends_with(" on") || value.ends_with(" all") {
                    return true;
                }
            }
        }

        let mut set_command = Command::new("tmux");
        set_command.args(["set", "-p", "allow-passthrough", "on"]);
        command_output_with_timeout(set_command, Duration::from_secs(2))
            .ok()
            .flatten()
            .is_some_and(|output| output.status.success())
    })
}

fn command_output_with_timeout(
    mut command: Command,
    timeout: Duration,
) -> io::Result<Option<Output>> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map(Some);
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn inside_tmux() -> bool {
    env::var_os("TMUX").is_some()
}

pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);

        encoded.push(TABLE[(first >> 2) as usize] as char);
        encoded.push(TABLE[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(TABLE[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char);
        } else {
            encoded.push('=');
        }

        if chunk.len() > 2 {
            encoded.push(TABLE[(third & 0b0011_1111) as usize] as char);
        } else {
            encoded.push('=');
        }
    }

    encoded
}

pub(crate) fn query_terminal_size() -> TerminalSize {
    query_terminal_size_from(io::stdout())
        .or_else(|| query_terminal_size_from(io::stderr()))
        .or_else(|| query_terminal_size_from(io::stdin()))
        .unwrap_or(TerminalSize {
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        })
}

#[cfg(unix)]
fn query_terminal_size_from<T: AsRawFd>(stream: T) -> Option<TerminalSize> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe { libc::ioctl(stream.as_raw_fd(), libc::TIOCGWINSZ, &mut size) };
    if result == 0 && size.ws_col > 0 && size.ws_row > 0 {
        Some(TerminalSize {
            cols: u32::from(size.ws_col),
            rows: u32::from(size.ws_row),
            pixel_width: u32::from(size.ws_xpixel),
            pixel_height: u32::from(size.ws_ypixel),
        })
    } else {
        None
    }
}

#[cfg(not(unix))]
fn query_terminal_size_from<T>(_stream: T) -> Option<TerminalSize> {
    None
}

#[cfg(test)]
mod tests {
    use super::encode_tmux_passthrough;

    #[test]
    fn tmux_passthrough_encoder_matches_kitty_and_tmux_faq_form() {
        let mut bytes = Vec::new();
        encode_tmux_passthrough(&mut bytes, b"\x1b]22;pointer\x07").unwrap();

        assert_eq!(bytes, b"\x1bPtmux;\x1b\x1b]22;pointer\x07\x1b\\");
    }
}
