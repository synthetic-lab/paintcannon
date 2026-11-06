//! One environment snapshot and its shared, lazily queried capabilities.
use super::environment::Environment;
use crate::glyph_detection::ProbeReport;
use crate::notifications::query::{Request, Support};
use crossbeam_channel::{never, Receiver, Sender};
use std::sync::OnceLock;

pub(crate) struct Detection {
    pub(crate) environment: OnceLock<Environment>,
    pub(crate) probed: OnceLock<ProbeReport>,
    pub(crate) notification_support: OnceLock<Option<Support>>,
    pub(crate) notification_queries: Option<Sender<Request>>,
    pub(crate) stop: Receiver<()>,
}

impl Default for Detection {
    fn default() -> Self {
        Self {
            environment: OnceLock::new(),
            probed: OnceLock::new(),
            notification_support: OnceLock::new(),
            notification_queries: None,
            stop: never(),
        }
    }
}

impl Detection {
    pub(crate) fn environment(&self) -> &Environment {
        self.environment.get_or_init(Environment::current)
    }
}
