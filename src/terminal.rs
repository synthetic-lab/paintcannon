use std::env;
use std::io::{self, IsTerminal, Write};

#[cfg(unix)]
use std::os::fd::AsRawFd;

use napi_derive::napi;

#[napi(object)]
pub struct TerminalSize {
    pub cols: u32,
    pub rows: u32,
}

pub(crate) fn reset_terminal() {
    let rows = query_terminal_size().rows;
    let mut out = io::stdout().lock();
    let _ = write_synchronized_output_end(&mut out);
    let _ = write!(out, "\x1b[0m\x1b[?25h\x1b[{rows};1H\x1b[2K\n");
}

pub(crate) fn stdout_is_terminal() -> bool {
    io::stdout().is_terminal()
}

pub(crate) fn write_synchronized_output_begin(out: &mut impl Write) -> io::Result<()> {
    if !stdout_is_terminal() {
        return Ok(());
    }

    if inside_tmux() {
        write!(out, "\x1bPtmux;\x1b\x1b[?2026h\x1b\\")
    } else {
        write!(out, "\x1b[?2026h")
    }
}

pub(crate) fn write_synchronized_output_end(out: &mut impl Write) -> io::Result<()> {
    if !stdout_is_terminal() {
        return Ok(());
    }

    write!(out, "\x1b[?2026l")?;
    if inside_tmux() {
        write!(out, "\x1bPtmux;\x1b\x1b[?2026l\x1b\\")?;
    }
    Ok(())
}

fn inside_tmux() -> bool {
    env::var_os("TMUX").is_some()
}

pub(crate) fn query_terminal_size() -> TerminalSize {
    query_terminal_size_from(io::stdout())
        .or_else(|| query_terminal_size_from(io::stderr()))
        .or_else(|| query_terminal_size_from(io::stdin()))
        .unwrap_or(TerminalSize { cols: 80, rows: 24 })
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
        })
    } else {
        None
    }
}

#[cfg(not(unix))]
fn query_terminal_size_from<T>(_stream: T) -> Option<TerminalSize> {
    None
}
