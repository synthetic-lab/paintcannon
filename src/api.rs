use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{bounded, Sender, TrySendError};
use napi::{Error, Result};
use napi_derive::napi;

use crate::input::{KeyboardEvent, TerminalInput, TerminalMouseEvent, TerminalResizeEvent};
use crate::renderer::{
    renderer_loop, ClickEvent, MouseClick, RenderCommand, ScrollMetrics, TransitionEvent,
};
use crate::style::{
    parse_align_items, parse_border_style, parse_cursor, parse_dimension, parse_display,
    parse_flex_direction, parse_flex_flow, parse_flex_shorthand, parse_flex_wrap, parse_gap,
    parse_grid_auto_flow, parse_grid_auto_tracks, parse_grid_line, parse_grid_placement,
    parse_grid_template_tracks, parse_image_rendering, parse_justify_content,
    parse_length_percentage, parse_non_negative_number, parse_overflow, parse_transition,
    parse_white_space, Background,
};
use crate::terminal::{query_terminal_size, reset_terminal, TerminalSize};

const RENDER_QUEUE_CAPACITY: usize = 32 * 1024;
const DEFAULT_SYNTHETIC_KEYUP_MS: u32 = 180;

#[napi(object)]
pub struct BatchCommand {
    pub r#type: String,
    pub id: Option<i32>,
    pub parent: Option<i32>,
    pub child: Option<i32>,
    pub text: Option<String>,
    pub src: Option<String>,
    pub cursor: Option<u32>,
    pub focused: Option<bool>,
    pub property: Option<String>,
    pub value: Option<String>,
}

#[napi(object)]
pub struct BatchIdMapping {
    pub temporary_id: i32,
    pub id: u32,
}

#[napi]
pub struct PaintCannon {
    tx: Sender<RenderCommand>,
    thread: Option<JoinHandle<()>>,
    input: Option<TerminalInput>,
    kitty_keyboard_enabled: bool,
    render_pending: Arc<AtomicBool>,
    transition_events: Arc<Mutex<VecDeque<TransitionEvent>>>,
    next_id: u32,
}

#[napi]
impl PaintCannon {
    #[napi(constructor)]
    pub fn new(
        force_compat_mode: Option<bool>,
        alternate_screen: Option<bool>,
        capture_mouse: Option<bool>,
        capture_ctrl_c: Option<bool>,
    ) -> Self {
        let (tx, rx) = bounded(RENDER_QUEUE_CAPACITY);
        let transition_events = Arc::new(Mutex::new(VecDeque::new()));
        let thread_transition_events = Arc::clone(&transition_events);
        let thread = thread::spawn(move || renderer_loop(rx, thread_transition_events));
        let render_pending = Arc::new(AtomicBool::new(false));
        let input = TerminalInput::start(
            DEFAULT_SYNTHETIC_KEYUP_MS,
            force_compat_mode.unwrap_or(false),
            alternate_screen.unwrap_or(false),
            capture_mouse.unwrap_or(false),
            capture_ctrl_c.unwrap_or(false),
            Some(tx.clone()),
        );
        let kitty_keyboard_enabled = input
            .as_ref()
            .map(TerminalInput::kitty_keyboard_enabled)
            .unwrap_or(false);

        Self {
            tx,
            thread: Some(thread),
            input,
            kitty_keyboard_enabled,
            render_pending,
            transition_events,
            next_id: 1,
        }
    }

    #[napi]
    pub fn create_div(&mut self) -> Result<u32> {
        let id = self.allocate_id();
        self.send(RenderCommand::CreateDiv { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_span(&mut self) -> Result<u32> {
        let id = self.allocate_id();
        self.send(RenderCommand::CreateSpan { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_image(&mut self) -> Result<u32> {
        let id = self.allocate_id();
        self.send(RenderCommand::CreateImage { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_input(&mut self) -> Result<u32> {
        let id = self.allocate_id();
        self.send(RenderCommand::CreateInput { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_text_area(&mut self) -> Result<u32> {
        let id = self.allocate_id();
        self.send(RenderCommand::CreateTextArea { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_text_node(&mut self, text: String) -> Result<u32> {
        let id = self.allocate_id();
        self.send(RenderCommand::CreateText { id, text })?;
        Ok(id)
    }

    #[napi]
    pub fn set_text_node_value(&self, id: u32, text: String) -> Result<()> {
        self.send(RenderCommand::SetText { id, text })
    }

    #[napi]
    pub fn set_image_source(&self, id: u32, src: String) -> Result<()> {
        self.send(RenderCommand::SetImageSource { id, src })
    }

    #[napi]
    pub fn set_input_value(&self, id: u32, value: String, cursor: u32) -> Result<()> {
        self.send(RenderCommand::SetInputValue { id, value, cursor })
    }

    #[napi]
    pub fn set_input_focused(&self, id: u32, focused: bool) -> Result<()> {
        self.send(RenderCommand::SetInputFocused { id, focused })
    }

    #[napi]
    pub fn set_root(&self, id: u32) -> Result<()> {
        self.send(RenderCommand::SetRoot { id })
    }

    #[napi]
    pub fn append_child(&self, parent: u32, child: u32) -> Result<()> {
        self.send(RenderCommand::AppendChild { parent, child })
    }

    #[napi]
    pub fn set_style_property(&self, id: u32, property: String, value: String) -> Result<()> {
        self.send(style_command(id, &property, &value)?)
    }

    #[napi]
    pub fn apply_batch(&mut self, commands: Vec<BatchCommand>) -> Result<Vec<BatchIdMapping>> {
        let mut id_map = HashMap::new();
        let mut mappings = Vec::new();
        let mut render_commands = Vec::with_capacity(commands.len());

        for command in commands {
            match command.r#type.as_str() {
                "createDiv" => {
                    let temporary_id = required_i32(command.id, "id", "createDiv")?;
                    let id = self.allocate_id();
                    id_map.insert(temporary_id, id);
                    mappings.push(BatchIdMapping { temporary_id, id });
                    render_commands.push(RenderCommand::CreateDiv { id });
                }
                "createSpan" => {
                    let temporary_id = required_i32(command.id, "id", "createSpan")?;
                    let id = self.allocate_id();
                    id_map.insert(temporary_id, id);
                    mappings.push(BatchIdMapping { temporary_id, id });
                    render_commands.push(RenderCommand::CreateSpan { id });
                }
                "createText" => {
                    let temporary_id = required_i32(command.id, "id", "createText")?;
                    let id = self.allocate_id();
                    let text = required_string(command.text, "text", "createText")?;
                    id_map.insert(temporary_id, id);
                    mappings.push(BatchIdMapping { temporary_id, id });
                    render_commands.push(RenderCommand::CreateText { id, text });
                }
                "createImage" => {
                    let temporary_id = required_i32(command.id, "id", "createImage")?;
                    let id = self.allocate_id();
                    id_map.insert(temporary_id, id);
                    mappings.push(BatchIdMapping { temporary_id, id });
                    render_commands.push(RenderCommand::CreateImage { id });
                }
                "createInput" => {
                    let temporary_id = required_i32(command.id, "id", "createInput")?;
                    let id = self.allocate_id();
                    id_map.insert(temporary_id, id);
                    mappings.push(BatchIdMapping { temporary_id, id });
                    render_commands.push(RenderCommand::CreateInput { id });
                }
                "createTextArea" => {
                    let temporary_id = required_i32(command.id, "id", "createTextArea")?;
                    let id = self.allocate_id();
                    id_map.insert(temporary_id, id);
                    mappings.push(BatchIdMapping { temporary_id, id });
                    render_commands.push(RenderCommand::CreateTextArea { id });
                }
                "setText" => {
                    let id = resolve_batch_id(command.id, "id", "setText", &id_map)?;
                    let text = required_string(command.text, "text", "setText")?;
                    render_commands.push(RenderCommand::SetText { id, text });
                }
                "setImageSource" => {
                    let id = resolve_batch_id(command.id, "id", "setImageSource", &id_map)?;
                    let src = required_string(command.src, "src", "setImageSource")?;
                    render_commands.push(RenderCommand::SetImageSource { id, src });
                }
                "setInputValue" => {
                    let id = resolve_batch_id(command.id, "id", "setInputValue", &id_map)?;
                    let value = required_string(command.value, "value", "setInputValue")?;
                    let cursor = command.cursor.unwrap_or(0);
                    render_commands.push(RenderCommand::SetInputValue { id, value, cursor });
                }
                "setInputFocused" => {
                    let id = resolve_batch_id(command.id, "id", "setInputFocused", &id_map)?;
                    let focused = command.focused.unwrap_or(false);
                    render_commands.push(RenderCommand::SetInputFocused { id, focused });
                }
                "setRoot" => {
                    let id = resolve_batch_id(command.id, "id", "setRoot", &id_map)?;
                    render_commands.push(RenderCommand::SetRoot { id });
                }
                "appendChild" => {
                    let parent =
                        resolve_batch_id(command.parent, "parent", "appendChild", &id_map)?;
                    let child = resolve_batch_id(command.child, "child", "appendChild", &id_map)?;
                    render_commands.push(RenderCommand::AppendChild { parent, child });
                }
                "setStyleProperty" => {
                    let id = resolve_batch_id(command.id, "id", "setStyleProperty", &id_map)?;
                    let property =
                        required_string(command.property, "property", "setStyleProperty")?;
                    let value = required_string(command.value, "value", "setStyleProperty")?;
                    render_commands.push(style_command(id, &property, &value)?);
                }
                value => {
                    return Err(Error::from_reason(format!(
                        "unsupported batch command: {value}"
                    )));
                }
            }
        }

        if !render_commands.is_empty() {
            self.send(RenderCommand::Batch {
                commands: render_commands,
            })?;
        }

        Ok(mappings)
    }

    #[napi]
    pub fn terminal_size(&self) -> TerminalSize {
        query_terminal_size()
    }

    #[napi(getter)]
    pub fn kitty_keyboard_enabled(&self) -> bool {
        self.kitty_keyboard_enabled
    }

    #[napi]
    pub fn render(&self) -> Result<()> {
        if self
            .render_pending
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(());
        }

        match self.tx.try_send(RenderCommand::Render {
            pending: Arc::clone(&self.render_pending),
        }) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.render_pending.store(false, Ordering::Release);
                Ok(())
            }
            Err(TrySendError::Disconnected(_)) => {
                self.render_pending.store(false, Ordering::Release);
                Err(Error::from_reason("renderer thread stopped"))
            }
        }
    }

    #[napi]
    pub fn render_sync(&self) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        self.send(RenderCommand::RenderSync {
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn drain_keyboard_events(&self) -> Vec<KeyboardEvent> {
        self.input
            .as_ref()
            .map(TerminalInput::drain)
            .unwrap_or_default()
    }

    #[napi]
    pub fn drain_mouse_events(&self) -> Vec<TerminalMouseEvent> {
        self.input
            .as_ref()
            .map(TerminalInput::drain_mouse_events)
            .unwrap_or_default()
    }

    #[napi]
    pub fn drain_resize_events(&self) -> Vec<TerminalResizeEvent> {
        self.input
            .as_ref()
            .map(TerminalInput::drain_resize_events)
            .unwrap_or_default()
    }

    #[napi]
    pub fn drain_transition_events(&self) -> Vec<TransitionEvent> {
        let Ok(mut events) = self.transition_events.lock() else {
            return Vec::new();
        };

        events.drain(..).collect()
    }

    #[napi]
    pub fn click_event_for_mouse_click(
        &self,
        x: u32,
        y: u32,
        button: u32,
        ctrl_key: bool,
        alt_key: bool,
        meta_key: bool,
        shift_key: bool,
    ) -> Result<Option<ClickEvent>> {
        let click = MouseClick {
            x,
            y,
            button,
            ctrl_key,
            alt_key,
            meta_key,
            shift_key,
        };
        let (response_tx, response_rx) = bounded(1);
        self.send(RenderCommand::HitTestClick {
            click,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn target_id_for_point(&self, x: u32, y: u32) -> Result<Option<u32>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(RenderCommand::HitTestPoint {
            x,
            y,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn set_scroll_offset(
        &self,
        id: u32,
        scroll_left: u32,
        scroll_top: u32,
    ) -> Result<Option<ScrollMetrics>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(RenderCommand::SetScrollOffset {
            id,
            scroll_left,
            scroll_top,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn scroll_metrics(&self, id: u32) -> Result<Option<ScrollMetrics>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(RenderCommand::GetScrollMetrics {
            id,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn set_synthetic_keyup_delay(&self, delay_ms: u32) {
        if let Some(input) = self.input.as_ref() {
            input.set_synthetic_keyup_delay(delay_ms);
        }
    }

    #[napi]
    pub fn release_terminal(&self) {
        if let Some(input) = self.input.as_ref() {
            input.release_terminal();
        }
        reset_terminal();
        let _ = self.tx.send(RenderCommand::InvalidateFrame);
    }

    #[napi]
    pub fn capture_terminal(&self) {
        if let Some(input) = self.input.as_ref() {
            input.capture_terminal();
        }
    }

    #[napi]
    pub fn interrupt_process_group(&self) -> Result<()> {
        signal_process_group(libc::SIGINT)
    }

    #[napi]
    pub fn suspend_process_group(&self) -> Result<()> {
        signal_process_group(libc::SIGTSTP)
    }

    #[napi]
    pub fn stop(&mut self) -> Result<()> {
        self.shutdown();
        Ok(())
    }
}

#[cfg(unix)]
fn signal_process_group(signal: libc::c_int) -> Result<()> {
    let result = unsafe { libc::kill(0, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(Error::from_reason(
            std::io::Error::last_os_error().to_string(),
        ))
    }
}

#[cfg(not(unix))]
fn signal_process_group(_signal: libc::c_int) -> Result<()> {
    Err(Error::from_reason(
        "process group signals are not supported on this platform",
    ))
}

impl PaintCannon {
    fn allocate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn send(&self, command: RenderCommand) -> Result<()> {
        self.tx
            .send(command)
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    fn shutdown(&mut self) {
        if let Some(input) = self.input.take() {
            input.shutdown();
        }

        let _ = self.tx.send(RenderCommand::Shutdown);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn style_command(id: u32, property: &str, value: &str) -> Result<RenderCommand> {
    let command = match property {
        "display" => RenderCommand::SetDisplay {
            id,
            display: parse_display(value)?,
        },
        "overflow" => RenderCommand::SetOverflow {
            id,
            overflow: parse_overflow(value)?,
        },
        "overflow-x" | "overflowX" => RenderCommand::SetOverflowX {
            id,
            overflow: parse_overflow(value)?,
        },
        "overflow-y" | "overflowY" => RenderCommand::SetOverflowY {
            id,
            overflow: parse_overflow(value)?,
        },
        "image-rendering" | "imageRendering" => RenderCommand::SetImageRendering {
            id,
            image_rendering: parse_image_rendering(value)?,
        },
        "flex-direction" | "flexDirection" => RenderCommand::SetFlexDirection {
            id,
            direction: parse_flex_direction(value)?,
        },
        "flex-wrap" | "flexWrap" => RenderCommand::SetFlexWrap {
            id,
            flex_wrap: parse_flex_wrap(value)?,
        },
        "flex-flow" | "flexFlow" => {
            let (direction, flex_wrap) = parse_flex_flow(value)?;
            RenderCommand::SetFlexFlow {
                id,
                direction,
                flex_wrap,
            }
        }
        "flex-basis" | "flexBasis" => RenderCommand::SetFlexBasis {
            id,
            flex_basis: parse_dimension(value)?,
        },
        "flex-grow" | "flexGrow" => RenderCommand::SetFlexGrow {
            id,
            flex_grow: parse_non_negative_number("flex-grow", value)?,
        },
        "flex-shrink" | "flexShrink" => RenderCommand::SetFlexShrink {
            id,
            flex_shrink: parse_non_negative_number("flex-shrink", value)?,
        },
        "flex" => parse_flex_shorthand(id, value)?,
        "justify-content" | "justifyContent" => RenderCommand::SetJustifyContent {
            id,
            justify_content: parse_justify_content(value)?,
        },
        "align-items" | "alignItems" => RenderCommand::SetAlignItems {
            id,
            align_items: parse_align_items(value)?,
        },
        "align-self" | "alignSelf" => RenderCommand::SetAlignSelf {
            id,
            align_self: parse_align_items(value)?,
        },
        "align-content" | "alignContent" => RenderCommand::SetAlignContent {
            id,
            align_content: parse_justify_content(value)?,
        },
        "justify-items" | "justifyItems" => RenderCommand::SetJustifyItems {
            id,
            justify_items: parse_align_items(value)?,
        },
        "justify-self" | "justifySelf" => RenderCommand::SetJustifySelf {
            id,
            justify_self: parse_align_items(value)?,
        },
        "gap" => {
            let (row_gap, column_gap) = parse_gap(value)?;
            RenderCommand::SetGap {
                id,
                row_gap,
                column_gap,
            }
        }
        "row-gap" | "rowGap" => RenderCommand::SetRowGap {
            id,
            row_gap: parse_length_percentage(value)?,
        },
        "column-gap" | "columnGap" => RenderCommand::SetColumnGap {
            id,
            column_gap: parse_length_percentage(value)?,
        },
        "width" => RenderCommand::SetWidth {
            id,
            width: parse_dimension(value)?,
        },
        "height" => RenderCommand::SetHeight {
            id,
            height: parse_dimension(value)?,
        },
        "min-height" | "minHeight" => RenderCommand::SetMinHeight {
            id,
            min_height: parse_dimension(value)?,
        },
        "white-space" | "whiteSpace" => RenderCommand::SetWhiteSpace {
            id,
            white_space: parse_white_space(value)?,
        },
        "border" => RenderCommand::SetBorder {
            id,
            style: parse_border_style(value)?,
        },
        "border-top" | "borderTop" => RenderCommand::SetBorderTop {
            id,
            style: parse_border_style(value)?,
        },
        "border-right" | "borderRight" => RenderCommand::SetBorderRight {
            id,
            style: parse_border_style(value)?,
        },
        "border-bottom" | "borderBottom" => RenderCommand::SetBorderBottom {
            id,
            style: parse_border_style(value)?,
        },
        "border-left" | "borderLeft" => RenderCommand::SetBorderLeft {
            id,
            style: parse_border_style(value)?,
        },
        "border-color" | "borderColor" => {
            let color = Background::parse(value)
                .ok_or_else(|| Error::from_reason(format!("unsupported border color: {value}")))?;
            RenderCommand::SetBorderColor { id, color }
        }
        "color" => {
            let color = Background::parse(value)
                .ok_or_else(|| Error::from_reason(format!("unsupported color: {value}")))?;
            RenderCommand::SetColor { id, color }
        }
        "transition" => RenderCommand::SetTransition {
            id,
            transitions: parse_transition(value),
        },
        "background" | "background-color" | "backgroundColor" => {
            let background = Background::parse(value)
                .ok_or_else(|| Error::from_reason(format!("unsupported background: {value}")))?;
            RenderCommand::SetBackground { id, background }
        }
        "selection-background-color" | "selectionBackgroundColor" => {
            let background = Background::parse(value).ok_or_else(|| {
                Error::from_reason(format!("unsupported selection background: {value}"))
            })?;
            RenderCommand::SetSelectionBackground { id, background }
        }
        "cursor" => RenderCommand::SetCursor {
            id,
            cursor: parse_cursor(value)?,
        },
        "grid-template-columns" | "gridTemplateColumns" => RenderCommand::SetGridTemplateColumns {
            id,
            tracks: parse_grid_template_tracks(value)?,
        },
        "grid-template-rows" | "gridTemplateRows" => RenderCommand::SetGridTemplateRows {
            id,
            tracks: parse_grid_template_tracks(value)?,
        },
        "grid-auto-columns" | "gridAutoColumns" => RenderCommand::SetGridAutoColumns {
            id,
            tracks: parse_grid_auto_tracks(value)?,
        },
        "grid-auto-rows" | "gridAutoRows" => RenderCommand::SetGridAutoRows {
            id,
            tracks: parse_grid_auto_tracks(value)?,
        },
        "grid-auto-flow" | "gridAutoFlow" => RenderCommand::SetGridAutoFlow {
            id,
            grid_auto_flow: parse_grid_auto_flow(value)?,
        },
        "grid-column" | "gridColumn" => RenderCommand::SetGridColumn {
            id,
            placement: parse_grid_line(value)?,
        },
        "grid-row" | "gridRow" => RenderCommand::SetGridRow {
            id,
            placement: parse_grid_line(value)?,
        },
        "grid-column-start" | "gridColumnStart" => RenderCommand::SetGridColumnStart {
            id,
            placement: parse_grid_placement(value)?,
        },
        "grid-column-end" | "gridColumnEnd" => RenderCommand::SetGridColumnEnd {
            id,
            placement: parse_grid_placement(value)?,
        },
        "grid-row-start" | "gridRowStart" => RenderCommand::SetGridRowStart {
            id,
            placement: parse_grid_placement(value)?,
        },
        "grid-row-end" | "gridRowEnd" => RenderCommand::SetGridRowEnd {
            id,
            placement: parse_grid_placement(value)?,
        },
        value => {
            return Err(Error::from_reason(format!(
                "unsupported style property: {value}"
            )))
        }
    };

    Ok(command)
}

fn required_i32(value: Option<i32>, field: &str, command: &str) -> Result<i32> {
    value.ok_or_else(|| Error::from_reason(format!("{command} requires {field}")))
}

fn required_string(value: Option<String>, field: &str, command: &str) -> Result<String> {
    value.ok_or_else(|| Error::from_reason(format!("{command} requires {field}")))
}

fn resolve_batch_id(
    value: Option<i32>,
    field: &str,
    command: &str,
    id_map: &HashMap<i32, u32>,
) -> Result<u32> {
    let value = required_i32(value, field, command)?;
    if value >= 0 {
        return Ok(value as u32);
    }

    id_map
        .get(&value)
        .copied()
        .ok_or_else(|| Error::from_reason(format!("{command} references unknown id {value}")))
}

impl Drop for PaintCannon {
    fn drop(&mut self) {
        self.shutdown();
    }
}
