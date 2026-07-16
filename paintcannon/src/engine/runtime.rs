use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError};
use termprofile::TermProfile;

use super::{
    profile_log, scrollbar_suppresses_selection, EngineCommand, PaintEngine, SelectionAction,
};
use crate::event_notification::EventNotification;
use crate::native_event::{NativeEvent, NativeEventQueue};
use crate::style::Background;
use crate::terminal::copy_text_to_clipboard;

pub(crate) struct EngineLoopOptions {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) fps: f64,
    pub(crate) color_profile: TermProfile,
    pub(crate) synchronized: bool,
    pub(crate) terminal_foreground: Background,
    pub(crate) terminal_background: Background,
    pub(crate) event_queue: Arc<NativeEventQueue>,
    pub(crate) event_notifier: Arc<dyn EventNotification>,
}

pub(super) struct EngineLoopState {
    pub(super) width: usize,
    pub(super) height: usize,
    frame_interval: Duration,
    next_frame: Instant,
    color_profile: TermProfile,
    synchronized: bool,
    event_queue: Arc<NativeEventQueue>,
    event_notifier: Arc<dyn EventNotification>,
}

impl EngineLoopState {
    pub(super) fn new(options: &EngineLoopOptions) -> Self {
        let frame_interval = frame_interval(options.fps);
        Self {
            width: options.width,
            height: options.height,
            frame_interval,
            next_frame: Instant::now() + frame_interval,
            color_profile: options.color_profile,
            synchronized: options.synchronized,
            event_queue: Arc::clone(&options.event_queue),
            event_notifier: Arc::clone(&options.event_notifier),
        }
    }

    fn set_frame_rate(&mut self, fps: f64) {
        self.frame_interval = frame_interval(fps);
        self.next_frame = Instant::now() + self.frame_interval;
    }
}

pub(crate) fn engine_loop(rx: Receiver<EngineCommand>, options: EngineLoopOptions) {
    let mut engine = PaintEngine::new();
    engine.truecolor_enabled = options.color_profile == TermProfile::TrueColor;
    engine.terminal_foreground = options.terminal_foreground;
    engine.terminal_background = options.terminal_background;
    let mut state = EngineLoopState::new(&options);

    loop {
        let now = Instant::now();
        if now < state.next_frame {
            match rx.recv_timeout(state.next_frame - now) {
                Ok(command) => {
                    if !apply_command(&mut engine, &mut state, command) {
                        break;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        let flush_result = if engine.prepare_frame_tick() {
            let frame_time = Instant::now();
            let mut out = io::stdout().lock();
            engine.flush_dirty_frame_to(
                state.width,
                state.height,
                &mut out,
                state.color_profile,
                state.synchronized,
                frame_time,
            )
        } else {
            Ok(false)
        };
        if flush_result.is_err() {
            engine.mark_paint_dirty();
        }
        publish_transition_events(&mut engine, &state);
        state.next_frame =
            next_frame_deadline(state.next_frame, state.frame_interval, Instant::now());
    }
}

pub(super) fn apply_command(
    engine: &mut PaintEngine,
    state: &mut EngineLoopState,
    command: EngineCommand,
) -> bool {
    engine.begin_transition_batch();
    let keep_running = apply_command_inner(engine, state, command);
    engine.finish_transition_batch(Instant::now());
    publish_transition_events(engine, state);
    keep_running
}

fn apply_command_inner(
    engine: &mut PaintEngine,
    state: &mut EngineLoopState,
    command: EngineCommand,
) -> bool {
    match command {
        EngineCommand::Batch { commands } => {
            let start = Instant::now();
            let command_count = commands.len();
            engine.reserve_for_batch(&commands);
            let mut pending_destroys = Vec::new();
            for command in commands {
                if let EngineCommand::DestroyNode { node } = command {
                    pending_destroys.push(node);
                    continue;
                }
                if !pending_destroys.is_empty() {
                    engine.destroy_nodes(pending_destroys.drain(..));
                }
                if !apply_command_inner(engine, state, command) {
                    profile_log(
                        "batch_apply",
                        start.elapsed(),
                        &[
                            ("commands", command_count.to_string()),
                            ("shutdown", true.to_string()),
                        ],
                    );
                    return false;
                }
            }
            if !pending_destroys.is_empty() {
                engine.destroy_nodes(pending_destroys);
            }
            profile_log(
                "batch_apply",
                start.elapsed(),
                &[
                    ("commands", command_count.to_string()),
                    ("shutdown", false.to_string()),
                ],
            );
        }
        #[cfg(test)]
        EngineCommand::CreateElement { style, response } => {
            let _ = response.send(engine.create_element(style));
        }
        EngineCommand::CreateElementWithId { id, style } => {
            engine.create_element_with_id(id, style);
        }
        #[cfg(test)]
        EngineCommand::CreateText { text, response } => {
            let _ = response.send(engine.create_text(text));
        }
        EngineCommand::CreateTextWithId { id, text } => {
            engine.create_text_with_id(id, text);
        }
        EngineCommand::CreateImageWithId {
            id,
            style,
            width_px,
            height_px,
            cell_width_px,
            cell_height_px,
        } => {
            engine.create_image_with_id(
                id,
                style,
                width_px,
                height_px,
                cell_width_px,
                cell_height_px,
            );
        }
        EngineCommand::CreateInputWithId { id, style, value } => {
            engine.create_input_with_id(id, style, value);
        }
        EngineCommand::CreateTextAreaWithId { id, style, value } => {
            engine.create_textarea_with_id(id, style, value);
        }
        EngineCommand::AppendChild { parent, child } => {
            engine.append_child(parent, child);
        }
        EngineCommand::InsertChildBefore {
            parent,
            child,
            before,
        } => {
            engine.insert_child_before(parent, child, before);
        }
        EngineCommand::SetRoot { root } => {
            engine.set_root(root);
        }
        EngineCommand::SetViewport { viewport } => {
            engine.set_viewport(viewport);
        }
        EngineCommand::DestroyNode { node } => {
            engine.destroy_node(node);
        }
        EngineCommand::DetachNode { node } => {
            engine.detach_node(node);
        }
        EngineCommand::MutateStyle { node, mutation } => {
            engine.mutate_style(node, mutation);
        }
        EngineCommand::SetTransition { node, transitions } => {
            engine.set_transition(node, transitions);
        }
        EngineCommand::SetText { node, text } => {
            engine.set_text(node, text);
        }
        EngineCommand::SetImageSource { node, src } => {
            engine.set_image_source(node, src);
        }
        EngineCommand::SetInputValue {
            node,
            value,
            cursor,
        } => {
            engine.set_input_value(node, value, cursor);
        }
        EngineCommand::SetInputFocused { node, focused } => {
            engine.set_input_focused(node, focused);
        }
        EngineCommand::SetInputPlaceholder { node, placeholder } => {
            engine.set_input_placeholder(node, placeholder);
        }
        EngineCommand::SetTextAreaValue {
            node,
            value,
            cursor,
        } => {
            engine.set_textarea_value(node, value, cursor);
        }
        EngineCommand::SetTextAreaFocused { node, focused } => {
            engine.set_textarea_focused(node, focused);
        }
        EngineCommand::SetTextAreaPlaceholder { node, placeholder } => {
            engine.set_textarea_placeholder(node, placeholder);
        }
        EngineCommand::MoveTextAreaCursorVertically {
            node,
            direction,
            response,
        } => {
            let _ = response.send(engine.move_textarea_cursor_vertically_for_size(
                node,
                direction,
                state.width,
                state.height,
            ));
        }
        EngineCommand::GetTextAreaCursorVisualPosition { node, response } => {
            let _ = response.send(engine.textarea_cursor_visual_position_for_size(
                node,
                state.width,
                state.height,
            ));
        }
        EngineCommand::GetTextAreaVisualLineRange {
            node,
            row,
            response,
        } => {
            let _ = response.send(engine.textarea_visual_line_range_for_size(
                node,
                row,
                state.width,
                state.height,
            ));
        }
        EngineCommand::SetTextControlCursorAtPoint {
            node,
            x,
            y,
            response,
        } => {
            let _ = response.send(engine.set_text_control_cursor_at_point_for_size(
                node,
                x,
                y,
                state.width,
                state.height,
            ));
        }
        EngineCommand::SetScrollOffset {
            node,
            scroll_left,
            scroll_top,
            response,
        } => {
            let _ = response.send(engine.set_scroll_offset_for_size(
                node,
                scroll_left,
                scroll_top,
                state.width,
                state.height,
            ));
        }
        EngineCommand::GetScrollMetrics { node, response } => {
            let _ = response.send(engine.scroll_metrics_for_size(node, state.width, state.height));
        }
        EngineCommand::HitTestPoint { x, y, response } => {
            let _ = response.send(engine.target_at(x, y));
        }
        EngineCommand::HitTestClick { click, response } => {
            let _ = response.send(engine.click_event_for(click));
        }
        EngineCommand::HitTestScrollbar { x, y, response } => {
            let _ = response.send(engine.scrollbar_hit_at(x, y));
        }
        EngineCommand::HandleSelection { event, response } => {
            if scrollbar_suppresses_selection(engine, event) {
                let _ = response.send(SelectionAction::None);
                return true;
            }
            let action = engine.handle_selection_event(event);
            if let SelectionAction::CopyToClipboard(text) = &action {
                copy_text_to_clipboard(text);
            }
            if matches!(
                &action,
                SelectionAction::Redraw | SelectionAction::CopyToClipboard(_)
            ) {
                engine.mark_paint_dirty();
            }
            let _ = response.send(action);
        }
        EngineCommand::HandlePointerMove { x, y } => {
            engine.handle_pointer_move(x, y);
        }
        #[cfg(test)]
        EngineCommand::RenderFrame {
            width,
            height,
            response,
        } => {
            let _ = response.send(engine.render_frame(width, height));
        }
        EngineCommand::FlushFrame { response } => {
            let result = {
                let mut out = io::stdout().lock();
                engine.flush_frame_to_at(
                    state.width,
                    state.height,
                    &mut out,
                    state.color_profile,
                    state.synchronized,
                    Instant::now(),
                )
            };
            let _ = response.send(result);
        }
        EngineCommand::SetRenderSize { width, height } => {
            if state.width != width || state.height != height {
                state.width = width;
                state.height = height;
                engine.mark_layout_dirty();
            }
        }
        EngineCommand::SetFrameRate { fps } => {
            state.set_frame_rate(fps);
        }
        EngineCommand::SetTerminalFocused { focused } => {
            engine.set_terminal_focused(focused);
        }
        EngineCommand::InvalidateFrame => engine.invalidate_frame(),
        EngineCommand::Shutdown { response } => {
            if let Some(response) = response {
                let _ = response.send(());
            }
            return false;
        }
    }

    true
}

fn publish_transition_events(engine: &mut PaintEngine, state: &EngineLoopState) {
    let events = engine.drain_transition_events();
    if events.is_empty() {
        return;
    }
    for event in events {
        state.event_queue.push(NativeEvent::transition(event));
    }
    state.event_notifier.notify();
}

fn frame_interval(fps: f64) -> Duration {
    Duration::from_secs_f64(1.0 / fps).max(Duration::from_nanos(1))
}

pub(super) fn next_frame_deadline(previous: Instant, interval: Duration, now: Instant) -> Instant {
    if previous > now {
        return previous;
    }

    let interval_nanos = interval.as_nanos();
    let elapsed_nanos = now.duration_since(previous).as_nanos();
    let steps = elapsed_nanos / interval_nanos + 1;
    let advance_nanos = interval_nanos.saturating_mul(steps).min(u64::MAX as u128) as u64;
    previous
        .checked_add(Duration::from_nanos(advance_nanos))
        .unwrap_or(now + interval)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    use crossbeam_channel::bounded;

    use super::*;
    use crate::engine::{Dirtiness, DomId, StyleMutation};
    use crate::style::{
        CssDimension, DivStyle, LayoutOverflow, TransitionProperty, TransitionSpec,
    };

    struct NoopEventNotification;

    impl EventNotification for NoopEventNotification {
        fn notify(&self) {}
    }

    #[derive(Default)]
    struct RecordingEventNotification {
        notifications: AtomicUsize,
    }

    impl EventNotification for RecordingEventNotification {
        fn notify(&self) {
            self.notifications.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn block_style(width: CssDimension, height: CssDimension) -> DivStyle {
        DivStyle {
            width,
            height,
            ..DivStyle::default()
        }
    }

    fn test_loop_options() -> EngineLoopOptions {
        EngineLoopOptions {
            width: 80,
            height: 24,
            fps: 0.001,
            color_profile: TermProfile::TrueColor,
            synchronized: false,
            terminal_foreground: Background::White,
            terminal_background: Background::Black,
            event_queue: Arc::new(NativeEventQueue::default()),
            event_notifier: Arc::new(NoopEventNotification),
        }
    }

    #[test]
    fn frame_deadlines_keep_the_original_cadence_and_skip_missed_ticks() {
        let start = Instant::now();
        let interval = Duration::from_millis(10);

        assert_eq!(
            next_frame_deadline(
                start + interval,
                interval,
                start + Duration::from_millis(12)
            ),
            start + Duration::from_millis(20)
        );
        assert_eq!(
            next_frame_deadline(
                start + interval,
                interval,
                start + Duration::from_millis(45)
            ),
            start + Duration::from_millis(50)
        );
    }

    #[test]
    fn render_size_changes_dirty_layout_without_rendering_immediately() {
        let mut engine = PaintEngine::new();
        let mut state = EngineLoopState::new(&test_loop_options());
        engine.dirtiness = Dirtiness::Clean;

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::SetRenderSize {
                width: 100,
                height: 40,
            },
        ));

        assert_eq!((state.width, state.height), (100, 40));
        assert_eq!(engine.dirtiness, Dirtiness::Layout);
        assert!(engine.previous_frame.is_none());
    }

    #[test]
    fn transition_events_are_queued_and_notify_javascript() {
        let mut engine = PaintEngine::new();
        let id = engine.create_element(DivStyle::default());
        let node = engine.node_for(id).unwrap();
        engine.transitions.set_specs(
            node,
            vec![TransitionSpec {
                property: TransitionProperty::Opacity,
                duration_ms: 100,
            }],
        );
        engine
            .transitions
            .style_opacity_changed(node, 1.0, 0.5, Instant::now(), true);
        let event_queue = Arc::new(NativeEventQueue::default());
        let notifier = Arc::new(RecordingEventNotification::default());
        let state = EngineLoopState::new(&EngineLoopOptions {
            event_queue: Arc::clone(&event_queue),
            event_notifier: notifier.clone(),
            ..test_loop_options()
        });

        publish_transition_events(&mut engine, &state);

        assert_eq!(notifier.notifications.load(Ordering::Relaxed), 1);
        let events = event_queue.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "transition");
        assert_eq!(
            events[0]
                .transition
                .as_ref()
                .map(|event| event.r#type.as_str()),
            Some("transitionstart")
        );
    }

    #[test]
    fn later_batch_starts_opacity_transition_before_first_frame() {
        let mut engine = PaintEngine::new();
        let event_queue = Arc::new(NativeEventQueue::default());
        let mut state = EngineLoopState::new(&EngineLoopOptions {
            event_queue: Arc::clone(&event_queue),
            ..test_loop_options()
        });
        let root = DomId(1);
        let mut initial_style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        initial_style.opacity = 0.0;

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::CreateElementWithId {
                        id: root,
                        style: initial_style,
                    },
                    EngineCommand::SetTransition {
                        node: root,
                        transitions: vec![TransitionSpec {
                            property: TransitionProperty::Opacity,
                            duration_ms: 100,
                        }],
                    },
                    EngineCommand::SetRoot { root },
                ],
            },
        ));
        assert!(!engine.has_active_transitions());
        assert!(event_queue.drain().is_empty());
        assert_eq!(engine.layout_passes(), 0);

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![EngineCommand::MutateStyle {
                    node: root,
                    mutation: StyleMutation::Opacity(0.4),
                }],
            },
        ));

        assert!(engine.has_active_transitions());
        assert_eq!(engine.layout_passes(), 0);
        let events = event_queue.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "transition");
        assert_eq!(
            events[0]
                .transition
                .as_ref()
                .map(|event| event.r#type.as_str()),
            Some("transitionstart")
        );
    }

    #[test]
    fn initial_connected_style_does_not_transition() {
        let mut engine = PaintEngine::new();
        let event_queue = Arc::new(NativeEventQueue::default());
        let mut state = EngineLoopState::new(&EngineLoopOptions {
            event_queue: Arc::clone(&event_queue),
            ..test_loop_options()
        });
        let root = DomId(1);
        let mut style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        style.opacity = 0.5;

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::CreateElementWithId { id: root, style },
                    EngineCommand::SetTransition {
                        node: root,
                        transitions: vec![TransitionSpec {
                            property: TransitionProperty::Opacity,
                            duration_ms: 100,
                        }],
                    },
                    EngineCommand::SetRoot { root },
                ],
            },
        ));

        assert!(!engine.has_active_transitions());
        assert!(event_queue.drain().is_empty());
        assert_eq!(engine.style_for(root).unwrap().opacity, 0.5);
    }

    #[test]
    fn detached_initial_styles_establish_when_the_subtree_is_attached() {
        let mut engine = PaintEngine::new();
        let event_queue = Arc::new(NativeEventQueue::default());
        let mut state = EngineLoopState::new(&EngineLoopOptions {
            event_queue: Arc::clone(&event_queue),
            ..test_loop_options()
        });
        let root = DomId(1);
        let overlay = DomId(2);

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::CreateElementWithId {
                        id: root,
                        style: DivStyle::default(),
                    },
                    EngineCommand::SetRoot { root },
                ],
            },
        ));
        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::CreateElementWithId {
                id: overlay,
                style: DivStyle::default(),
            },
        ));
        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::SetTransition {
                node: overlay,
                transitions: vec![TransitionSpec {
                    property: TransitionProperty::Opacity,
                    duration_ms: 100,
                }],
            },
        ));
        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::MutateStyle {
                node: overlay,
                mutation: StyleMutation::Opacity(0.0),
            },
        ));
        assert!(!engine.has_active_transitions());

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::AppendChild {
                parent: root,
                child: overlay,
            },
        ));
        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::MutateStyle {
                node: overlay,
                mutation: StyleMutation::Opacity(0.4),
            },
        ));

        assert!(engine.has_active_transitions());
        assert_eq!(event_queue.drain().len(), 1);
        assert_eq!(engine.layout_passes(), 0);
    }

    #[test]
    fn batch_uses_final_transition_declaration_regardless_of_command_order() {
        let mut engine = PaintEngine::new();
        let event_queue = Arc::new(NativeEventQueue::default());
        let mut state = EngineLoopState::new(&EngineLoopOptions {
            event_queue: Arc::clone(&event_queue),
            ..test_loop_options()
        });
        let root = DomId(1);
        let mut style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        style.opacity = 0.0;
        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::CreateElementWithId { id: root, style },
                    EngineCommand::SetRoot { root },
                ],
            },
        ));

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::MutateStyle {
                        node: root,
                        mutation: StyleMutation::Opacity(0.5),
                    },
                    EngineCommand::SetTransition {
                        node: root,
                        transitions: vec![TransitionSpec {
                            property: TransitionProperty::Opacity,
                            duration_ms: 100,
                        }],
                    },
                ],
            },
        ));

        assert!(engine.has_active_transitions());
        assert_eq!(event_queue.drain().len(), 1);
    }

    #[test]
    fn batch_transitions_to_the_final_style_value() {
        let mut engine = PaintEngine::new();
        let mut state = EngineLoopState::new(&test_loop_options());
        let root = DomId(1);
        let mut style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        style.opacity = 0.0;
        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::CreateElementWithId { id: root, style },
                    EngineCommand::SetTransition {
                        node: root,
                        transitions: vec![TransitionSpec {
                            property: TransitionProperty::Opacity,
                            duration_ms: 100,
                        }],
                    },
                    EngineCommand::SetRoot { root },
                ],
            },
        ));

        assert!(apply_command(
            &mut engine,
            &mut state,
            EngineCommand::Batch {
                commands: vec![
                    EngineCommand::MutateStyle {
                        node: root,
                        mutation: StyleMutation::Opacity(0.2),
                    },
                    EngineCommand::MutateStyle {
                        node: root,
                        mutation: StyleMutation::Opacity(0.7),
                    },
                ],
            },
        ));

        let node = engine.node_for(root).unwrap();
        assert_eq!(engine.style_for(root).unwrap().opacity, 0.7);
        assert_eq!(
            engine.transitions.paint_opacity(
                node,
                0.7,
                Instant::now() + Duration::from_millis(200),
                true,
            ),
            0.7,
        );
    }

    #[test]
    fn batched_sibling_destruction_preserves_survivors_and_reuses_slots() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(DivStyle::default());
        let survivor = engine.create_element(DivStyle::default());
        assert!(engine.append_child(root, survivor));

        let removed = (0..1_000)
            .map(|_| {
                let child = engine.create_element(DivStyle::default());
                assert!(engine.append_child(root, child));
                child
            })
            .collect::<Vec<_>>();
        let allocated_slots = engine.arena.stats().allocated_slot_count;
        let mut loop_state = EngineLoopState::new(&test_loop_options());

        assert!(apply_command(
            &mut engine,
            &mut loop_state,
            EngineCommand::Batch {
                commands: removed
                    .iter()
                    .map(|node| EngineCommand::DestroyNode { node: *node })
                    .collect(),
            },
        ));

        assert_eq!(engine.children.get(&root), Some(&vec![survivor]));
        assert_eq!(
            engine.arena.children(engine.node_for(root).unwrap()).len(),
            1
        );
        assert_eq!(engine.arena.stats().node_count, 2);
        for node in removed {
            assert!(engine.node_for(node).is_none());
        }

        for _ in 0..1_000 {
            engine.create_element(DivStyle::default());
        }
        assert_eq!(engine.arena.stats().allocated_slot_count, allocated_slots);
    }

    #[test]
    fn command_loop_creates_renders_and_hit_tests() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx, test_loop_options()));

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::CreateElement {
            style: block_style(CssDimension::Length(4.0), CssDimension::Length(1.0)),
            response: response_tx,
        })
        .unwrap();
        let root = response_rx.recv().unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::CreateText {
            text: "hi".to_string(),
            response: response_tx,
        })
        .unwrap();
        let text = response_rx.recv().unwrap();

        tx.send(EngineCommand::AppendChild {
            parent: root,
            child: text,
        })
        .unwrap();
        tx.send(EngineCommand::SetRoot { root }).unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::RenderFrame {
            width: 4,
            height: 1,
            response: response_tx,
        })
        .unwrap();
        let frame = response_rx.recv().unwrap().unwrap();
        assert_eq!(frame.cell(0, 0).unwrap().character, 'h');

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::HitTestPoint {
            x: 0,
            y: 0,
            response: response_tx,
        })
        .unwrap();
        assert_eq!(response_rx.recv().unwrap(), Some(root));

        tx.send(EngineCommand::Shutdown { response: None }).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn command_loop_batches_explicit_id_creates() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx, test_loop_options()));

        tx.send(EngineCommand::Batch {
            commands: vec![
                EngineCommand::CreateElementWithId {
                    id: DomId(1),
                    style: block_style(CssDimension::Length(4.0), CssDimension::Length(1.0)),
                },
                EngineCommand::CreateTextWithId {
                    id: DomId(2),
                    text: "ok".to_string(),
                },
                EngineCommand::AppendChild {
                    parent: DomId(1),
                    child: DomId(2),
                },
                EngineCommand::SetRoot { root: DomId(1) },
            ],
        })
        .unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::RenderFrame {
            width: 4,
            height: 1,
            response: response_tx,
        })
        .unwrap();
        let frame = response_rx.recv().unwrap().unwrap();

        assert_eq!(frame.cell(0, 0).unwrap().character, 'o');
        assert_eq!(frame.cell(1, 0).unwrap().character, 'k');

        tx.send(EngineCommand::Shutdown { response: None }).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn command_loop_acknowledges_shutdown_after_earlier_commands() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx, test_loop_options()));

        tx.send(EngineCommand::CreateTextWithId {
            id: DomId(1),
            text: "queued before shutdown".to_string(),
        })
        .unwrap();
        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::Shutdown {
            response: Some(response_tx),
        })
        .unwrap();

        response_rx.recv().unwrap();
        thread.join().unwrap();
        assert!(tx.send(EngineCommand::InvalidateFrame).is_err());
    }

    #[test]
    fn command_loop_scroll_metrics_use_current_render_size() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx, test_loop_options()));

        let mut viewport_style =
            block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        tx.send(EngineCommand::CreateElementWithId {
            id: DomId(1),
            style: viewport_style,
        })
        .unwrap();
        tx.send(EngineCommand::CreateElementWithId {
            id: DomId(2),
            style: block_style(CssDimension::Length(10.0), CssDimension::Length(20.0)),
        })
        .unwrap();
        tx.send(EngineCommand::AppendChild {
            parent: DomId(1),
            child: DomId(2),
        })
        .unwrap();
        tx.send(EngineCommand::SetRoot { root: DomId(1) }).unwrap();

        tx.send(EngineCommand::SetRenderSize {
            width: 10,
            height: 5,
        })
        .unwrap();
        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::GetScrollMetrics {
            node: DomId(1),
            response: response_tx,
        })
        .unwrap();
        let small = response_rx.recv().unwrap().unwrap();

        tx.send(EngineCommand::SetRenderSize {
            width: 10,
            height: 12,
        })
        .unwrap();
        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::GetScrollMetrics {
            node: DomId(1),
            response: response_tx,
        })
        .unwrap();
        let large = response_rx.recv().unwrap().unwrap();

        assert_eq!(small.client_height, 5);
        assert_eq!(small.scroll_height, 20);
        assert_eq!(large.client_height, 12);
        assert_eq!(large.scroll_height, 20);

        tx.send(EngineCommand::Shutdown { response: None }).unwrap();
        thread.join().unwrap();
    }
}
