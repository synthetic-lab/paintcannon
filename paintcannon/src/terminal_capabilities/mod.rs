//! Shared terminal detection, including live tmux client capabilities.
mod capabilities;
mod detection;
pub(crate) mod environment;
mod environment_markers;
mod monitor;
mod ssh;
mod terminals;
mod tmux;

pub(crate) use capabilities::{Capabilities, HyperlinkCapability, NotificationProtocols};
pub(crate) use detection::Detection;
pub(crate) use monitor::CapabilityMonitor;
