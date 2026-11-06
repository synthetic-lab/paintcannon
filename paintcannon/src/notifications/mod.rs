//! Desktop notifications. All terminal I/O and local process execution stay native.
mod local;
pub(crate) mod protocol;
pub(crate) mod query;

use crate::event_notification::EventNotification;
use crate::native_event::{NativeEvent, NativeEventQueue};
use crate::terminal_capabilities::{Capabilities, Detection};
use crossbeam_channel::{bounded, select_biased, Sender};
use napi::{Env, Result, Task};
use napi_derive::napi;
use protocol::Protocol;
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Clone, Debug)]
#[napi(object)]
pub struct NotificationOptions {
    pub title: String,
    pub body: String,
    pub tag: String,
    pub silent: Option<bool>,
    pub require_interaction: bool,
}

#[derive(Clone, Debug)]
#[napi(object)]
pub struct NotificationEvent {
    pub id: u32,
    pub r#type: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Backend {
    Osc(Protocol),
    Local(local::Backend),
}

impl Backend {
    fn reports_events(&self) -> bool {
        matches!(self, Self::Osc(Protocol::Osc99(support)) if support.report || support.close_events)
    }

    fn supports_close(&self) -> bool {
        match self {
            Self::Osc(protocol) => protocol.supports_close(),
            Self::Local(backend) => local::supports_close(backend),
        }
    }
}

fn choose_backend(
    capabilities: Capabilities,
    tty: bool,
    osc99: Option<query::Support>,
    local: Option<local::Backend>,
) -> Option<Backend> {
    let protocols = capabilities.notifications;
    if tty {
        if let Some(support) = osc99 {
            return Some(Backend::Osc(Protocol::Osc99(support)));
        }
        if protocols.osc777 {
            return Some(Backend::Osc(Protocol::Osc777));
        }
        if protocols.osc9 {
            return Some(Backend::Osc(Protocol::Osc9));
        }
    }
    capabilities
        .local
        .then_some(local)
        .flatten()
        .map(Backend::Local)
}

fn backend(detection: &Detection) -> Option<Backend> {
    let caps = detection.environment().capabilities();
    let tty = io::stdout().is_terminal();
    if tty && std::env::var_os("TMUX").is_some() {
        crate::terminal::allow_tmux_passthrough();
    }
    choose_backend(
        caps,
        tty,
        tty.then(|| detection.notification_support()).flatten(),
        caps.local.then(local::detect).flatten(),
    )
}

pub(crate) enum NotificationCapability {
    Delivery,
    Close,
}

pub struct NotificationSupport {
    pub(crate) detection: Arc<Detection>,
    pub(crate) capability: NotificationCapability,
}

#[napi]
impl Task for NotificationSupport {
    type Output = bool;
    type JsValue = bool;
    fn compute(&mut self) -> Result<bool> {
        let backend = backend(&self.detection);
        Ok(match self.capability {
            NotificationCapability::Delivery => backend.is_some(),
            NotificationCapability::Close => {
                backend.is_some_and(|backend| backend.supports_close())
            }
        })
    }
    fn resolve(&mut self, _env: Env, output: bool) -> Result<bool> {
        Ok(output)
    }
}

/// Only the reader consumes replies; it publishes into the same ordered queue as
/// keyboard/paste events. Unknown IDs (including replies for another pane) are ignored.
pub(crate) struct Reports {
    active: Mutex<HashMap<String, u32>>,
    events: Arc<NativeEventQueue>,
    notifier: Mutex<Option<Arc<dyn EventNotification>>>,
}

impl Reports {
    fn event(&self, id: u32, kind: &str, error: Option<String>) {
        self.events
            .push(NativeEvent::notification(NotificationEvent {
                id,
                r#type: kind.into(),
                error,
            }));
        if let Some(notifier) = self.notifier.lock().unwrap().as_ref() {
            notifier.notify();
        }
    }

    pub(crate) fn reply(&self, reply: protocol::Reply) {
        let mut active = self.active.lock().unwrap();
        if let Some(&id) = active.get(&reply.id) {
            if reply.kind == "close" {
                active.remove(&reply.id);
            }
            self.event(id, reply.kind, None);
        }
    }
}

enum Command {
    Show {
        id: u32,
        options: NotificationOptions,
        detection: Arc<Detection>,
    },
    Close(u32),
}

struct Shown {
    backend: Backend,
    wire_id: String,
    local_id: Option<u32>,
    tag: String,
}

pub(crate) struct Notifications {
    sender: Sender<Command>,
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
    pub(crate) reports: Arc<Reports>,
}

impl Notifications {
    pub(crate) fn start(
        events: Arc<NativeEventQueue>,
        notifier: Arc<dyn EventNotification>,
    ) -> Self {
        let reports = Arc::new(Reports {
            active: Mutex::default(),
            events,
            notifier: Mutex::new(Some(notifier)),
        });
        let published = reports.clone();
        let (sender, commands) = bounded(64);
        let (stop, stopping) = bounded(0);
        let tmux = std::env::var_os("TMUX").is_some();
        let namespace = format!(
            "pc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let thread = thread::spawn(move || {
            let mut shown: HashMap<u32, Shown> = HashMap::new();
            loop {
                let command = select_biased! {
                    recv(stopping) -> _ => break,
                    recv(commands) -> command => match command { Ok(command) => command, Err(_) => break },
                };
                // A terminal dismissal is delivered by the input thread. Retire
                // those records before considering tag replacement or close().
                {
                    let active = published.active.lock().unwrap();
                    shown.retain(|_, notification| {
                        !notification.backend.reports_events()
                            || active.contains_key(&notification.wire_id)
                    });
                }
                match command {
                    Command::Show {
                        id,
                        options,
                        detection,
                    } => {
                        let result = (|| -> io::Result<()> {
                            let selected = backend(&detection).ok_or_else(|| io::Error::other("no notification backend is available for the connected terminal"))?;
                            let previous = (!options.tag.is_empty())
                                .then(|| {
                                    shown
                                        .iter()
                                        .find(|(_, value)| value.tag == options.tag)
                                        .map(|(&id, _)| id)
                                })
                                .flatten();
                            let old = previous.and_then(|id| shown.get(&id));
                            let replacing = old.filter(|old| old.backend == selected);
                            let wire_id = replacing
                                .map(|old| old.wire_id.clone())
                                .unwrap_or_else(|| format!("{namespace}-{id}"));
                            let mut reports = None;
                            let local_id = match &selected {
                                Backend::Osc(protocol) => {
                                    if tmux {
                                        crate::terminal::allow_tmux_passthrough();
                                    }
                                    // Keep replies behind the successful write's
                                    // show event, even for a very fast terminal.
                                    reports = Some(published.active.lock().unwrap());
                                    let mut out = io::stdout().lock();
                                    protocol::show(&mut out, *protocol, tmux, &wire_id, &options)?;
                                    out.flush()?;
                                    None
                                }
                                Backend::Local(local) => local::show(
                                    local,
                                    &options,
                                    replacing.and_then(|old| old.local_id),
                                )?,
                            };
                            let mut active =
                                reports.unwrap_or_else(|| published.active.lock().unwrap());
                            if let Some(old_id) = previous {
                                if let Some(old) = shown.remove(&old_id) {
                                    active.remove(&old.wire_id);
                                    published.event(old_id, "close", None);
                                }
                            }
                            if selected.reports_events() {
                                active.insert(wire_id.clone(), id);
                            }
                            shown.insert(
                                id,
                                Shown {
                                    backend: selected,
                                    wire_id,
                                    local_id,
                                    tag: options.tag,
                                },
                            );
                            published.event(id, "show", None);
                            Ok(())
                        })();
                        if let Err(error) = result {
                            published.event(id, "error", Some(error.to_string()));
                        }
                    }
                    Command::Close(id) => {
                        if let Some(notification) = shown.remove(&id) {
                            published
                                .active
                                .lock()
                                .unwrap()
                                .remove(&notification.wire_id);
                            let result = match notification.backend {
                                Backend::Osc(protocol) => {
                                    let mut out = io::stdout().lock();
                                    protocol::close(&mut out, protocol, tmux, &notification.wire_id)
                                        .and_then(|()| out.flush())
                                }
                                Backend::Local(ref backend) => {
                                    local::close(backend, notification.local_id)
                                }
                            };
                            if let Err(error) = result {
                                published.event(id, "error", Some(error.to_string()));
                            }
                            published.event(id, "close", None);
                        }
                    }
                }
            }
            published.active.lock().unwrap().clear();
        });
        Self {
            sender,
            stop: Some(stop),
            thread: Some(thread),
            reports,
        }
    }

    pub(crate) fn show(
        &self,
        id: u32,
        options: NotificationOptions,
        detection: Arc<Detection>,
    ) -> Result<()> {
        self.sender
            .try_send(Command::Show {
                id,
                options,
                detection,
            })
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
    pub(crate) fn close(&self, id: u32) -> Result<()> {
        self.sender
            .try_send(Command::Close(id))
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
    pub(crate) fn shutdown(&mut self) {
        self.stop.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        // The NAPI object can outlive stop(). Release its event-loop reference
        // now, rather than waiting for JavaScript to garbage-collect it.
        self.reports.notifier.lock().unwrap().take();
    }
}

impl Drop for Notifications {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_capabilities::NotificationProtocols;

    #[test]
    fn kitty_identity_alone_does_not_enable_osc99_or_dismissal() {
        let capabilities = crate::terminal_capabilities::environment::Environment::from_pairs(&[(
            "TERM",
            "xterm-kitty",
        )])
        .capabilities();
        let selected = choose_backend(capabilities, true, None, None).unwrap();
        assert_eq!(selected, Backend::Osc(Protocol::Osc777));
        assert!(!selected.supports_close());
    }

    #[test]
    fn legacy_notification_backends_match_confirmed_terminal_support() {
        use crate::terminal_capabilities::environment::Environment;

        for (program, osc9, osc777) in [
            ("terminology", false, true),
            ("secure shellfish", false, true),
            ("konsole", false, true),
            ("absolutetelnet", true, true),
            ("rio", true, true),
            ("kitty", true, true),
            ("iterm.app", true, false),
            ("windows_terminal", false, false),
            ("mintty", false, false),
            ("domterm", false, false),
            ("gnome-terminal", false, false),
            ("tilix", false, false),
            ("vscode", false, false),
            ("urxvt", false, false),
        ] {
            let capabilities = Environment::from_pairs(&[("TERM_PROGRAM", program)]).capabilities();
            assert_eq!(
                capabilities.notifications,
                NotificationProtocols { osc9, osc777 },
                "{program}"
            );
            let expected = if osc777 {
                Some(Backend::Osc(Protocol::Osc777))
            } else if osc9 {
                Some(Backend::Osc(Protocol::Osc9))
            } else {
                None
            };
            assert_eq!(
                choose_backend(capabilities, true, None, None),
                expected,
                "{program}"
            );
        }
    }

    #[test]
    fn queried_features_override_terminal_identity_in_both_directions() {
        // Unknown terminals can implement OSC 99, and kitty can report no close.
        for capabilities in [
            Capabilities::default(),
            Capabilities {
                notifications: NotificationProtocols::ITERM,
                ..Default::default()
            },
        ] {
            for close in [false, true] {
                let support = query::Support {
                    close,
                    ..query::Support::default()
                };
                let selected = choose_backend(capabilities, true, Some(support), None).unwrap();
                assert_eq!(selected, Backend::Osc(Protocol::Osc99(support)));
                assert_eq!(selected.supports_close(), close);
            }
        }
    }

    #[test]
    fn queried_otty_uses_osc99_without_needing_a_local_desktop() {
        let capabilities = crate::terminal_capabilities::environment::Environment::from_pairs(&[
            ("TERM", "xterm-256color"),
            ("TERM_PROGRAM", "otty"),
        ])
        .capabilities();
        assert_eq!(
            choose_backend(capabilities, true, Some(query::Support::default()), None),
            Some(Backend::Osc(Protocol::Osc99(query::Support::default())))
        );
        assert_eq!(
            choose_backend(capabilities, true, None, None),
            Some(Backend::Osc(Protocol::Osc777))
        );
    }

    #[test]
    fn shutdown_releases_the_event_loop_callback_even_if_the_api_object_is_retained() {
        struct Notify;
        impl EventNotification for Notify {
            fn notify(&self) {}
        }
        let notifier = Arc::new(Notify);
        let weak = Arc::downgrade(&notifier);
        let mut notifications =
            Notifications::start(Arc::new(NativeEventQueue::default()), notifier);
        assert!(weak.upgrade().is_some());
        notifications.shutdown();
        assert!(weak.upgrade().is_none());
        notifications.shutdown();
    }

    #[test]
    fn remote_and_headless_sessions_never_notify_the_server_desktop() {
        let local = local::Backend::NotifySend("/bin/notify-send".into());
        assert_eq!(
            choose_backend(Capabilities::default(), true, None, Some(local.clone())),
            None
        );
        let supported = Capabilities {
            notifications: NotificationProtocols::ITERM,
            ..Default::default()
        };
        assert_eq!(
            choose_backend(
                supported,
                true,
                Some(query::Support::default()),
                Some(local.clone())
            ),
            Some(Backend::Osc(Protocol::Osc99(query::Support::default())))
        );
        assert_eq!(
            choose_backend(
                supported,
                false,
                Some(query::Support::default()),
                Some(local.clone())
            ),
            None
        );
        assert_eq!(
            choose_backend(
                Capabilities {
                    local: true,
                    ..Default::default()
                },
                true,
                None,
                Some(local.clone())
            ),
            Some(Backend::Local(local))
        );
    }
    #[test]
    fn notification_replies_share_the_native_queue_and_ignore_unknown_ids() {
        struct Notify;
        impl EventNotification for Notify {
            fn notify(&self) {}
        }
        let events = Arc::new(NativeEventQueue::default());
        let reports = Reports {
            active: Mutex::new(HashMap::from([("pc-1".into(), 1)])),
            events: events.clone(),
            notifier: Mutex::new(Some(Arc::new(Notify))),
        };
        events.push(NativeEvent::paste("before".into()));
        reports.reply(protocol::Reply {
            id: "pc-other".into(),
            kind: "click",
        });
        reports.reply(protocol::Reply {
            id: "pc-1".into(),
            kind: "click",
        });
        events.push(NativeEvent::paste("after".into()));
        let result = events.drain();
        assert_eq!(
            result.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(),
            ["paste", "notification", "paste"]
        );
        assert_eq!(result[1].notification.as_ref().unwrap().id, 1);
        reports.reply(protocol::Reply {
            id: "pc-1".into(),
            kind: "close",
        });
        reports.reply(protocol::Reply {
            id: "pc-1".into(),
            kind: "click",
        });
        assert_eq!(events.drain().len(), 1);
    }
}
