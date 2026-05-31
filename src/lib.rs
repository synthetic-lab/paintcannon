use std::collections::HashMap;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{bounded, Receiver, Sender};
use napi::{Error, Result};
use napi_derive::napi;
use taffy::prelude::*;

const RENDER_QUEUE_CAPACITY: usize = 32 * 1024;

#[napi(object)]
pub struct TerminalSize {
    pub cols: u32,
    pub rows: u32,
}

#[napi]
pub struct PaintCannon {
    tx: Sender<RenderCommand>,
    thread: Option<JoinHandle<()>>,
    next_id: u32,
}

#[napi]
impl PaintCannon {
    #[napi(constructor)]
    pub fn new() -> Self {
        let (tx, rx) = bounded(RENDER_QUEUE_CAPACITY);
        let thread = thread::spawn(move || renderer_loop(rx));

        Self {
            tx,
            thread: Some(thread),
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

    #[napi]
    pub fn render(&self) -> Result<()> {
        self.send(RenderCommand::Render)
    }

    #[napi]
    pub fn stop(&mut self) -> Result<()> {
        self.shutdown();
        Ok(())
    }
}

impl PaintCannon {
    fn send(&self, command: RenderCommand) -> Result<()> {
        self.tx
            .send(command)
            .map_err(|_| Error::from_reason("renderer thread stopped"))
    }

    fn shutdown(&mut self) {
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

enum RenderCommand {
    CreateDiv {
        id: u32,
    },
    SetRoot {
        id: u32,
    },
    AppendChild {
        parent: u32,
        child: u32,
    },
    SetDisplay {
        id: u32,
        display: LayoutDisplay,
    },
    SetFlexDirection {
        id: u32,
        direction: LayoutFlexDirection,
    },
    SetWidth {
        id: u32,
        width: CssDimension,
    },
    SetHeight {
        id: u32,
        height: CssDimension,
    },
    SetBackground {
        id: u32,
        background: Background,
    },
    Render,
    Shutdown,
}

#[derive(Clone)]
struct DivNode {
    children: Vec<u32>,
    style: DivStyle,
}

impl Default for DivNode {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            style: DivStyle::default(),
        }
    }
}

#[derive(Clone)]
struct DivStyle {
    display: LayoutDisplay,
    flex_direction: LayoutFlexDirection,
    width: CssDimension,
    height: CssDimension,
    background: Background,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            display: LayoutDisplay::Block,
            flex_direction: LayoutFlexDirection::Row,
            width: CssDimension::Auto,
            height: CssDimension::Auto,
            background: Background::Default,
        }
    }
}

#[derive(Clone, Copy)]
enum LayoutDisplay {
    Block,
    Flex,
    Grid,
}

#[derive(Clone, Copy)]
enum LayoutFlexDirection {
    Row,
    Column,
}

#[derive(Clone, Copy)]
enum CssDimension {
    Auto,
    Length(f32),
    Percent(f32),
}

impl CssDimension {
    fn to_taffy(self) -> Dimension {
        match self {
            Self::Auto => Dimension::AUTO,
            Self::Length(value) => Dimension::length(value),
            Self::Percent(value) => Dimension::percent(value),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Background {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Background {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "black" => Some(Self::Black),
            "red" => Some(Self::Red),
            "green" => Some(Self::Green),
            "yellow" => Some(Self::Yellow),
            "blue" => Some(Self::Blue),
            "magenta" => Some(Self::Magenta),
            "cyan" => Some(Self::Cyan),
            "white" => Some(Self::White),
            _ => None,
        }
    }

    fn ansi_bg(self) -> &'static str {
        match self {
            Self::Default => "\x1b[49m",
            Self::Black => "\x1b[40m",
            Self::Red => "\x1b[41m",
            Self::Green => "\x1b[42m",
            Self::Yellow => "\x1b[43m",
            Self::Blue => "\x1b[44m",
            Self::Magenta => "\x1b[45m",
            Self::Cyan => "\x1b[46m",
            Self::White => "\x1b[47m",
        }
    }
}

struct Renderer {
    root: Option<u32>,
    nodes: HashMap<u32, DivNode>,
    previous_frame: Option<Frame>,
}

impl Renderer {
    fn new() -> Self {
        Self {
            root: None,
            nodes: HashMap::new(),
            previous_frame: None,
        }
    }

    fn apply(&mut self, command: RenderCommand) -> bool {
        match command {
            RenderCommand::CreateDiv { id } => {
                self.nodes.insert(id, DivNode::default());
            }
            RenderCommand::SetRoot { id } => {
                self.root = Some(id);
            }
            RenderCommand::AppendChild { parent, child } => {
                if self.nodes.contains_key(&child) {
                    if let Some(parent) = self.nodes.get_mut(&parent) {
                        parent.children.push(child);
                    }
                }
            }
            RenderCommand::SetDisplay { id, display } => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.style.display = display;
                }
            }
            RenderCommand::SetFlexDirection { id, direction } => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.style.flex_direction = direction;
                }
            }
            RenderCommand::SetWidth { id, width } => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.style.width = width;
                }
            }
            RenderCommand::SetHeight { id, height } => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.style.height = height;
                }
            }
            RenderCommand::SetBackground { id, background } => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.style.background = background;
                }
            }
            RenderCommand::Render => {
                self.render();
            }
            RenderCommand::Shutdown => return false,
        }

        true
    }

    fn render(&mut self) {
        let Some(root) = self.root else {
            return;
        };

        let TerminalSize { cols, rows } = query_terminal_size();
        let mut taffy = TaffyTree::<u32>::new();
        let mut taffy_ids = HashMap::new();
        let Some(root_node) = self.build_taffy(root, &mut taffy, &mut taffy_ids) else {
            return;
        };

        let available = Size {
            width: AvailableSpace::Definite(cols as f32),
            height: AvailableSpace::Definite(rows as f32),
        };

        if taffy.compute_layout(root_node, available).is_err() {
            return;
        }

        let mut frame = Frame::new(cols as usize, rows as usize);
        self.paint_node(root, 0.0, 0.0, &taffy, &taffy_ids, &mut frame);
        let _ = frame.write_diff_to_stdout(self.previous_frame.as_ref());
        self.previous_frame = Some(frame);
    }

    fn build_taffy(
        &self,
        id: u32,
        taffy: &mut TaffyTree<u32>,
        taffy_ids: &mut HashMap<u32, NodeId>,
    ) -> Option<NodeId> {
        let Some(node) = self.nodes.get(&id) else {
            return None;
        };

        let children = node
            .children
            .iter()
            .filter_map(|child| self.build_taffy(*child, taffy, taffy_ids))
            .collect::<Vec<_>>();

        let taffy_id = taffy
            .new_with_children(node.style.to_taffy(), &children)
            .ok()?;
        taffy_ids.insert(id, taffy_id);
        Some(taffy_id)
    }

    fn paint_node(
        &self,
        id: u32,
        parent_x: f32,
        parent_y: f32,
        taffy: &TaffyTree<u32>,
        taffy_ids: &HashMap<u32, NodeId>,
        frame: &mut Frame,
    ) {
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let Some(taffy_id) = taffy_ids.get(&id) else {
            return;
        };
        let Ok(layout) = taffy.layout(*taffy_id) else {
            return;
        };

        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;
        frame.fill_rect(
            x.round() as i32,
            y.round() as i32,
            layout.size.width.round() as i32,
            layout.size.height.round() as i32,
            node.style.background,
        );

        for child in &node.children {
            self.paint_node(*child, x, y, taffy, taffy_ids, frame);
        }
    }
}

impl DivStyle {
    fn to_taffy(&self) -> Style {
        Style {
            display: match self.display {
                LayoutDisplay::Block => Display::Block,
                LayoutDisplay::Flex => Display::Flex,
                LayoutDisplay::Grid => Display::Grid,
            },
            flex_direction: match self.flex_direction {
                LayoutFlexDirection::Row => FlexDirection::Row,
                LayoutFlexDirection::Column => FlexDirection::Column,
            },
            size: Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            ..Default::default()
        }
    }
}

struct Frame {
    width: usize,
    height: usize,
    cells: Vec<Background>,
}

impl Frame {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Background::Default; width * height],
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, background: Background) {
        if background == Background::Default || width <= 0 || height <= 0 {
            return;
        }

        let left = x.max(0) as usize;
        let top = y.max(0) as usize;
        let right = (x + width).min(self.width as i32).max(0) as usize;
        let bottom = (y + height).min(self.height as i32).max(0) as usize;

        for row in top..bottom {
            let start = row * self.width;
            for col in left..right {
                self.cells[start + col] = background;
            }
        }
    }

    fn write_diff_to_stdout(&self, previous: Option<&Frame>) -> io::Result<()> {
        let mut out = io::stdout().lock();
        write!(out, "\x1b[?25l")?;

        let Some(previous) = previous else {
            self.write_full_to(&mut out)?;
            return out.flush();
        };

        if previous.width != self.width || previous.height != self.height {
            write!(out, "\x1b[2J")?;
            self.write_full_to(&mut out)?;
            return out.flush();
        }

        for row in 0..self.height {
            let mut col = 0;
            while col < self.width {
                let index = row * self.width + col;
                if previous.cells[index] == self.cells[index] {
                    col += 1;
                    continue;
                }

                let start = col;
                while col < self.width {
                    let index = row * self.width + col;
                    if previous.cells[index] == self.cells[index] {
                        break;
                    }
                    col += 1;
                }

                self.write_span_to(&mut out, row, start, col)?;
            }
        }

        out.flush()
    }

    fn write_full_to(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, "\x1b[H")?;

        for row in 0..self.height {
            self.write_span_to(out, row, 0, self.width)?;
        }

        Ok(())
    }

    fn write_span_to(
        &self,
        out: &mut impl Write,
        row: usize,
        start_col: usize,
        end_col: usize,
    ) -> io::Result<()> {
        if start_col >= end_col {
            return Ok(());
        }

        write!(out, "\x1b[{};{}H", row + 1, start_col + 1)?;

        let mut current = Background::Default;
        for col in start_col..end_col {
            let background = self.cells[row * self.width + col];
            if background != current {
                write!(out, "{}", background.ansi_bg())?;
                current = background;
            }
            write!(out, " ")?;
        }

        write!(out, "\x1b[49m")
    }
}

fn renderer_loop(rx: Receiver<RenderCommand>) {
    let mut renderer = Renderer::new();

    while let Ok(command) = rx.recv() {
        if !renderer.apply(command) {
            break;
        }

        while let Ok(command) = rx.try_recv() {
            if !renderer.apply(command) {
                return reset_terminal();
            }
        }
    }

    reset_terminal();
}

fn reset_terminal() {
    let _ = write!(io::stdout().lock(), "\x1b[0m\x1b[?25h\n");
}

fn query_terminal_size() -> TerminalSize {
    query_terminal_size_from(io::stdout())
        .or_else(|| query_terminal_size_from(io::stderr()))
        .or_else(|| query_terminal_size_from(io::stdin()))
        .unwrap_or(TerminalSize { cols: 80, rows: 24 })
}

#[cfg(unix)]
fn query_terminal_size_from<T: AsRawFd>(stream: T) -> Option<TerminalSize> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe { libc::ioctl(stream.as_raw_fd(), libc::TIOCGWINSZ, &mut size) };
    if result == 0 && size.ws_col > 0 && size.ws_row > 0 {
        Some(TerminalSize {
            cols: u32::from(size.ws_col),
            rows: u32::from(size.ws_row),
        })
    } else {
        None
    }
}

#[cfg(not(unix))]
fn query_terminal_size_from<T>(_stream: T) -> Option<TerminalSize> {
    None
}

fn parse_display(value: &str) -> Result<LayoutDisplay> {
    match value.trim() {
        "block" => Ok(LayoutDisplay::Block),
        "flex" => Ok(LayoutDisplay::Flex),
        "grid" => Ok(LayoutDisplay::Grid),
        value => Err(Error::from_reason(format!("unsupported display: {value}"))),
    }
}

fn parse_flex_direction(value: &str) -> Result<LayoutFlexDirection> {
    match value.trim() {
        "row" => Ok(LayoutFlexDirection::Row),
        "column" => Ok(LayoutFlexDirection::Column),
        value => Err(Error::from_reason(format!(
            "unsupported flex direction: {value}"
        ))),
    }
}

fn parse_dimension(value: &str) -> Result<CssDimension> {
    let value = value.trim();

    if value == "auto" || value.is_empty() {
        return Ok(CssDimension::Auto);
    }

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid percentage dimension: {value}")))?;
        return Ok(CssDimension::Percent(percent / 100.0));
    }

    let number = value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid dimension: {value}")))?;

    Ok(CssDimension::Length(number))
}
