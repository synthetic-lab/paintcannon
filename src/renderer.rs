use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Cursor;
use std::io::{self, Write};
use std::ops::Range;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender as StdSender,
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::{Receiver, Sender};
use napi_derive::napi;
use taffy::geometry::Point;
use taffy::prelude::*;
use termprofile::{DetectorSettings, TermProfile};
use unicode_width::UnicodeWidthChar;

use crate::style::*;
use crate::terminal::{
    copy_text_to_clipboard, query_terminal_size, reset_terminal, write_pointer_shape,
    write_synchronized_output_begin, write_synchronized_output_end, TerminalSize,
};

pub(crate) enum RenderCommand {
    Batch {
        commands: Vec<RenderCommand>,
    },
    CreateDiv {
        id: u32,
    },
    CreateSpan {
        id: u32,
    },
    CreateImage {
        id: u32,
    },
    CreateInput {
        id: u32,
    },
    CreateTextArea {
        id: u32,
    },
    CreateText {
        id: u32,
        text: String,
    },
    SetImageSource {
        id: u32,
        src: String,
    },
    SetInputValue {
        id: u32,
        value: String,
        cursor: u32,
    },
    SetInputFocused {
        id: u32,
        focused: bool,
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
    SetImageRendering {
        id: u32,
        image_rendering: ImageRendering,
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
    SetMinHeight {
        id: u32,
        min_height: CssDimension,
    },
    SetBorder {
        id: u32,
        style: BorderStyle,
    },
    SetBorderTop {
        id: u32,
        style: BorderStyle,
    },
    SetBorderRight {
        id: u32,
        style: BorderStyle,
    },
    SetBorderBottom {
        id: u32,
        style: BorderStyle,
    },
    SetBorderLeft {
        id: u32,
        style: BorderStyle,
    },
    SetBorderColor {
        id: u32,
        color: Background,
    },
    SetColor {
        id: u32,
        color: Background,
    },
    SetTransition {
        id: u32,
        transitions: Vec<TransitionSpec>,
    },
    SetBackground {
        id: u32,
        background: Background,
    },
    SetSelectionBackground {
        id: u32,
        background: Background,
    },
    SetCursor {
        id: u32,
        cursor: CursorStyle,
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
    HandlePointerMove {
        x: u32,
        y: u32,
    },
    InvalidateFrame,
    Render {
        pending: Arc<AtomicBool>,
    },
    RenderSync {
        response: StdSender<()>,
    },
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
    Image(ImageNode),
    Input(InputNode),
    TextArea(TextAreaNode),
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
struct ImageNode {
    style: DivStyle,
    src: Option<String>,
    image: Option<ImageData>,
}

impl Default for ImageNode {
    fn default() -> Self {
        Self {
            style: DivStyle::default(),
            src: None,
            image: None,
        }
    }
}

#[derive(Clone)]
struct ImageData {
    width_px: u32,
    height_px: u32,
    rgb: Vec<u8>,
}

#[derive(Clone)]
struct InputNode {
    style: DivStyle,
    value: String,
    cursor: u32,
    focused: bool,
}

impl Default for InputNode {
    fn default() -> Self {
        Self {
            style: DivStyle::default(),
            value: String::new(),
            cursor: 0,
            focused: false,
        }
    }
}

#[derive(Clone)]
struct TextAreaNode {
    style: DivStyle,
    value: String,
    cursor: u32,
    focused: bool,
}

impl Default for TextAreaNode {
    fn default() -> Self {
        Self {
            style: DivStyle::default(),
            value: String::new(),
            cursor: 0,
            focused: false,
        }
    }
}

#[derive(Clone)]
struct TextNode {
    text: String,
    metrics: TextMetrics,
}

impl TextNode {
    fn style(&self) -> Style {
        let TextMetrics { width, height } = self.metrics;

        Style {
            size: Size {
                width: Dimension::length(width as f32),
                height: Dimension::length(height as f32),
            },
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TransitionKey {
    id: u32,
    property: ColorTransitionProperty,
}

#[derive(Clone, Copy)]
struct ActiveColorTransition {
    from: Background,
    to: Background,
    started_at: Instant,
    duration: Duration,
}

struct Renderer {
    root: Option<u32>,
    nodes: HashMap<u32, DomNode>,
    taffy: TaffyTree<u32>,
    taffy_ids: HashMap<u32, NodeId>,
    parent_by_child: HashMap<u32, u32>,
    previous_frame: Option<Frame>,
    content_frame: Option<Frame>,
    hit_regions: Vec<HitRegion>,
    scroll_metrics: HashMap<u32, ScrollMetrics>,
    scroll_metrics_dirty: bool,
    inline_metrics_cache: HashMap<InlineMetricsKey, InlineMetrics>,
    last_layout_size: Option<(u32, u32)>,
    transition_specs: HashMap<u32, HashMap<ColorTransitionProperty, Duration>>,
    active_transitions: HashMap<TransitionKey, ActiveColorTransition>,
    transition_events: Arc<Mutex<VecDeque<TransitionEvent>>>,
    color_profile: TermProfile,
    current_pointer_shape: Option<&'static str>,
    last_pointer_position: Option<(u32, u32)>,
    selection: Option<Selection>,
}

impl Renderer {
    fn new(transition_events: Arc<Mutex<VecDeque<TransitionEvent>>>) -> Self {
        let mut taffy = TaffyTree::new();
        taffy.disable_rounding();
        termprofile::set_color_cache_enabled(true);
        let color_profile = TermProfile::detect(&io::stdout(), DetectorSettings::default());

        Self {
            root: None,
            nodes: HashMap::new(),
            taffy,
            taffy_ids: HashMap::new(),
            parent_by_child: HashMap::new(),
            previous_frame: None,
            content_frame: None,
            hit_regions: Vec::new(),
            scroll_metrics: HashMap::new(),
            scroll_metrics_dirty: true,
            inline_metrics_cache: HashMap::new(),
            last_layout_size: None,
            transition_specs: HashMap::new(),
            active_transitions: HashMap::new(),
            transition_events,
            color_profile,
            current_pointer_shape: None,
            last_pointer_position: None,
            selection: None,
        }
    }

    fn apply(&mut self, command: RenderCommand) -> bool {
        match command {
            RenderCommand::Batch { commands } => {
                for command in commands {
                    if !self.apply(command) {
                        return false;
                    }
                }
            }
            RenderCommand::CreateDiv { id } => {
                let node = DivNode::default();
                if let Ok(taffy_id) = self.taffy.new_leaf(node.style.to_taffy()) {
                    self.taffy_ids.insert(id, taffy_id);
                }
                self.nodes.insert(id, DomNode::Div(node));
            }
            RenderCommand::CreateSpan { id } => {
                let node = SpanNode::default();
                if let Ok(taffy_id) = self.taffy.new_leaf(node.style.to_taffy()) {
                    self.taffy_ids.insert(id, taffy_id);
                }
                self.nodes.insert(id, DomNode::Span(node));
            }
            RenderCommand::CreateImage { id } => {
                let node = ImageNode::default();
                if let Ok(taffy_id) = self.taffy.new_leaf(node.style.to_taffy()) {
                    self.taffy_ids.insert(id, taffy_id);
                }
                self.nodes.insert(id, DomNode::Image(node));
            }
            RenderCommand::CreateInput { id } => {
                let node = InputNode::default();
                if let Ok(taffy_id) = self.taffy.new_leaf(input_taffy_style(&node)) {
                    self.taffy_ids.insert(id, taffy_id);
                }
                self.nodes.insert(id, DomNode::Input(node));
            }
            RenderCommand::CreateTextArea { id } => {
                let node = TextAreaNode::default();
                if let Ok(taffy_id) = self.taffy.new_leaf(textarea_taffy_style(&node)) {
                    self.taffy_ids.insert(id, taffy_id);
                }
                self.nodes.insert(id, DomNode::TextArea(node));
            }
            RenderCommand::CreateText { id, text } => {
                let node = TextNode {
                    metrics: measure_text(&text),
                    text,
                };
                if let Ok(taffy_id) = self.taffy.new_leaf(node.style()) {
                    self.taffy_ids.insert(id, taffy_id);
                }
                self.nodes.insert(id, DomNode::Text(node));
            }
            RenderCommand::SetText { id, text } => {
                self.set_text(id, text);
            }
            RenderCommand::SetImageSource { id, src } => {
                self.set_image_source(id, src);
            }
            RenderCommand::SetInputValue { id, value, cursor } => {
                self.set_input_value(id, value, cursor);
            }
            RenderCommand::SetInputFocused { id, focused } => {
                self.set_input_focused(id, focused);
            }
            RenderCommand::SetRoot { id } => {
                self.root = Some(id);
                self.mark_layout_dirty();
            }
            RenderCommand::AppendChild {
                parent: parent_id,
                child,
            } => {
                if self.nodes.contains_key(&child) {
                    if let Some(children) = self.children_mut(parent_id) {
                        children.push(child);
                    }
                    self.parent_by_child.insert(child, parent_id);
                    self.sync_appended_taffy_child(parent_id, child);
                    self.mark_layout_dirty();
                }
            }
            RenderCommand::SetDisplay { id, display } => {
                self.update_style(id, |node| node.display = display);
            }
            RenderCommand::SetOverflow { id, overflow } => {
                self.update_style(id, |node| {
                    node.overflow_x = overflow;
                    node.overflow_y = overflow;
                });
            }
            RenderCommand::SetOverflowX { id, overflow } => {
                self.update_style(id, |node| node.overflow_x = overflow);
            }
            RenderCommand::SetOverflowY { id, overflow } => {
                self.update_style(id, |node| node.overflow_y = overflow);
            }
            RenderCommand::SetImageRendering {
                id,
                image_rendering,
            } => {
                self.update_visual_style(id, |node| node.image_rendering = image_rendering);
            }
            RenderCommand::SetScrollOffset {
                id,
                scroll_left,
                scroll_top,
                response,
            } => {
                self.ensure_layout_and_scroll_metrics();
                let metrics = self.set_scroll_offset(id, scroll_left, scroll_top);
                let _ = response.send(metrics);
            }
            RenderCommand::GetScrollMetrics { id, response } => {
                self.ensure_layout_and_scroll_metrics();
                let _ = response.send(self.scroll_metrics_for(id));
            }
            RenderCommand::SetFlexDirection { id, direction } => {
                self.update_style(id, |node| node.flex_direction = direction);
            }
            RenderCommand::SetFlexWrap { id, flex_wrap } => {
                self.update_style(id, |node| node.flex_wrap = flex_wrap);
            }
            RenderCommand::SetFlexFlow {
                id,
                direction,
                flex_wrap,
            } => {
                self.update_style(id, |node| {
                    node.flex_direction = direction;
                    node.flex_wrap = flex_wrap;
                });
            }
            RenderCommand::SetFlexBasis { id, flex_basis } => {
                self.update_style(id, |node| node.flex_basis = flex_basis);
            }
            RenderCommand::SetFlexGrow { id, flex_grow } => {
                self.update_style(id, |node| node.flex_grow = flex_grow);
            }
            RenderCommand::SetFlexShrink { id, flex_shrink } => {
                self.update_style(id, |node| node.flex_shrink = flex_shrink);
            }
            RenderCommand::SetFlex {
                id,
                flex_grow,
                flex_shrink,
                flex_basis,
            } => {
                self.update_style(id, |node| {
                    node.flex_grow = flex_grow;
                    node.flex_shrink = flex_shrink;
                    node.flex_basis = flex_basis;
                });
            }
            RenderCommand::SetJustifyContent {
                id,
                justify_content,
            } => {
                self.update_style(id, |node| node.justify_content = Some(justify_content));
            }
            RenderCommand::SetAlignItems { id, align_items } => {
                self.update_style(id, |node| node.align_items = Some(align_items));
            }
            RenderCommand::SetAlignSelf { id, align_self } => {
                self.update_style(id, |node| node.align_self = Some(align_self));
            }
            RenderCommand::SetAlignContent { id, align_content } => {
                self.update_style(id, |node| node.align_content = Some(align_content));
            }
            RenderCommand::SetJustifyItems { id, justify_items } => {
                self.update_style(id, |node| node.justify_items = Some(justify_items));
            }
            RenderCommand::SetJustifySelf { id, justify_self } => {
                self.update_style(id, |node| node.justify_self = Some(justify_self));
            }
            RenderCommand::SetGap {
                id,
                row_gap,
                column_gap,
            } => {
                self.update_style(id, |node| {
                    node.row_gap = row_gap;
                    node.column_gap = column_gap;
                });
            }
            RenderCommand::SetRowGap { id, row_gap } => {
                self.update_style(id, |node| node.row_gap = row_gap);
            }
            RenderCommand::SetColumnGap { id, column_gap } => {
                self.update_style(id, |node| node.column_gap = column_gap);
            }
            RenderCommand::SetWidth { id, width } => {
                self.update_style(id, |node| node.width = width);
            }
            RenderCommand::SetHeight { id, height } => {
                self.update_style(id, |node| node.height = height);
            }
            RenderCommand::SetMinHeight { id, min_height } => {
                self.update_style(id, |node| node.min_height = min_height);
            }
            RenderCommand::SetBorder { id, style } => {
                self.update_style(id, |node| {
                    node.border_top = style;
                    node.border_right = style;
                    node.border_bottom = style;
                    node.border_left = style;
                });
            }
            RenderCommand::SetBorderTop { id, style } => {
                self.update_style(id, |node| node.border_top = style);
            }
            RenderCommand::SetBorderRight { id, style } => {
                self.update_style(id, |node| node.border_right = style);
            }
            RenderCommand::SetBorderBottom { id, style } => {
                self.update_style(id, |node| node.border_bottom = style);
            }
            RenderCommand::SetBorderLeft { id, style } => {
                self.update_style(id, |node| node.border_left = style);
            }
            RenderCommand::SetBorderColor { id, color } => {
                self.set_paint_color(id, ColorTransitionProperty::BorderColor, color);
            }
            RenderCommand::SetColor { id, color } => {
                self.set_paint_color(id, ColorTransitionProperty::Color, color);
            }
            RenderCommand::SetTransition { id, transitions } => {
                self.set_transition(id, transitions);
            }
            RenderCommand::SetBackground { id, background } => {
                self.set_paint_color(id, ColorTransitionProperty::BackgroundColor, background);
            }
            RenderCommand::SetSelectionBackground { id, background } => {
                self.update_visual_style(id, |node| node.selection_background = Some(background));
            }
            RenderCommand::SetCursor { id, cursor } => {
                self.update_visual_style(id, |node| node.cursor = cursor);
                self.refresh_pointer_shape();
            }
            RenderCommand::SetGridTemplateColumns { id, tracks } => {
                self.update_style(id, |node| node.grid_template_columns = tracks);
            }
            RenderCommand::SetGridTemplateRows { id, tracks } => {
                self.update_style(id, |node| node.grid_template_rows = tracks);
            }
            RenderCommand::SetGridAutoColumns { id, tracks } => {
                self.update_style(id, |node| node.grid_auto_columns = tracks);
            }
            RenderCommand::SetGridAutoRows { id, tracks } => {
                self.update_style(id, |node| node.grid_auto_rows = tracks);
            }
            RenderCommand::SetGridAutoFlow { id, grid_auto_flow } => {
                self.update_style(id, |node| node.grid_auto_flow = grid_auto_flow);
            }
            RenderCommand::SetGridColumn { id, placement } => {
                self.update_style(id, |node| node.grid_column = placement);
            }
            RenderCommand::SetGridRow { id, placement } => {
                self.update_style(id, |node| node.grid_row = placement);
            }
            RenderCommand::SetGridColumnStart { id, placement } => {
                self.update_style(id, |node| node.grid_column.start = placement);
            }
            RenderCommand::SetGridColumnEnd { id, placement } => {
                self.update_style(id, |node| node.grid_column.end = placement);
            }
            RenderCommand::SetGridRowStart { id, placement } => {
                self.update_style(id, |node| node.grid_row.start = placement);
            }
            RenderCommand::SetGridRowEnd { id, placement } => {
                self.update_style(id, |node| node.grid_row.end = placement);
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
            RenderCommand::HandlePointerMove { x, y } => {
                self.last_pointer_position = Some((x, y));
                self.refresh_pointer_shape();
            }
            RenderCommand::Render { pending } => {
                self.render();
                pending.store(false, Ordering::Release);
            }
            RenderCommand::RenderSync { response } => {
                self.render();
                let _ = response.send(());
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
            DomNode::Image(_) => None,
            DomNode::Input(_) => None,
            DomNode::TextArea(_) => None,
            DomNode::Text(_) => None,
        }
    }

    fn style_mut(&mut self, id: u32) -> Option<&mut DivStyle> {
        match self.nodes.get_mut(&id)? {
            DomNode::Div(node) => Some(&mut node.style),
            DomNode::Span(node) => Some(&mut node.style),
            DomNode::Image(node) => Some(&mut node.style),
            DomNode::Input(node) => Some(&mut node.style),
            DomNode::TextArea(node) => Some(&mut node.style),
            DomNode::Text(_) => None,
        }
    }

    fn set_text(&mut self, id: u32, text: String) {
        let mut metrics_changed = false;
        if let Some(DomNode::Text(node)) = self.nodes.get_mut(&id) {
            let metrics = measure_text(&text);
            metrics_changed = node.metrics != metrics;
            node.text = text;
            node.metrics = metrics;
        }

        if metrics_changed {
            self.sync_taffy_style(id);
            self.mark_layout_dirty();
        }
    }

    fn set_image_source(&mut self, id: u32, src: String) {
        let image = load_png_image(&src).ok();
        if let Some(DomNode::Image(node)) = self.nodes.get_mut(&id) {
            node.src = Some(src);
            node.image = image;
            self.sync_taffy_style(id);
            self.mark_layout_dirty();
        }
    }

    fn set_input_value(&mut self, id: u32, value: String, cursor: u32) {
        let mut natural_size_changed = false;
        match self.nodes.get_mut(&id) {
            Some(DomNode::Input(node)) => {
                let previous_width = input_natural_width(node);
                node.value = value;
                node.cursor = cursor.min(node.value.chars().count() as u32);
                natural_size_changed = previous_width != input_natural_width(node);
            }
            Some(DomNode::TextArea(node)) => {
                let previous = textarea_natural_size(node, textarea_explicit_width(node));
                node.value = value;
                node.cursor = cursor.min(node.value.chars().count() as u32);
                natural_size_changed =
                    previous != textarea_natural_size(node, textarea_explicit_width(node));
            }
            _ => {}
        }

        if natural_size_changed {
            self.sync_taffy_style(id);
            self.mark_layout_dirty();
        }
    }

    fn set_input_focused(&mut self, id: u32, focused: bool) {
        match self.nodes.get_mut(&id) {
            Some(DomNode::Input(node)) => node.focused = focused,
            Some(DomNode::TextArea(node)) => node.focused = focused,
            _ => {}
        }
    }

    fn update_style(&mut self, id: u32, update: impl FnOnce(&mut DivStyle)) {
        if let Some(style) = self.style_mut(id) {
            update(style);
            self.sync_taffy_style(id);
            self.sync_taffy_children(id);
            self.sync_parent_taffy_children(id);
            self.mark_layout_dirty();
        }
    }

    fn update_visual_style(&mut self, id: u32, update: impl FnOnce(&mut DivStyle)) {
        if let Some(style) = self.style_mut(id) {
            update(style);
        }
    }

    fn set_transition(&mut self, id: u32, transitions: Vec<TransitionSpec>) {
        let mut specs = HashMap::new();
        for transition in transitions {
            specs.insert(
                transition.property,
                Duration::from_millis(transition.duration_ms),
            );
        }
        if specs.is_empty() {
            self.transition_specs.remove(&id);
        } else {
            self.transition_specs.insert(id, specs);
        }
    }

    fn set_paint_color(&mut self, id: u32, property: ColorTransitionProperty, target: Background) {
        let now = Instant::now();
        let current = self.paint_color(id, property, now).unwrap_or(target);
        let duration = self
            .transition_specs
            .get(&id)
            .and_then(|specs| specs.get(&property))
            .copied();

        self.active_transitions
            .remove(&TransitionKey { id, property });
        self.set_style_color(id, property, target);

        if self.color_profile != TermProfile::TrueColor {
            return;
        }

        let Some(duration) = duration else {
            return;
        };
        if duration.is_zero()
            || current == target
            || current.rgb().is_none()
            || target.rgb().is_none()
        {
            return;
        }

        self.active_transitions.insert(
            TransitionKey { id, property },
            ActiveColorTransition {
                from: current,
                to: target,
                started_at: now,
                duration,
            },
        );
        self.push_transition_event("transitionstart", id, property);
    }

    fn paint_color(
        &self,
        id: u32,
        property: ColorTransitionProperty,
        now: Instant,
    ) -> Option<Background> {
        let key = TransitionKey { id, property };
        if let Some(transition) = self.active_transitions.get(&key) {
            return Some(transition.color_at(now));
        }

        self.nodes
            .get(&id)
            .and_then(|node| node_style(node))
            .map(|style| style_color(style, property))
    }

    fn set_style_color(&mut self, id: u32, property: ColorTransitionProperty, color: Background) {
        if let Some(style) = self.style_mut(id) {
            match property {
                ColorTransitionProperty::Color => style.color = color,
                ColorTransitionProperty::BackgroundColor => style.background = color,
                ColorTransitionProperty::BorderColor => style.border_color = color,
            }
        }
    }

    fn push_transition_event(&self, event_type: &str, id: u32, property: ColorTransitionProperty) {
        if let Ok(mut events) = self.transition_events.lock() {
            events.push_back(TransitionEvent {
                r#type: event_type.to_string(),
                target_id: id,
                property_name: transition_property_name(property).to_string(),
            });
            while events.len() > 1024 {
                events.pop_front();
            }
        }
    }

    fn mark_layout_dirty(&mut self) {
        self.scroll_metrics_dirty = true;
        self.inline_metrics_cache.clear();
    }

    fn sync_taffy_style(&mut self, id: u32) {
        let Some(taffy_id) = self.taffy_ids.get(&id).copied() else {
            return;
        };
        let Some(style) = self.taffy_style(id) else {
            return;
        };
        let _ = self.taffy.set_style(taffy_id, style);
    }

    fn sync_taffy_children(&mut self, id: u32) {
        let Some(taffy_id) = self.taffy_ids.get(&id).copied() else {
            return;
        };
        let children = self.taffy_children_for(id);
        let _ = self.taffy.set_children(taffy_id, &children);
    }

    fn sync_parent_taffy_children(&mut self, id: u32) {
        if let Some(parent) = self.parent_by_child.get(&id).copied() {
            self.sync_taffy_children(parent);
        }
    }

    fn sync_image_taffy_styles(&mut self, terminal_size: TerminalSize) {
        let image_ids = self
            .nodes
            .iter()
            .filter_map(|(id, node)| matches!(node, DomNode::Image(_)).then_some(*id))
            .collect::<Vec<_>>();

        for id in image_ids {
            let Some(taffy_id) = self.taffy_ids.get(&id).copied() else {
                continue;
            };
            let Some(DomNode::Image(node)) = self.nodes.get(&id) else {
                continue;
            };
            let _ = self
                .taffy
                .set_style(taffy_id, image_taffy_style(node, terminal_size));
        }
    }

    fn sync_appended_taffy_child(&mut self, parent_id: u32, child: u32) {
        if self.can_fast_append_taffy_child(parent_id, child) {
            if let (Some(parent_taffy_id), Some(child_taffy_id)) = (
                self.taffy_ids.get(&parent_id).copied(),
                self.taffy_ids.get(&child).copied(),
            ) {
                if self
                    .taffy
                    .add_child(parent_taffy_id, child_taffy_id)
                    .is_ok()
                {
                    return;
                }
            }
        }

        self.sync_taffy_children(parent_id);
    }

    fn can_fast_append_taffy_child(&self, parent_id: u32, child: u32) -> bool {
        let Some(parent) = self.nodes.get(&parent_id) else {
            return false;
        };

        match parent {
            DomNode::Div(node) => match node.style.display {
                LayoutDisplay::Flex | LayoutDisplay::Grid => true,
                LayoutDisplay::Block => !self.is_inline_child(child),
                LayoutDisplay::Inline => false,
            },
            DomNode::Span(node) => {
                node.style.display != LayoutDisplay::Inline && !self.is_inline_child(child)
            }
            DomNode::Image(_) => false,
            DomNode::Input(_) => false,
            DomNode::TextArea(_) => false,
            DomNode::Text(_) => false,
        }
    }

    fn is_inline_child(&self, id: u32) -> bool {
        match self.nodes.get(&id) {
            Some(DomNode::Text(_)) => true,
            Some(DomNode::Div(node)) => node.style.display == LayoutDisplay::Inline,
            Some(DomNode::Span(node)) => node.style.display == LayoutDisplay::Inline,
            Some(DomNode::Image(node)) => node.style.display == LayoutDisplay::Inline,
            Some(DomNode::Input(node)) => node.style.display == LayoutDisplay::Inline,
            Some(DomNode::TextArea(node)) => node.style.display == LayoutDisplay::Inline,
            None => false,
        }
    }

    fn taffy_style(&self, id: u32) -> Option<Style> {
        match self.nodes.get(&id)? {
            DomNode::Div(node) => Some(node.style.to_taffy()),
            DomNode::Span(node) => Some(node.style.to_taffy()),
            DomNode::Image(node) => Some(image_taffy_style(node, query_terminal_size())),
            DomNode::Input(node) => Some(input_taffy_style(node)),
            DomNode::TextArea(node) => Some(textarea_taffy_style(node)),
            DomNode::Text(node) => Some(node.style()),
        }
    }

    fn taffy_children_for(&self, id: u32) -> Vec<NodeId> {
        let Some(node) = self.nodes.get(&id) else {
            return Vec::new();
        };

        let children = match node {
            DomNode::Div(node) => {
                if self.is_inline_container(node) {
                    return Vec::new();
                }
                &node.children
            }
            DomNode::Span(node) => {
                if self.span_uses_inline_layout(node) {
                    return Vec::new();
                }
                &node.children
            }
            DomNode::Image(_) => return Vec::new(),
            DomNode::Input(_) => return Vec::new(),
            DomNode::TextArea(_) => return Vec::new(),
            DomNode::Text(_) => return Vec::new(),
        };

        children
            .iter()
            .filter_map(|child| self.taffy_ids.get(child).copied())
            .collect()
    }

    fn scroll_offset_mut(&mut self, id: u32) -> Option<(&mut u32, &mut u32)> {
        match self.nodes.get_mut(&id)? {
            DomNode::Div(node) => Some((&mut node.scroll_left, &mut node.scroll_top)),
            DomNode::Span(node) => Some((&mut node.scroll_left, &mut node.scroll_top)),
            DomNode::Image(_) => None,
            DomNode::Input(_) => None,
            DomNode::TextArea(_) => None,
            DomNode::Text(_) => None,
        }
    }

    fn scroll_offset(&self, id: u32) -> Option<(u32, u32)> {
        match self.nodes.get(&id)? {
            DomNode::Div(node) => Some((node.scroll_left, node.scroll_top)),
            DomNode::Span(node) => Some((node.scroll_left, node.scroll_top)),
            DomNode::Image(_) => None,
            DomNode::Input(_) => None,
            DomNode::TextArea(_) => None,
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
            DomNode::Image(_) => None,
            DomNode::Input(_) => None,
            DomNode::TextArea(_) => None,
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

    fn ensure_layout_and_scroll_metrics(&mut self) -> Option<TerminalSize> {
        let root = self.root?;
        let root_node = self.taffy_ids.get(&root).copied()?;

        let terminal_size = query_terminal_size();
        let TerminalSize { cols, rows, .. } = terminal_size;
        let layout_size = (cols, rows);
        if self.last_layout_size != Some(layout_size) {
            self.mark_layout_dirty();
            self.sync_image_taffy_styles(terminal_size);
        }

        if self.scroll_metrics_dirty {
            let available = Size {
                width: AvailableSpace::Definite(cols as f32),
                height: AvailableSpace::Definite(rows as f32),
            };

            if self.taffy.compute_layout(root_node, available).is_err() {
                return None;
            }

            let mut scroll_metrics = HashMap::new();
            self.collect_scroll_metrics(root, &mut scroll_metrics);
            self.clamp_scroll_offsets(&mut scroll_metrics);
            self.scroll_metrics = scroll_metrics;
            self.scroll_metrics_dirty = false;
            self.last_layout_size = Some(layout_size);
        }

        Some(terminal_size)
    }

    fn render(&mut self) {
        let Some(root) = self.root else {
            return;
        };
        let Some(terminal_size) = self.ensure_layout_and_scroll_metrics() else {
            return;
        };
        let TerminalSize { cols, rows, .. } = terminal_size;

        let mut frame = Frame::new(
            cols as usize,
            rows as usize,
            self.selection
                .as_ref()
                .is_some_and(|selection| selection.selecting),
        );
        let mut hit_regions = Vec::new();
        self.paint_node(
            root,
            0.0,
            0.0,
            &mut frame,
            &mut hit_regions,
            None,
            Background::Default,
            ClipBounds::unbounded(),
        );
        self.refresh_active_selection_focus(&frame);
        self.content_frame = Some(frame.clone());
        let mut output_frame = frame;
        output_frame.apply_selection(self.selection.as_ref());
        let _ = output_frame.write_diff_to_stdout(self.previous_frame.as_ref(), self.color_profile);
        self.previous_frame = Some(output_frame);
        self.hit_regions = hit_regions;
        self.refresh_pointer_shape();
        self.finish_completed_transitions(Instant::now());
    }

    fn has_active_transitions(&self) -> bool {
        !self.active_transitions.is_empty()
    }

    fn finish_completed_transitions(&mut self, now: Instant) {
        let completed = self
            .active_transitions
            .iter()
            .filter_map(|(key, transition)| {
                transition.is_complete(now).then_some((*key, transition.to))
            })
            .collect::<Vec<_>>();

        for (key, target) in completed {
            self.active_transitions.remove(&key);
            self.set_style_color(key.id, key.property, target);
            self.push_transition_event("transitionend", key.id, key.property);
        }
    }

    fn collect_scroll_metrics(&mut self, id: u32, metrics: &mut HashMap<u32, ScrollMetrics>) {
        let Some(dom_node) = self.nodes.get(&id) else {
            return;
        };
        let Some(taffy_id) = self.taffy_ids.get(&id) else {
            return;
        };
        let Ok(layout) = self.taffy.layout(*taffy_id) else {
            return;
        };

        let children = match dom_node {
            DomNode::Div(node) => Some(node.children.clone()),
            DomNode::Span(node) => Some(node.children.clone()),
            DomNode::Image(_) => None,
            DomNode::Input(_) => None,
            DomNode::TextArea(_) => None,
            DomNode::Text(_) => None,
        };
        let Some(children) = children else {
            return;
        };

        let content_box = content_box_size(dom_node, layout.size);
        let content_origin = content_box_origin(dom_node);
        let client_width = dimension_to_cells(content_box.width);
        let client_height = dimension_to_cells(content_box.height);
        let mut scroll_width = client_width;
        let mut scroll_height = client_height;

        if self.is_inline_children(&children) {
            let inline = self.measure_inline_children_for(id, &children, client_width.max(1));
            scroll_width = scroll_width.max(inline.width);
            scroll_height = scroll_height.max(inline.height);
        } else {
            for child in children {
                self.collect_scroll_metrics(child, metrics);

                let Some(child_taffy_id) = self.taffy_ids.get(&child) else {
                    continue;
                };
                let Ok(child_layout) = self.taffy.layout(*child_taffy_id) else {
                    continue;
                };

                scroll_width = scroll_width.max(edge_to_cells(
                    child_layout.location.x + child_layout.size.width - content_origin.x,
                ));
                scroll_height = scroll_height.max(edge_to_cells(
                    child_layout.location.y + child_layout.size.height - content_origin.y,
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
                DomNode::Image(_) => {}
                DomNode::Input(_) => {}
                DomNode::TextArea(_) => {}
                DomNode::Text(_) => {}
            }
        }
    }

    fn measure_inline_children_for(
        &mut self,
        id: u32,
        children: &[u32],
        width: u32,
    ) -> InlineMetrics {
        let key = InlineMetricsKey {
            id,
            width: width.max(1),
        };
        if let Some(metrics) = self.inline_metrics_cache.get(&key).copied() {
            return metrics;
        }

        let metrics = measure_inline_children(children, key.width, &self.nodes);
        self.inline_metrics_cache.insert(key, metrics);
        metrics
    }

    fn paint_node(
        &self,
        id: u32,
        parent_x: f32,
        parent_y: f32,
        frame: &mut Frame,
        hit_regions: &mut Vec<HitRegion>,
        selection_background: Option<Background>,
        foreground: Background,
        clip: ClipBounds,
    ) {
        let Some(dom_node) = self.nodes.get(&id) else {
            return;
        };
        let Some(taffy_id) = self.taffy_ids.get(&id) else {
            return;
        };
        let Ok(layout) = self.taffy.layout(*taffy_id) else {
            return;
        };

        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;
        let now = Instant::now();
        let bounds = cell_rect_from_edges(x, y, layout.size.width, layout.size.height);
        if !frame.capture_hidden_selection_units && clip.clip_rect(bounds).is_none() {
            return;
        }

        match dom_node {
            DomNode::Div(node) => {
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                let node_foreground = self
                    .paint_color(id, ColorTransitionProperty::Color, now)
                    .unwrap_or(node.style.color);
                let foreground = effective_foreground(node_foreground, foreground);
                let background = self
                    .paint_color(id, ColorTransitionProperty::BackgroundColor, now)
                    .unwrap_or(node.style.background);
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(bounds, background, selection_background, clip);
                frame.clear_chunky_rounded_corners(bounds, &node.style, clip);

                let content_bounds = content_box_rect(bounds, &node.style);
                let child_clip = child_clip_for(
                    node.style.overflow_x,
                    node.style.overflow_y,
                    content_bounds,
                    clip,
                );
                let child_x = x - scroll_offset(node.style.overflow_x, node.scroll_left);
                let child_y = y - scroll_offset(node.style.overflow_y, node.scroll_top);
                if self.is_inline_container(node) {
                    self.paint_inline_children(
                        &node.children,
                        content_bounds.left
                            - scroll_offset_cells(node.style.overflow_x, node.scroll_left),
                        content_bounds.top
                            - scroll_offset_cells(node.style.overflow_y, node.scroll_top),
                        content_bounds.width(),
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        foreground,
                        child_clip,
                    );
                } else {
                    let child_range = self.visible_child_range(
                        &node.style,
                        &node.children,
                        child_y,
                        child_clip,
                        frame,
                    );
                    for child in &node.children[child_range] {
                        self.paint_node(
                            *child,
                            child_x,
                            child_y,
                            frame,
                            hit_regions,
                            selection_background,
                            foreground,
                            child_clip,
                        );
                    }
                }
                let border_color = self
                    .paint_color(id, ColorTransitionProperty::BorderColor, now)
                    .unwrap_or(node.style.border_color);
                frame.stroke_border(
                    bounds,
                    &node.style,
                    border_color,
                    selection_background,
                    clip,
                );
            }
            DomNode::Span(node) => {
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                let node_foreground = self
                    .paint_color(id, ColorTransitionProperty::Color, now)
                    .unwrap_or(node.style.color);
                let foreground = effective_foreground(node_foreground, foreground);
                let background = self
                    .paint_color(id, ColorTransitionProperty::BackgroundColor, now)
                    .unwrap_or(node.style.background);
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(bounds, background, selection_background, clip);
                frame.clear_chunky_rounded_corners(bounds, &node.style, clip);
                let content_bounds = content_box_rect(bounds, &node.style);
                let child_clip = child_clip_for(
                    node.style.overflow_x,
                    node.style.overflow_y,
                    content_bounds,
                    clip,
                );
                let child_x = x - scroll_offset(node.style.overflow_x, node.scroll_left);
                let child_y = y - scroll_offset(node.style.overflow_y, node.scroll_top);
                if self.span_uses_inline_layout(node) {
                    self.paint_inline_children(
                        &node.children,
                        content_bounds.left
                            - scroll_offset_cells(node.style.overflow_x, node.scroll_left),
                        content_bounds.top
                            - scroll_offset_cells(node.style.overflow_y, node.scroll_top),
                        content_bounds.width(),
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        foreground,
                        child_clip,
                    );
                } else {
                    let child_range = self.visible_child_range(
                        &node.style,
                        &node.children,
                        child_y,
                        child_clip,
                        frame,
                    );
                    for child in &node.children[child_range] {
                        self.paint_node(
                            *child,
                            child_x,
                            child_y,
                            frame,
                            hit_regions,
                            selection_background,
                            foreground,
                            child_clip,
                        );
                    }
                }
                let border_color = self
                    .paint_color(id, ColorTransitionProperty::BorderColor, now)
                    .unwrap_or(node.style.border_color);
                frame.stroke_border(
                    bounds,
                    &node.style,
                    border_color,
                    selection_background,
                    clip,
                );
            }
            DomNode::Text(node) => {
                frame.write_text(
                    bounds.left,
                    bounds.top,
                    &node.text,
                    foreground,
                    selection_background,
                    clip,
                );
            }
            DomNode::Image(node) => {
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(bounds, Background::Default, selection_background, clip);
                frame.write_image(node, bounds, selection_background, clip);
            }
            DomNode::Input(node) => {
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                let node_foreground = self
                    .paint_color(id, ColorTransitionProperty::Color, now)
                    .unwrap_or(node.style.color);
                let foreground = effective_foreground(node_foreground, foreground);
                let background = self
                    .paint_color(id, ColorTransitionProperty::BackgroundColor, now)
                    .unwrap_or(node.style.background);
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(bounds, background, selection_background, clip);
                frame.clear_chunky_rounded_corners(bounds, &node.style, clip);
                frame.write_input(
                    content_box_rect(bounds, &node.style),
                    &node.value,
                    node.cursor,
                    node.focused,
                    foreground,
                    selection_background,
                    clip,
                );
                let border_color = self
                    .paint_color(id, ColorTransitionProperty::BorderColor, now)
                    .unwrap_or(node.style.border_color);
                frame.stroke_border(
                    bounds,
                    &node.style,
                    border_color,
                    selection_background,
                    clip,
                );
            }
            DomNode::TextArea(node) => {
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                let node_foreground = self
                    .paint_color(id, ColorTransitionProperty::Color, now)
                    .unwrap_or(node.style.color);
                let foreground = effective_foreground(node_foreground, foreground);
                let background = self
                    .paint_color(id, ColorTransitionProperty::BackgroundColor, now)
                    .unwrap_or(node.style.background);
                push_hit_region(hit_regions, id, bounds, clip);
                frame.fill_rect(bounds, background, selection_background, clip);
                frame.clear_chunky_rounded_corners(bounds, &node.style, clip);
                frame.write_textarea(
                    content_box_rect(bounds, &node.style),
                    &node.value,
                    node.cursor,
                    node.focused,
                    foreground,
                    selection_background,
                    clip,
                );
                let border_color = self
                    .paint_color(id, ColorTransitionProperty::BorderColor, now)
                    .unwrap_or(node.style.border_color);
                frame.stroke_border(
                    bounds,
                    &node.style,
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }
    }

    fn visible_child_range(
        &self,
        style: &DivStyle,
        children: &[u32],
        child_y: f32,
        clip: ClipBounds,
        frame: &Frame,
    ) -> Range<usize> {
        if frame.capture_hidden_selection_units || !can_cull_vertical_children(style) {
            return 0..children.len();
        }

        self.vertical_visible_child_range(children, child_y, clip)
            .unwrap_or(0..children.len())
    }

    fn vertical_visible_child_range(
        &self,
        children: &[u32],
        child_y: f32,
        clip: ClipBounds,
    ) -> Option<Range<usize>> {
        let clip_top = clip.top? as f32;
        let clip_bottom = clip.bottom? as f32;
        if children.is_empty() || clip_top >= clip_bottom {
            return Some(0..0);
        }

        let mut low = 0;
        let mut high = children.len();
        while low < high {
            let mid = low + (high - low) / 2;
            let (_, bottom) = self.child_vertical_edges(children[mid], child_y)?;
            if bottom <= clip_top {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let start = low;

        low = start;
        high = children.len();
        while low < high {
            let mid = low + (high - low) / 2;
            let (top, _) = self.child_vertical_edges(children[mid], child_y)?;
            if top < clip_bottom {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        Some(start..low)
    }

    fn child_vertical_edges(&self, id: u32, parent_y: f32) -> Option<(f32, f32)> {
        let taffy_id = self.taffy_ids.get(&id)?;
        let layout = self.taffy.layout(*taffy_id).ok()?;
        let top = parent_y + layout.location.y;
        Some((top, top + layout.size.height))
    }

    fn is_inline_container(&self, node: &DivNode) -> bool {
        node.style.display == LayoutDisplay::Block && self.is_inline_children(&node.children)
    }

    fn span_uses_inline_layout(&self, node: &SpanNode) -> bool {
        node.style.display == LayoutDisplay::Inline
            || (node.style.display == LayoutDisplay::Block
                && self.is_inline_children(&node.children))
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
                Some(DomNode::Image(node)) if node.style.display == LayoutDisplay::Inline => {
                    has_inline_element = true;
                }
                Some(DomNode::Input(node)) if node.style.display == LayoutDisplay::Inline => {
                    has_inline_element = true;
                }
                Some(DomNode::TextArea(node)) if node.style.display == LayoutDisplay::Inline => {
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
        foreground: Background,
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
                foreground,
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
        foreground: Background,
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
                    foreground,
                    clip,
                );
            }
            Some(DomNode::Span(node)) => {
                let node_background = self
                    .paint_color(id, ColorTransitionProperty::BackgroundColor, Instant::now())
                    .unwrap_or(node.style.background);
                let background = if node_background == Background::Default {
                    background
                } else {
                    node_background
                };
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                let node_foreground = self
                    .paint_color(id, ColorTransitionProperty::Color, Instant::now())
                    .unwrap_or(node.style.color);
                let foreground = effective_foreground(node_foreground, foreground);

                for child in &node.children {
                    self.paint_inline_node(
                        *child,
                        cursor,
                        background,
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        foreground,
                        clip,
                    );
                }
            }
            Some(DomNode::Div(node)) if node.style.display == LayoutDisplay::Inline => {
                let node_background = self
                    .paint_color(id, ColorTransitionProperty::BackgroundColor, Instant::now())
                    .unwrap_or(node.style.background);
                let background = if node_background == Background::Default {
                    background
                } else {
                    node_background
                };
                let selection_background = effective_selection_background(
                    node.style.selection_background,
                    selection_background,
                );
                let node_foreground = self
                    .paint_color(id, ColorTransitionProperty::Color, Instant::now())
                    .unwrap_or(node.style.color);
                let foreground = effective_foreground(node_foreground, foreground);

                for child in &node.children {
                    self.paint_inline_node(
                        *child,
                        cursor,
                        background,
                        frame,
                        hit_regions,
                        Some(id),
                        selection_background,
                        foreground,
                        clip,
                    );
                }
            }
            Some(DomNode::Image(node)) if node.style.display == LayoutDisplay::Inline => {
                write_inline_image(id, node, cursor, frame, hit_regions, hit_target, clip);
            }
            Some(DomNode::Input(node)) if node.style.display == LayoutDisplay::Inline => {
                write_inline_input(
                    id,
                    node,
                    cursor,
                    frame,
                    hit_regions,
                    hit_target,
                    foreground,
                    clip,
                );
            }
            Some(DomNode::TextArea(node)) if node.style.display == LayoutDisplay::Inline => {
                write_inline_textarea(
                    id,
                    node,
                    cursor,
                    frame,
                    hit_regions,
                    hit_target,
                    foreground,
                    clip,
                );
            }
            Some(DomNode::Div(_))
            | Some(DomNode::Image(_))
            | Some(DomNode::Input(_))
            | Some(DomNode::TextArea(_))
            | None => {}
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

    fn refresh_pointer_shape(&mut self) {
        let Some((x, y)) = self.last_pointer_position else {
            return;
        };

        let shape = self.pointer_shape_for_point(x, y);
        if shape == self.current_pointer_shape {
            return;
        }

        let mut out = io::stdout().lock();
        if write_pointer_shape(&mut out, shape).is_ok() && out.flush().is_ok() {
            self.current_pointer_shape = shape;
        }
    }

    fn pointer_shape_for_point(&self, x: u32, y: u32) -> Option<&'static str> {
        let mut current = self.hit_test_id(x, y);
        while let Some(id) = current {
            if let Some(shape) = self
                .nodes
                .get(&id)
                .and_then(node_style)
                .and_then(|style| style.cursor.osc_shape())
            {
                return Some(shape);
            }
            current = self.parent_by_child.get(&id).copied();
        }
        Some("default")
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
        let _ = output_frame.write_diff_to_stdout(self.previous_frame.as_ref(), self.color_profile);
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

    fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
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

fn content_box_rect(bounds: ClipRect, style: &DivStyle) -> ClipRect {
    let left = bounds.left + border_extent_cells(style.border_left);
    let top = bounds.top + border_extent_cells(style.border_top);
    let right = bounds.right - border_extent_cells(style.border_right);
    let bottom = bounds.bottom - border_extent_cells(style.border_bottom);

    ClipRect {
        left: left.min(right),
        top: top.min(bottom),
        right: right.max(left),
        bottom: bottom.max(top),
    }
}

fn content_box_size(node: &DomNode, size: Size<f32>) -> Size<f32> {
    let Some(style) = node_style(node) else {
        return size;
    };

    Size {
        width: (size.width - border_extent(style.border_left) - border_extent(style.border_right))
            .max(0.0),
        height: (size.height
            - border_extent(style.border_top)
            - border_extent(style.border_bottom))
        .max(0.0),
    }
}

fn content_box_origin(node: &DomNode) -> Point<f32> {
    let Some(style) = node_style(node) else {
        return Point::ZERO;
    };

    Point {
        x: border_extent(style.border_left),
        y: border_extent(style.border_top),
    }
}

fn border_extent(style: BorderStyle) -> f32 {
    border_extent_cells(style) as f32
}

fn border_extent_cells(style: BorderStyle) -> i32 {
    if style == BorderStyle::None {
        0
    } else {
        1
    }
}

fn can_cull_vertical_children(style: &DivStyle) -> bool {
    match style.display {
        LayoutDisplay::Block => true,
        LayoutDisplay::Flex => {
            matches!(style.flex_direction, LayoutFlexDirection::Column)
                && matches!(style.flex_wrap, LayoutFlexWrap::NoWrap)
        }
        LayoutDisplay::Inline | LayoutDisplay::Grid => false,
    }
}

fn scroll_offset(overflow: LayoutOverflow, value: u32) -> f32 {
    if overflow == LayoutOverflow::Scroll {
        value as f32
    } else {
        0.0
    }
}

fn scroll_offset_cells(overflow: LayoutOverflow, value: u32) -> i32 {
    if overflow == LayoutOverflow::Scroll {
        value.min(i32::MAX as u32) as i32
    } else {
        0
    }
}

fn cell_rect_from_edges(x: f32, y: f32, width: f32, height: f32) -> ClipRect {
    let left = x.round() as i32;
    let top = y.round() as i32;
    let right = (x + width).round() as i32;
    let bottom = (y + height).round() as i32;

    ClipRect {
        left,
        top,
        right: right.max(left),
        bottom: bottom.max(top),
    }
}

fn effective_selection_background(
    own: Option<Background>,
    inherited: Option<Background>,
) -> Option<Background> {
    own.filter(|background| *background != Background::Default)
        .or(inherited)
}

fn effective_foreground(own: Background, inherited: Background) -> Background {
    if own == Background::Default {
        inherited
    } else {
        own
    }
}

impl ActiveColorTransition {
    fn color_at(self, now: Instant) -> Background {
        let progress = (now - self.started_at).as_secs_f32() / self.duration.as_secs_f32();
        interpolate_background(self.from, self.to, progress.clamp(0.0, 1.0)).unwrap_or(self.to)
    }

    fn is_complete(self, now: Instant) -> bool {
        now.duration_since(self.started_at) >= self.duration
    }
}

fn interpolate_background(from: Background, to: Background, progress: f32) -> Option<Background> {
    let from = Oklab::from_rgb(from.rgb()?);
    let to = Oklab::from_rgb(to.rgb()?);
    Some(
        Oklab {
            l: interpolate_float(from.l, to.l, progress),
            a: interpolate_float(from.a, to.a, progress),
            b: interpolate_float(from.b, to.b, progress),
        }
        .to_background(),
    )
}

fn interpolate_float(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

#[derive(Clone, Copy)]
struct Oklab {
    l: f32,
    a: f32,
    b: f32,
}

impl Oklab {
    fn from_rgb((red, green, blue): (u8, u8, u8)) -> Self {
        let red = srgb_to_linear(red);
        let green = srgb_to_linear(green);
        let blue = srgb_to_linear(blue);

        let l = 0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue;
        let m = 0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue;
        let s = 0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue;

        let l = l.cbrt();
        let m = m.cbrt();
        let s = s.cbrt();

        Self {
            l: 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            a: 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            b: 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
        }
    }

    fn to_background(self) -> Background {
        let l = self.l + 0.396_337_78 * self.a + 0.215_803_76 * self.b;
        let m = self.l - 0.105_561_346 * self.a - 0.063_854_17 * self.b;
        let s = self.l - 0.089_484_18 * self.a - 1.291_485_5 * self.b;

        let l = l * l * l;
        let m = m * m * m;
        let s = s * s * s;

        let red = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
        let green = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
        let blue = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;

        Background::Rgb(
            linear_to_srgb(red),
            linear_to_srgb(green),
            linear_to_srgb(blue),
        )
    }
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let value = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

fn ascii_pixel_char(red: u8, green: u8, blue: u8) -> char {
    const ASCII_CHARS: &[u8] =
        b"$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'. ";
    let intensity =
        (u16::from(red) + u16::from(green) + u16::from(blue) + 255) as f32 / (255.0 * 4.0);
    let index =
        ASCII_CHARS.len() - 1 - (intensity * (ASCII_CHARS.len() - 1) as f32).floor() as usize;
    ASCII_CHARS[index] as char
}

fn image_pixel(image: &ImageData, x: u32, y: u32) -> Option<[u8; 3]> {
    let pixel_index = (y as usize * image.width_px as usize + x as usize) * 3;
    let pixel = image.rgb.get(pixel_index..pixel_index + 3)?;
    Some([pixel[0], pixel[1], pixel[2]])
}

fn node_style(node: &DomNode) -> Option<&DivStyle> {
    match node {
        DomNode::Div(node) => Some(&node.style),
        DomNode::Span(node) => Some(&node.style),
        DomNode::Image(node) => Some(&node.style),
        DomNode::Input(node) => Some(&node.style),
        DomNode::TextArea(node) => Some(&node.style),
        DomNode::Text(_) => None,
    }
}

fn style_color(style: &DivStyle, property: ColorTransitionProperty) -> Background {
    match property {
        ColorTransitionProperty::Color => style.color,
        ColorTransitionProperty::BackgroundColor => style.background,
        ColorTransitionProperty::BorderColor => style.border_color,
    }
}

fn transition_property_name(property: ColorTransitionProperty) -> &'static str {
    match property {
        ColorTransitionProperty::Color => "color",
        ColorTransitionProperty::BackgroundColor => "background-color",
        ColorTransitionProperty::BorderColor => "border-color",
    }
}

fn has_border(style: &DivStyle) -> bool {
    style.border_top != BorderStyle::None
        || style.border_right != BorderStyle::None
        || style.border_bottom != BorderStyle::None
        || style.border_left != BorderStyle::None
}

fn has_chunky_rounded_corner(style: &DivStyle) -> bool {
    (style.border_top == BorderStyle::ChunkyRounded
        && (style.border_left == BorderStyle::ChunkyRounded
            || style.border_right == BorderStyle::ChunkyRounded))
        || (style.border_bottom == BorderStyle::ChunkyRounded
            && (style.border_left == BorderStyle::ChunkyRounded
                || style.border_right == BorderStyle::ChunkyRounded))
}

#[derive(Clone, Copy)]
struct BorderGlyphs {
    horizontal: char,
    vertical: char,
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
}

fn border_glyphs(style: BorderStyle) -> BorderGlyphs {
    match style {
        BorderStyle::None | BorderStyle::Solid => BorderGlyphs {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        },
        BorderStyle::Double => BorderGlyphs {
            horizontal: '═',
            vertical: '║',
            top_left: '╔',
            top_right: '╗',
            bottom_left: '╚',
            bottom_right: '╝',
        },
        BorderStyle::Heavy => BorderGlyphs {
            horizontal: '━',
            vertical: '┃',
            top_left: '┏',
            top_right: '┓',
            bottom_left: '┗',
            bottom_right: '┛',
        },
        BorderStyle::Rounded => BorderGlyphs {
            horizontal: '─',
            vertical: '│',
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
        },
        BorderStyle::ChunkyRounded => BorderGlyphs {
            horizontal: '█',
            vertical: '█',
            top_left: '🭁',
            top_right: '🭌',
            bottom_left: '🭒',
            bottom_right: '🭝',
        },
        BorderStyle::Ascii => BorderGlyphs {
            horizontal: '-',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        },
    }
}

fn border_char_at(
    style: &DivStyle,
    at_left: bool,
    at_right: bool,
    at_top: bool,
    at_bottom: bool,
) -> char {
    let border_top = at_top && style.border_top != BorderStyle::None;
    let border_right = at_right && style.border_right != BorderStyle::None;
    let border_bottom = at_bottom && style.border_bottom != BorderStyle::None;
    let border_left = at_left && style.border_left != BorderStyle::None;
    let corner_style = if border_top {
        style.border_top
    } else if border_bottom {
        style.border_bottom
    } else if border_left {
        style.border_left
    } else if border_right {
        style.border_right
    } else {
        BorderStyle::None
    };
    let glyphs = border_glyphs(corner_style);

    match (border_top, border_right, border_bottom, border_left) {
        (true, true, _, _) => glyphs.top_right,
        (true, _, _, true) => glyphs.top_left,
        (_, true, true, _) => glyphs.bottom_right,
        (_, _, true, true) => glyphs.bottom_left,
        (true, _, _, _) | (_, _, true, _) => glyphs.horizontal,
        (_, true, _, _) | (_, _, _, true) => glyphs.vertical,
        _ => ' ',
    }
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
    value.max(0.0).round() as u32
}

fn image_taffy_style(node: &ImageNode, terminal_size: TerminalSize) -> Style {
    let mut style = node.style.to_taffy();
    let Some(image) = node.image.as_ref() else {
        return style;
    };

    let natural = natural_image_cells(image, terminal_size);
    style.aspect_ratio = image_cell_aspect_ratio(image, terminal_size);

    if matches!(node.style.width, CssDimension::Auto) {
        style.size.width = Dimension::length(natural.width);
    }
    if matches!(node.style.height, CssDimension::Auto) {
        style.size.height = Dimension::length(natural.height);
    }

    style
}

fn image_cell_size(node: &ImageNode, terminal_size: TerminalSize) -> (u32, u32) {
    let Some(image) = node.image.as_ref() else {
        return (0, 0);
    };

    let natural = natural_image_cells(image, terminal_size);
    let width = match node.style.width {
        CssDimension::Length(value) => value.max(0.0).round() as u32,
        _ => natural.width.round() as u32,
    };
    let height = match node.style.height {
        CssDimension::Length(value) => value.max(0.0).round() as u32,
        _ => natural.height.round() as u32,
    };

    (width.max(1), height.max(1))
}

struct NaturalImageCells {
    width: f32,
    height: f32,
}

fn natural_image_cells(image: &ImageData, terminal_size: TerminalSize) -> NaturalImageCells {
    let cell_width_px = terminal_size
        .pixel_width
        .checked_div(terminal_size.cols.max(1))
        .filter(|value| *value > 0)
        .unwrap_or(8);
    let cell_height_px = terminal_size
        .pixel_height
        .checked_div(terminal_size.rows.max(1))
        .filter(|value| *value > 0)
        .unwrap_or(16);

    NaturalImageCells {
        width: (image.width_px as f32 / cell_width_px as f32).max(1.0),
        height: (image.height_px as f32 / cell_height_px as f32).max(1.0),
    }
}

fn image_cell_aspect_ratio(image: &ImageData, terminal_size: TerminalSize) -> Option<f32> {
    if image.width_px == 0 || image.height_px == 0 {
        return None;
    }

    let natural = natural_image_cells(image, terminal_size);
    Some(natural.width / natural.height)
}

fn input_taffy_style(node: &InputNode) -> Style {
    let mut style = node.style.to_taffy();

    if matches!(node.style.width, CssDimension::Auto) {
        style.size.width = Dimension::length(input_natural_width(node) as f32);
    }
    if matches!(node.style.height, CssDimension::Auto) {
        style.size.height = Dimension::length(1.0);
    }

    style
}

fn input_natural_width(node: &InputNode) -> u32 {
    node.value.chars().count().max(1) as u32
}

fn input_cell_size(node: &InputNode) -> (u32, u32) {
    let width = match node.style.width {
        CssDimension::Length(value) => value.max(0.0).round() as u32,
        _ => input_natural_width(node),
    };
    let height = match node.style.height {
        CssDimension::Length(value) => value.max(0.0).round() as u32,
        _ => 1,
    };

    (width.max(1), height.max(1))
}

fn textarea_taffy_style(node: &TextAreaNode) -> Style {
    let mut style = node.style.to_taffy();
    let natural = textarea_natural_size(node, textarea_explicit_width(node));

    if matches!(node.style.width, CssDimension::Auto) {
        style.size.width = Dimension::length(natural.0 as f32);
    }
    if matches!(node.style.height, CssDimension::Auto) {
        style.size.height = Dimension::length(natural.1 as f32);
    }

    style
}

fn textarea_natural_size(node: &TextAreaNode, wrap_width: Option<usize>) -> (u32, u32) {
    let layout = TextLayout::new(&node.value, wrap_width, TextWrapMode::Word);
    (layout.width.max(1) as u32, layout.height.max(1) as u32)
}

fn textarea_cell_size(node: &TextAreaNode) -> (u32, u32) {
    let explicit_width = textarea_explicit_width(node);
    let natural = textarea_natural_size(node, explicit_width);
    let width = match node.style.width {
        CssDimension::Length(value) => value.max(0.0).round() as u32,
        _ => natural.0,
    };
    let height = match node.style.height {
        CssDimension::Length(value) => value.max(0.0).round() as u32,
        _ => natural.1,
    };

    (width.max(1), height.max(1))
}

fn textarea_explicit_width(node: &TextAreaNode) -> Option<usize> {
    match node.style.width {
        CssDimension::Length(value) => Some((value.max(0.0).round() as usize).max(1)),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum TextWrapMode {
    Word,
}

#[derive(Clone)]
struct TextLayout {
    glyphs: Vec<PositionedGlyph>,
    cursor_positions: Vec<(usize, usize)>,
    width: usize,
    height: usize,
    wrap_width: Option<usize>,
}

#[derive(Clone, Copy)]
struct PositionedGlyph {
    character: char,
    row: usize,
    col: usize,
    width: usize,
}

impl TextLayout {
    fn new(text: &str, wrap_width: Option<usize>, wrap_mode: TextWrapMode) -> Self {
        Self::new_with_start_col(text, wrap_width, 0, wrap_mode)
    }

    fn new_with_start_col(
        text: &str,
        wrap_width: Option<usize>,
        start_col: usize,
        wrap_mode: TextWrapMode,
    ) -> Self {
        let chars = text.chars().collect::<Vec<_>>();
        let mut layout = Self {
            glyphs: Vec::new(),
            cursor_positions: vec![(0, start_col); chars.len() + 1],
            width: start_col,
            height: 1,
            wrap_width: wrap_width.map(|width| width.max(1)),
        };

        match wrap_mode {
            TextWrapMode::Word => layout.layout_word_wrapped(&chars, start_col),
        }

        layout
    }

    fn cursor_position(&self, cursor: usize) -> (usize, usize) {
        let (row, col) = self.cursor_positions[cursor.min(self.cursor_positions.len() - 1)];
        if let Some(width) = self.wrap_width {
            (row, col.min(width.saturating_sub(1)))
        } else {
            (row, col)
        }
    }

    fn end_position(&self) -> (usize, usize) {
        self.cursor_positions[self.cursor_positions.len() - 1]
    }

    fn layout_word_wrapped(&mut self, chars: &[char], start_col: usize) {
        let mut row = 0;
        let mut col = start_col;
        let mut index = 0;

        while index < chars.len() {
            self.cursor_positions[index] = (row, col);
            let character = chars[index];

            if character == '\r' {
                index += 1;
                continue;
            }

            if character == '\n' {
                row += 1;
                col = 0;
                self.height = self.height.max(row + 1);
                index += 1;
                continue;
            }

            if !character.is_whitespace() {
                let word_end = next_word_end(chars, index);
                let word_width = text_width(&chars[index..word_end]);
                if self.should_wrap_before(col, word_width) {
                    row += 1;
                    col = 0;
                    self.height = self.height.max(row + 1);
                }
                while index < word_end {
                    self.cursor_positions[index] = (row, col);
                    let width = character_cell_width(chars[index]);
                    if self.should_wrap_before(col, width) {
                        row += 1;
                        col = 0;
                        self.height = self.height.max(row + 1);
                        self.cursor_positions[index] = (row, col);
                    }
                    self.push_glyph(chars[index], row, col, width);
                    col += width;
                    self.width = self.width.max(col);
                    index += 1;
                }
                continue;
            }

            let width = character_cell_width(character);
            if self.should_wrap_before(col, width) {
                row += 1;
                col = 0;
                self.height = self.height.max(row + 1);
                if character == ' ' || character == '\t' {
                    index += 1;
                    continue;
                }
                self.cursor_positions[index] = (row, col);
            }
            self.push_glyph(character, row, col, width);
            col += width;
            self.width = self.width.max(col);
            index += 1;
        }

        self.cursor_positions[chars.len()] = (row, col);
        self.height = self.height.max(row + 1);
        self.width = self.width.max(1);
    }

    fn should_wrap_before(&self, col: usize, next_width: usize) -> bool {
        matches!(self.wrap_width, Some(width) if col > 0 && next_width > 0 && col + next_width > width)
    }

    fn push_glyph(&mut self, character: char, row: usize, col: usize, width: usize) {
        if width == 0 {
            return;
        }
        self.glyphs.push(PositionedGlyph {
            character,
            row,
            col,
            width,
        });
    }
}

fn next_word_end(chars: &[char], start: usize) -> usize {
    let mut index = start;
    while index < chars.len() && !chars[index].is_whitespace() {
        index += 1;
    }
    index
}

fn text_width(chars: &[char]) -> usize {
    chars
        .iter()
        .map(|character| character_cell_width(*character))
        .sum()
}

fn character_cell_width(character: char) -> usize {
    if character == '\t' {
        return 4;
    }
    UnicodeWidthChar::width(character).unwrap_or(0)
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
    foreground: Background,
    clip: ClipBounds,
) {
    let layout = TextLayout::new_with_start_col(
        text,
        Some(cursor.width.max(1) as usize),
        cursor.col.max(0) as usize,
        TextWrapMode::Word,
    );

    for glyph in &layout.glyphs {
        let x = cursor.x + glyph.col as i32;
        let y = cursor.y + cursor.row + glyph.row as i32;
        frame.write_glyph(
            x,
            y,
            glyph.character,
            glyph.width,
            background,
            foreground,
            selection_background,
            clip,
        );
        if let Some(hit_target) = hit_target {
            push_hit_region(
                hit_regions,
                hit_target,
                ClipRect::new(x, y, glyph.width as i32, 1),
                clip,
            );
        }
    }

    cursor.row += layout.end_position().0 as i32;
    cursor.col = layout.end_position().1 as i32;
}

fn write_inline_image(
    _id: u32,
    node: &ImageNode,
    cursor: &mut InlineCursor,
    frame: &mut Frame,
    hit_regions: &mut Vec<HitRegion>,
    hit_target: Option<u32>,
    clip: ClipBounds,
) {
    let (width, height) = image_cell_size(node, query_terminal_size());
    if width == 0 || height == 0 {
        return;
    }
    if cursor.col + width as i32 > cursor.width {
        cursor.col = 0;
        cursor.row += 1;
    }

    let x = cursor.x + cursor.col;
    let y = cursor.y + cursor.row;
    let rect = ClipRect::new(x, y, width as i32, height as i32);
    frame.write_image(node, rect, None, clip);
    if let Some(hit_target) = hit_target {
        push_hit_region(hit_regions, hit_target, rect, clip);
    }
    cursor.col += width as i32;
    cursor.max_row(height as i32);
}

fn write_inline_input(
    id: u32,
    node: &InputNode,
    cursor: &mut InlineCursor,
    frame: &mut Frame,
    hit_regions: &mut Vec<HitRegion>,
    hit_target: Option<u32>,
    foreground: Background,
    clip: ClipBounds,
) {
    let (width, height) = input_cell_size(node);
    if cursor.col + width as i32 > cursor.width {
        cursor.col = 0;
        cursor.row += 1;
    }

    let x = cursor.x + cursor.col;
    let y = cursor.y + cursor.row;
    let rect = ClipRect::new(x, y, width as i32, height as i32);
    frame.write_input(
        rect,
        &node.value,
        node.cursor,
        node.focused,
        foreground,
        None,
        clip,
    );
    push_hit_region(hit_regions, hit_target.unwrap_or(id), rect, clip);
    cursor.col += width as i32;
    cursor.max_row(height as i32);
}

fn write_inline_textarea(
    id: u32,
    node: &TextAreaNode,
    cursor: &mut InlineCursor,
    frame: &mut Frame,
    hit_regions: &mut Vec<HitRegion>,
    hit_target: Option<u32>,
    foreground: Background,
    clip: ClipBounds,
) {
    let (width, height) = textarea_cell_size(node);
    if cursor.col + width as i32 > cursor.width {
        cursor.col = 0;
        cursor.row += 1;
    }

    let x = cursor.x + cursor.col;
    let y = cursor.y + cursor.row;
    let rect = ClipRect::new(x, y, width as i32, height as i32);
    frame.write_textarea(
        rect,
        &node.value,
        node.cursor,
        node.focused,
        foreground,
        None,
        clip,
    );
    push_hit_region(hit_regions, hit_target.unwrap_or(id), rect, clip);
    cursor.col += width as i32;
    cursor.max_row(height as i32);
}

impl InlineCursor {
    fn max_row(&mut self, height: i32) {
        if height > 1 {
            self.row += height - 1;
        }
    }
}

#[derive(Clone)]
struct Frame {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    selection_units: Vec<SelectionUnit>,
    next_selection_order: usize,
    capture_hidden_selection_units: bool,
}

impl Frame {
    fn new(width: usize, height: usize, capture_hidden_selection_units: bool) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
            selection_units: Vec::new(),
            next_selection_order: 0,
            capture_hidden_selection_units,
        }
    }

    fn fill_rect(
        &mut self,
        rect: ClipRect,
        background: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        if background == Background::Default && selection_background.is_none() {
            return;
        }

        let Some(bounds) = clip.clip_rect(rect) else {
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
                    foreground: Background::Default,
                    selection_background,
                    selection_order: None,
                    reversed: false,
                    wide_continuation: false,
                };
            }
        }
    }

    fn clear_chunky_rounded_corners(&mut self, rect: ClipRect, style: &DivStyle, clip: ClipBounds) {
        if !has_chunky_rounded_corner(style) {
            return;
        }

        let left = rect.left;
        let right = rect.right - 1;
        let top = rect.top;
        let bottom = rect.bottom - 1;
        if left > right || top > bottom {
            return;
        }

        if style.border_top == BorderStyle::ChunkyRounded
            && style.border_left == BorderStyle::ChunkyRounded
        {
            self.clear_cell(left, top, clip);
        }
        if style.border_top == BorderStyle::ChunkyRounded
            && style.border_right == BorderStyle::ChunkyRounded
            && right != left
        {
            self.clear_cell(right, top, clip);
        }
        if style.border_bottom == BorderStyle::ChunkyRounded
            && style.border_left == BorderStyle::ChunkyRounded
            && bottom != top
        {
            self.clear_cell(left, bottom, clip);
        }
        if style.border_bottom == BorderStyle::ChunkyRounded
            && style.border_right == BorderStyle::ChunkyRounded
            && right != left
            && bottom != top
        {
            self.clear_cell(right, bottom, clip);
        }
    }

    fn clear_cell(&mut self, x: i32, y: i32, clip: ClipBounds) {
        if x < 0
            || y < 0
            || x >= self.width as i32
            || y >= self.height as i32
            || !clip.contains(x, y)
        {
            return;
        }

        self.cells[y as usize * self.width + x as usize] = Cell::default();
    }

    fn write_image(
        &mut self,
        node: &ImageNode,
        rect: ClipRect,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        let Some(image) = node.image.as_ref() else {
            return;
        };
        let Some(bounds) = clip.clip_rect(rect) else {
            return;
        };

        if bounds.width() <= 0 || bounds.height() <= 0 {
            return;
        }

        match node.style.image_rendering {
            ImageRendering::Ascii => {
                self.write_ascii_image_data(image, rect, bounds, selection_background);
            }
            ImageRendering::HalfBlock => {
                self.write_half_block_image_data(image, rect, bounds, selection_background);
            }
        }
    }

    fn write_ascii_image_data(
        &mut self,
        image: &ImageData,
        rect: ClipRect,
        bounds: ClipRect,
        selection_background: Option<Background>,
    ) {
        let rect_width = rect.width().max(1) as u32;
        let rect_height = rect.height().max(1) as u32;
        for y in bounds.top..bounds.bottom {
            for x in bounds.left..bounds.right {
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }

                let local_x = (x - rect.left).max(0) as u32;
                let local_y = (y - rect.top).max(0) as u32;
                let source_x = (local_x.saturating_mul(image.width_px) / rect_width)
                    .min(image.width_px.saturating_sub(1));
                let source_y = (local_y.saturating_mul(image.height_px) / rect_height)
                    .min(image.height_px.saturating_sub(1));
                let pixel_index =
                    (source_y as usize * image.width_px as usize + source_x as usize) * 3;
                let Some(pixel) = image.rgb.get(pixel_index..pixel_index + 3) else {
                    continue;
                };

                let red = pixel[0];
                let green = pixel[1];
                let blue = pixel[2];
                let index = y as usize * self.width + x as usize;
                self.cells[index].character = ascii_pixel_char(red, green, blue);
                self.cells[index].foreground = Background::Rgb(red, green, blue);
                self.cells[index].background = Background::Default;
                self.cells[index].selection_order = None;
                self.cells[index].wide_continuation = false;
                if selection_background.is_some() {
                    self.cells[index].selection_background = selection_background;
                }
            }
        }
    }

    fn write_half_block_image_data(
        &mut self,
        image: &ImageData,
        rect: ClipRect,
        bounds: ClipRect,
        selection_background: Option<Background>,
    ) {
        let rect_width = rect.width().max(1) as u32;
        let virtual_height = (rect.height().max(1) as u32).saturating_mul(2);
        for y in bounds.top..bounds.bottom {
            for x in bounds.left..bounds.right {
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }

                let local_x = (x - rect.left).max(0) as u32;
                let local_y = (y - rect.top).max(0) as u32;
                let source_x = (local_x.saturating_mul(image.width_px) / rect_width)
                    .min(image.width_px.saturating_sub(1));
                let top_y = (local_y.saturating_mul(2).saturating_mul(image.height_px)
                    / virtual_height)
                    .min(image.height_px.saturating_sub(1));
                let bottom_y = ((local_y.saturating_mul(2).saturating_add(1))
                    .saturating_mul(image.height_px)
                    / virtual_height)
                    .min(image.height_px.saturating_sub(1));

                let Some(top_pixel) = image_pixel(image, source_x, top_y) else {
                    continue;
                };
                let Some(bottom_pixel) = image_pixel(image, source_x, bottom_y) else {
                    continue;
                };

                let index = y as usize * self.width + x as usize;
                self.cells[index].character = '▄';
                self.cells[index].background =
                    Background::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]);
                self.cells[index].foreground =
                    Background::Rgb(bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);
                self.cells[index].selection_order = None;
                self.cells[index].wide_continuation = false;
                if selection_background.is_some() {
                    self.cells[index].selection_background = selection_background;
                }
            }
        }
    }

    fn write_text(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        foreground: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        let layout = TextLayout::new(text, None, TextWrapMode::Word);
        for glyph in layout.glyphs {
            self.write_glyph(
                x + glyph.col as i32,
                y + glyph.row as i32,
                glyph.character,
                glyph.width,
                Background::Default,
                foreground,
                selection_background,
                clip,
            );
        }
    }

    fn write_input(
        &mut self,
        rect: ClipRect,
        value: &str,
        cursor: u32,
        focused: bool,
        foreground: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        let Some(bounds) = clip.clip_rect(rect) else {
            return;
        };
        let width = rect.width().max(0) as usize;
        if width == 0 || rect.height() <= 0 {
            return;
        }

        let chars = value.chars().collect::<Vec<_>>();
        let cursor = (cursor as usize).min(chars.len());
        let start = if focused && cursor >= width {
            cursor + 1 - width
        } else {
            0
        };
        let cursor_col = focused.then_some(cursor.saturating_sub(start));

        for y in bounds.top..bounds.bottom {
            for x in bounds.left..bounds.right {
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }

                let local_col = (x - rect.left).max(0) as usize;
                let char_index = start + local_col;
                let character = chars.get(char_index).copied().unwrap_or(' ');
                let index = y as usize * self.width + x as usize;
                self.cells[index].character = character;
                self.cells[index].foreground = foreground;
                self.cells[index].selection_order = None;
                self.cells[index].wide_continuation = false;
                if let Some(order) =
                    (char_index < chars.len()).then(|| self.push_selection_unit(y, character))
                {
                    self.cells[index].selection_order = Some(order);
                }
                if selection_background.is_some() {
                    self.cells[index].selection_background = selection_background;
                }
                if focused && y == rect.top && cursor_col == Some(local_col) {
                    self.cells[index].reversed = true;
                }
            }
        }
    }

    fn write_textarea(
        &mut self,
        rect: ClipRect,
        value: &str,
        cursor: u32,
        focused: bool,
        foreground: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        let Some(bounds) = clip.clip_rect(rect) else {
            return;
        };
        if rect.width() <= 0 || rect.height() <= 0 {
            return;
        }

        let layout = TextLayout::new(
            value,
            Some(rect.width().max(1) as usize),
            TextWrapMode::Word,
        );
        let (cursor_row, cursor_col) = layout.cursor_position(cursor as usize);
        for y in bounds.top..bounds.bottom {
            for x in bounds.left..bounds.right {
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }

                let local_row = (y - rect.top).max(0) as usize;
                let local_col = (x - rect.left).max(0) as usize;
                let index = y as usize * self.width + x as usize;
                self.cells[index].character = ' ';
                self.cells[index].foreground = foreground;
                self.cells[index].selection_order = None;
                self.cells[index].wide_continuation = false;
                if selection_background.is_some() {
                    self.cells[index].selection_background = selection_background;
                }
                if focused && local_row == cursor_row && local_col == cursor_col {
                    self.cells[index].reversed = true;
                }
            }
        }

        for glyph in layout.glyphs {
            let x = rect.left + glyph.col as i32;
            let y = rect.top + glyph.row as i32;
            if x < bounds.left
                || y < bounds.top
                || x >= bounds.right
                || y >= bounds.bottom
                || x < 0
                || y < 0
                || x >= self.width as i32
                || y >= self.height as i32
            {
                continue;
            }

            let index = y as usize * self.width + x as usize;
            self.cells[index].character = glyph.character;
            self.cells[index].foreground = foreground;
            self.cells[index].selection_order = Some(self.push_selection_unit(y, glyph.character));
            self.cells[index].wide_continuation = false;
            if selection_background.is_some() {
                self.cells[index].selection_background = selection_background;
            }

            for offset in 1..glyph.width {
                let continuation_x = x + offset as i32;
                if continuation_x < bounds.left
                    || continuation_x >= bounds.right
                    || continuation_x < 0
                    || continuation_x >= self.width as i32
                {
                    continue;
                }
                let continuation_index = y as usize * self.width + continuation_x as usize;
                self.cells[continuation_index].character = ' ';
                self.cells[continuation_index].foreground = foreground;
                self.cells[continuation_index].selection_order = None;
                self.cells[continuation_index].wide_continuation = true;
                if selection_background.is_some() {
                    self.cells[continuation_index].selection_background = selection_background;
                }
            }
        }
    }

    fn write_glyph(
        &mut self,
        x: i32,
        y: i32,
        character: char,
        width: usize,
        background: Background,
        foreground: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        if width == 0 {
            return;
        }

        let visible = x >= 0
            && y >= 0
            && x < self.width as i32
            && y < self.height as i32
            && clip.contains(x, y);
        if !visible && !self.capture_hidden_selection_units {
            return;
        }

        let selection_order = self.push_selection_unit(y, character);
        if !visible {
            return;
        }

        let index = y as usize * self.width + x as usize;
        self.cells[index].character = character;
        self.cells[index].foreground = foreground;
        self.cells[index].selection_order = Some(selection_order);
        self.cells[index].wide_continuation = false;
        if background != Background::Default {
            self.cells[index].background = background;
        }
        if selection_background.is_some() {
            self.cells[index].selection_background = selection_background;
        }

        for offset in 1..width {
            let continuation_x = x + offset as i32;
            if continuation_x < 0
                || continuation_x >= self.width as i32
                || !clip.contains(continuation_x, y)
            {
                continue;
            }

            let continuation_index = y as usize * self.width + continuation_x as usize;
            self.cells[continuation_index].character = ' ';
            self.cells[continuation_index].foreground = foreground;
            self.cells[continuation_index].selection_order = None;
            self.cells[continuation_index].wide_continuation = true;
            if background != Background::Default {
                self.cells[continuation_index].background = background;
            }
            if selection_background.is_some() {
                self.cells[continuation_index].selection_background = selection_background;
            }
        }
    }

    fn stroke_border(
        &mut self,
        rect: ClipRect,
        style: &DivStyle,
        border_color: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        if !has_border(style) || rect.left >= rect.right || rect.top >= rect.bottom {
            return;
        }

        let left = rect.left;
        let right = rect.right - 1;
        let top = rect.top;
        let bottom = rect.bottom - 1;

        if style.border_top != BorderStyle::None {
            for x in left..=right {
                self.write_border_cell(
                    x,
                    top,
                    border_char_at(style, x == left, x == right, true, false),
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }

        if style.border_bottom != BorderStyle::None && bottom != top {
            for x in left..=right {
                self.write_border_cell(
                    x,
                    bottom,
                    border_char_at(style, x == left, x == right, false, true),
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }

        if style.border_left != BorderStyle::None {
            let start = if style.border_top == BorderStyle::None {
                top
            } else {
                top + 1
            };
            let end = if style.border_bottom == BorderStyle::None {
                bottom
            } else {
                bottom - 1
            };
            for y in start..=end {
                self.write_border_cell(
                    left,
                    y,
                    border_glyphs(style.border_left).vertical,
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }

        if style.border_right != BorderStyle::None && right != left {
            let start = if style.border_top == BorderStyle::None {
                top
            } else {
                top + 1
            };
            let end = if style.border_bottom == BorderStyle::None {
                bottom
            } else {
                bottom - 1
            };
            for y in start..=end {
                self.write_border_cell(
                    right,
                    y,
                    border_glyphs(style.border_right).vertical,
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }
    }

    fn write_border_cell(
        &mut self,
        x: i32,
        y: i32,
        character: char,
        foreground: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        if x < 0
            || y < 0
            || x >= self.width as i32
            || y >= self.height as i32
            || !clip.contains(x, y)
        {
            return;
        }

        let index = y as usize * self.width + x as usize;
        self.cells[index].character = character;
        self.cells[index].foreground = foreground;
        self.cells[index].selection_order = None;
        self.cells[index].wide_continuation = false;
        if selection_background.is_some() {
            self.cells[index].selection_background = selection_background;
        }
    }

    fn write_diff_to_stdout(
        &self,
        previous: Option<&Frame>,
        color_profile: TermProfile,
    ) -> io::Result<()> {
        let mut out = io::stdout().lock();
        write_synchronized_output_begin(&mut out)?;
        let result: io::Result<()> = (|| {
            write!(out, "\x1b[?25l")?;

            let Some(previous) = previous else {
                self.write_full_to(&mut out, color_profile)?;
                return Ok(());
            };

            if previous.width != self.width || previous.height != self.height {
                write!(out, "\x1b[2J")?;
                self.write_full_to(&mut out, color_profile)?;
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

                    self.write_span_to(&mut out, row, start, col, color_profile)?;
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

    fn write_full_to(&self, out: &mut impl Write, color_profile: TermProfile) -> io::Result<()> {
        write!(out, "\x1b[H")?;

        for row in 0..self.height {
            self.write_span_to(out, row, 0, self.width, color_profile)?;
        }

        Ok(())
    }

    fn write_span_to(
        &self,
        out: &mut impl Write,
        row: usize,
        start_col: usize,
        end_col: usize,
        color_profile: TermProfile,
    ) -> io::Result<()> {
        if start_col >= end_col {
            return Ok(());
        }

        write!(out, "\x1b[{};{}H", row + 1, start_col + 1)?;

        let mut current_background = Background::Default;
        let mut current_foreground = Background::Default;
        let mut current_reversed = false;
        for col in start_col..end_col {
            let cell = self.cells[row * self.width + col];
            if cell.wide_continuation {
                continue;
            }
            let background = cell.background;
            let foreground = cell.foreground;
            if cell.reversed != current_reversed {
                if cell.reversed {
                    write!(out, "\x1b[7m")?;
                } else {
                    write!(out, "\x1b[27m")?;
                }
                current_reversed = cell.reversed;
            }
            if background != current_background {
                write!(out, "{}", background.ansi_bg(color_profile))?;
                current_background = background;
            }
            if foreground != current_foreground {
                write!(out, "{}", foreground.ansi_fg(color_profile))?;
                current_foreground = foreground;
            }
            write!(out, "{}", cell.character)?;
        }

        write!(out, "\x1b[27m\x1b[39m\x1b[49m")
    }
}

fn load_png_image(src: &str) -> Result<ImageData, ()> {
    let bytes = fs::read(src).map_err(|_| ())?;
    let mut decoder = png::Decoder::new(Cursor::new(&bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| ())?;
    let mut decoded = vec![0; reader.output_buffer_size().ok_or(())?];
    let output = reader.next_frame(&mut decoded).map_err(|_| ())?;
    if output.width == 0 || output.height == 0 {
        return Err(());
    }
    let rgb = decoded_png_to_rgb(&decoded[..output.buffer_size()], output.color_type)?;

    Ok(ImageData {
        width_px: output.width,
        height_px: output.height,
        rgb,
    })
}

fn decoded_png_to_rgb(bytes: &[u8], color_type: png::ColorType) -> Result<Vec<u8>, ()> {
    let mut rgb = Vec::new();
    match color_type {
        png::ColorType::Rgb => {
            rgb.extend_from_slice(bytes);
        }
        png::ColorType::Rgba => {
            rgb.reserve(bytes.len() / 4 * 3);
            for pixel in bytes.chunks_exact(4) {
                rgb.extend_from_slice(&pixel[..3]);
            }
        }
        png::ColorType::Grayscale => {
            rgb.reserve(bytes.len() * 3);
            for value in bytes {
                rgb.extend_from_slice(&[*value, *value, *value]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            rgb.reserve(bytes.len() / 2 * 3);
            for pixel in bytes.chunks_exact(2) {
                let value = pixel[0];
                rgb.extend_from_slice(&[value, value, value]);
            }
        }
        png::ColorType::Indexed => return Err(()),
    }
    Ok(rgb)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cell {
    background: Background,
    character: char,
    foreground: Background,
    selection_background: Option<Background>,
    selection_order: Option<usize>,
    reversed: bool,
    wide_continuation: bool,
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
            foreground: Background::Default,
            selection_background: None,
            selection_order: None,
            reversed: false,
            wide_continuation: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TextMetrics {
    width: usize,
    height: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct InlineMetricsKey {
    id: u32,
    width: u32,
}

#[derive(Clone, Copy)]
struct InlineMetrics {
    width: u32,
    height: u32,
}

fn measure_text(text: &str) -> TextMetrics {
    let layout = TextLayout::new(text, None, TextWrapMode::Word);

    TextMetrics {
        width: layout.width,
        height: layout.height,
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
        Some(DomNode::Image(node)) if node.style.display == LayoutDisplay::Inline => {
            measure_inline_image(node, cursor);
        }
        Some(DomNode::Input(node)) if node.style.display == LayoutDisplay::Inline => {
            measure_inline_input(node, cursor);
        }
        Some(DomNode::TextArea(node)) if node.style.display == LayoutDisplay::Inline => {
            measure_inline_textarea(node, cursor);
        }
        _ => {}
    }
}

fn measure_inline_text(text: &str, cursor: &mut InlineMeasureCursor) {
    let layout = TextLayout::new_with_start_col(
        text,
        Some(cursor.width.max(1) as usize),
        cursor.col as usize,
        TextWrapMode::Word,
    );
    cursor.max_col = cursor.max_col.max(layout.width as u32);
    cursor.row += layout.end_position().0 as u32;
    cursor.col = layout.end_position().1 as u32;
}

fn measure_inline_image(node: &ImageNode, cursor: &mut InlineMeasureCursor) {
    let (width, height) = image_cell_size(node, query_terminal_size());
    if width == 0 || height == 0 {
        return;
    }
    if cursor.col + width > cursor.width {
        cursor.max_col = cursor.max_col.max(cursor.col);
        cursor.col = 0;
        cursor.row += 1;
    }

    cursor.col += width;
    cursor.max_col = cursor.max_col.max(cursor.col);
    if height > 1 {
        cursor.row += height - 1;
    }
}

fn measure_inline_input(node: &InputNode, cursor: &mut InlineMeasureCursor) {
    let (width, height) = input_cell_size(node);
    if cursor.col + width > cursor.width {
        cursor.max_col = cursor.max_col.max(cursor.col);
        cursor.col = 0;
        cursor.row += 1;
    }

    cursor.col += width;
    cursor.max_col = cursor.max_col.max(cursor.col);
    if height > 1 {
        cursor.row += height - 1;
    }
}

fn measure_inline_textarea(node: &TextAreaNode, cursor: &mut InlineMeasureCursor) {
    let (width, height) = textarea_cell_size(node);
    if cursor.col + width > cursor.width {
        cursor.max_col = cursor.max_col.max(cursor.col);
        cursor.col = 0;
        cursor.row += 1;
    }

    cursor.col += width;
    cursor.max_col = cursor.max_col.max(cursor.col);
    if height > 1 {
        cursor.row += height - 1;
    }
}

pub(crate) fn renderer_loop(
    rx: Receiver<RenderCommand>,
    transition_events: Arc<Mutex<VecDeque<TransitionEvent>>>,
) {
    let mut renderer = Renderer::new(transition_events);

    loop {
        let command = if renderer.has_active_transitions() {
            match rx.recv_timeout(Duration::from_millis(16)) {
                Ok(command) => command,
                Err(RecvTimeoutError::Timeout) => {
                    renderer.render();
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(command) => command,
                Err(_) => break,
            }
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_renderer() -> Renderer {
        Renderer::new(Arc::new(Mutex::new(VecDeque::new())))
    }

    fn apply(renderer: &mut Renderer, command: RenderCommand) {
        assert!(renderer.apply(command));
    }

    fn div(renderer: &mut Renderer, id: u32) {
        apply(renderer, RenderCommand::CreateDiv { id });
    }

    fn textarea(renderer: &mut Renderer, id: u32) {
        apply(renderer, RenderCommand::CreateTextArea { id });
    }

    fn append(renderer: &mut Renderer, parent: u32, child: u32) {
        apply(renderer, RenderCommand::AppendChild { parent, child });
    }

    fn rgb(value: Background) -> Option<(u8, u8, u8)> {
        match value {
            Background::Rgb(red, green, blue) => Some((red, green, blue)),
            _ => None,
        }
    }

    fn set_width(renderer: &mut Renderer, id: u32, width: CssDimension) {
        apply(renderer, RenderCommand::SetWidth { id, width });
    }

    fn set_height(renderer: &mut Renderer, id: u32, height: CssDimension) {
        apply(renderer, RenderCommand::SetHeight { id, height });
    }

    fn set_min_height(renderer: &mut Renderer, id: u32, min_height: CssDimension) {
        apply(renderer, RenderCommand::SetMinHeight { id, min_height });
    }

    fn set_display(renderer: &mut Renderer, id: u32, display: LayoutDisplay) {
        apply(renderer, RenderCommand::SetDisplay { id, display });
    }

    fn set_flex_direction(renderer: &mut Renderer, id: u32, direction: LayoutFlexDirection) {
        apply(renderer, RenderCommand::SetFlexDirection { id, direction });
    }

    fn compute_layout(renderer: &mut Renderer, root: u32, width: f32, height: f32) {
        let root = renderer.taffy_ids[&root];
        renderer
            .taffy
            .compute_layout(
                root,
                Size {
                    width: AvailableSpace::Definite(width),
                    height: AvailableSpace::Definite(height),
                },
            )
            .unwrap();
    }

    fn set_scroll_top_direct(renderer: &mut Renderer, id: u32, scroll_top: u32) {
        match renderer.nodes.get_mut(&id).unwrap() {
            DomNode::Div(node) => node.scroll_top = scroll_top,
            DomNode::Span(node) => node.scroll_top = scroll_top,
            _ => panic!("node is not scrollable"),
        }
    }

    fn collect_metrics_for(
        renderer: &mut Renderer,
        root: u32,
        viewport: u32,
        width: f32,
        height: f32,
    ) -> ScrollMetrics {
        compute_layout(renderer, root, width, height);
        let mut metrics = HashMap::new();
        renderer.collect_scroll_metrics(root, &mut metrics);
        metrics.remove(&viewport).unwrap()
    }

    #[test]
    fn bordered_scroll_container_uses_content_box_for_scroll_metrics() {
        let mut renderer = test_renderer();

        div(&mut renderer, 1);
        set_display(&mut renderer, 1, LayoutDisplay::Flex);
        set_flex_direction(&mut renderer, 1, LayoutFlexDirection::Column);
        set_width(&mut renderer, 1, CssDimension::Length(20.0));
        set_height(&mut renderer, 1, CssDimension::Length(10.0));
        apply(&mut renderer, RenderCommand::SetRoot { id: 1 });

        div(&mut renderer, 2);
        set_width(&mut renderer, 2, CssDimension::Length(20.0));
        set_height(&mut renderer, 2, CssDimension::Length(10.0));
        apply(
            &mut renderer,
            RenderCommand::SetOverflowY {
                id: 2,
                overflow: LayoutOverflow::Scroll,
            },
        );
        apply(
            &mut renderer,
            RenderCommand::SetOverflowX {
                id: 2,
                overflow: LayoutOverflow::Hidden,
            },
        );
        apply(
            &mut renderer,
            RenderCommand::SetBorder {
                id: 2,
                style: BorderStyle::Rounded,
            },
        );
        append(&mut renderer, 1, 2);

        div(&mut renderer, 3);
        set_display(&mut renderer, 3, LayoutDisplay::Flex);
        set_flex_direction(&mut renderer, 3, LayoutFlexDirection::Column);
        set_width(&mut renderer, 3, CssDimension::Length(18.0));
        append(&mut renderer, 2, 3);

        for id in 4..20 {
            div(&mut renderer, id);
            set_width(&mut renderer, id, CssDimension::Length(18.0));
            set_height(&mut renderer, id, CssDimension::Length(1.0));
            append(&mut renderer, 3, id);
        }

        let viewport_metrics = collect_metrics_for(&mut renderer, 1, 2, 20.0, 10.0);

        assert_eq!(viewport_metrics.client_height, 8);
        assert_eq!(viewport_metrics.scroll_height, 16);
        assert_eq!(max_scroll_top(&viewport_metrics), 8);
    }

    #[test]
    fn scroll_metrics_query_computes_metrics_before_first_render() {
        let mut renderer = test_renderer();
        build_demo_shaped_scroll_tree(&mut renderer);

        let (tx, rx) = crossbeam_channel::bounded(1);
        apply(
            &mut renderer,
            RenderCommand::GetScrollMetrics {
                id: 4,
                response: tx,
            },
        );
        let metrics = rx.recv().unwrap().unwrap();

        assert!(metrics.client_height > 0, "{metrics:?}");
        assert!(metrics.scroll_height > metrics.client_height, "{metrics:?}");
    }

    #[test]
    fn demo_shaped_scroll_container_has_nonzero_scroll_metrics() {
        let mut renderer = test_renderer();
        build_demo_shaped_scroll_tree(&mut renderer);

        let metrics = collect_metrics_for(&mut renderer, 1, 4, 80.0, 24.0);

        assert_eq!(metrics.client_height, 20);
        assert!(metrics.scroll_height > metrics.client_height, "{metrics:?}");
        assert!(max_scroll_top(&metrics) > 0, "{metrics:?}");
    }

    #[test]
    fn image_rendering_defaults_to_half_block() {
        let mut frame = Frame::new(1, 1, false);
        let node = ImageNode {
            style: DivStyle::default(),
            src: None,
            image: Some(ImageData {
                width_px: 1,
                height_px: 2,
                rgb: vec![255, 0, 0, 0, 0, 255],
            }),
        };

        frame.write_image(
            &node,
            ClipRect::new(0, 0, 1, 1),
            None,
            ClipBounds::unbounded(),
        );

        let cell = frame.cells[0];
        assert_eq!(cell.character, '▄');
        assert_eq!(rgb(cell.background), Some((255, 0, 0)));
        assert_eq!(rgb(cell.foreground), Some((0, 0, 255)));
    }

    #[test]
    fn image_rendering_can_use_ascii() {
        let mut frame = Frame::new(1, 1, false);
        let mut style = DivStyle::default();
        style.image_rendering = ImageRendering::Ascii;
        let node = ImageNode {
            style,
            src: None,
            image: Some(ImageData {
                width_px: 1,
                height_px: 2,
                rgb: vec![255, 0, 0, 0, 0, 255],
            }),
        };

        frame.write_image(
            &node,
            ClipRect::new(0, 0, 1, 1),
            None,
            ClipBounds::unbounded(),
        );

        let cell = frame.cells[0];
        assert_ne!(cell.character, '▄');
        assert!(matches!(cell.background, Background::Default));
        assert_eq!(rgb(cell.foreground), Some((255, 0, 0)));
    }

    #[test]
    fn focused_input_renders_value_and_cursor() {
        let mut frame = Frame::new(4, 1, false);
        frame.write_input(
            ClipRect::new(0, 0, 4, 1),
            "abc",
            1,
            true,
            Background::Rgb(255, 255, 255),
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cells[0].character, 'a');
        assert_eq!(frame.cells[1].character, 'b');
        assert!(frame.cells[1].reversed);
        assert_eq!(frame.cells[2].character, 'c');
        assert!(!frame.cells[2].reversed);
    }

    #[test]
    fn focused_input_scrolls_value_to_cursor() {
        let mut frame = Frame::new(3, 1, false);
        frame.write_input(
            ClipRect::new(0, 0, 3, 1),
            "abcd",
            4,
            true,
            Background::Rgb(255, 255, 255),
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cells[0].character, 'c');
        assert_eq!(frame.cells[1].character, 'd');
        assert_eq!(frame.cells[2].character, ' ');
        assert!(frame.cells[2].reversed);
    }

    #[test]
    fn focused_textarea_renders_multiline_value_and_cursor() {
        let mut frame = Frame::new(3, 2, false);
        frame.write_textarea(
            ClipRect::new(0, 0, 3, 2),
            "ab\ncd",
            4,
            true,
            Background::Rgb(255, 255, 255),
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cells[0].character, 'a');
        assert_eq!(frame.cells[1].character, 'b');
        assert_eq!(frame.cells[3].character, 'c');
        assert_eq!(frame.cells[4].character, 'd');
        assert!(frame.cells[4].reversed);
    }

    #[test]
    fn textarea_word_wraps_long_lines() {
        let mut frame = Frame::new(5, 2, false);
        frame.write_textarea(
            ClipRect::new(0, 0, 5, 2),
            "hello world",
            6,
            true,
            Background::Rgb(255, 255, 255),
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cells[0].character, 'h');
        assert_eq!(frame.cells[4].character, 'o');
        assert_eq!(frame.cells[5].character, 'w');
        assert_eq!(frame.cells[9].character, 'd');
        assert!(frame.cells[5].reversed);
    }

    #[test]
    fn inline_text_uses_word_wrapping_layout() {
        let mut cursor = InlineMeasureCursor {
            col: 0,
            row: 0,
            width: 5,
            max_col: 0,
        };

        measure_inline_text("hello world", &mut cursor);

        assert_eq!(cursor.row, 1);
        assert_eq!(cursor.col, 5);
        assert_eq!(cursor.max_col, 5);
    }

    #[test]
    fn inline_text_uses_unicode_cell_width() {
        let mut cursor = InlineMeasureCursor {
            col: 0,
            row: 0,
            width: 2,
            max_col: 0,
        };

        measure_inline_text("界a", &mut cursor);

        assert_eq!(cursor.row, 1);
        assert_eq!(cursor.col, 1);
        assert_eq!(cursor.max_col, 2);
    }

    #[test]
    fn inline_text_paints_from_shared_layout() {
        let mut frame = Frame::new(5, 2, false);
        let mut cursor = InlineCursor {
            x: 0,
            y: 0,
            col: 0,
            row: 0,
            width: 5,
        };
        let mut hit_regions = Vec::new();

        write_inline_text(
            "hello world",
            &mut cursor,
            Background::Default,
            &mut frame,
            &mut hit_regions,
            None,
            None,
            Background::Rgb(255, 255, 255),
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cells[0].character, 'h');
        assert_eq!(frame.cells[4].character, 'o');
        assert_eq!(frame.cells[5].character, 'w');
        assert_eq!(frame.cells[9].character, 'd');
        assert_eq!(cursor.row, 1);
        assert_eq!(cursor.col, 5);
    }

    #[test]
    fn textarea_wraps_using_unicode_cell_width() {
        let mut frame = Frame::new(2, 2, false);
        frame.write_textarea(
            ClipRect::new(0, 0, 2, 2),
            "界a",
            0,
            false,
            Background::Rgb(255, 255, 255),
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cells[0].character, '界');
        assert!(frame.cells[1].wide_continuation);
        assert_eq!(frame.cells[2].character, 'a');
    }

    #[test]
    fn textarea_auto_height_tracks_newlines() {
        let mut renderer = test_renderer();

        div(&mut renderer, 1);
        set_display(&mut renderer, 1, LayoutDisplay::Flex);
        set_flex_direction(&mut renderer, 1, LayoutFlexDirection::Column);
        set_width(&mut renderer, 1, CssDimension::Length(20.0));
        apply(&mut renderer, RenderCommand::SetRoot { id: 1 });

        textarea(&mut renderer, 2);
        apply(
            &mut renderer,
            RenderCommand::SetInputValue {
                id: 2,
                value: "alpha\nbeta\ngamma".to_string(),
                cursor: 0,
            },
        );
        append(&mut renderer, 1, 2);

        compute_layout(&mut renderer, 1, 20.0, 10.0);
        let textarea_layout = renderer.taffy.layout(renderer.taffy_ids[&2]).unwrap();

        assert_eq!(textarea_layout.size.width, 5.0);
        assert_eq!(textarea_layout.size.height, 3.0);
    }

    #[test]
    fn textarea_auto_height_tracks_word_wrapping_for_fixed_width() {
        let mut renderer = test_renderer();

        div(&mut renderer, 1);
        set_width(&mut renderer, 1, CssDimension::Length(20.0));
        apply(&mut renderer, RenderCommand::SetRoot { id: 1 });

        textarea(&mut renderer, 2);
        set_width(&mut renderer, 2, CssDimension::Length(5.0));
        apply(
            &mut renderer,
            RenderCommand::SetInputValue {
                id: 2,
                value: "hello world".to_string(),
                cursor: 0,
            },
        );
        append(&mut renderer, 1, 2);

        compute_layout(&mut renderer, 1, 20.0, 10.0);
        let textarea_layout = renderer.taffy.layout(renderer.taffy_ids[&2]).unwrap();

        assert_eq!(textarea_layout.size.width, 5.0);
        assert_eq!(textarea_layout.size.height, 2.0);
    }

    #[test]
    fn min_height_expands_auto_sized_textarea() {
        let mut renderer = test_renderer();

        div(&mut renderer, 1);
        set_width(&mut renderer, 1, CssDimension::Length(20.0));
        apply(&mut renderer, RenderCommand::SetRoot { id: 1 });

        textarea(&mut renderer, 2);
        set_min_height(&mut renderer, 2, CssDimension::Length(5.0));
        apply(
            &mut renderer,
            RenderCommand::SetInputValue {
                id: 2,
                value: "short".to_string(),
                cursor: 0,
            },
        );
        append(&mut renderer, 1, 2);

        compute_layout(&mut renderer, 1, 20.0, 10.0);
        let textarea_layout = renderer.taffy.layout(renderer.taffy_ids[&2]).unwrap();

        assert_eq!(textarea_layout.size.height, 5.0);
    }

    #[test]
    fn scrolling_content_is_clipped_inside_border() {
        let mut renderer = test_renderer();

        div(&mut renderer, 1);
        set_width(&mut renderer, 1, CssDimension::Length(8.0));
        set_height(&mut renderer, 1, CssDimension::Length(5.0));
        apply(&mut renderer, RenderCommand::SetRoot { id: 1 });

        div(&mut renderer, 2);
        set_width(&mut renderer, 2, CssDimension::Length(8.0));
        set_height(&mut renderer, 2, CssDimension::Length(5.0));
        apply(
            &mut renderer,
            RenderCommand::SetOverflowY {
                id: 2,
                overflow: LayoutOverflow::Scroll,
            },
        );
        apply(
            &mut renderer,
            RenderCommand::SetBorder {
                id: 2,
                style: BorderStyle::Rounded,
            },
        );
        append(&mut renderer, 1, 2);

        div(&mut renderer, 3);
        set_width(&mut renderer, 3, CssDimension::Length(6.0));
        set_height(&mut renderer, 3, CssDimension::Length(8.0));
        apply(
            &mut renderer,
            RenderCommand::SetBackground {
                id: 3,
                background: Background::Rgb(255, 0, 0),
            },
        );
        append(&mut renderer, 2, 3);

        compute_layout(&mut renderer, 1, 8.0, 5.0);
        set_scroll_top_direct(&mut renderer, 2, 1);

        let mut frame = Frame::new(8, 5, false);
        let mut hit_regions = Vec::new();
        renderer.paint_node(
            1,
            0.0,
            0.0,
            &mut frame,
            &mut hit_regions,
            None,
            Background::Default,
            ClipBounds::unbounded(),
        );

        let top_border = frame.cells[1];
        assert_eq!(top_border.character, '─');
        assert!(
            matches!(top_border.background, Background::Default),
            "top border background should not contain scrolled child background"
        );
    }

    fn build_demo_shaped_scroll_tree(renderer: &mut Renderer) {
        div(renderer, 1);
        set_display(renderer, 1, LayoutDisplay::Flex);
        set_flex_direction(renderer, 1, LayoutFlexDirection::Column);
        set_width(renderer, 1, CssDimension::Percent(1.0));
        set_height(renderer, 1, CssDimension::Percent(1.0));
        apply(renderer, RenderCommand::SetRoot { id: 1 });

        div(renderer, 2);
        set_width(renderer, 2, CssDimension::Percent(1.0));
        set_height(renderer, 2, CssDimension::Length(2.0));
        append(renderer, 1, 2);

        div(renderer, 3);
        set_display(renderer, 3, LayoutDisplay::Flex);
        set_flex_direction(renderer, 3, LayoutFlexDirection::Row);
        set_width(renderer, 3, CssDimension::Percent(1.0));
        apply(
            renderer,
            RenderCommand::SetFlex {
                id: 3,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: CssDimension::Length(0.0),
            },
        );
        append(renderer, 1, 3);

        div(renderer, 4);
        set_width(renderer, 4, CssDimension::Percent(0.8));
        set_height(renderer, 4, CssDimension::Percent(1.0));
        apply(
            renderer,
            RenderCommand::SetOverflowY {
                id: 4,
                overflow: LayoutOverflow::Scroll,
            },
        );
        apply(
            renderer,
            RenderCommand::SetOverflowX {
                id: 4,
                overflow: LayoutOverflow::Hidden,
            },
        );
        apply(
            renderer,
            RenderCommand::SetBorder {
                id: 4,
                style: BorderStyle::Rounded,
            },
        );
        append(renderer, 3, 4);

        div(renderer, 5);
        set_width(renderer, 5, CssDimension::Percent(0.2));
        set_height(renderer, 5, CssDimension::Percent(1.0));
        append(renderer, 3, 5);

        div(renderer, 6);
        set_display(renderer, 6, LayoutDisplay::Flex);
        set_flex_direction(renderer, 6, LayoutFlexDirection::Column);
        set_width(renderer, 6, CssDimension::Percent(1.0));
        apply(
            renderer,
            RenderCommand::SetGap {
                id: 6,
                row_gap: CssLengthPercentage::Length(1.0),
                column_gap: CssLengthPercentage::Length(1.0),
            },
        );
        append(renderer, 4, 6);

        for id in 7..31 {
            div(renderer, id);
            set_width(renderer, id, CssDimension::Percent(1.0));
            set_height(renderer, id, CssDimension::Length(1.0));
            append(renderer, 6, id);
        }
    }
}
