use super::{environment::Environment, protocol::Request, Detection, ProbeReport, SAMPLES};
use crate::engine::EngineCommand;
use crate::event_notification::EventNotification;
use crate::native_event::{NativeEvent, NativeEventQueue};
use crossbeam_channel::{bounded, select, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub(crate) struct FontMonitor {
    current: Arc<RwLock<Arc<Detection>>>,
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl FontMonitor {
    pub(crate) fn start(
        queries: Option<Sender<Request>>,
        commands: Sender<EngineCommand>,
        events: Arc<NativeEventQueue>,
        notifier: Arc<dyn EventNotification>,
    ) -> Self {
        Self::start_with_environment(queries, commands, events, notifier, Environment::current)
    }

    fn start_with_environment(
        queries: Option<Sender<Request>>,
        commands: Sender<EngineCommand>,
        events: Arc<NativeEventQueue>,
        notifier: Arc<dyn EventNotification>,
        mut environment: impl FnMut() -> Environment + Send + 'static,
    ) -> Self {
        let (stop, stopping) = bounded(0);
        let current = Arc::new(RwLock::new(Arc::new(Detection {
            stop: stopping.clone(),
            ..Detection::default()
        })));
        let shared = current.clone();
        let thread = thread::spawn(move || {
            let mut published = Published {
                report: ProbeReport::default(),
                osc8: false,
                commands,
                events,
                notifier,
                stopping: stopping.clone(),
            };
            loop {
                let deadline = Instant::now() + POLL_INTERVAL;
                let detection = shared.read().unwrap().clone();
                let snapshot = detection.environment.get_or_init(&mut environment);
                if !published.update_osc8(snapshot.capabilities().osc8) {
                    break;
                }
                // Known sets needn't wait for unrelated, unrecognized sets. Keep
                // the previous result for those until this pass's probe completes.
                let known = ProbeReport {
                    nerd_fonts: detection
                        .known_nerd_fonts()
                        .unwrap_or(published.report.nerd_fonts),
                    legacy_symbols: detection
                        .known_legacy_symbols()
                        .unwrap_or(published.report.legacy_symbols),
                };
                if !published.update(known) {
                    break;
                }
                let report = ProbeReport {
                    nerd_fonts: detection.detect(queries.as_ref()),
                    legacy_symbols: detection.detect_legacy(queries.as_ref()),
                };
                if !published.update(report)
                    || stopping.recv_deadline(deadline) != Err(RecvTimeoutError::Timeout)
                {
                    break;
                }
                // Resolve the environment outside the shared lock: asynchronous
                // API calls can keep reading the last snapshot during a tmux query.
                let next = Arc::new(Detection {
                    environment: OnceLock::from(environment()),
                    stop: stopping.clone(),
                    ..Detection::default()
                });
                *shared.write().unwrap() = next;
            }
        });
        Self {
            current,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    pub(crate) fn detection(&self) -> Arc<Detection> {
        self.current.read().unwrap().clone()
    }

    pub(crate) fn shutdown(&mut self) {
        // Disconnect wakes every waiter, including a probe running on a NAPI
        // worker. Don't sleep out the two-second polling/probe interval on exit.
        self.stop.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for FontMonitor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Published {
    report: ProbeReport,
    osc8: bool,
    commands: Sender<EngineCommand>,
    events: Arc<NativeEventQueue>,
    notifier: Arc<dyn EventNotification>,
    stopping: Receiver<()>,
}

impl Published {
    fn update_osc8(&mut self, supported: bool) -> bool {
        if self.osc8 != supported {
            select! {
                recv(self.stopping) -> _ => return false,
                send(self.commands, EngineCommand::SetHyperlinksSupported { supported }) -> result =>
                    if result.is_err() { return false; },
            }
            self.osc8 = supported;
        }
        true
    }

    fn update(&mut self, report: ProbeReport) -> bool {
        if self.stopping.try_recv() == Err(TryRecvError::Disconnected) {
            return false;
        }
        for (changed, command) in [
            (
                self.report.nerd_fonts.powerline_extended != report.nerd_fonts.powerline_extended,
                EngineCommand::SetPowerlineExtended {
                    supported: report.nerd_fonts.powerline_extended,
                },
            ),
            (
                self.report.legacy_symbols != report.legacy_symbols,
                EngineCommand::SetLegacySymbols {
                    supported: report.legacy_symbols,
                },
            ),
        ] {
            if changed {
                select! {
                    recv(self.stopping) -> _ => return false,
                    send(self.commands, command) -> result => if result.is_err() { return false; },
                }
            }
        }
        let mut changed = false;
        for &(set, _) in SAMPLES {
            let supported = report.nerd_fonts.supports(set);
            if supported != self.report.nerd_fonts.supports(set) {
                self.events.push(NativeEvent::font(Some(set), supported));
                changed = true;
            }
        }
        if report.legacy_symbols != self.report.legacy_symbols {
            self.events
                .push(NativeEvent::font(None, report.legacy_symbols));
            changed = true;
        }
        self.report = report;
        if changed {
            self.notifier.notify();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::{NerdFontSet, NerdFontSupport};
    use super::*;
    use crossbeam_channel::{never, unbounded};

    struct Notify(Sender<()>);
    impl EventNotification for Notify {
        fn notify(&self) {
            let _ = self.0.send(());
        }
    }

    #[test]
    fn osc8_changes_update_the_engine_without_becoming_font_events() {
        let (commands, rx) = unbounded();
        let (wake, wakes) = unbounded();
        let events = Arc::new(NativeEventQueue::default());
        let mut published = Published {
            report: ProbeReport::default(),
            osc8: false,
            commands,
            events: events.clone(),
            notifier: Arc::new(Notify(wake)),
            stopping: never(),
        };
        assert!(published.update_osc8(true));
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::SetHyperlinksSupported { supported: true }
        ));
        assert!(published.update_osc8(true));
        assert!(rx.is_empty());
        assert!(published.update_osc8(false));
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::SetHyperlinksSupported { supported: false }
        ));
        assert!(events.drain().is_empty());
        assert!(wakes.is_empty());
    }

    #[test]
    fn publishes_only_changes_including_downgrades_and_border_commands() {
        let (commands, rx) = unbounded();
        let (wake, wakes) = unbounded();
        let events = Arc::new(NativeEventQueue::default());
        let mut published = Published {
            report: ProbeReport::default(),
            osc8: false,
            commands,
            events: events.clone(),
            notifier: Arc::new(Notify(wake)),
            stopping: never(),
        };
        let supported = ProbeReport {
            nerd_fonts: NerdFontSupport::uniform(true),
            legacy_symbols: true,
        };
        assert!(published.update(supported));
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::SetPowerlineExtended { supported: true }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::SetLegacySymbols { supported: true }
        ));
        let initial = events.drain();
        assert_eq!(initial.len(), SAMPLES.len() + 1);
        assert!(initial
            .iter()
            .all(|event| event.font.as_ref().unwrap().supported));
        assert_eq!(wakes.try_iter().count(), 1);

        assert!(published.update(supported));
        assert!(events.drain().is_empty());
        assert!(rx.is_empty());
        assert!(wakes.is_empty());

        events.push(NativeEvent::paste("before font change".into()));
        let downgraded = ProbeReport {
            nerd_fonts: NerdFontSupport {
                powerline_extended: false,
                ..supported.nerd_fonts
            },
            legacy_symbols: false,
        };
        assert!(published.update(downgraded));
        let changes = events.drain();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].kind, "paste");
        assert_eq!(
            changes[1].font.as_ref().unwrap().set,
            Some(NerdFontSet::PowerlineExtended)
        );
        assert_eq!(changes[2].font.as_ref().unwrap().set, None);
        assert!(changes[1..]
            .iter()
            .all(|event| !event.font.as_ref().unwrap().supported));
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::SetPowerlineExtended { supported: false }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::SetLegacySymbols { supported: false }
        ));
        assert_eq!(wakes.try_iter().count(), 1);
    }

    #[test]
    fn polling_refreshes_the_environment_and_the_api_snapshot() {
        let (commands, _rx) = unbounded();
        let (wake, wakes) = unbounded();
        let events = Arc::new(NativeEventQueue::default());
        let mut pass = 0;
        let mut monitor = FontMonitor::start_with_environment(
            None,
            commands,
            events.clone(),
            Arc::new(Notify(wake)),
            move || {
                pass += 1;
                let program = if pass == 1 {
                    "ghostty"
                } else {
                    "Apple_Terminal"
                };
                Environment::from_pairs(&[("TERM_PROGRAM", program)])
            },
        );
        wakes.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(monitor.detection().detect(None).powerline_extended);
        assert_eq!(events.drain().len(), SAMPLES.len() + 1);

        wakes.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(monitor.detection().detect(None), NerdFontSupport::default());
        assert!(!monitor.detection().detect_legacy(None));
        let changes = events.drain();
        assert_eq!(changes.len(), SAMPLES.len() + 1);
        assert!(changes
            .iter()
            .all(|event| !event.font.as_ref().unwrap().supported));
        monitor.shutdown();
        assert!(wakes.recv().is_err(), "shutdown must release the notifier");
    }

    #[test]
    fn shutdown_interrupts_an_unanswered_probe() {
        let (commands, _rx) = unbounded();
        let (queries, requests) = bounded(1);
        let (wake, wakes) = unbounded();
        let mut monitor = FontMonitor::start_with_environment(
            Some(queries),
            commands,
            Arc::new(NativeEventQueue::default()),
            Arc::new(Notify(wake)),
            || Environment::from_pairs(&[]),
        );
        let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
        let before = Instant::now();
        monitor.shutdown();
        assert!(
            before.elapsed() < Duration::from_secs(1),
            "must not wait for the probe deadline"
        );
        assert!(request
            .reply
            .send(vec![Some(true); request.characters.len()])
            .is_err());
        assert!(wakes.recv().is_err());
    }
}
