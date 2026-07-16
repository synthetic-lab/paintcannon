use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::{mpsc::Sender as StdSender, OnceLock};
use std::time::Instant;

use crossbeam_channel::Sender;
use taffy::{AvailableSpace, NodeId, Size};
use termprofile::TermProfile;

use crate::frame::Frame;
use crate::image::load_png_image;
use crate::layout::{
    ArenaScrollMetrics, ArenaScrollbarHit, LayoutArena, LayoutNodeKind, ScrollbarAxis,
};
use crate::paint::{paint_arena_with_options, HitRegion, PaintOptions};
use crate::selection::{
    SelectionAction, SelectionMouseEvent, SelectionMouseEventType, SelectionState,
};
use crate::style::{
    Background, BorderStyle, CssDimension, CssFontStyle, CssFontWeight, CssGridLine,
    CssGridPlacement, CssGridTemplateTrack, CssLengthPercentage, CssLengthPercentageAuto,
    CssOverflowWrap, CssPosition, CssTextDecorationLine, CssTrackSizing, CssVisibility,
    CssWhiteSpace, CssWordBreak, CssZIndex, CursorStyle, DivStyle, ImageRendering,
    LayoutAlignItems, LayoutDisplay, LayoutFlexDirection, LayoutFlexWrap, LayoutGridAutoFlow,
    LayoutJustifyContent, LayoutOverflow, ScrollbarColor, ScrollbarGutter, TransitionProperty,
    TransitionSpec,
};
use crate::terminal::{query_terminal_size, write_pointer_shape};
use crate::transition::{TransitionEvent, TransitionEventType, TransitionState};

mod runtime;

pub(crate) use runtime::{engine_loop, EngineLoopOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DomId(pub(crate) u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EngineTransitionEvent {
    pub(crate) event_type: TransitionEventType,
    pub(crate) target: DomId,
    pub(crate) property: TransitionProperty,
}

#[derive(Clone, Debug)]
pub(crate) struct MouseClick {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) button: u32,
    pub(crate) ctrl_key: bool,
    pub(crate) alt_key: bool,
    pub(crate) meta_key: bool,
    pub(crate) shift_key: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ClickEvent {
    pub(crate) target_id: DomId,
    pub(crate) client_x: u32,
    pub(crate) client_y: u32,
    pub(crate) button: u32,
    pub(crate) ctrl_key: bool,
    pub(crate) alt_key: bool,
    pub(crate) meta_key: bool,
    pub(crate) shift_key: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScrollbarHit {
    pub(crate) target_id: DomId,
    pub(crate) axis: ScrollbarAxis,
    pub(crate) rail_start: u32,
    pub(crate) rail_length: u32,
    pub(crate) thumb_start: u32,
    pub(crate) thumb_length: u32,
    pub(crate) scroll_offset: u32,
    pub(crate) max_scroll: u32,
    pub(crate) client_length: u32,
    pub(crate) scroll_length: u32,
}

pub(crate) enum StyleMutation {
    Reset(StyleReset),
    Display(LayoutDisplay),
    Position(CssPosition),
    Top(CssLengthPercentageAuto),
    Right(CssLengthPercentageAuto),
    Bottom(CssLengthPercentageAuto),
    Left(CssLengthPercentageAuto),
    ZIndex(CssZIndex),
    Visibility(CssVisibility),
    Opacity(f32),
    Overflow(LayoutOverflow),
    OverflowX(LayoutOverflow),
    OverflowY(LayoutOverflow),
    ScrollbarColor(ScrollbarColor),
    ScrollbarGutter(ScrollbarGutter),
    ImageRendering(ImageRendering),
    WhiteSpace(CssWhiteSpace),
    OverflowWrap(CssOverflowWrap),
    WordBreak(CssWordBreak),
    FlexDirection(LayoutFlexDirection),
    FlexWrap(LayoutFlexWrap),
    FlexFlow {
        direction: LayoutFlexDirection,
        flex_wrap: LayoutFlexWrap,
    },
    FlexBasis(CssDimension),
    FlexGrow(f32),
    FlexShrink(f32),
    Flex {
        flex_grow: f32,
        flex_shrink: f32,
        flex_basis: CssDimension,
    },
    JustifyContent(LayoutJustifyContent),
    AlignItems(LayoutAlignItems),
    AlignSelf(LayoutAlignItems),
    AlignContent(LayoutJustifyContent),
    JustifyItems(LayoutAlignItems),
    JustifySelf(LayoutAlignItems),
    Gap {
        row_gap: CssLengthPercentage,
        column_gap: CssLengthPercentage,
    },
    RowGap(CssLengthPercentage),
    ColumnGap(CssLengthPercentage),
    Padding {
        top: CssLengthPercentage,
        right: CssLengthPercentage,
        bottom: CssLengthPercentage,
        left: CssLengthPercentage,
    },
    PaddingTop(CssLengthPercentage),
    PaddingRight(CssLengthPercentage),
    PaddingBottom(CssLengthPercentage),
    PaddingLeft(CssLengthPercentage),
    Margin {
        top: CssLengthPercentageAuto,
        right: CssLengthPercentageAuto,
        bottom: CssLengthPercentageAuto,
        left: CssLengthPercentageAuto,
    },
    MarginTop(CssLengthPercentageAuto),
    MarginRight(CssLengthPercentageAuto),
    MarginBottom(CssLengthPercentageAuto),
    MarginLeft(CssLengthPercentageAuto),
    Width(CssDimension),
    Height(CssDimension),
    MinWidth(CssDimension),
    MaxWidth(CssDimension),
    MinHeight(CssDimension),
    MaxHeight(CssDimension),
    Border(BorderStyle),
    BorderTop(BorderStyle),
    BorderRight(BorderStyle),
    BorderBottom(BorderStyle),
    BorderLeft(BorderStyle),
    BorderColor(Background),
    Color(Background),
    PlaceholderColor(Background),
    Background(Background),
    SelectionBackground(Background),
    FontWeight(CssFontWeight),
    FontStyle(CssFontStyle),
    TextDecorationLine(CssTextDecorationLine),
    Cursor(CursorStyle),
    GridTemplateColumns(Vec<CssGridTemplateTrack>),
    GridTemplateRows(Vec<CssGridTemplateTrack>),
    GridAutoColumns(Vec<CssTrackSizing>),
    GridAutoRows(Vec<CssTrackSizing>),
    GridAutoFlow(LayoutGridAutoFlow),
    GridColumn(CssGridLine),
    GridRow(CssGridLine),
    GridColumnStart(CssGridPlacement),
    GridColumnEnd(CssGridPlacement),
    GridRowStart(CssGridPlacement),
    GridRowEnd(CssGridPlacement),
}

pub(crate) enum StyleReset {
    Display,
    Position,
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,
    Visibility,
    Opacity,
    Overflow,
    OverflowX,
    OverflowY,
    ScrollbarColor,
    ScrollbarGutter,
    ImageRendering,
    WhiteSpace,
    OverflowWrap,
    WordBreak,
    FlexDirection,
    FlexWrap,
    FlexFlow,
    FlexBasis,
    FlexGrow,
    FlexShrink,
    Flex,
    JustifyContent,
    AlignItems,
    AlignSelf,
    AlignContent,
    JustifyItems,
    JustifySelf,
    Gap,
    RowGap,
    ColumnGap,
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Width,
    Height,
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
    Border,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderColor,
    Color,
    PlaceholderColor,
    Background,
    SelectionBackground,
    FontWeight,
    FontStyle,
    TextDecorationLine,
    Cursor,
    GridTemplateColumns,
    GridTemplateRows,
    GridAutoColumns,
    GridAutoRows,
    GridAutoFlow,
    GridColumn,
    GridRow,
    GridColumnStart,
    GridColumnEnd,
    GridRowStart,
    GridRowEnd,
}

pub(crate) enum EngineCommand {
    Batch {
        commands: Vec<EngineCommand>,
    },
    #[cfg(test)]
    CreateElement {
        style: DivStyle,
        response: Sender<DomId>,
    },
    CreateElementWithId {
        id: DomId,
        style: DivStyle,
    },
    #[cfg(test)]
    CreateText {
        text: String,
        response: Sender<DomId>,
    },
    CreateTextWithId {
        id: DomId,
        text: String,
    },
    CreateImageWithId {
        id: DomId,
        style: DivStyle,
        width_px: u32,
        height_px: u32,
        cell_width_px: u32,
        cell_height_px: u32,
    },
    CreateInputWithId {
        id: DomId,
        style: DivStyle,
        value: String,
    },
    CreateTextAreaWithId {
        id: DomId,
        style: DivStyle,
        value: String,
    },
    AppendChild {
        parent: DomId,
        child: DomId,
    },
    InsertChildBefore {
        parent: DomId,
        child: DomId,
        before: DomId,
    },
    SetRoot {
        root: DomId,
    },
    SetViewport {
        viewport: DomId,
    },
    DestroyNode {
        node: DomId,
    },
    DetachNode {
        node: DomId,
    },
    MutateStyle {
        node: DomId,
        mutation: StyleMutation,
    },
    SetTransition {
        node: DomId,
        transitions: Vec<TransitionSpec>,
    },
    SetText {
        node: DomId,
        text: String,
    },
    SetImageSource {
        node: DomId,
        src: String,
    },
    SetInputValue {
        node: DomId,
        value: String,
        cursor: u32,
    },
    SetInputFocused {
        node: DomId,
        focused: bool,
    },
    SetInputPlaceholder {
        node: DomId,
        placeholder: String,
    },
    SetTextAreaValue {
        node: DomId,
        value: String,
        cursor: u32,
    },
    SetTextAreaFocused {
        node: DomId,
        focused: bool,
    },
    SetTextAreaPlaceholder {
        node: DomId,
        placeholder: String,
    },
    MoveTextAreaCursorVertically {
        node: DomId,
        direction: i32,
        response: Sender<Option<u32>>,
    },
    GetTextAreaCursorVisualPosition {
        node: DomId,
        response: Sender<Option<(u32, u32)>>,
    },
    GetTextAreaVisualLineRange {
        node: DomId,
        row: u32,
        response: Sender<Option<(u32, u32)>>,
    },
    SetTextControlCursorAtPoint {
        node: DomId,
        x: u32,
        y: u32,
        response: Sender<Option<u32>>,
    },
    SetScrollOffset {
        node: DomId,
        scroll_left: u32,
        scroll_top: u32,
        response: Sender<Option<ArenaScrollMetrics>>,
    },
    GetScrollMetrics {
        node: DomId,
        response: Sender<Option<ArenaScrollMetrics>>,
    },
    HitTestPoint {
        x: u32,
        y: u32,
        response: Sender<Option<DomId>>,
    },
    HitTestClick {
        click: MouseClick,
        response: Sender<Option<ClickEvent>>,
    },
    HitTestScrollbar {
        x: u32,
        y: u32,
        response: Sender<Option<ScrollbarHit>>,
    },
    HandleSelection {
        event: SelectionMouseEvent,
        response: Sender<SelectionAction>,
    },
    HandlePointerMove {
        x: u32,
        y: u32,
    },
    #[cfg(test)]
    RenderFrame {
        width: usize,
        height: usize,
        response: Sender<Option<Frame>>,
    },
    FlushFrame {
        response: StdSender<io::Result<()>>,
    },
    SetRenderSize {
        width: usize,
        height: usize,
    },
    SetFrameRate {
        fps: f64,
    },
    SetTerminalFocused {
        focused: bool,
    },
    InvalidateFrame,
    Shutdown {
        response: Option<Sender<()>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dirtiness {
    Clean,
    Paint,
    Layout,
}

#[derive(Default)]
struct TransitionBatch {
    previous_styles: HashMap<NodeId, DivStyle>,
    style_order: Vec<NodeId>,
    newly_connected: HashSet<NodeId>,
}

pub(crate) struct PaintEngine {
    arena: LayoutArena,
    root: Option<DomId>,
    viewport: Option<DomId>,
    next_dom_id: u32,
    dom_to_node: HashMap<DomId, NodeId>,
    node_to_dom: HashMap<NodeId, DomId>,
    parents: HashMap<DomId, DomId>,
    children: HashMap<DomId, Vec<DomId>>,
    dirtiness: Dirtiness,
    last_layout_size: Option<(usize, usize)>,
    previous_frame: Option<Frame>,
    current_frame: Option<Frame>,
    hit_regions: Vec<HitRegion>,
    selection: SelectionState,
    transitions: TransitionState,
    transition_batch: Option<TransitionBatch>,
    connected_nodes: HashSet<NodeId>,
    established_nodes: HashSet<NodeId>,
    truecolor_enabled: bool,
    terminal_foreground: Background,
    terminal_background: Background,
    terminal_focused: bool,
    current_pointer_shape: Option<&'static str>,
    last_pointer_position: Option<(u32, u32)>,
    scrollbar_selection_suppressed: bool,
    selection_scroll_node: Option<NodeId>,
}

impl PaintEngine {
    pub(crate) fn new() -> Self {
        Self {
            arena: LayoutArena::new(),
            root: None,
            viewport: None,
            next_dom_id: 1,
            dom_to_node: HashMap::new(),
            node_to_dom: HashMap::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
            dirtiness: Dirtiness::Clean,
            last_layout_size: None,
            previous_frame: None,
            current_frame: None,
            hit_regions: Vec::new(),
            selection: SelectionState::default(),
            transitions: TransitionState::default(),
            transition_batch: None,
            connected_nodes: HashSet::new(),
            established_nodes: HashSet::new(),
            truecolor_enabled: true,
            terminal_foreground: Background::White,
            terminal_background: Background::Black,
            terminal_focused: true,
            current_pointer_shape: None,
            last_pointer_position: None,
            scrollbar_selection_suppressed: false,
            selection_scroll_node: None,
        }
    }

    fn mark_layout_dirty(&mut self) {
        self.dirtiness = Dirtiness::Layout;
    }

    fn mark_paint_dirty(&mut self) {
        if self.dirtiness == Dirtiness::Clean {
            self.dirtiness = Dirtiness::Paint;
        }
    }

    fn begin_transition_batch(&mut self) {
        debug_assert!(self.transition_batch.is_none());
        self.transition_batch = Some(TransitionBatch::default());
    }

    fn finish_transition_batch(&mut self, now: Instant) {
        let Some(batch) = self.transition_batch.take() else {
            return;
        };

        for node in batch.style_order {
            let Some(previous) = batch.previous_styles.get(&node) else {
                continue;
            };
            if !self.node_to_dom.contains_key(&node) {
                continue;
            }
            let style = self.arena.style(node).clone();
            let transitions_enabled = self.truecolor_enabled
                && self.established_nodes.contains(&node)
                && self.connected_nodes.contains(&node);
            self.apply_style_transitions(node, previous, &style, now, transitions_enabled);
        }

        self.established_nodes.extend(
            batch
                .newly_connected
                .into_iter()
                .filter(|node| self.connected_nodes.contains(node)),
        );
    }

    #[cfg(test)]
    pub(crate) fn create_element(&mut self, style: DivStyle) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_element(style);
        self.register_node(node)
    }

    fn reserve_for_batch(&mut self, commands: &[EngineCommand]) {
        let mut create_count = 0;
        let mut append_counts = HashMap::new();
        for command in commands {
            match command {
                EngineCommand::CreateElementWithId { .. }
                | EngineCommand::CreateTextWithId { .. }
                | EngineCommand::CreateImageWithId { .. }
                | EngineCommand::CreateInputWithId { .. }
                | EngineCommand::CreateTextAreaWithId { .. } => create_count += 1,
                #[cfg(test)]
                EngineCommand::CreateElement { .. } | EngineCommand::CreateText { .. } => {
                    create_count += 1
                }
                EngineCommand::AppendChild { parent, .. } => {
                    *append_counts.entry(*parent).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        if create_count > 0 {
            self.arena.reserve_nodes(create_count);
            self.dom_to_node.reserve(create_count);
            self.node_to_dom.reserve(create_count);
        }
        if !append_counts.is_empty() {
            let append_count = append_counts.values().sum();
            self.parents.reserve(append_count);
            self.children.reserve(append_counts.len());
            for (parent, child_count) in append_counts {
                self.children
                    .entry(parent)
                    .or_default()
                    .reserve(child_count);
                if let Some(parent_node) = self.node_for(parent) {
                    self.arena.reserve_children(parent_node, child_count);
                }
            }
        }
    }

    pub(crate) fn create_element_with_id(&mut self, id: DomId, style: DivStyle) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_element(style);
        self.register_node_with_id(id, node)
    }

    #[cfg(test)]
    pub(crate) fn create_text(&mut self, text: impl Into<String>) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_text(text);
        self.register_node(node)
    }

    pub(crate) fn create_text_with_id(&mut self, id: DomId, text: impl Into<String>) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_text(text);
        self.register_node_with_id(id, node)
    }

    pub(crate) fn create_image_with_id(
        &mut self,
        id: DomId,
        style: DivStyle,
        width_px: u32,
        height_px: u32,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> DomId {
        self.mark_layout_dirty();
        let node =
            self.arena
                .create_image(style, width_px, height_px, cell_width_px, cell_height_px);
        self.register_node_with_id(id, node)
    }

    pub(crate) fn create_input_with_id(
        &mut self,
        id: DomId,
        style: DivStyle,
        value: impl Into<String>,
    ) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_input(style, value);
        self.register_node_with_id(id, node)
    }

    #[cfg(test)]
    pub(crate) fn create_textarea(&mut self, style: DivStyle, value: impl Into<String>) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_textarea(style, value);
        self.register_node(node)
    }

    pub(crate) fn create_textarea_with_id(
        &mut self,
        id: DomId,
        style: DivStyle,
        value: impl Into<String>,
    ) -> DomId {
        self.mark_layout_dirty();
        let node = self.arena.create_textarea(style, value);
        self.register_node_with_id(id, node)
    }

    pub(crate) fn append_child(&mut self, parent: DomId, child: DomId) -> bool {
        let Some(parent_node) = self.node_for(parent) else {
            return false;
        };
        let Some(child_node) = self.node_for(child) else {
            return false;
        };
        let parent_is_connected = self.connected_nodes.contains(&parent_node);
        let child_was_connected = self.connected_nodes.contains(&child_node);

        if let Some(old_parent) = self.parents.insert(child, parent) {
            if let Some(old_parent_node) = self.node_for(old_parent) {
                self.arena.remove_child(old_parent_node, child_node);
            }
            if let Some(siblings) = self.children.get_mut(&old_parent) {
                siblings.retain(|id| *id != child);
            }
        }

        self.children.entry(parent).or_default().push(child);
        self.mark_layout_dirty();
        self.arena.append_child(parent_node, child_node);
        self.update_reparented_subtree_connectivity(
            child,
            child_was_connected,
            parent_is_connected,
        );
        true
    }

    pub(crate) fn insert_child_before(
        &mut self,
        parent: DomId,
        child: DomId,
        before: DomId,
    ) -> bool {
        let Some(parent_node) = self.node_for(parent) else {
            return false;
        };
        let Some(child_node) = self.node_for(child) else {
            return false;
        };
        let Some(before_node) = self.node_for(before) else {
            return false;
        };
        let parent_is_connected = self.connected_nodes.contains(&parent_node);
        let child_was_connected = self.connected_nodes.contains(&child_node);

        if let Some(old_parent) = self.parents.insert(child, parent) {
            if let Some(old_parent_node) = self.node_for(old_parent) {
                self.arena.remove_child(old_parent_node, child_node);
            }
            if let Some(siblings) = self.children.get_mut(&old_parent) {
                siblings.retain(|id| *id != child);
            }
        }

        let siblings = self.children.entry(parent).or_default();
        siblings.retain(|id| *id != child);
        let index = siblings
            .iter()
            .position(|id| *id == before)
            .unwrap_or(siblings.len());
        siblings.insert(index, child);
        self.mark_layout_dirty();
        self.arena
            .insert_child_before(parent_node, child_node, before_node);
        self.update_reparented_subtree_connectivity(
            child,
            child_was_connected,
            parent_is_connected,
        );
        true
    }

    pub(crate) fn set_root(&mut self, root: DomId) -> bool {
        if self.node_for(root).is_none() {
            return false;
        }
        if self.root == Some(root) {
            self.mark_layout_dirty();
            return true;
        }
        if let Some(previous_root) = self.root {
            self.disconnect_subtree(previous_root);
        }
        self.root = Some(root);
        self.connect_subtree(root);
        self.mark_layout_dirty();
        true
    }

    pub(crate) fn set_viewport(&mut self, viewport: DomId) -> bool {
        let Some(node) = self.node_for(viewport) else {
            return false;
        };
        let mut style = self.arena.style(node).clone();
        style.overflow_x = LayoutOverflow::Hidden;
        style.overflow_y = LayoutOverflow::Hidden;
        self.arena.set_style(node, style);
        self.viewport = Some(viewport);
        self.mark_layout_dirty();
        true
    }

    pub(crate) fn destroy_node(&mut self, node: DomId) -> bool {
        self.destroy_nodes([node]) > 0
    }

    fn destroy_nodes(&mut self, roots: impl IntoIterator<Item = DomId>) -> usize {
        let mut removed = HashSet::new();
        let mut stack = roots
            .into_iter()
            .filter(|node| self.node_for(*node).is_some())
            .collect::<Vec<_>>();
        while let Some(node) = stack.pop() {
            if !removed.insert(node) {
                continue;
            }
            if let Some(children) = self.children.get(&node) {
                stack.extend(children.iter().copied());
            }
        }
        if removed.is_empty() {
            return 0;
        }

        let mut removed_by_parent: HashMap<DomId, HashSet<DomId>> = HashMap::new();
        for node in &removed {
            if let Some(parent) = self.parents.get(node).copied() {
                if !removed.contains(&parent) {
                    removed_by_parent.entry(parent).or_default().insert(*node);
                }
            }
        }
        for (parent, removed_children) in removed_by_parent {
            if let Some(siblings) = self.children.get_mut(&parent) {
                siblings.retain(|child| !removed_children.contains(child));
            }
            if let Some(parent_node) = self.node_for(parent) {
                let removed_child_nodes = removed_children
                    .iter()
                    .filter_map(|child| self.node_for(*child))
                    .collect::<HashSet<_>>();
                self.arena
                    .remove_children(parent_node, &removed_child_nodes);
            }
        }

        let removed_layout_nodes = removed
            .iter()
            .filter_map(|node| self.node_for(*node))
            .collect::<HashSet<_>>();
        for node in &removed_layout_nodes {
            self.connected_nodes.remove(node);
            self.established_nodes.remove(node);
            if let Some(batch) = self.transition_batch.as_mut() {
                batch.newly_connected.remove(node);
            }
        }
        if self
            .selection_scroll_node
            .is_some_and(|node| removed_layout_nodes.contains(&node))
        {
            self.selection_scroll_node = None;
        }

        if self.root.is_some_and(|root| removed.contains(&root)) {
            self.root = None;
        }
        if self
            .viewport
            .is_some_and(|viewport| removed.contains(&viewport))
        {
            self.viewport = None;
        }

        for node in &removed {
            self.parents.remove(node);
            self.children.remove(node);
        }
        for node in &removed {
            if let Some(node_id) = self.dom_to_node.remove(node) {
                self.node_to_dom.remove(&node_id);
                self.transitions.clear_node(node_id);
                self.arena.remove_node(node_id);
            }
        }

        self.mark_layout_dirty();
        removed.len()
    }

    pub(crate) fn detach_node(&mut self, node: DomId) -> bool {
        let Some(node_id) = self.node_for(node) else {
            return false;
        };

        if self.root == Some(node) {
            self.disconnect_subtree(node);
            self.root = None;
            self.mark_layout_dirty();
            return true;
        }

        let Some(parent) = self.parents.remove(&node) else {
            return false;
        };

        if let Some(parent_node) = self.node_for(parent) {
            self.arena.remove_child(parent_node, node_id);
        }
        if let Some(siblings) = self.children.get_mut(&parent) {
            siblings.retain(|id| *id != node);
        }
        self.disconnect_subtree(node);
        self.mark_layout_dirty();
        true
    }

    #[cfg(test)]
    pub(crate) fn set_style(&mut self, node: DomId, style: DivStyle) -> bool {
        self.set_style_at(node, style, Instant::now())
    }

    pub(crate) fn mutate_style(&mut self, node: DomId, mutation: StyleMutation) -> bool {
        self.mutate_style_at(node, mutation, Instant::now())
    }

    fn mutate_style_at(&mut self, node: DomId, mutation: StyleMutation, now: Instant) -> bool {
        let Some(node_id) = self.node_for(node) else {
            return false;
        };
        let mut style = self.arena.style(node_id).clone();
        apply_style_mutation(&mut style, mutation);
        self.set_node_style_at(node_id, style, now);
        self.refresh_pointer_shape();
        true
    }

    #[cfg(test)]
    fn set_style_at(&mut self, node: DomId, style: DivStyle, now: Instant) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.set_node_style_at(node, style, now);
        true
    }

    fn set_node_style_at(&mut self, node: NodeId, style: DivStyle, now: Instant) {
        let previous = self.arena.style(node).clone();
        if let Some(batch) = self.transition_batch.as_mut() {
            if !batch.previous_styles.contains_key(&node) {
                batch.style_order.push(node);
                batch.previous_styles.insert(node, previous.clone());
            }
        } else {
            let transitions_enabled = self.truecolor_enabled
                && self.established_nodes.contains(&node)
                && self.connected_nodes.contains(&node);
            self.apply_style_transitions(node, &previous, &style, now, transitions_enabled);
        }
        if previous.to_taffy() != style.to_taffy()
            || previous.white_space != style.white_space
            || previous.overflow_wrap != style.overflow_wrap
            || previous.word_break != style.word_break
            || previous.position != style.position
        {
            self.mark_layout_dirty();
        } else {
            self.mark_paint_dirty();
        }
        self.arena.set_style(node, style);
    }

    fn apply_style_transitions(
        &mut self,
        node: NodeId,
        previous: &DivStyle,
        style: &DivStyle,
        now: Instant,
        transitions_enabled: bool,
    ) {
        self.transitions.style_color_changed(
            node,
            TransitionProperty::Color,
            previous.color,
            style.color,
            now,
            transitions_enabled,
        );
        self.transitions.style_color_changed(
            node,
            TransitionProperty::BackgroundColor,
            previous.background,
            style.background,
            now,
            transitions_enabled,
        );
        self.transitions.style_color_changed(
            node,
            TransitionProperty::BorderColor,
            previous.border_color,
            style.border_color,
            now,
            transitions_enabled,
        );
        self.transitions.style_opacity_changed(
            node,
            previous.opacity,
            style.opacity,
            now,
            transitions_enabled,
        );
        self.arena
            .set_opacity_transition_active(node, self.transitions.has_active_opacity(node));
    }

    pub(crate) fn set_transition(&mut self, node: DomId, transitions: Vec<TransitionSpec>) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.transitions.set_specs(node, transitions);
        true
    }

    #[cfg(test)]
    pub(crate) fn set_truecolor_enabled(&mut self, enabled: bool) {
        if self.truecolor_enabled == enabled {
            return;
        }
        self.truecolor_enabled = enabled;
        self.mark_paint_dirty();
    }

    pub(crate) fn set_terminal_focused(&mut self, focused: bool) {
        if self.terminal_focused == focused {
            return;
        }
        self.terminal_focused = focused;
        self.mark_paint_dirty();
    }

    pub(crate) fn set_text(&mut self, node: DomId, text: impl Into<String>) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.mark_layout_dirty();
        self.arena.set_text(node, text);
        true
    }

    pub(crate) fn set_image_source(
        &mut self,
        node: DomId,
        src: impl AsRef<std::path::Path>,
    ) -> bool {
        let Ok(image) = load_png_image(src) else {
            return false;
        };
        let terminal = query_terminal_size();
        let cell_width_px = if terminal.cols == 0 || terminal.pixel_width == 0 {
            8
        } else {
            (terminal.pixel_width / terminal.cols).max(1)
        };
        let cell_height_px = if terminal.rows == 0 || terminal.pixel_height == 0 {
            16
        } else {
            (terminal.pixel_height / terminal.rows).max(1)
        };
        self.set_image_pixels_with_cell_size(
            node,
            image.width_px,
            image.height_px,
            cell_width_px,
            cell_height_px,
            image.rgb,
        )
    }

    fn set_image_pixels_with_cell_size(
        &mut self,
        node: DomId,
        width_px: u32,
        height_px: u32,
        cell_width_px: u32,
        cell_height_px: u32,
        rgb: Vec<u8>,
    ) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.mark_layout_dirty();
        self.arena.set_image_pixels_and_cell_size(
            node,
            width_px,
            height_px,
            cell_width_px,
            cell_height_px,
            rgb,
        );
        true
    }

    pub(crate) fn set_input_value(
        &mut self,
        node: DomId,
        value: impl Into<String>,
        cursor: u32,
    ) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        let value = value.into();
        self.mark_layout_dirty();
        self.arena.set_input_value(node, value.clone(), cursor);
        self.arena.set_textarea_value(node, value, cursor);
        true
    }

    pub(crate) fn set_input_focused(&mut self, node: DomId, focused: bool) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.arena.set_input_focused(node, focused);
        self.arena.set_textarea_focused(node, focused);
        self.mark_paint_dirty();
        true
    }

    pub(crate) fn set_input_placeholder(
        &mut self,
        node: DomId,
        placeholder: impl Into<String>,
    ) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        let placeholder = placeholder.into();
        self.arena.set_input_placeholder(node, placeholder.clone());
        self.arena.set_textarea_placeholder(node, placeholder);
        self.mark_paint_dirty();
        true
    }

    pub(crate) fn set_textarea_value(
        &mut self,
        node: DomId,
        value: impl Into<String>,
        cursor: u32,
    ) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.mark_layout_dirty();
        self.arena.set_textarea_value(node, value, cursor);
        true
    }

    pub(crate) fn set_textarea_focused(&mut self, node: DomId, focused: bool) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.arena.set_textarea_focused(node, focused);
        self.mark_paint_dirty();
        true
    }

    pub(crate) fn set_textarea_placeholder(
        &mut self,
        node: DomId,
        placeholder: impl Into<String>,
    ) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.arena.set_textarea_placeholder(node, placeholder);
        self.mark_paint_dirty();
        true
    }

    pub(crate) fn move_textarea_cursor_vertically_for_size(
        &mut self,
        node: DomId,
        direction: i32,
        width: usize,
        height: usize,
    ) -> Option<u32> {
        let node = self.node_for(node)?;
        self.ensure_layout_for_size(width, height);
        let cursor = self.arena.move_textarea_cursor_vertically(node, direction);
        if cursor.is_some() {
            self.mark_paint_dirty();
        }
        cursor
    }

    pub(crate) fn textarea_cursor_visual_position_for_size(
        &mut self,
        node: DomId,
        width: usize,
        height: usize,
    ) -> Option<(u32, u32)> {
        let node = self.node_for(node)?;
        self.ensure_layout_for_size(width, height);
        self.arena.textarea_cursor_visual_position(node)
    }

    pub(crate) fn textarea_visual_line_range_for_size(
        &mut self,
        node: DomId,
        row: u32,
        width: usize,
        height: usize,
    ) -> Option<(u32, u32)> {
        let node = self.node_for(node)?;
        self.ensure_layout_for_size(width, height);
        self.arena.textarea_visual_line_range(node, row)
    }

    pub(crate) fn set_text_control_cursor_at_point_for_size(
        &mut self,
        node: DomId,
        x: u32,
        y: u32,
        width: usize,
        height: usize,
    ) -> Option<u32> {
        let node = self.node_for(node)?;
        self.ensure_layout_for_size(width, height);
        let cursor = self.arena.set_text_control_cursor_at_point(node, x, y);
        if cursor.is_some() {
            self.mark_paint_dirty();
        }
        cursor
    }

    pub(crate) fn scroll_metrics(&mut self, node: DomId) -> Option<ArenaScrollMetrics> {
        self.node_for(node)
            .and_then(|node| self.arena.scroll_metrics(node))
    }

    pub(crate) fn scroll_metrics_for_size(
        &mut self,
        node: DomId,
        width: usize,
        height: usize,
    ) -> Option<ArenaScrollMetrics> {
        self.ensure_layout_for_size(width, height);
        self.scroll_metrics(node)
    }

    pub(crate) fn set_scroll_offset(
        &mut self,
        node: DomId,
        scroll_left: u32,
        scroll_top: u32,
    ) -> Option<ArenaScrollMetrics> {
        let metrics = self
            .node_for(node)
            .and_then(|node| self.arena.set_scroll_offset(node, scroll_left, scroll_top));
        if metrics.is_some() {
            self.mark_paint_dirty();
        }
        metrics
    }

    pub(crate) fn set_scroll_offset_for_size(
        &mut self,
        node: DomId,
        scroll_left: u32,
        scroll_top: u32,
        width: usize,
        height: usize,
    ) -> Option<ArenaScrollMetrics> {
        self.ensure_layout_for_size(width, height);
        self.set_scroll_offset(node, scroll_left, scroll_top)
    }

    #[cfg(test)]
    pub(crate) fn flush_frame_to(
        &mut self,
        width: usize,
        height: usize,
        out: &mut impl Write,
        color_profile: TermProfile,
        synchronized: bool,
    ) -> io::Result<()> {
        self.flush_frame_to_at(
            width,
            height,
            out,
            color_profile,
            synchronized,
            Instant::now(),
        )
    }

    fn flush_frame_to_at(
        &mut self,
        width: usize,
        height: usize,
        out: &mut impl Write,
        color_profile: TermProfile,
        synchronized: bool,
        now: Instant,
    ) -> io::Result<()> {
        self.write_frame_to_at(width, height, out, color_profile, synchronized, now)?;
        self.dirtiness = Dirtiness::Clean;
        Ok(())
    }

    fn write_frame_to_at(
        &mut self,
        width: usize,
        height: usize,
        out: &mut impl Write,
        color_profile: TermProfile,
        synchronized: bool,
        now: Instant,
    ) -> io::Result<()> {
        let total_start = Instant::now();
        let Some(frame) = self.render_frame_at(width, height, now) else {
            return Ok(());
        };
        let diff_start = Instant::now();
        frame.write_diff_to(
            out,
            self.previous_frame.as_ref(),
            color_profile,
            synchronized,
        )?;
        profile_log(
            "frame_write_diff",
            diff_start.elapsed(),
            &[("has_previous", self.previous_frame.is_some().to_string())],
        );
        self.previous_frame = Some(frame);
        let flush_start = Instant::now();
        out.flush()?;
        profile_log("stdout_flush", flush_start.elapsed(), &[]);
        profile_log(
            "frame_flush_total",
            total_start.elapsed(),
            &[("width", width.to_string()), ("height", height.to_string())],
        );
        Ok(())
    }

    #[cfg(test)]
    fn flush_if_dirty_to(
        &mut self,
        width: usize,
        height: usize,
        out: &mut impl Write,
        color_profile: TermProfile,
        synchronized: bool,
        now: Instant,
    ) -> io::Result<bool> {
        if !self.prepare_frame_tick() {
            return Ok(false);
        }
        self.flush_dirty_frame_to(width, height, out, color_profile, synchronized, now)
    }

    fn prepare_frame_tick(&mut self) -> bool {
        if self.transitions.has_active() {
            self.mark_paint_dirty();
        }
        self.dirtiness != Dirtiness::Clean
    }

    fn flush_dirty_frame_to(
        &mut self,
        width: usize,
        height: usize,
        out: &mut impl Write,
        color_profile: TermProfile,
        synchronized: bool,
        now: Instant,
    ) -> io::Result<bool> {
        self.flush_frame_to_at(width, height, out, color_profile, synchronized, now)?;
        Ok(true)
    }

    #[cfg(test)]
    pub(crate) fn render_frame(&mut self, width: usize, height: usize) -> Option<Frame> {
        self.render_frame_at(width, height, Instant::now())
    }

    fn render_frame_at(&mut self, width: usize, height: usize, now: Instant) -> Option<Frame> {
        self.render_frame_at_with_selection_capture(width, height, now, false)
    }

    fn render_frame_at_with_selection_capture(
        &mut self,
        width: usize,
        height: usize,
        now: Instant,
        force_capture_hidden_selection_units: bool,
    ) -> Option<Frame> {
        self.sync_viewport_scrollbar_color();
        let root = self.root.and_then(|root| self.node_for(root))?;
        let total_start = Instant::now();
        let layout_start = Instant::now();
        let ensure_layout_start = Instant::now();
        let layout_changed = self.ensure_layout(width, height, root);
        profile_log("ensure_layout", ensure_layout_start.elapsed(), &[]);
        let textarea_scroll_start = Instant::now();
        self.arena.ensure_dirty_textareas_visible();
        self.arena.prepare_paint(root);
        profile_log(
            "ensure_dirty_textareas_visible",
            textarea_scroll_start.elapsed(),
            &[],
        );
        if profile_enabled() {
            let layout_stats_start = Instant::now();
            let stats = self.arena.stats();
            let layout_profile = self.arena.profile_stats();
            profile_log("layout_stats", layout_stats_start.elapsed(), &[]);
            profile_log(
                "layout",
                layout_start.elapsed(),
                &[
                    ("dirty", layout_changed.to_string()),
                    ("passes", self.layout_passes().to_string()),
                    ("nodes", stats.node_count.to_string()),
                    ("inline_contexts", stats.inline_context_count.to_string()),
                    ("inline_fragments", stats.inline_fragment_count.to_string()),
                    (
                        "inline_width_calls",
                        layout_profile.inline_width_calls.to_string(),
                    ),
                    (
                        "inline_height_calls",
                        layout_profile.inline_height_calls.to_string(),
                    ),
                    (
                        "inline_fragment_calls",
                        layout_profile.inline_fragment_calls.to_string(),
                    ),
                    (
                        "dirty_descendant_visits",
                        layout_profile.dirty_descendant_visits.to_string(),
                    ),
                    (
                        "visible_overflow_visits",
                        layout_profile.visible_overflow_visits.to_string(),
                    ),
                    (
                        "scroll_clamp_visits",
                        layout_profile.scroll_clamp_visits.to_string(),
                    ),
                    (
                        "dirty_textarea_visits",
                        layout_profile.dirty_textarea_visits.to_string(),
                    ),
                    (
                        "absolute_layout_visits",
                        layout_profile.absolute_layout_visits.to_string(),
                    ),
                    (
                        "stacking_tree_visits",
                        layout_profile.stacking_tree_visits.to_string(),
                    ),
                    ("taffy_ms", ns_to_ms(layout_profile.taffy_ns)),
                    (
                        "dirty_descendants_ms",
                        ns_to_ms(layout_profile.dirty_descendants_ns),
                    ),
                    (
                        "visible_overflow_ms",
                        ns_to_ms(layout_profile.visible_overflow_ns),
                    ),
                    ("inline_width_ms", ns_to_ms(layout_profile.inline_width_ns)),
                    (
                        "inline_height_ms",
                        ns_to_ms(layout_profile.inline_height_ns),
                    ),
                    (
                        "inline_fragment_ms",
                        ns_to_ms(layout_profile.inline_fragment_ns),
                    ),
                ],
            );
        }

        let capture_hidden_selection_units =
            force_capture_hidden_selection_units || self.selection.active_selection().is_some();
        let paint_start = Instant::now();
        let mut output = paint_arena_with_options(
            &self.arena,
            root,
            width,
            height,
            capture_hidden_selection_units,
            PaintOptions {
                transitions: Some(&self.transitions),
                now,
                truecolor_enabled: self.truecolor_enabled,
                default_foreground: self.terminal_foreground,
                default_background: self.terminal_background,
                terminal_focused: self.terminal_focused,
            },
        );
        profile_log(
            "paint",
            paint_start.elapsed(),
            &[
                ("hit_regions", output.hit_regions.len().to_string()),
                (
                    "capture_hidden_selection",
                    capture_hidden_selection_units.to_string(),
                ),
            ],
        );
        let bookkeeping_start = Instant::now();
        output
            .frame
            .apply_selection(self.selection.active_selection().as_ref());
        self.selection
            .refresh_focus_from_last_pointer(&output.frame);
        self.hit_regions = output.hit_regions;
        self.current_frame = Some(output.frame.clone());
        for (node, property) in self.transitions.finish_completed(now) {
            if property == TransitionProperty::Opacity {
                self.arena.set_opacity_transition_active(node, false);
            }
        }
        profile_log("frame_bookkeeping", bookkeeping_start.elapsed(), &[]);
        profile_log("render_frame_total", total_start.elapsed(), &[]);
        Some(output.frame)
    }

    pub(crate) fn target_at(&self, x: u32, y: u32) -> Option<DomId> {
        let x = x.min(i32::MAX as u32) as i32;
        let y = y.min(i32::MAX as u32) as i32;
        self.hit_regions
            .iter()
            .rev()
            .find(|region| {
                x >= region.left && x < region.right && y >= region.top && y < region.bottom
            })
            .and_then(|region| self.dom_for(region.id))
    }

    pub(crate) fn click_event_for(&self, click: MouseClick) -> Option<ClickEvent> {
        Some(ClickEvent {
            target_id: self.target_at(click.x, click.y)?,
            client_x: click.x,
            client_y: click.y,
            button: click.button,
            ctrl_key: click.ctrl_key,
            alt_key: click.alt_key,
            meta_key: click.meta_key,
            shift_key: click.shift_key,
        })
    }

    pub(crate) fn scrollbar_hit_at(&self, x: u32, y: u32) -> Option<ScrollbarHit> {
        let x_i32 = x.min(i32::MAX as u32) as i32;
        let y_i32 = y.min(i32::MAX as u32) as i32;
        self.hit_regions
            .iter()
            .rev()
            .filter(|region| {
                x_i32 >= region.left
                    && x_i32 < region.right
                    && y_i32 >= region.top
                    && y_i32 < region.bottom
            })
            .find_map(|region| self.arena.scrollbar_hit_for_point(region.id, x, y))
            .and_then(|hit| self.scrollbar_hit_for_dom(hit))
    }

    pub(crate) fn handle_pointer_move(&mut self, x: u32, y: u32) {
        self.last_pointer_position = Some((x, y));
        self.refresh_pointer_shape();
    }

    pub(crate) fn handle_selection_event(&mut self, event: SelectionMouseEvent) -> SelectionAction {
        match event.event_type {
            SelectionMouseEventType::Down => self.handle_selection_down(event),
            SelectionMouseEventType::Drag => {
                let (event, autoscrolled) = self.autoscroll_selection_event(event);
                let action = self
                    .selection
                    .handle_event(event, self.current_frame.as_ref());
                if autoscrolled && matches!(action, SelectionAction::None) {
                    SelectionAction::Redraw
                } else {
                    action
                }
            }
            SelectionMouseEventType::Up => {
                let (event, _) = self.autoscroll_selection_event(event);
                let action = self
                    .selection
                    .handle_event(event, self.current_frame.as_ref());
                self.selection_scroll_node = None;
                action
            }
            SelectionMouseEventType::Scroll => self
                .selection
                .handle_event(event, self.current_frame.as_ref()),
        }
    }

    fn handle_selection_down(&mut self, event: SelectionMouseEvent) -> SelectionAction {
        self.repaint_current_frame_with_selection_capture();
        let selection_scroll_node = self.selection_scroll_node_at(event.x, event.y);
        let action = self
            .selection
            .handle_event(event, self.current_frame.as_ref());
        self.selection_scroll_node = self
            .selection
            .is_selecting()
            .then_some(selection_scroll_node)
            .flatten();
        action
    }

    fn autoscroll_selection_event(
        &mut self,
        event: SelectionMouseEvent,
    ) -> (SelectionMouseEvent, bool) {
        let Some(node) = self.selection_scroll_node else {
            return (event, false);
        };
        let Some(rect) = self.arena.scrollport_absolute_rect(node) else {
            return (event, false);
        };
        if rect.left >= rect.right || rect.top >= rect.bottom {
            return (event, false);
        }
        let Some(metrics) = self.arena.scroll_metrics_snapshot(node) else {
            return (event, false);
        };

        let x = event.x.min(i32::MAX as u32) as i32;
        let y = event.y.min(i32::MAX as u32) as i32;
        let mut next_left = metrics.scroll_left;
        let mut next_top = metrics.scroll_top;

        if x < rect.left {
            next_left = next_left.saturating_sub((rect.left - x) as u32);
        } else if x >= rect.right {
            next_left = next_left.saturating_add((x - rect.right + 1) as u32);
        }

        if y < rect.top {
            next_top = next_top.saturating_sub((rect.top - y) as u32);
        } else if y >= rect.bottom {
            next_top = next_top.saturating_add((y - rect.bottom + 1) as u32);
        }

        let Some(next_metrics) = self.arena.set_scroll_offset(node, next_left, next_top) else {
            return (event, false);
        };
        if next_metrics.scroll_left == metrics.scroll_left
            && next_metrics.scroll_top == metrics.scroll_top
        {
            return (event, false);
        }

        self.repaint_current_frame_with_selection_capture();
        (
            SelectionMouseEvent {
                x: clamp_pointer_to_rect_axis(x, rect.left, rect.right),
                y: clamp_pointer_to_rect_axis(y, rect.top, rect.bottom),
                ..event
            },
            true,
        )
    }

    fn repaint_current_frame_with_selection_capture(&mut self) {
        if let Some((width, height)) = self
            .current_frame
            .as_ref()
            .map(|frame| (frame.width(), frame.height()))
        {
            self.render_frame_at_with_selection_capture(width, height, Instant::now(), true);
        }
    }

    fn selection_scroll_node_at(&self, x: u32, y: u32) -> Option<NodeId> {
        let x = x.min(i32::MAX as u32) as i32;
        let y = y.min(i32::MAX as u32) as i32;
        let mut current = self
            .hit_regions
            .iter()
            .rev()
            .find(|region| {
                x >= region.left && x < region.right && y >= region.top && y < region.bottom
            })
            .map(|region| region.id);

        while let Some(node) = current {
            if self.is_selection_scroll_node(node)
                && self
                    .arena
                    .scrollport_absolute_rect(node)
                    .is_some_and(|rect| rect.contains(x, y))
            {
                return Some(node);
            }
            current = self.arena.parent(node);
        }

        None
    }

    fn is_selection_scroll_node(&self, node: NodeId) -> bool {
        let Some(metrics) = self.arena.scroll_metrics_snapshot(node) else {
            return false;
        };

        match self.arena.kind(node) {
            LayoutNodeKind::Element => {
                let style = self.arena.style(node);
                let can_scroll_x = style.overflow_x == LayoutOverflow::Scroll
                    && metrics.scroll_width > metrics.client_width;
                let can_scroll_y = style.overflow_y == LayoutOverflow::Scroll
                    && metrics.scroll_height > metrics.client_height;
                can_scroll_x || can_scroll_y
            }
            LayoutNodeKind::TextArea(_) => metrics.scroll_height > metrics.client_height,
            LayoutNodeKind::Text(_) | LayoutNodeKind::Image(_) | LayoutNodeKind::Input(_) => false,
        }
    }

    pub(crate) fn layout_passes(&self) -> u64 {
        self.arena.layout_passes()
    }

    pub(crate) fn invalidate_frame(&mut self) {
        self.previous_frame = None;
        self.mark_paint_dirty();
    }

    pub(crate) fn drain_transition_events(&mut self) -> Vec<EngineTransitionEvent> {
        self.transitions
            .drain_events()
            .into_iter()
            .filter_map(|event| self.transition_event_for_dom(event))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn has_active_transitions(&self) -> bool {
        self.transitions.has_active()
    }

    fn update_reparented_subtree_connectivity(
        &mut self,
        child: DomId,
        was_connected: bool,
        is_connected: bool,
    ) {
        match (was_connected, is_connected) {
            (false, true) => self.connect_subtree(child),
            (true, false) => self.disconnect_subtree(child),
            _ => {}
        }
    }

    fn connect_subtree(&mut self, root: DomId) {
        for node in self.subtree_nodes(root) {
            if self.connected_nodes.insert(node) {
                if let Some(batch) = self.transition_batch.as_mut() {
                    batch.newly_connected.insert(node);
                } else {
                    self.established_nodes.insert(node);
                }
            }
        }
    }

    fn disconnect_subtree(&mut self, root: DomId) {
        for node in self.subtree_nodes(root) {
            self.connected_nodes.remove(&node);
            self.established_nodes.remove(&node);
            if let Some(batch) = self.transition_batch.as_mut() {
                batch.newly_connected.remove(&node);
            }
        }
    }

    fn subtree_nodes(&self, root: DomId) -> Vec<NodeId> {
        let mut nodes = Vec::new();
        let mut pending = vec![root];
        while let Some(dom_id) = pending.pop() {
            let Some(node) = self.node_for(dom_id) else {
                continue;
            };
            nodes.push(node);
            if let Some(children) = self.children.get(&dom_id) {
                pending.extend(children.iter().rev().copied());
            }
        }
        nodes
    }

    fn ensure_layout(&mut self, width: usize, height: usize, root: NodeId) -> bool {
        let size = (width, height);
        if self.dirtiness != Dirtiness::Layout && self.last_layout_size == Some(size) {
            return false;
        }

        let available = Size {
            width: AvailableSpace::Definite(width as f32),
            height: AvailableSpace::Definite(height as f32),
        };
        for _ in 0..3 {
            self.arena.compute_layout(root, available);
            let Some(viewport) = self.viewport.and_then(|id| self.node_for(id)) else {
                break;
            };
            let Some(metrics) = self.arena.scroll_metrics_snapshot(viewport) else {
                break;
            };
            let mut style = self.arena.style(viewport).clone();
            let overflow_x = if metrics.scroll_width > metrics.client_width {
                LayoutOverflow::Scroll
            } else {
                LayoutOverflow::Hidden
            };
            let overflow_y = if metrics.scroll_height > metrics.client_height {
                LayoutOverflow::Scroll
            } else {
                LayoutOverflow::Hidden
            };
            if style.overflow_x == overflow_x && style.overflow_y == overflow_y {
                break;
            }
            style.overflow_x = overflow_x;
            style.overflow_y = overflow_y;
            self.arena.set_style(viewport, style);
        }
        let clamp_scroll_start = Instant::now();
        self.arena.clamp_scroll_offsets();
        profile_log("clamp_scroll_offsets", clamp_scroll_start.elapsed(), &[]);
        self.dirtiness = Dirtiness::Paint;
        self.last_layout_size = Some(size);
        true
    }

    fn ensure_layout_for_size(&mut self, width: usize, height: usize) {
        if let Some(root) = self.root.and_then(|root| self.node_for(root)) {
            self.ensure_layout(width, height, root);
        }
    }

    fn sync_viewport_scrollbar_color(&mut self) {
        let Some(viewport) = self.viewport else {
            return;
        };
        let Some(content_root) = self
            .children
            .get(&viewport)
            .and_then(|children| children.first())
            .copied()
        else {
            return;
        };
        let Some(viewport_node) = self.node_for(viewport) else {
            return;
        };
        let Some(content_root_node) = self.node_for(content_root) else {
            return;
        };
        let scrollbar_color = self.arena.style(content_root_node).scrollbar_color;
        if self.arena.style(viewport_node).scrollbar_color == scrollbar_color {
            return;
        }

        let mut style = self.arena.style(viewport_node).clone();
        style.scrollbar_color = scrollbar_color;
        self.arena.set_style(viewport_node, style);
    }

    #[cfg(test)]
    fn register_node(&mut self, node: NodeId) -> DomId {
        let id = DomId(self.next_dom_id);
        self.next_dom_id = self.next_dom_id.checked_add(1).expect("DOM id overflow");
        self.register_node_with_id(id, node)
    }

    fn register_node_with_id(&mut self, id: DomId, node: NodeId) -> DomId {
        self.next_dom_id = self
            .next_dom_id
            .max(id.0.checked_add(1).expect("DOM id overflow"));
        self.dom_to_node.insert(id, node);
        self.node_to_dom.insert(node, id);
        self.children.insert(id, Vec::new());
        id
    }

    fn node_for(&self, id: DomId) -> Option<NodeId> {
        self.dom_to_node.get(&id).copied()
    }

    fn dom_for(&self, node: NodeId) -> Option<DomId> {
        self.node_to_dom.get(&node).copied()
    }

    fn scrollbar_hit_for_dom(&self, hit: ArenaScrollbarHit) -> Option<ScrollbarHit> {
        Some(ScrollbarHit {
            target_id: self.dom_for(hit.node)?,
            axis: hit.axis,
            rail_start: hit.rail_start,
            rail_length: hit.rail_length,
            thumb_start: hit.thumb_start,
            thumb_length: hit.thumb_length,
            scroll_offset: hit.scroll_offset,
            max_scroll: hit.max_scroll,
            client_length: hit.client_length,
            scroll_length: hit.scroll_length,
        })
    }

    fn style_for(&self, id: DomId) -> Option<&DivStyle> {
        self.node_for(id).map(|node| self.arena.style(node))
    }

    fn transition_event_for_dom(&self, event: TransitionEvent) -> Option<EngineTransitionEvent> {
        Some(EngineTransitionEvent {
            event_type: event.event_type,
            target: self.dom_for(event.target)?,
            property: event.property,
        })
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
        let mut current = self.target_at(x, y);
        while let Some(id) = current {
            if let Some(shape) = self
                .style_for(id)
                .and_then(|style| style.cursor.osc_shape())
            {
                return Some(shape);
            }
            current = self.parents.get(&id).copied();
        }
        Some("default")
    }
}

pub(crate) fn apply_style_mutation(style: &mut DivStyle, mutation: StyleMutation) {
    match mutation {
        StyleMutation::Reset(reset) => reset_style_property(style, reset),
        StyleMutation::Display(display) => style.display = display,
        StyleMutation::Position(position) => style.position = position,
        StyleMutation::Top(top) => style.top = top,
        StyleMutation::Right(right) => style.right = right,
        StyleMutation::Bottom(bottom) => style.bottom = bottom,
        StyleMutation::Left(left) => style.left = left,
        StyleMutation::ZIndex(z_index) => style.z_index = z_index,
        StyleMutation::Visibility(visibility) => style.visibility = visibility,
        StyleMutation::Opacity(opacity) => style.opacity = opacity,
        StyleMutation::Overflow(overflow) => {
            style.overflow_x = overflow;
            style.overflow_y = overflow;
        }
        StyleMutation::OverflowX(overflow) => style.overflow_x = overflow,
        StyleMutation::OverflowY(overflow) => style.overflow_y = overflow,
        StyleMutation::ScrollbarColor(scrollbar_color) => {
            style.scrollbar_color = scrollbar_color;
        }
        StyleMutation::ScrollbarGutter(scrollbar_gutter) => {
            style.scrollbar_gutter = scrollbar_gutter;
        }
        StyleMutation::ImageRendering(image_rendering) => style.image_rendering = image_rendering,
        StyleMutation::WhiteSpace(white_space) => style.white_space = white_space,
        StyleMutation::OverflowWrap(overflow_wrap) => style.overflow_wrap = overflow_wrap,
        StyleMutation::WordBreak(word_break) => style.word_break = word_break,
        StyleMutation::FlexDirection(direction) => style.flex_direction = direction,
        StyleMutation::FlexWrap(flex_wrap) => style.flex_wrap = flex_wrap,
        StyleMutation::FlexFlow {
            direction,
            flex_wrap,
        } => {
            style.flex_direction = direction;
            style.flex_wrap = flex_wrap;
        }
        StyleMutation::FlexBasis(flex_basis) => style.flex_basis = flex_basis,
        StyleMutation::FlexGrow(flex_grow) => style.flex_grow = flex_grow,
        StyleMutation::FlexShrink(flex_shrink) => style.flex_shrink = flex_shrink,
        StyleMutation::Flex {
            flex_grow,
            flex_shrink,
            flex_basis,
        } => {
            style.flex_grow = flex_grow;
            style.flex_shrink = flex_shrink;
            style.flex_basis = flex_basis;
        }
        StyleMutation::JustifyContent(justify_content) => {
            style.justify_content = Some(justify_content);
        }
        StyleMutation::AlignItems(align_items) => style.align_items = Some(align_items),
        StyleMutation::AlignSelf(align_self) => style.align_self = Some(align_self),
        StyleMutation::AlignContent(align_content) => style.align_content = Some(align_content),
        StyleMutation::JustifyItems(justify_items) => style.justify_items = Some(justify_items),
        StyleMutation::JustifySelf(justify_self) => style.justify_self = Some(justify_self),
        StyleMutation::Gap {
            row_gap,
            column_gap,
        } => {
            style.row_gap = row_gap;
            style.column_gap = column_gap;
        }
        StyleMutation::RowGap(row_gap) => style.row_gap = row_gap,
        StyleMutation::ColumnGap(column_gap) => style.column_gap = column_gap,
        StyleMutation::Padding {
            top,
            right,
            bottom,
            left,
        } => {
            style.padding_top = top;
            style.padding_right = right;
            style.padding_bottom = bottom;
            style.padding_left = left;
        }
        StyleMutation::PaddingTop(padding) => style.padding_top = padding,
        StyleMutation::PaddingRight(padding) => style.padding_right = padding,
        StyleMutation::PaddingBottom(padding) => style.padding_bottom = padding,
        StyleMutation::PaddingLeft(padding) => style.padding_left = padding,
        StyleMutation::Margin {
            top,
            right,
            bottom,
            left,
        } => {
            style.margin_top = top;
            style.margin_right = right;
            style.margin_bottom = bottom;
            style.margin_left = left;
        }
        StyleMutation::MarginTop(margin) => style.margin_top = margin,
        StyleMutation::MarginRight(margin) => style.margin_right = margin,
        StyleMutation::MarginBottom(margin) => style.margin_bottom = margin,
        StyleMutation::MarginLeft(margin) => style.margin_left = margin,
        StyleMutation::Width(width) => style.width = width,
        StyleMutation::Height(height) => style.height = height,
        StyleMutation::MinWidth(min_width) => style.min_width = min_width,
        StyleMutation::MaxWidth(max_width) => style.max_width = max_width,
        StyleMutation::MinHeight(min_height) => style.min_height = min_height,
        StyleMutation::MaxHeight(max_height) => style.max_height = max_height,
        StyleMutation::Border(border) => {
            style.border_top = border;
            style.border_right = border;
            style.border_bottom = border;
            style.border_left = border;
        }
        StyleMutation::BorderTop(border) => style.border_top = border,
        StyleMutation::BorderRight(border) => style.border_right = border,
        StyleMutation::BorderBottom(border) => style.border_bottom = border,
        StyleMutation::BorderLeft(border) => style.border_left = border,
        StyleMutation::BorderColor(color) => style.border_color = color,
        StyleMutation::Color(color) => style.color = color,
        StyleMutation::PlaceholderColor(color) => style.placeholder_color = color,
        StyleMutation::Background(background) => style.background = background,
        StyleMutation::SelectionBackground(background) => {
            style.selection_background = Some(background);
        }
        StyleMutation::FontWeight(font_weight) => style.font_weight = font_weight,
        StyleMutation::FontStyle(font_style) => style.font_style = font_style,
        StyleMutation::TextDecorationLine(text_decoration_line) => {
            style.text_decoration_line = text_decoration_line;
        }
        StyleMutation::Cursor(cursor) => style.cursor = cursor,
        StyleMutation::GridTemplateColumns(tracks) => style.grid_template_columns = tracks,
        StyleMutation::GridTemplateRows(tracks) => style.grid_template_rows = tracks,
        StyleMutation::GridAutoColumns(tracks) => style.grid_auto_columns = tracks,
        StyleMutation::GridAutoRows(tracks) => style.grid_auto_rows = tracks,
        StyleMutation::GridAutoFlow(grid_auto_flow) => style.grid_auto_flow = grid_auto_flow,
        StyleMutation::GridColumn(placement) => style.grid_column = placement,
        StyleMutation::GridRow(placement) => style.grid_row = placement,
        StyleMutation::GridColumnStart(placement) => style.grid_column.start = placement,
        StyleMutation::GridColumnEnd(placement) => style.grid_column.end = placement,
        StyleMutation::GridRowStart(placement) => style.grid_row.start = placement,
        StyleMutation::GridRowEnd(placement) => style.grid_row.end = placement,
    }
}

fn reset_style_property(style: &mut DivStyle, reset: StyleReset) {
    let default = DivStyle::default();
    match reset {
        StyleReset::Display => style.display = default.display,
        StyleReset::Position => style.position = default.position,
        StyleReset::Top => style.top = default.top,
        StyleReset::Right => style.right = default.right,
        StyleReset::Bottom => style.bottom = default.bottom,
        StyleReset::Left => style.left = default.left,
        StyleReset::ZIndex => style.z_index = default.z_index,
        StyleReset::Visibility => style.visibility = default.visibility,
        StyleReset::Opacity => style.opacity = default.opacity,
        StyleReset::Overflow => {
            style.overflow_x = default.overflow_x;
            style.overflow_y = default.overflow_y;
        }
        StyleReset::OverflowX => style.overflow_x = default.overflow_x,
        StyleReset::OverflowY => style.overflow_y = default.overflow_y,
        StyleReset::ScrollbarColor => style.scrollbar_color = default.scrollbar_color,
        StyleReset::ScrollbarGutter => style.scrollbar_gutter = default.scrollbar_gutter,
        StyleReset::ImageRendering => style.image_rendering = default.image_rendering,
        StyleReset::WhiteSpace => style.white_space = default.white_space,
        StyleReset::OverflowWrap => style.overflow_wrap = default.overflow_wrap,
        StyleReset::WordBreak => style.word_break = default.word_break,
        StyleReset::FlexDirection => style.flex_direction = default.flex_direction,
        StyleReset::FlexWrap => style.flex_wrap = default.flex_wrap,
        StyleReset::FlexFlow => {
            style.flex_direction = default.flex_direction;
            style.flex_wrap = default.flex_wrap;
        }
        StyleReset::FlexBasis => style.flex_basis = default.flex_basis,
        StyleReset::FlexGrow => style.flex_grow = default.flex_grow,
        StyleReset::FlexShrink => style.flex_shrink = default.flex_shrink,
        StyleReset::Flex => {
            style.flex_grow = default.flex_grow;
            style.flex_shrink = default.flex_shrink;
            style.flex_basis = default.flex_basis;
        }
        StyleReset::JustifyContent => style.justify_content = default.justify_content,
        StyleReset::AlignItems => style.align_items = default.align_items,
        StyleReset::AlignSelf => style.align_self = default.align_self,
        StyleReset::AlignContent => style.align_content = default.align_content,
        StyleReset::JustifyItems => style.justify_items = default.justify_items,
        StyleReset::JustifySelf => style.justify_self = default.justify_self,
        StyleReset::Gap => {
            style.row_gap = default.row_gap;
            style.column_gap = default.column_gap;
        }
        StyleReset::RowGap => style.row_gap = default.row_gap,
        StyleReset::ColumnGap => style.column_gap = default.column_gap,
        StyleReset::Padding => {
            style.padding_top = default.padding_top;
            style.padding_right = default.padding_right;
            style.padding_bottom = default.padding_bottom;
            style.padding_left = default.padding_left;
        }
        StyleReset::PaddingTop => style.padding_top = default.padding_top,
        StyleReset::PaddingRight => style.padding_right = default.padding_right,
        StyleReset::PaddingBottom => style.padding_bottom = default.padding_bottom,
        StyleReset::PaddingLeft => style.padding_left = default.padding_left,
        StyleReset::Margin => {
            style.margin_top = default.margin_top;
            style.margin_right = default.margin_right;
            style.margin_bottom = default.margin_bottom;
            style.margin_left = default.margin_left;
        }
        StyleReset::MarginTop => style.margin_top = default.margin_top,
        StyleReset::MarginRight => style.margin_right = default.margin_right,
        StyleReset::MarginBottom => style.margin_bottom = default.margin_bottom,
        StyleReset::MarginLeft => style.margin_left = default.margin_left,
        StyleReset::Width => style.width = default.width,
        StyleReset::Height => style.height = default.height,
        StyleReset::MinWidth => style.min_width = default.min_width,
        StyleReset::MaxWidth => style.max_width = default.max_width,
        StyleReset::MinHeight => style.min_height = default.min_height,
        StyleReset::MaxHeight => style.max_height = default.max_height,
        StyleReset::Border => {
            style.border_top = default.border_top;
            style.border_right = default.border_right;
            style.border_bottom = default.border_bottom;
            style.border_left = default.border_left;
        }
        StyleReset::BorderTop => style.border_top = default.border_top,
        StyleReset::BorderRight => style.border_right = default.border_right,
        StyleReset::BorderBottom => style.border_bottom = default.border_bottom,
        StyleReset::BorderLeft => style.border_left = default.border_left,
        StyleReset::BorderColor => style.border_color = default.border_color,
        StyleReset::Color => style.color = default.color,
        StyleReset::PlaceholderColor => style.placeholder_color = default.placeholder_color,
        StyleReset::Background => style.background = default.background,
        StyleReset::SelectionBackground => {
            style.selection_background = default.selection_background
        }
        StyleReset::FontWeight => style.font_weight = default.font_weight,
        StyleReset::FontStyle => style.font_style = default.font_style,
        StyleReset::TextDecorationLine => style.text_decoration_line = default.text_decoration_line,
        StyleReset::Cursor => style.cursor = default.cursor,
        StyleReset::GridTemplateColumns => {
            style.grid_template_columns = default.grid_template_columns
        }
        StyleReset::GridTemplateRows => style.grid_template_rows = default.grid_template_rows,
        StyleReset::GridAutoColumns => style.grid_auto_columns = default.grid_auto_columns,
        StyleReset::GridAutoRows => style.grid_auto_rows = default.grid_auto_rows,
        StyleReset::GridAutoFlow => style.grid_auto_flow = default.grid_auto_flow,
        StyleReset::GridColumn => style.grid_column = default.grid_column,
        StyleReset::GridRow => style.grid_row = default.grid_row,
        StyleReset::GridColumnStart => style.grid_column.start = default.grid_column.start,
        StyleReset::GridColumnEnd => style.grid_column.end = default.grid_column.end,
        StyleReset::GridRowStart => style.grid_row.start = default.grid_row.start,
        StyleReset::GridRowEnd => style.grid_row.end = default.grid_row.end,
    }
}

fn scrollbar_suppresses_selection(engine: &mut PaintEngine, event: SelectionMouseEvent) -> bool {
    match event.event_type {
        SelectionMouseEventType::Down if engine.scrollbar_hit_at(event.x, event.y).is_some() => {
            engine.scrollbar_selection_suppressed = true;
            true
        }
        SelectionMouseEventType::Drag | SelectionMouseEventType::Scroll => {
            engine.scrollbar_selection_suppressed
        }
        SelectionMouseEventType::Up if engine.scrollbar_selection_suppressed => {
            engine.scrollbar_selection_suppressed = false;
            true
        }
        SelectionMouseEventType::Down | SelectionMouseEventType::Up => false,
    }
}

fn clamp_pointer_to_rect_axis(value: i32, start: i32, end: i32) -> u32 {
    if end <= start {
        return start.max(0) as u32;
    }

    value.clamp(start, end - 1).max(0) as u32
}

fn profile_log(label: &str, duration: std::time::Duration, fields: &[(&str, String)]) {
    if !profile_enabled() {
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

fn profile_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("PAINTCANNON_PROFILE").is_some())
}

fn ns_to_ms(ns: u128) -> String {
    format!("{:.3}", ns as f64 / 1_000_000.0)
}

#[cfg(test)]
mod tests;
