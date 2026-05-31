use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossterm::{
    event::{
        self as terminal_event, DisableMouseCapture, EnableMouseCapture, Event as TerminalEvent,
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
use napi_derive::napi;

use crate::renderer::{RenderCommand, SelectionMouseEvent, SelectionMouseEventType};

const DEFAULT_SYNTHETIC_KEYUP_MS: u32 = 180;

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

pub(crate) struct TerminalInput {
    keyboard_events: Arc<Mutex<VecDeque<KeyboardEvent>>>,
    mouse_events: Arc<Mutex<VecDeque<TerminalMouseEvent>>>,
    stop: Arc<AtomicBool>,
    synthetic_keyup_delay_ms: Arc<Mutex<u32>>,
    kitty_keyboard_enabled: bool,
    force_compat_mode: bool,
    alternate_screen: bool,
    capture_mouse: bool,
    keyboard_enhancement_pushed: Arc<Mutex<bool>>,
    alternate_screen_entered: Arc<Mutex<bool>>,
    mouse_capture_enabled: Arc<Mutex<bool>>,
    terminal_captured: Arc<Mutex<bool>>,
    thread: JoinHandle<()>,
}

impl TerminalInput {
    pub(crate) fn start(
        synthetic_keyup_delay_ms: u32,
        force_compat_mode: bool,
        alternate_screen: bool,
        capture_mouse: bool,
        renderer_tx: Option<crossbeam_channel::Sender<RenderCommand>>,
    ) -> Option<Self> {
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

        let kitty_keyboard_enabled =
            !force_compat_mode && supports_keyboard_enhancement().unwrap_or(false);
        let keyboard_enhancement_pushed = Arc::new(Mutex::new(false));
        if kitty_keyboard_enabled {
            let _ = execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
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

        let keyboard_events = Arc::new(Mutex::new(VecDeque::new()));
        let mouse_events = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let synthetic_keyup_delay_ms = Arc::new(Mutex::new(synthetic_keyup_delay_ms));
        let terminal_captured = Arc::new(Mutex::new(true));
        let thread_keyboard_events = Arc::clone(&keyboard_events);
        let thread_mouse_events = Arc::clone(&mouse_events);
        let thread_stop = Arc::clone(&stop);
        let thread_synthetic_keyup_delay_ms = Arc::clone(&synthetic_keyup_delay_ms);
        let thread_renderer_tx = if capture_mouse { renderer_tx } else { None };

        let thread = thread::spawn(move || {
            let mut pressed_keys: HashMap<String, PressedKey> = HashMap::new();
            let mut mouse_down: Option<MouseDown> = None;

            while !thread_stop.load(Ordering::Relaxed) {
                match terminal_event::poll(Duration::from_millis(25)) {
                    Ok(true) => {
                        if let Ok(event) = terminal_event::read() {
                            match event {
                                TerminalEvent::Key(event) => {
                                    handle_terminal_key_event(
                                        event,
                                        &mut pressed_keys,
                                        &thread_keyboard_events,
                                        &thread_synthetic_keyup_delay_ms,
                                        kitty_keyboard_enabled,
                                    );
                                }
                                TerminalEvent::Mouse(event) => {
                                    handle_terminal_mouse_event(
                                        event,
                                        &mut mouse_down,
                                        &thread_mouse_events,
                                        thread_renderer_tx.as_ref(),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(false) => {
                        synthesize_expired_keyups(&mut pressed_keys, &thread_keyboard_events);
                    }
                    Err(_) => break,
                }
            }

            for (_, pressed_key) in pressed_keys {
                push_keyboard_event(
                    &thread_keyboard_events,
                    keyboard_event_from_pressed_key("keyup", false, &pressed_key),
                );
            }
        });

        Some(Self {
            keyboard_events,
            mouse_events,
            stop,
            synthetic_keyup_delay_ms,
            kitty_keyboard_enabled,
            force_compat_mode,
            alternate_screen,
            capture_mouse,
            keyboard_enhancement_pushed,
            alternate_screen_entered,
            mouse_capture_enabled,
            terminal_captured,
            thread,
        })
    }

    pub(crate) fn drain(&self) -> Vec<KeyboardEvent> {
        let Ok(mut events) = self.keyboard_events.lock() else {
            return Vec::new();
        };

        events.drain(..).collect()
    }

    pub(crate) fn drain_mouse_events(&self) -> Vec<TerminalMouseEvent> {
        let Ok(mut events) = self.mouse_events.lock() else {
            return Vec::new();
        };

        events.drain(..).collect()
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
        if !swap_bool(&self.terminal_captured, false) {
            return;
        }

        if swap_bool(&self.mouse_capture_enabled, false) {
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }
        if swap_bool(&self.keyboard_enhancement_pushed, false) {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        }
        if swap_bool(&self.alternate_screen_entered, false) {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        }
        let _ = disable_raw_mode();
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

        if self.kitty_keyboard_enabled {
            let _ = execute!(
                io::stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
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
    events: &Arc<Mutex<VecDeque<KeyboardEvent>>>,
    synthetic_keyup_delay_ms: &Arc<Mutex<u32>>,
    kitty_keyboard_enabled: bool,
) {
    let Some(pressed_key) = pressed_key_from_terminal(event) else {
        return;
    };

    let code = pressed_key.code.clone();
    let has_real_release = event.kind == KeyEventKind::Release;
    match event.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => {
            let repeat = event.kind == KeyEventKind::Repeat || pressed_keys.contains_key(&code);
            let mut pressed_key = pressed_key;
            pressed_key.synthetic_keyup_at = if kitty_keyboard_enabled {
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

    if has_real_release {
        for pressed_key in pressed_keys.values_mut() {
            pressed_key.synthetic_keyup_at = None;
        }
    }
}

fn synthesize_expired_keyups(
    pressed_keys: &mut HashMap<String, PressedKey>,
    events: &Arc<Mutex<VecDeque<KeyboardEvent>>>,
) {
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

    for code in released {
        if let Some(pressed_key) = pressed_keys.remove(&code) {
            push_keyboard_event(
                events,
                keyboard_event_from_pressed_key("keyup", false, &pressed_key),
            );
        }
    }
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

fn push_keyboard_event(events: &Arc<Mutex<VecDeque<KeyboardEvent>>>, event: Option<KeyboardEvent>) {
    let Some(event) = event else {
        return;
    };

    if let Ok(mut events) = events.lock() {
        events.push_back(event);
        while events.len() > 1024 {
            events.pop_front();
        }
    }
}

fn handle_terminal_mouse_event(
    event: CrosstermMouseEvent,
    mouse_down: &mut Option<MouseDown>,
    events: &Arc<Mutex<VecDeque<TerminalMouseEvent>>>,
    renderer_tx: Option<&crossbeam_channel::Sender<RenderCommand>>,
) {
    match event.kind {
        MouseEventKind::Down(button) => {
            *mouse_down = Some(MouseDown { button });
            send_selection_event(renderer_tx, SelectionMouseEventType::Down, &event, button);
            push_mouse_event(
                events,
                mouse_event_from_terminal("mousedown", event, button),
            );
        }
        MouseEventKind::Up(button) => {
            let button = mouse_down.take().map(|down| down.button).unwrap_or(button);
            send_selection_event(renderer_tx, SelectionMouseEventType::Up, &event, button);
            push_mouse_event(events, mouse_event_from_terminal("mouseup", event, button));
            push_mouse_event(events, mouse_event_from_terminal("click", event, button));
        }
        MouseEventKind::Drag(button) => {
            send_selection_event(renderer_tx, SelectionMouseEventType::Drag, &event, button);
            push_mouse_event(
                events,
                mouse_event_from_terminal("mousedrag", event, button),
            );
        }
        MouseEventKind::Moved => {
            push_mouse_event(
                events,
                mouse_event_from_terminal("mousemove", event, MouseButton::Left),
            );
        }
        MouseEventKind::ScrollUp => {
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, 0, -1));
        }
        MouseEventKind::ScrollDown => {
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, 0, 1));
        }
        MouseEventKind::ScrollLeft => {
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, -1, 0));
        }
        MouseEventKind::ScrollRight => {
            send_selection_cursor_event(renderer_tx, &event);
            push_mouse_event(events, wheel_event_from_terminal(event, 1, 0));
        }
    }
}

fn send_selection_event(
    renderer_tx: Option<&crossbeam_channel::Sender<RenderCommand>>,
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

    let _ = renderer_tx.send(RenderCommand::HandleTextSelection {
        event: SelectionMouseEvent {
            event_type,
            x: u32::from(event.column),
            y: u32::from(event.row),
            button: mouse_button_value(button),
        },
    });
}

fn send_selection_cursor_event(
    renderer_tx: Option<&crossbeam_channel::Sender<RenderCommand>>,
    event: &CrosstermMouseEvent,
) {
    let Some(renderer_tx) = renderer_tx else {
        return;
    };

    let _ = renderer_tx.send(RenderCommand::HandleTextSelection {
        event: SelectionMouseEvent {
            event_type: SelectionMouseEventType::Scroll,
            x: u32::from(event.column),
            y: u32::from(event.row),
            button: 0,
        },
    });
}

fn push_mouse_event(events: &Arc<Mutex<VecDeque<TerminalMouseEvent>>>, event: TerminalMouseEvent) {
    if let Ok(mut events) = events.lock() {
        events.push_back(event);
        while events.len() > 1024 {
            events.pop_front();
        }
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
