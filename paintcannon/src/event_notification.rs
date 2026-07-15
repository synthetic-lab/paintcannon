use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use napi::bindgen_prelude::Function;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{Result, Status};

type EventCallback = ThreadsafeFunction<(), (), (), Status, false, false, 1>;

pub(crate) struct EventNotifier {
    callback: EventCallback,
    gate: Arc<NotificationGate>,
}

pub(crate) trait EventNotification: Send + Sync {
    fn notify(&self);
}

impl EventNotifier {
    pub(crate) fn new(callback: Function<'_, (), ()>) -> Result<Self> {
        let gate = Arc::new(NotificationGate::default());
        let callback_gate = Arc::clone(&gate);
        let callback = callback
            .build_threadsafe_function::<()>()
            .max_queue_size::<1>()
            .build_callback(move |_| {
                callback_gate.acknowledge();
                Ok(())
            })?;
        Ok(Self { callback, gate })
    }

    fn send_notification(&self) {
        if !self.gate.schedule() {
            return;
        }

        if self
            .callback
            .call((), ThreadsafeFunctionCallMode::NonBlocking)
            != Status::Ok
        {
            self.gate.acknowledge();
        }
    }
}

impl EventNotification for EventNotifier {
    fn notify(&self) {
        self.send_notification();
    }
}

#[derive(Default)]
struct NotificationGate {
    scheduled: AtomicBool,
}

impl NotificationGate {
    fn schedule(&self) -> bool {
        self.scheduled
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn acknowledge(&self) {
        self.scheduled.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_gate_coalesces_until_acknowledged() {
        let gate = NotificationGate::default();

        assert!(gate.schedule());
        assert!(!gate.schedule());

        gate.acknowledge();
        assert!(gate.schedule());
    }
}
