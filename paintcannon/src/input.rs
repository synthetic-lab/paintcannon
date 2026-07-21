use std::collections::HashMap;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossterm::{
    event::{
        self as terminal_event, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture,
        EnableBracketedPaste, EnableFocusChange, EnableMouseCapture, Event as TerminalEvent,
        KeyCode, KeyEvent as TerminalKeyEvent, KeyEventKind, KeyModifiers,
        KeyboardEnhancementFlags, MouseButton, MouseEvent as CrosstermMouseEvent, MouseEventKind,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use crate::engine::EngineCommand;
use crate::event_notification::EventNotification;
use crate::native_event::{
    KeyboardEvent, NativeEvent, NativeEventQueue, TerminalFocusEvent, TerminalMouseEvent,
    TerminalResizeEvent,
};
use crate::selection::{SelectionAction, SelectionMouseEvent, SelectionMouseEventType};
use crate::terminal::{
    reset_pointer_shape, reset_terminal, try_query_terminal_size, try_query_tmux_pane_active,
};

const DEFAULT_SYNTHETIC_KEYUP_MS: u32 = 180;
const TERMINAL_SIZE_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) struct TerminalInput {
    focused: Arc<AtomicBool>,
    interrupted_by_ctrl_c: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    synthetic_keyup_delay_ms: Arc<Mutex<u32>>,
    kitty_keyboard_enabled: bool,
    force_compat_mode: bool,
    alternate_screen: bool,
    capture_mouse: bool,
    keyboard_enhancement_pushed: Arc<Mutex<bool>>,
    alternate_screen_entered: Arc<Mutex<bool>>,
    mouse_capture_enabled: Arc<Mutex<bool>>,
    focus_change_enabled: Arc<Mutex<bool>>,
    bracketed_paste_enabled: Arc<Mutex<bool>>,
    terminal_captured: Arc<Mutex<bool>>,
    thread: JoinHandle<()>,
}

pub(crate) struct TerminalInputOptions {
    pub(crate) synthetic_keyup_delay_ms: u32,
    pub(crate) force_compat_mode: bool,
    pub(crate) alternate_screen: bool,
    pub(crate) capture_mouse: bool,
    pub(crate) capture_ctrl_c: bool,
    pub(crate) initial_terminal_size: (u32, u32),
}

impl TerminalInput {
    pub(crate) fn start(
        options: TerminalInputOptions,
        renderer_tx: Option<crossbeam_channel::Sender<EngineCommand>>,
        event_queue: Arc<NativeEventQueue>,
        event_notifier: Arc<dyn EventNotification>,
    ) -> Option<Self> {
        let TerminalInputOptions {
            synthetic_keyup_delay_ms,
            force_compat_mode,
            alternate_screen,
            capture_mouse,
            capture_ctrl_c,
            initial_terminal_size,
        } = options;
        if enable_raw_mode().is_err() {
            return None;
        }

        let alternate_screen_entered = Arc::new(Mutex::new(false));
        if alternate_screen && execute!(io::stdout(), EnterAlternateScreen).is_ok() {
            set_bool(&alternate_screen_entered, true);
        }

        let mouse_capture_enabled = Arc::new(Mutex::new(false));
        if capture_mouse && execute!(io::stdout(), EnableMouseCapture).is_ok() {
            set_bool(&mouse_capture_enabled, true);
        }

        let focus_change_enabled = Arc::new(Mutex::new(false));

        let bracketed_paste_enabled = Arc::new(Mutex::new(false));
        if execute!(io::stdout(), EnableBracketedPaste).is_ok() {
            set_bool(&bracketed_paste_enabled, true);
        }

        let kitty_keyboard_enabled =
            !force_compat_mode && supports_keyboard_enhancement().unwrap_or(false);
        let keyboard_enhancement_pushed = Arc::new(Mutex::new(false));
        if kitty_keyboard_enabled {
            let _ = execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            );
            set_bool(&keyboard_enhancement_pushed, true);
        } else if !force_compat_mode {
            let _ = execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            );
            set_bool(&keyboard_enhancement_pushed, true);
        }

        let focused = Arc::new(AtomicBool::new(true));
        let interrupted_by_ctrl_c = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let synthetic_keyup_delay_ms = Arc::new(Mutex::new(synthetic_keyup_delay_ms));
        let terminal_captured = Arc::new(Mutex::new(true));
        let thread_event_queue = event_queue;
        let thread_focused = Arc::clone(&focused);
        let thread_interrupted_by_ctrl_c = Arc::clone(&interrupted_by_ctrl_c);
        let thread_stop = Arc::clone(&stop);
        let thread_synthetic_keyup_delay_ms = Arc::clone(&synthetic_keyup_delay_ms);
        let thread_renderer_tx = renderer_tx;
        let thread_event_notifier = event_notifier;
        let thread_keyboard_enhancement_pushed = Arc::clone(&keyboard_enhancement_pushed);
        let thread_alternate_screen_entered = Arc::clone(&alternate_screen_entered);
        let thread_mouse_capture_enabled = Arc::clone(&mouse_capture_enabled);
        let thread_focus_change_enabled = Arc::clone(&focus_change_enabled);
        let thread_bracketed_paste_enabled = Arc::clone(&bracketed_paste_enabled);
        let thread_terminal_captured = Arc::clone(&terminal_captured);
        let thread = thread::spawn(move || {
            let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();
            let mut mouse_down: Option<MouseDown> = None;
            let mut awaiting_initial_focus_report = true;
            let mut observed_terminal_size = Some(initial_terminal_size);
            let mut next_terminal_size_poll = Instant::now() + TERMINAL_SIZE_POLL_INTERVAL;
            let mut tmux_focus_check_pending = false;

            while !thread_stop.load(Ordering::Relaxed) {
                match terminal_event::poll(Duration::from_millis(25)) {
                    Ok(true) => {
                        if let Ok(event) = terminal_event::read() {
                            match event {
                                TerminalEvent::Key(event) => {
                                    if !capture_ctrl_c && is_ctrl_c_event(&event) {
                                        thread_interrupted_by_ctrl_c.store(true, Ordering::Release);
                                        stop_renderer(thread_renderer_tx.as_ref());
                                        reset_terminal(!get_bool(&thread_alternate_screen_entered));
                                        release_terminal_state(
                                            &thread_terminal_captured,
                                            &thread_mouse_capture_enabled,
                                            &thread_focus_change_enabled,
                                            &thread_bracketed_paste_enabled,
                                            &thread_keyboard_enhancement_pushed,
                                            &thread_alternate_screen_entered,
                                        );
                                        signal_process_group(libc::SIGINT);
                                        thread_stop.store(true, Ordering::Relaxed);
                                        break;
                                    }

                                    if handle_terminal_key_event(
                                        event,
                                        &mut pressed_keys,
                                        &thread_event_queue,
                                        &thread_synthetic_keyup_delay_ms,
                                        kitty_keyboard_enabled,
                                    ) {
                                        thread_event_notifier.notify();
                                    }
                                }
                                TerminalEvent::Paste(data) => {
                                    push_paste_event(&thread_event_queue, data);
                                    thread_event_notifier.notify();
                                }
                                TerminalEvent::Mouse(event) => {
                                    handle_terminal_mouse_event(
                                        event,
                                        &mut mouse_down,
                                        &thread_event_queue,
                                        if capture_mouse {
                                            thread_renderer_tx.as_ref()
                                        } else {
                                            None
                                        },
                                    );
                                    thread_event_notifier.notify();
                                }
                                TerminalEvent::Resize(cols, rows) => {
                                    if observe_terminal_size(
                                        &mut observed_terminal_size,
                                        &thread_event_queue,
                                        (u32::from(cols), u32::from(rows)),
                                        thread_renderer_tx.as_ref(),
                                    ) {
                                        tmux_focus_check_pending = true;
                                        thread_event_notifier.notify();
                                    }
                                }
                                TerminalEvent::FocusGained => {
                                    let initial_report = awaiting_initial_focus_report;
                                    awaiting_initial_focus_report = false;
                                    if handle_terminal_focus_event(
                                        true,
                                        &thread_event_queue,
                                        &thread_focused,
                                        thread_renderer_tx.as_ref(),
                                        initial_report,
                                    ) {
                                        thread_event_notifier.notify();
                                    }
                                }
                                TerminalEvent::FocusLost => {
                                    let initial_report = awaiting_initial_focus_report;
                                    awaiting_initial_focus_report = false;
                                    if handle_terminal_focus_event(
                                        false,
                                        &thread_event_queue,
                                        &thread_focused,
                                        thread_renderer_tx.as_ref(),
                                        initial_report,
                                    ) {
                                        thread_event_notifier.notify();
                                    }
                                }
                            }
                        }
                    }
                    Ok(false) => {
                        if synthesize_expired_keyups(&mut pressed_keys, &thread_event_queue) {
                            thread_event_notifier.notify();
                        }
                    }
                    Err(_) => break,
                }

                let now = Instant::now();
                if now >= next_terminal_size_poll {
                    let resized = try_query_terminal_size().is_some_and(|size| {
                        observe_terminal_size(
                            &mut observed_terminal_size,
                            &thread_event_queue,
                            terminal_cell_size(size),
                            thread_renderer_tx.as_ref(),
                        )
                    });
                    tmux_focus_check_pending |= resized;

                    let recovered_tmux_blur = if tmux_focus_check_pending {
                        tmux_focus_check_pending = false;
                        // tmux occasionally drops its focus report when selecting another pane
                        // also unzooms and resizes this pane. Recheck only after that resize, once
                        // tmux has settled, rather than polling focus during ordinary operation.
                        recover_missing_tmux_blur(
                            try_query_tmux_pane_active(),
                            &thread_event_queue,
                            &thread_focused,
                            thread_renderer_tx.as_ref(),
                        )
                    } else {
                        false
                    };
                    if recovered_tmux_blur {
                        awaiting_initial_focus_report = false;
                    }

                    if resized || recovered_tmux_blur {
                        thread_event_notifier.notify();
                    }
                    next_terminal_size_poll = next_poll_deadline(
                        next_terminal_size_poll,
                        TERMINAL_SIZE_POLL_INTERVAL,
                        now,
                    );
                }
            }

            for (_, pressed_key) in pressed_keys {
                push_keyboard_event(
                    &thread_event_queue,
                    keyboard_event_from_pressed_key("keyup", false, &pressed_key),
                );
            }
        });

        if execute!(io::stdout(), EnableFocusChange).is_ok() {
            set_bool(&focus_change_enabled, true);
        }

        Some(Self {
            focused,
            interrupted_by_ctrl_c,
            stop,
            synthetic_keyup_delay_ms,
            kitty_keyboard_enabled,
            force_compat_mode,
            alternate_screen,
            capture_mouse,
            keyboard_enhancement_pushed,
            alternate_screen_entered,
            mouse_capture_enabled,
            focus_change_enabled,
            bracketed_paste_enabled,
            terminal_captured,
            thread,
        })
    }

    pub(crate) fn has_focus(&self) -> bool {
        self.focused.load(Ordering::Relaxed)
    }

    pub(crate) fn interrupted_by_ctrl_c(&self) -> bool {
        self.interrupted_by_ctrl_c.load(Ordering::Acquire)
    }

    pub(crate) fn set_synthetic_keyup_delay(&self, delay_ms: u32) {
        if let Ok(mut current_delay) = self.synthetic_keyup_delay_ms.lock() {
            *current_delay = delay_ms;
        }
    }

    pub(crate) fn kitty_keyboard_enabled(&self) -> bool {
        self.kitty_keyboard_enabled
    }

    pub(crate) fn release_terminal(&self) {
        release_terminal_state(
            &self.terminal_captured,
            &self.mouse_capture_enabled,
            &self.focus_change_enabled,
            &self.bracketed_paste_enabled,
            &self.keyboard_enhancement_pushed,
            &self.alternate_screen_entered,
        );
    }

    pub(crate) fn is_captured(&self) -> bool {
        get_bool(&self.terminal_captured)
    }

    pub(crate) fn is_alternate_screen_entered(&self) -> bool {
        get_bool(&self.alternate_screen_entered)
    }

    pub(crate) fn capture_terminal(&self) {
        if swap_bool(&self.terminal_captured, true) {
            return;
        }

        if enable_raw_mode().is_err() {
            set_bool(&self.terminal_captured, false);
            return;
        }

        if self.alternate_screen && execute!(io::stdout(), EnterAlternateScreen).is_ok() {
            set_bool(&self.alternate_screen_entered, true);
        }

        if self.capture_mouse && execute!(io::stdout(), EnableMouseCapture).is_ok() {
            set_bool(&self.mouse_capture_enabled, true);
        }

        if execute!(io::stdout(), EnableFocusChange).is_ok() {
            set_bool(&self.focus_change_enabled, true);
        }

        if execute!(io::stdout(), EnableBracketedPaste).is_ok() {
            set_bool(&self.bracketed_paste_enabled, true);
        }

        if self.kitty_keyboard_enabled {
            let _ = execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            );
            set_bool(&self.keyboard_enhancement_pushed, true);
        } else if !self.force_compat_mode {
            let _ = execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            );
            set_bool(&self.keyboard_enhancement_pushed, true);
        }
    }

    pub(crate) fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
        self.release_terminal();
        let _ = self.thread.join();
    }
}

fn set_bool(value: &Arc<Mutex<bool>>, next: bool) {
    if let Ok(mut value) = value.lock() {
        *value = next;
    }
}

fn swap_bool(value: &Arc<Mutex<bool>>, next: bool) -> bool {
    let Ok(mut value) = value.lock() else {
        return false;
    };

    let previous = *value;
    *value = next;
    previous
}

fn get_bool(value: &Arc<Mutex<bool>>) -> bool {
    value.lock().is_ok_and(|value| *value)
}

fn stop_renderer(renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>) {
    let Some(renderer_tx) = renderer_tx else {
        return;
    };

    let (response_tx, response_rx) = crossbeam_channel::bounded(1);
    if renderer_tx
        .send(EngineCommand::Shutdown {
            response: Some(response_tx),
        })
        .is_ok()
    {
        let _ = response_rx.recv();
    }
}

fn release_terminal_state(
    terminal_captured: &Arc<Mutex<bool>>,
    mouse_capture_enabled: &Arc<Mutex<bool>>,
    focus_change_enabled: &Arc<Mutex<bool>>,
    bracketed_paste_enabled: &Arc<Mutex<bool>>,
    keyboard_enhancement_pushed: &Arc<Mutex<bool>>,
    alternate_screen_entered: &Arc<Mutex<bool>>,
) {
    if !swap_bool(terminal_captured, false) {
        return;
    }

    if swap_bool(mouse_capture_enabled, false) {
        reset_pointer_shape();
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
    if swap_bool(focus_change_enabled, false) {
        let _ = execute!(io::stdout(), DisableFocusChange);
    }
    if swap_bool(bracketed_paste_enabled, false) {
        let _ = execute!(io::stdout(), DisableBracketedPaste);
    }
    if swap_bool(keyboard_enhancement_pushed, false) {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    if swap_bool(alternate_screen_entered, false) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
    let _ = disable_raw_mode();
}

fn is_ctrl_c_event(event: &TerminalKeyEvent) -> bool {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return false;
    }

    match event.code {
        KeyCode::Char('c') | KeyCode::Char('C') => event.modifiers.contains(KeyModifiers::CONTROL),
        KeyCode::Char('\u{3}') => true,
        _ => false,
    }
}

#[cfg(unix)]
fn signal_process_group(signal: libc::c_int) {
    let _ = unsafe { libc::kill(0, signal) };
}

#[cfg(not(unix))]
fn signal_process_group(_signal: libc::c_int) {}

#[derive(Clone)]
struct PressedKey {
    key: String,
    code: String,
    ctrl_key: bool,
    alt_key: bool,
    meta_key: bool,
    shift_key: bool,
    synthetic_keyup_at: Option<Instant>,
}

struct MouseDown {
    button: MouseButton,
}

fn handle_terminal_key_event(
    event: TerminalKeyEvent,
    pressed_keys: &mut HashMap<String, PressedKey>,
    events: &NativeEventQueue,
    synthetic_keyup_delay_ms: &Arc<Mutex<u32>>,
    kitty_keyboard_enabled: bool,
) -> bool {
    let Some(pressed_key) = pressed_key_from_terminal(event) else {
        return false;
    };

    let code = pressed_key.code.clone();
    match event.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => {
            let repeat = event.kind == KeyEventKind::Repeat || pressed_keys.contains_key(&code);
            let mut pressed_key = pressed_key;
            pressed_key.synthetic_keyup_at = if kitty_keyboard_enabled
                && is_non_printable_key(&event.code) // under kitty, non-printable keys (arrows, Tab, ...) report their own keyup
            {
                None
            } else {
                synthetic_keyup_deadline(synthetic_keyup_delay_ms)
            };
            pressed_keys.insert(code, pressed_key.clone());
            push_keyboard_event(
                events,
                keyboard_event_from_pressed_key("keydown", repeat, &pressed_key),
            );
        }
        KeyEventKind::Release => {
            let pressed_key = pressed_keys.remove(&code).unwrap_or(pressed_key);
            push_keyboard_event(
                events,
                keyboard_event_from_pressed_key("keyup", false, &pressed_key),
            );
        }
    }

    true
}

fn synthesize_expired_keyups(
    pressed_keys: &mut HashMap<String, PressedKey>,
    events: &NativeEventQueue,
) -> bool {
    let now = Instant::now();
    let released: Vec<String> = pressed_keys
        .iter()
        .filter_map(|(code, pressed_key)| {
            if pressed_key
                .synthetic_keyup_at
                .is_some_and(|deadline| deadline <= now)
            {
                Some(code.clone())
            } else {
                None
            }
        })
        .collect();

    let synthesized = !released.is_empty();
    for code in released {
        if let Some(pressed_key) = pressed_keys.remove(&code) {
            push_keyboard_event(
                events,
                keyboard_event_from_pressed_key("keyup", false, &pressed_key),
            );
        }
    }
    synthesized
}

fn synthetic_keyup_deadline(delay_ms: &Arc<Mutex<u32>>) -> Option<Instant> {
    let delay_ms = delay_ms
        .lock()
        .map(|delay_ms| *delay_ms)
        .unwrap_or(DEFAULT_SYNTHETIC_KEYUP_MS);

    if delay_ms == 0 {
        None
    } else {
        Some(Instant::now() + Duration::from_millis(delay_ms.into()))
    }
}

fn push_keyboard_event(events: &NativeEventQueue, event: Option<KeyboardEvent>) {
    let Some(event) = event else {
        return;
    };
    events.push(NativeEvent::keyboard(event));
}

fn push_paste_event(events: &NativeEventQueue, data: String) {
    events.push(NativeEvent::paste(data));
}

fn handle_terminal_mouse_event(
    event: CrosstermMouseEvent,
    mouse_down: &mut Option<MouseDown>,
    events: &NativeEventQueue,
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
) {
    match event.kind {
        MouseEventKind::Down(button) => {
            *mouse_down = Some(MouseDown { button });
            send_pointer_event(renderer_tx, &event);
            send_selection_event(renderer_tx, SelectionMouseEventType::Down, &event, button);
            push_mouse_event(
                events,
                mouse_event_from_terminal("mousedown", event, button),
            );
        }
        MouseEventKind::Up(button) => {
            let button = mouse_down.take().map(|down| down.button).unwrap_or(button);
            send_pointer_event(renderer_tx, &event);
            send_selection_event(renderer_tx, SelectionMouseEventType::Up, &event, button);
            push_mouse_event(events, mouse_event_from_terminal("mouseup", event, button));
            push_mouse_event(events, mouse_event_from_terminal("click", event, button));
        }
        MouseEventKind::Drag(button) => {
            send_pointer_event(renderer_tx, &event);
            send_selection_event(renderer_tx, SelectionMouseEventType::Drag, &event, button);
            push_mouse_event(
                events,
                mouse_event_from_terminal("mousedrag", event, button),
            );
        }
        MouseEventKind::Moved => {
            send_pointer_event(renderer_tx, &event);
            push_mouse_event(
                events,
                mouse_event_from_terminal("mousemove", event, MouseButton::Left),
            );
        }
        MouseEventKind::ScrollUp => {
            send_pointer_event(renderer_tx, &event);
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, 0, -1));
        }
        MouseEventKind::ScrollDown => {
            send_pointer_event(renderer_tx, &event);
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, 0, 1));
        }
        MouseEventKind::ScrollLeft => {
            send_pointer_event(renderer_tx, &event);
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, -1, 0));
        }
        MouseEventKind::ScrollRight => {
            send_pointer_event(renderer_tx, &event);
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, 1, 0));
        }
    }
}

fn send_selection_event(
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
    event_type: SelectionMouseEventType,
    event: &CrosstermMouseEvent,
    button: MouseButton,
) {
    if button != MouseButton::Left {
        return;
    }

    let Some(renderer_tx) = renderer_tx else {
        return;
    };

    let (response, _rx) = crossbeam_channel::bounded::<SelectionAction>(1);
    let _ = renderer_tx.try_send(EngineCommand::HandleSelection {
        event: SelectionMouseEvent {
            event_type,
            x: u32::from(event.column),
            y: u32::from(event.row),
            button: mouse_button_value(button),
        },
        response,
    });
}

fn send_pointer_event(
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
    event: &CrosstermMouseEvent,
) {
    let Some(renderer_tx) = renderer_tx else {
        return;
    };

    let _ = renderer_tx.try_send(EngineCommand::HandlePointerMove {
        x: u32::from(event.column),
        y: u32::from(event.row),
    });
}

fn send_selection_cursor_event(
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
    event: &CrosstermMouseEvent,
) {
    let Some(renderer_tx) = renderer_tx else {
        return;
    };

    let (response, _rx) = crossbeam_channel::bounded::<SelectionAction>(1);
    let _ = renderer_tx.try_send(EngineCommand::HandleSelection {
        event: SelectionMouseEvent {
            event_type: SelectionMouseEventType::Scroll,
            x: u32::from(event.column),
            y: u32::from(event.row),
            button: 0,
        },
        response,
    });
}

fn push_mouse_event(events: &NativeEventQueue, event: TerminalMouseEvent) {
    events.push(NativeEvent::mouse(event));
}

fn terminal_cell_size(size: crate::terminal::TerminalSize) -> (u32, u32) {
    (size.cols, size.rows)
}

fn observe_terminal_size(
    observed: &mut Option<(u32, u32)>,
    events: &NativeEventQueue,
    size: (u32, u32),
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
) -> bool {
    if size.0 == 0 || size.1 == 0 || *observed == Some(size) {
        return false;
    }
    *observed = Some(size);
    if let Some(renderer_tx) = renderer_tx {
        let _ = renderer_tx.send(EngineCommand::SetRenderSize {
            width: size.0 as usize,
            height: size.1 as usize,
        });
    }
    events.push(NativeEvent::resize(TerminalResizeEvent {
        cols: size.0,
        rows: size.1,
    }));
    true
}

fn next_poll_deadline(previous: Instant, interval: Duration, now: Instant) -> Instant {
    let mut next = previous + interval;
    while next <= now {
        next += interval;
    }
    next
}

fn handle_terminal_focus_event(
    focused: bool,
    events: &NativeEventQueue,
    current_focus: &Arc<AtomicBool>,
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
    initial_report: bool,
) -> bool {
    current_focus.store(focused, Ordering::Relaxed);
    if let Some(renderer_tx) = renderer_tx {
        let _ = renderer_tx.send(EngineCommand::SetTerminalFocused { focused });
    }
    if !initial_report || !focused {
        push_focus_event(events, terminal_focus_event(focused));
        true
    } else {
        false
    }
}

fn recover_missing_tmux_blur(
    tmux_pane_active: Option<bool>,
    events: &NativeEventQueue,
    current_focus: &Arc<AtomicBool>,
    renderer_tx: Option<&crossbeam_channel::Sender<EngineCommand>>,
) -> bool {
    if tmux_pane_active != Some(false) || !current_focus.load(Ordering::Relaxed) {
        return false;
    }

    handle_terminal_focus_event(false, events, current_focus, renderer_tx, false)
}

fn push_focus_event(events: &NativeEventQueue, event: TerminalFocusEvent) {
    events.push(NativeEvent::focus(event));
}

fn terminal_focus_event(focused: bool) -> TerminalFocusEvent {
    TerminalFocusEvent {
        r#type: if focused { "focus" } else { "blur" }.to_string(),
    }
}

fn mouse_event_from_terminal(
    event_type: &str,
    event: CrosstermMouseEvent,
    button: MouseButton,
) -> TerminalMouseEvent {
    let modifiers = event.modifiers;
    TerminalMouseEvent {
        r#type: event_type.to_string(),
        x: u32::from(event.column),
        y: u32::from(event.row),
        button: mouse_button_value(button),
        delta_x: 0,
        delta_y: 0,
        ctrl_key: modifiers.contains(KeyModifiers::CONTROL),
        alt_key: modifiers.contains(KeyModifiers::ALT),
        meta_key: modifiers
            .intersects(KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER),
        shift_key: modifiers.contains(KeyModifiers::SHIFT),
    }
}

fn wheel_event_from_terminal(
    event: CrosstermMouseEvent,
    delta_x: i32,
    delta_y: i32,
) -> TerminalMouseEvent {
    let modifiers = event.modifiers;
    TerminalMouseEvent {
        r#type: "wheel".to_string(),
        x: u32::from(event.column),
        y: u32::from(event.row),
        button: 0,
        delta_x,
        delta_y,
        ctrl_key: modifiers.contains(KeyModifiers::CONTROL),
        alt_key: modifiers.contains(KeyModifiers::ALT),
        meta_key: modifiers
            .intersects(KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER),
        shift_key: modifiers.contains(KeyModifiers::SHIFT),
    }
}

fn mouse_button_value(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

fn pressed_key_from_terminal(event: TerminalKeyEvent) -> Option<PressedKey> {
    if let KeyCode::Char(character) = event.code {
        if let Some(pressed_key) = control_character_key(character, event.modifiers) {
            return Some(pressed_key);
        }
    }

    let key = key_value(event.code)?;
    let code = code_value(event.code);
    let modifiers = event.modifiers;

    Some(PressedKey {
        key,
        code,
        ctrl_key: modifiers.contains(KeyModifiers::CONTROL),
        alt_key: modifiers.contains(KeyModifiers::ALT),
        meta_key: modifiers
            .intersects(KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER),
        shift_key: modifiers.contains(KeyModifiers::SHIFT),
        synthetic_keyup_at: None,
    })
}

const fn is_non_printable_key(code: &KeyCode) -> bool {
    !matches!(code, KeyCode::Char(_))
}

fn control_character_key(character: char, modifiers: KeyModifiers) -> Option<PressedKey> {
    let value = character as u32;
    if !(1..=26).contains(&value) {
        return None;
    }

    let letter = char::from_u32(u32::from(b'a') + value - 1)?;
    Some(PressedKey {
        key: letter.to_string(),
        code: format!("Key{}", letter.to_ascii_uppercase()),
        ctrl_key: true,
        alt_key: modifiers.contains(KeyModifiers::ALT),
        meta_key: modifiers
            .intersects(KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER),
        shift_key: modifiers.contains(KeyModifiers::SHIFT),
        synthetic_keyup_at: None,
    })
}

fn keyboard_event_from_pressed_key(
    event_type: &str,
    repeat: bool,
    pressed_key: &PressedKey,
) -> Option<KeyboardEvent> {
    Some(KeyboardEvent {
        r#type: event_type.to_string(),
        key: pressed_key.key.clone(),
        code: pressed_key.code.clone(),
        ctrl_key: pressed_key.ctrl_key,
        alt_key: pressed_key.alt_key,
        meta_key: pressed_key.meta_key,
        shift_key: pressed_key.shift_key,
        repeat,
    })
}

fn key_value(code: KeyCode) -> Option<String> {
    Some(match code {
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "ArrowLeft".to_string(),
        KeyCode::Right => "ArrowRight".to_string(),
        KeyCode::Up => "ArrowUp".to_string(),
        KeyCode::Down => "ArrowDown".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab | KeyCode::BackTab => "Tab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(number) => format!("F{number}"),
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Null => "Unidentified".to_string(),
        KeyCode::Esc => "Escape".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::PrintScreen => "PrintScreen".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "ContextMenu".to_string(),
        KeyCode::KeypadBegin => "Clear".to_string(),
        KeyCode::Media(media) => media.to_string(),
        KeyCode::Modifier(modifier) => modifier.to_string(),
    })
}

fn code_value(code: KeyCode) -> String {
    match code {
        KeyCode::Char(character) if character.is_ascii_alphabetic() => {
            format!("Key{}", character.to_ascii_uppercase())
        }
        KeyCode::Char(character) if character.is_ascii_digit() => format!("Digit{character}"),
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(character) => character_code(character).to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Left => "ArrowLeft".to_string(),
        KeyCode::Right => "ArrowRight".to_string(),
        KeyCode::Up => "ArrowUp".to_string(),
        KeyCode::Down => "ArrowDown".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Tab | KeyCode::BackTab => "Tab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(number) => format!("F{number}"),
        KeyCode::Null => "Unidentified".to_string(),
        KeyCode::Esc => "Escape".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::PrintScreen => "PrintScreen".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "ContextMenu".to_string(),
        KeyCode::KeypadBegin => "NumpadClear".to_string(),
        KeyCode::Media(media) => media.to_string().replace(' ', ""),
        KeyCode::Modifier(modifier) => modifier.to_string().replace(' ', ""),
    }
}

fn character_code(character: char) -> &'static str {
    match character {
        '`' | '~' => "Backquote",
        '-' | '_' => "Minus",
        '=' | '+' => "Equal",
        '[' | '{' => "BracketLeft",
        ']' | '}' => "BracketRight",
        '\\' | '|' => "Backslash",
        ';' | ':' => "Semicolon",
        '\'' | '"' => "Quote",
        ',' | '<' => "Comma",
        '.' | '>' => "Period",
        '/' | '?' => "Slash",
        _ => "Unidentified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard_event(key: &str) -> KeyboardEvent {
        KeyboardEvent {
            r#type: "keydown".to_string(),
            key: key.to_string(),
            code: "Unidentified".to_string(),
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            shift_key: false,
            repeat: false,
        }
    }

    #[test]
    fn keyboard_and_paste_events_share_one_ordered_queue() {
        let events = NativeEventQueue::default();

        push_keyboard_event(&events, Some(keyboard_event("a")));
        push_paste_event(&events, "pasted".to_string());
        push_keyboard_event(&events, Some(keyboard_event("b")));

        let events = events.drain();
        assert_eq!(
            events[0].keyboard.as_ref().map(|event| event.key.as_str()),
            Some("a")
        );
        assert_eq!(events[1].paste.as_deref(), Some("pasted"));
        assert_eq!(
            events[2].keyboard.as_ref().map(|event| event.key.as_str()),
            Some("b")
        );
    }

    #[test]
    fn terminal_focus_events_use_browser_event_names() {
        assert_eq!(terminal_focus_event(true).r#type, "focus");
        assert_eq!(terminal_focus_event(false).r#type, "blur");
    }

    #[test]
    fn terminal_resize_updates_renderer_size_without_waiting_for_javascript() {
        let events = NativeEventQueue::default();
        let (renderer_tx, renderer_rx) = crossbeam_channel::bounded(1);
        let mut observed = Some((80, 24));

        assert!(observe_terminal_size(
            &mut observed,
            &events,
            (100, 40),
            Some(&renderer_tx),
        ));

        assert!(matches!(
            renderer_rx
                .recv()
                .expect("renderer size command should be queued"),
            EngineCommand::SetRenderSize {
                width: 100,
                height: 40,
            }
        ));
        let events = events.drain();
        assert_eq!(
            events[0]
                .resize
                .as_ref()
                .map(|event| (event.cols, event.rows)),
            Some((100, 40))
        );
    }

    #[test]
    fn polled_size_then_matching_sigwinch_emits_resize_once() {
        let events = NativeEventQueue::default();
        let (renderer_tx, renderer_rx) = crossbeam_channel::bounded(2);
        let mut observed = Some((80, 24));

        assert!(observe_terminal_size(
            &mut observed,
            &events,
            (100, 40),
            Some(&renderer_tx),
        ));
        assert!(!observe_terminal_size(
            &mut observed,
            &events,
            (100, 40),
            Some(&renderer_tx),
        ));

        assert_eq!(events.drain().len(), 1);
        assert!(renderer_rx.recv().is_ok());
        assert!(renderer_rx.try_recv().is_err());
    }

    #[test]
    fn terminal_size_poll_deadline_keeps_its_cadence() {
        let start = Instant::now();
        let interval = Duration::from_millis(100);
        let first_deadline = start + interval;

        assert_eq!(
            next_poll_deadline(first_deadline, interval, first_deadline),
            start + Duration::from_millis(200)
        );
        assert_eq!(
            next_poll_deadline(first_deadline, interval, start + Duration::from_millis(350)),
            start + Duration::from_millis(400)
        );
    }

    #[test]
    fn handle_terminal_focus_event_updates_focus_state_and_queues_event() {
        let events = NativeEventQueue::default();
        let focused = Arc::new(AtomicBool::new(true));
        let (renderer_tx, renderer_rx) = crossbeam_channel::bounded(1);

        handle_terminal_focus_event(false, &events, &focused, Some(&renderer_tx), true);

        assert!(!focused.load(Ordering::Relaxed));
        assert!(matches!(
            renderer_rx
                .recv()
                .expect("renderer command should be queued"),
            EngineCommand::SetTerminalFocused { focused: false }
        ));
        let events = events.drain();
        assert_eq!(
            events[0].focus.as_ref().map(|event| event.r#type.as_str()),
            Some("blur")
        );
    }

    #[test]
    fn initial_focus_confirmation_does_not_queue_a_public_event() {
        let events = NativeEventQueue::default();
        let focused = Arc::new(AtomicBool::new(true));

        handle_terminal_focus_event(true, &events, &focused, None, true);

        assert!(focused.load(Ordering::Relaxed));
        assert!(events.drain().is_empty());
    }

    #[test]
    fn tmux_resize_recovery_queues_missing_blur_after_resize() {
        let events = NativeEventQueue::default();
        let focused = Arc::new(AtomicBool::new(true));
        let (renderer_tx, renderer_rx) = crossbeam_channel::bounded(2);
        let mut observed = Some((120, 40));

        assert!(observe_terminal_size(
            &mut observed,
            &events,
            (80, 20),
            Some(&renderer_tx),
        ));
        assert!(recover_missing_tmux_blur(
            Some(false),
            &events,
            &focused,
            Some(&renderer_tx),
        ));

        assert!(!focused.load(Ordering::Relaxed));
        assert!(matches!(
            renderer_rx.recv().unwrap(),
            EngineCommand::SetRenderSize {
                width: 80,
                height: 20,
            }
        ));
        assert!(matches!(
            renderer_rx.recv().unwrap(),
            EngineCommand::SetTerminalFocused { focused: false }
        ));
        let events = events.drain();
        assert!(events[0].resize.is_some());
        assert_eq!(
            events[1].focus.as_ref().map(|event| event.r#type.as_str()),
            Some("blur")
        );
    }

    #[test]
    fn tmux_resize_recovery_does_not_duplicate_or_invent_blur() {
        let events = NativeEventQueue::default();
        let focused = Arc::new(AtomicBool::new(true));

        assert!(!recover_missing_tmux_blur(
            Some(true),
            &events,
            &focused,
            None,
        ));
        assert!(focused.load(Ordering::Relaxed));

        focused.store(false, Ordering::Relaxed);
        assert!(!recover_missing_tmux_blur(
            Some(false),
            &events,
            &focused,
            None,
        ));
        assert!(events.drain().is_empty());
    }

    #[test]
    fn pressed_key_from_terminal_passes_through_shifted_byte() {
        let event = TerminalKeyEvent::new_with_kind(
            KeyCode::Char('A'),
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        );
        let pressed_key =
            pressed_key_from_terminal(event).expect("pressed key should be produced");
        assert_eq!(pressed_key.key, "A");
        assert_eq!(pressed_key.code, "KeyA");
        assert!(pressed_key.shift_key);
        assert!(!pressed_key.ctrl_key);
        assert!(!pressed_key.alt_key);
        assert!(!pressed_key.meta_key);
    }

    #[test]
    fn pressed_key_from_terminal_passes_through_shifted_symbol() {
        let event = TerminalKeyEvent::new_with_kind(
            KeyCode::Char('!'),
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        );
        let pressed_key =
            pressed_key_from_terminal(event).expect("pressed key should be produced");
        assert_eq!(pressed_key.key, "!");
        assert_eq!(pressed_key.code, "Digit1");
        assert!(pressed_key.shift_key);
    }

    #[test]
    fn pressed_key_from_terminal_plain_letter_without_shift_stays_lowercase() {
        let event =
            TerminalKeyEvent::new_with_kind(KeyCode::Char('a'), KeyModifiers::NONE, KeyEventKind::Press);
        let pressed_key =
            pressed_key_from_terminal(event).expect("pressed key should be produced");
        assert_eq!(pressed_key.key, "a");
        assert!(!pressed_key.shift_key);
    }

    #[test]
    fn pressed_key_from_terminal_shift_tab_keeps_named_key_and_shift_flag() {
        let event = TerminalKeyEvent::new_with_kind(
            KeyCode::BackTab,
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        );
        let pressed_key =
            pressed_key_from_terminal(event).expect("pressed key should be produced");
        assert_eq!(pressed_key.key, "Tab");
        assert_eq!(pressed_key.code, "Tab");
        assert!(pressed_key.shift_key);
    }

    fn synthetic_keyup_delay(delay_ms: u32) -> Arc<Mutex<u32>> {
        Arc::new(Mutex::new(delay_ms))
    }

    #[test]
    fn printable_char_schedules_synthetic_keyup_under_kitty_without_all_keys_flag() {
        let events = NativeEventQueue::default();
        let delay = synthetic_keyup_delay(DEFAULT_SYNTHETIC_KEYUP_MS);
        let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();

        handle_terminal_key_event(
            TerminalKeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ),
            &mut pressed_keys,
            &events,
            &delay,
            true,
        );

        let pressed_key = pressed_keys
            .get("KeyA")
            .expect("printable press should be tracked");
        assert!(
            pressed_key.synthetic_keyup_at.is_some(),
            "printable chars need a synthetic keyup when REPORT_ALL_KEYS_AS_ESCAPE_CODES is off"
        );
    }

    #[test]
    fn disambiguated_key_does_not_schedule_synthetic_keyup_under_kitty() {
        let events = NativeEventQueue::default();
        let delay = synthetic_keyup_delay(DEFAULT_SYNTHETIC_KEYUP_MS);
        let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();

        handle_terminal_key_event(
            TerminalKeyEvent::new_with_kind(
                KeyCode::Tab,
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ),
            &mut pressed_keys,
            &events,
            &delay,
            true,
        );

        let pressed_key = pressed_keys
            .get("Tab")
            .expect("tab press should be tracked");
        assert!(
            pressed_key.synthetic_keyup_at.is_none(),
            "disambiguated keys report real release events under kitty and must not schedule a synthetic keyup"
        );

        events.drain();
    }

    #[test]
    fn arrow_key_does_not_schedule_synthetic_keyup_under_kitty() {
        let events = NativeEventQueue::default();
        let delay = synthetic_keyup_delay(DEFAULT_SYNTHETIC_KEYUP_MS);
        let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();

        handle_terminal_key_event(
            TerminalKeyEvent::new_with_kind(
                KeyCode::Left,
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ),
            &mut pressed_keys,
            &events,
            &delay,
            true,
        );

        let pressed_key = pressed_keys
            .get("ArrowLeft")
            .expect("arrow press should be tracked");
        assert!(pressed_key.synthetic_keyup_at.is_none());
        events.drain();
    }

    #[test]
    fn every_key_schedules_synthetic_keyup_in_legacy_mode() {
        let events = NativeEventQueue::default();
        let delay = synthetic_keyup_delay(DEFAULT_SYNTHETIC_KEYUP_MS);
        let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();

        handle_terminal_key_event(
            TerminalKeyEvent::new_with_kind(
                KeyCode::Tab,
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ),
            &mut pressed_keys,
            &events,
            &delay,
            false,
        );

        let pressed_key = pressed_keys
            .get("Tab")
            .expect("tab press should be tracked");
        assert!(
            pressed_key.synthetic_keyup_at.is_some(),
            "legacy mode never reports real releases, so every key needs a synthetic keyup"
        );
        events.drain();
    }

    #[test]
    fn zero_synthetic_keyup_delay_disables_synthetic_release() {
        let events = NativeEventQueue::default();
        let delay = synthetic_keyup_delay(0);
        let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();

        handle_terminal_key_event(
            TerminalKeyEvent::new_with_kind(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
                KeyEventKind::Press,
            ),
            &mut pressed_keys,
            &events,
            &delay,
            true,
        );

        let pressed_key = pressed_keys
            .get("KeyA")
            .expect("printable press should be tracked");
        assert!(
            pressed_key.synthetic_keyup_at.is_none(),
            "a zero synthetic keyup delay must disable synthetic releases"
        );
        events.drain();
    }
}
