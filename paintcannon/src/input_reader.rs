//! The single owner of terminal input. Terminput decodes user events; control
//! replies stay here so they cannot be interpreted as typed text.
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use crate::glyph_detection::protocol::{Queries, Request};
use terminput::{Event, KeyCode};

const ESCAPE_DELAY: Duration = Duration::from_millis(25);
const MAX_SEQUENCE: usize = 4096;
const PASTE_END: &[u8] = b"\x1b[201~";

#[derive(Default)]
struct Parser {
    sequence: Vec<u8>,
    paste: Option<Vec<u8>>,
    control_string: Option<u8>,
    control_escape: bool,
    control_bytes: Vec<u8>,
    control_overflow: bool,
    glyph_queries: Option<Queries>,
    escape_since: Option<Instant>,
    events: VecDeque<Event>,
    kitty: Option<bool>,
}

impl Parser {
    fn feed(&mut self, bytes: &[u8], now: Instant) {
        for &byte in bytes {
            if let Some(paste) = &mut self.paste {
                paste.push(byte);
                if paste.ends_with(PASTE_END) {
                    paste.truncate(paste.len() - PASTE_END.len());
                    self.events
                        .push_back(Event::Paste(String::from_utf8_lossy(paste).into_owned()));
                    self.paste = None;
                }
                continue;
            }
            if let Some(kind) = self.control_string {
                if self.control_escape && byte == b'\\' && kind == b'_' && !self.control_overflow {
                    self.control_bytes.pop(); // The ESC preceding ST is not part of the body.
                    if let Some(queries) = &mut self.glyph_queries {
                        queries.reply(&self.control_bytes);
                    }
                }
                if (self.control_escape && byte == b'\\')
                    || (kind == b']' && byte == 7)
                    || matches!(byte, 0x18 | 0x1a)
                {
                    self.control_string = None;
                }
                if kind == b'_' && !self.control_overflow {
                    if self.control_bytes.len() < MAX_SEQUENCE {
                        self.control_bytes.push(byte);
                    } else {
                        self.control_overflow = true;
                        self.control_bytes.clear();
                    }
                }
                self.control_escape = byte == 0x1b;
                continue;
            }
            // A fresh ESC cancels a malformed/incomplete sequence, except ESC ESC
            // which represents two escape keys (the second may prefix another key).
            if byte == 0x1b && !self.sequence.is_empty() {
                if self.sequence == b"\x1b" {
                    self.events.push_back(Event::Key(KeyCode::Esc.into()));
                }
                self.sequence.clear();
            }
            self.sequence.push(byte);
            if self.sequence == b"\x1b" {
                self.escape_since = Some(now);
                continue;
            }
            self.escape_since = None;
            if self.sequence.len() == 2
                && self.sequence[0] == 0x1b
                && matches!(byte, b'_' | b'P' | b']' | b'^' | b'X')
            {
                self.control_string = Some(byte);
                self.control_escape = false;
                self.control_bytes.clear();
                self.control_overflow = false;
                self.sequence.clear();
                continue;
            }
            if self.sequence == b"\x1b[200~" {
                self.paste = Some(Vec::new());
                self.sequence.clear();
                continue;
            }
            // Wait for a complete CSI, including query replies the input parser
            // doesn't recognize. X10 mouse's three data bytes follow CSI M.
            if self.sequence.starts_with(b"\x1b[") {
                let len = self.sequence.len();
                if len <= 2 {
                    continue;
                }
                if self.sequence.starts_with(b"\x1b[M") && len < 6 {
                    continue;
                }
                if self.sequence == b"\x1b[[" {
                    continue;
                }
                if !(0x40..=0x7e).contains(&byte) && !self.sequence.starts_with(b"\x1b[M") {
                    if len >= MAX_SEQUENCE {
                        self.sequence.clear();
                    }
                    continue;
                }
                if let Some(flags) = self
                    .sequence
                    .strip_prefix(b"\x1b[?")
                    .and_then(|s| s.strip_suffix(b"u"))
                {
                    if std::str::from_utf8(flags)
                        .ok()
                        .and_then(|s| s.parse::<u32>().ok())
                        .is_some()
                    {
                        self.kitty = Some(true);
                    }
                    self.sequence.clear();
                    continue;
                }
                if self.sequence.starts_with(b"\x1b[?") && byte == b'c' {
                    self.kitty.get_or_insert(false);
                    self.sequence.clear();
                    continue;
                }
                if self.sequence == b"\x1b[0n" {
                    if let Some(queries) = &mut self.glyph_queries {
                        queries.heartbeat(now);
                    }
                    self.sequence.clear();
                    continue;
                }
            }
            match Event::parse_from(&self.sequence) {
                Ok(Some(event)) => {
                    self.events.push_back(event);
                    self.sequence.clear();
                }
                Ok(None) if self.sequence.len() < MAX_SEQUENCE => {}
                _ => self.sequence.clear(),
            }
        }
    }

    fn expire_escape(&mut self, now: Instant) {
        if self
            .escape_since
            .is_some_and(|since| now.duration_since(since) >= ESCAPE_DELAY)
        {
            self.events.push_back(Event::Key(KeyCode::Esc.into()));
            self.sequence.clear();
            self.escape_since = None;
        }
    }
}

pub(crate) struct InputReader {
    file: File,
    parser: Parser,
    resized: Arc<AtomicBool>,
    resize_signal: signal_hook::SigId,
}

impl InputReader {
    pub(crate) fn enable_glyph_queries(&mut self) -> crossbeam_channel::Sender<Request> {
        let (sender, queries) = Queries::new();
        self.parser.glyph_queries = Some(queries);
        sender
    }

    pub(crate) fn poll_glyph_queries(&mut self, captured: bool) {
        if let Some(queries) = &mut self.parser.glyph_queries {
            if !queries.has_work() {
                return;
            }
            queries.poll(
                captured && io::stdout().is_terminal(),
                &mut io::stdout().lock(),
                std::env::var_os("TMUX").is_some(),
                Instant::now(),
            );
        }
    }

    pub(crate) fn open() -> io::Result<Self> {
        let file = if io::stdin().is_terminal() {
            // Duplicate the descriptor without changing its blocking flags or owning stdin.
            let fd = unsafe { BorrowedFd::borrow_raw(libc::STDIN_FILENO) };
            File::from(fd.try_clone_to_owned()?)
        } else {
            OpenOptions::new().read(true).open("/dev/tty")?
        };
        Self::from_file(file)
    }

    fn from_file(file: File) -> io::Result<Self> {
        let resized = Arc::new(AtomicBool::new(false));
        let resize_signal = signal_hook::flag::register(libc::SIGWINCH, Arc::clone(&resized))?;
        Ok(Self {
            file,
            parser: Parser::default(),
            resized,
            resize_signal,
        })
    }

    pub(crate) fn negotiate_keyboard(&mut self, out: &mut impl Write) -> io::Result<bool> {
        out.write_all(b"\x1b[?u\x1b[c")?;
        out.flush()?;
        let deadline = Instant::now() + Duration::from_millis(200);
        while self.parser.kitty.is_none() && Instant::now() < deadline {
            self.read_bytes(deadline.saturating_duration_since(Instant::now()))?;
        }
        // User events received during negotiation remain in the same ordered queue.
        Ok(self.parser.kitty.unwrap_or(false))
    }

    pub(crate) fn read(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        let deadline = Instant::now() + timeout;
        loop {
            self.parser.expire_escape(Instant::now());
            if let Some(event) = self.parser.events.pop_front() {
                return Ok(Some(event));
            }
            if self.resized.swap(false, Ordering::Relaxed) {
                if let Some(size) = crate::terminal::try_query_terminal_size() {
                    return Ok(Some(Event::Resize {
                        cols: size.cols,
                        rows: size.rows,
                    }));
                }
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let escape_deadline = self
                .parser
                .escape_since
                .map(|t| t + ESCAPE_DELAY)
                .unwrap_or(deadline);
            self.read_bytes(deadline.min(escape_deadline).saturating_duration_since(now))?;
        }
    }

    fn read_bytes(&mut self, timeout: Duration) -> io::Result<()> {
        let mut descriptor = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let milliseconds = timeout.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128) as i32;
        // poll only observes our owned descriptor; the array remains valid for the call.
        let result = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
        if result < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::Interrupted {
                Ok(())
            } else {
                Err(error)
            };
        }
        if result == 0 {
            return Ok(());
        }
        if descriptor.revents & libc::POLLIN != 0 {
            let mut bytes = [0; 8192];
            match self.file.read(&mut bytes) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                Ok(length) => self.parser.feed(&bytes[..length], Instant::now()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        } else if descriptor.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        Ok(())
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        signal_hook::low_level::unregister(self.resize_signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::net::UnixStream;
    use terminput::{KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind, ScrollDirection};

    fn connected_reader() -> (InputReader, UnixStream) {
        let (input, terminal) = UnixStream::pair().unwrap();
        let fd: OwnedFd = input.into();
        (InputReader::from_file(File::from(fd)).unwrap(), terminal)
    }

    #[test]
    fn glyph_replies_are_fragment_safe_and_do_not_reorder_user_events() {
        use crate::glyph_detection::protocol::QUERY_TIMEOUT;
        let bytes = b"a\x1b_25a1;q;cp=e0a0;status=system\x1b\\\x1b[200~pasted\x1b[201~\x1b[Ob";
        for split in 0..=bytes.len() {
            let (tx, mut queries) = Queries::new();
            let (reply, rx) = crossbeam_channel::bounded(1);
            let now = Instant::now();
            tx.send(Request {
                characters: vec!['\u{e0a0}'],
                deadline: now + QUERY_TIMEOUT,
                reply,
            })
            .unwrap();
            queries.poll(true, &mut Vec::new(), false, now);
            let mut parser = Parser {
                glyph_queries: Some(queries),
                ..Default::default()
            };
            parser.feed(&bytes[..split], now);
            parser.feed(&bytes[split..], now);
            assert_eq!(rx.try_recv().unwrap(), vec![Some(true)]);
            assert_eq!(
                parser.events.into_iter().collect::<Vec<_>>(),
                vec![
                    Event::Key(KeyCode::Char('a').into()),
                    Event::Paste("pasted".into()),
                    Event::FocusLost,
                    Event::Key(KeyCode::Char('b').into())
                ]
            );
        }
    }

    #[test]
    fn glyph_worker_round_trip_uses_the_existing_reader() {
        for (status, supported) in [("system", true), ("", false)] {
            let (mut reader, mut terminal) = connected_reader();
            let tx = reader.enable_glyph_queries();
            let worker = std::thread::spawn(move || {
                let (reply, response) = crossbeam_channel::bounded(1);
                tx.send(Request {
                    characters: vec!['\u{e0a0}'],
                    deadline: Instant::now() + Duration::from_secs(1),
                    reply,
                })
                .unwrap();
                response.recv_timeout(Duration::from_secs(1)).unwrap()
            });
            let mut query = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(1);
            while query.is_empty() && Instant::now() < deadline {
                reader.parser.glyph_queries.as_mut().unwrap().poll(
                    true,
                    &mut query,
                    false,
                    Instant::now(),
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            terminal
                .write_all(format!("a\x1b_25a1;q;cp=e0a0;status={status}\x1b\\b").as_bytes())
                .unwrap();
            let first = reader.read(Duration::from_millis(100)).unwrap();
            let second = reader.read(Duration::from_millis(100)).unwrap();
            let result = worker.join().unwrap();
            assert_eq!(query, b"\x1b_25a1;q;cp=e0a0\x1b\\\x1b[5n");
            assert_eq!(first, Some(Event::Key(KeyCode::Char('a').into())));
            assert_eq!(second, Some(Event::Key(KeyCode::Char('b').into())));
            assert_eq!(result, vec![Some(supported)]);
        }
    }

    #[test]
    fn heartbeat_fragments_are_control_replies_not_keyboard_events() {
        let bytes = b"a\x1b[0n\x1b[Ob";
        for split in 0..=bytes.len() {
            let (tx, mut queries) = Queries::new();
            let (reply, rx) = crossbeam_channel::bounded(1);
            let start = Instant::now();
            tx.send(Request {
                characters: vec!['x'],
                deadline: start + Duration::from_secs(2),
                reply,
            })
            .unwrap();
            queries.poll(true, &mut Vec::new(), false, start);
            let mut parser = Parser {
                glyph_queries: Some(queries),
                ..Default::default()
            };
            let received = start + Duration::from_millis(10);
            parser.feed(&bytes[..split], received);
            parser.feed(&bytes[split..], received);
            parser.glyph_queries.as_mut().unwrap().poll(
                true,
                &mut Vec::new(),
                false,
                received + Duration::from_millis(50),
            );
            assert_eq!(rx.try_recv().unwrap(), vec![None]);
            assert_eq!(
                parser.events.into_iter().collect::<Vec<_>>(),
                vec![
                    Event::Key(KeyCode::Char('a').into()),
                    Event::FocusLost,
                    Event::Key(KeyCode::Char('b').into())
                ]
            );
        }
    }

    #[test]
    fn pasted_heartbeat_does_not_shorten_probe_deadline() {
        let (tx, mut queries) = Queries::new();
        let (reply, rx) = crossbeam_channel::bounded(1);
        let now = Instant::now();
        tx.send(Request {
            characters: vec!['x'],
            deadline: now + Duration::from_secs(2),
            reply,
        })
        .unwrap();
        queries.poll(true, &mut Vec::new(), false, now);
        let mut parser = Parser {
            glyph_queries: Some(queries),
            ..Default::default()
        };
        parser.feed(b"\x1b[200~\x1b[0n\x1b[201~", now);
        parser.glyph_queries.as_mut().unwrap().poll(
            true,
            &mut Vec::new(),
            false,
            now + Duration::from_millis(100),
        );
        assert!(rx.is_empty());
        assert_eq!(
            parser.events.pop_front(),
            Some(Event::Paste("\x1b[0n".into()))
        );
    }

    #[test]
    fn pasted_and_oversized_replies_cannot_confirm_a_glyph() {
        let (tx, mut queries) = Queries::new();
        let (reply, rx) = crossbeam_channel::bounded(1);
        let now = Instant::now();
        tx.send(Request {
            characters: vec!['\u{e0a0}'],
            deadline: now + Duration::from_secs(1),
            reply,
        })
        .unwrap();
        queries.poll(true, &mut Vec::new(), false, now);
        let mut parser = Parser {
            glyph_queries: Some(queries),
            ..Default::default()
        };
        parser.feed(
            b"\x1b[200~\x1b_25a1;q;cp=e0a0;status=system\x1b\\\x1b[201~",
            now,
        );
        assert!(rx.is_empty());
        let oversized = format!(
            "\x1b_25a1;q;cp=e0a0;status=system;extra={}\x1b\\",
            "x".repeat(MAX_SEQUENCE)
        );
        parser.feed(oversized.as_bytes(), now);
        assert!(rx.is_empty());
        parser.feed(b"\x1b_25a1;q;cp=e0a0;status=\x1b\\", now);
        assert_eq!(rx.try_recv().unwrap(), vec![Some(false)]);
        assert_eq!(parser.events.len(), 1);
        assert!(matches!(parser.events[0], Event::Paste(_)));
    }

    #[test]
    fn raw_tty_reads_control_keys_paste_and_focus_in_order() {
        let mut master = -1;
        let mut slave = -1;
        // openpty initializes both descriptors on success; File owns them below.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        let mut terminal = unsafe { File::from_raw_fd(master) };
        let input = unsafe { File::from_raw_fd(slave) };
        let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
        assert_eq!(
            unsafe { libc::tcgetattr(input.as_raw_fd(), attributes.as_mut_ptr()) },
            0
        );
        // tcgetattr initialized the termios structure. Only this test's slave
        // terminal is changed, never the process's controlling terminal.
        let mut attributes = unsafe { attributes.assume_init() };
        unsafe { libc::cfmakeraw(&mut attributes) };
        assert_eq!(
            unsafe { libc::tcsetattr(input.as_raw_fd(), libc::TCSANOW, &attributes) },
            0
        );
        let mut reader = InputReader::from_file(input).unwrap();
        terminal
            .write_all(b"\x03\x1b[200~hello\nworld\x1b[201~\x1b[O\x1b[I\x1b[A")
            .unwrap();
        for expected in [
            Event::Key(KeyEvent::new(KeyCode::Char('c')).modifiers(KeyModifiers::CTRL)),
            Event::Paste("hello\nworld".into()),
            Event::FocusLost,
            Event::FocusGained,
            Event::Key(KeyCode::Up.into()),
        ] {
            assert_eq!(reader.read(Duration::from_secs(1)).unwrap(), Some(expected));
        }
    }

    #[test]
    fn reader_negotiates_without_losing_interleaved_input() {
        let (mut reader, mut terminal) = connected_reader();
        terminal
            .write_all(b"a\x1b[200~paste\x1b[201~\x1b[?0u\x1b[?1;2c\x1b[Ob")
            .unwrap();
        let mut output = Vec::new();
        assert!(reader.negotiate_keyboard(&mut output).unwrap());
        assert_eq!(output, b"\x1b[?u\x1b[c");
        for expected in [
            Event::Key(KeyCode::Char('a').into()),
            Event::Paste("paste".into()),
            Event::FocusLost,
            Event::Key(KeyCode::Char('b').into()),
        ] {
            assert_eq!(
                reader.read(Duration::from_millis(100)).unwrap(),
                Some(expected)
            );
        }
        assert_eq!(reader.read(Duration::from_millis(1)).unwrap(), None);
    }

    #[test]
    fn reader_handles_escape_idle_and_disconnect() {
        let (mut reader, mut terminal) = connected_reader();
        assert_eq!(reader.read(Duration::from_millis(1)).unwrap(), None);
        terminal.write_all(b"\x1b").unwrap();
        assert_eq!(
            reader.read(Duration::from_secs(1)).unwrap(),
            Some(Event::Key(KeyCode::Esc.into()))
        );
        drop(terminal);
        assert_eq!(
            reader.read(Duration::from_secs(1)).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn unsupported_keyboard_query_preserves_pending_keys() {
        let (mut reader, mut terminal) = connected_reader();
        terminal.write_all(b"x\x1b[?1;2c").unwrap();
        assert!(!reader.negotiate_keyboard(&mut Vec::new()).unwrap());
        assert_eq!(
            reader.read(Duration::ZERO).unwrap(),
            Some(Event::Key(KeyCode::Char('x').into()))
        );
    }

    #[test]
    fn keyboard_query_timeout_preserves_pending_keys() {
        let (mut reader, mut terminal) = connected_reader();
        terminal.write_all(b"x").unwrap();
        assert!(!reader.negotiate_keyboard(&mut Vec::new()).unwrap());
        assert_eq!(
            reader.read(Duration::ZERO).unwrap(),
            Some(Event::Key(KeyCode::Char('x').into()))
        );
    }

    #[test]
    fn large_paste_spans_reads_and_preserves_following_key() {
        let payload = "hello\n".repeat(4000);
        let bytes = format!("\x1b[200~{payload}\x1b[201~z");
        let chunks: Vec<_> = bytes.as_bytes().chunks(8192).collect();
        assert_eq!(
            parse(&chunks),
            vec![Event::Paste(payload), Event::Key(KeyCode::Char('z').into())]
        );
    }

    fn parse(chunks: &[&[u8]]) -> Vec<Event> {
        let mut parser = Parser::default();
        let now = Instant::now();
        for chunk in chunks {
            parser.feed(chunk, now);
        }
        parser.expire_escape(now + ESCAPE_DELAY);
        parser.events.into_iter().collect()
    }

    #[test]
    fn fragmented_utf8_and_kitty_keys_preserve_order() {
        let bytes = "a\u{1f680}\x1b[97;5:2u\x1b[97;5:3u\x1b[Z".as_bytes();
        let expected = vec![
            Event::Key(KeyCode::Char('a').into()),
            Event::Key(KeyCode::Char('\u{1f680}').into()),
            Event::Key(
                KeyEvent::new(KeyCode::Char('a'))
                    .modifiers(KeyModifiers::CTRL)
                    .kind(KeyEventKind::Repeat),
            ),
            Event::Key(
                KeyEvent::new(KeyCode::Char('a'))
                    .modifiers(KeyModifiers::CTRL)
                    .kind(KeyEventKind::Release),
            ),
            Event::Key(KeyEvent::new(KeyCode::Tab).modifiers(KeyModifiers::SHIFT)),
        ];
        for split in 0..=bytes.len() {
            assert_eq!(parse(&[&bytes[..split], &bytes[split..]]), expected);
        }
    }

    #[test]
    fn paste_keeps_literal_replies_and_escape_sequences() {
        let payload = "a\x1b_25a1;q;cp=e0a0;status=system\x1b\\\x1b[I\n\u{1f680}";
        let bytes = format!("x\x1b[200~{payload}\x1b[201~y");
        let expected = vec![
            Event::Key(KeyCode::Char('x').into()),
            Event::Paste(payload.into()),
            Event::Key(KeyCode::Char('y').into()),
        ];
        for split in 0..=bytes.len() {
            assert_eq!(
                parse(&[&bytes.as_bytes()[..split], &bytes.as_bytes()[split..]]),
                expected
            );
        }
    }

    #[test]
    fn protocol_strings_never_become_keys() {
        let bytes = b"a\x1b_25a1;q;cp=e0a0;status=system\x1b\\\x1b]10;rgb:ff/ff/ff\x07\x1bP1+r544e=72696f\x1b\\b";
        for split in 0..=bytes.len() {
            assert_eq!(
                parse(&[&bytes[..split], &bytes[split..]]),
                vec![
                    Event::Key(KeyCode::Char('a').into()),
                    Event::Key(KeyCode::Char('b').into())
                ]
            );
        }
    }

    #[test]
    fn focus_mouse_and_keys_are_ordered() {
        let events = parse(&[b"\x1b[O\x1b[<64;8;4M\x1b[I\x03"]);
        assert_eq!(events[0], Event::FocusLost);
        let Event::Mouse(mouse) = events[1] else {
            panic!("expected mouse");
        };
        assert_eq!((mouse.column, mouse.row), (7, 3));
        assert_eq!(mouse.kind, MouseEventKind::Scroll(ScrollDirection::Up));
        assert_eq!(events[2], Event::FocusGained);
        assert_eq!(
            events[3],
            Event::Key(KeyEvent::new(KeyCode::Char('c')).modifiers(KeyModifiers::CTRL))
        );
    }

    #[test]
    fn escape_waits_but_alt_and_arrow_sequences_survive_fragmentation() {
        let now = Instant::now();
        let mut parser = Parser::default();
        parser.feed(b"\x1b", now);
        parser.expire_escape(now + ESCAPE_DELAY / 2);
        assert!(parser.events.is_empty());
        parser.expire_escape(now + ESCAPE_DELAY);
        assert_eq!(
            parser.events.pop_front(),
            Some(Event::Key(KeyCode::Esc.into()))
        );
        assert_eq!(
            parse(&[b"\x1b", b"x\x1b", b"[A"]),
            vec![
                Event::Key(KeyEvent::new(KeyCode::Char('x')).modifiers(KeyModifiers::ALT)),
                Event::Key(KeyCode::Up.into())
            ]
        );
    }

    #[test]
    fn kitty_negotiation_keeps_user_input_and_does_not_leak_replies() {
        let mut parser = Parser::default();
        parser.feed(b"a\x1b[?0u\x1b[?1;2c\x1b[Ob", Instant::now());
        assert_eq!(parser.kitty, Some(true));
        assert_eq!(
            parser.events.into_iter().collect::<Vec<_>>(),
            vec![
                Event::Key(KeyCode::Char('a').into()),
                Event::FocusLost,
                Event::Key(KeyCode::Char('b').into())
            ]
        );
        let mut parser = Parser::default();
        parser.feed(b"\x1b[?1;2c", Instant::now());
        assert_eq!(parser.kitty, Some(false));
    }
}
