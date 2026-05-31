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
    flex_wrap: LayoutFlexWrap,
    flex_basis: CssDimension,
    flex_grow: f32,
    flex_shrink: f32,
    justify_content: Option<LayoutJustifyContent>,
    align_items: Option<LayoutAlignItems>,
    align_self: Option<LayoutAlignItems>,
    align_content: Option<LayoutJustifyContent>,
    justify_items: Option<LayoutAlignItems>,
    justify_self: Option<LayoutAlignItems>,
    row_gap: CssLengthPercentage,
    column_gap: CssLengthPercentage,
    width: CssDimension,
    height: CssDimension,
    grid_template_columns: Vec<CssGridTemplateTrack>,
    grid_template_rows: Vec<CssGridTemplateTrack>,
    grid_auto_columns: Vec<CssTrackSizing>,
    grid_auto_rows: Vec<CssTrackSizing>,
    grid_auto_flow: LayoutGridAutoFlow,
    grid_column: CssGridLine,
    grid_row: CssGridLine,
    background: Background,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            display: LayoutDisplay::Block,
            flex_direction: LayoutFlexDirection::Row,
            flex_wrap: LayoutFlexWrap::NoWrap,
            flex_basis: CssDimension::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            justify_content: None,
            align_items: None,
            align_self: None,
            align_content: None,
            justify_items: None,
            justify_self: None,
            row_gap: CssLengthPercentage::Length(0.0),
            column_gap: CssLengthPercentage::Length(0.0),
            width: CssDimension::Auto,
            height: CssDimension::Auto,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_auto_columns: Vec::new(),
            grid_auto_rows: Vec::new(),
            grid_auto_flow: LayoutGridAutoFlow::Row,
            grid_column: CssGridLine::default(),
            grid_row: CssGridLine::default(),
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
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy)]
enum LayoutFlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy)]
enum LayoutJustifyContent {
    Start,
    FlexStart,
    Center,
    End,
    FlexEnd,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy)]
enum LayoutAlignItems {
    Start,
    FlexStart,
    Center,
    End,
    FlexEnd,
    Baseline,
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

#[derive(Clone, Copy)]
enum CssLengthPercentage {
    Length(f32),
    Percent(f32),
}

impl CssLengthPercentage {
    fn to_taffy(self) -> LengthPercentage {
        match self {
            Self::Length(value) => LengthPercentage::length(value),
            Self::Percent(value) => LengthPercentage::percent(value),
        }
    }
}

#[derive(Clone, Copy)]
enum CssTrackSizing {
    Auto,
    Length(f32),
    Percent(f32),
    Fr(f32),
    MinContent,
    MaxContent,
}

impl CssTrackSizing {
    fn to_taffy(self) -> TrackSizingFunction {
        match self {
            Self::Auto => TrackSizingFunction::AUTO,
            Self::Length(value) => TrackSizingFunction::from_length(value),
            Self::Percent(value) => TrackSizingFunction::from_percent(value),
            Self::Fr(value) => TrackSizingFunction::from_fr(value),
            Self::MinContent => TrackSizingFunction::MIN_CONTENT,
            Self::MaxContent => TrackSizingFunction::MAX_CONTENT,
        }
    }
}

#[derive(Clone, Copy)]
enum CssGridTemplateTrack {
    Single(CssTrackSizing),
}

impl CssGridTemplateTrack {
    fn to_taffy(self) -> GridTemplateComponent<String> {
        match self {
            Self::Single(track) => GridTemplateComponent::Single(track.to_taffy()),
        }
    }
}

#[derive(Clone, Copy)]
enum LayoutGridAutoFlow {
    Row,
    Column,
    RowDense,
    ColumnDense,
}

impl LayoutGridAutoFlow {
    fn to_taffy(self) -> GridAutoFlow {
        match self {
            Self::Row => GridAutoFlow::Row,
            Self::Column => GridAutoFlow::Column,
            Self::RowDense => GridAutoFlow::RowDense,
            Self::ColumnDense => GridAutoFlow::ColumnDense,
        }
    }
}

#[derive(Clone, Copy)]
enum CssGridPlacement {
    Auto,
    Line(i16),
    Span(u16),
}

impl CssGridPlacement {
    fn to_taffy(self) -> GridPlacement {
        match self {
            Self::Auto => GridPlacement::Auto,
            Self::Line(value) => GridPlacement::Line(value.into()),
            Self::Span(value) => GridPlacement::Span(value),
        }
    }
}

#[derive(Clone, Copy)]
struct CssGridLine {
    start: CssGridPlacement,
    end: CssGridPlacement,
}

impl Default for CssGridLine {
    fn default() -> Self {
        Self {
            start: CssGridPlacement::Auto,
            end: CssGridPlacement::Auto,
        }
    }
}

impl CssGridLine {
    fn to_taffy(self) -> Line<GridPlacement> {
        Line {
            start: self.start.to_taffy(),
            end: self.end.to_taffy(),
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
                LayoutFlexDirection::RowReverse => FlexDirection::RowReverse,
                LayoutFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
            },
            flex_wrap: match self.flex_wrap {
                LayoutFlexWrap::NoWrap => FlexWrap::NoWrap,
                LayoutFlexWrap::Wrap => FlexWrap::Wrap,
                LayoutFlexWrap::WrapReverse => FlexWrap::WrapReverse,
            },
            flex_basis: self.flex_basis.to_taffy(),
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            justify_content: self.justify_content.map(|value| match value {
                LayoutJustifyContent::Start => JustifyContent::Start,
                LayoutJustifyContent::FlexStart => JustifyContent::FlexStart,
                LayoutJustifyContent::Center => JustifyContent::Center,
                LayoutJustifyContent::End => JustifyContent::End,
                LayoutJustifyContent::FlexEnd => JustifyContent::FlexEnd,
                LayoutJustifyContent::Stretch => JustifyContent::Stretch,
                LayoutJustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
                LayoutJustifyContent::SpaceAround => JustifyContent::SpaceAround,
                LayoutJustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
            }),
            align_items: self.align_items.map(layout_align_items_to_taffy),
            align_self: self.align_self.map(layout_align_items_to_taffy),
            align_content: self.align_content.map(|value| match value {
                LayoutJustifyContent::Start => AlignContent::Start,
                LayoutJustifyContent::FlexStart => AlignContent::FlexStart,
                LayoutJustifyContent::Center => AlignContent::Center,
                LayoutJustifyContent::End => AlignContent::End,
                LayoutJustifyContent::FlexEnd => AlignContent::FlexEnd,
                LayoutJustifyContent::Stretch => AlignContent::Stretch,
                LayoutJustifyContent::SpaceBetween => AlignContent::SpaceBetween,
                LayoutJustifyContent::SpaceAround => AlignContent::SpaceAround,
                LayoutJustifyContent::SpaceEvenly => AlignContent::SpaceEvenly,
            }),
            justify_items: self.justify_items.map(layout_align_items_to_taffy),
            justify_self: self.justify_self.map(layout_align_items_to_taffy),
            gap: Size {
                width: self.column_gap.to_taffy(),
                height: self.row_gap.to_taffy(),
            },
            size: Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            grid_template_columns: self
                .grid_template_columns
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_template_rows: self
                .grid_template_rows
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_auto_columns: self
                .grid_auto_columns
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_auto_rows: self
                .grid_auto_rows
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_auto_flow: self.grid_auto_flow.to_taffy(),
            grid_column: self.grid_column.to_taffy(),
            grid_row: self.grid_row.to_taffy(),
            ..Default::default()
        }
    }
}

fn layout_align_items_to_taffy(value: LayoutAlignItems) -> AlignItems {
    match value {
        LayoutAlignItems::Start => AlignItems::Start,
        LayoutAlignItems::FlexStart => AlignItems::FlexStart,
        LayoutAlignItems::Center => AlignItems::Center,
        LayoutAlignItems::End => AlignItems::End,
        LayoutAlignItems::FlexEnd => AlignItems::FlexEnd,
        LayoutAlignItems::Baseline => AlignItems::Baseline,
        LayoutAlignItems::Stretch => AlignItems::Stretch,
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
        "row-reverse" => Ok(LayoutFlexDirection::RowReverse),
        "column-reverse" => Ok(LayoutFlexDirection::ColumnReverse),
        value => Err(Error::from_reason(format!(
            "unsupported flex direction: {value}"
        ))),
    }
}

fn parse_flex_wrap(value: &str) -> Result<LayoutFlexWrap> {
    match value.trim() {
        "nowrap" => Ok(LayoutFlexWrap::NoWrap),
        "wrap" => Ok(LayoutFlexWrap::Wrap),
        "wrap-reverse" => Ok(LayoutFlexWrap::WrapReverse),
        value => Err(Error::from_reason(format!(
            "unsupported flex-wrap: {value}"
        ))),
    }
}

fn parse_flex_flow(value: &str) -> Result<(LayoutFlexDirection, LayoutFlexWrap)> {
    let mut direction = None;
    let mut flex_wrap = None;

    for part in value.split_whitespace() {
        if direction.is_none() {
            if let Ok(parsed) = parse_flex_direction(part) {
                direction = Some(parsed);
                continue;
            }
        }

        if flex_wrap.is_none() {
            if let Ok(parsed) = parse_flex_wrap(part) {
                flex_wrap = Some(parsed);
                continue;
            }
        }

        return Err(Error::from_reason(format!(
            "unsupported flex-flow: {value}"
        )));
    }

    Ok((
        direction.unwrap_or(LayoutFlexDirection::Row),
        flex_wrap.unwrap_or(LayoutFlexWrap::NoWrap),
    ))
}

fn parse_flex_shorthand(id: u32, value: &str) -> Result<RenderCommand> {
    let value = value.trim();
    if value == "none" {
        return Ok(RenderCommand::SetFlex {
            id,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            flex_basis: CssDimension::Auto,
        });
    }
    if value == "auto" {
        return Ok(RenderCommand::SetFlex {
            id,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Auto,
        });
    }
    if value == "initial" {
        return Ok(RenderCommand::SetFlex {
            id,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Auto,
        });
    }

    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [grow] => Ok(RenderCommand::SetFlex {
            id,
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Length(0.0),
        }),
        [grow, shrink] => Ok(RenderCommand::SetFlex {
            id,
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: parse_non_negative_number("flex-shrink", shrink)?,
            flex_basis: CssDimension::Length(0.0),
        }),
        [grow, shrink, basis] => Ok(RenderCommand::SetFlex {
            id,
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: parse_non_negative_number("flex-shrink", shrink)?,
            flex_basis: parse_dimension(basis)?,
        }),
        _ => Err(Error::from_reason(format!(
            "unsupported flex shorthand: {value}"
        ))),
    }
}

fn parse_justify_content(value: &str) -> Result<LayoutJustifyContent> {
    match value.trim() {
        "start" => Ok(LayoutJustifyContent::Start),
        "flex-start" => Ok(LayoutJustifyContent::FlexStart),
        "center" => Ok(LayoutJustifyContent::Center),
        "end" => Ok(LayoutJustifyContent::End),
        "flex-end" => Ok(LayoutJustifyContent::FlexEnd),
        "stretch" => Ok(LayoutJustifyContent::Stretch),
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
        "start" => Ok(LayoutAlignItems::Start),
        "flex-start" => Ok(LayoutAlignItems::FlexStart),
        "center" => Ok(LayoutAlignItems::Center),
        "end" => Ok(LayoutAlignItems::End),
        "flex-end" => Ok(LayoutAlignItems::FlexEnd),
        "baseline" => Ok(LayoutAlignItems::Baseline),
        "stretch" => Ok(LayoutAlignItems::Stretch),
        value => Err(Error::from_reason(format!(
            "unsupported align-items: {value}"
        ))),
    }
}

fn parse_non_negative_number(property: &str, value: &str) -> Result<f32> {
    let parsed = value
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid {property}: {value}")))?;

    if parsed.is_sign_negative() {
        Err(Error::from_reason(format!(
            "{property} must be non-negative: {value}"
        )))
    } else {
        Ok(parsed)
    }
}

fn parse_gap(value: &str) -> Result<(CssLengthPercentage, CssLengthPercentage)> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [both] => {
            let parsed = parse_length_percentage(both)?;
            Ok((parsed, parsed))
        }
        [row, column] => Ok((
            parse_length_percentage(row)?,
            parse_length_percentage(column)?,
        )),
        _ => Err(Error::from_reason(format!("invalid gap: {value}"))),
    }
}

fn parse_length_percentage(value: &str) -> Result<CssLengthPercentage> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid percentage: {value}")))?;
        return Ok(CssLengthPercentage::Percent(percent / 100.0));
    }

    let number = value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid length: {value}")))?;
    Ok(CssLengthPercentage::Length(number))
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

fn parse_grid_template_tracks(value: &str) -> Result<Vec<CssGridTemplateTrack>> {
    parse_grid_auto_tracks(value).map(|tracks| {
        tracks
            .into_iter()
            .map(CssGridTemplateTrack::Single)
            .collect()
    })
}

fn parse_grid_auto_tracks(value: &str) -> Result<Vec<CssTrackSizing>> {
    let tracks = value
        .split_whitespace()
        .map(parse_track_sizing)
        .collect::<Result<Vec<_>>>()?;

    if tracks.is_empty() {
        Err(Error::from_reason("grid track list cannot be empty"))
    } else {
        Ok(tracks)
    }
}

fn parse_track_sizing(value: &str) -> Result<CssTrackSizing> {
    let value = value.trim();
    match value {
        "auto" => return Ok(CssTrackSizing::Auto),
        "min-content" => return Ok(CssTrackSizing::MinContent),
        "max-content" => return Ok(CssTrackSizing::MaxContent),
        _ => {}
    }

    if let Some(fr) = value.strip_suffix("fr") {
        let fr = fr
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid fr track size: {value}")))?;
        return Ok(CssTrackSizing::Fr(fr));
    }

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid percentage track size: {value}")))?;
        return Ok(CssTrackSizing::Percent(percent / 100.0));
    }

    let number = value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid track size: {value}")))?;
    Ok(CssTrackSizing::Length(number))
}

fn parse_grid_auto_flow(value: &str) -> Result<LayoutGridAutoFlow> {
    let mut axis = None;
    let mut dense = false;

    for part in value.split_whitespace() {
        match part {
            "row" => axis = Some("row"),
            "column" => axis = Some("column"),
            "dense" => dense = true,
            _ => {
                return Err(Error::from_reason(format!(
                    "unsupported grid-auto-flow: {value}"
                )))
            }
        }
    }

    match (axis, dense) {
        (Some("row") | None, false) => Ok(LayoutGridAutoFlow::Row),
        (Some("column"), false) => Ok(LayoutGridAutoFlow::Column),
        (Some("row") | None, true) => Ok(LayoutGridAutoFlow::RowDense),
        (Some("column"), true) => Ok(LayoutGridAutoFlow::ColumnDense),
        _ => Err(Error::from_reason(format!(
            "unsupported grid-auto-flow: {value}"
        ))),
    }
}

fn parse_grid_line(value: &str) -> Result<CssGridLine> {
    let parts = value.split('/').map(str::trim).collect::<Vec<_>>();
    match parts.as_slice() {
        [single] => Ok(CssGridLine {
            start: parse_grid_placement(single)?,
            end: CssGridPlacement::Auto,
        }),
        [start, end] => Ok(CssGridLine {
            start: parse_grid_placement(start)?,
            end: parse_grid_placement(end)?,
        }),
        _ => Err(Error::from_reason(format!(
            "invalid grid placement: {value}"
        ))),
    }
}

fn parse_grid_placement(value: &str) -> Result<CssGridPlacement> {
    let value = value.trim();
    if value == "auto" {
        return Ok(CssGridPlacement::Auto);
    }

    if let Some(span) = value.strip_prefix("span ") {
        let span = span
            .trim()
            .parse::<u16>()
            .map_err(|_| Error::from_reason(format!("invalid grid span: {value}")))?;
        return Ok(CssGridPlacement::Span(span));
    }

    let line = value
        .parse::<i16>()
        .map_err(|_| Error::from_reason(format!("invalid grid line: {value}")))?;
    Ok(CssGridPlacement::Line(line))
}
