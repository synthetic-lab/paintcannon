use std::collections::HashMap;
use std::io::{self, Write};

use crossbeam_channel::{Receiver, Sender};
use napi_derive::napi;
use taffy::prelude::*;

use crate::style::*;
use crate::terminal::{
    copy_text_to_clipboard, query_terminal_size, reset_terminal, write_synchronized_output_begin,
    write_synchronized_output_end, TerminalSize,
};

pub(crate) enum RenderCommand {
    CreateDiv {
        id: u32,
    },
    CreateSpan {
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
    SetOverflow {
        id: u32,
        overflow: LayoutOverflow,
    },
    SetOverflowX {
        id: u32,
        overflow: LayoutOverflow,
    },
    SetOverflowY {
        id: u32,
        overflow: LayoutOverflow,
    },
    SetScrollOffset {
        id: u32,
        scroll_left: u32,
        scroll_top: u32,
        response: Sender<Option<ScrollMetrics>>,
    },
    GetScrollMetrics {
        id: u32,
        response: Sender<Option<ScrollMetrics>>,
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
    SetSelectionBackground {
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
    HitTestClick {
        click: MouseClick,
        response: Sender<Option<ClickEvent>>,
    },
    HitTestPoint {
        x: u32,
        y: u32,
        response: Sender<Option<u32>>,
    },
    HandleTextSelection {
        event: SelectionMouseEvent,
    },
    InvalidateFrame,
    Render,
    Shutdown,
}

#[derive(Clone)]
pub(crate) struct MouseClick {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) button: u32,
    pub(crate) ctrl_key: bool,
    pub(crate) alt_key: bool,
    pub(crate) meta_key: bool,
    pub(crate) shift_key: bool,
}

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

#[derive(Clone)]
#[napi(object)]
pub struct ScrollMetrics {
    pub scroll_left: u32,
    pub scroll_top: u32,
    pub scroll_width: u32,
    pub scroll_height: u32,
    pub client_width: u32,
    pub client_height: u32,
}

pub(crate) struct SelectionMouseEvent {
    pub(crate) event_type: SelectionMouseEventType,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) button: u32,
}

#[derive(Clone, Copy)]
pub(crate) enum SelectionMouseEventType {
    Down,
    Drag,
    Scroll,
    Up,
}

#[derive(Clone)]
enum DomNode {
    Div(DivNode),
    Span(SpanNode),
    Text(TextNode),
}

#[derive(Clone)]
struct DivNode {
    children: Vec<u32>,
    style: DivStyle,
    scroll_left: u32,
    scroll_top: u32,
}

impl Default for DivNode {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            style: DivStyle::default(),
            scroll_left: 0,
            scroll_top: 0,
        }
    }
}

#[derive(Clone)]
struct SpanNode {
    children: Vec<u32>,
    style: DivStyle,
    scroll_left: u32,
    scroll_top: u32,
}

impl Default for SpanNode {
    fn default() -> Self {
        let mut style = DivStyle::default();
        style.display = LayoutDisplay::Inline;

        Self {
            children: Vec::new(),
            style,
            scroll_left: 0,
            scroll_top: 0,
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
    content_frame: Option<Frame>,
    hit_regions: Vec<HitRegion>,
    scroll_metrics: HashMap<u32, ScrollMetrics>,
    selection: Option<Selection>,
}

impl Renderer {
    fn new() -> Self {
        Self {
            root: None,
            nodes: HashMap::new(),
            previous_frame: None,
            content_frame: None,
            hit_regions: Vec::new(),
            scroll_metrics: HashMap::new(),
            selection: None,
        }
    }

    fn apply(&mut self, command: RenderCommand) -> bool {
        match command {
            RenderCommand::CreateDiv { id } => {
                self.nodes.insert(id, DomNode::Div(DivNode::default()));
            }
            RenderCommand::CreateSpan { id } => {
                self.nodes.insert(id, DomNode::Span(SpanNode::default()));
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
                    if let Some(parent) = self.children_mut(parent) {
                        parent.push(child);
                    }
                }
            }
            RenderCommand::SetDisplay { id, display } => {
                if let Some(node) = self.style_mut(id) {
                    node.display = display;
                }
            }
            RenderCommand::SetOverflow { id, overflow } => {
                if let Some(node) = self.style_mut(id) {
                    node.overflow_x = overflow;
                    node.overflow_y = overflow;
                }
            }
            RenderCommand::SetOverflowX { id, overflow } => {
                if let Some(node) = self.style_mut(id) {
                    node.overflow_x = overflow;
                }
            }
            RenderCommand::SetOverflowY { id, overflow } => {
                if let Some(node) = self.style_mut(id) {
                    node.overflow_y = overflow;
                }
            }
            RenderCommand::SetScrollOffset {
                id,
                scroll_left,
                scroll_top,
                response,
            } => {
                let metrics = self.set_scroll_offset(id, scroll_left, scroll_top);
                let _ = response.send(metrics);
            }
            RenderCommand::GetScrollMetrics { id, response } => {
                let _ = response.send(self.scroll_metrics_for(id));
            }
            RenderCommand::SetFlexDirection { id, direction } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_direction = direction;
                }
            }
            RenderCommand::SetFlexWrap { id, flex_wrap } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_wrap = flex_wrap;
                }
            }
            RenderCommand::SetFlexFlow {
                id,
                direction,
                flex_wrap,
            } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_direction = direction;
                    node.flex_wrap = flex_wrap;
                }
            }
            RenderCommand::SetFlexBasis { id, flex_basis } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_basis = flex_basis;
                }
            }
            RenderCommand::SetFlexGrow { id, flex_grow } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_grow = flex_grow;
                }
            }
            RenderCommand::SetFlexShrink { id, flex_shrink } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_shrink = flex_shrink;
                }
            }
            RenderCommand::SetFlex {
                id,
                flex_grow,
                flex_shrink,
                flex_basis,
            } => {
                if let Some(node) = self.style_mut(id) {
                    node.flex_grow = flex_grow;
                    node.flex_shrink = flex_shrink;
                    node.flex_basis = flex_basis;
                }
            }
            RenderCommand::SetJustifyContent {
                id,
                justify_content,
            } => {
                if let Some(node) = self.style_mut(id) {
                    node.justify_content = Some(justify_content);
                }
            }
            RenderCommand::SetAlignItems { id, align_items } => {
                if let Some(node) = self.style_mut(id) {
                    node.align_items = Some(align_items);
                }
            }
            RenderCommand::SetAlignSelf { id, align_self } => {
                if let Some(node) = self.style_mut(id) {
                    node.align_self = Some(align_self);
                }
            }
            RenderCommand::SetAlignContent { id, align_content } => {
                if let Some(node) = self.style_mut(id) {
                    node.align_content = Some(align_content);
                }
            }
            RenderCommand::SetJustifyItems { id, justify_items } => {
                if let Some(node) = self.style_mut(id) {
                    node.justify_items = Some(justify_items);
                }
            }
            RenderCommand::SetJustifySelf { id, justify_self } => {
                if let Some(node) = self.style_mut(id) {
                    node.justify_self = Some(justify_self);
                }
            }
            RenderCommand::SetGap {
                id,
                row_gap,
                column_gap,
            } => {
                if let Some(node) = self.style_mut(id) {
                    node.row_gap = row_gap;
                    node.column_gap = column_gap;
                }
            }
            RenderCommand::SetRowGap { id, row_gap } => {
                if let Some(node) = self.style_mut(id) {
                    node.row_gap = row_gap;
                }
            }
            RenderCommand::SetColumnGap { id, column_gap } => {
                if let Some(node) = self.style_mut(id) {
                    node.column_gap = column_gap;
                }
            }
            RenderCommand::SetWidth { id, width } => {
                if let Some(node) = self.style_mut(id) {
                    node.width = width;
                }
            }
            RenderCommand::SetHeight { id, height } => {
                if let Some(node) = self.style_mut(id) {
                    node.height = height;
                }
            }
            RenderCommand::SetBackground { id, background } => {
                if let Some(node) = self.style_mut(id) {
                    node.background = background;
                }
            }
            RenderCommand::SetSelectionBackground { id, background } => {
                if let Some(node) = self.style_mut(id) {
                    node.selection_background = Some(background);
                }
            }
            RenderCommand::SetGridTemplateColumns { id, tracks } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_template_columns = tracks;
                }
            }
            RenderCommand::SetGridTemplateRows { id, tracks } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_template_rows = tracks;
                }
            }
            RenderCommand::SetGridAutoColumns { id, tracks } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_auto_columns = tracks;
                }
            }
            RenderCommand::SetGridAutoRows { id, tracks } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_auto_rows = tracks;
                }
            }
            RenderCommand::SetGridAutoFlow { id, grid_auto_flow } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_auto_flow = grid_auto_flow;
                }
            }
            RenderCommand::SetGridColumn { id, placement } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_column = placement;
                }
            }
            RenderCommand::SetGridRow { id, placement } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_row = placement;
                }
            }
            RenderCommand::SetGridColumnStart { id, placement } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_column.start = placement;
                }
            }
            RenderCommand::SetGridColumnEnd { id, placement } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_column.end = placement;
                }
            }
            RenderCommand::SetGridRowStart { id, placement } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_row.start = placement;
                }
            }
            RenderCommand::SetGridRowEnd { id, placement } => {
                if let Some(node) = self.style_mut(id) {
                    node.grid_row.end = placement;
                }
            }
            RenderCommand::HitTestClick { click, response } => {
                let _ = response.send(self.hit_test_click(click));
            }
            RenderCommand::HitTestPoint { x, y, response } => {
                let _ = response.send(self.hit_test_id(x, y));
            }
            RenderCommand::HandleTextSelection { event } => {
                self.handle_text_selection(event);
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

    fn children_mut(&mut self, id: u32) -> Option<&mut Vec<u32>> {
        match self.nodes.get_mut(&id)? {
            DomNode::Div(node) => Some(&mut node.children),
            DomNode::Span(node) => Some(&mut node.children),
            DomNode::Text(_) => None,
        }
    }

    fn style_mut(&mut self, id: u32) -> Option<&mut DivStyle> {
        match self.nodes.get_mut(&id)? {
            DomNode::Div(node) => Some(&mut node.style),
            DomNode::Span(node) => Some(&mut node.style),
            DomNode::Text(_) => None,
        }
    }

    fn scroll_offset_mut(&mut self, id: u32) -> Option<(&mut u32, &mut u32)> {
        match self.nodes.get_mut(&id)? {
            DomNode::Div(node) => Some((&mut node.scroll_left, &mut node.scroll_top)),
            DomNode::Span(node) => Some((&mut node.scroll_left, &mut node.scroll_top)),
            DomNode::Text(_) => None,
        }
    }

    fn scroll_offset(&self, id: u32) -> Option<(u32, u32)> {
        match self.nodes.get(&id)? {
            DomNode::Div(node) => Some((node.scroll_left, node.scroll_top)),
            DomNode::Span(node) => Some((node.scroll_left, node.scroll_top)),
            DomNode::Text(_) => None,
        }
    }

    fn set_scroll_offset(
        &mut self,
        id: u32,
        scroll_left: u32,
        scroll_top: u32,
    ) -> Option<ScrollMetrics> {
        let mut metrics = self.scroll_metrics_for(id)?;
        let (max_left, max_top) = self.max_scroll_for_node(id, &metrics)?;
        metrics.scroll_left = scroll_left.min(max_left);
        metrics.scroll_top = scroll_top.min(max_top);

        if let Some((left, top)) = self.scroll_offset_mut(id) {
            *left = metrics.scroll_left;
            *top = metrics.scroll_top;
        }
        self.scroll_metrics.insert(id, metrics.clone());
        Some(metrics)
    }

    fn max_scroll_for_node(&self, id: u32, metrics: &ScrollMetrics) -> Option<(u32, u32)> {
        match self.nodes.get(&id)? {
            DomNode::Div(node) => Some((
                axis_max_scroll(node.style.overflow_x, max_scroll_left(metrics)),
                axis_max_scroll(node.style.overflow_y, max_scroll_top(metrics)),
            )),
            DomNode::Span(node) => Some((
                axis_max_scroll(node.style.overflow_x, max_scroll_left(metrics)),
                axis_max_scroll(node.style.overflow_y, max_scroll_top(metrics)),
            )),
            DomNode::Text(_) => None,
        }
    }

    fn scroll_metrics_for(&self, id: u32) -> Option<ScrollMetrics> {
        self.scroll_metrics
            .get(&id)
            .cloned()
            .or_else(|| self.fallback_scroll_metrics(id))
    }

    fn fallback_scroll_metrics(&self, id: u32) -> Option<ScrollMetrics> {
        let (scroll_left, scroll_top) = self.scroll_offset(id)?;
        Some(ScrollMetrics {
            scroll_left,
            scroll_top,
            scroll_width: 0,
            scroll_height: 0,
            client_width: 0,
            client_height: 0,
        })
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

        let mut scroll_metrics = HashMap::new();
        self.collect_scroll_metrics(root, &taffy, &taffy_ids, &mut scroll_metrics);
        self.clamp_scroll_offsets(&mut scroll_metrics);
        self.scroll_metrics = scroll_metrics;

        let mut frame = Frame::new(cols as usize, rows as usize);
        let mut hit_regions = Vec::new();
        self.paint_node(
            root,
            0.0,
            0.0,
            &taffy,
            &taffy_ids,
            &mut frame,
            &mut hit_regions,
            None,
            ClipBounds::unbounded(),
        );
        self.refresh_active_selection_focus(&frame);
        self.content_frame = Some(frame.clone());
        let mut output_frame = frame;
        output_frame.apply_selection(self.selection.as_ref());
        let _ = output_frame.write_diff_to_stdout(self.previous_frame.as_ref());
        self.previous_frame = Some(output_frame);
        self.hit_regions = hit_regions;
    }

    fn build_taffy(
        &self,
        id: u32,
        taffy: &mut TaffyTree<u32>,
        taffy_ids: &mut HashMap<u32, NodeId>,
    ) -> Option<NodeId> {
        let taffy_id = match self.nodes.get(&id)? {
            DomNode::Div(node) => {
                let children = if self.is_inline_container(node) {
                    Vec::new()
                } else {
                    node.children
                        .iter()
                        .filter_map(|child| self.build_taffy(*child, taffy, taffy_ids))
                        .collect::<Vec<_>>()
                };

                taffy
                    .new_with_children(node.style.to_taffy(), &children)
                    .ok()?
            }
            DomNode::Span(node) => {
                if node.style.display == LayoutDisplay::Inline {
                    taffy.new_leaf(node.style.to_taffy()).ok()?
                } else {
                    let children = if self.is_inline_children(&node.children) {
                        Vec::new()
                    } else {
                        node.children
                            .iter()
                            .filter_map(|child| self.build_taffy(*child, taffy, taffy_ids))
                            .collect::<Vec<_>>()
                    };

                    taffy
                        .new_with_children(node.style.to_taffy(), &children)
                        .ok()?
                }
            }
            DomNode::Text(node) => taffy.new_leaf(node.style()).ok()?,
        };

        taffy_ids.insert(id, taffy_id);
        Some(taffy_id)
    }

    fn collect_scroll_metrics(
        &self,
        id: u32,
        taffy: &TaffyTree<u32>,
        taffy_ids: &HashMap<u32, NodeId>,
        metrics: &mut HashMap<u32, ScrollMetrics>,
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

        let children = match dom_node {
            DomNode::Div(node) => Some(&node.children),
            DomNode::Span(node) => Some(&node.children),
            DomNode::Text(_) => None,
        };
        let Some(children) = children else {
            return;
        };

        let client_width = dimension_to_cells(layout.size.width);
        let client_height = dimension_to_cells(layout.size.height);
        let mut scroll_width = client_width;
        let mut scroll_height = client_height;

        if self.is_inline_children(children) {
            let inline = measure_inline_children(children, client_width.max(1), &self.nodes);
            scroll_width = scroll_width.max(inline.width);
            scroll_height = scroll_height.max(inline.height);
        } else {
            for child in children {
                self.collect_scroll_metrics(*child, taffy, taffy_ids, metrics);

                let Some(child_taffy_id) = taffy_ids.get(child) else {
                    continue;
                };
                let Ok(child_layout) = taffy.layout(*child_taffy_id) else {
                    continue;
                };

                scroll_width = scroll_width.max(edge_to_cells(
                    child_layout.location.x + child_layout.size.width,
                ));
                scroll_height = scroll_height.max(edge_to_cells(
                    child_layout.location.y + child_layout.size.height,
                ));
            }
        }

        let (scroll_left, scroll_top) = self.scroll_offset(id).unwrap_or((0, 0));
        metrics.insert(
            id,
            ScrollMetrics {
                scroll_left,
                scroll_top,
                scroll_width,
                scroll_height,
                client_width,
                client_height,
            },
        );
    }

    fn clamp_scroll_offsets(&mut self, metrics: &mut HashMap<u32, ScrollMetrics>) {
        for (id, metrics) in metrics {
            let Some(node) = self.nodes.get_mut(id) else {
                continue;
            };

            match node {
                DomNode::Div(node) => {
                    let max_left = if node.style.overflow_x == LayoutOverflow::Scroll {
                        max_scroll_left(metrics)
                    } else {
                        0
                    };
                    let max_top = if node.style.overflow_y == LayoutOverflow::Scroll {
                        max_scroll_top(metrics)
                    } else {
                        0
                    };
                    node.scroll_left = node.scroll_left.min(max_left);
                    node.scroll_top = node.scroll_top.min(max_top);
                    metrics.scroll_left = node.scroll_left;
                    metrics.scroll_top = node.scroll_top;
                }
                DomNode::Span(node) => {
                    let max_left = if node.style.overflow_x == LayoutOverflow::Scroll {
                        max_scroll_left(metrics)
                    } else {
                        0
                    };
                    let max_top = if node.style.overflow_y == LayoutOverflow::Scroll {
                        max_scroll_top(metrics)
                    } else {
                        0
                    };
                    node.scroll_left = node.scroll_left.min(max_left);
                    node.scroll_top = node.scroll_top.min(max_top);
                    metrics.scroll_left = node.scroll_left;
                    metrics.scroll_top = node.scroll_top;
                }
                DomNode::Text(_) => {}
            }
        }
    }

    fn paint_node(
        &self,
        id: u32,
        parent_x: f32,
        parent_y: f32,
        taffy: &TaffyTree<u32>,
        taffy_ids: &HashMap<u32, NodeId>,
        frame: &mut Frame,
        hit_regions: &mut Vec<HitRegion>,
        selection_background: Option<Background>,
        clip: ClipBounds,
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
        let bounds = ClipRect::new(
            x.round() as i32,
            y.round() as i32,
            layout.size.width.round() as i32,
            layout.size.height.round() as i32,
        );

        match dom_node {
            DomNode::Div(node) => {
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(
                    x.round() as i32,
                    y.round() as i32,
                    layout.size.width.round() as i32,
                    layout.size.height.round() as i32,
                    node.style.background,
                    selection_background,
                    clip,
                );

                let child_clip =
                    child_clip_for(node.style.overflow_x, node.style.overflow_y, bounds, clip);
                let child_x = x - scroll_offset(node.style.overflow_x, node.scroll_left);
                let child_y = y - scroll_offset(node.style.overflow_y, node.scroll_top);
                if self.is_inline_container(node) {
                    self.paint_inline_children(
                        &node.children,
                        child_x.round() as i32,
                        child_y.round() as i32,
                        layout.size.width.round() as i32,
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        child_clip,
                    );
                } else {
                    for child in &node.children {
                        self.paint_node(
                            *child,
                            child_x,
                            child_y,
                            taffy,
                            taffy_ids,
                            frame,
                            hit_regions,
                            selection_background,
                            child_clip,
                        );
                    }
                }
            }
            DomNode::Span(node) => {
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(
                    x.round() as i32,
                    y.round() as i32,
                    layout.size.width.round() as i32,
                    layout.size.height.round() as i32,
                    node.style.background,
                    selection_background,
                    clip,
                );
                let child_clip =
                    child_clip_for(node.style.overflow_x, node.style.overflow_y, bounds, clip);
                let child_x = x - scroll_offset(node.style.overflow_x, node.scroll_left);
                let child_y = y - scroll_offset(node.style.overflow_y, node.scroll_top);
                if self.is_inline_children(&node.children) {
                    self.paint_inline_children(
                        &node.children,
                        child_x.round() as i32,
                        child_y.round() as i32,
                        layout.size.width.round() as i32,
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        child_clip,
                    );
                } else {
                    for child in &node.children {
                        self.paint_node(
                            *child,
                            child_x,
                            child_y,
                            taffy,
                            taffy_ids,
                            frame,
                            hit_regions,
                            selection_background,
                            child_clip,
                        );
                    }
                }
            }
            DomNode::Text(node) => {
                frame.write_text(
                    x.round() as i32,
                    y.round() as i32,
                    &node.text,
                    selection_background,
                    clip,
                );
            }
        }
    }

    fn is_inline_container(&self, node: &DivNode) -> bool {
        self.is_inline_children(&node.children)
    }

    fn is_inline_children(&self, children: &[u32]) -> bool {
        let mut has_inline_element = false;
        for child in children {
            match self.nodes.get(child) {
                Some(DomNode::Text(_)) => {}
                Some(DomNode::Div(node)) if node.style.display == LayoutDisplay::Inline => {
                    has_inline_element = true;
                }
                Some(DomNode::Span(node)) if node.style.display == LayoutDisplay::Inline => {
                    has_inline_element = true;
                }
                _ => return false,
            }
        }
        has_inline_element
    }

    fn paint_inline_children(
        &self,
        children: &[u32],
        x: i32,
        y: i32,
        width: i32,
        frame: &mut Frame,
        hit_regions: &mut Vec<HitRegion>,
        hit_target: Option<u32>,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        let mut cursor = InlineCursor {
            x,
            y,
            col: 0,
            row: 0,
            width: width.max(1),
        };

        for child in children {
            self.paint_inline_node(
                *child,
                &mut cursor,
                Background::Default,
                frame,
                hit_regions,
                hit_target,
                selection_background,
                clip,
            );
        }
    }

    fn paint_inline_node(
        &self,
        id: u32,
        cursor: &mut InlineCursor,
        background: Background,
        frame: &mut Frame,
        hit_regions: &mut Vec<HitRegion>,
        hit_target: Option<u32>,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        match self.nodes.get(&id) {
            Some(DomNode::Text(node)) => {
                write_inline_text(
                    &node.text,
                    cursor,
                    background,
                    frame,
                    hit_regions,
                    hit_target,
                    selection_background,
                    clip,
                );
            }
            Some(DomNode::Span(node)) => {
                let background = if node.style.background == Background::Default {
                    background
                } else {
                    node.style.background
                };
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );

                for child in &node.children {
                    self.paint_inline_node(
                        *child,
                        cursor,
                        background,
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        clip,
                    );
                }
            }
            Some(DomNode::Div(node)) if node.style.display == LayoutDisplay::Inline => {
                let background = if node.style.background == Background::Default {
                    background
                } else {
                    node.style.background
                };
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );

                for child in &node.children {
                    self.paint_inline_node(
                        *child,
                        cursor,
                        background,
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        clip,
                    );
                }
            }
            Some(DomNode::Div(_)) | None => {}
        }
    }

    fn hit_test_click(&self, click: MouseClick) -> Option<ClickEvent> {
        let target_id = self.hit_test_id(click.x, click.y)?;

        Some(ClickEvent {
            r#type: "click".to_string(),
            target_id,
            client_x: click.x,
            client_y: click.y,
            button: click.button,
            ctrl_key: click.ctrl_key,
            alt_key: click.alt_key,
            meta_key: click.meta_key,
            shift_key: click.shift_key,
        })
    }

    fn hit_test_id(&self, x: u32, y: u32) -> Option<u32> {
        self.hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(x as i32, y as i32))
            .map(|region| region.id)
    }

    fn handle_text_selection(&mut self, event: SelectionMouseEvent) {
        let Some(frame) = self.content_frame.as_ref() else {
            return;
        };

        match event.event_type {
            SelectionMouseEventType::Down => {
                let Some(point) = frame.selection_point_for(event.x, event.y) else {
                    self.selection = None;
                    self.redraw_selection();
                    return;
                };
                self.selection = Some(Selection {
                    anchor: point,
                    focus: point,
                    selecting: true,
                    moved: false,
                    last_x: event.x,
                    last_y: event.y,
                });
                self.redraw_selection();
            }
            SelectionMouseEventType::Drag => {
                let Some(point) = self
                    .content_frame
                    .as_ref()
                    .and_then(|frame| frame.selection_point_for(event.x, event.y))
                else {
                    return;
                };
                if let Some(selection) = self
                    .selection
                    .as_mut()
                    .filter(|selection| selection.selecting && event.button == 0)
                {
                    selection.moved = selection.moved || selection.focus != point;
                    selection.focus = point;
                    selection.last_x = event.x;
                    selection.last_y = event.y;
                    self.redraw_selection();
                }
            }
            SelectionMouseEventType::Scroll => {
                if let Some(selection) = self
                    .selection
                    .as_mut()
                    .filter(|selection| selection.selecting)
                {
                    selection.last_x = event.x;
                    selection.last_y = event.y;
                    if let Some(point) = self
                        .content_frame
                        .as_ref()
                        .and_then(|frame| frame.selection_point_for(event.x, event.y))
                    {
                        selection.moved = selection.moved || selection.focus != point;
                        selection.focus = point;
                    }
                    self.redraw_selection();
                }
            }
            SelectionMouseEventType::Up => {
                let point = self
                    .content_frame
                    .as_ref()
                    .and_then(|frame| frame.selection_point_for(event.x, event.y));
                let mut should_copy = false;
                if let Some(selection) = self
                    .selection
                    .as_mut()
                    .filter(|selection| selection.selecting && event.button == 0)
                {
                    if let Some(point) = point {
                        selection.moved = selection.moved || selection.focus != point;
                        selection.focus = point;
                    }
                    selection.last_x = event.x;
                    selection.last_y = event.y;
                    selection.selecting = false;
                    should_copy = selection.moved;
                }

                if should_copy {
                    let selected_text = self.selection.as_ref().and_then(|selection| {
                        self.content_frame
                            .as_ref()
                            .and_then(|frame| frame.selected_text(selection))
                    });
                    if let Some(text) = selected_text {
                        copy_text_to_clipboard(&text);
                    }
                    self.selection = None;
                } else {
                    self.selection = None;
                }

                self.redraw_selection();
            }
        }
    }

    fn redraw_selection(&mut self) {
        let Some(content_frame) = self.content_frame.as_ref() else {
            return;
        };

        let mut output_frame = content_frame.clone();
        output_frame.apply_selection(self.selection.as_ref());
        let _ = output_frame.write_diff_to_stdout(self.previous_frame.as_ref());
        self.previous_frame = Some(output_frame);
    }

    fn refresh_active_selection_focus(&mut self, frame: &Frame) {
        if let Some(selection) = self
            .selection
            .as_mut()
            .filter(|selection| selection.selecting)
        {
            if let Some(point) = frame.selection_point_for(selection.last_x, selection.last_y) {
                selection.moved = selection.moved || selection.focus != point;
                selection.focus = point;
            }
        }
    }
}

struct HitRegion {
    id: u32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

struct Selection {
    anchor: SelectionPoint,
    focus: SelectionPoint,
    selecting: bool,
    moved: bool,
    last_x: u32,
    last_y: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SelectionPoint {
    order: usize,
}

fn normalized_selection(selection: &Selection) -> (SelectionPoint, SelectionPoint) {
    if selection.anchor.order <= selection.focus.order {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}

#[derive(Clone, Copy)]
struct ClipRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Clone, Copy)]
struct ClipBounds {
    left: Option<i32>,
    top: Option<i32>,
    right: Option<i32>,
    bottom: Option<i32>,
}

impl ClipRect {
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width.max(0),
            bottom: y + height.max(0),
        }
    }
}

impl ClipBounds {
    fn unbounded() -> Self {
        Self {
            left: None,
            top: None,
            right: None,
            bottom: None,
        }
    }

    fn from_rect_axes(rect: ClipRect, clip_x: bool, clip_y: bool) -> Self {
        Self {
            left: clip_x.then_some(rect.left),
            right: clip_x.then_some(rect.right),
            top: clip_y.then_some(rect.top),
            bottom: clip_y.then_some(rect.bottom),
        }
    }

    fn intersect(self, other: Self) -> Self {
        Self {
            left: max_option(self.left, other.left),
            top: max_option(self.top, other.top),
            right: min_option(self.right, other.right),
            bottom: min_option(self.bottom, other.bottom),
        }
    }

    fn clip_rect(self, rect: ClipRect) -> Option<ClipRect> {
        let clipped = ClipRect {
            left: self.left.map_or(rect.left, |left| rect.left.max(left)),
            top: self.top.map_or(rect.top, |top| rect.top.max(top)),
            right: self.right.map_or(rect.right, |right| rect.right.min(right)),
            bottom: self
                .bottom
                .map_or(rect.bottom, |bottom| rect.bottom.min(bottom)),
        };

        if clipped.left < clipped.right && clipped.top < clipped.bottom {
            Some(clipped)
        } else {
            None
        }
    }

    fn contains(self, x: i32, y: i32) -> bool {
        self.left.is_none_or(|left| x >= left)
            && self.right.is_none_or(|right| x < right)
            && self.top.is_none_or(|top| y >= top)
            && self.bottom.is_none_or(|bottom| y < bottom)
    }
}

fn max_option(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn min_option(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

impl HitRegion {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

fn push_hit_region(hit_regions: &mut Vec<HitRegion>, id: u32, bounds: ClipRect, clip: ClipBounds) {
    let Some(bounds) = clip.clip_rect(bounds) else {
        return;
    };

    hit_regions.push(HitRegion {
        id,
        left: bounds.left,
        top: bounds.top,
        right: bounds.right,
        bottom: bounds.bottom,
    });
}

fn child_clip_for(
    overflow_x: LayoutOverflow,
    overflow_y: LayoutOverflow,
    bounds: ClipRect,
    clip: ClipBounds,
) -> ClipBounds {
    let clips_x = overflow_x != LayoutOverflow::Visible;
    let clips_y = overflow_y != LayoutOverflow::Visible;
    clip.intersect(ClipBounds::from_rect_axes(bounds, clips_x, clips_y))
}

fn scroll_offset(overflow: LayoutOverflow, value: u32) -> f32 {
    if overflow == LayoutOverflow::Scroll {
        value as f32
    } else {
        0.0
    }
}

fn effective_selection_background(
    own: Option<Background>,
    inherited: Option<Background>,
) -> Option<Background> {
    own.filter(|background| *background != Background::Default)
        .or(inherited)
}

fn axis_max_scroll(overflow: LayoutOverflow, value: u32) -> u32 {
    if overflow == LayoutOverflow::Scroll {
        value
    } else {
        0
    }
}

fn max_scroll_left(metrics: &ScrollMetrics) -> u32 {
    metrics.scroll_width.saturating_sub(metrics.client_width)
}

fn max_scroll_top(metrics: &ScrollMetrics) -> u32 {
    metrics.scroll_height.saturating_sub(metrics.client_height)
}

fn dimension_to_cells(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

fn edge_to_cells(value: f32) -> u32 {
    value.max(0.0).ceil() as u32
}

struct InlineCursor {
    x: i32,
    y: i32,
    col: i32,
    row: i32,
    width: i32,
}

fn write_inline_text(
    text: &str,
    cursor: &mut InlineCursor,
    background: Background,
    frame: &mut Frame,
    hit_regions: &mut Vec<HitRegion>,
    hit_target: Option<u32>,
    selection_background: Option<Background>,
    clip: ClipBounds,
) {
    for character in text.chars() {
        if character == '\n' {
            cursor.col = 0;
            cursor.row += 1;
            continue;
        }

        if cursor.col >= cursor.width {
            cursor.col = 0;
            cursor.row += 1;
        }

        let x = cursor.x + cursor.col;
        let y = cursor.y + cursor.row;
        frame.write_char(x, y, character, background, selection_background, clip);
        if let Some(hit_target) = hit_target {
            push_hit_region(hit_regions, hit_target, ClipRect::new(x, y, 1, 1), clip);
        }
        cursor.col += 1;
    }
}

#[derive(Clone)]
struct Frame {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    selection_units: Vec<SelectionUnit>,
    next_selection_order: usize,
}

impl Frame {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
            selection_units: Vec::new(),
            next_selection_order: 0,
        }
    }

    fn fill_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        background: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        if (background == Background::Default && selection_background.is_none())
            || width <= 0
            || height <= 0
        {
            return;
        }

        let Some(bounds) = clip.clip_rect(ClipRect::new(x, y, width, height)) else {
            return;
        };

        let left = bounds.left.max(0) as usize;
        let top = bounds.top.max(0) as usize;
        let right = bounds.right.min(self.width as i32).max(0) as usize;
        let bottom = bounds.bottom.min(self.height as i32).max(0) as usize;

        for row in top..bottom {
            let start = row * self.width;
            for col in left..right {
                self.cells[start + col] = Cell {
                    background,
                    character: ' ',
                    selection_background,
                    selection_order: None,
                    reversed: false,
                };
            }
        }
    }

    fn write_text(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        for (line_offset, line) in text.lines().enumerate() {
            let row = y + line_offset as i32;
            let mut col = x;
            for character in line.chars() {
                let selection_order = self.push_selection_unit(row, character);

                if row >= 0
                    && row < self.height as i32
                    && col >= 0
                    && col < self.width as i32
                    && clip.contains(col, row)
                {
                    let index = row as usize * self.width + col as usize;
                    self.cells[index].character = character;
                    self.cells[index].selection_order = Some(selection_order);
                    if selection_background.is_some() {
                        self.cells[index].selection_background = selection_background;
                    }
                }
                col += 1;
            }
        }
    }

    fn write_char(
        &mut self,
        x: i32,
        y: i32,
        character: char,
        background: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        let selection_order = self.push_selection_unit(y, character);

        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if !clip.contains(x, y) {
            return;
        }

        let index = y as usize * self.width + x as usize;
        self.cells[index].character = character;
        self.cells[index].selection_order = Some(selection_order);
        if background != Background::Default {
            self.cells[index].background = background;
        }
        if selection_background.is_some() {
            self.cells[index].selection_background = selection_background;
        }
    }

    fn write_diff_to_stdout(&self, previous: Option<&Frame>) -> io::Result<()> {
        let mut out = io::stdout().lock();
        write_synchronized_output_begin(&mut out)?;
        let result: io::Result<()> = (|| {
            write!(out, "\x1b[?25l")?;

            let Some(previous) = previous else {
                self.write_full_to(&mut out)?;
                return Ok(());
            };

            if previous.width != self.width || previous.height != self.height {
                write!(out, "\x1b[2J")?;
                self.write_full_to(&mut out)?;
                return Ok(());
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

            Ok(())
        })();

        let end_result = write_synchronized_output_end(&mut out);
        let flush_result = out.flush();

        result?;
        end_result?;
        flush_result
    }

    fn apply_selection(&mut self, selection: Option<&Selection>) {
        let Some(selection) = selection else {
            return;
        };

        let (start, end) = normalized_selection(selection);
        for cell in &mut self.cells {
            if cell
                .selection_order
                .is_some_and(|order| order >= start.order && order <= end.order)
            {
                if let Some(background) = cell.selection_background {
                    cell.background = background;
                    cell.reversed = false;
                } else {
                    cell.reversed = true;
                }
            }
        }
    }

    fn selected_text(&self, selection: &Selection) -> Option<String> {
        let (start, end) = normalized_selection(selection);
        let mut lines = Vec::new();
        let mut current_row = None;
        let mut current_line = String::new();

        for unit in &self.selection_units {
            if unit.order < start.order || unit.order > end.order {
                continue;
            }

            match current_row {
                Some(row) if row == unit.row => {}
                Some(_) => {
                    lines.push(current_line.trim_end().to_string());
                    current_line.clear();
                    current_row = Some(unit.row);
                }
                None => current_row = Some(unit.row),
            }

            current_line.push(unit.character);
        }

        if current_row.is_some() {
            lines.push(current_line.trim_end().to_string());
        }

        let text = lines.join("\n");
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn selection_point_for(&self, x: u32, y: u32) -> Option<SelectionPoint> {
        if self.width == 0 || self.height == 0 {
            return None;
        }

        let row = (y as usize).min(self.height - 1);
        let col = (x as usize).min(self.width - 1);
        let row_start = row * self.width;

        if let Some(order) = self.cells[row_start + col].selection_order {
            return Some(SelectionPoint { order });
        }

        let selectable_cols = (0..self.width)
            .filter_map(|candidate_col| {
                self.cells[row_start + candidate_col]
                    .selection_order
                    .map(|order| (candidate_col, order))
            })
            .collect::<Vec<_>>();

        let (first_col, first_order) = *selectable_cols.first()?;
        let (last_col, last_order) = *selectable_cols.last()?;

        if col <= first_col {
            return Some(SelectionPoint { order: first_order });
        }
        if col >= last_col {
            return Some(SelectionPoint { order: last_order });
        }

        selectable_cols
            .into_iter()
            .min_by_key(|(candidate_col, _)| candidate_col.abs_diff(col))
            .map(|(_, order)| SelectionPoint { order })
    }

    fn push_selection_unit(&mut self, row: i32, character: char) -> usize {
        let order = self.next_selection_order;
        self.next_selection_order += 1;
        self.selection_units.push(SelectionUnit {
            order,
            row,
            character,
        });
        order
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

        let mut current_background = Background::Default;
        let mut current_reversed = false;
        for col in start_col..end_col {
            let cell = self.cells[row * self.width + col];
            let background = cell.background;
            if cell.reversed != current_reversed {
                if cell.reversed {
                    write!(out, "\x1b[7m")?;
                } else {
                    write!(out, "\x1b[27m")?;
                }
                current_reversed = cell.reversed;
            }
            if background != current_background {
                write!(out, "{}", background.ansi_bg())?;
                current_background = background;
            }
            write!(out, "{}", cell.character)?;
        }

        write!(out, "\x1b[27m\x1b[49m")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cell {
    background: Background,
    character: char,
    selection_background: Option<Background>,
    selection_order: Option<usize>,
    reversed: bool,
}

#[derive(Clone, Copy)]
struct SelectionUnit {
    order: usize,
    row: i32,
    character: char,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            background: Background::Default,
            character: ' ',
            selection_background: None,
            selection_order: None,
            reversed: false,
        }
    }
}

struct TextMetrics {
    width: usize,
    height: usize,
}

struct InlineMetrics {
    width: u32,
    height: u32,
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

fn measure_inline_children(
    children: &[u32],
    width: u32,
    nodes: &HashMap<u32, DomNode>,
) -> InlineMetrics {
    let mut cursor = InlineMeasureCursor {
        col: 0,
        row: 0,
        width: width.max(1),
        max_col: 0,
    };

    for child in children {
        measure_inline_node(*child, nodes, &mut cursor);
    }

    InlineMetrics {
        width: cursor.max_col.max(cursor.col).max(1),
        height: cursor.row + 1,
    }
}

struct InlineMeasureCursor {
    col: u32,
    row: u32,
    width: u32,
    max_col: u32,
}

fn measure_inline_node(id: u32, nodes: &HashMap<u32, DomNode>, cursor: &mut InlineMeasureCursor) {
    match nodes.get(&id) {
        Some(DomNode::Text(node)) => measure_inline_text(&node.text, cursor),
        Some(DomNode::Span(node)) if node.style.display == LayoutDisplay::Inline => {
            for child in &node.children {
                measure_inline_node(*child, nodes, cursor);
            }
        }
        Some(DomNode::Div(node)) if node.style.display == LayoutDisplay::Inline => {
            for child in &node.children {
                measure_inline_node(*child, nodes, cursor);
            }
        }
        _ => {}
    }
}

fn measure_inline_text(text: &str, cursor: &mut InlineMeasureCursor) {
    for character in text.chars() {
        if character == '\n' {
            cursor.max_col = cursor.max_col.max(cursor.col);
            cursor.col = 0;
            cursor.row += 1;
            continue;
        }

        if cursor.col >= cursor.width {
            cursor.max_col = cursor.max_col.max(cursor.col);
            cursor.col = 0;
            cursor.row += 1;
        }

        cursor.col += 1;
        cursor.max_col = cursor.max_col.max(cursor.col);
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
