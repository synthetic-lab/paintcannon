use std::collections::HashMap;
use std::io::{self, Write};

use crossbeam_channel::Receiver;
use taffy::prelude::*;

use crate::style::*;
use crate::terminal::{query_terminal_size, reset_terminal, TerminalSize};

pub(crate) enum RenderCommand {
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
    SetFlexWrap {
        id: u32,
        flex_wrap: LayoutFlexWrap,
    },
    SetFlexFlow {
        id: u32,
        direction: LayoutFlexDirection,
        flex_wrap: LayoutFlexWrap,
    },
    SetFlexBasis {
        id: u32,
        flex_basis: CssDimension,
    },
    SetFlexGrow {
        id: u32,
        flex_grow: f32,
    },
    SetFlexShrink {
        id: u32,
        flex_shrink: f32,
    },
    SetFlex {
        id: u32,
        flex_grow: f32,
        flex_shrink: f32,
        flex_basis: CssDimension,
    },
    SetJustifyContent {
        id: u32,
        justify_content: LayoutJustifyContent,
    },
    SetAlignItems {
        id: u32,
        align_items: LayoutAlignItems,
    },
    SetAlignSelf {
        id: u32,
        align_self: LayoutAlignItems,
    },
    SetAlignContent {
        id: u32,
        align_content: LayoutJustifyContent,
    },
    SetJustifyItems {
        id: u32,
        justify_items: LayoutAlignItems,
    },
    SetJustifySelf {
        id: u32,
        justify_self: LayoutAlignItems,
    },
    SetGap {
        id: u32,
        row_gap: CssLengthPercentage,
        column_gap: CssLengthPercentage,
    },
    SetRowGap {
        id: u32,
        row_gap: CssLengthPercentage,
    },
    SetColumnGap {
        id: u32,
        column_gap: CssLengthPercentage,
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
    SetGridTemplateColumns {
        id: u32,
        tracks: Vec<CssGridTemplateTrack>,
    },
    SetGridTemplateRows {
        id: u32,
        tracks: Vec<CssGridTemplateTrack>,
    },
    SetGridAutoColumns {
        id: u32,
        tracks: Vec<CssTrackSizing>,
    },
    SetGridAutoRows {
        id: u32,
        tracks: Vec<CssTrackSizing>,
    },
    SetGridAutoFlow {
        id: u32,
        grid_auto_flow: LayoutGridAutoFlow,
    },
    SetGridColumn {
        id: u32,
        placement: CssGridLine,
    },
    SetGridRow {
        id: u32,
        placement: CssGridLine,
    },
    SetGridColumnStart {
        id: u32,
        placement: CssGridPlacement,
    },
    SetGridColumnEnd {
        id: u32,
        placement: CssGridPlacement,
    },
    SetGridRowStart {
        id: u32,
        placement: CssGridPlacement,
    },
    SetGridRowEnd {
        id: u32,
        placement: CssGridPlacement,
    },
    InvalidateFrame,
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
            RenderCommand::SetFlexWrap { id, flex_wrap } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_wrap = flex_wrap;
                }
            }
            RenderCommand::SetFlexFlow {
                id,
                direction,
                flex_wrap,
            } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_direction = direction;
                    node.style.flex_wrap = flex_wrap;
                }
            }
            RenderCommand::SetFlexBasis { id, flex_basis } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_basis = flex_basis;
                }
            }
            RenderCommand::SetFlexGrow { id, flex_grow } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_grow = flex_grow;
                }
            }
            RenderCommand::SetFlexShrink { id, flex_shrink } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_shrink = flex_shrink;
                }
            }
            RenderCommand::SetFlex {
                id,
                flex_grow,
                flex_shrink,
                flex_basis,
            } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.flex_grow = flex_grow;
                    node.style.flex_shrink = flex_shrink;
                    node.style.flex_basis = flex_basis;
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
            RenderCommand::SetAlignSelf { id, align_self } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.align_self = Some(align_self);
                }
            }
            RenderCommand::SetAlignContent { id, align_content } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.align_content = Some(align_content);
                }
            }
            RenderCommand::SetJustifyItems { id, justify_items } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.justify_items = Some(justify_items);
                }
            }
            RenderCommand::SetJustifySelf { id, justify_self } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.justify_self = Some(justify_self);
                }
            }
            RenderCommand::SetGap {
                id,
                row_gap,
                column_gap,
            } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.row_gap = row_gap;
                    node.style.column_gap = column_gap;
                }
            }
            RenderCommand::SetRowGap { id, row_gap } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.row_gap = row_gap;
                }
            }
            RenderCommand::SetColumnGap { id, column_gap } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.column_gap = column_gap;
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
            RenderCommand::SetGridTemplateColumns { id, tracks } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_template_columns = tracks;
                }
            }
            RenderCommand::SetGridTemplateRows { id, tracks } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_template_rows = tracks;
                }
            }
            RenderCommand::SetGridAutoColumns { id, tracks } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_auto_columns = tracks;
                }
            }
            RenderCommand::SetGridAutoRows { id, tracks } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_auto_rows = tracks;
                }
            }
            RenderCommand::SetGridAutoFlow { id, grid_auto_flow } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_auto_flow = grid_auto_flow;
                }
            }
            RenderCommand::SetGridColumn { id, placement } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_column = placement;
                }
            }
            RenderCommand::SetGridRow { id, placement } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_row = placement;
                }
            }
            RenderCommand::SetGridColumnStart { id, placement } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_column.start = placement;
                }
            }
            RenderCommand::SetGridColumnEnd { id, placement } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_column.end = placement;
                }
            }
            RenderCommand::SetGridRowStart { id, placement } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_row.start = placement;
                }
            }
            RenderCommand::SetGridRowEnd { id, placement } => {
                if let Some(DomNode::Div(node)) = self.nodes.get_mut(&id) {
                    node.style.grid_row.end = placement;
                }
            }
            RenderCommand::Render => {
                self.render();
            }
            RenderCommand::InvalidateFrame => {
                self.previous_frame = None;
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

pub(crate) fn renderer_loop(rx: Receiver<RenderCommand>) {
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
