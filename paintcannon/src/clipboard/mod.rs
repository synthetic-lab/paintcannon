//! Rich clipboard reads are only initiated by a user's terminal paste event.
//! No clipboard polling, arbitrary reads, or terminal-name heuristics.
pub(crate) mod protocol;
mod support;
mod tmux;

pub(crate) use support::{RichPasteSupport, Support};

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::native_event::{ClipboardImage, NativeEvent, NativeEventQueue};
use protocol::Packet;

const IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_LIST_BYTES: usize = 64 * 1024;
const MAX_PASTES: usize = 8;

fn write_sequence(out: &mut impl Write, sequence: &[u8], tmux: bool) -> io::Result<()> {
    if tmux {
        crate::terminal::encode_tmux_passthrough(out, sequence)
    } else {
        out.write_all(sequence)
    }
}

#[derive(Default)]
pub(crate) struct PasteMode {
    tmux: bool,
    unfocused: bool,
    queried: bool,
    blocked: bool,
    pub(crate) enabled: bool,
    support: Arc<Support>,
}

impl PasteMode {
    pub(crate) fn new(tmux: bool) -> Self {
        Self {
            tmux,
            ..Self::default()
        }
    }

    pub(crate) fn support(&self) -> Arc<Support> {
        Arc::clone(&self.support)
    }

    pub(crate) fn query(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.query_with_tmux_setup(out, tmux::prepare_rich_paste)
    }

    fn query_with_tmux_setup(
        &mut self,
        out: &mut impl Write,
        prepare_tmux: impl FnOnce() -> bool,
    ) -> io::Result<()> {
        if !self.queried && !self.unfocused && !self.blocked {
            // Passthrough affects every pane sharing the outer terminal. Require
            // focus reports and a way to reset the mode even after our pane is hidden.
            if self.tmux && !prepare_tmux() {
                self.blocked = true;
                self.support.set(Some(false));
                return Ok(());
            }
            self.support.set(None);
            if let Err(error) =
                write_sequence(out, protocol::QUERY, self.tmux).and_then(|()| out.flush())
            {
                self.support.set(Some(false));
                return Err(error);
            }
            self.queried = true;
        }
        Ok(())
    }

    pub(crate) fn report(&mut self, status: u8, out: &mut impl Write) -> io::Result<()> {
        if !self.queried {
            return Ok(());
        }
        let result = if status == 2 {
            write_sequence(out, protocol::ENABLE, self.tmux).and_then(|()| out.flush())
        } else {
            Ok(())
        };
        self.enabled = result.is_ok() && matches!(status, 1..=3);
        self.support.set(Some(self.enabled));
        result
    }

    pub(crate) fn restore(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.support.set(Some(false));
        self.deactivate(out)
    }

    pub(crate) fn set_focused(&mut self, focused: bool, out: &mut impl Write) -> io::Result<()> {
        if self.unfocused == focused {
            self.unfocused = !focused;
            // Passthrough changes the outer terminal's mode, not a pane-local mode.
            // Release even an inherited mode so other panes can paste normally.
            if self.tmux && !focused && (self.queried || self.enabled) {
                return self.deactivate(out);
            }
            // Another program may have reset the mode while we were unfocused.
            // Re-query on focus return; ignore outstanding replies while blurred.
            self.queried = false;
            self.blocked = false;
        }
        Ok(())
    }

    fn deactivate(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.queried = false;
        self.blocked = false;
        self.enabled = false;
        // Always leave normal paste behind, including a mode inherited from a
        // crashed process, an unanswered query, or a partially failed write.
        write_sequence(out, protocol::DISABLE, self.tmux)?;
        out.flush()
    }
}

struct Listing {
    id: u64,
    password: Option<String>,
    primary: bool,
    types: Vec<u8>,
    deadline: Instant,
}

struct Transfer {
    id: u64,
    mime: String,
    data: Vec<u8>,
    started: bool,
    deadline: Instant,
}

pub(crate) struct Pastes {
    tmux: bool,
    namespace: String,
    next_id: u64,
    listing: Option<Listing>,
    transfers: HashMap<String, Transfer>,
}

impl Default for Pastes {
    fn default() -> Self {
        Self {
            tmux: false,
            namespace: format!(
                "pc-paste-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            next_id: 0,
            listing: None,
            transfers: HashMap::new(),
        }
    }
}

impl Pastes {
    pub(crate) fn new(tmux: bool) -> Self {
        Self {
            tmux,
            ..Self::default()
        }
    }

    pub(crate) fn receive(
        &mut self,
        packet: Packet,
        queue: &NativeEventQueue,
        out: &mut impl Write,
        now: Instant,
    ) {
        if let Some(id) = &packet.id {
            self.receive_data(id.clone(), packet, queue, now);
            return;
        }
        if packet.status == "OK" {
            if let Some(previous) = self.listing.take() {
                queue.finish_paste(previous.id, None);
            }
            if packet.data.is_none() || self.transfers.len() >= MAX_PASTES {
                return;
            }
            self.next_id += 1;
            queue.reserve_paste(self.next_id);
            self.listing = Some(Listing {
                id: self.next_id,
                password: packet.password,
                primary: packet.primary,
                types: Vec::new(),
                deadline: now + IDLE_TIMEOUT,
            });
            return;
        }
        let Some(mut listing) = self.listing.take() else {
            return;
        };
        let Some(data) = packet.data else {
            queue.finish_paste(listing.id, None);
            return;
        };
        if packet
            .password
            .as_ref()
            .is_some_and(|password| Some(password) != listing.password.as_ref())
        {
            queue.finish_paste(listing.id, None);
            return;
        }
        match packet.status.as_str() {
            "DATA"
                if packet.mime.as_deref() == Some(".")
                    && listing.types.len() + data.len() <= MAX_LIST_BYTES =>
            {
                listing.types.extend(data);
                listing.deadline = now + IDLE_TIMEOUT;
                self.listing = Some(listing);
            }
            "DONE" => {
                let Some(mime) = preferred_mime(&listing.types) else {
                    queue.finish_paste(listing.id, None);
                    return;
                };
                let id = format!("{}-{}", self.namespace, listing.id);
                let request = protocol::read_request(
                    &id,
                    &mime,
                    listing.primary,
                    listing.password.as_deref(),
                );
                if write_sequence(out, &request, self.tmux)
                    .and_then(|()| out.flush())
                    .is_err()
                {
                    queue.finish_paste(listing.id, None);
                    return;
                }
                self.transfers.insert(
                    id,
                    Transfer {
                        id: listing.id,
                        mime,
                        data: Vec::new(),
                        started: false,
                        deadline: now + IDLE_TIMEOUT,
                    },
                );
            }
            _ => queue.finish_paste(listing.id, None),
        }
    }

    fn receive_data(&mut self, id: String, packet: Packet, queue: &NativeEventQueue, now: Instant) {
        let Some(mut transfer) = self.transfers.remove(&id) else {
            return;
        };
        let Some(data) = packet.data else {
            queue.finish_paste(transfer.id, None);
            return;
        };
        match packet.status.as_str() {
            "OK" if !transfer.started => transfer.started = true,
            "DATA"
                if transfer.started
                    && packet.mime.as_ref() == Some(&transfer.mime)
                    && transfer.data.len()
                        + data.len()
                        + self
                            .transfers
                            .values()
                            .map(|transfer| transfer.data.len())
                            .sum::<usize>()
                        <= MAX_BYTES =>
            {
                transfer.data.extend(data);
            }
            "DONE" if transfer.started => {
                let event = if transfer.mime.starts_with("image/") {
                    if transfer.data.is_empty() {
                        None
                    } else {
                        let mut event = NativeEvent::paste(String::new());
                        event.paste_image = Some(ClipboardImage {
                            mime_type: transfer.mime,
                            data: transfer.data.into(),
                        });
                        Some(event)
                    }
                } else {
                    Some(NativeEvent::paste(
                        String::from_utf8_lossy(&transfer.data).into_owned(),
                    ))
                };
                queue.finish_paste(transfer.id, event);
                return;
            }
            _ => {
                queue.finish_paste(transfer.id, None);
                return;
            }
        }
        transfer.deadline = now + IDLE_TIMEOUT;
        self.transfers.insert(id, transfer);
    }

    /// Cancel stalled reads (or all reads on suspension) without blocking stdin.
    pub(crate) fn expire(
        &mut self,
        queue: &NativeEventQueue,
        now: Instant,
        captured: bool,
    ) -> bool {
        let mut changed = false;
        if self
            .listing
            .as_ref()
            .is_some_and(|listing| !captured || now >= listing.deadline)
        {
            if let Some(listing) = self.listing.take() {
                queue.finish_paste(listing.id, None);
                changed = true;
            }
        }
        self.transfers.retain(|_, transfer| {
            if !captured || now >= transfer.deadline {
                queue.finish_paste(transfer.id, None);
                changed = true;
                false
            } else {
                true
            }
        });
        changed
    }
}

fn preferred_mime(types: &[u8]) -> Option<String> {
    let types: Vec<_> = std::str::from_utf8(types)
        .ok()?
        .split_ascii_whitespace()
        .collect();
    // These are alternate encodings of one clipboard image, not separate files.
    for mime in [
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/gif",
        "text/plain",
    ] {
        if types.contains(&mime) {
            return Some(mime.to_owned());
        }
    }
    types
        .into_iter()
        .find(|mime| mime.eq_ignore_ascii_case("text/plain;charset=utf-8"))
        .map(str::to_owned)
}
