use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender as StdSender,
    Arc,
};
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};
use taffy::{AvailableSpace, NodeId, Size};
use termprofile::{DetectorSettings, TermProfile};

use crate::frame::Frame;
use crate::image::load_png_image;
use crate::layout::{ArenaScrollMetrics, LayoutArena};
use crate::paint::{paint_arena_with_options, HitRegion, PaintOptions};
use crate::selection::{SelectionAction, SelectionMouseEvent, SelectionState};
use crate::style::{
    Background, BorderStyle, ColorTransitionProperty, CssDimension, CssGridLine, CssGridPlacement,
    CssGridTemplateTrack, CssLengthPercentage, CssTrackSizing, CssWhiteSpace, CursorStyle,
    DivStyle, ImageRendering, LayoutAlignItems, LayoutDisplay, LayoutFlexDirection, LayoutFlexWrap,
    LayoutGridAutoFlow, LayoutJustifyContent, LayoutOverflow, TransitionSpec,
};
use crate::terminal::{copy_text_to_clipboard, query_terminal_size, write_pointer_shape};
use crate::transition::{TransitionEvent, TransitionEventType, TransitionState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DomId(pub(crate) u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EngineTransitionEvent {
    pub(crate) event_type: TransitionEventType,
    pub(crate) target: DomId,
    pub(crate) property: ColorTransitionProperty,
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

pub(crate) enum StyleMutation {
    Display(LayoutDisplay),
    Overflow(LayoutOverflow),
    OverflowX(LayoutOverflow),
    OverflowY(LayoutOverflow),
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
    Width(CssDimension),
    Height(CssDimension),
    MinHeight(CssDimension),
    Border(BorderStyle),
    BorderTop(BorderStyle),
    BorderRight(BorderStyle),
    BorderBottom(BorderStyle),
    BorderLeft(BorderStyle),
    BorderColor(Background),
    Color(Background),
    Background(Background),
    SelectionBackground(Background),
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
    SetRoot {
        root: DomId,
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
    SetTextAreaValue {
        node: DomId,
        value: String,
        cursor: u32,
    },
    SetTextAreaFocused {
        node: DomId,
        focused: bool,
    },
    MoveTextAreaCursorVertically {
        node: DomId,
        direction: i32,
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
    SetTruecolorEnabled {
        enabled: bool,
    },
    InvalidateFrame,
    Shutdown,
}

pub(crate) struct PaintEngine {
    arena: LayoutArena,
    root: Option<DomId>,
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
    current_pointer_shape: Option<&'static str>,
    last_pointer_position: Option<(u32, u32)>,
}

impl PaintEngine {
    pub(crate) fn new() -> Self {
        Self {
            arena: LayoutArena::new(),
            root: None,
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
            current_pointer_shape: None,
            last_pointer_position: None,
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
        let mut append_count = 0;
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
                EngineCommand::AppendChild { .. } => append_count += 1,
                _ => {}
            }
        }

        if create_count > 0 {
            self.arena.reserve_nodes(create_count);
            self.dom_to_node.reserve(create_count);
            self.node_to_dom.reserve(create_count);
        }
        if append_count > 0 {
            self.parents.reserve(append_count);
            self.children.reserve(append_count);
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

    pub(crate) fn set_root(&mut self, root: DomId) -> bool {
        if self.node_for(root).is_none() {
            return false;
        }
        self.root = Some(root);
        self.layout_dirty = true;
        true
    }

    pub(crate) fn destroy_node(&mut self, node: DomId) -> bool {
        let Some(node_id) = self.node_for(node) else {
            return false;
        };

        if let Some(parent) = self.parents.remove(&node) {
            if let Some(parent_node) = self.node_for(parent) {
                self.arena.remove_child(parent_node, node_id);
            }
            if let Some(siblings) = self.children.get_mut(&parent) {
                siblings.retain(|id| *id != node);
            }
        }

        self.delete_subtree(node);
        self.layout_dirty = true;
        true
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
        self.transitions.style_color_changed(
            node,
            ColorTransitionProperty::Color,
            previous.color,
            style.color,
            now,
            self.truecolor_enabled,
        );
        self.transitions.style_color_changed(
            node,
            ColorTransitionProperty::BackgroundColor,
            previous.background,
            style.background,
            now,
            self.truecolor_enabled,
        );
        self.transitions.style_color_changed(
            node,
            ColorTransitionProperty::BorderColor,
            previous.border_color,
            style.border_color,
            now,
            self.truecolor_enabled,
        );
        self.layout_dirty = self.layout_dirty
            || previous.to_taffy() != style.to_taffy()
            || previous.white_space != style.white_space;
        self.arena.set_style(node, style);
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
        let root = self.root.and_then(|root| self.node_for(root))?;
        let total_start = Instant::now();
        let layout_start = Instant::now();
        let layout_passes_before = self.layout_passes();
        self.ensure_layout(width, height, root);
        let stats = self.arena.stats();
        let layout_profile = self.arena.profile_stats();
        profile_log(
            "layout",
            layout_start.elapsed(),
            &[
                (
                    "dirty",
                    (layout_passes_before != self.layout_passes()).to_string(),
                ),
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

        let capture_hidden_selection_units = self.selection.active_selection().is_some();
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
        self.transitions.finish_completed(now);
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

    pub(crate) fn handle_pointer_move(&mut self, x: u32, y: u32) {
        self.last_pointer_position = Some((x, y));
        self.refresh_pointer_shape();
    }

    pub(crate) fn handle_selection_event(&mut self, event: SelectionMouseEvent) -> SelectionAction {
        self.selection
            .handle_event(event, self.current_frame.as_ref())
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

    fn ensure_layout(&mut self, width: usize, height: usize, root: NodeId) {
        let size = (width, height);
        if !self.layout_dirty && self.last_layout_size == Some(size) {
            return;
        }

        self.arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(width as f32),
                height: AvailableSpace::Definite(height as f32),
            },
        );
        self.layout_dirty = false;
        self.last_layout_size = Some(size);
    }

    fn ensure_layout_for_size(&mut self, width: usize, height: usize) {
        if let Some(root) = self.root.and_then(|root| self.node_for(root)) {
            self.ensure_layout(width, height, root);
        }
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

    fn style_for(&self, id: DomId) -> Option<&DivStyle> {
        self.node_for(id).map(|node| self.arena.style(node))
    }

    fn delete_subtree(&mut self, node: DomId) {
        if self.root == Some(node) {
            self.root = None;
        }

        let children = self.children.remove(&node).unwrap_or_default();
        for child in children {
            self.parents.remove(&child);
            self.delete_subtree(child);
        }

        if let Some(node_id) = self.dom_to_node.remove(&node) {
            self.node_to_dom.remove(&node_id);
            self.transitions.clear_node(node_id);
        }
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
        StyleMutation::Display(display) => style.display = display,
        StyleMutation::Overflow(overflow) => {
            style.overflow_x = overflow;
            style.overflow_y = overflow;
        }
        StyleMutation::OverflowX(overflow) => style.overflow_x = overflow,
        StyleMutation::OverflowY(overflow) => style.overflow_y = overflow,
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
        StyleMutation::Width(width) => style.width = width,
        StyleMutation::Height(height) => style.height = height,
        StyleMutation::MinHeight(min_height) => style.min_height = min_height,
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
        StyleMutation::Background(background) => style.background = background,
        StyleMutation::SelectionBackground(background) => {
            style.selection_background = Some(background);
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
            for command in commands {
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
        EngineCommand::SetRoot { root } => {
            engine.set_root(root);
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
        EngineCommand::MoveTextAreaCursorVertically {
            node,
            direction,
            response,
        } => {
            let size = query_terminal_size();
            let _ = response.send(engine.move_textarea_cursor_vertically_for_size(
                node,
                direction,
                size.cols as usize,
                size.rows as usize,
            ));
        }
        EngineCommand::SetScrollOffset {
            node,
            scroll_left,
            scroll_top,
            response,
        } => {
            let size = query_terminal_size();
            let _ = response.send(engine.set_scroll_offset_for_size(
                node,
                scroll_left,
                scroll_top,
                size.cols as usize,
                size.rows as usize,
            ));
        }
        EngineCommand::GetScrollMetrics { node, response } => {
            let size = query_terminal_size();
            let _ = response.send(engine.scroll_metrics_for_size(
                node,
                size.cols as usize,
                size.rows as usize,
            ));
        }
        EngineCommand::HitTestPoint { x, y, response } => {
            let _ = response.send(engine.target_at(x, y));
        }
        EngineCommand::HitTestClick { click, response } => {
            let _ = response.send(engine.click_event_for(click));
        }
        EngineCommand::HandleSelection { event, response } => {
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
        EngineCommand::SetTruecolorEnabled { enabled } => {
            engine.set_truecolor_enabled(enabled);
        }
        EngineCommand::InvalidateFrame => engine.invalidate_frame(),
        EngineCommand::Shutdown => return false,
    }

    true
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
        Background, ColorTransitionProperty, CssDimension, LayoutFlexDirection, LayoutOverflow,
        TransitionSpec,
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
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Length(1.0));
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
            block_style(CssDimension::Percent(0.85), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.overflow_x = LayoutOverflow::Hidden;
        viewport_style.background = Background::Blue;
        let viewport = engine.create_element(viewport_style);

        let mut rail_style = block_style(CssDimension::Percent(0.15), CssDimension::Percent(1.0));
        rail_style.background = Background::Magenta;
        rail_style.white_space = CssWhiteSpace::Pre;
        let rail = engine.create_element(rail_style);
        let scrollbar = engine.create_text("|");
        engine.append_child(rail, scrollbar);

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
        engine.append_child(body, rail);
        engine.append_child(root, header);
        engine.append_child(root, body);
        engine.set_root(root);

        engine.render_frame(80, 24).unwrap();
        let viewport_node = engine.node_for(viewport).unwrap();
        let rail_node = engine.node_for(rail).unwrap();
        let first_row_node = engine.node_for(row_ids[0]).unwrap();
        let fourth_row_node = engine.node_for(row_ids[3]).unwrap();
        let before_viewport = engine.arena.layout(viewport_node);
        let before_rail = engine.arena.layout(rail_node);
        assert_eq!(before_viewport.size.width, 68.0);
        assert_eq!(before_rail.location.x, 68.0);
        assert_eq!(before_rail.size.width, 12.0);
        assert_eq!(engine.arena.layout(first_row_node).size.width, 68.0);
        assert_eq!(engine.arena.layout(fourth_row_node).size.width, 68.0);

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
        engine.set_text(
            scrollbar,
            "|\n|\n#\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|\n|",
        );

        let frame = engine.render_frame(80, 24).unwrap();
        let after_viewport = engine.arena.layout(viewport_node);
        let after_rail = engine.arena.layout(rail_node);

        assert_eq!(after_viewport.size.width, 68.0);
        assert_eq!(after_rail.location.x, 68.0);
        assert_eq!(after_rail.size.width, 12.0);
        assert_eq!(engine.arena.layout(fourth_row_node).size.width, 68.0);
        let visible_row_prefix: String = (0..11)
            .map(|x| frame.cell(x, 2).unwrap().character)
            .collect();
        assert_eq!(visible_row_prefix, "percent row");
        assert_eq!(frame.cell(68, 2).unwrap().background, Background::Magenta);
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
                property: ColorTransitionProperty::BackgroundColor,
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
                property: ColorTransitionProperty::BackgroundColor,
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
                property: ColorTransitionProperty::BackgroundColor,
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
                property: ColorTransitionProperty::Color,
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

        tx.send(EngineCommand::Shutdown).unwrap();
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

        tx.send(EngineCommand::Shutdown).unwrap();
        thread.join().unwrap();
    }
}
