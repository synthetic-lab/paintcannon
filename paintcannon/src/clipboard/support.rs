use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use napi::{Env, Result, Task};
use napi_derive::napi;

const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(crate) struct Support {
    value: Mutex<Option<bool>>,
    changed: Condvar,
}

impl Support {
    pub(super) fn set(&self, supported: Option<bool>) {
        if let Ok(mut value) = self.value.lock() {
            *value = supported;
            self.changed.notify_all();
        }
    }

    pub(super) fn wait(&self, timeout: Duration) -> bool {
        let Ok(value) = self.value.lock() else {
            return false;
        };
        let Ok((mut value, _)) = self
            .changed
            .wait_timeout_while(value, timeout, |value| value.is_none())
        else {
            return false;
        };
        // Cache an unanswered query, but allow a late reply or a resumed
        // terminal's negotiation to replace it. No repeated two-second waits.
        let supported = *value.get_or_insert(false);
        self.changed.notify_all();
        supported
    }
}

pub struct RichPasteSupport {
    pub(crate) support: Option<Arc<Support>>,
}

#[napi]
impl Task for RichPasteSupport {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<bool> {
        Ok(self
            .support
            .as_ref()
            .is_some_and(|support| support.wait(QUERY_TIMEOUT)))
    }

    fn resolve(&mut self, _env: Env, output: bool) -> Result<bool> {
        Ok(output)
    }
}
