use std::collections::VecDeque;
use std::sync::Mutex;

use napi_derive::napi;

use crate::engine::{EngineIntersectionEvent, EngineTransitionEvent};
use crate::layout::AbsoluteRect;
use crate::style::TransitionProperty;
use crate::transition::TransitionEventType;

#[derive(Clone)]
#[napi(object)]
pub struct KeyboardEvent {
    pub r#type: String,
    pub key: String,
    pub code: String,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
    pub repeat: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct TerminalMouseEvent {
    pub r#type: String,
    pub x: u32,
    pub y: u32,
    pub button: u32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct TerminalResizeEvent {
    pub cols: u32,
    pub rows: u32,
}

#[derive(Clone)]
#[napi(object)]
pub struct TerminalFocusEvent {
    pub r#type: String,
}

#[derive(Clone)]
#[napi(object)]
pub struct TransitionEvent {
    pub r#type: String,
    pub target_id: u32,
    pub property_name: String,
}

#[derive(Clone, Debug)]
#[napi(object)]
pub struct IntersectionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Debug)]
#[napi(object)]
pub struct IntersectionEntry {
    pub target_id: u32,
    pub bounding_client_rect: IntersectionRect,
    pub intersection_rect: IntersectionRect,
    pub root_bounds: IntersectionRect,
    pub is_intersecting: bool,
    pub intersection_ratio: f64,
    pub time: f64,
}

#[derive(Clone, Debug)]
#[napi(object)]
pub struct IntersectionEvent {
    pub observer_id: u32,
    pub entries: Vec<IntersectionEntry>,
}

#[derive(Clone)]
#[napi(object)]
pub struct ClipboardWritePayload {
    pub text: String,
    pub success: bool,
}

#[derive(Clone)]
#[napi(object)]
pub struct NativeEvent {
    pub kind: String,
    pub keyboard: Option<KeyboardEvent>,
    pub paste: Option<String>,
    pub clipboard_write: Option<ClipboardWritePayload>,
    pub mouse: Option<TerminalMouseEvent>,
    pub resize: Option<TerminalResizeEvent>,
    pub focus: Option<TerminalFocusEvent>,
    pub transition: Option<TransitionEvent>,
    pub intersection: Option<IntersectionEvent>,
}

impl NativeEvent {
    pub(crate) fn keyboard(event: KeyboardEvent) -> Self {
        Self::with_kind("keyboard", |native| native.keyboard = Some(event))
    }

    pub(crate) fn paste(data: String) -> Self {
        Self::with_kind("paste", |native| native.paste = Some(data))
    }

    pub(crate) fn clipboard_write(text: String, success: bool) -> Self {
        Self::with_kind("clipboardWrite", |native| {
            native.clipboard_write = Some(ClipboardWritePayload { text, success })
        })
    }

    pub(crate) fn mouse(event: TerminalMouseEvent) -> Self {
        Self::with_kind("mouse", |native| native.mouse = Some(event))
    }

    pub(crate) fn resize(event: TerminalResizeEvent) -> Self {
        Self::with_kind("resize", |native| native.resize = Some(event))
    }

    pub(crate) fn focus(event: TerminalFocusEvent) -> Self {
        Self::with_kind("focus", |native| native.focus = Some(event))
    }

    pub(crate) fn transition(event: EngineTransitionEvent) -> Self {
        let event = TransitionEvent {
            r#type: match event.event_type {
                TransitionEventType::Start => "transitionstart",
                TransitionEventType::End => "transitionend",
            }
            .to_string(),
            target_id: event.target.0,
            property_name: transition_property_name(event.property).to_string(),
        };
        Self::with_kind("transition", |native| native.transition = Some(event))
    }

    pub(crate) fn intersection(event: EngineIntersectionEvent) -> Self {
        let event = IntersectionEvent {
            observer_id: event.observer_id,
            entries: event
                .entries
                .into_iter()
                .map(|entry| IntersectionEntry {
                    target_id: entry.target.0,
                    bounding_client_rect: intersection_rect(entry.bounding_client_rect),
                    intersection_rect: intersection_rect(entry.intersection_rect),
                    root_bounds: intersection_rect(entry.root_bounds),
                    is_intersecting: entry.is_intersecting,
                    intersection_ratio: entry.intersection_ratio,
                    time: entry.time,
                })
                .collect(),
        };
        Self::with_kind("intersection", |native| native.intersection = Some(event))
    }

    fn with_kind(kind: &str, set_payload: impl FnOnce(&mut Self)) -> Self {
        let mut event = Self {
            kind: kind.to_string(),
            keyboard: None,
            paste: None,
            clipboard_write: None,
            mouse: None,
            resize: None,
            focus: None,
            transition: None,
            intersection: None,
        };
        set_payload(&mut event);
        event
    }
}

fn intersection_rect(rect: AbsoluteRect) -> IntersectionRect {
    IntersectionRect {
        x: rect.left as f64,
        y: rect.top as f64,
        width: rect.width() as f64,
        height: rect.height() as f64,
        top: rect.top as f64,
        right: rect.right as f64,
        bottom: rect.bottom as f64,
        left: rect.left as f64,
    }
}

#[derive(Default)]
pub(crate) struct NativeEventQueue {
    events: Mutex<VecDeque<NativeEvent>>,
}

impl NativeEventQueue {
    pub(crate) fn push(&self, event: NativeEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push_back(event);
        }
    }

    pub(crate) fn drain(&self) -> Vec<NativeEvent> {
        let Ok(mut events) = self.events.lock() else {
            return Vec::new();
        };
        events.drain(..).collect()
    }
}

fn transition_property_name(property: TransitionProperty) -> &'static str {
    match property {
        TransitionProperty::Color => "color",
        TransitionProperty::BackgroundColor => "background-color",
        TransitionProperty::BorderColor => "border-color",
        TransitionProperty::Opacity => "opacity",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_preserves_mixed_event_order_without_coalescing_resizes() {
        let queue = NativeEventQueue::default();
        queue.push(NativeEvent::keyboard(KeyboardEvent {
            r#type: "keydown".to_string(),
            key: "a".to_string(),
            code: "KeyA".to_string(),
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            shift_key: false,
            repeat: false,
        }));
        queue.push(NativeEvent::resize(TerminalResizeEvent {
            cols: 80,
            rows: 24,
        }));
        queue.push(NativeEvent::focus(TerminalFocusEvent {
            r#type: "blur".to_string(),
        }));
        queue.push(NativeEvent::resize(TerminalResizeEvent {
            cols: 100,
            rows: 40,
        }));

        let events = queue.drain();
        assert_eq!(
            events
                .iter()
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            vec!["keyboard", "resize", "focus", "resize"]
        );
        assert_eq!(events[1].resize.as_ref().map(|event| event.cols), Some(80));
        assert_eq!(events[3].resize.as_ref().map(|event| event.cols), Some(100));
        assert!(queue.drain().is_empty());
    }

    #[test]
    fn opacity_transition_events_use_the_css_property_name() {
        assert_eq!(
            transition_property_name(TransitionProperty::Opacity),
            "opacity"
        );
    }
}
