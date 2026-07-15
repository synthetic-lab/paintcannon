use std::collections::{HashMap, HashSet};
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender as StdSender,
    Arc, OnceLock,
};
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};
use taffy::{AvailableSpace, NodeId, Size};
use termprofile::{DetectorSettings, TermProfile};

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
    CssPosition, CssTextDecorationLine, CssTrackSizing, CssVisibility, CssWhiteSpace, CssZIndex,
    CursorStyle, DivStyle, ImageRendering, LayoutAlignItems, LayoutDisplay, LayoutFlexDirection,
    LayoutFlexWrap, LayoutGridAutoFlow, LayoutJustifyContent, LayoutOverflow, ScrollbarColor,
    ScrollbarGutter, TransitionProperty, TransitionSpec,
};
use crate::terminal::{copy_text_to_clipboard, query_terminal_size, write_pointer_shape};
use crate::transition::{TransitionEvent, TransitionEventType, TransitionState};

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
        width: usize,
        height: usize,
        response: Sender<Option<u32>>,
    },
    GetTextAreaCursorVisualPosition {
        node: DomId,
        width: usize,
        height: usize,
        response: Sender<Option<(u32, u32)>>,
    },
    GetTextAreaVisualLineRange {
        node: DomId,
        row: u32,
        width: usize,
        height: usize,
        response: Sender<Option<(u32, u32)>>,
    },
    SetTextControlCursorAtPoint {
        node: DomId,
        x: u32,
        y: u32,
        width: usize,
        height: usize,
        response: Sender<Option<u32>>,
    },
    SetScrollOffset {
        node: DomId,
        scroll_left: u32,
        scroll_top: u32,
        width: usize,
        height: usize,
        response: Sender<Option<ArenaScrollMetrics>>,
    },
    GetScrollMetrics {
        node: DomId,
        width: usize,
        height: usize,
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
    RenderPending {
        width: usize,
        height: usize,
        color_profile: TermProfile,
        synchronized: bool,
        pending: Arc<AtomicBool>,
    },
    RenderStdout {
        width: usize,
        height: usize,
        color_profile: TermProfile,
        synchronized: bool,
        response: StdSender<io::Result<()>>,
    },
    DrainTransitionEvents {
        response: Sender<Vec<EngineTransitionEvent>>,
    },
    HasActiveTransitions {
        response: Sender<bool>,
    },
    SetTruecolorEnabled {
        enabled: bool,
    },
    SetTerminalColors {
        foreground: Background,
        background: Background,
    },
    SetTerminalFocused {
        focused: bool,
    },
    InvalidateFrame,
    Shutdown {
        response: Option<Sender<()>>,
    },
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
    layout_dirty: bool,
    last_layout_size: Option<(usize, usize)>,
    previous_frame: Option<Frame>,
    current_frame: Option<Frame>,
    hit_regions: Vec<HitRegion>,
    selection: SelectionState,
    transitions: TransitionState,
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
            layout_dirty: false,
            last_layout_size: None,
            previous_frame: None,
            current_frame: None,
            hit_regions: Vec::new(),
            selection: SelectionState::default(),
            transitions: TransitionState::default(),
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

    #[cfg(test)]
    pub(crate) fn create_element(&mut self, style: DivStyle) -> DomId {
        self.layout_dirty = true;
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
        self.layout_dirty = true;
        let node = self.arena.create_element(style);
        self.register_node_with_id(id, node)
    }

    #[cfg(test)]
    pub(crate) fn create_text(&mut self, text: impl Into<String>) -> DomId {
        self.layout_dirty = true;
        let node = self.arena.create_text(text);
        self.register_node(node)
    }

    pub(crate) fn create_text_with_id(&mut self, id: DomId, text: impl Into<String>) -> DomId {
        self.layout_dirty = true;
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
        self.layout_dirty = true;
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
        self.layout_dirty = true;
        let node = self.arena.create_input(style, value);
        self.register_node_with_id(id, node)
    }

    #[cfg(test)]
    pub(crate) fn create_textarea(&mut self, style: DivStyle, value: impl Into<String>) -> DomId {
        self.layout_dirty = true;
        let node = self.arena.create_textarea(style, value);
        self.register_node(node)
    }

    pub(crate) fn create_textarea_with_id(
        &mut self,
        id: DomId,
        style: DivStyle,
        value: impl Into<String>,
    ) -> DomId {
        self.layout_dirty = true;
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

        if let Some(old_parent) = self.parents.insert(child, parent) {
            if let Some(old_parent_node) = self.node_for(old_parent) {
                self.arena.remove_child(old_parent_node, child_node);
            }
            if let Some(siblings) = self.children.get_mut(&old_parent) {
                siblings.retain(|id| *id != child);
            }
        }

        self.children.entry(parent).or_default().push(child);
        self.layout_dirty = true;
        self.arena.append_child(parent_node, child_node);
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
        self.layout_dirty = true;
        self.arena
            .insert_child_before(parent_node, child_node, before_node);
        true
    }

    pub(crate) fn set_root(&mut self, root: DomId) -> bool {
        if self.node_for(root).is_none() {
            return false;
        }
        self.root = Some(root);
        self.layout_dirty = true;
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
        self.layout_dirty = true;
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

        self.layout_dirty = true;
        removed.len()
    }

    pub(crate) fn detach_node(&mut self, node: DomId) -> bool {
        let Some(node_id) = self.node_for(node) else {
            return false;
        };

        if self.root == Some(node) {
            self.root = None;
            self.layout_dirty = true;
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
        self.layout_dirty = true;
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
        let transitions_enabled = self.truecolor_enabled && self.arena.has_computed_layout(node);
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
        self.layout_dirty = self.layout_dirty
            || previous.to_taffy() != style.to_taffy()
            || previous.white_space != style.white_space
            || previous.position != style.position;
        self.arena.set_style(node, style);
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

    pub(crate) fn set_truecolor_enabled(&mut self, enabled: bool) {
        self.truecolor_enabled = enabled;
    }

    pub(crate) fn set_terminal_colors(&mut self, foreground: Background, background: Background) {
        self.terminal_foreground = foreground;
        self.terminal_background = background;
    }

    pub(crate) fn set_terminal_focused(&mut self, focused: bool) {
        self.terminal_focused = focused;
    }

    pub(crate) fn set_text(&mut self, node: DomId, text: impl Into<String>) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.layout_dirty = true;
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
        self.layout_dirty = true;
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
        self.layout_dirty = true;
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
        self.layout_dirty = true;
        self.arena.set_textarea_value(node, value, cursor);
        true
    }

    pub(crate) fn set_textarea_focused(&mut self, node: DomId, focused: bool) -> bool {
        let Some(node) = self.node_for(node) else {
            return false;
        };
        self.arena.set_textarea_focused(node, focused);
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
        self.arena.move_textarea_cursor_vertically(node, direction)
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
        self.arena.set_text_control_cursor_at_point(node, x, y)
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
        self.node_for(node)
            .and_then(|node| self.arena.set_scroll_offset(node, scroll_left, scroll_top))
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

    pub(crate) fn render_to(
        &mut self,
        width: usize,
        height: usize,
        out: &mut impl Write,
        color_profile: TermProfile,
        synchronized: bool,
    ) -> io::Result<()> {
        let total_start = Instant::now();
        let Some(frame) = self.render_frame(width, height) else {
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
            "render_to_total",
            total_start.elapsed(),
            &[("width", width.to_string()), ("height", height.to_string())],
        );
        Ok(())
    }

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
        let layout_passes_before = self.layout_passes();
        let ensure_layout_start = Instant::now();
        self.ensure_layout(width, height, root);
        profile_log("ensure_layout", ensure_layout_start.elapsed(), &[]);
        let layout_changed = layout_passes_before != self.layout_passes();
        if layout_changed {
            let clamp_scroll_start = Instant::now();
            self.arena.clamp_scroll_offsets();
            profile_log("clamp_scroll_offsets", clamp_scroll_start.elapsed(), &[]);
        }
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
    }

    pub(crate) fn drain_transition_events(&mut self) -> Vec<EngineTransitionEvent> {
        self.transitions
            .drain_events()
            .into_iter()
            .filter_map(|event| self.transition_event_for_dom(event))
            .collect()
    }

    pub(crate) fn has_active_transitions(&self) -> bool {
        self.transitions.has_active()
    }

    fn ensure_layout(&mut self, width: usize, height: usize, root: NodeId) {
        let size = (width, height);
        if !self.layout_dirty && self.last_layout_size == Some(size) {
            return;
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
        self.layout_dirty = false;
        self.last_layout_size = Some(size);
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

pub(crate) fn engine_loop(rx: Receiver<EngineCommand>) {
    let mut engine = PaintEngine::new();

    while let Ok(command) = rx.recv() {
        if !apply_command(&mut engine, command) {
            break;
        }
    }
}

fn apply_command(engine: &mut PaintEngine, command: EngineCommand) -> bool {
    match command {
        EngineCommand::Batch { commands } => {
            let start = Instant::now();
            let command_count = commands.len();
            engine.reserve_for_batch(&commands);
            let mut pending_destroys = Vec::new();
            for command in commands {
                if let EngineCommand::DestroyNode { node } = command {
                    pending_destroys.push(node);
                    continue;
                }
                if !pending_destroys.is_empty() {
                    engine.destroy_nodes(pending_destroys.drain(..));
                }
                if !apply_command(engine, command) {
                    profile_log(
                        "batch_apply",
                        start.elapsed(),
                        &[
                            ("commands", command_count.to_string()),
                            ("shutdown", true.to_string()),
                        ],
                    );
                    return false;
                }
            }
            if !pending_destroys.is_empty() {
                engine.destroy_nodes(pending_destroys);
            }
            profile_log(
                "batch_apply",
                start.elapsed(),
                &[
                    ("commands", command_count.to_string()),
                    ("shutdown", false.to_string()),
                ],
            );
        }
        #[cfg(test)]
        EngineCommand::CreateElement { style, response } => {
            let _ = response.send(engine.create_element(style));
        }
        EngineCommand::CreateElementWithId { id, style } => {
            engine.create_element_with_id(id, style);
        }
        #[cfg(test)]
        EngineCommand::CreateText { text, response } => {
            let _ = response.send(engine.create_text(text));
        }
        EngineCommand::CreateTextWithId { id, text } => {
            engine.create_text_with_id(id, text);
        }
        EngineCommand::CreateImageWithId {
            id,
            style,
            width_px,
            height_px,
            cell_width_px,
            cell_height_px,
        } => {
            engine.create_image_with_id(
                id,
                style,
                width_px,
                height_px,
                cell_width_px,
                cell_height_px,
            );
        }
        EngineCommand::CreateInputWithId { id, style, value } => {
            engine.create_input_with_id(id, style, value);
        }
        EngineCommand::CreateTextAreaWithId { id, style, value } => {
            engine.create_textarea_with_id(id, style, value);
        }
        EngineCommand::AppendChild { parent, child } => {
            engine.append_child(parent, child);
        }
        EngineCommand::InsertChildBefore {
            parent,
            child,
            before,
        } => {
            engine.insert_child_before(parent, child, before);
        }
        EngineCommand::SetRoot { root } => {
            engine.set_root(root);
        }
        EngineCommand::SetViewport { viewport } => {
            engine.set_viewport(viewport);
        }
        EngineCommand::DestroyNode { node } => {
            engine.destroy_node(node);
        }
        EngineCommand::DetachNode { node } => {
            engine.detach_node(node);
        }
        EngineCommand::MutateStyle { node, mutation } => {
            engine.mutate_style(node, mutation);
        }
        EngineCommand::SetTransition { node, transitions } => {
            engine.set_transition(node, transitions);
        }
        EngineCommand::SetText { node, text } => {
            engine.set_text(node, text);
        }
        EngineCommand::SetImageSource { node, src } => {
            engine.set_image_source(node, src);
        }
        EngineCommand::SetInputValue {
            node,
            value,
            cursor,
        } => {
            engine.set_input_value(node, value, cursor);
        }
        EngineCommand::SetInputFocused { node, focused } => {
            engine.set_input_focused(node, focused);
        }
        EngineCommand::SetInputPlaceholder { node, placeholder } => {
            engine.set_input_placeholder(node, placeholder);
        }
        EngineCommand::SetTextAreaValue {
            node,
            value,
            cursor,
        } => {
            engine.set_textarea_value(node, value, cursor);
        }
        EngineCommand::SetTextAreaFocused { node, focused } => {
            engine.set_textarea_focused(node, focused);
        }
        EngineCommand::SetTextAreaPlaceholder { node, placeholder } => {
            engine.set_textarea_placeholder(node, placeholder);
        }
        EngineCommand::MoveTextAreaCursorVertically {
            node,
            direction,
            width,
            height,
            response,
        } => {
            let _ = response.send(
                engine.move_textarea_cursor_vertically_for_size(node, direction, width, height),
            );
        }
        EngineCommand::GetTextAreaCursorVisualPosition {
            node,
            width,
            height,
            response,
        } => {
            let _ =
                response.send(engine.textarea_cursor_visual_position_for_size(node, width, height));
        }
        EngineCommand::GetTextAreaVisualLineRange {
            node,
            row,
            width,
            height,
            response,
        } => {
            let _ =
                response.send(engine.textarea_visual_line_range_for_size(node, row, width, height));
        }
        EngineCommand::SetTextControlCursorAtPoint {
            node,
            x,
            y,
            width,
            height,
            response,
        } => {
            let _ = response
                .send(engine.set_text_control_cursor_at_point_for_size(node, x, y, width, height));
        }
        EngineCommand::SetScrollOffset {
            node,
            scroll_left,
            scroll_top,
            width,
            height,
            response,
        } => {
            let _ = response.send(engine.set_scroll_offset_for_size(
                node,
                scroll_left,
                scroll_top,
                width,
                height,
            ));
        }
        EngineCommand::GetScrollMetrics {
            node,
            width,
            height,
            response,
        } => {
            let _ = response.send(engine.scroll_metrics_for_size(node, width, height));
        }
        EngineCommand::HitTestPoint { x, y, response } => {
            let _ = response.send(engine.target_at(x, y));
        }
        EngineCommand::HitTestClick { click, response } => {
            let _ = response.send(engine.click_event_for(click));
        }
        EngineCommand::HitTestScrollbar { x, y, response } => {
            let _ = response.send(engine.scrollbar_hit_at(x, y));
        }
        EngineCommand::HandleSelection { event, response } => {
            if scrollbar_suppresses_selection(engine, event) {
                let _ = response.send(SelectionAction::None);
                return true;
            }
            let action = engine.handle_selection_event(event);
            if let SelectionAction::CopyToClipboard(text) = &action {
                copy_text_to_clipboard(text);
            }
            if matches!(
                &action,
                SelectionAction::Redraw | SelectionAction::CopyToClipboard(_)
            ) {
                let size = query_terminal_size();
                let color_profile = TermProfile::detect(&io::stdout(), DetectorSettings::default());
                let mut out = io::stdout().lock();
                let _ = engine.render_to(
                    size.cols as usize,
                    size.rows as usize,
                    &mut out,
                    color_profile,
                    io::stdout().is_terminal(),
                );
            }
            let _ = response.send(action);
        }
        EngineCommand::HandlePointerMove { x, y } => {
            engine.handle_pointer_move(x, y);
        }
        #[cfg(test)]
        EngineCommand::RenderFrame {
            width,
            height,
            response,
        } => {
            let _ = response.send(engine.render_frame(width, height));
        }
        EngineCommand::RenderPending {
            width,
            height,
            color_profile,
            synchronized,
            pending,
        } => {
            {
                let mut out = io::stdout().lock();
                let _ = engine.render_to(width, height, &mut out, color_profile, synchronized);
            }
            pending.store(false, Ordering::Release);
        }
        EngineCommand::RenderStdout {
            width,
            height,
            color_profile,
            synchronized,
            response,
        } => {
            let result = {
                let mut out = io::stdout().lock();
                engine.render_to(width, height, &mut out, color_profile, synchronized)
            };
            let _ = response.send(result);
        }
        EngineCommand::DrainTransitionEvents { response } => {
            let _ = response.send(engine.drain_transition_events());
        }
        EngineCommand::HasActiveTransitions { response } => {
            let _ = response.send(engine.has_active_transitions());
        }
        EngineCommand::SetTruecolorEnabled { enabled } => {
            engine.set_truecolor_enabled(enabled);
        }
        EngineCommand::SetTerminalColors {
            foreground,
            background,
        } => {
            engine.set_terminal_colors(foreground, background);
        }
        EngineCommand::SetTerminalFocused { focused } => {
            engine.set_terminal_focused(focused);
        }
        EngineCommand::InvalidateFrame => engine.invalidate_frame(),
        EngineCommand::Shutdown { response } => {
            if let Some(response) = response {
                let _ = response.send(());
            }
            return false;
        }
    }

    true
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
mod tests {
    use super::*;
    use crossbeam_channel::bounded;
    use std::thread;

    use crate::selection::{SelectionMouseEvent, SelectionMouseEventType};
    use crate::style::{
        Background, CssDimension, CssFontWeight, LayoutFlexDirection, LayoutOverflow,
        TransitionProperty, TransitionSpec,
    };
    use crate::transition::TransitionEventType;

    fn block_style(width: CssDimension, height: CssDimension) -> DivStyle {
        let mut style = DivStyle::default();
        style.width = width;
        style.height = height;
        style
    }

    fn scroll_engine() -> (PaintEngine, DomId) {
        let mut engine = PaintEngine::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = engine.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = engine.create_element(content_style);
        for text in ["aaaaa", "bbbbb"] {
            let row =
                engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
            let text = engine.create_text(text);
            engine.append_child(row, text);
            engine.append_child(content, row);
        }
        engine.append_child(viewport, content);
        engine.set_root(viewport);
        (engine, viewport)
    }

    #[test]
    fn scroll_offset_render_does_not_recompute_layout() {
        let (mut engine, viewport) = scroll_engine();
        let first = engine.render_frame(5, 1).unwrap();
        assert_eq!(first.cell(0, 0).unwrap().character, 'a');
        let passes = engine.layout_passes();

        engine.set_scroll_offset(viewport, 0, 1);
        let second = engine.render_frame(5, 1).unwrap();

        assert_eq!(engine.layout_passes(), passes);
        assert_eq!(second.cell(0, 0).unwrap().character, 'b');
    }

    #[test]
    fn render_clamps_scroll_offset_after_viewport_grows() {
        let mut engine = PaintEngine::new();
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = engine.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = engine.create_element(content_style);
        for index in 0..10 {
            let row =
                engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
            let text = engine.create_text(format!("{index}{index}{index}{index}{index}"));
            engine.append_child(row, text);
            engine.append_child(content, row);
        }
        engine.append_child(viewport, content);
        engine.set_root(viewport);

        engine.render_frame(5, 3).unwrap();
        engine.set_scroll_offset_for_size(viewport, 0, 100, 5, 3);
        let small = engine.render_frame(5, 3).unwrap();
        assert_eq!(small.cell(0, 0).unwrap().character, '7');

        let large = engine.render_frame(5, 8).unwrap();
        assert_eq!(large.cell(0, 0).unwrap().character, '2');
        assert_eq!(large.cell(0, 7).unwrap().character, '9');
    }

    #[test]
    fn scroll_metrics_query_before_first_render_computes_layout() {
        let mut engine = PaintEngine::new();

        let mut root_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        root_style.display = crate::style::LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = engine.create_element(root_style);

        let header = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(2.0),
        ));

        let mut body_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        body_style.display = crate::style::LayoutDisplay::Flex;
        body_style.flex_direction = LayoutFlexDirection::Row;
        body_style.flex_grow = 1.0;
        body_style.flex_shrink = 1.0;
        body_style.flex_basis = CssDimension::Length(0.0);
        let body = engine.create_element(body_style);

        let viewport = engine.create_element(block_style(
            CssDimension::Percent(0.8),
            CssDimension::Percent(1.0),
        ));
        let rail = engine.create_element(block_style(
            CssDimension::Percent(0.2),
            CssDimension::Percent(1.0),
        ));
        let scrollbar = engine.create_text("#");
        engine.append_child(rail, scrollbar);

        engine.append_child(body, viewport);
        engine.append_child(body, rail);
        engine.append_child(root, header);
        engine.append_child(root, body);
        engine.set_root(root);

        let rail_metrics = engine.scroll_metrics_for_size(rail, 80, 24).unwrap();
        let viewport_metrics = engine.scroll_metrics_for_size(viewport, 80, 24).unwrap();

        assert_eq!(rail_metrics.client_height, 22);
        assert_eq!(viewport_metrics.client_height, 22);
        assert_eq!(engine.layout_passes(), 1);
    }

    #[test]
    fn scrollbar_hit_testing_uses_rendered_regions_and_suppresses_selection() {
        let mut engine = PaintEngine::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.overflow_x = LayoutOverflow::Scroll;
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = engine.create_element(viewport_style);
        let child = engine.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(5.0),
        ));
        engine.append_child(viewport, child);
        engine.set_root(viewport);

        engine.render_frame(6, 3).unwrap();

        let hit = engine.scrollbar_hit_at(5, 1).unwrap();
        assert_eq!(hit.target_id, viewport);
        assert_eq!(hit.axis, ScrollbarAxis::Vertical);

        assert!(scrollbar_suppresses_selection(
            &mut engine,
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Down,
                x: 5,
                y: 1,
                button: 0,
            },
        ));
        assert!(scrollbar_suppresses_selection(
            &mut engine,
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Drag,
                x: 5,
                y: 0,
                button: 0,
            },
        ));
        assert!(scrollbar_suppresses_selection(
            &mut engine,
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Up,
                x: 5,
                y: 0,
                button: 0,
            },
        ));
    }

    #[test]
    fn viewport_automatically_enables_scrollbars_for_descendant_overflow() {
        let mut engine = PaintEngine::new();
        let viewport = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let root = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let content = engine.create_element(block_style(
            CssDimension::Length(20.0),
            CssDimension::Length(10.0),
        ));
        assert!(engine.append_child(root, content));
        assert!(engine.append_child(viewport, root));
        assert!(engine.set_viewport(viewport));
        assert!(engine.set_root(viewport));

        engine.render_frame(10, 4).unwrap();

        let metrics = engine.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_width, 9);
        assert_eq!(metrics.client_height, 3);
        assert_eq!(metrics.scroll_width, 20);
        assert_eq!(metrics.scroll_height, 10);
        let vertical = engine.scrollbar_hit_at(9, 1).unwrap();
        assert_eq!(vertical.target_id, viewport);
        assert_eq!(vertical.axis, ScrollbarAxis::Vertical);
        let horizontal = engine.scrollbar_hit_at(2, 3).unwrap();
        assert_eq!(horizontal.target_id, viewport);
        assert_eq!(horizontal.axis, ScrollbarAxis::Horizontal);

        let layout_passes = engine.layout_passes();
        let thumb_color = Background::Rgb(56, 189, 248);
        assert!(engine.mutate_style(
            root,
            StyleMutation::ScrollbarColor(ScrollbarColor::Colors {
                thumb: thumb_color,
                track: Background::Rgb(17, 24, 39),
            }),
        ));
        let recolored = engine.render_frame(10, 4).unwrap();
        assert_eq!(engine.layout_passes(), layout_passes);
        assert_eq!(recolored.cell(9, 0).unwrap().background, thumb_color);

        engine.set_scroll_offset(viewport, 8, 6).unwrap();
        engine.render_frame(30, 12).unwrap();
        let resized = engine.scroll_metrics(viewport).unwrap();
        assert_eq!(resized.scroll_left, 0);
        assert_eq!(resized.scroll_top, 0);
        assert!(engine.scrollbar_hit_at(29, 1).is_none());
        assert!(engine.scrollbar_hit_at(1, 11).is_none());
    }

    #[test]
    fn viewport_does_not_render_scrollbars_when_content_fits() {
        let mut engine = PaintEngine::new();
        let viewport = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let content = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        assert!(engine.append_child(viewport, content));
        assert!(engine.set_viewport(viewport));
        assert!(engine.set_root(viewport));

        engine.render_frame(10, 4).unwrap();

        assert!(engine.scrollbar_hit_at(9, 1).is_none());
        let viewport_node = engine.node_for(viewport).unwrap();
        let style = engine.arena.style(viewport_node);
        assert!(style.overflow_x == LayoutOverflow::Hidden);
        assert!(style.overflow_y == LayoutOverflow::Hidden);
    }

    #[test]
    fn viewport_frame_is_unchanged_when_scrolling_past_the_end() {
        let mut engine = PaintEngine::new();
        let viewport = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let root = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = engine.create_element(content_style);
        for index in 0..80 {
            let row = engine.create_element(block_style(
                CssDimension::Percent(1.0),
                CssDimension::Length(1.0),
            ));
            let text = engine.create_text(format!("{index:02}"));
            assert!(engine.append_child(row, text));
            assert!(engine.append_child(content, row));
        }
        assert!(engine.append_child(root, content));
        assert!(engine.append_child(viewport, root));
        assert!(engine.set_viewport(viewport));
        assert!(engine.set_root(viewport));

        engine.render_frame(10, 4).unwrap();
        let metrics = engine.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_height, 4);
        assert_eq!(metrics.scroll_height, 80);
        let max_top = metrics.scroll_height - metrics.client_height;
        let at_max_metrics = engine.set_scroll_offset(viewport, 0, max_top).unwrap();
        let at_max = engine.render_frame(10, 4).unwrap();

        let past_end_metrics = engine
            .set_scroll_offset(viewport, 0, max_top.saturating_add(3))
            .unwrap();
        let past_end = engine.render_frame(10, 4).unwrap();

        assert_eq!(at_max_metrics, past_end_metrics);
        for y in 0..at_max.height() {
            for x in 0..at_max.width() {
                assert_eq!(at_max.cell(x, y), past_end.cell(x, y));
            }
        }
        assert!((0..past_end.height())
            .flat_map(|y| (0..past_end.width()).map(move |x| (x, y)))
            .any(|(x, y)| past_end
                .cell(x, y)
                .is_some_and(|cell| cell.character != ' ')));
    }

    #[test]
    fn selection_in_later_scroll_pane_does_not_capture_hidden_text_from_earlier_pane() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(2.0));
        root_style.display = crate::style::LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Row;
        let root = engine.create_element(root_style);

        let mut left_style = block_style(CssDimension::Length(10.0), CssDimension::Length(2.0));
        left_style.overflow_y = LayoutOverflow::Scroll;
        let left = engine.create_element(left_style);
        let mut left_content_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        left_content_style.display = crate::style::LayoutDisplay::Flex;
        left_content_style.flex_direction = LayoutFlexDirection::Column;
        let left_content = engine.create_element(left_content_style);
        for index in 0..8 {
            let row = engine.create_element(block_style(
                CssDimension::Length(10.0),
                CssDimension::Length(1.0),
            ));
            let text = engine.create_text(format!("left-{index}"));
            engine.append_child(row, text);
            engine.append_child(left_content, row);
        }
        engine.append_child(left, left_content);

        let mut right_style = block_style(CssDimension::Length(10.0), CssDimension::Length(2.0));
        right_style.overflow_y = LayoutOverflow::Scroll;
        let right = engine.create_element(right_style);
        let mut right_content_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        right_content_style.display = crate::style::LayoutDisplay::Flex;
        right_content_style.flex_direction = LayoutFlexDirection::Column;
        let right_content = engine.create_element(right_content_style);
        for text in ["RIGHT-0", "RIGHT-1"] {
            let row = engine.create_element(block_style(
                CssDimension::Length(10.0),
                CssDimension::Length(1.0),
            ));
            let text = engine.create_text(text);
            engine.append_child(row, text);
            engine.append_child(right_content, row);
        }
        engine.append_child(right, right_content);

        engine.append_child(root, left);
        engine.append_child(root, right);
        engine.set_root(root);

        engine.render_frame(20, 2).unwrap();
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Down,
            x: 10,
            y: 0,
            button: 0,
        });
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Drag,
            x: 16,
            y: 0,
            button: 0,
        });
        engine.render_frame(20, 2).unwrap();

        let action = engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Up,
            x: 16,
            y: 0,
            button: 0,
        });

        assert_eq!(
            action,
            SelectionAction::CopyToClipboard("RIGHT-0".to_string())
        );
    }

    #[test]
    fn selection_drag_below_scroll_pane_scrolls_pane_before_selecting_outside() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(4.0));
        root_style.display = crate::style::LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = engine.create_element(root_style);

        let mut viewport_style = block_style(CssDimension::Length(12.0), CssDimension::Length(2.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = engine.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Length(12.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = engine.create_element(content_style);
        for index in 0..6 {
            let row = engine.create_element(block_style(
                CssDimension::Length(12.0),
                CssDimension::Length(1.0),
            ));
            let text = engine.create_text(format!("RIGHT-{index}"));
            engine.append_child(row, text);
            engine.append_child(content, row);
        }
        engine.append_child(viewport, content);

        let footer = engine.create_element(block_style(
            CssDimension::Length(12.0),
            CssDimension::Length(2.0),
        ));
        let footer_text = engine.create_text("FOOTER");
        engine.append_child(footer, footer_text);

        engine.append_child(root, viewport);
        engine.append_child(root, footer);
        engine.set_root(root);

        engine.render_frame(12, 4).unwrap();
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Down,
            x: 0,
            y: 0,
            button: 0,
        });
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Drag,
            x: 6,
            y: 2,
            button: 0,
        });
        let action = engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Up,
            x: 6,
            y: 2,
            button: 0,
        });

        assert_eq!(
            action,
            SelectionAction::CopyToClipboard(
                ["RIGHT-0", "RIGHT-1", "RIGHT-2", "RIGHT-3"].join("\n")
            )
        );
    }

    #[test]
    fn selection_drag_right_of_scroll_pane_scrolls_pane_before_selecting_outside() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(2.0));
        root_style.display = crate::style::LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Row;
        let root = engine.create_element(root_style);

        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(2.0));
        viewport_style.overflow_x = LayoutOverflow::Scroll;
        let viewport = engine.create_element(viewport_style);
        let content = engine.create_element(block_style(
            CssDimension::Length(12.0),
            CssDimension::Length(1.0),
        ));
        let content_text = engine.create_text("ABCDEFGHIJKL");
        engine.append_child(content, content_text);
        engine.append_child(viewport, content);

        let sibling = engine.create_element(block_style(
            CssDimension::Length(6.0),
            CssDimension::Length(2.0),
        ));
        let sibling_text = engine.create_text("OUT");
        engine.append_child(sibling, sibling_text);

        engine.append_child(root, viewport);
        engine.append_child(root, sibling);
        engine.set_root(root);

        engine.render_frame(12, 2).unwrap();
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Down,
            x: 0,
            y: 0,
            button: 0,
        });
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Drag,
            x: 8,
            y: 0,
            button: 0,
        });
        let action = engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Up,
            x: 8,
            y: 0,
            button: 0,
        });

        assert_eq!(
            action,
            SelectionAction::CopyToClipboard("ABCDEFGHIJKL".to_string())
        );
    }

    #[test]
    fn percent_scroll_demo_keeps_widths_after_scroll_text_updates() {
        let mut engine = PaintEngine::new();

        let mut root_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        root_style.display = crate::style::LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        root_style.background = Background::Black;
        let root = engine.create_element(root_style);

        let mut header_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(0.1));
        header_style.background = Background::Cyan;
        let header = engine.create_element(header_style);
        let status = engine.create_text(
            "Percent scroll demo. Resize the terminal; wheel over the panel. Ctrl-C exits.",
        );
        engine.append_child(header, status);

        let mut body_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(0.9));
        body_style.display = crate::style::LayoutDisplay::Flex;
        body_style.flex_direction = LayoutFlexDirection::Row;
        let body = engine.create_element(body_style);

        let mut viewport_style =
            block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.overflow_x = LayoutOverflow::Hidden;
        viewport_style.background = Background::Blue;
        let viewport = engine.create_element(viewport_style);

        let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = engine.create_element(content_style);
        let mut row_ids = Vec::new();
        for index in 1..=200 {
            let row =
                engine.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
            row_ids.push(row);
            let text = engine.create_text(format!(
                "percent row {index:02} - resize changes visible content"
            ));
            engine.append_child(row, text);
            engine.append_child(content, row);
        }

        engine.append_child(viewport, content);
        engine.append_child(body, viewport);
        engine.append_child(root, header);
        engine.append_child(root, body);
        engine.set_root(root);

        engine.render_frame(80, 24).unwrap();
        let viewport_node = engine.node_for(viewport).unwrap();
        let first_row_node = engine.node_for(row_ids[0]).unwrap();
        let fourth_row_node = engine.node_for(row_ids[3]).unwrap();
        let before_viewport = engine.arena.layout(viewport_node);
        assert_eq!(before_viewport.size.width, 80.0);
        assert_eq!(engine.arena.layout(first_row_node).size.width, 79.0);
        assert_eq!(engine.arena.layout(fourth_row_node).size.width, 79.0);

        let metrics = engine
            .set_scroll_offset_for_size(viewport, 0, 3, 80, 24)
            .unwrap();
        engine.set_text(
            status,
            format!(
                "scrollTop={}/{}, clientHeight={}",
                metrics.scroll_top, metrics.scroll_height, metrics.client_height
            ),
        );

        let frame = engine.render_frame(80, 24).unwrap();
        let after_viewport = engine.arena.layout(viewport_node);

        assert_eq!(after_viewport.size.width, 80.0);
        assert_eq!(engine.arena.layout(fourth_row_node).size.width, 79.0);
        let visible_row_prefix: String = (0..11)
            .map(|x| frame.cell(x, 2).unwrap().character)
            .collect();
        assert_eq!(visible_row_prefix, "percent row");
    }

    #[test]
    fn text_mutation_recomputes_layout() {
        let mut engine = PaintEngine::new();
        let root =
            engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
        let text = engine.create_text("short");
        engine.append_child(root, text);
        engine.set_root(root);

        engine.render_frame(5, 5).unwrap();
        let passes = engine.layout_passes();
        engine.set_text(text, "hello world");
        engine.render_frame(5, 5).unwrap();

        assert_eq!(engine.layout_passes(), passes + 1);
    }

    #[test]
    fn hit_testing_uses_last_rendered_regions() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        root_style.background = Background::Blue;
        let root = engine.create_element(root_style);
        engine.set_root(root);
        engine.render_frame(4, 1).unwrap();

        assert_eq!(engine.target_at(0, 0), Some(root));
        assert_eq!(engine.target_at(4, 0), None);
    }

    #[test]
    fn dom_ids_are_stable_and_not_reused_after_destroy() {
        let mut engine = PaintEngine::new();
        let first = engine.create_element(DivStyle::default());
        assert!(engine.destroy_node(first));

        let second = engine.create_element(DivStyle::default());

        assert_ne!(first, second);
        assert!(!engine.destroy_node(first));
        assert!(engine.set_root(second));
    }

    #[test]
    fn destroying_subtrees_reclaims_layout_nodes_for_future_trees() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(DivStyle::default());
        for index in 0..1_000 {
            let row = engine.create_element(DivStyle::default());
            let text = engine.create_text(format!("row {index}"));
            assert!(engine.append_child(row, text));
            assert!(engine.append_child(root, row));
        }

        assert_eq!(engine.arena.stats().node_count, 2_001);
        assert!(engine.destroy_node(root));
        assert_eq!(engine.arena.stats().node_count, 0);

        let replacement_root = engine.create_element(DivStyle::default());
        for index in 0..1_000 {
            let row = engine.create_element(DivStyle::default());
            let text = engine.create_text(format!("replacement {index}"));
            assert!(engine.append_child(row, text));
            assert!(engine.append_child(replacement_root, row));
        }

        let stats = engine.arena.stats();
        assert_eq!(stats.node_count, 2_001);
        assert_eq!(stats.allocated_slot_count, 2_001);
    }

    #[test]
    fn batched_sibling_destruction_preserves_survivors_and_reuses_slots() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(DivStyle::default());
        let survivor = engine.create_element(DivStyle::default());
        assert!(engine.append_child(root, survivor));

        let removed = (0..1_000)
            .map(|_| {
                let child = engine.create_element(DivStyle::default());
                assert!(engine.append_child(root, child));
                child
            })
            .collect::<Vec<_>>();
        let allocated_slots = engine.arena.stats().allocated_slot_count;

        assert!(apply_command(
            &mut engine,
            EngineCommand::Batch {
                commands: removed
                    .iter()
                    .map(|node| EngineCommand::DestroyNode { node: *node })
                    .collect(),
            },
        ));

        assert_eq!(engine.children.get(&root), Some(&vec![survivor]));
        assert_eq!(
            engine.arena.children(engine.node_for(root).unwrap()).len(),
            1
        );
        assert_eq!(engine.arena.stats().node_count, 2);
        for node in removed {
            assert!(engine.node_for(node).is_none());
        }

        for _ in 0..1_000 {
            engine.create_element(DivStyle::default());
        }
        assert_eq!(engine.arena.stats().allocated_slot_count, allocated_slots);
    }

    #[test]
    fn invalid_dom_ids_do_not_mutate_or_panic() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(2.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text("ok");
        let missing = DomId(99_999);
        assert!(engine.append_child(root, text));
        assert!(engine.set_root(root));

        assert!(!engine.append_child(root, missing));
        assert!(!engine.set_root(missing));
        assert!(!engine.set_text(missing, "nope"));
        assert!(!engine.set_style(missing, DivStyle::default()));
        assert_eq!(engine.scroll_metrics(missing), None);

        let frame = engine.render_frame(2, 1).unwrap();
        assert_eq!(frame.cell(0, 0).unwrap().character, 'o');
        assert_eq!(frame.cell(1, 0).unwrap().character, 'k');
    }

    #[test]
    fn destroying_child_detaches_it_from_layout_and_hit_testing() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        root_style.background = Background::Blue;
        let root = engine.create_element(root_style);

        let mut child_style = block_style(CssDimension::Length(1.0), CssDimension::Length(1.0));
        child_style.background = Background::Red;
        let child = engine.create_element(child_style);

        assert!(engine.append_child(root, child));
        assert!(engine.set_root(root));
        engine.render_frame(4, 1).unwrap();
        assert_eq!(engine.target_at(0, 0), Some(child));

        assert!(engine.destroy_node(child));
        engine.render_frame(4, 1).unwrap();

        assert_eq!(engine.target_at(0, 0), Some(root));
        assert!(!engine.set_root(child));
    }

    #[test]
    fn detaching_child_detaches_without_destroying_it() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(4.0),
            CssDimension::Length(1.0),
        ));
        let child = engine.create_element(block_style(
            CssDimension::Length(1.0),
            CssDimension::Length(1.0),
        ));

        assert!(engine.append_child(root, child));
        assert!(engine.set_root(root));
        engine.render_frame(4, 1).unwrap();

        assert!(engine.detach_node(child));
        engine.render_frame(4, 1).unwrap();
        assert_eq!(engine.target_at(0, 0), Some(root));

        assert!(engine.append_child(root, child));
        engine.render_frame(4, 1).unwrap();
        assert_eq!(engine.target_at(0, 0), Some(child));
    }

    #[test]
    fn destroying_root_clears_render_output() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(2.0),
            CssDimension::Length(1.0),
        ));
        assert!(engine.set_root(root));

        assert!(engine.render_frame(2, 1).is_some());
        assert!(engine.destroy_node(root));

        assert!(engine.render_frame(2, 1).is_none());
    }

    #[test]
    fn selection_action_uses_current_layout_frame() {
        let (mut engine, _viewport) = scroll_engine();
        engine.render_frame(5, 1).unwrap();

        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Down,
            x: 0,
            y: 0,
            button: 0,
        });
        assert_eq!(
            engine.handle_selection_event(SelectionMouseEvent {
                event_type: SelectionMouseEventType::Up,
                x: 4,
                y: 0,
                button: 0,
            }),
            SelectionAction::CopyToClipboard("aaaaa".to_string())
        );
    }

    #[test]
    fn render_to_diffs_against_previous_frame() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(4.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text("ab");
        engine.append_child(root, text);
        engine.set_root(root);

        let mut first = Vec::new();
        engine
            .render_to(4, 1, &mut first, TermProfile::NoColor, false)
            .unwrap();
        engine.set_text(text, "ac");
        let mut second = Vec::new();
        engine
            .render_to(4, 1, &mut second, TermProfile::NoColor, false)
            .unwrap();
        let second = String::from_utf8(second).unwrap();

        assert!(second.contains("\x1b[1;2Hc"));
        assert!(!second.contains("\x1b[H"));
    }

    #[test]
    fn render_to_flushes_output_after_writing_frame() {
        struct FlushProbe {
            bytes: Vec<u8>,
            flushes: usize,
        }

        impl std::io::Write for FlushProbe {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.bytes.extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.flushes += 1;
                Ok(())
            }
        }

        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(2.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text("ok");
        engine.append_child(root, text);
        engine.set_root(root);

        let mut out = FlushProbe {
            bytes: Vec::new(),
            flushes: 0,
        };
        engine
            .render_to(2, 1, &mut out, TermProfile::NoColor, false)
            .unwrap();

        assert!(String::from_utf8(out.bytes).unwrap().contains("ok"));
        assert_eq!(out.flushes, 1);
    }

    #[test]
    fn resize_recomputes_layout() {
        let mut engine = PaintEngine::new();
        let root =
            engine.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
        let text = engine.create_text("hello world");
        engine.append_child(root, text);
        engine.set_root(root);

        engine.render_frame(5, 5).unwrap();
        let passes = engine.layout_passes();
        engine.render_frame(10, 5).unwrap();

        assert_eq!(engine.layout_passes(), passes + 1);
    }

    #[test]
    fn invalidating_frame_forces_full_repaint() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(4.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text("ab");
        engine.append_child(root, text);
        engine.set_root(root);

        let mut first = Vec::new();
        engine
            .render_to(4, 1, &mut first, TermProfile::NoColor, false)
            .unwrap();
        engine.invalidate_frame();
        let mut second = Vec::new();
        engine
            .render_to(4, 1, &mut second, TermProfile::NoColor, false)
            .unwrap();
        let second = String::from_utf8(second).unwrap();

        assert!(second.contains("\x1b[H"));
    }

    #[test]
    fn text_attribute_change_paints_without_recomputing_layout() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(4.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text("ok");
        engine.append_child(root, text);
        engine.set_root(root);

        engine.render_frame(4, 1).unwrap();
        let passes = engine.layout_passes();
        assert!(engine.mutate_style(root, StyleMutation::FontWeight(CssFontWeight::Bold)));
        let frame = engine.render_frame(4, 1).unwrap();

        assert_eq!(engine.layout_passes(), passes);
        assert!(frame.cell(0, 0).unwrap().bold);
    }

    #[test]
    fn z_index_change_reorders_paint_without_recomputing_layout() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(4.0), CssDimension::Length(2.0));
        root_style.position = CssPosition::Relative;
        let root = engine.create_element(root_style);
        let mut red_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        red_style.position = CssPosition::Absolute;
        red_style.z_index = CssZIndex::Integer(1);
        red_style.background = Background::Red;
        let red = engine.create_element(red_style);
        let mut blue_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        blue_style.position = CssPosition::Absolute;
        blue_style.z_index = CssZIndex::Integer(2);
        blue_style.background = Background::Blue;
        let blue = engine.create_element(blue_style);
        engine.append_child(root, red);
        engine.append_child(root, blue);
        engine.set_root(root);

        let first = engine.render_frame(4, 2).unwrap();
        assert_eq!(first.cell(0, 0).unwrap().background, Background::Blue);
        let passes = engine.layout_passes();
        assert!(engine.mutate_style(red, StyleMutation::ZIndex(CssZIndex::Integer(3))));
        let second = engine.render_frame(4, 2).unwrap();

        assert_eq!(engine.layout_passes(), passes);
        assert_eq!(second.cell(0, 0).unwrap().background, Background::Red);
    }

    #[test]
    fn opacity_change_repaints_without_recomputing_layout() {
        let mut engine = PaintEngine::new();
        let mut style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        style.background = Background::Red;
        let root = engine.create_element(style);
        engine.set_root(root);

        engine.render_frame(2, 1).unwrap();
        let passes = engine.layout_passes();
        assert!(engine.mutate_style(root, StyleMutation::Opacity(0.5)));
        let frame = engine.render_frame(2, 1).unwrap();

        assert_eq!(engine.layout_passes(), passes);
        assert_eq!(
            frame.cell(0, 0).unwrap().background,
            Background::Rgb(128, 0, 0)
        );
    }

    #[test]
    fn opacity_transition_fades_in_as_a_stacking_context_without_recomputing_layout() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        root_style.background = Background::Blue;
        let root = engine.create_element(root_style);
        let mut child_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        child_style.background = Background::Red;
        child_style.opacity = 0.0;
        let child = engine.create_element(child_style.clone());
        engine.append_child(root, child);
        engine.set_root(root);
        let start = std::time::Instant::now();
        engine.render_frame_at(2, 1, start).unwrap();
        let passes = engine.layout_passes();

        engine.set_transition(
            child,
            vec![TransitionSpec {
                property: TransitionProperty::Opacity,
                duration_ms: 100,
            }],
        );
        child_style.opacity = 1.0;
        engine.set_style_at(child, child_style, start);

        assert!(engine.has_active_transitions());
        assert_eq!(
            engine.drain_transition_events(),
            vec![EngineTransitionEvent {
                event_type: TransitionEventType::Start,
                target: child,
                property: TransitionProperty::Opacity,
            }]
        );
        let midway = engine
            .render_frame_at(2, 1, start + std::time::Duration::from_millis(50))
            .unwrap();
        assert_eq!(engine.layout_passes(), passes);
        assert_eq!(
            midway.cell(0, 0).unwrap().background,
            Background::Rgb(128, 0, 128)
        );

        let finished = engine
            .render_frame_at(2, 1, start + std::time::Duration::from_millis(100))
            .unwrap();
        assert_eq!(engine.layout_passes(), passes);
        assert_eq!(finished.cell(0, 0).unwrap().background, Background::Red);
        assert!(!engine.has_active_transitions());
        assert_eq!(
            engine.drain_transition_events(),
            vec![EngineTransitionEvent {
                event_type: TransitionEventType::End,
                target: child,
                property: TransitionProperty::Opacity,
            }]
        );
    }

    #[test]
    fn initial_opacity_does_not_transition_from_the_internal_default() {
        let mut engine = PaintEngine::new();
        let mut root_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        root_style.background = Background::Blue;
        let root = engine.create_element(root_style);
        let mut overlay_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        overlay_style.background = Background::Black;
        let overlay = engine.create_element(overlay_style.clone());
        engine.append_child(root, overlay);
        engine.set_root(root);
        let start = std::time::Instant::now();

        engine.set_transition(
            overlay,
            vec![TransitionSpec {
                property: TransitionProperty::Opacity,
                duration_ms: 200,
            }],
        );
        overlay_style.opacity = 0.5;
        engine.set_style_at(overlay, overlay_style, start);

        let frame = engine.render_frame_at(2, 1, start).unwrap();
        assert_eq!(
            frame.cell(0, 0).unwrap().background,
            Background::Rgb(0, 0, 128)
        );
        assert!(engine.drain_transition_events().is_empty());
    }

    #[test]
    fn color_transition_paints_without_recomputing_layout() {
        let mut engine = PaintEngine::new();
        let mut style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        style.background = Background::Rgb(0, 0, 255);
        let root = engine.create_element(style.clone());
        engine.set_root(root);
        let start = std::time::Instant::now();
        engine.render_frame_at(4, 1, start).unwrap();
        let passes = engine.layout_passes();

        engine.set_transition(
            root,
            vec![TransitionSpec {
                property: TransitionProperty::BackgroundColor,
                duration_ms: 100,
            }],
        );
        style.background = Background::Rgb(0, 255, 255);
        engine.set_style_at(root, style, start);

        assert_eq!(engine.layout_passes(), passes);
        assert_eq!(
            engine.drain_transition_events(),
            vec![EngineTransitionEvent {
                event_type: TransitionEventType::Start,
                target: root,
                property: TransitionProperty::BackgroundColor,
            }]
        );

        let midway = engine
            .render_frame_at(4, 1, start + std::time::Duration::from_millis(50))
            .unwrap();
        let midway_background = midway.cell(0, 0).unwrap().background;
        assert_ne!(midway_background, Background::Rgb(0, 0, 255));
        assert_ne!(midway_background, Background::Rgb(0, 255, 255));
        assert_eq!(engine.layout_passes(), passes);

        let finished = engine
            .render_frame_at(4, 1, start + std::time::Duration::from_millis(100))
            .unwrap();
        assert_eq!(
            finished.cell(0, 0).unwrap().background,
            Background::Rgb(0, 255, 255)
        );
        assert_eq!(
            engine.drain_transition_events(),
            vec![EngineTransitionEvent {
                event_type: TransitionEventType::End,
                target: root,
                property: TransitionProperty::BackgroundColor,
            }]
        );
    }

    #[test]
    fn truecolor_disabled_skips_transition() {
        let mut engine = PaintEngine::new();
        engine.set_truecolor_enabled(false);
        let mut style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        style.color = Background::Rgb(255, 0, 0);
        let root = engine.create_element(style.clone());
        let text = engine.create_text("x");
        engine.append_child(root, text);
        engine.set_root(root);
        let start = std::time::Instant::now();
        engine.render_frame_at(2, 1, start).unwrap();

        engine.set_transition(
            root,
            vec![TransitionSpec {
                property: TransitionProperty::Color,
                duration_ms: 100,
            }],
        );
        style.color = Background::Rgb(0, 255, 0);
        engine.set_style_at(root, style, start);

        let frame = engine
            .render_frame_at(2, 1, start + std::time::Duration::from_millis(50))
            .unwrap();
        assert_eq!(
            frame.cell(0, 0).unwrap().foreground,
            Background::Rgb(0, 255, 0)
        );
        assert!(engine.drain_transition_events().is_empty());
    }

    #[test]
    fn input_value_command_updates_textarea_for_typescript_compatibility() {
        let mut engine = PaintEngine::new();
        let root = engine.create_element(block_style(
            CssDimension::Length(8.0),
            CssDimension::Length(2.0),
        ));
        let textarea = engine.create_textarea(
            block_style(CssDimension::Length(8.0), CssDimension::Auto),
            "",
        );
        engine.append_child(root, textarea);
        engine.set_root(root);

        assert!(engine.set_input_value(textarea, "hello", 5));
        let frame = engine.render_frame(8, 2).unwrap();

        assert_eq!(frame.cell(0, 0).unwrap().character, 'h');
        assert_eq!(frame.cell(4, 0).unwrap().character, 'o');
    }

    #[test]
    fn textarea_vertical_cursor_move_uses_soft_wrapped_rows() {
        let mut engine = PaintEngine::new();
        let textarea = engine.create_textarea(
            block_style(CssDimension::Length(6.0), CssDimension::Auto),
            "abcd efgh",
        );
        engine.set_root(textarea);
        engine.set_textarea_value(textarea, "abcd efgh", 7);
        engine.set_textarea_focused(textarea, true);

        assert_eq!(
            engine.move_textarea_cursor_vertically_for_size(textarea, -1, 6, 3),
            Some(2)
        );

        let frame = engine.render_frame(6, 3).unwrap();
        assert!(frame.cell(2, 0).unwrap().reversed);
    }

    #[test]
    fn textarea_cursor_visual_position_query_uses_current_layout_without_relayout() {
        let mut engine = PaintEngine::new();
        let textarea = engine.create_textarea(
            block_style(CssDimension::Length(4.0), CssDimension::Auto),
            "hahahaha",
        );
        engine.set_root(textarea);
        engine.set_textarea_value(textarea, "hahahaha", 5);

        assert_eq!(
            engine.textarea_cursor_visual_position_for_size(textarea, 8, 4),
            Some((1, 1))
        );
        let layout_passes = engine.layout_passes();

        assert_eq!(
            engine.textarea_cursor_visual_position_for_size(textarea, 8, 4),
            Some((1, 1))
        );
        assert_eq!(
            engine.textarea_visual_line_range_for_size(textarea, 1, 8, 4),
            Some((4, 8))
        );
        assert_eq!(engine.layout_passes(), layout_passes);
    }

    #[test]
    fn clicking_input_moves_cursor_to_clicked_column() {
        let mut engine = PaintEngine::new();
        let input = engine.create_input_with_id(
            DomId(1),
            block_style(CssDimension::Length(6.0), CssDimension::Length(1.0)),
            "abcdef",
        );
        engine.set_root(input);
        engine.set_input_focused(input, true);

        assert_eq!(
            engine.set_text_control_cursor_at_point_for_size(input, 3, 0, 6, 1),
            Some(3)
        );

        let frame = engine.render_frame(6, 1).unwrap();
        assert!(frame.cell(3, 0).unwrap().reversed);
    }

    #[test]
    fn terminal_focus_changes_cursor_without_recomputing_layout() {
        let mut engine = PaintEngine::new();
        let input = engine.create_input_with_id(
            DomId(1),
            block_style(CssDimension::Length(6.0), CssDimension::Length(1.0)),
            "abcdef",
        );
        engine.set_root(input);
        engine.set_input_focused(input, true);
        engine.set_input_value(input, "abcdef", 3);

        let focused_frame = engine.render_frame(6, 1).unwrap();
        assert!(focused_frame.cell(3, 0).unwrap().reversed);
        let layout_passes = engine.layout_passes();

        engine.set_terminal_focused(false);
        let blurred_frame = engine.render_frame(6, 1).unwrap();
        assert!(!blurred_frame.cell(3, 0).unwrap().reversed);
        assert_eq!(engine.layout_passes(), layout_passes);

        engine.set_terminal_focused(true);
        let refocused_frame = engine.render_frame(6, 1).unwrap();
        assert!(refocused_frame.cell(3, 0).unwrap().reversed);
        assert_eq!(engine.layout_passes(), layout_passes);
    }

    #[test]
    fn clicking_textarea_uses_soft_wrapped_visual_position() {
        let mut engine = PaintEngine::new();
        let textarea = engine.create_textarea(
            block_style(CssDimension::Length(6.0), CssDimension::Auto),
            "abcd efgh",
        );
        engine.set_root(textarea);
        engine.set_textarea_focused(textarea, true);

        assert_eq!(
            engine.set_text_control_cursor_at_point_for_size(textarea, 2, 1, 6, 3),
            Some(7)
        );

        let frame = engine.render_frame(6, 3).unwrap();
        assert!(frame.cell(2, 1).unwrap().reversed);
    }

    #[test]
    fn command_loop_creates_renders_and_hit_tests() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx));

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::CreateElement {
            style: block_style(CssDimension::Length(4.0), CssDimension::Length(1.0)),
            response: response_tx,
        })
        .unwrap();
        let root = response_rx.recv().unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::CreateText {
            text: "hi".to_string(),
            response: response_tx,
        })
        .unwrap();
        let text = response_rx.recv().unwrap();

        tx.send(EngineCommand::AppendChild {
            parent: root,
            child: text,
        })
        .unwrap();
        tx.send(EngineCommand::SetRoot { root }).unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::RenderFrame {
            width: 4,
            height: 1,
            response: response_tx,
        })
        .unwrap();
        let frame = response_rx.recv().unwrap().unwrap();
        assert_eq!(frame.cell(0, 0).unwrap().character, 'h');

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::HitTestPoint {
            x: 0,
            y: 0,
            response: response_tx,
        })
        .unwrap();
        assert_eq!(response_rx.recv().unwrap(), Some(root));

        tx.send(EngineCommand::Shutdown { response: None }).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn command_loop_batches_explicit_id_creates() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx));

        tx.send(EngineCommand::Batch {
            commands: vec![
                EngineCommand::CreateElementWithId {
                    id: DomId(1),
                    style: block_style(CssDimension::Length(4.0), CssDimension::Length(1.0)),
                },
                EngineCommand::CreateTextWithId {
                    id: DomId(2),
                    text: "ok".to_string(),
                },
                EngineCommand::AppendChild {
                    parent: DomId(1),
                    child: DomId(2),
                },
                EngineCommand::SetRoot { root: DomId(1) },
            ],
        })
        .unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::RenderFrame {
            width: 4,
            height: 1,
            response: response_tx,
        })
        .unwrap();
        let frame = response_rx.recv().unwrap().unwrap();

        assert_eq!(frame.cell(0, 0).unwrap().character, 'o');
        assert_eq!(frame.cell(1, 0).unwrap().character, 'k');

        tx.send(EngineCommand::Shutdown { response: None }).unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn command_loop_acknowledges_shutdown_after_earlier_commands() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx));

        tx.send(EngineCommand::CreateTextWithId {
            id: DomId(1),
            text: "queued before shutdown".to_string(),
        })
        .unwrap();
        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::Shutdown {
            response: Some(response_tx),
        })
        .unwrap();

        response_rx.recv().unwrap();
        thread.join().unwrap();
        assert!(tx.send(EngineCommand::InvalidateFrame).is_err());
    }

    #[test]
    fn command_loop_scroll_metrics_use_explicit_command_size() {
        let (tx, rx) = bounded(32);
        let thread = thread::spawn(move || engine_loop(rx));

        let mut viewport_style =
            block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        tx.send(EngineCommand::CreateElementWithId {
            id: DomId(1),
            style: viewport_style,
        })
        .unwrap();
        tx.send(EngineCommand::CreateElementWithId {
            id: DomId(2),
            style: block_style(CssDimension::Length(10.0), CssDimension::Length(20.0)),
        })
        .unwrap();
        tx.send(EngineCommand::AppendChild {
            parent: DomId(1),
            child: DomId(2),
        })
        .unwrap();
        tx.send(EngineCommand::SetRoot { root: DomId(1) }).unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::GetScrollMetrics {
            node: DomId(1),
            width: 10,
            height: 5,
            response: response_tx,
        })
        .unwrap();
        let small = response_rx.recv().unwrap().unwrap();

        let (response_tx, response_rx) = bounded(1);
        tx.send(EngineCommand::GetScrollMetrics {
            node: DomId(1),
            width: 10,
            height: 12,
            response: response_tx,
        })
        .unwrap();
        let large = response_rx.recv().unwrap().unwrap();

        assert_eq!(small.client_height, 5);
        assert_eq!(small.scroll_height, 20);
        assert_eq!(large.client_height, 12);
        assert_eq!(large.scroll_height, 20);

        tx.send(EngineCommand::Shutdown { response: None }).unwrap();
        thread.join().unwrap();
    }
}
