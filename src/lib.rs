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
            "justify-content" | "justifyContent" => RenderCommand::SetJustifyContent {
                id,
                justify_content: parse_justify_content(&value)?,
            },
            "align-items" | "alignItems" => RenderCommand::SetAlignItems {
                id,
                align_items: parse_align_items(&value)?,
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
    CreateText {
        id: u32,
        text: String,
    },
    SetText {
        id: u32,
        text: String,
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
    SetJustifyContent {
        id: u32,
        justify_content: LayoutJustifyContent,
    },
    SetAlignItems {
        id: u32,
        align_items: LayoutAlignItems,
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
enum DomNode {
    Div(DivNode),
    Text(TextNode),
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
struct TextNode {
    text: String,
}

impl TextNode {
    fn style(&self) -> Style {
        let TextMetrics { width, height } = measure_text(&self.text);

        Style {
            size: Size {
                width: Dimension::length(width as f32),
                height: Dimension::length(height as f32),
            },
            ..Default::default()
        }
    }
}

#[derive(Clone)]
struct DivStyle {
    display: LayoutDisplay,
    flex_direction: LayoutFlexDirection,
    justify_content: Option<LayoutJustifyContent>,
    align_items: Option<LayoutAlignItems>,
    width: CssDimension,
    height: CssDimension,
    background: Background,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            display: LayoutDisplay::Block,
            flex_direction: LayoutFlexDirection::Row,
            justify_content: None,
            align_items: None,
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
enum LayoutJustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy)]
enum LayoutAlignItems {
    Start,
    Center,
    End,
    Stretch,
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
    nodes: HashMap<u32, DomNode>,
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
                self.nodes.insert(id, DomNode::Div(DivNode::default()));
            }
            RenderCommand::CreateText { id, text } => {
                self.nodes.insert(id, DomNode::Text(TextNode { text }));
            }
            RenderCommand::SetText { id, text } => {
                if let Some(DomNode::Text(node)) = self.nodes.get_mut(&id) {
                    node.text = text;
                }
            }
            RenderCommand::SetRoot { id } => {
                self.root = Some(id);
            }
            RenderCommand::AppendChild { parent, child } => {
                if self.nodes.contains_key(&child) {
                    if let Some(DomNode::Div(parent)) = self.nodes.get_mut(&parent) {
                        parent.children.push(child);
                    }
                }
            }
            RenderCommand::SetDisplay { id, display } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.display = display;
                }
            }
            RenderCommand::SetFlexDirection { id, direction } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_direction = direction;
                }
            }
            RenderCommand::SetJustifyContent {
                id,
                justify_content,
            } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.justify_content = Some(justify_content);
                }
            }
            RenderCommand::SetAlignItems { id, align_items } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.align_items = Some(align_items);
                }
            }
            RenderCommand::SetWidth { id, width } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.width = width;
                }
            }
            RenderCommand::SetHeight { id, height } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.height = height;
                }
            }
            RenderCommand::SetBackground { id, background } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
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
        let taffy_id = match self.nodes.get(&id)? {
            DomNode::Div(node) => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|child| self.build_taffy(*child, taffy, taffy_ids))
                    .collect::<Vec<_>>();

                taffy
                    .new_with_children(node.style.to_taffy(), &children)
                    .ok()?
            }
            DomNode::Text(node) => taffy.new_leaf(node.style()).ok()?,
        };

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
        let Some(dom_node) = self.nodes.get(&id) else {
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

        match dom_node {
            DomNode::Div(node) => {
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
            DomNode::Text(node) => {
                frame.write_text(x.round() as i32, y.round() as i32, &node.text);
            }
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
            justify_content: self.justify_content.map(|value| match value {
                LayoutJustifyContent::Start => JustifyContent::Start,
                LayoutJustifyContent::Center => JustifyContent::Center,
                LayoutJustifyContent::End => JustifyContent::End,
                LayoutJustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
                LayoutJustifyContent::SpaceAround => JustifyContent::SpaceAround,
                LayoutJustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
            }),
            align_items: self.align_items.map(|value| match value {
                LayoutAlignItems::Start => AlignItems::Start,
                LayoutAlignItems::Center => AlignItems::Center,
                LayoutAlignItems::End => AlignItems::End,
                LayoutAlignItems::Stretch => AlignItems::Stretch,
            }),
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
    cells: Vec<Cell>,
}

impl Frame {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
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
                self.cells[start + col] = Cell {
                    background,
                    character: ' ',
                };
            }
        }
    }

    fn write_text(&mut self, x: i32, y: i32, text: &str) {
        for (line_offset, line) in text.lines().enumerate() {
            let row = y + line_offset as i32;
            if row < 0 || row >= self.height as i32 {
                continue;
            }

            let mut col = x;
            for character in line.chars() {
                if col >= self.width as i32 {
                    break;
                }

                if col >= 0 {
                    let index = row as usize * self.width + col as usize;
                    self.cells[index].character = character;
                }
                col += 1;
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
            let cell = self.cells[row * self.width + col];
            let background = cell.background;
            if background != current {
                write!(out, "{}", background.ansi_bg())?;
                current = background;
            }
            write!(out, "{}", cell.character)?;
        }

        write!(out, "\x1b[49m")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cell {
    background: Background,
    character: char,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            background: Background::Default,
            character: ' ',
        }
    }
}

struct TextMetrics {
    width: usize,
    height: usize,
}

fn measure_text(text: &str) -> TextMetrics {
    let mut width = 0;
    let mut height = 0;

    for line in text.lines() {
        height += 1;
        width = width.max(line.chars().count());
    }

    TextMetrics {
        width,
        height: height.max(1),
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

fn parse_justify_content(value: &str) -> Result<LayoutJustifyContent> {
    match value.trim() {
        "start" | "flex-start" => Ok(LayoutJustifyContent::Start),
        "center" => Ok(LayoutJustifyContent::Center),
        "end" | "flex-end" => Ok(LayoutJustifyContent::End),
        "space-between" => Ok(LayoutJustifyContent::SpaceBetween),
        "space-around" => Ok(LayoutJustifyContent::SpaceAround),
        "space-evenly" => Ok(LayoutJustifyContent::SpaceEvenly),
        value => Err(Error::from_reason(format!(
            "unsupported justify-content: {value}"
        ))),
    }
}

fn parse_align_items(value: &str) -> Result<LayoutAlignItems> {
    match value.trim() {
        "start" | "flex-start" => Ok(LayoutAlignItems::Start),
        "center" => Ok(LayoutAlignItems::Center),
        "end" | "flex-end" => Ok(LayoutAlignItems::End),
        "stretch" => Ok(LayoutAlignItems::Stretch),
        value => Err(Error::from_reason(format!(
            "unsupported align-items: {value}"
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
