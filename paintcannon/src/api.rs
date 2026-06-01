use std::collections::HashMap;
use std::io::{self, IsTerminal};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crossbeam_channel::{bounded, Sender, TrySendError};
use napi::{Error, Result};
use napi_derive::napi;
use termprofile::{DetectorSettings, TermProfile};

use crate::engine::{
    apply_style_mutation, engine_loop, ClickEvent as EngineClickEvent, DomId, EngineCommand,
    EngineTransitionEvent, MouseClick, StyleMutation,
};
use crate::input::{KeyboardEvent, TerminalInput, TerminalMouseEvent, TerminalResizeEvent};
use crate::layout::ArenaScrollMetrics;
use crate::style::{
    parse_align_items, parse_border_style, parse_box_lengths, parse_cursor, parse_dimension,
    parse_display, parse_flex_direction, parse_flex_flow, parse_flex_shorthand, parse_flex_wrap,
    parse_gap, parse_grid_auto_flow, parse_grid_auto_tracks, parse_grid_line, parse_grid_placement,
    parse_grid_template_tracks, parse_image_rendering, parse_justify_content,
    parse_length_percentage, parse_length_percentage_auto, parse_margin_lengths,
    parse_non_negative_number, parse_overflow, parse_transition, parse_white_space, Background,
};
use crate::terminal::{query_terminal_size, reset_terminal, TerminalSize};

const RENDER_QUEUE_CAPACITY: usize = 32 * 1024;
const DEFAULT_SYNTHETIC_KEYUP_MS: u32 = 180;

#[derive(Clone)]
#[napi(object)]
pub struct ClickEvent {
    pub r#type: String,
    pub target_id: u32,
    pub client_x: u32,
    pub client_y: u32,
    pub button: u32,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}

#[derive(Clone, Debug)]
#[napi(object)]
pub struct ScrollMetrics {
    pub scroll_left: u32,
    pub scroll_top: u32,
    pub scroll_width: u32,
    pub scroll_height: u32,
    pub client_width: u32,
    pub client_height: u32,
}

#[derive(Clone)]
#[napi(object)]
pub struct TransitionEvent {
    pub r#type: String,
    pub target_id: u32,
    pub property_name: String,
}

#[napi(object)]
pub struct BatchCommand {
    pub r#type: String,
    pub id: Option<i32>,
    pub parent: Option<i32>,
    pub child: Option<i32>,
    pub before: Option<i32>,
    pub text: Option<String>,
    pub src: Option<String>,
    pub cursor: Option<u32>,
    pub focused: Option<bool>,
    pub placeholder: Option<String>,
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
    tx: Sender<EngineCommand>,
    thread: Option<JoinHandle<()>>,
    input: Option<TerminalInput>,
    kitty_keyboard_enabled: bool,
    render_pending: Arc<AtomicBool>,
    next_dom_id: u32,
    color_profile: TermProfile,
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
        termprofile::set_color_cache_enabled(true);
        let color_profile = TermProfile::detect(&io::stdout(), DetectorSettings::default());
        let thread = thread::spawn(move || engine_loop(rx));
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
        let _ = tx.send(EngineCommand::SetTruecolorEnabled {
            enabled: color_profile == TermProfile::TrueColor,
        });

        Self {
            tx,
            thread: Some(thread),
            input,
            kitty_keyboard_enabled,
            render_pending,
            next_dom_id: 1,
            color_profile,
        }
    }

    #[napi]
    pub fn create_div(&mut self) -> Result<u32> {
        self.create_node(|id| EngineCommand::CreateElementWithId {
            id,
            style: Default::default(),
        })
    }

    #[napi]
    pub fn create_span(&mut self) -> Result<u32> {
        let mut style = crate::style::DivStyle::default();
        style.display = crate::style::LayoutDisplay::Inline;
        self.create_node(|id| EngineCommand::CreateElementWithId { id, style })
    }

    #[napi]
    pub fn create_image(&mut self) -> Result<u32> {
        self.create_node(|id| EngineCommand::CreateImageWithId {
            id,
            style: Default::default(),
            width_px: 1,
            height_px: 1,
            cell_width_px: 1,
            cell_height_px: 1,
        })
    }

    #[napi]
    pub fn create_input(&mut self) -> Result<u32> {
        self.create_node(|id| EngineCommand::CreateInputWithId {
            id,
            style: Default::default(),
            value: String::new(),
        })
    }

    #[napi]
    pub fn create_text_area(&mut self) -> Result<u32> {
        self.create_node(|id| EngineCommand::CreateTextAreaWithId {
            id,
            style: Default::default(),
            value: String::new(),
        })
    }

    #[napi]
    pub fn create_text_node(&mut self, text: String) -> Result<u32> {
        self.create_node(|id| EngineCommand::CreateTextWithId { id, text })
    }

    #[napi]
    pub fn set_text_node_value(&self, id: u32, text: String) -> Result<()> {
        self.send(EngineCommand::SetText {
            node: DomId(id),
            text,
        })
    }

    #[napi]
    pub fn set_image_source(&self, id: u32, src: String) -> Result<()> {
        self.send(EngineCommand::SetImageSource {
            node: DomId(id),
            src,
        })
    }

    #[napi]
    pub fn set_input_value(&self, id: u32, value: String, cursor: u32) -> Result<()> {
        self.send(EngineCommand::SetInputValue {
            node: DomId(id),
            value,
            cursor,
        })
    }

    #[napi]
    pub fn set_input_focused(&self, id: u32, focused: bool) -> Result<()> {
        self.send(EngineCommand::SetInputFocused {
            node: DomId(id),
            focused,
        })
    }

    #[napi]
    pub fn set_input_placeholder(&self, id: u32, placeholder: String) -> Result<()> {
        self.send(EngineCommand::SetInputPlaceholder {
            node: DomId(id),
            placeholder,
        })
    }

    #[napi]
    pub fn set_text_area_value(&self, id: u32, value: String, cursor: u32) -> Result<()> {
        self.send(EngineCommand::SetTextAreaValue {
            node: DomId(id),
            value,
            cursor,
        })
    }

    #[napi]
    pub fn set_text_area_focused(&self, id: u32, focused: bool) -> Result<()> {
        self.send(EngineCommand::SetTextAreaFocused {
            node: DomId(id),
            focused,
        })
    }

    #[napi]
    pub fn set_text_area_placeholder(&self, id: u32, placeholder: String) -> Result<()> {
        self.send(EngineCommand::SetTextAreaPlaceholder {
            node: DomId(id),
            placeholder,
        })
    }

    #[napi]
    pub fn move_text_area_cursor_vertically(&self, id: u32, direction: i32) -> Result<Option<u32>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(EngineCommand::MoveTextAreaCursorVertically {
            node: DomId(id),
            direction,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn set_text_control_cursor_at_point(&self, id: u32, x: u32, y: u32) -> Result<Option<u32>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(EngineCommand::SetTextControlCursorAtPoint {
            node: DomId(id),
            x,
            y,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    #[napi]
    pub fn set_root(&self, id: u32) -> Result<()> {
        self.send(EngineCommand::SetRoot { root: DomId(id) })
    }

    #[napi]
    pub fn append_child(&self, parent: u32, child: u32) -> Result<()> {
        self.send(EngineCommand::AppendChild {
            parent: DomId(parent),
            child: DomId(child),
        })
    }

    #[napi]
    pub fn insert_child_before(&self, parent: u32, child: u32, before: u32) -> Result<()> {
        self.send(EngineCommand::InsertChildBefore {
            parent: DomId(parent),
            child: DomId(child),
            before: DomId(before),
        })
    }

    #[napi]
    pub fn detach_node(&self, id: u32) -> Result<()> {
        self.send(EngineCommand::DetachNode { node: DomId(id) })
    }

    #[napi]
    pub fn destroy_node(&self, id: u32) -> Result<()> {
        self.send(EngineCommand::DestroyNode { node: DomId(id) })
    }

    #[napi]
    pub fn set_style_property(&self, id: u32, property: String, value: String) -> Result<()> {
        self.send(style_command(id, &property, &value)?)
    }

    #[napi]
    pub fn apply_batch(&mut self, commands: Vec<BatchCommand>) -> Result<Vec<BatchIdMapping>> {
        let total_start = Instant::now();
        let input_count = commands.len();
        let mut id_map = HashMap::new();
        let mut create_command_by_temporary_id = HashMap::new();
        let mut mappings = Vec::new();
        let mut render_commands = Vec::with_capacity(commands.len());

        let rewrite_start = Instant::now();
        for command in commands {
            match command.r#type.as_str() {
                "createDiv" => {
                    let temporary_id = required_i32(command.id, "id", "createDiv")?;
                    let id = self.allocate_dom_id()?;
                    id_map.insert(temporary_id, id.0);
                    mappings.push(BatchIdMapping {
                        temporary_id,
                        id: id.0,
                    });
                    render_commands.push(EngineCommand::CreateElementWithId {
                        id,
                        style: Default::default(),
                    });
                    create_command_by_temporary_id.insert(temporary_id, render_commands.len() - 1);
                }
                "createSpan" => {
                    let temporary_id = required_i32(command.id, "id", "createSpan")?;
                    let id = self.allocate_dom_id()?;
                    id_map.insert(temporary_id, id.0);
                    mappings.push(BatchIdMapping {
                        temporary_id,
                        id: id.0,
                    });
                    let mut style = crate::style::DivStyle::default();
                    style.display = crate::style::LayoutDisplay::Inline;
                    render_commands.push(EngineCommand::CreateElementWithId { id, style });
                    create_command_by_temporary_id.insert(temporary_id, render_commands.len() - 1);
                }
                "createText" => {
                    let temporary_id = required_i32(command.id, "id", "createText")?;
                    let text = required_string(command.text, "text", "createText")?;
                    let id = self.allocate_dom_id()?;
                    id_map.insert(temporary_id, id.0);
                    mappings.push(BatchIdMapping {
                        temporary_id,
                        id: id.0,
                    });
                    render_commands.push(EngineCommand::CreateTextWithId { id, text });
                }
                "createImage" => {
                    let temporary_id = required_i32(command.id, "id", "createImage")?;
                    let id = self.allocate_dom_id()?;
                    id_map.insert(temporary_id, id.0);
                    mappings.push(BatchIdMapping {
                        temporary_id,
                        id: id.0,
                    });
                    render_commands.push(EngineCommand::CreateImageWithId {
                        id,
                        style: Default::default(),
                        width_px: 1,
                        height_px: 1,
                        cell_width_px: 1,
                        cell_height_px: 1,
                    });
                    create_command_by_temporary_id.insert(temporary_id, render_commands.len() - 1);
                }
                "createInput" => {
                    let temporary_id = required_i32(command.id, "id", "createInput")?;
                    let id = self.allocate_dom_id()?;
                    id_map.insert(temporary_id, id.0);
                    mappings.push(BatchIdMapping {
                        temporary_id,
                        id: id.0,
                    });
                    render_commands.push(EngineCommand::CreateInputWithId {
                        id,
                        style: Default::default(),
                        value: String::new(),
                    });
                    create_command_by_temporary_id.insert(temporary_id, render_commands.len() - 1);
                }
                "createTextArea" => {
                    let temporary_id = required_i32(command.id, "id", "createTextArea")?;
                    let id = self.allocate_dom_id()?;
                    id_map.insert(temporary_id, id.0);
                    mappings.push(BatchIdMapping {
                        temporary_id,
                        id: id.0,
                    });
                    render_commands.push(EngineCommand::CreateTextAreaWithId {
                        id,
                        style: Default::default(),
                        value: String::new(),
                    });
                    create_command_by_temporary_id.insert(temporary_id, render_commands.len() - 1);
                }
                "setText" => {
                    let id = resolve_batch_id(command.id, "id", "setText", &id_map)?;
                    let text = required_string(command.text, "text", "setText")?;
                    render_commands.push(EngineCommand::SetText {
                        node: DomId(id),
                        text,
                    });
                }
                "setImageSource" => {
                    let id = resolve_batch_id(command.id, "id", "setImageSource", &id_map)?;
                    let src = required_string(command.src, "src", "setImageSource")?;
                    render_commands.push(EngineCommand::SetImageSource {
                        node: DomId(id),
                        src,
                    });
                }
                "setInputValue" => {
                    let id = resolve_batch_id(command.id, "id", "setInputValue", &id_map)?;
                    let value = required_string(command.value, "value", "setInputValue")?;
                    let cursor = command.cursor.unwrap_or(0);
                    render_commands.push(EngineCommand::SetInputValue {
                        node: DomId(id),
                        value,
                        cursor,
                    });
                }
                "setInputFocused" => {
                    let id = resolve_batch_id(command.id, "id", "setInputFocused", &id_map)?;
                    let focused = command.focused.unwrap_or(false);
                    render_commands.push(EngineCommand::SetInputFocused {
                        node: DomId(id),
                        focused,
                    });
                }
                "setInputPlaceholder" => {
                    let id = resolve_batch_id(command.id, "id", "setInputPlaceholder", &id_map)?;
                    let placeholder =
                        required_string(command.placeholder, "placeholder", "setInputPlaceholder")?;
                    render_commands.push(EngineCommand::SetInputPlaceholder {
                        node: DomId(id),
                        placeholder,
                    });
                }
                "setTextAreaValue" => {
                    let id = resolve_batch_id(command.id, "id", "setTextAreaValue", &id_map)?;
                    let value = required_string(command.value, "value", "setTextAreaValue")?;
                    let cursor = command.cursor.unwrap_or(0);
                    render_commands.push(EngineCommand::SetTextAreaValue {
                        node: DomId(id),
                        value,
                        cursor,
                    });
                }
                "setTextAreaFocused" => {
                    let id = resolve_batch_id(command.id, "id", "setTextAreaFocused", &id_map)?;
                    let focused = command.focused.unwrap_or(false);
                    render_commands.push(EngineCommand::SetTextAreaFocused {
                        node: DomId(id),
                        focused,
                    });
                }
                "setTextAreaPlaceholder" => {
                    let id = resolve_batch_id(command.id, "id", "setTextAreaPlaceholder", &id_map)?;
                    let placeholder = required_string(
                        command.placeholder,
                        "placeholder",
                        "setTextAreaPlaceholder",
                    )?;
                    render_commands.push(EngineCommand::SetTextAreaPlaceholder {
                        node: DomId(id),
                        placeholder,
                    });
                }
                "setRoot" => {
                    let id = resolve_batch_id(command.id, "id", "setRoot", &id_map)?;
                    render_commands.push(EngineCommand::SetRoot { root: DomId(id) });
                }
                "appendChild" => {
                    let parent =
                        resolve_batch_id(command.parent, "parent", "appendChild", &id_map)?;
                    let child = resolve_batch_id(command.child, "child", "appendChild", &id_map)?;
                    render_commands.push(EngineCommand::AppendChild {
                        parent: DomId(parent),
                        child: DomId(child),
                    });
                }
                "insertChildBefore" => {
                    let parent =
                        resolve_batch_id(command.parent, "parent", "insertChildBefore", &id_map)?;
                    let child =
                        resolve_batch_id(command.child, "child", "insertChildBefore", &id_map)?;
                    let before =
                        resolve_batch_id(command.before, "before", "insertChildBefore", &id_map)?;
                    render_commands.push(EngineCommand::InsertChildBefore {
                        parent: DomId(parent),
                        child: DomId(child),
                        before: DomId(before),
                    });
                }
                "detachNode" => {
                    let id = resolve_batch_id(command.id, "id", "detachNode", &id_map)?;
                    render_commands.push(EngineCommand::DetachNode { node: DomId(id) });
                }
                "destroyNode" => {
                    let id = resolve_batch_id(command.id, "id", "destroyNode", &id_map)?;
                    render_commands.push(EngineCommand::DestroyNode { node: DomId(id) });
                }
                "setStyleProperty" => {
                    let temporary_id = command.id.filter(|id| *id < 0);
                    let id = resolve_batch_id(command.id, "id", "setStyleProperty", &id_map)?;
                    let property =
                        required_string(command.property, "property", "setStyleProperty")?;
                    let value = required_string(command.value, "value", "setStyleProperty")?;
                    match style_command(id, &property, &value)? {
                        EngineCommand::MutateStyle { node, mutation } => {
                            if let Some(temporary_id) = temporary_id {
                                if let Some(command_index) =
                                    create_command_by_temporary_id.get(&temporary_id).copied()
                                {
                                    if let Some(style) = create_command_style_mut(
                                        &mut render_commands[command_index],
                                    ) {
                                        apply_style_mutation(style, mutation);
                                        continue;
                                    }
                                }
                            }
                            render_commands.push(EngineCommand::MutateStyle { node, mutation });
                        }
                        command => render_commands.push(command),
                    }
                }
                value => {
                    return Err(Error::from_reason(format!(
                        "unsupported batch command: {value}"
                    )));
                }
            }
        }
        profile_log(
            "napi_apply_batch_rewrite",
            rewrite_start.elapsed(),
            &[
                ("input_commands", input_count.to_string()),
                ("render_commands", render_commands.len().to_string()),
                ("mappings", mappings.len().to_string()),
            ],
        );

        if render_commands.is_empty() {
            profile_log("napi_apply_batch_total", total_start.elapsed(), &[]);
            return Ok(Vec::new());
        }

        let send_start = Instant::now();
        self.send(EngineCommand::Batch {
            commands: render_commands,
        })?;
        profile_log("napi_apply_batch_send", send_start.elapsed(), &[]);
        profile_log("napi_apply_batch_total", total_start.elapsed(), &[]);
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

        let size = query_terminal_size();
        match self.tx.try_send(EngineCommand::RenderPending {
            width: size.cols as usize,
            height: size.rows as usize,
            color_profile: self.color_profile,
            synchronized: io::stdout().is_terminal(),
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
        let size = query_terminal_size();
        self.send(EngineCommand::RenderStdout {
            width: size.cols as usize,
            height: size.rows as usize,
            color_profile: self.color_profile,
            synchronized: io::stdout().is_terminal(),
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
            .and_then(|result| result.map_err(|error| Error::from_reason(error.to_string())))
    }

    #[napi]
    pub fn invalidate_frame(&self) {
        let _ = self.tx.send(EngineCommand::InvalidateFrame);
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
        let (response_tx, response_rx) = bounded(1);
        if self
            .send(EngineCommand::DrainTransitionEvents {
                response: response_tx,
            })
            .is_err()
        {
            return Vec::new();
        }
        response_rx
            .recv()
            .map(|events| events.into_iter().map(transition_event_to_napi).collect())
            .unwrap_or_default()
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
        self.send(EngineCommand::HitTestClick {
            click,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
            .map(|event| event.map(click_event_to_napi))
    }

    #[napi]
    pub fn target_id_for_point(&self, x: u32, y: u32) -> Result<Option<u32>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(EngineCommand::HitTestPoint {
            x,
            y,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
            .map(|id| id.map(|id| id.0))
    }

    #[napi]
    pub fn set_scroll_offset(
        &self,
        id: u32,
        scroll_left: u32,
        scroll_top: u32,
    ) -> Result<Option<ScrollMetrics>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(EngineCommand::SetScrollOffset {
            node: DomId(id),
            scroll_left,
            scroll_top,
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
            .map(|metrics| metrics.map(scroll_metrics_to_napi))
    }

    #[napi]
    pub fn scroll_metrics(&self, id: u32) -> Result<Option<ScrollMetrics>> {
        let (response_tx, response_rx) = bounded(1);
        self.send(EngineCommand::GetScrollMetrics {
            node: DomId(id),
            response: response_tx,
        })?;
        response_rx
            .recv()
            .map_err(|_| Error::from_reason("renderer thread stopped"))
            .map(|metrics| metrics.map(scroll_metrics_to_napi))
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
        let _ = self.tx.send(EngineCommand::InvalidateFrame);
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
    fn create_node(&mut self, command: impl FnOnce(DomId) -> EngineCommand) -> Result<u32> {
        let id = self.allocate_dom_id()?;
        self.send(command(id))?;
        Ok(id.0)
    }

    fn allocate_dom_id(&mut self) -> Result<DomId> {
        let id = self.next_dom_id;
        self.next_dom_id = self
            .next_dom_id
            .checked_add(1)
            .ok_or_else(|| Error::from_reason("DOM id overflow"))?;
        Ok(DomId(id))
    }

    fn send(&self, command: EngineCommand) -> Result<()> {
        self.tx
            .send(command)
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    fn shutdown(&mut self) {
        let should_reset_terminal = self.input.is_some() || self.thread.is_some();
        if let Some(input) = self.input.take() {
            input.shutdown();
        }
        if should_reset_terminal {
            reset_terminal();
        }

        let _ = self.tx.send(EngineCommand::Shutdown);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn create_command_style_mut(command: &mut EngineCommand) -> Option<&mut crate::style::DivStyle> {
    match command {
        EngineCommand::CreateElementWithId { style, .. }
        | EngineCommand::CreateImageWithId { style, .. }
        | EngineCommand::CreateInputWithId { style, .. }
        | EngineCommand::CreateTextAreaWithId { style, .. } => Some(style),
        _ => None,
    }
}

fn style_command(id: u32, property: &str, value: &str) -> Result<EngineCommand> {
    let node = DomId(id);
    let mutation = match property {
        "display" => StyleMutation::Display(parse_display(value)?),
        "overflow" => StyleMutation::Overflow(parse_overflow(value)?),
        "overflow-x" | "overflowX" => StyleMutation::OverflowX(parse_overflow(value)?),
        "overflow-y" | "overflowY" => StyleMutation::OverflowY(parse_overflow(value)?),
        "image-rendering" | "imageRendering" => {
            StyleMutation::ImageRendering(parse_image_rendering(value)?)
        }
        "flex-direction" | "flexDirection" => {
            StyleMutation::FlexDirection(parse_flex_direction(value)?)
        }
        "flex-wrap" | "flexWrap" => StyleMutation::FlexWrap(parse_flex_wrap(value)?),
        "flex-flow" | "flexFlow" => {
            let (direction, flex_wrap) = parse_flex_flow(value)?;
            StyleMutation::FlexFlow {
                direction,
                flex_wrap,
            }
        }
        "flex-basis" | "flexBasis" => StyleMutation::FlexBasis(parse_dimension(value)?),
        "flex-grow" | "flexGrow" => {
            StyleMutation::FlexGrow(parse_non_negative_number("flex-grow", value)?)
        }
        "flex-shrink" | "flexShrink" => {
            StyleMutation::FlexShrink(parse_non_negative_number("flex-shrink", value)?)
        }
        "flex" => {
            let flex = parse_flex_shorthand(value)?;
            StyleMutation::Flex {
                flex_grow: flex.flex_grow,
                flex_shrink: flex.flex_shrink,
                flex_basis: flex.flex_basis,
            }
        }
        "justify-content" | "justifyContent" => {
            StyleMutation::JustifyContent(parse_justify_content(value)?)
        }
        "align-items" | "alignItems" => StyleMutation::AlignItems(parse_align_items(value)?),
        "align-self" | "alignSelf" => StyleMutation::AlignSelf(parse_align_items(value)?),
        "align-content" | "alignContent" => {
            StyleMutation::AlignContent(parse_justify_content(value)?)
        }
        "justify-items" | "justifyItems" => StyleMutation::JustifyItems(parse_align_items(value)?),
        "justify-self" | "justifySelf" => StyleMutation::JustifySelf(parse_align_items(value)?),
        "gap" => {
            let (row_gap, column_gap) = parse_gap(value)?;
            StyleMutation::Gap {
                row_gap,
                column_gap,
            }
        }
        "row-gap" | "rowGap" => StyleMutation::RowGap(parse_length_percentage(value)?),
        "column-gap" | "columnGap" => StyleMutation::ColumnGap(parse_length_percentage(value)?),
        "padding" => {
            let (top, right, bottom, left) = parse_box_lengths("padding", value)?;
            StyleMutation::Padding {
                top,
                right,
                bottom,
                left,
            }
        }
        "padding-top" | "paddingTop" => StyleMutation::PaddingTop(parse_length_percentage(value)?),
        "padding-right" | "paddingRight" => {
            StyleMutation::PaddingRight(parse_length_percentage(value)?)
        }
        "padding-bottom" | "paddingBottom" => {
            StyleMutation::PaddingBottom(parse_length_percentage(value)?)
        }
        "padding-left" | "paddingLeft" => {
            StyleMutation::PaddingLeft(parse_length_percentage(value)?)
        }
        "margin" => {
            let (top, right, bottom, left) = parse_margin_lengths(value)?;
            StyleMutation::Margin {
                top,
                right,
                bottom,
                left,
            }
        }
        "margin-top" | "marginTop" => {
            StyleMutation::MarginTop(parse_length_percentage_auto(value)?)
        }
        "margin-right" | "marginRight" => {
            StyleMutation::MarginRight(parse_length_percentage_auto(value)?)
        }
        "margin-bottom" | "marginBottom" => {
            StyleMutation::MarginBottom(parse_length_percentage_auto(value)?)
        }
        "margin-left" | "marginLeft" => {
            StyleMutation::MarginLeft(parse_length_percentage_auto(value)?)
        }
        "width" => StyleMutation::Width(parse_dimension(value)?),
        "height" => StyleMutation::Height(parse_dimension(value)?),
        "min-height" | "minHeight" => StyleMutation::MinHeight(parse_dimension(value)?),
        "max-height" | "maxHeight" => StyleMutation::MaxHeight(parse_dimension(value)?),
        "white-space" | "whiteSpace" => StyleMutation::WhiteSpace(parse_white_space(value)?),
        "border" => StyleMutation::Border(parse_border_style(value)?),
        "border-top" | "borderTop" => StyleMutation::BorderTop(parse_border_style(value)?),
        "border-right" | "borderRight" => StyleMutation::BorderRight(parse_border_style(value)?),
        "border-bottom" | "borderBottom" => StyleMutation::BorderBottom(parse_border_style(value)?),
        "border-left" | "borderLeft" => StyleMutation::BorderLeft(parse_border_style(value)?),
        "border-color" | "borderColor" => {
            let color = Background::parse(value)
                .ok_or_else(|| Error::from_reason(format!("unsupported border color: {value}")))?;
            StyleMutation::BorderColor(color)
        }
        "color" => {
            let color = Background::parse(value)
                .ok_or_else(|| Error::from_reason(format!("unsupported color: {value}")))?;
            StyleMutation::Color(color)
        }
        "placeholder-color" | "placeholderColor" => {
            let color = Background::parse(value).ok_or_else(|| {
                Error::from_reason(format!("unsupported placeholder color: {value}"))
            })?;
            StyleMutation::PlaceholderColor(color)
        }
        "transition" => {
            return Ok(EngineCommand::SetTransition {
                node,
                transitions: parse_transition(value),
            });
        }
        "background" | "background-color" | "backgroundColor" => {
            let background = Background::parse(value)
                .ok_or_else(|| Error::from_reason(format!("unsupported background: {value}")))?;
            StyleMutation::Background(background)
        }
        "selection-background-color" | "selectionBackgroundColor" => {
            let background = Background::parse(value).ok_or_else(|| {
                Error::from_reason(format!("unsupported selection background: {value}"))
            })?;
            StyleMutation::SelectionBackground(background)
        }
        "cursor" => StyleMutation::Cursor(parse_cursor(value)?),
        "grid-template-columns" | "gridTemplateColumns" => {
            StyleMutation::GridTemplateColumns(parse_grid_template_tracks(value)?)
        }
        "grid-template-rows" | "gridTemplateRows" => {
            StyleMutation::GridTemplateRows(parse_grid_template_tracks(value)?)
        }
        "grid-auto-columns" | "gridAutoColumns" => {
            StyleMutation::GridAutoColumns(parse_grid_auto_tracks(value)?)
        }
        "grid-auto-rows" | "gridAutoRows" => {
            StyleMutation::GridAutoRows(parse_grid_auto_tracks(value)?)
        }
        "grid-auto-flow" | "gridAutoFlow" => {
            StyleMutation::GridAutoFlow(parse_grid_auto_flow(value)?)
        }
        "grid-column" | "gridColumn" => StyleMutation::GridColumn(parse_grid_line(value)?),
        "grid-row" | "gridRow" => StyleMutation::GridRow(parse_grid_line(value)?),
        "grid-column-start" | "gridColumnStart" => {
            StyleMutation::GridColumnStart(parse_grid_placement(value)?)
        }
        "grid-column-end" | "gridColumnEnd" => {
            StyleMutation::GridColumnEnd(parse_grid_placement(value)?)
        }
        "grid-row-start" | "gridRowStart" => {
            StyleMutation::GridRowStart(parse_grid_placement(value)?)
        }
        "grid-row-end" | "gridRowEnd" => StyleMutation::GridRowEnd(parse_grid_placement(value)?),
        value => {
            return Err(Error::from_reason(format!(
                "unsupported style property: {value}"
            )))
        }
    };

    Ok(EngineCommand::MutateStyle { node, mutation })
}

fn scroll_metrics_to_napi(metrics: ArenaScrollMetrics) -> ScrollMetrics {
    ScrollMetrics {
        scroll_left: metrics.scroll_left,
        scroll_top: metrics.scroll_top,
        scroll_width: metrics.scroll_width,
        scroll_height: metrics.scroll_height,
        client_width: metrics.client_width,
        client_height: metrics.client_height,
    }
}

fn click_event_to_napi(event: EngineClickEvent) -> ClickEvent {
    ClickEvent {
        r#type: "click".to_string(),
        target_id: event.target_id.0,
        client_x: event.client_x,
        client_y: event.client_y,
        button: event.button,
        ctrl_key: event.ctrl_key,
        alt_key: event.alt_key,
        meta_key: event.meta_key,
        shift_key: event.shift_key,
    }
}

fn transition_event_to_napi(event: EngineTransitionEvent) -> TransitionEvent {
    TransitionEvent {
        r#type: match event.event_type {
            crate::transition::TransitionEventType::Start => "transitionstart",
            crate::transition::TransitionEventType::End => "transitionend",
        }
        .to_string(),
        target_id: event.target.0,
        property_name: transition_property_name(event.property).to_string(),
    }
}

fn transition_property_name(property: crate::style::ColorTransitionProperty) -> &'static str {
    match property {
        crate::style::ColorTransitionProperty::Color => "color",
        crate::style::ColorTransitionProperty::BackgroundColor => "background-color",
        crate::style::ColorTransitionProperty::BorderColor => "border-color",
    }
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

fn profile_log(label: &str, duration: std::time::Duration, fields: &[(&str, String)]) {
    if std::env::var_os("PAINTCANNON_PROFILE").is_none() {
        return;
    }

    eprint!(
        "[paintcannon-profile] event={} duration_ms={:.3}",
        label,
        duration.as_secs_f64() * 1000.0
    );
    for (key, value) in fields {
        eprint!(" {key}={value}");
    }
    eprintln!();
}

impl Drop for PaintCannon {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::CssDimension;

    #[test]
    fn can_fold_style_mutation_into_create_command() {
        let mut command = EngineCommand::CreateElementWithId {
            id: DomId(1),
            style: Default::default(),
        };

        let style = create_command_style_mut(&mut command).expect("create command has style");
        apply_style_mutation(style, StyleMutation::Width(CssDimension::Percent(1.0)));

        match command {
            EngineCommand::CreateElementWithId { style, .. } => {
                assert!(matches!(style.width, CssDimension::Percent(value) if value == 1.0));
            }
            _ => panic!("expected create element command"),
        }
    }

    #[test]
    fn text_create_command_has_no_foldable_style() {
        let mut command = EngineCommand::CreateTextWithId {
            id: DomId(1),
            text: "hello".to_string(),
        };

        assert!(create_command_style_mut(&mut command).is_none());
    }

    #[test]
    fn max_height_style_command_is_supported() {
        let command = style_command(1, "max-height", "90%").unwrap();
        match command {
            EngineCommand::MutateStyle {
                mutation: StyleMutation::MaxHeight(CssDimension::Percent(value)),
                ..
            } => assert_eq!(value, 0.9),
            _ => panic!("expected max-height style mutation"),
        }
    }
}
