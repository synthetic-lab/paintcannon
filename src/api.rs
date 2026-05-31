use std::thread::{self, JoinHandle};

use crossbeam_channel::{bounded, Sender};
use napi::{Error, Result};
use napi_derive::napi;

use crate::input::{KeyboardEvent, TerminalInput};
use crate::renderer::{renderer_loop, RenderCommand};
use crate::style::{
    parse_align_items, parse_dimension, parse_display, parse_flex_direction, parse_flex_flow,
    parse_flex_shorthand, parse_flex_wrap, parse_gap, parse_grid_auto_flow, parse_grid_auto_tracks,
    parse_grid_line, parse_grid_placement, parse_grid_template_tracks, parse_justify_content,
    parse_length_percentage, parse_non_negative_number, Background,
};
use crate::terminal::{query_terminal_size, reset_terminal, TerminalSize};

const RENDER_QUEUE_CAPACITY: usize = 32 * 1024;
const DEFAULT_SYNTHETIC_KEYUP_MS: u32 = 180;

#[napi]
pub struct PaintCannon {
    tx: Sender<RenderCommand>,
    thread: Option<JoinHandle<()>>,
    input: Option<TerminalInput>,
    kitty_keyboard_enabled: bool,
    next_id: u32,
}

#[napi]
impl PaintCannon {
    #[napi(constructor)]
    pub fn new(force_compat_mode: Option<bool>) -> Self {
        let (tx, rx) = bounded(RENDER_QUEUE_CAPACITY);
        let thread = thread::spawn(move || renderer_loop(rx));
        let input = TerminalInput::start(
            DEFAULT_SYNTHETIC_KEYUP_MS,
            force_compat_mode.unwrap_or(false),
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
            next_id: 1,
        }
    }

    #[napi]
    pub fn create_div(&mut self) -> Result<u32> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(RenderCommand::CreateDiv { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_span(&mut self) -> Result<u32> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(RenderCommand::CreateSpan { id })?;
        Ok(id)
    }

    #[napi]
    pub fn create_text_node(&mut self, text: String) -> Result<u32> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(RenderCommand::CreateText { id, text })?;
        Ok(id)
    }

    #[napi]
    pub fn set_text_node_value(&self, id: u32, text: String) -> Result<()> {
        self.send(RenderCommand::SetText { id, text })
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
        let command = match property.as_str() {
            "display" => RenderCommand::SetDisplay {
                id,
                display: parse_display(&value)?,
            },
            "flex-direction" | "flexDirection" => RenderCommand::SetFlexDirection {
                id,
                direction: parse_flex_direction(&value)?,
            },
            "flex-wrap" | "flexWrap" => RenderCommand::SetFlexWrap {
                id,
                flex_wrap: parse_flex_wrap(&value)?,
            },
            "flex-flow" | "flexFlow" => {
                let (direction, flex_wrap) = parse_flex_flow(&value)?;
                RenderCommand::SetFlexFlow {
                    id,
                    direction,
                    flex_wrap,
                }
            }
            "flex-basis" | "flexBasis" => RenderCommand::SetFlexBasis {
                id,
                flex_basis: parse_dimension(&value)?,
            },
            "flex-grow" | "flexGrow" => RenderCommand::SetFlexGrow {
                id,
                flex_grow: parse_non_negative_number("flex-grow", &value)?,
            },
            "flex-shrink" | "flexShrink" => RenderCommand::SetFlexShrink {
                id,
                flex_shrink: parse_non_negative_number("flex-shrink", &value)?,
            },
            "flex" => parse_flex_shorthand(id, &value)?,
            "justify-content" | "justifyContent" => RenderCommand::SetJustifyContent {
                id,
                justify_content: parse_justify_content(&value)?,
            },
            "align-items" | "alignItems" => RenderCommand::SetAlignItems {
                id,
                align_items: parse_align_items(&value)?,
            },
            "align-self" | "alignSelf" => RenderCommand::SetAlignSelf {
                id,
                align_self: parse_align_items(&value)?,
            },
            "align-content" | "alignContent" => RenderCommand::SetAlignContent {
                id,
                align_content: parse_justify_content(&value)?,
            },
            "justify-items" | "justifyItems" => RenderCommand::SetJustifyItems {
                id,
                justify_items: parse_align_items(&value)?,
            },
            "justify-self" | "justifySelf" => RenderCommand::SetJustifySelf {
                id,
                justify_self: parse_align_items(&value)?,
            },
            "gap" => {
                let (row_gap, column_gap) = parse_gap(&value)?;
                RenderCommand::SetGap {
                    id,
                    row_gap,
                    column_gap,
                }
            }
            "row-gap" | "rowGap" => RenderCommand::SetRowGap {
                id,
                row_gap: parse_length_percentage(&value)?,
            },
            "column-gap" | "columnGap" => RenderCommand::SetColumnGap {
                id,
                column_gap: parse_length_percentage(&value)?,
            },
            "width" => RenderCommand::SetWidth {
                id,
                width: parse_dimension(&value)?,
            },
            "height" => RenderCommand::SetHeight {
                id,
                height: parse_dimension(&value)?,
            },
            "background" | "background-color" | "backgroundColor" => {
                let background = Background::parse(&value).ok_or_else(|| {
                    Error::from_reason(format!("unsupported background: {value}"))
                })?;
                RenderCommand::SetBackground { id, background }
            }
            "grid-template-columns" | "gridTemplateColumns" => {
                RenderCommand::SetGridTemplateColumns {
                    id,
                    tracks: parse_grid_template_tracks(&value)?,
                }
            }
            "grid-template-rows" | "gridTemplateRows" => RenderCommand::SetGridTemplateRows {
                id,
                tracks: parse_grid_template_tracks(&value)?,
            },
            "grid-auto-columns" | "gridAutoColumns" => RenderCommand::SetGridAutoColumns {
                id,
                tracks: parse_grid_auto_tracks(&value)?,
            },
            "grid-auto-rows" | "gridAutoRows" => RenderCommand::SetGridAutoRows {
                id,
                tracks: parse_grid_auto_tracks(&value)?,
            },
            "grid-auto-flow" | "gridAutoFlow" => RenderCommand::SetGridAutoFlow {
                id,
                grid_auto_flow: parse_grid_auto_flow(&value)?,
            },
            "grid-column" | "gridColumn" => RenderCommand::SetGridColumn {
                id,
                placement: parse_grid_line(&value)?,
            },
            "grid-row" | "gridRow" => RenderCommand::SetGridRow {
                id,
                placement: parse_grid_line(&value)?,
            },
            "grid-column-start" | "gridColumnStart" => RenderCommand::SetGridColumnStart {
                id,
                placement: parse_grid_placement(&value)?,
            },
            "grid-column-end" | "gridColumnEnd" => RenderCommand::SetGridColumnEnd {
                id,
                placement: parse_grid_placement(&value)?,
            },
            "grid-row-start" | "gridRowStart" => RenderCommand::SetGridRowStart {
                id,
                placement: parse_grid_placement(&value)?,
            },
            "grid-row-end" | "gridRowEnd" => RenderCommand::SetGridRowEnd {
                id,
                placement: parse_grid_placement(&value)?,
            },
            value => {
                return Err(Error::from_reason(format!(
                    "unsupported style property: {value}"
                )))
            }
        };

        self.send(command)
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
        self.send(RenderCommand::Render)
    }

    #[napi]
    pub fn drain_keyboard_events(&self) -> Vec<KeyboardEvent> {
        self.input
            .as_ref()
            .map(TerminalInput::drain)
            .unwrap_or_default()
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

impl Drop for PaintCannon {
    fn drop(&mut self) {
        self.shutdown();
    }
}
