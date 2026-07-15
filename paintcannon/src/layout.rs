use std::{collections::HashSet, time::Instant};

use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_root_layout, AvailableSpace, Cache,
    CacheTree, CoreStyle, Layout, LayoutInput, LayoutOutput, LayoutPartialTree, Line, MaybeMath,
    MaybeResolve, NodeId, Point, Rect, RequestedAxis, ResolveOrZero, RoundTree, RunMode, Size,
    SizingMode, Style, TraversePartialTree, TraverseTree,
};

use crate::style::{
    BorderStyle, CssDimension, CssLengthPercentageAuto, CssPosition, CssWhiteSpace, CssZIndex,
    DivStyle, LayoutDisplay, LayoutOverflow,
};
use crate::text::{character_cell_width, parse_text_for_single_line, parse_text_for_white_space};
use crate::text_wrap::WrappedText;

#[derive(Clone)]
pub(crate) enum LayoutNodeKind {
    Element,
    Text(String),
    Image(ImageLayoutData),
    Input(InputLayoutData),
    TextArea(TextAreaLayoutData),
}

#[derive(Clone)]
pub(crate) struct ImageLayoutData {
    pub(crate) width_px: u32,
    pub(crate) height_px: u32,
    pub(crate) cell_width_px: u32,
    pub(crate) cell_height_px: u32,
    pub(crate) rgb: Option<Vec<u8>>,
}

#[derive(Clone)]
pub(crate) struct InputLayoutData {
    pub(crate) value: String,
    pub(crate) placeholder: String,
    pub(crate) cursor: u32,
    pub(crate) focused: bool,
}

#[derive(Clone)]
pub(crate) struct TextAreaLayoutData {
    pub(crate) value: String,
    pub(crate) placeholder: String,
    pub(crate) cursor: u32,
    pub(crate) focused: bool,
    pub(crate) scroll_cursor_dirty: bool,
}

pub(crate) struct LayoutArena {
    nodes: Vec<LayoutNode>,
    free_nodes: Vec<usize>,
    scroll_nodes: HashSet<NodeId>,
    absolute_nodes: HashSet<NodeId>,
    stacking_candidates: HashSet<NodeId>,
    dirty_textareas: HashSet<NodeId>,
    layout_passes: u64,
    layout_mode_stack: Vec<RunMode>,
    profile: LayoutProfileStats,
    last_root_layout: Option<(NodeId, Size<AvailableSpace>)>,
    stacking_tree_dirty: bool,
    stacking_tree_populated: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LayoutProfileStats {
    pub(crate) inline_width_calls: u64,
    pub(crate) inline_height_calls: u64,
    pub(crate) inline_fragment_calls: u64,
    pub(crate) inline_width_ns: u128,
    pub(crate) inline_height_ns: u128,
    pub(crate) inline_fragment_ns: u128,
    pub(crate) dirty_descendant_visits: u64,
    pub(crate) visible_overflow_visits: u64,
    pub(crate) scroll_clamp_visits: u64,
    pub(crate) dirty_textarea_visits: u64,
    pub(crate) absolute_layout_visits: u64,
    pub(crate) stacking_tree_visits: u64,
    pub(crate) taffy_ns: u128,
    pub(crate) dirty_descendants_ns: u128,
    pub(crate) visible_overflow_ns: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutStats {
    pub(crate) node_count: usize,
    pub(crate) allocated_slot_count: usize,
    pub(crate) inline_context_count: usize,
    pub(crate) inline_fragment_count: usize,
}

struct LayoutNode {
    occupied: bool,
    kind: LayoutNodeKind,
    style: DivStyle,
    taffy_style: Style,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
    stacking_children: Vec<NodeId>,
    has_relative_inline_descendants: bool,
    has_inline_stacking_contexts: bool,
    inline_static_position: Option<Point<u32>>,
    layout: Layout,
    cache: Cache,
    layout_dirty: bool,
    fragments_dirty: bool,
    post_layout_dirty: bool,
    visible_overflow_dirty: bool,
    scroll_left: u32,
    scroll_top: u32,
    fragments: Vec<InlineFragment>,
    measure_cache: InlineMeasureCache,
    visible_overflow_size: Size<f32>,
}

#[derive(Clone, Copy)]
struct ContentWidths {
    min: f32,
    max: f32,
}

#[derive(Default)]
struct InlineMeasureCache {
    width: Option<InlineWidthCacheEntry>,
    extra_widths: Vec<InlineWidthCacheEntry>,
    height: Option<InlineHeightCacheEntry>,
    extra_heights: Vec<InlineHeightCacheEntry>,
}

#[derive(Clone, Copy)]
struct InlineWidthCacheEntry {
    white_space: CssWhiteSpace,
    widths: ContentWidths,
}

#[derive(Clone, Copy)]
struct InlineHeightCacheEntry {
    white_space: CssWhiteSpace,
    width: u32,
    height: f32,
}

impl InlineMeasureCache {
    fn width(&self, white_space: CssWhiteSpace) -> Option<ContentWidths> {
        self.width
            .iter()
            .chain(&self.extra_widths)
            .find(|entry| entry.white_space == white_space)
            .map(|entry| entry.widths)
    }

    fn insert_width(&mut self, entry: InlineWidthCacheEntry) {
        if self.width.is_none() {
            self.width = Some(entry);
        } else {
            self.extra_widths.push(entry);
        }
    }

    fn height(&self, white_space: CssWhiteSpace, width: u32) -> Option<f32> {
        self.height
            .iter()
            .chain(&self.extra_heights)
            .find(|entry| entry.white_space == white_space && entry.width == width)
            .map(|entry| entry.height)
    }

    fn insert_height(&mut self, entry: InlineHeightCacheEntry) {
        if self.height.is_none() {
            self.height = Some(entry);
        } else {
            self.extra_heights.push(entry);
        }
    }

    fn clear(&mut self) {
        self.width = None;
        self.extra_widths.clear();
        self.height = None;
        self.extra_heights.clear();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArenaScrollMetrics {
    pub(crate) scroll_left: u32,
    pub(crate) scroll_top: u32,
    pub(crate) scroll_width: u32,
    pub(crate) scroll_height: u32,
    pub(crate) client_width: u32,
    pub(crate) client_height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollbarAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArenaScrollbarHit {
    pub(crate) node: NodeId,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InlineFragment {
    pub(crate) node: NodeId,
    pub(crate) hit_node: Option<NodeId>,
    pub(crate) kind: InlineFragmentKind,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InlineFragmentKind {
    Text {
        character: char,
        selection_order: usize,
    },
    Replaced,
}

impl LayoutArena {
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            scroll_nodes: HashSet::new(),
            absolute_nodes: HashSet::new(),
            stacking_candidates: HashSet::new(),
            dirty_textareas: HashSet::new(),
            layout_passes: 0,
            layout_mode_stack: Vec::new(),
            profile: LayoutProfileStats::default(),
            last_root_layout: None,
            stacking_tree_dirty: true,
            stacking_tree_populated: false,
        }
    }

    pub(crate) fn create_element(&mut self, style: DivStyle) -> NodeId {
        self.push_node(LayoutNodeKind::Element, style)
    }

    pub(crate) fn reserve_nodes(&mut self, additional: usize) {
        self.nodes
            .reserve(additional.saturating_sub(self.free_nodes.len()));
    }

    pub(crate) fn reserve_children(&mut self, node: NodeId, additional: usize) {
        let item = &mut self.nodes[node_index(node)];
        item.children.reserve(additional);
        item.stacking_children.reserve(additional);
    }

    pub(crate) fn create_text(&mut self, text: impl Into<String>) -> NodeId {
        self.push_node(LayoutNodeKind::Text(text.into()), DivStyle::default())
    }

    pub(crate) fn create_image(
        &mut self,
        style: DivStyle,
        width_px: u32,
        height_px: u32,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> NodeId {
        self.push_node(
            LayoutNodeKind::Image(ImageLayoutData {
                width_px,
                height_px,
                cell_width_px,
                cell_height_px,
                rgb: None,
            }),
            style,
        )
    }

    pub(crate) fn create_input(&mut self, style: DivStyle, value: impl Into<String>) -> NodeId {
        self.push_node(
            LayoutNodeKind::Input(InputLayoutData {
                value: value.into(),
                placeholder: String::new(),
                cursor: 0,
                focused: false,
            }),
            style,
        )
    }

    pub(crate) fn create_textarea(&mut self, style: DivStyle, value: impl Into<String>) -> NodeId {
        self.push_node(
            LayoutNodeKind::TextArea(TextAreaLayoutData {
                value: value.into(),
                placeholder: String::new(),
                cursor: 0,
                focused: false,
                scroll_cursor_dirty: false,
            }),
            style,
        )
    }

    #[cfg(test)]
    pub(crate) fn set_image_pixels(
        &mut self,
        node: NodeId,
        width_px: u32,
        height_px: u32,
        rgb: Vec<u8>,
    ) {
        self.set_image_pixels_and_cell_size(node, width_px, height_px, 1, 1, rgb);
    }

    pub(crate) fn set_image_pixels_and_cell_size(
        &mut self,
        node: NodeId,
        width_px: u32,
        height_px: u32,
        cell_width_px: u32,
        cell_height_px: u32,
        rgb: Vec<u8>,
    ) {
        let item = &mut self.nodes[node_index(node)];
        if let LayoutNodeKind::Image(image) = &mut item.kind {
            image.width_px = width_px;
            image.height_px = height_px;
            image.cell_width_px = cell_width_px.max(1);
            image.cell_height_px = cell_height_px.max(1);
            image.rgb = Some(rgb);
            self.clear_cache_from(node);
        }
    }

    pub(crate) fn set_text(&mut self, node: NodeId, text: impl Into<String>) {
        let item = &mut self.nodes[node_index(node)];
        if let LayoutNodeKind::Text(value) = &mut item.kind {
            *value = text.into();
            self.clear_cache_from(node);
        }
    }

    pub(crate) fn set_input_value(&mut self, node: NodeId, value: impl Into<String>, cursor: u32) {
        let item = &mut self.nodes[node_index(node)];
        if let LayoutNodeKind::Input(input) = &mut item.kind {
            input.value = value.into();
            input.cursor = cursor;
            self.clear_cache_from(node);
        }
    }

    pub(crate) fn set_input_focused(&mut self, node: NodeId, focused: bool) {
        if let LayoutNodeKind::Input(input) = &mut self.nodes[node_index(node)].kind {
            input.focused = focused;
        }
    }

    pub(crate) fn set_input_placeholder(&mut self, node: NodeId, placeholder: impl Into<String>) {
        if let LayoutNodeKind::Input(input) = &mut self.nodes[node_index(node)].kind {
            input.placeholder = placeholder.into();
        }
    }

    pub(crate) fn set_textarea_value(
        &mut self,
        node: NodeId,
        value: impl Into<String>,
        cursor: u32,
    ) {
        let item = &mut self.nodes[node_index(node)];
        if let LayoutNodeKind::TextArea(textarea) = &mut item.kind {
            textarea.value = value.into();
            textarea.cursor = cursor;
            textarea.scroll_cursor_dirty = true;
            self.dirty_textareas.insert(node);
            self.clear_cache_from(node);
        }
    }

    pub(crate) fn set_textarea_focused(&mut self, node: NodeId, focused: bool) {
        if let LayoutNodeKind::TextArea(textarea) = &mut self.nodes[node_index(node)].kind {
            textarea.focused = focused;
            if focused {
                textarea.scroll_cursor_dirty = true;
                self.dirty_textareas.insert(node);
            }
        }
    }

    pub(crate) fn set_textarea_placeholder(
        &mut self,
        node: NodeId,
        placeholder: impl Into<String>,
    ) {
        if let LayoutNodeKind::TextArea(textarea) = &mut self.nodes[node_index(node)].kind {
            textarea.placeholder = placeholder.into();
        }
    }

    pub(crate) fn move_textarea_cursor_vertically(
        &mut self,
        node: NodeId,
        direction: i32,
    ) -> Option<u32> {
        let index = node_index(node);
        let layout = self.layout(node);
        let wrap_width = float_to_cells(layout.content_box_size().width).max(1) as usize;
        let LayoutNodeKind::TextArea(textarea) = &mut self.nodes[index].kind else {
            return None;
        };
        let wrapped = WrappedText::new(&textarea.value, wrap_width);
        let next = wrapped.cursor_after_vertical_move(textarea.cursor as usize, direction);
        let next = next.min(textarea.value.chars().count()) as u32;
        textarea.cursor = next;
        textarea.scroll_cursor_dirty = true;
        self.dirty_textareas.insert(node);
        Some(next)
    }

    pub(crate) fn textarea_cursor_visual_position(&self, node: NodeId) -> Option<(u32, u32)> {
        let LayoutNodeKind::TextArea(textarea) = &self.nodes[node_index(node)].kind else {
            return None;
        };
        let wrap_width = float_to_cells(self.layout(node).content_box_size().width) as usize;
        if wrap_width == 0 {
            return None;
        }

        let (row, column) =
            WrappedText::new(&textarea.value, wrap_width).cursor_position(textarea.cursor as usize);
        Some((
            row.min(u32::MAX as usize) as u32,
            column.min(u32::MAX as usize) as u32,
        ))
    }

    pub(crate) fn textarea_visual_line_range(&self, node: NodeId, row: u32) -> Option<(u32, u32)> {
        let LayoutNodeKind::TextArea(textarea) = &self.nodes[node_index(node)].kind else {
            return None;
        };
        let wrap_width = float_to_cells(self.layout(node).content_box_size().width) as usize;
        if wrap_width == 0 {
            return None;
        }

        let range =
            WrappedText::new(&textarea.value, wrap_width).visual_line_range(row as usize)?;
        Some((
            range.start.min(u32::MAX as usize) as u32,
            range.end.min(u32::MAX as usize) as u32,
        ))
    }

    pub(crate) fn set_text_control_cursor_at_point(
        &mut self,
        node: NodeId,
        x: u32,
        y: u32,
    ) -> Option<u32> {
        let rect = self.content_box_absolute_rect(node);
        if rect.width == 0 || rect.height == 0 {
            return None;
        }

        let local_x = (x as i32 - rect.left).clamp(0, rect.width.saturating_sub(1) as i32) as usize;
        let local_y = (y as i32 - rect.top).clamp(0, rect.height.saturating_sub(1) as i32) as usize;
        let index = node_index(node);
        let scroll_top = self.nodes[index].scroll_top as usize;
        match &mut self.nodes[index].kind {
            LayoutNodeKind::Input(input) => {
                let value_len = input.value.chars().count();
                let width = rect.width.max(1);
                let cursor = (input.cursor as usize).min(value_len);
                let start = if input.focused && value_len > 0 && cursor >= width {
                    cursor + 1 - width
                } else {
                    0
                };
                let next = (start + local_x).min(value_len) as u32;
                input.cursor = next;
                Some(next)
            }
            LayoutNodeKind::TextArea(textarea) => {
                let wrap_width = rect.width.max(1);
                let value_len = textarea.value.chars().count();
                if value_len == 0 {
                    textarea.cursor = 0;
                    return Some(0);
                }

                let layout = WrappedText::new(&textarea.value, wrap_width);
                let next = layout
                    .cursor_for_visual_position(scroll_top + local_y, local_x)
                    .unwrap_or(value_len)
                    .min(value_len) as u32;
                textarea.cursor = next;
                textarea.scroll_cursor_dirty = false;
                self.dirty_textareas.remove(&node);
                Some(next)
            }
            LayoutNodeKind::Element | LayoutNodeKind::Text(_) | LayoutNodeKind::Image(_) => None,
        }
    }

    pub(crate) fn scrollbar_hit_for_point(
        &self,
        node: NodeId,
        x: u32,
        y: u32,
    ) -> Option<ArenaScrollbarHit> {
        let index = node_index(node);
        if !matches!(self.nodes[index].kind, LayoutNodeKind::Element) {
            return None;
        }

        let style = self.style(node);
        let layout = self.layout(node);
        let bounds = self.absolute_border_rect(node);
        let metrics = self.scroll_metrics_snapshot(node)?;
        let x = x.min(i32::MAX as u32) as i32;
        let y = y.min(i32::MAX as u32) as i32;

        if style.overflow_y == LayoutOverflow::Scroll && layout.scrollbar_size.width >= 0.5 {
            if let Some(rail) = vertical_scrollbar_rect(bounds, layout) {
                if rail.contains(x, y) {
                    return Some(vertical_scrollbar_hit(node, rail, &metrics));
                }
            }
        }

        if style.overflow_x == LayoutOverflow::Scroll && layout.scrollbar_size.height >= 0.5 {
            if let Some(rail) = horizontal_scrollbar_rect(bounds, layout) {
                if rail.contains(x, y) {
                    return Some(horizontal_scrollbar_hit(node, rail, &metrics));
                }
            }
        }

        None
    }

    pub(crate) fn scrollport_absolute_rect(&self, node: NodeId) -> Option<AbsoluteRect> {
        let index = node_index(node);
        if !matches!(
            self.nodes[index].kind,
            LayoutNodeKind::Element | LayoutNodeKind::TextArea(_)
        ) {
            return None;
        }

        Some(absolute_scrollport_rect(
            self.absolute_border_rect(node),
            self.layout(node),
        ))
    }

    pub(crate) fn append_child(&mut self, parent: NodeId, child: NodeId) {
        let previous_parent = self.nodes[node_index(child)].parent;
        if let Some(previous_parent) = previous_parent {
            let children = &mut self.nodes[node_index(previous_parent)].children;
            if let Some(index) = children.iter().position(|id| *id == child) {
                children.remove(index);
                self.clear_cache_from(previous_parent);
            }
        }
        if previous_parent.is_some_and(|previous_parent| previous_parent != parent) {
            self.clear_cache_subtree(child);
        }
        self.nodes[node_index(parent)].children.push(child);
        self.nodes[node_index(child)].parent = Some(parent);
        self.stacking_tree_dirty = true;
        self.clear_cache_from(parent);
    }

    pub(crate) fn insert_child_before(&mut self, parent: NodeId, child: NodeId, before: NodeId) {
        if child == before {
            return;
        }

        let child_index = node_index(child);
        if let Some(previous_parent) = self.nodes[child_index].parent {
            let children = &mut self.nodes[node_index(previous_parent)].children;
            if let Some(index) = children.iter().position(|id| *id == child) {
                children.remove(index);
                self.clear_cache_from(previous_parent);
            }
            if previous_parent != parent {
                self.clear_cache_subtree(child);
            }
        }

        let parent_index = node_index(parent);
        let children = &mut self.nodes[parent_index].children;
        if let Some(index) = children.iter().position(|id| *id == child) {
            children.remove(index);
        }
        let index = children
            .iter()
            .position(|id| *id == before)
            .unwrap_or(children.len());
        children.insert(index, child);
        self.nodes[child_index].parent = Some(parent);
        self.stacking_tree_dirty = true;
        self.clear_cache_from(parent);
    }

    pub(crate) fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        let children = &mut self.nodes[node_index(parent)].children;
        if let Some(index) = children.iter().position(|id| *id == child) {
            children.remove(index);
            self.nodes[node_index(child)].parent = None;
            self.stacking_tree_dirty = true;
            self.clear_cache_subtree(child);
            self.clear_cache_from(parent);
        }
    }

    pub(crate) fn remove_children(
        &mut self,
        parent: NodeId,
        removed: &std::collections::HashSet<NodeId>,
    ) {
        let children = &mut self.nodes[node_index(parent)].children;
        let original_len = children.len();
        children.retain(|child| !removed.contains(child));
        if children.len() == original_len {
            return;
        }
        for child in removed {
            let child_index = node_index(*child);
            if self.nodes[child_index].parent == Some(parent) {
                self.nodes[child_index].parent = None;
            }
        }
        self.stacking_tree_dirty = true;
        self.clear_cache_from(parent);
    }

    pub(crate) fn remove_node(&mut self, node: NodeId) {
        let index = node_index(node);
        self.scroll_nodes.remove(&node);
        self.absolute_nodes.remove(&node);
        self.stacking_candidates.remove(&node);
        self.dirty_textareas.remove(&node);
        let item = &mut self.nodes[index];
        debug_assert!(item.occupied, "layout node removed twice");
        item.occupied = false;
        item.kind = LayoutNodeKind::Element;
        item.style = DivStyle::default();
        item.taffy_style = item.style.to_taffy();
        item.children.clear();
        item.parent = None;
        item.stacking_children.clear();
        item.layout = Layout::new();
        item.cache.clear();
        item.layout_dirty = true;
        item.fragments_dirty = true;
        item.post_layout_dirty = true;
        item.visible_overflow_dirty = true;
        item.fragments.clear();
        item.measure_cache = InlineMeasureCache::default();
        item.visible_overflow_size = Size::ZERO;
        self.stacking_tree_dirty = true;
        self.free_nodes.push(index);
    }

    pub(crate) fn set_style(&mut self, node: NodeId, style: DivStyle) {
        let item = &mut self.nodes[node_index(node)];
        let taffy_style = style.to_taffy();
        let layout_changed = item.taffy_style != taffy_style
            || item.style.white_space != style.white_space
            || item.style.position != style.position;
        if item.style.position != style.position
            || item.style.z_index != style.z_index
            || (item.style.opacity < 1.0) != (style.opacity < 1.0)
        {
            self.stacking_tree_dirty = true;
        }
        item.taffy_style = taffy_style;
        item.style = style;
        let is_absolute = item.style.position == CssPosition::Absolute;
        if is_absolute {
            self.absolute_nodes.insert(node);
        } else {
            self.absolute_nodes.remove(&node);
        }
        if item.style.position != CssPosition::Static
            || item.style.z_index != CssZIndex::Auto
            || item.style.opacity < 1.0
        {
            self.stacking_candidates.insert(node);
        } else {
            self.stacking_candidates.remove(&node);
        }
        self.update_scroll_node(node);
        if layout_changed {
            self.clear_cache_subtree_and_ancestors(node);
        }
    }

    pub(crate) fn compute_layout(&mut self, root: NodeId, available: Size<AvailableSpace>) {
        self.layout_passes += 1;
        self.profile = LayoutProfileStats::default();
        if self.stacking_tree_dirty {
            self.rebuild_stacking_tree(root);
        }
        if self.last_root_layout != Some((root, available)) {
            self.mark_post_layout_subtree_dirty(root);
            self.last_root_layout = Some((root, available));
        }
        let taffy_start = Instant::now();
        compute_root_layout(self, root, available);
        self.layout_absolute_nodes(root);
        self.profile.taffy_ns = taffy_start.elapsed().as_nanos();
        let dirty_descendants_start = Instant::now();
        self.ensure_dirty_descendants_are_laid_out(root);
        self.profile.dirty_descendants_ns = dirty_descendants_start.elapsed().as_nanos();
        let visible_overflow_start = Instant::now();
        self.compute_visible_overflow_sizes(root);
        self.profile.visible_overflow_ns = visible_overflow_start.elapsed().as_nanos();
    }

    pub(crate) fn layout(&self, node: NodeId) -> Layout {
        self.snapped_layout(node)
    }

    pub(crate) fn kind(&self, node: NodeId) -> &LayoutNodeKind {
        &self.nodes[node_index(node)].kind
    }

    pub(crate) fn style(&self, node: NodeId) -> &DivStyle {
        &self.nodes[node_index(node)].style
    }

    pub(crate) fn children(&self, node: NodeId) -> &[NodeId] {
        &self.nodes[node_index(node)].children
    }

    pub(crate) fn visible_overflow_size(&self, node: NodeId) -> Size<f32> {
        self.nodes[node_index(node)].visible_overflow_size
    }

    pub(crate) fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node_index(node)].parent
    }

    pub(crate) fn scroll_offset(&self, node: NodeId) -> (u32, u32) {
        let item = &self.nodes[node_index(node)];
        if let LayoutNodeKind::TextArea(textarea) = &item.kind {
            let layout = self.layout(node);
            let content_size = layout.content_box_size();
            let client_width = float_to_cells(content_size.width);
            let client_height = float_to_cells(content_size.height);
            let scroll_height =
                textarea_content_height(&textarea.value, client_width.max(1) as usize);
            let max_top = scroll_height.saturating_sub(client_height);
            return (0, item.scroll_top.min(max_top));
        }
        (item.scroll_left, item.scroll_top)
    }

    pub(crate) fn layout_passes(&self) -> u64 {
        self.layout_passes
    }

    pub(crate) fn stats(&self) -> LayoutStats {
        LayoutStats {
            node_count: self.nodes.iter().filter(|node| node.occupied).count(),
            allocated_slot_count: self.nodes.len(),
            inline_context_count: self
                .nodes
                .iter()
                .enumerate()
                .filter(|(index, node)| {
                    node.occupied && self.is_inline_context(NodeId::from(*index))
                })
                .count(),
            inline_fragment_count: self
                .nodes
                .iter()
                .filter(|node| node.occupied)
                .map(|node| node.fragments.len())
                .sum(),
        }
    }

    pub(crate) fn profile_stats(&self) -> LayoutProfileStats {
        self.profile
    }

    fn push_node(&mut self, kind: LayoutNodeKind, style: DivStyle) -> NodeId {
        let is_absolute = style.position == CssPosition::Absolute;
        let node = LayoutNode {
            occupied: true,
            kind,
            taffy_style: style.to_taffy(),
            style,
            children: Vec::new(),
            parent: None,
            stacking_children: Vec::new(),
            has_relative_inline_descendants: false,
            has_inline_stacking_contexts: false,
            inline_static_position: None,
            layout: Layout::new(),
            cache: Cache::new(),
            layout_dirty: true,
            fragments_dirty: true,
            post_layout_dirty: true,
            visible_overflow_dirty: true,
            scroll_left: 0,
            scroll_top: 0,
            fragments: Vec::new(),
            measure_cache: InlineMeasureCache::default(),
            visible_overflow_size: Size::ZERO,
        };
        let id = if let Some(index) = self.free_nodes.pop() {
            self.nodes[index] = node;
            NodeId::from(index)
        } else {
            let id = NodeId::from(self.nodes.len());
            self.nodes.push(node);
            id
        };
        if is_absolute {
            self.absolute_nodes.insert(id);
        }
        if self.nodes[node_index(id)].style.position != CssPosition::Static
            || self.nodes[node_index(id)].style.z_index != CssZIndex::Auto
            || self.nodes[node_index(id)].style.opacity < 1.0
        {
            self.stacking_candidates.insert(id);
        }
        self.update_scroll_node(id);
        self.stacking_tree_dirty = true;
        id
    }

    pub(crate) fn prepare_paint(&mut self, root: NodeId) {
        if self.stacking_tree_dirty {
            self.rebuild_stacking_tree(root);
        }
    }

    pub(crate) fn creates_stacking_context(&self, node: NodeId) -> bool {
        let item = &self.nodes[node_index(node)];
        if item.style.opacity < 1.0 {
            return true;
        }
        if item.style.z_index == CssZIndex::Auto {
            return false;
        }
        if item.style.position != CssPosition::Static {
            return true;
        }

        item.parent.is_some_and(|parent| {
            matches!(
                self.nodes[node_index(parent)].style.display,
                LayoutDisplay::Flex | LayoutDisplay::Grid
            )
        })
    }

    pub(crate) fn is_stacking_item(&self, node: NodeId) -> bool {
        let item = &self.nodes[node_index(node)];
        item.style.opacity < 1.0
            || item.style.position != CssPosition::Static
            || (item.style.z_index != CssZIndex::Auto
                && item.parent.is_some_and(|parent| {
                    matches!(
                        self.nodes[node_index(parent)].style.display,
                        LayoutDisplay::Flex | LayoutDisplay::Grid
                    )
                }))
    }

    pub(crate) fn stacking_children(&self, node: NodeId) -> &[NodeId] {
        &self.nodes[node_index(node)].stacking_children
    }

    fn rebuild_stacking_tree(&mut self, root: NodeId) {
        if self.stacking_candidates.is_empty() && !self.stacking_tree_populated {
            self.stacking_tree_dirty = false;
            return;
        }
        for node in &mut self.nodes {
            if node.occupied {
                node.stacking_children.clear();
                node.has_inline_stacking_contexts = false;
            }
        }
        if self.stacking_candidates.is_empty() {
            self.stacking_tree_populated = false;
            self.stacking_tree_dirty = false;
            return;
        }
        self.rebuild_stacking_subtree(root, root);
        let z_indices = self
            .nodes
            .iter()
            .map(|node| node.style.z_index.value())
            .collect::<Vec<_>>();
        for node in &mut self.nodes {
            if node.stacking_children.len() > 1 {
                node.stacking_children
                    .sort_by_key(|child| z_indices[node_index(*child)]);
            }
        }
        self.stacking_tree_populated = true;
        self.stacking_tree_dirty = false;
    }

    fn rebuild_stacking_subtree(&mut self, node: NodeId, stacking_context: NodeId) {
        self.profile.stacking_tree_visits += 1;
        let child_count = self.nodes[node_index(node)].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[node_index(node)].children[child_index];
            if self.is_stacking_item(child) {
                if let Some(inline_context) = self.inline_formatting_context(child) {
                    self.nodes[node_index(inline_context)].has_inline_stacking_contexts = true;
                }
                self.nodes[node_index(stacking_context)]
                    .stacking_children
                    .push(child);
                let descendant_context = if self.creates_stacking_context(child) {
                    child
                } else {
                    stacking_context
                };
                self.rebuild_stacking_subtree(child, descendant_context);
            } else {
                self.rebuild_stacking_subtree(child, stacking_context);
            }
        }
    }

    fn layout_absolute_nodes(&mut self, root: NodeId) {
        let mut absolute_nodes = self
            .absolute_nodes
            .iter()
            .filter_map(|node| {
                let mut current = self.nodes[node_index(*node)].parent;
                let mut containing_block = None;
                let mut depth = 0usize;
                let mut belongs_to_root = false;
                while let Some(ancestor) = current {
                    depth += 1;
                    if containing_block.is_none()
                        && self.nodes[node_index(ancestor)].style.position != CssPosition::Static
                    {
                        containing_block = Some(ancestor);
                    }
                    if ancestor == root {
                        belongs_to_root = true;
                        break;
                    }
                    current = self.nodes[node_index(ancestor)].parent;
                }
                belongs_to_root.then_some((depth, *node, containing_block.unwrap_or(root)))
            })
            .collect::<Vec<_>>();
        absolute_nodes.sort_by_key(|(depth, _, _)| *depth);
        self.profile.absolute_layout_visits += absolute_nodes.len() as u64;

        for (_, child, containing_block) in absolute_nodes {
            let parent = self.nodes[node_index(child)]
                .parent
                .expect("absolute nodes attached below the root have a parent");
            let static_origin = if self.nodes[node_index(child)].layout_dirty {
                if let (Some(inline_context), Some(position)) = (
                    self.inline_formatting_context(child),
                    self.nodes[node_index(child)].inline_static_position,
                ) {
                    let context_layout = self.layout(inline_context);
                    let context_origin = self.absolute_layout_origin(inline_context);
                    Point {
                        x: context_origin.x
                            + context_layout.border.left
                            + context_layout.padding.left
                            + position.x as f32,
                        y: context_origin.y
                            + context_layout.border.top
                            + context_layout.padding.top
                            + position.y as f32,
                    }
                } else {
                    let parent_layout = self.layout(parent);
                    let parent_origin = self.absolute_layout_origin(parent);
                    Point {
                        x: parent_origin.x + parent_layout.border.left + parent_layout.padding.left,
                        y: parent_origin.y + parent_layout.border.top + parent_layout.padding.top,
                    }
                }
            } else {
                self.absolute_layout_origin(child)
            };
            let containing_origin = self.absolute_layout_origin(containing_block);
            let static_position = Point {
                x: static_origin.x - containing_origin.x,
                y: static_origin.y - containing_origin.y,
            };
            self.layout_absolute_child(containing_block, child, static_position);
        }
    }

    fn layout_absolute_child(
        &mut self,
        containing_block: NodeId,
        child: NodeId,
        static_position: Point<f32>,
    ) {
        let containing_layout = self.layout(containing_block);
        let containing_origin = self.absolute_layout_origin(containing_block);
        let parent_origin = if let Some(parent) = self.nodes[node_index(child)].parent {
            self.absolute_layout_origin(parent)
        } else {
            Point { x: 0.0, y: 0.0 }
        };
        let area_offset = Point {
            x: containing_layout.border.left,
            y: containing_layout.border.top,
        };
        let area_size = Size {
            width: (containing_layout.size.width
                - containing_layout.border.horizontal_axis_sum()
                - containing_layout.scrollbar_size.width)
                .max(0.0),
            height: (containing_layout.size.height
                - containing_layout.border.vertical_axis_sum()
                - containing_layout.scrollbar_size.height)
                .max(0.0),
        };
        let style = self.nodes[node_index(child)].taffy_style.clone();
        let aspect_ratio = style.aspect_ratio;
        let margin = style
            .margin
            .map(|margin| margin.resolve_to_option(area_size.width, |_, _| 0.0));
        let padding = style
            .padding
            .resolve_or_zero(Some(area_size.width), |_, _| 0.0);
        let border = style
            .border
            .resolve_or_zero(Some(area_size.width), |_, _| 0.0);
        let padding_border_sum = (padding + border).sum_axes();
        let box_sizing_adjustment = if style.box_sizing == taffy::BoxSizing::ContentBox {
            padding_border_sum
        } else {
            Size::ZERO
        };
        let left = style.inset.left.maybe_resolve(area_size.width, |_, _| 0.0);
        let right = style.inset.right.maybe_resolve(area_size.width, |_, _| 0.0);
        let top = style.inset.top.maybe_resolve(area_size.height, |_, _| 0.0);
        let bottom = style
            .inset
            .bottom
            .maybe_resolve(area_size.height, |_, _| 0.0);
        let style_size = style
            .size
            .maybe_resolve(area_size, |_, _| 0.0)
            .maybe_apply_aspect_ratio(aspect_ratio)
            .maybe_add(box_sizing_adjustment);
        let min_size = style
            .min_size
            .maybe_resolve(area_size, |_, _| 0.0)
            .maybe_apply_aspect_ratio(aspect_ratio)
            .maybe_add(box_sizing_adjustment)
            .or(padding_border_sum.map(Some))
            .maybe_max(padding_border_sum);
        let max_size = style
            .max_size
            .maybe_resolve(area_size, |_, _| 0.0)
            .maybe_apply_aspect_ratio(aspect_ratio)
            .maybe_add(box_sizing_adjustment);
        let mut known_dimensions = style_size.maybe_clamp(min_size, max_size);

        if let (None, Some(left), Some(right)) = (known_dimensions.width, left, right) {
            known_dimensions.width = Some(
                (area_size.width
                    - margin.left.unwrap_or(0.0)
                    - margin.right.unwrap_or(0.0)
                    - left
                    - right)
                    .max(0.0),
            );
            known_dimensions = known_dimensions
                .maybe_apply_aspect_ratio(aspect_ratio)
                .maybe_clamp(min_size, max_size);
        }
        if let (None, Some(top), Some(bottom)) = (known_dimensions.height, top, bottom) {
            known_dimensions.height = Some(
                (area_size.height
                    - margin.top.unwrap_or(0.0)
                    - margin.bottom.unwrap_or(0.0)
                    - top
                    - bottom)
                    .max(0.0),
            );
            known_dimensions = known_dimensions
                .maybe_apply_aspect_ratio(aspect_ratio)
                .maybe_clamp(min_size, max_size);
        }

        let available_space = Size {
            width: AvailableSpace::Definite(
                area_size.width.maybe_clamp(min_size.width, max_size.width),
            ),
            height: AvailableSpace::Definite(
                area_size
                    .height
                    .maybe_clamp(min_size.height, max_size.height),
            ),
        };
        let measured_size = self
            .compute_child_layout(
                child,
                LayoutInput {
                    known_dimensions,
                    parent_size: area_size.map(Some),
                    available_space,
                    sizing_mode: SizingMode::ContentSize,
                    axis: RequestedAxis::Both,
                    run_mode: RunMode::ComputeSize,
                    vertical_margins_are_collapsible: Line::FALSE,
                },
            )
            .size;
        let final_size = known_dimensions
            .unwrap_or(measured_size)
            .maybe_clamp(min_size, max_size);
        let layout_output = self.compute_child_layout(
            child,
            LayoutInput {
                known_dimensions: final_size.map(Some),
                parent_size: area_size.map(Some),
                available_space,
                sizing_mode: SizingMode::ContentSize,
                axis: RequestedAxis::Both,
                run_mode: RunMode::PerformLayout,
                vertical_margins_are_collapsible: Line::FALSE,
            },
        );
        let non_auto_margin = Rect {
            left: if left.is_some() {
                margin.left.unwrap_or(0.0)
            } else {
                0.0
            },
            right: if right.is_some() {
                margin.right.unwrap_or(0.0)
            } else {
                0.0
            },
            top: if top.is_some() {
                margin.top.unwrap_or(0.0)
            } else {
                0.0
            },
            bottom: if bottom.is_some() {
                margin.bottom.unwrap_or(0.0)
            } else {
                0.0
            },
        };
        let absolute_auto_margin_space = Point {
            x: right
                .map(|right| area_size.width - right - left.unwrap_or(0.0))
                .unwrap_or(final_size.width),
            y: bottom
                .map(|bottom| area_size.height - bottom - top.unwrap_or(0.0))
                .unwrap_or(final_size.height),
        };
        let free_space = Size {
            width: absolute_auto_margin_space.x
                - final_size.width
                - non_auto_margin.horizontal_axis_sum(),
            height: absolute_auto_margin_space.y
                - final_size.height
                - non_auto_margin.vertical_axis_sum(),
        };
        let horizontal_auto_margin_count =
            margin.left.is_none() as u8 + margin.right.is_none() as u8;
        let vertical_auto_margin_count = margin.top.is_none() as u8 + margin.bottom.is_none() as u8;
        let auto_margin = Size {
            width: if horizontal_auto_margin_count == 2
                && (style_size.width.is_none()
                    || style_size.width.unwrap_or(0.0) >= free_space.width)
            {
                0.0
            } else if horizontal_auto_margin_count > 0 {
                free_space.width / horizontal_auto_margin_count as f32
            } else {
                0.0
            },
            height: if vertical_auto_margin_count == 2
                && (style_size.height.is_none()
                    || style_size.height.unwrap_or(0.0) >= free_space.height)
            {
                0.0
            } else if vertical_auto_margin_count > 0 {
                free_space.height / vertical_auto_margin_count as f32
            } else {
                0.0
            },
        };
        let resolved_margin = Rect {
            left: margin.left.unwrap_or(auto_margin.width),
            right: margin.right.unwrap_or(auto_margin.width),
            top: margin.top.unwrap_or(auto_margin.height),
            bottom: margin.bottom.unwrap_or(auto_margin.height),
        };
        let x = match (left, right) {
            (Some(left), _) => left + resolved_margin.left,
            (None, Some(right)) => {
                area_size.width - final_size.width - right - resolved_margin.right
            }
            (None, None) => static_position.x + resolved_margin.left - area_offset.x,
        };
        let y = match (top, bottom) {
            (Some(top), _) => top + resolved_margin.top,
            (None, Some(bottom)) => {
                area_size.height - final_size.height - bottom - resolved_margin.bottom
            }
            (None, None) => static_position.y + resolved_margin.top - area_offset.y,
        };
        let scrollbar_size = Size {
            width: if style.overflow.y == taffy::Overflow::Scroll {
                style.scrollbar_width
            } else {
                0.0
            },
            height: if style.overflow.x == taffy::Overflow::Scroll {
                style.scrollbar_width
            } else {
                0.0
            },
        };
        self.set_unrounded_layout(
            child,
            &Layout {
                order: 0,
                size: final_size,
                content_size: layout_output.content_size,
                scrollbar_size,
                location: Point {
                    x: containing_origin.x + area_offset.x + x - parent_origin.x,
                    y: containing_origin.y + area_offset.y + y - parent_origin.y,
                },
                padding,
                border,
                margin: resolved_margin,
            },
        );
    }

    fn update_scroll_node(&mut self, node: NodeId) {
        let item = &self.nodes[node_index(node)];
        let is_scroll_node = matches!(item.kind, LayoutNodeKind::TextArea(_))
            || matches!(item.kind, LayoutNodeKind::Element)
                && (item.style.overflow_x != LayoutOverflow::Visible
                    || item.style.overflow_y != LayoutOverflow::Visible);
        if is_scroll_node {
            self.scroll_nodes.insert(node);
        } else {
            self.scroll_nodes.remove(&node);
        }
    }

    pub(crate) fn inline_fragments(&self, node: NodeId) -> &[InlineFragment] {
        &self.nodes[node_index(node)].fragments
    }

    pub(crate) fn inline_formatting_context(&self, node: NodeId) -> Option<NodeId> {
        let mut context = None;
        let mut current = self.nodes[node_index(node)].parent;
        while let Some(ancestor) = current {
            if self.is_inline_context(ancestor) {
                context = Some(ancestor);
            }
            current = self.nodes[node_index(ancestor)].parent;
        }
        context
    }

    pub(crate) fn inline_fragment_stacking_context(
        &self,
        inline_context: NodeId,
        fragment_node: NodeId,
    ) -> Option<NodeId> {
        if !self.nodes[node_index(inline_context)].has_inline_stacking_contexts {
            return None;
        }

        let mut current = Some(fragment_node);
        while let Some(node) = current {
            if node == inline_context {
                break;
            }
            if self.is_stacking_item(node) {
                return Some(node);
            }
            current = self.nodes[node_index(node)].parent;
        }
        None
    }

    pub(crate) fn inline_fragment_relative_offset(
        &self,
        inline_context: NodeId,
        fragment_node: NodeId,
    ) -> Point<i32> {
        if !self.nodes[node_index(inline_context)].has_relative_inline_descendants {
            return Point { x: 0, y: 0 };
        }

        let containing_size = self.layout(inline_context).content_box_size();
        let mut offset = Point { x: 0.0, y: 0.0 };
        let mut current = Some(fragment_node);
        while let Some(node) = current {
            if node == inline_context {
                break;
            }
            let style = &self.nodes[node_index(node)].style;
            if style.position == CssPosition::Relative {
                offset.x += relative_axis_offset(style.left, style.right, containing_size.width);
                offset.y += relative_axis_offset(style.top, style.bottom, containing_size.height);
            }
            current = self.nodes[node_index(node)].parent;
        }
        Point {
            x: offset.x.round() as i32,
            y: offset.y.round() as i32,
        }
    }

    fn snapped_layout(&self, node: NodeId) -> Layout {
        let raw = self.nodes[node_index(node)].layout;
        let parent_origin = self.raw_parent_origin(node);
        let absolute_origin = Point {
            x: parent_origin.x + raw.location.x,
            y: parent_origin.y + raw.location.y,
        };
        let absolute_end = Point {
            x: absolute_origin.x + raw.size.width.max(0.0),
            y: absolute_origin.y + raw.size.height.max(0.0),
        };

        let mut layout = raw;
        layout.location.x = absolute_origin.x.round() - parent_origin.x.round();
        layout.location.y = absolute_origin.y.round() - parent_origin.y.round();
        layout.size.width = absolute_end.x.round() - absolute_origin.x.round();
        layout.size.height = absolute_end.y.round() - absolute_origin.y.round();
        layout.scrollbar_size.width = raw.scrollbar_size.width.round();
        layout.scrollbar_size.height = raw.scrollbar_size.height.round();
        layout.border.left =
            (absolute_origin.x + raw.border.left).round() - absolute_origin.x.round();
        layout.border.right = absolute_end.x.round() - (absolute_end.x - raw.border.right).round();
        layout.border.top =
            (absolute_origin.y + raw.border.top).round() - absolute_origin.y.round();
        layout.border.bottom =
            absolute_end.y.round() - (absolute_end.y - raw.border.bottom).round();
        layout.padding.left =
            (absolute_origin.x + raw.padding.left).round() - absolute_origin.x.round();
        layout.padding.right =
            absolute_end.x.round() - (absolute_end.x - raw.padding.right).round();
        layout.padding.top =
            (absolute_origin.y + raw.padding.top).round() - absolute_origin.y.round();
        layout.padding.bottom =
            absolute_end.y.round() - (absolute_end.y - raw.padding.bottom).round();
        layout
    }

    fn raw_parent_origin(&self, node: NodeId) -> Point<f32> {
        let mut origin = Point { x: 0.0, y: 0.0 };
        let mut current = self.nodes[node_index(node)].parent;
        while let Some(parent) = current {
            let parent_node = &self.nodes[node_index(parent)];
            origin.x += parent_node.layout.location.x;
            origin.y += parent_node.layout.location.y;
            current = parent_node.parent;
        }
        origin
    }

    fn absolute_layout_origin(&self, node: NodeId) -> Point<f32> {
        let mut origin = Point { x: 0.0, y: 0.0 };
        let mut path = Vec::new();
        let mut current = Some(node);
        while let Some(node_id) = current {
            path.push(node_id);
            current = self.nodes[node_index(node_id)].parent;
        }

        for node_id in path.into_iter().rev() {
            let layout = self.layout(node_id);
            origin.x += layout.location.x;
            origin.y += layout.location.y;
        }
        origin
    }

    fn child_layout_offset(&self, parent: NodeId, child: NodeId) -> Point<f32> {
        debug_assert_eq!(self.nodes[node_index(child)].parent, Some(parent));
        self.layout(child).location
    }

    fn absolute_paint_layout_origin(&self, node: NodeId) -> Point<f32> {
        let mut origin = self.absolute_layout_origin(node);
        let mut current = self.nodes[node_index(node)].parent;
        while let Some(ancestor) = current {
            let item = &self.nodes[node_index(ancestor)];
            if item.style.overflow_x == LayoutOverflow::Scroll {
                origin.x -= item.scroll_left as f32;
            }
            if item.style.overflow_y == LayoutOverflow::Scroll {
                origin.y -= item.scroll_top as f32;
            }
            current = item.parent;
        }
        origin
    }

    pub(crate) fn absolute_paint_origin(&self, node: NodeId) -> Point<f32> {
        self.absolute_paint_layout_origin(node)
    }

    fn absolute_border_rect(&self, node: NodeId) -> AbsoluteRect {
        let layout = self.layout(node);
        let origin = self.absolute_paint_layout_origin(node);
        AbsoluteRect::from_edges(
            origin.x.round() as i32,
            origin.y.round() as i32,
            (origin.x + layout.size.width).round() as i32,
            (origin.y + layout.size.height).round() as i32,
        )
    }

    fn content_box_absolute_rect(&self, node: NodeId) -> AbsoluteContentRect {
        let layout = self.layout(node);
        let origin = self.absolute_layout_origin(node);
        let content_size = layout.content_box_size();
        AbsoluteContentRect {
            left: (origin.x + layout.border.left + layout.padding.left).round() as i32,
            top: (origin.y + layout.border.top + layout.padding.top).round() as i32,
            width: float_to_cells(content_size.width) as usize,
            height: float_to_cells(content_size.height) as usize,
        }
    }

    pub(crate) fn scroll_metrics(&mut self, node: NodeId) -> Option<ArenaScrollMetrics> {
        self.scroll_metrics_for_node(node)
    }

    pub(crate) fn scroll_metrics_snapshot(&self, node_id: NodeId) -> Option<ArenaScrollMetrics> {
        let index = node_index(node_id);
        if matches!(self.nodes[index].kind, LayoutNodeKind::TextArea(_)) {
            return Some(self.textarea_scroll_metrics_for_node(node_id));
        }
        if !matches!(self.nodes[index].kind, LayoutNodeKind::Element) {
            return None;
        }

        let layout = self.layout(node_id);
        let overflow_x = self.nodes[index].style.overflow_x;
        let overflow_y = self.nodes[index].style.overflow_y;
        let scroll_left = self.nodes[index].scroll_left;
        let scroll_top = self.nodes[index].scroll_top;
        let padding_size = scrollport_size(layout);
        let padding_origin = Point {
            x: layout.border.left,
            y: layout.border.top,
        };
        let client_width = float_to_cells(padding_size.width);
        let client_height = float_to_cells(padding_size.height);
        let mut scroll_width = client_width;
        let mut scroll_height = client_height;

        if self.is_inline_context(node_id) {
            for fragment in &self.nodes[index].fragments {
                scroll_width = scroll_width.max(
                    float_to_cells(layout.padding.left)
                        .saturating_add(fragment.x)
                        .saturating_add(fragment.width)
                        .saturating_add(float_to_cells(layout.padding.right)),
                );
                scroll_height = scroll_height.max(
                    float_to_cells(layout.padding.top)
                        .saturating_add(fragment.y)
                        .saturating_add(fragment.height)
                        .saturating_add(float_to_cells(layout.padding.bottom)),
                );
            }
        } else {
            let child_count = self.nodes[index].children.len();
            for child_index in 0..child_count {
                let child = self.nodes[index].children[child_index];
                let child_layout = self.layout(child);
                let child_offset = self.child_layout_offset(node_id, child);
                let child_style = &self.nodes[node_index(child)].style;
                let child_overflow = self.nodes[node_index(child)].visible_overflow_size;
                let child_width = if child_style.overflow_x == LayoutOverflow::Visible {
                    child_overflow.width
                } else {
                    child_layout.size.width
                };
                let child_height = if child_style.overflow_y == LayoutOverflow::Visible {
                    child_overflow.height
                } else {
                    child_layout.size.height
                };
                scroll_width = scroll_width.max(float_to_cells(
                    child_offset.x + child_width - padding_origin.x + layout.padding.right,
                ));
                scroll_height = scroll_height.max(float_to_cells(
                    child_offset.y + child_height - padding_origin.y + layout.padding.bottom,
                ));
            }
        }

        Some(ArenaScrollMetrics {
            scroll_left: scroll_left.min(axis_max_scroll(
                overflow_x,
                scroll_width.saturating_sub(client_width),
            )),
            scroll_top: scroll_top.min(axis_max_scroll(
                overflow_y,
                scroll_height.saturating_sub(client_height),
            )),
            scroll_width,
            scroll_height,
            client_width,
            client_height,
        })
    }

    pub(crate) fn set_scroll_offset(
        &mut self,
        node: NodeId,
        scroll_left: u32,
        scroll_top: u32,
    ) -> Option<ArenaScrollMetrics> {
        let metrics = self.scroll_metrics_for_node(node)?;
        let index = node_index(node);
        let max_left = if matches!(self.nodes[index].kind, LayoutNodeKind::TextArea(_)) {
            0
        } else {
            axis_max_scroll(
                self.nodes[index].style.overflow_x,
                metrics.scroll_width.saturating_sub(metrics.client_width),
            )
        };
        let max_top = if matches!(self.nodes[index].kind, LayoutNodeKind::TextArea(_)) {
            metrics.scroll_height.saturating_sub(metrics.client_height)
        } else {
            axis_max_scroll(
                self.nodes[index].style.overflow_y,
                metrics.scroll_height.saturating_sub(metrics.client_height),
            )
        };
        let item = &mut self.nodes[node_index(node)];
        item.scroll_left = scroll_left.min(max_left);
        item.scroll_top = scroll_top.min(max_top);
        if let LayoutNodeKind::TextArea(textarea) = &mut item.kind {
            textarea.scroll_cursor_dirty = false;
            self.dirty_textareas.remove(&node);
        }
        self.scroll_metrics_for_node(node)
    }

    pub(crate) fn clamp_scroll_offsets(&mut self) {
        let nodes = self.scroll_nodes.iter().copied().collect::<Vec<_>>();
        for node in nodes {
            self.profile.scroll_clamp_visits += 1;
            let Some(metrics) = self.scroll_metrics_for_node(node) else {
                continue;
            };
            let item = &mut self.nodes[node_index(node)];
            item.scroll_left = metrics.scroll_left;
            item.scroll_top = metrics.scroll_top;
        }
    }

    fn scroll_metrics_for_node(&mut self, node_id: NodeId) -> Option<ArenaScrollMetrics> {
        let index = node_index(node_id);
        if matches!(self.nodes[index].kind, LayoutNodeKind::TextArea(_)) {
            return Some(self.textarea_scroll_metrics_for_node(node_id));
        }
        if !matches!(self.nodes[index].kind, LayoutNodeKind::Element) {
            return None;
        }

        let layout = self.layout(node_id);
        let white_space = self.nodes[index].style.white_space;
        let overflow_x = self.nodes[index].style.overflow_x;
        let overflow_y = self.nodes[index].style.overflow_y;
        let scroll_left = self.nodes[index].scroll_left;
        let scroll_top = self.nodes[index].scroll_top;
        let padding_size = scrollport_size(layout);
        let padding_origin = Point {
            x: layout.border.left,
            y: layout.border.top,
        };
        let client_width = float_to_cells(padding_size.width);
        let client_height = float_to_cells(padding_size.height);
        let mut scroll_width = client_width;
        let mut scroll_height = client_height;

        if self.is_inline_context(node_id) {
            let widths = self.inline_content_widths(node_id, white_space);
            let content_width = float_to_cells(scroll_content_box_size(layout).width).max(1);
            let height = self.inline_content_height(node_id, white_space, content_width);
            scroll_width = scroll_width.max(float_to_cells(
                layout.padding.left + widths.max + layout.padding.right,
            ));
            scroll_height = scroll_height.max(float_to_cells(
                layout.padding.top + height + layout.padding.bottom,
            ));
        } else {
            let child_count = self.nodes[index].children.len();
            for child_index in 0..child_count {
                let child = self.nodes[index].children[child_index];
                let child_layout = self.layout(child);
                let child_offset = self.child_layout_offset(node_id, child);
                let child_style = &self.nodes[node_index(child)].style;
                let child_overflow = self.nodes[node_index(child)].visible_overflow_size;
                let child_width = if child_style.overflow_x == LayoutOverflow::Visible {
                    child_overflow.width
                } else {
                    child_layout.size.width
                };
                let child_height = if child_style.overflow_y == LayoutOverflow::Visible {
                    child_overflow.height
                } else {
                    child_layout.size.height
                };
                scroll_width = scroll_width.max(float_to_cells(
                    child_offset.x + child_width - padding_origin.x + layout.padding.right,
                ));
                scroll_height = scroll_height.max(float_to_cells(
                    child_offset.y + child_height - padding_origin.y + layout.padding.bottom,
                ));
            }
        }

        Some(ArenaScrollMetrics {
            scroll_left: scroll_left.min(axis_max_scroll(
                overflow_x,
                scroll_width.saturating_sub(client_width),
            )),
            scroll_top: scroll_top.min(axis_max_scroll(
                overflow_y,
                scroll_height.saturating_sub(client_height),
            )),
            scroll_width,
            scroll_height,
            client_width,
            client_height,
        })
    }

    fn textarea_scroll_metrics_for_node(&self, node_id: NodeId) -> ArenaScrollMetrics {
        let index = node_index(node_id);
        let layout = self.layout(node_id);
        let content_size = layout.content_box_size();
        let client_width = float_to_cells(content_size.width);
        let client_height = float_to_cells(content_size.height);
        let scroll_height = match &self.nodes[index].kind {
            LayoutNodeKind::TextArea(textarea) => {
                textarea_content_height(&textarea.value, client_width.max(1) as usize)
                    .max(client_height)
            }
            _ => client_height,
        };
        let max_top = scroll_height.saturating_sub(client_height);
        let scroll_top = self.nodes[index].scroll_top.min(max_top);

        ArenaScrollMetrics {
            scroll_left: 0,
            scroll_top,
            scroll_width: client_width,
            scroll_height,
            client_width,
            client_height,
        }
    }

    pub(crate) fn ensure_dirty_textareas_visible(&mut self) {
        let nodes = std::mem::take(&mut self.dirty_textareas);
        self.profile.dirty_textarea_visits += nodes.len() as u64;

        for node in nodes {
            self.ensure_textarea_cursor_visible(node);
        }
    }

    pub(crate) fn ensure_textarea_cursor_visible(&mut self, node: NodeId) -> Option<()> {
        let index = node_index(node);
        let layout = self.layout(node);
        let content_size = layout.content_box_size();
        let viewport_width = float_to_cells(content_size.width).max(1) as usize;
        let viewport_height = float_to_cells(content_size.height).max(1) as usize;
        let (value, cursor) = match &self.nodes[index].kind {
            LayoutNodeKind::TextArea(textarea) => (textarea.value.clone(), textarea.cursor),
            _ => return None,
        };
        let wrapped = WrappedText::new(&value, viewport_width);
        let (cursor_row, _) = wrapped.cursor_position(cursor as usize);
        let scroll_height = wrapped.row_count() as u32;
        let max_top = scroll_height.saturating_sub(viewport_height as u32) as usize;
        let current = self.nodes[index].scroll_top as usize;
        let next = textarea_scroll_top_for_cursor(current, cursor_row, viewport_height, max_top);

        let item = &mut self.nodes[index];
        item.scroll_top = next as u32;
        if let LayoutNodeKind::TextArea(textarea) = &mut item.kind {
            textarea.scroll_cursor_dirty = false;
        }
        Some(())
    }

    fn clear_cache_from(&mut self, node: NodeId) {
        let mut current = Some(node);
        while let Some(node_id) = current {
            let item = &mut self.nodes[node_index(node_id)];
            item.cache.clear();
            item.measure_cache.clear();
            item.layout_dirty = true;
            item.fragments_dirty = true;
            item.post_layout_dirty = true;
            item.visible_overflow_dirty = true;
            current = item.parent;
        }
    }

    fn clear_cache_subtree_and_ancestors(&mut self, node: NodeId) {
        self.clear_cache_subtree(node);
        self.clear_cache_from(node);
    }

    fn clear_cache_subtree(&mut self, node: NodeId) {
        let index = node_index(node);
        self.nodes[index].cache.clear();
        self.nodes[index].measure_cache.clear();
        self.nodes[index].layout_dirty = true;
        self.nodes[index].fragments_dirty = true;
        self.nodes[index].post_layout_dirty = true;
        self.nodes[index].visible_overflow_dirty = true;
        let child_count = self.nodes[index].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            self.clear_cache_subtree(child);
        }
    }

    fn mark_post_layout_subtree_dirty(&mut self, node: NodeId) {
        let index = node_index(node);
        self.nodes[index].post_layout_dirty = true;
        self.nodes[index].visible_overflow_dirty = true;
        let child_count = self.nodes[index].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            self.mark_post_layout_subtree_dirty(child);
        }
    }

    fn should_store_layout(&self) -> bool {
        !self
            .layout_mode_stack
            .iter()
            .any(|mode| *mode == RunMode::ComputeSize)
    }

    fn ensure_dirty_descendants_are_laid_out(&mut self, node: NodeId) {
        if !self.nodes[node_index(node)].post_layout_dirty {
            return;
        }
        self.profile.dirty_descendant_visits += 1;
        if self.inline_fragments_need_layout(node) {
            self.compute_inline_fragments_for_stored_layout(node);
        }

        if self.has_dirty_child_layout(node) {
            self.compute_subtree_layout(node);
        }

        let child_count = self.nodes[node_index(node)].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[node_index(node)].children[child_index];
            self.ensure_dirty_descendants_are_laid_out(child);
        }
        self.nodes[node_index(node)].post_layout_dirty = false;
    }

    fn compute_visible_overflow_sizes(&mut self, node: NodeId) -> Size<f32> {
        if !self.nodes[node_index(node)].visible_overflow_dirty {
            return self.nodes[node_index(node)].visible_overflow_size;
        }
        self.profile.visible_overflow_visits += 1;
        let child_count = self.nodes[node_index(node)].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[node_index(node)].children[child_index];
            self.compute_visible_overflow_sizes(child);
        }

        let layout = self.layout(node);
        let mut size = layout.size;
        if self.is_inline_context(node) {
            for fragment in &self.nodes[node_index(node)].fragments {
                size.width = size.width.max(
                    layout.border.left
                        + layout.padding.left
                        + fragment.x as f32
                        + fragment.width as f32
                        + layout.padding.right
                        + layout.border.right,
                );
                size.height = size.height.max(
                    layout.border.top
                        + layout.padding.top
                        + fragment.y as f32
                        + fragment.height as f32
                        + layout.padding.bottom
                        + layout.border.bottom,
                );
            }
        } else {
            for child_index in 0..child_count {
                let child = self.nodes[node_index(node)].children[child_index];
                let child_layout = self.layout(child);
                let child_offset = self.child_layout_offset(node, child);
                let child_style = &self.nodes[node_index(child)].style;
                let child_overflow = self.nodes[node_index(child)].visible_overflow_size;
                let propagated_width = if child_style.overflow_x == LayoutOverflow::Visible {
                    child_overflow.width
                } else {
                    child_layout.size.width
                };
                let propagated_height = if child_style.overflow_y == LayoutOverflow::Visible {
                    child_overflow.height
                } else {
                    child_layout.size.height
                };
                size.width = size.width.max(child_offset.x + propagated_width);
                size.height = size.height.max(child_offset.y + propagated_height);
            }
        }

        self.nodes[node_index(node)].visible_overflow_size = size;
        self.nodes[node_index(node)].visible_overflow_dirty = false;
        size
    }

    fn has_dirty_child_layout(&self, node: NodeId) -> bool {
        if self.is_inline_context(node) {
            return false;
        }

        self.nodes[node_index(node)]
            .children
            .iter()
            .any(|child| self.nodes[node_index(*child)].layout_dirty)
    }

    fn inline_fragments_need_layout(&self, node: NodeId) -> bool {
        self.is_inline_context(node) && self.nodes[node_index(node)].fragments_dirty
    }

    fn compute_inline_fragments_for_stored_layout(&mut self, node: NodeId) {
        let index = node_index(node);
        let width = scroll_content_box_size(self.nodes[index].layout)
            .width
            .max(1.0)
            .round() as u32;
        let white_space = self.nodes[index].style.white_space;
        self.compute_inline_fragments(node, white_space, width);
        self.mark_inline_descendants_clean(node);
    }

    fn compute_subtree_layout(&mut self, node: NodeId) {
        let saved_layout = self.nodes[node_index(node)].layout;
        compute_root_layout(
            self,
            node,
            Size {
                width: AvailableSpace::Definite(saved_layout.size.width),
                height: AvailableSpace::Definite(saved_layout.size.height),
            },
        );
        let item = &mut self.nodes[node_index(node)];
        item.layout = saved_layout;
        item.layout_dirty = false;
    }

    fn is_inline_context(&self, node: NodeId) -> bool {
        let node = &self.nodes[node_index(node)];
        matches!(node.kind, LayoutNodeKind::Element)
            && matches!(
                node.style.display,
                LayoutDisplay::Block | LayoutDisplay::Inline
            )
            && self.has_only_inline_children(&node.children)
    }

    fn has_only_inline_children(&self, children: &[NodeId]) -> bool {
        let mut has_inline = false;
        for child in children {
            let node = &self.nodes[node_index(*child)];
            if node.style.position == CssPosition::Absolute {
                continue;
            }
            match &node.kind {
                LayoutNodeKind::Text(_) => has_inline = true,
                LayoutNodeKind::Element if node.style.display == LayoutDisplay::Inline => {
                    has_inline = true;
                }
                LayoutNodeKind::Image(_)
                | LayoutNodeKind::Input(_)
                | LayoutNodeKind::TextArea(_)
                    if node.style.display == LayoutDisplay::Inline =>
                {
                    has_inline = true;
                }
                LayoutNodeKind::Element => return false,
                LayoutNodeKind::Image(_)
                | LayoutNodeKind::Input(_)
                | LayoutNodeKind::TextArea(_) => {
                    return false;
                }
            }
        }
        has_inline
    }

    fn compute_inline_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let LayoutInput {
            known_dimensions,
            parent_size,
            available_space,
            sizing_mode,
            run_mode,
            ..
        } = inputs;

        let (
            margin_style,
            padding_style,
            border_style,
            box_sizing,
            style_aspect_ratio,
            style_size,
            style_min_size,
            style_max_size,
            scrollbar_gutter,
            white_space,
        ) = {
            let node = &self.nodes[node_index(node_id)];
            let style = &node.taffy_style;
            (
                style.margin,
                style.padding,
                style.border,
                style.box_sizing,
                style.aspect_ratio,
                style.size,
                style.min_size,
                style.max_size,
                scrollbar_gutter_for_style(style),
                node.style.white_space,
            )
        };

        let margin = margin_style.resolve_or_zero(parent_size.width, |_, _| 0.0);
        let padding = padding_style.resolve_or_zero(parent_size.width, |_, _| 0.0);
        let border = border_style.resolve_or_zero(parent_size.width, |_, _| 0.0);
        let padding_border = padding + border;
        let padding_border_size = padding_border.sum_axes();
        let box_sizing_adjustment = if box_sizing == taffy::BoxSizing::ContentBox {
            padding_border_size
        } else {
            Size::ZERO
        };

        let (node_size, node_min_size, node_max_size, aspect_ratio) = match sizing_mode {
            SizingMode::ContentSize => (known_dimensions, Size::NONE, Size::NONE, None),
            SizingMode::InherentSize => {
                let aspect_ratio = style_aspect_ratio;
                let style_size = style_size
                    .maybe_resolve(parent_size, |_, _| 0.0)
                    .maybe_apply_aspect_ratio(aspect_ratio)
                    .maybe_add(box_sizing_adjustment);
                let style_min_size = style_min_size
                    .maybe_resolve(parent_size, |_, _| 0.0)
                    .maybe_apply_aspect_ratio(aspect_ratio)
                    .maybe_add(box_sizing_adjustment);
                let style_max_size = style_max_size
                    .maybe_resolve(parent_size, |_, _| 0.0)
                    .maybe_add(box_sizing_adjustment);

                let node_size = known_dimensions.or(style_size);
                (node_size, style_min_size, style_max_size, aspect_ratio)
            }
        };

        let styled_known_dimensions = known_dimensions
            .or(node_size)
            .maybe_clamp(node_min_size, node_max_size)
            .maybe_max(padding_border_size);

        if run_mode == RunMode::ComputeSize {
            if let Size {
                width: Some(width),
                height: Some(height),
            } = styled_known_dimensions
            {
                return LayoutOutput::from_outer_size(Size { width, height });
            }
        }

        let content_box_inset = padding_border + scrollbar_gutter;
        let content_available = Size {
            width: known_dimensions
                .width
                .map(AvailableSpace::from)
                .unwrap_or(available_space.width)
                .maybe_sub(margin.horizontal_axis_sum())
                .maybe_set(known_dimensions.width)
                .maybe_set(node_size.width)
                .map_definite_value(|size| {
                    size.maybe_clamp(node_min_size.width, node_max_size.width)
                        - content_box_inset.horizontal_axis_sum()
                }),
            height: known_dimensions
                .height
                .map(AvailableSpace::from)
                .unwrap_or(available_space.height)
                .maybe_sub(margin.vertical_axis_sum())
                .maybe_set(known_dimensions.height)
                .maybe_set(node_size.height)
                .map_definite_value(|size| {
                    size.maybe_clamp(node_min_size.height, node_max_size.height)
                        - content_box_inset.vertical_axis_sum()
                }),
        };

        let inline_width_start = Instant::now();
        let content_widths = self.inline_content_widths(node_id, white_space);
        self.profile.inline_width_calls += 1;
        self.profile.inline_width_ns += inline_width_start.elapsed().as_nanos();
        let content_width = known_dimensions
            .width
            .map(|width| (width - content_box_inset.horizontal_axis_sum()).max(0.0))
            .unwrap_or_else(|| {
                let computed = match content_available.width {
                    AvailableSpace::MinContent => content_widths.min,
                    AvailableSpace::MaxContent => content_widths.max,
                    AvailableSpace::Definite(limit) => limit
                        .max(0.0)
                        .min(content_widths.max)
                        .max(content_widths.min),
                };

                node_size
                    .width
                    .map(|width| (width - content_box_inset.horizontal_axis_sum()).max(0.0))
                    .unwrap_or(computed)
            });

        let inline_height_start = Instant::now();
        let content_height =
            self.inline_content_height(node_id, white_space, content_width.max(1.0).round() as u32);
        self.profile.inline_height_calls += 1;
        self.profile.inline_height_ns += inline_height_start.elapsed().as_nanos();
        if run_mode == RunMode::PerformLayout && self.should_store_layout() {
            let inline_fragment_start = Instant::now();
            self.compute_inline_fragments(
                node_id,
                white_space,
                content_width.max(1.0).round() as u32,
            );
            self.mark_inline_descendants_clean(node_id);
            self.profile.inline_fragment_calls += 1;
            self.profile.inline_fragment_ns += inline_fragment_start.elapsed().as_nanos();
        }

        let measured_size = Size {
            width: content_width,
            height: content_height,
        };
        let outer_size = known_dimensions
            .or(node_size)
            .unwrap_or(measured_size + content_box_inset.sum_axes())
            .maybe_clamp(node_min_size, node_max_size);
        let outer_size = Size {
            width: outer_size.width,
            height: outer_size.height.max(
                aspect_ratio
                    .map(|ratio| outer_size.width / ratio)
                    .unwrap_or(0.0),
            ),
        }
        .maybe_max(padding_border_size.map(Some));

        LayoutOutput::from_sizes(outer_size, measured_size)
    }

    fn inline_content_widths(
        &mut self,
        node_id: NodeId,
        white_space: CssWhiteSpace,
    ) -> ContentWidths {
        let index = node_index(node_id);
        if let Some(widths) = self.nodes[index].measure_cache.width(white_space) {
            return widths;
        }

        let child_count = self.nodes[index].children.len();
        let mut widths = ContentWidths { min: 1.0, max: 0.0 };
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            let item = self.inline_node_widths(child, white_space);
            widths = ContentWidths {
                min: widths.min.max(item.min),
                max: widths.max + item.max,
            };
        }
        self.nodes[index]
            .measure_cache
            .insert_width(InlineWidthCacheEntry {
                white_space,
                widths,
            });
        widths
    }

    fn inline_node_widths(
        &mut self,
        node_id: NodeId,
        inherited_white_space: CssWhiteSpace,
    ) -> ContentWidths {
        let index = node_index(node_id);
        if self.nodes[index].style.position == CssPosition::Absolute {
            return ContentWidths { min: 0.0, max: 0.0 };
        }
        match &self.nodes[index].kind {
            LayoutNodeKind::Text(text) => text_content_widths(text, inherited_white_space),
            LayoutNodeKind::Element if self.nodes[index].style.display == LayoutDisplay::Inline => {
                let white_space = effective_white_space(
                    inherited_white_space,
                    self.nodes[index].style.white_space,
                );
                self.inline_content_widths(node_id, white_space)
            }
            LayoutNodeKind::Image(_) | LayoutNodeKind::Input(_) | LayoutNodeKind::TextArea(_)
                if self.nodes[index].style.display == LayoutDisplay::Inline =>
            {
                let size = self.inline_replaced_node_size(node_id);
                ContentWidths {
                    min: size.width.max(1.0),
                    max: size.width.max(1.0),
                }
            }
            LayoutNodeKind::Element
            | LayoutNodeKind::Image(_)
            | LayoutNodeKind::Input(_)
            | LayoutNodeKind::TextArea(_) => ContentWidths { min: 1.0, max: 1.0 },
        }
    }

    fn inline_content_height(
        &mut self,
        node: NodeId,
        white_space: CssWhiteSpace,
        width: u32,
    ) -> f32 {
        let width = width.max(1);
        let index = node_index(node);
        if let Some(height) = self.nodes[index].measure_cache.height(white_space, width) {
            return height;
        }

        let mut cursor = InlineMeasureCursor {
            col: 0,
            row: 0,
            width,
            max_col: 0,
        };
        let child_count = self.nodes[index].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            self.measure_inline_node(child, white_space, &mut cursor);
        }
        let height = (cursor.row + 1).max(1) as f32;
        self.nodes[index]
            .measure_cache
            .insert_height(InlineHeightCacheEntry {
                white_space,
                width,
                height,
            });
        height
    }

    fn measure_inline_node(
        &mut self,
        node: NodeId,
        inherited_white_space: CssWhiteSpace,
        cursor: &mut InlineMeasureCursor,
    ) {
        let index = node_index(node);
        if self.nodes[index].style.position == CssPosition::Absolute {
            return;
        }
        match &self.nodes[index].kind {
            LayoutNodeKind::Text(text) => measure_inline_text(text, inherited_white_space, cursor),
            LayoutNodeKind::Element if self.nodes[index].style.display == LayoutDisplay::Inline => {
                let white_space = effective_white_space(
                    inherited_white_space,
                    self.nodes[index].style.white_space,
                );
                let child_count = self.nodes[index].children.len();
                for child_index in 0..child_count {
                    let child = self.nodes[index].children[child_index];
                    self.measure_inline_node(child, white_space, cursor);
                }
            }
            LayoutNodeKind::Image(_) | LayoutNodeKind::Input(_) | LayoutNodeKind::TextArea(_)
                if self.nodes[index].style.display == LayoutDisplay::Inline =>
            {
                let size = self.inline_replaced_node_size(node);
                measure_inline_replaced(size, cursor);
            }
            LayoutNodeKind::Element
            | LayoutNodeKind::Image(_)
            | LayoutNodeKind::Input(_)
            | LayoutNodeKind::TextArea(_) => {}
        }
    }

    fn compute_inline_fragments(&mut self, node: NodeId, white_space: CssWhiteSpace, width: u32) {
        let index = node_index(node);
        let fragment_capacity = self.nodes[index]
            .measure_cache
            .width(white_space)
            .map(|widths| widths.max.ceil() as usize)
            .unwrap_or(0);
        let mut cursor = InlineLayoutCursor {
            col: 0,
            row: 0,
            width: width.max(1),
            max_col: 0,
            selection_order: 0,
            has_relative_descendants: false,
            absolute_positions: Vec::new(),
            fragments: Vec::with_capacity(fragment_capacity),
        };
        let child_count = self.nodes[index].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            self.layout_inline_node(child, white_space, None, &mut cursor);
        }
        for (absolute, position) in cursor.absolute_positions {
            self.nodes[node_index(absolute)].inline_static_position = Some(position);
        }
        self.nodes[index].has_relative_inline_descendants = cursor.has_relative_descendants;
        self.nodes[index].fragments = cursor.fragments;
        self.nodes[index].fragments_dirty = false;
    }

    fn mark_inline_descendants_clean(&mut self, node: NodeId) {
        let child_count = self.nodes[node_index(node)].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[node_index(node)].children[child_index];
            if self.nodes[node_index(child)].style.position == CssPosition::Absolute {
                continue;
            }
            self.nodes[node_index(child)].layout_dirty = false;
            self.nodes[node_index(child)].fragments_dirty = false;
            if matches!(self.nodes[node_index(child)].kind, LayoutNodeKind::Element) {
                self.mark_inline_descendants_clean(child);
            }
        }
    }

    fn layout_inline_node(
        &self,
        node: NodeId,
        inherited_white_space: CssWhiteSpace,
        hit_target: Option<NodeId>,
        cursor: &mut InlineLayoutCursor,
    ) {
        let item = &self.nodes[node_index(node)];
        if item.style.position == CssPosition::Absolute {
            cursor.absolute_positions.push((
                node,
                Point {
                    x: cursor.col,
                    y: cursor.row,
                },
            ));
            return;
        }
        if item.style.position == CssPosition::Relative {
            cursor.has_relative_descendants = true;
        }
        match &item.kind {
            LayoutNodeKind::Text(text) => {
                layout_inline_text(node, hit_target, text, inherited_white_space, cursor)
            }
            LayoutNodeKind::Element if item.style.display == LayoutDisplay::Inline => {
                let white_space =
                    effective_white_space(inherited_white_space, item.style.white_space);
                for child in &item.children {
                    self.layout_inline_node(*child, white_space, Some(node), cursor);
                }
            }
            LayoutNodeKind::Image(_) | LayoutNodeKind::Input(_) | LayoutNodeKind::TextArea(_)
                if item.style.display == LayoutDisplay::Inline =>
            {
                let size = self.inline_replaced_node_size(node);
                layout_inline_replaced(node, hit_target, size, cursor);
            }
            LayoutNodeKind::Element
            | LayoutNodeKind::Image(_)
            | LayoutNodeKind::Input(_)
            | LayoutNodeKind::TextArea(_) => {}
        }
    }

    fn measure_leaf_node(
        &self,
        node_id: NodeId,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> Size<f32> {
        let node = &self.nodes[node_index(node_id)];
        match &node.kind {
            LayoutNodeKind::Element => Size::ZERO,
            LayoutNodeKind::Text(text) => text_leaf_size(text, known_dimensions, available_space),
            LayoutNodeKind::Image(_) | LayoutNodeKind::Input(_) | LayoutNodeKind::TextArea(_) => {
                self.replaced_node_size(node_id, known_dimensions, available_space)
            }
        }
    }

    fn replaced_node_size(
        &self,
        node_id: NodeId,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> Size<f32> {
        let node = &self.nodes[node_index(node_id)];
        let natural = match &node.kind {
            LayoutNodeKind::Image(image) => image_natural_size(image),
            LayoutNodeKind::Input(input) => input_natural_size(input),
            LayoutNodeKind::TextArea(textarea) => {
                let wrap_width = known_dimensions
                    .width
                    .or_else(|| available_space.width.into_option())
                    .map(float_to_cells)
                    // Zero cells is an intrinsic sizing query, not a one-cell wrap width.
                    .filter(|width| *width > 0);
                textarea_natural_size(textarea, wrap_width)
            }
            LayoutNodeKind::Element | LayoutNodeKind::Text(_) => Size::ZERO,
        };

        Size {
            width: known_dimensions
                .width
                .unwrap_or_else(|| styled_or_natural_width(node.style.width, natural.width)),
            height: known_dimensions
                .height
                .unwrap_or_else(|| styled_or_natural_height(node.style.height, natural.height)),
        }
    }

    fn inline_replaced_node_size(&self, node_id: NodeId) -> Size<f32> {
        let node = &self.nodes[node_index(node_id)];
        let natural = match &node.kind {
            LayoutNodeKind::Image(image) => image_natural_size(image),
            LayoutNodeKind::Input(input) => input_natural_size(input),
            LayoutNodeKind::TextArea(textarea) => {
                textarea_natural_size(textarea, explicit_content_width_cells(&node.style))
            }
            LayoutNodeKind::Element | LayoutNodeKind::Text(_) => Size::ZERO,
        };
        let border = border_size_cells(&node.style);

        Size {
            width: styled_or_natural_width(node.style.width, natural.width + border.width),
            height: styled_or_natural_height(node.style.height, natural.height + border.height),
        }
    }
}

impl TraversePartialTree for LayoutArena {
    type ChildIter<'a>
        = std::iter::Copied<std::slice::Iter<'a, NodeId>>
    where
        Self: 'a;

    fn child_ids(&self, parent_node_id: NodeId) -> Self::ChildIter<'_> {
        if self.is_inline_context(parent_node_id) {
            return self.nodes[node_index(parent_node_id)].children[0..0]
                .iter()
                .copied();
        }
        self.nodes[node_index(parent_node_id)]
            .children
            .iter()
            .copied()
    }

    fn child_count(&self, parent_node_id: NodeId) -> usize {
        if self.is_inline_context(parent_node_id) {
            return 0;
        }
        self.nodes[node_index(parent_node_id)].children.len()
    }

    fn get_child_id(&self, parent_node_id: NodeId, child_index: usize) -> NodeId {
        self.nodes[node_index(parent_node_id)].children[child_index]
    }
}

impl TraverseTree for LayoutArena {}

impl LayoutPartialTree for LayoutArena {
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        &self.nodes[node_index(node_id)].taffy_style
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        if self.should_store_layout() {
            let item = &mut self.nodes[node_index(node_id)];
            item.layout = *layout;
            item.layout_dirty = false;
        }
    }

    fn compute_child_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        self.layout_mode_stack.push(inputs.run_mode);
        let output = if inputs.run_mode == RunMode::PerformHiddenLayout {
            compute_hidden_layout(self, node_id)
        } else {
            compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
                if tree.is_inline_context(node_id) {
                    return tree.compute_inline_layout(node_id, inputs);
                }

                let display = tree.nodes[node_index(node_id)].taffy_style.display;
                let has_children = tree.child_count(node_id) > 0;
                match (display, has_children) {
                    (taffy::Display::None, _) => compute_hidden_layout(tree, node_id),
                    (taffy::Display::Block, true) => {
                        compute_block_layout(tree, node_id, inputs, None)
                    }
                    (taffy::Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
                    (taffy::Display::Grid, true) => compute_grid_layout(tree, node_id, inputs),
                    (_, false) => {
                        let style = tree.nodes[node_index(node_id)].taffy_style.clone();
                        let measure = |known_dimensions, available_space| {
                            tree.measure_leaf_node(node_id, known_dimensions, available_space)
                        };
                        compute_leaf_layout(inputs, &style, |_, _| 0.0, measure)
                    }
                }
            })
        };
        self.layout_mode_stack.pop();
        output
    }
}

impl CacheTree for LayoutArena {
    fn cache_get(&self, node_id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.nodes[node_index(node_id)].cache.get(input)
    }

    fn cache_store(&mut self, node_id: NodeId, input: &LayoutInput, layout_output: LayoutOutput) {
        self.nodes[node_index(node_id)]
            .cache
            .store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        self.nodes[node_index(node_id)].cache.clear();
    }
}

impl RoundTree for LayoutArena {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        self.nodes[node_index(node_id)].layout
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.nodes[node_index(node_id)].layout = *layout;
    }
}

impl taffy::LayoutBlockContainer for LayoutArena {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }

    fn compute_block_child_layout(
        &mut self,
        node_id: NodeId,
        inputs: LayoutInput,
        block_ctx: Option<&mut taffy::BlockContext<'_>>,
    ) -> LayoutOutput {
        let _ = block_ctx;
        self.compute_child_layout(node_id, inputs)
    }
}

impl taffy::LayoutFlexboxContainer for LayoutArena {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

impl taffy::LayoutGridContainer for LayoutArena {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.get_core_container_style(node_id)
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.get_core_container_style(child_node_id)
    }
}

struct InlineMeasureCursor {
    col: u32,
    row: u32,
    width: u32,
    max_col: u32,
}

struct InlineLayoutCursor {
    col: u32,
    row: u32,
    width: u32,
    max_col: u32,
    selection_order: usize,
    has_relative_descendants: bool,
    absolute_positions: Vec<(NodeId, Point<u32>)>,
    fragments: Vec<InlineFragment>,
}

fn relative_axis_offset(
    start: CssLengthPercentageAuto,
    end: CssLengthPercentageAuto,
    basis: f32,
) -> f32 {
    match resolve_length_percentage_auto(start, basis) {
        Some(value) => value,
        None => -resolve_length_percentage_auto(end, basis).unwrap_or(0.0),
    }
}

fn resolve_length_percentage_auto(value: CssLengthPercentageAuto, basis: f32) -> Option<f32> {
    match value {
        CssLengthPercentageAuto::Auto => None,
        CssLengthPercentageAuto::Length(value) => Some(value),
        CssLengthPercentageAuto::Percent(value) => Some(value * basis),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AbsoluteContentRect {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AbsoluteRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl AbsoluteRect {
    fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left: left.min(right),
            top: top.min(bottom),
            right: right.max(left),
            bottom: bottom.max(top),
        }
    }

    pub(crate) fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    fn width(self) -> u32 {
        (self.right - self.left).max(0) as u32
    }

    fn height(self) -> u32 {
        (self.bottom - self.top).max(0) as u32
    }
}

fn node_index(node: NodeId) -> usize {
    node.into()
}

fn textarea_scroll_top(cursor_row: usize, viewport_height: usize) -> usize {
    if viewport_height == 0 {
        return 0;
    }

    cursor_row.saturating_add(1).saturating_sub(viewport_height)
}

fn textarea_scroll_top_for_cursor(
    current: usize,
    cursor_row: usize,
    viewport_height: usize,
    max_top: usize,
) -> usize {
    if viewport_height == 0 {
        return 0;
    }

    if cursor_row < current {
        return cursor_row.min(max_top);
    }

    if cursor_row >= current.saturating_add(viewport_height) {
        return textarea_scroll_top(cursor_row, viewport_height).min(max_top);
    }

    current.min(max_top)
}

fn textarea_content_height(value: &str, wrap_width: usize) -> u32 {
    WrappedText::new(value, wrap_width.max(1)).row_count() as u32
}

fn image_natural_size(image: &ImageLayoutData) -> Size<f32> {
    let cell_width = image.cell_width_px.max(1) as f32;
    let cell_height = image.cell_height_px.max(1) as f32;
    Size {
        width: (image.width_px as f32 / cell_width).max(1.0),
        height: (image.height_px as f32 / cell_height).max(1.0),
    }
}

fn input_natural_size(input: &InputLayoutData) -> Size<f32> {
    Size {
        width: parse_text_for_single_line(&input.value).len().max(1) as f32,
        height: 1.0,
    }
}

fn textarea_natural_size(textarea: &TextAreaLayoutData, wrap_width: Option<u32>) -> Size<f32> {
    if let Some(wrap_width) = wrap_width {
        let wrapped = WrappedText::new(&textarea.value, wrap_width.max(1) as usize);
        return Size {
            width: wrapped.max_line_width().max(1) as f32,
            height: wrapped.row_count().max(1) as f32,
        };
    }

    let mut cursor = InlineMeasureCursor {
        col: 0,
        row: 0,
        width: u32::MAX,
        max_col: 0,
    };
    measure_inline_text(&textarea.value, CssWhiteSpace::PreWrap, &mut cursor);
    let width = cursor.max_col.max(cursor.col).max(1);
    // Taffy may reuse this result when its returned width later becomes known.
    let wrapped = WrappedText::new(&textarea.value, width as usize);
    Size {
        width: width as f32,
        height: wrapped.row_count().max(1) as f32,
    }
}

fn text_leaf_size(
    text: &str,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    let wrap_width = known_dimensions
        .width
        .or_else(|| available_space.width.into_option())
        .map(float_to_cells);
    let mut cursor = InlineMeasureCursor {
        col: 0,
        row: 0,
        width: wrap_width.unwrap_or(u32::MAX).max(1),
        max_col: 0,
    };
    measure_inline_text(text, CssWhiteSpace::Normal, &mut cursor);
    Size {
        width: known_dimensions
            .width
            .unwrap_or_else(|| cursor.max_col.max(cursor.col).max(1) as f32),
        height: known_dimensions
            .height
            .unwrap_or_else(|| (cursor.row + 1).max(1) as f32),
    }
}

fn styled_or_natural_width(width: CssDimension, natural: f32) -> f32 {
    match width {
        CssDimension::Length(value) => value.max(0.0),
        CssDimension::Auto | CssDimension::Percent(_) => natural.max(1.0),
    }
}

fn styled_or_natural_height(height: CssDimension, natural: f32) -> f32 {
    match height {
        CssDimension::Length(value) => value.max(0.0),
        CssDimension::Auto | CssDimension::Percent(_) => natural.max(1.0),
    }
}

fn explicit_width_cells(width: CssDimension) -> Option<u32> {
    match width {
        CssDimension::Length(value) => Some(float_to_cells(value).max(1)),
        CssDimension::Auto | CssDimension::Percent(_) => None,
    }
}

fn explicit_content_width_cells(style: &DivStyle) -> Option<u32> {
    explicit_width_cells(style.width).map(|width| {
        width
            .saturating_sub(border_size_cells(style).width as u32)
            .max(1)
    })
}

fn border_size_cells(style: &DivStyle) -> Size<f32> {
    Size {
        width: border_edge_cells(style.border_left) + border_edge_cells(style.border_right),
        height: border_edge_cells(style.border_top) + border_edge_cells(style.border_bottom),
    }
}

fn border_edge_cells(style: BorderStyle) -> f32 {
    if style == BorderStyle::None {
        0.0
    } else {
        1.0
    }
}

fn float_to_cells(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

fn padding_box_size(layout: Layout) -> Size<f32> {
    Size {
        width: (layout.size.width - layout.border.horizontal_axis_sum()).max(0.0),
        height: (layout.size.height - layout.border.vertical_axis_sum()).max(0.0),
    }
}

fn scrollport_size(layout: Layout) -> Size<f32> {
    let padding = padding_box_size(layout);
    Size {
        width: (padding.width - layout.scrollbar_size.width).max(0.0),
        height: (padding.height - layout.scrollbar_size.height).max(0.0),
    }
}

fn scroll_content_box_size(layout: Layout) -> Size<f32> {
    let content = layout.content_box_size();
    Size {
        width: (content.width - layout.scrollbar_size.width).max(0.0),
        height: (content.height - layout.scrollbar_size.height).max(0.0),
    }
}

fn absolute_padding_box_rect(bounds: AbsoluteRect, layout: Layout) -> AbsoluteRect {
    AbsoluteRect::from_edges(
        bounds.left + layout.border.left.round() as i32,
        bounds.top + layout.border.top.round() as i32,
        bounds.right - layout.border.right.round() as i32,
        bounds.bottom - layout.border.bottom.round() as i32,
    )
}

fn absolute_scrollport_rect(bounds: AbsoluteRect, layout: Layout) -> AbsoluteRect {
    let padding = absolute_padding_box_rect(bounds, layout);
    AbsoluteRect::from_edges(
        padding.left,
        padding.top,
        (padding.right - layout.scrollbar_size.width.round() as i32).max(padding.left),
        (padding.bottom - layout.scrollbar_size.height.round() as i32).max(padding.top),
    )
}

fn vertical_scrollbar_rect(bounds: AbsoluteRect, layout: Layout) -> Option<AbsoluteRect> {
    let width = layout.scrollbar_size.width.round() as i32;
    if width <= 0 {
        return None;
    }

    let padding = absolute_padding_box_rect(bounds, layout);
    let left = (padding.right - width).max(padding.left);
    let bottom = (padding.bottom - layout.scrollbar_size.height.round() as i32).max(padding.top);
    (left < padding.right && padding.top < bottom).then_some(AbsoluteRect::from_edges(
        left,
        padding.top,
        padding.right,
        bottom,
    ))
}

fn horizontal_scrollbar_rect(bounds: AbsoluteRect, layout: Layout) -> Option<AbsoluteRect> {
    let height = layout.scrollbar_size.height.round() as i32;
    if height <= 0 {
        return None;
    }

    let padding = absolute_padding_box_rect(bounds, layout);
    let top = (padding.bottom - height).max(padding.top);
    let right = (padding.right - layout.scrollbar_size.width.round() as i32).max(padding.left);
    (padding.left < right && top < padding.bottom).then_some(AbsoluteRect::from_edges(
        padding.left,
        top,
        right,
        padding.bottom,
    ))
}

fn vertical_scrollbar_hit(
    node: NodeId,
    rail: AbsoluteRect,
    metrics: &ArenaScrollMetrics,
) -> ArenaScrollbarHit {
    let rail_length = rail.height();
    let thumb_length =
        scrollbar_thumb_length(rail_length, metrics.client_height, metrics.scroll_height);
    let max_scroll = metrics.scroll_height.saturating_sub(metrics.client_height);
    let thumb_offset =
        scrollbar_thumb_offset(rail_length, thumb_length, metrics.scroll_top, max_scroll);
    ArenaScrollbarHit {
        node,
        axis: ScrollbarAxis::Vertical,
        rail_start: rail.top.max(0) as u32,
        rail_length,
        thumb_start: (rail.top + thumb_offset as i32).max(0) as u32,
        thumb_length,
        scroll_offset: metrics.scroll_top,
        max_scroll,
        client_length: metrics.client_height,
        scroll_length: metrics.scroll_height,
    }
}

fn horizontal_scrollbar_hit(
    node: NodeId,
    rail: AbsoluteRect,
    metrics: &ArenaScrollMetrics,
) -> ArenaScrollbarHit {
    let rail_length = rail.width();
    let thumb_length =
        scrollbar_thumb_length(rail_length, metrics.client_width, metrics.scroll_width);
    let max_scroll = metrics.scroll_width.saturating_sub(metrics.client_width);
    let thumb_offset =
        scrollbar_thumb_offset(rail_length, thumb_length, metrics.scroll_left, max_scroll);
    ArenaScrollbarHit {
        node,
        axis: ScrollbarAxis::Horizontal,
        rail_start: rail.left.max(0) as u32,
        rail_length,
        thumb_start: (rail.left + thumb_offset as i32).max(0) as u32,
        thumb_length,
        scroll_offset: metrics.scroll_left,
        max_scroll,
        client_length: metrics.client_width,
        scroll_length: metrics.scroll_width,
    }
}

fn scrollbar_thumb_length(rail_length: u32, client_length: u32, scroll_length: u32) -> u32 {
    if scroll_length <= client_length {
        return rail_length.max(1);
    }

    (((client_length as f32 / scroll_length.max(1) as f32) * rail_length as f32).floor() as u32)
        .clamp(1, rail_length.max(1))
}

fn scrollbar_thumb_offset(
    rail_length: u32,
    thumb_length: u32,
    scroll_offset: u32,
    max_scroll: u32,
) -> u32 {
    let max_thumb_offset = rail_length.saturating_sub(thumb_length);
    if max_scroll == 0 || max_thumb_offset == 0 {
        return 0;
    }

    (((scroll_offset as f32 / max_scroll as f32) * max_thumb_offset as f32).round() as u32)
        .min(max_thumb_offset)
}

fn scrollbar_gutter_for_style(style: &Style) -> Rect<f32> {
    let offsets = style.overflow().transpose().map(|overflow| match overflow {
        taffy::style::Overflow::Scroll => style.scrollbar_width(),
        _ => 0.0,
    });

    match style.direction() {
        taffy::style::Direction::Ltr => Rect {
            top: 0.0,
            left: 0.0,
            right: offsets.x,
            bottom: offsets.y,
        },
        taffy::style::Direction::Rtl => Rect {
            top: 0.0,
            left: offsets.x,
            right: 0.0,
            bottom: offsets.y,
        },
    }
}

fn measure_inline_replaced(size: Size<f32>, cursor: &mut InlineMeasureCursor) {
    let width = float_to_cells(size.width).max(1);
    let height = float_to_cells(size.height).max(1);
    if cursor.col > 0 && cursor.col + width > cursor.width {
        cursor.max_col = cursor.max_col.max(cursor.col);
        cursor.row += 1;
        cursor.col = 0;
    }

    cursor.col += width;
    cursor.max_col = cursor.max_col.max(cursor.col);
    if height > 1 {
        cursor.row += height - 1;
    }
}

fn layout_inline_replaced(
    node: NodeId,
    hit_target: Option<NodeId>,
    size: Size<f32>,
    cursor: &mut InlineLayoutCursor,
) {
    let width = float_to_cells(size.width).max(1);
    let height = float_to_cells(size.height).max(1);
    if cursor.col > 0 && cursor.col + width > cursor.width {
        cursor.max_col = cursor.max_col.max(cursor.col);
        cursor.row += 1;
        cursor.col = 0;
    }

    cursor.fragments.push(InlineFragment {
        node,
        hit_node: hit_target.or(Some(node)),
        kind: InlineFragmentKind::Replaced,
        x: cursor.col,
        y: cursor.row,
        width,
        height,
    });
    cursor.col += width;
    cursor.max_col = cursor.max_col.max(cursor.col);
    if height > 1 {
        cursor.row += height - 1;
    }
}

fn axis_max_scroll(overflow: LayoutOverflow, max_scroll: u32) -> u32 {
    if overflow == LayoutOverflow::Scroll {
        max_scroll
    } else {
        0
    }
}

fn text_content_widths(text: &str, white_space: CssWhiteSpace) -> ContentWidths {
    let chars = parse_text_for_white_space(text, white_space);
    if !white_space_allows_wrapping(white_space) {
        let width = max_line_width(&chars, white_space_preserves_newlines(white_space));
        return ContentWidths {
            min: width,
            max: width,
        };
    }

    let mut min_width = 1;
    let mut current_word = 0;
    let mut max_width = 0;
    for character in chars {
        if character == '\r' {
            continue;
        }
        if character == '\n' && white_space_preserves_newlines(white_space) {
            min_width = min_width.max(current_word);
            current_word = 0;
            continue;
        }
        let width = character_cell_width(character) as u32;
        max_width += width;
        if character.is_whitespace() {
            min_width = min_width.max(current_word);
            current_word = 0;
        } else {
            current_word += width;
        }
    }
    min_width = min_width.max(current_word);

    ContentWidths {
        min: min_width.max(1) as f32,
        max: max_width.max(1) as f32,
    }
}

fn measure_inline_text(text: &str, white_space: CssWhiteSpace, cursor: &mut InlineMeasureCursor) {
    let chars = parse_text_for_white_space(text, white_space);
    let wrap = white_space_allows_wrapping(white_space);
    let preserve_newlines = white_space_preserves_newlines(white_space);
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if character == '\r' {
            index += 1;
            continue;
        }
        if character == '\n' && preserve_newlines {
            cursor.max_col = cursor.max_col.max(cursor.col);
            cursor.row += 1;
            cursor.col = 0;
            index += 1;
            continue;
        }
        if wrap && is_word_start(&chars, index) {
            let word_end = next_word_end(&chars, index);
            let word_width = text_width(&chars[index..word_end]);
            if word_width <= cursor.width
                && cursor.col > 0
                && cursor.col + word_width > cursor.width
            {
                cursor.max_col = cursor.max_col.max(cursor.col);
                cursor.row += 1;
                cursor.col = 0;
            }
        }
        let width = character_cell_width(character) as u32;
        if wrap && cursor.col > 0 && width > 0 && cursor.col + width > cursor.width {
            cursor.max_col = cursor.max_col.max(cursor.col);
            cursor.row += 1;
            cursor.col = 0;
            if character == ' ' {
                index += 1;
                continue;
            }
        }
        cursor.col += width;
        cursor.max_col = cursor.max_col.max(cursor.col);
        index += 1;
    }
}

fn layout_inline_text(
    node: NodeId,
    hit_target: Option<NodeId>,
    text: &str,
    white_space: CssWhiteSpace,
    cursor: &mut InlineLayoutCursor,
) {
    let chars = parse_text_for_white_space(text, white_space);
    let wrap = white_space_allows_wrapping(white_space);
    let preserve_newlines = white_space_preserves_newlines(white_space);
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if character == '\r' {
            index += 1;
            continue;
        }
        if character == '\n' && preserve_newlines {
            cursor.max_col = cursor.max_col.max(cursor.col);
            cursor.row += 1;
            cursor.col = 0;
            index += 1;
            continue;
        }
        if wrap && is_word_start(&chars, index) {
            let word_end = next_word_end(&chars, index);
            let word_width = text_width(&chars[index..word_end]);
            if word_width <= cursor.width
                && cursor.col > 0
                && cursor.col + word_width > cursor.width
            {
                cursor.max_col = cursor.max_col.max(cursor.col);
                cursor.row += 1;
                cursor.col = 0;
            }
        }
        let width = character_cell_width(character) as u32;
        if wrap && cursor.col > 0 && width > 0 && cursor.col + width > cursor.width {
            cursor.max_col = cursor.max_col.max(cursor.col);
            cursor.row += 1;
            cursor.col = 0;
            if character == ' ' {
                index += 1;
                continue;
            }
        }
        if width > 0 {
            let selection_order = cursor.selection_order;
            cursor.selection_order += 1;
            cursor.fragments.push(InlineFragment {
                node,
                hit_node: hit_target,
                kind: InlineFragmentKind::Text {
                    character,
                    selection_order,
                },
                x: cursor.col,
                y: cursor.row,
                width,
                height: 1,
            });
        }
        cursor.col += width;
        cursor.max_col = cursor.max_col.max(cursor.col);
        index += 1;
    }
}

fn white_space_allows_wrapping(white_space: CssWhiteSpace) -> bool {
    matches!(
        white_space,
        CssWhiteSpace::Normal | CssWhiteSpace::PreWrap | CssWhiteSpace::PreLine
    )
}

fn white_space_preserves_newlines(white_space: CssWhiteSpace) -> bool {
    matches!(
        white_space,
        CssWhiteSpace::Pre | CssWhiteSpace::PreWrap | CssWhiteSpace::PreLine
    )
}

fn max_line_width(chars: &[char], preserve_newlines: bool) -> f32 {
    let mut max_width = 1;
    let mut width = 0;
    for character in chars {
        if *character == '\r' {
            continue;
        }
        if *character == '\n' && preserve_newlines {
            max_width = max_width.max(width);
            width = 0;
            continue;
        }
        width += character_cell_width(*character) as u32;
    }
    max_width.max(width).max(1) as f32
}

fn next_word_end(chars: &[char], start: usize) -> usize {
    let mut index = start;
    while index < chars.len() && !chars[index].is_whitespace() {
        index += 1;
    }
    index
}

fn is_word_start(chars: &[char], index: usize) -> bool {
    !chars[index].is_whitespace() && (index == 0 || chars[index - 1].is_whitespace())
}

fn text_width(chars: &[char]) -> u32 {
    chars
        .iter()
        .map(|character| character_cell_width(*character) as u32)
        .sum()
}

fn effective_white_space(inherited: CssWhiteSpace, own: CssWhiteSpace) -> CssWhiteSpace {
    if own == CssWhiteSpace::Normal {
        inherited
    } else {
        own
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{
        BorderStyle, CssDimension, CssGridTemplateTrack, CssLengthPercentage,
        CssLengthPercentageAuto, CssPosition, CssTrackSizing, CssWhiteSpace, LayoutAlignItems,
        LayoutFlexDirection, LayoutJustifyContent, LayoutOverflow, ScrollbarGutter,
    };

    fn block_style(width: CssDimension, height: CssDimension) -> DivStyle {
        let mut style = DivStyle::default();
        style.width = width;
        style.height = height;
        style
    }

    fn fixed_box(arena: &mut LayoutArena, width: f32, height: f32) -> NodeId {
        arena.create_element(block_style(
            CssDimension::Length(width),
            CssDimension::Length(height),
        ))
    }

    #[test]
    fn relative_position_offsets_paint_box_without_changing_sibling_flow() {
        let mut arena = LayoutArena::new();
        let root = fixed_box(&mut arena, 10.0, 8.0);
        let mut positioned_style =
            block_style(CssDimension::Length(4.0), CssDimension::Length(2.0));
        positioned_style.position = CssPosition::Relative;
        positioned_style.left = CssLengthPercentageAuto::Length(2.0);
        positioned_style.top = CssLengthPercentageAuto::Length(1.0);
        let positioned = arena.create_element(positioned_style);
        let sibling = fixed_box(&mut arena, 4.0, 2.0);
        arena.append_child(root, positioned);
        arena.append_child(root, sibling);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(8.0),
            },
        );

        assert_eq!(arena.layout(positioned).location, Point { x: 2.0, y: 1.0 });
        assert_eq!(arena.layout(sibling).location, Point { x: 0.0, y: 2.0 });
    }

    #[test]
    fn absolute_descendant_uses_nearest_non_static_ancestor_and_leaves_flow() {
        let mut arena = LayoutArena::new();
        let root = fixed_box(&mut arena, 20.0, 10.0);
        let mut containing_style =
            block_style(CssDimension::Length(12.0), CssDimension::Length(6.0));
        containing_style.position = CssPosition::Relative;
        let containing = arena.create_element(containing_style);
        let wrapper =
            arena.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
        let flow_child = fixed_box(&mut arena, 5.0, 2.0);
        let mut absolute_style = block_style(CssDimension::Length(3.0), CssDimension::Length(2.0));
        absolute_style.position = CssPosition::Absolute;
        absolute_style.left = CssLengthPercentageAuto::Percent(0.5);
        absolute_style.top = CssLengthPercentageAuto::Length(1.0);
        let absolute = arena.create_element(absolute_style);

        arena.append_child(root, containing);
        arena.append_child(containing, wrapper);
        arena.append_child(wrapper, flow_child);
        arena.append_child(wrapper, absolute);
        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(10.0),
            },
        );

        assert_eq!(arena.parent(absolute), Some(wrapper));
        assert_eq!(arena.layout(wrapper).size.height, 2.0);
        assert_eq!(arena.layout(absolute).location, Point { x: 6.0, y: 1.0 });
        assert_eq!(
            arena.child_layout_offset(wrapper, absolute),
            Point { x: 6.0, y: 1.0 }
        );
    }

    #[test]
    fn absolute_child_is_laid_out_when_containing_block_has_inline_content() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(5.0));
        root_style.position = CssPosition::Relative;
        let root = arena.create_element(root_style);
        let text = arena.create_text("inline content");
        let mut absolute_style = block_style(CssDimension::Length(3.0), CssDimension::Length(1.0));
        absolute_style.position = CssPosition::Absolute;
        absolute_style.right = CssLengthPercentageAuto::Length(1.0);
        absolute_style.bottom = CssLengthPercentageAuto::Length(1.0);
        let absolute = arena.create_element(absolute_style);
        arena.append_child(root, text);
        arena.append_child(root, absolute);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(12.0),
                height: AvailableSpace::Definite(5.0),
            },
        );

        assert_eq!(
            arena.layout(absolute).size,
            Size {
                width: 3.0,
                height: 1.0
            }
        );
        assert_eq!(arena.layout(absolute).location, Point { x: 8.0, y: 3.0 });
        assert_eq!(arena.inline_fragments(root).len(), "inline content".len());
    }

    #[test]
    fn absolute_descendant_with_auto_insets_keeps_its_static_position() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(10.0), CssDimension::Length(8.0));
        root_style.position = CssPosition::Relative;
        let root = arena.create_element(root_style);
        let spacer = fixed_box(&mut arena, 10.0, 2.0);
        let wrapper = fixed_box(&mut arena, 10.0, 2.0);
        let mut absolute_style = block_style(CssDimension::Length(3.0), CssDimension::Length(1.0));
        absolute_style.position = CssPosition::Absolute;
        let absolute = arena.create_element(absolute_style);

        arena.append_child(root, spacer);
        arena.append_child(root, wrapper);
        arena.append_child(wrapper, absolute);
        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(8.0),
            },
        );

        assert_eq!(arena.layout(wrapper).location.y, 2.0);
        assert_eq!(arena.absolute_layout_origin(absolute).y, 2.0);
    }

    #[test]
    fn absolute_percentage_size_resolves_against_containing_block() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(6.0));
        root_style.position = CssPosition::Relative;
        let root = arena.create_element(root_style);
        let wrapper = fixed_box(&mut arena, 4.0, 2.0);
        let mut absolute_style =
            block_style(CssDimension::Percent(0.5), CssDimension::Percent(0.5));
        absolute_style.position = CssPosition::Absolute;
        absolute_style.left = CssLengthPercentageAuto::Length(0.0);
        absolute_style.top = CssLengthPercentageAuto::Length(0.0);
        let absolute = arena.create_element(absolute_style);
        arena.append_child(root, wrapper);
        arena.append_child(wrapper, absolute);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(12.0),
                height: AvailableSpace::Definite(6.0),
            },
        );

        assert_eq!(
            arena.layout(absolute).size,
            Size {
                width: 6.0,
                height: 3.0
            }
        );
    }

    #[test]
    fn absolute_inline_descendant_uses_inline_static_position() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(8.0), CssDimension::Length(2.0));
        root_style.position = CssPosition::Relative;
        let root = arena.create_element(root_style);
        let before = arena.create_text("abc");
        let mut absolute_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        absolute_style.position = CssPosition::Absolute;
        let absolute = arena.create_element(absolute_style);
        let after = arena.create_text("d");
        arena.append_child(root, before);
        arena.append_child(root, absolute);
        arena.append_child(root, after);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(8.0),
                height: AvailableSpace::Definite(2.0),
            },
        );

        assert_eq!(
            arena.absolute_layout_origin(absolute),
            Point { x: 3.0, y: 0.0 }
        );
        assert_eq!(
            arena
                .inline_fragments(root)
                .iter()
                .filter(|fragment| matches!(fragment.kind, InlineFragmentKind::Text { .. }))
                .count(),
            4
        );
    }

    #[test]
    fn absolute_positioning_lays_out_replaced_node_types() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(10.0), CssDimension::Length(5.0));
        root_style.position = CssPosition::Relative;
        let root = arena.create_element(root_style);

        let absolute_style = |top| {
            let mut style = block_style(CssDimension::Length(3.0), CssDimension::Length(1.0));
            style.position = CssPosition::Absolute;
            style.left = CssLengthPercentageAuto::Length(2.0);
            style.top = CssLengthPercentageAuto::Length(top);
            style
        };
        let image = arena.create_image(absolute_style(0.0), 6, 2, 2, 2);
        let input = arena.create_input(absolute_style(1.0), "input");
        let textarea = arena.create_textarea(absolute_style(2.0), "textarea");
        arena.append_child(root, image);
        arena.append_child(root, input);
        arena.append_child(root, textarea);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(5.0),
            },
        );

        assert_eq!(arena.layout(image).location, Point { x: 2.0, y: 0.0 });
        assert_eq!(arena.layout(input).location, Point { x: 2.0, y: 1.0 });
        assert_eq!(arena.layout(textarea).location, Point { x: 2.0, y: 2.0 });
    }

    #[test]
    fn absolute_auto_margins_center_between_opposing_insets() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(10.0), CssDimension::Length(3.0));
        root_style.position = CssPosition::Relative;
        let root = arena.create_element(root_style);
        let mut child_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        child_style.position = CssPosition::Absolute;
        child_style.left = CssLengthPercentageAuto::Length(0.0);
        child_style.right = CssLengthPercentageAuto::Length(0.0);
        child_style.margin_left = CssLengthPercentageAuto::Auto;
        child_style.margin_right = CssLengthPercentageAuto::Auto;
        let child = arena.create_element(child_style);
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        assert_eq!(arena.layout(child).location.x, 4.0);
        assert_eq!(arena.layout(child).margin.left, 4.0);
        assert_eq!(arena.layout(child).margin.right, 4.0);
    }

    fn image_scroll_demo_image_block(arena: &mut LayoutArena, label: &str) -> NodeId {
        let mut style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        style.display = LayoutDisplay::Flex;
        style.flex_direction = LayoutFlexDirection::Column;
        style.row_gap = CssLengthPercentage::Length(1.0);
        let block = arena.create_element(style);

        let title =
            arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
        let title_text = arena.create_text(label);
        arena.append_child(title, title_text);
        arena.append_child(block, title);

        let image = arena.create_image(
            block_style(CssDimension::Length(48.0), CssDimension::Length(14.0)),
            16,
            8,
            8,
            16,
        );
        arena.append_child(block, image);
        block
    }

    #[test]
    fn inline_context_gets_full_layout_input_and_wraps_at_resolved_percent_width() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(10.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let row = arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
        let text = arena.create_text("word word word word word word");
        arena.append_child(row, text);
        arena.append_child(root, row);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(10.0),
            },
        );

        let row_layout = arena.layout(row);
        assert_eq!(row_layout.size.width, 20.0);
        assert!(row_layout.size.height > 1.0);
    }

    #[test]
    fn auto_width_inline_context_uses_min_content_for_min_content_query() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let text = arena.create_text("one threeeeee two");
        arena.append_child(row, text);

        let output = arena.compute_inline_layout(
            row,
            LayoutInput {
                known_dimensions: Size::NONE,
                parent_size: Size::NONE,
                available_space: Size {
                    width: AvailableSpace::MinContent,
                    height: AvailableSpace::MaxContent,
                },
                sizing_mode: SizingMode::InherentSize,
                axis: taffy::RequestedAxis::Both,
                run_mode: RunMode::ComputeSize,
                vertical_margins_are_collapsible: taffy::Line::FALSE,
            },
        );

        assert_eq!(output.size.width, 9.0);
        assert_eq!(output.size.height, 3.0);
    }

    #[test]
    fn percent_width_inline_context_resolves_against_parent_size() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
        let text = arena.create_text("one threeeeee two");
        arena.append_child(row, text);

        let output = arena.compute_inline_layout(
            row,
            LayoutInput {
                known_dimensions: Size::NONE,
                parent_size: Size {
                    width: Some(20.0),
                    height: None,
                },
                available_space: Size {
                    width: AvailableSpace::Definite(20.0),
                    height: AvailableSpace::MaxContent,
                },
                sizing_mode: SizingMode::InherentSize,
                axis: taffy::RequestedAxis::Both,
                run_mode: RunMode::ComputeSize,
                vertical_margins_are_collapsible: taffy::Line::FALSE,
            },
        );

        assert_eq!(output.size.width, 20.0);
        assert_eq!(output.size.height, 1.0);
    }

    #[test]
    fn style_updates_clear_cached_layout() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Length(20.0), CssDimension::Auto));
        let text = arena.create_text("word word word word");
        arena.append_child(row, text);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::MaxContent,
            },
        );
        assert_eq!(arena.layout(row).size.width, 20.0);

        arena.set_style(
            row,
            block_style(CssDimension::Length(10.0), CssDimension::Auto),
        );
        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(arena.layout(row).size.width, 10.0);
        assert!(arena.layout(row).size.height > 1.0);
    }

    #[test]
    fn block_layout_places_children_vertically() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(20.0),
            CssDimension::Length(10.0),
        ));
        let first = fixed_box(&mut arena, 5.0, 2.0);
        let second = fixed_box(&mut arena, 6.0, 3.0);
        arena.append_child(root, first);
        arena.append_child(root, second);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(10.0),
            },
        );

        assert_eq!(arena.layout(first).location.y, 0.0);
        assert_eq!(arena.layout(second).location.y, 2.0);
    }

    #[test]
    fn insert_child_before_reorders_without_duplicate_children() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let first = fixed_box(&mut arena, 1.0, 1.0);
        let second = fixed_box(&mut arena, 1.0, 1.0);
        let third = fixed_box(&mut arena, 1.0, 1.0);
        arena.append_child(root, first);
        arena.append_child(root, third);

        arena.insert_child_before(root, second, third);
        assert_eq!(arena.children(root), &[first, second, third]);

        arena.insert_child_before(root, third, second);
        assert_eq!(arena.children(root), &[first, third, second]);
    }

    #[test]
    fn padding_and_margin_affect_layout() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Auto);
        root_style.padding_top = CssLengthPercentage::Length(1.0);
        root_style.padding_left = CssLengthPercentage::Length(2.0);
        let root = arena.create_element(root_style);
        let mut child_style = block_style(CssDimension::Length(4.0), CssDimension::Length(2.0));
        child_style.margin_top = CssLengthPercentageAuto::Length(3.0);
        child_style.margin_left = CssLengthPercentageAuto::Length(5.0);
        let child = arena.create_element(child_style);
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(arena.layout(child).location.x, 7.0);
        assert_eq!(arena.layout(child).location.y, 4.0);
        assert_eq!(arena.layout(root).size.height, 6.0);
    }

    #[test]
    fn auto_height_inline_context_includes_symmetric_vertical_padding() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        root_style.padding_top = CssLengthPercentage::Length(1.0);
        root_style.padding_right = CssLengthPercentage::Length(4.0);
        root_style.padding_bottom = CssLengthPercentage::Length(1.0);
        root_style.padding_left = CssLengthPercentage::Length(4.0);
        let root = arena.create_element(root_style);
        let text = arena.create_text("hi");
        arena.append_child(root, text);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(root);
        assert_eq!(layout.size.height, 3.0);
        assert_eq!(layout.padding.top, 1.0);
        assert_eq!(layout.padding.bottom, 1.0);
    }

    #[test]
    fn auto_margins_are_passed_to_taffy() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(5.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Row;
        let root = arena.create_element(root_style);
        let mut child_style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
        child_style.margin_left = CssLengthPercentageAuto::Auto;
        child_style.margin_right = CssLengthPercentageAuto::Auto;
        let child = arena.create_element(child_style);
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(5.0),
            },
        );

        assert_eq!(arena.layout(child).location.x, 8.0);
    }

    #[test]
    fn flex_layout_places_children_on_main_axis() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(5.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Row;
        let root = arena.create_element(root_style);
        let first = fixed_box(&mut arena, 5.0, 1.0);
        let second = fixed_box(&mut arena, 6.0, 1.0);
        arena.append_child(root, first);
        arena.append_child(root, second);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(5.0),
            },
        );

        assert_eq!(arena.layout(first).location.x, 0.0);
        assert_eq!(arena.layout(second).location.x, 5.0);
    }

    #[test]
    fn grid_layout_uses_template_tracks() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(5.0));
        root_style.display = LayoutDisplay::Grid;
        root_style.grid_template_columns = vec![
            CssGridTemplateTrack::Single(CssTrackSizing::Length(5.0)),
            CssGridTemplateTrack::Single(CssTrackSizing::Length(7.0)),
        ];
        let root = arena.create_element(root_style);
        let first = fixed_box(&mut arena, 1.0, 1.0);
        let second = fixed_box(&mut arena, 1.0, 1.0);
        arena.append_child(root, first);
        arena.append_child(root, second);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(5.0),
            },
        );

        assert_eq!(arena.layout(first).location.x, 0.0);
        assert_eq!(arena.layout(second).location.x, 5.0);
    }

    #[test]
    fn percent_size_resolves_against_parent() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(20.0),
            CssDimension::Length(10.0),
        ));
        let child = arena.create_element(block_style(
            CssDimension::Percent(0.5),
            CssDimension::Percent(0.5),
        ));
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(10.0),
            },
        );

        assert_eq!(arena.layout(child).size.width, 10.0);
        assert_eq!(arena.layout(child).size.height, 5.0);
    }

    #[test]
    fn percent_child_inside_fractional_bordered_parent_fits_rounded_content_box() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(81.0), CssDimension::Length(6.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Row;
        let root = arena.create_element(root_style);

        let mut column_style = block_style(CssDimension::Auto, CssDimension::Percent(1.0));
        column_style.display = LayoutDisplay::Flex;
        column_style.flex_direction = LayoutFlexDirection::Column;
        column_style.flex_grow = 1.0;
        column_style.flex_shrink = 1.0;
        column_style.flex_basis = CssDimension::Length(0.0);
        column_style.border_top = BorderStyle::Rounded;
        column_style.border_right = BorderStyle::Rounded;
        column_style.border_bottom = BorderStyle::Rounded;
        column_style.border_left = BorderStyle::Rounded;
        let left = arena.create_element(column_style.clone());
        let right = arena.create_element(column_style);
        arena.append_child(root, left);
        arena.append_child(root, right);

        let mut child_style = block_style(CssDimension::Percent(1.0), CssDimension::Length(4.0));
        child_style.border_top = BorderStyle::ChunkyRounded;
        child_style.border_right = BorderStyle::ChunkyRounded;
        child_style.border_bottom = BorderStyle::ChunkyRounded;
        child_style.border_left = BorderStyle::ChunkyRounded;
        let child = arena.create_element(child_style);
        arena.append_child(right, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(81.0),
                height: AvailableSpace::Definite(6.0),
            },
        );

        let right_layout = arena.layout(right);
        let child_layout = arena.layout(child);
        let rounded_right_content_width =
            (right_layout.location.x + right_layout.size.width - right_layout.border.right).round()
                - (right_layout.location.x + right_layout.border.left).round();
        let rounded_child_width = (child_layout.location.x + child_layout.size.width).round()
            - child_layout.location.x.round();

        assert_eq!(right_layout.location.x, 41.0);
        assert_eq!(right_layout.size.width, 40.0);
        assert_eq!(child_layout.location.x, 1.0);
        assert_eq!(child_layout.size.width, 38.0);
        assert_eq!(rounded_right_content_width, 38.0);
        assert_eq!(rounded_child_width, rounded_right_content_width);
    }

    #[test]
    fn min_height_expands_auto_height_inline_context() {
        let mut arena = LayoutArena::new();
        let mut style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        style.min_height = CssDimension::Length(5.0);
        let row = arena.create_element(style);
        let text = arena.create_text("short");
        arena.append_child(row, text);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(arena.layout(row).size.height, 5.0);
    }

    #[test]
    fn min_width_expands_an_explicitly_smaller_child() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(1.0),
        ));

        let mut child_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        child_style.min_width = CssDimension::Length(5.0);
        let child = arena.create_element(child_style);
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(1.0),
            },
        );

        assert_eq!(arena.layout(child).size.width, 5.0);
    }

    #[test]
    fn percent_max_width_constrains_child_against_parent_width() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(1.0),
        ));

        let mut child_style = block_style(CssDimension::Percent(1.0), CssDimension::Length(1.0));
        child_style.max_width = CssDimension::Percent(0.5);
        let child = arena.create_element(child_style);
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(1.0),
            },
        );

        assert_eq!(arena.layout(child).size.width, 5.0);
    }

    #[test]
    fn percent_max_height_constrains_scroll_container() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(10.0),
        ));

        let mut viewport_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        viewport_style.max_height = CssDimension::Percent(0.5);
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);

        let content = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(10.0),
        ));
        arena.append_child(viewport, content);
        arena.append_child(root, viewport);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(10.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(arena.layout(viewport).size.height, 5.0);
        assert_eq!(metrics.client_height, 5);
        assert_eq!(metrics.scroll_height, 10);
    }

    #[test]
    fn less_demo_inline_viewport_scrolls_inside_flex_column() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let header = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(1.0),
        ));
        arena.append_child(root, header);

        let mut body_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        body_style.display = LayoutDisplay::Flex;
        body_style.flex_direction = LayoutFlexDirection::Row;
        body_style.flex_grow = 1.0;
        body_style.flex_shrink = 1.0;
        body_style.flex_basis = CssDimension::Length(0.0);
        body_style.min_height = CssDimension::Length(0.0);
        let body = arena.create_element(body_style);
        arena.append_child(root, body);

        let mut viewport_style =
            block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.overflow_x = LayoutOverflow::Hidden;
        viewport_style.border_top = BorderStyle::Rounded;
        viewport_style.border_right = BorderStyle::Rounded;
        viewport_style.border_bottom = BorderStyle::Rounded;
        viewport_style.border_left = BorderStyle::Rounded;
        let viewport = arena.create_element(viewport_style);
        arena.append_child(body, viewport);

        let mut span_style = block_style(CssDimension::Auto, CssDimension::Auto);
        span_style.display = LayoutDisplay::Inline;
        span_style.white_space = CssWhiteSpace::PreWrap;
        let span = arena.create_element(span_style);
        let text = arena.create_text("long line wraps here\n\n".repeat(80));
        arena.append_child(span, text);
        arena.append_child(viewport, span);

        let footer = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(1.0),
        ));
        arena.append_child(root, footer);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(80.0),
                height: AvailableSpace::Definite(24.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(arena.layout(body).size.height, 22.0);
        assert_eq!(arena.layout(viewport).size.height, 22.0);
        assert_eq!(metrics.client_height, 20);
        assert!(metrics.scroll_height > metrics.client_height);
    }

    #[test]
    fn scroll_flex_item_auto_min_height_is_zero() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(10.0), CssDimension::Length(6.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let header = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(2.0),
        ));
        arena.append_child(root, header);

        let mut viewport_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        viewport_style.flex_grow = 1.0;
        viewport_style.flex_shrink = 1.0;
        viewport_style.flex_basis = CssDimension::Length(0.0);
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        arena.append_child(root, viewport);

        let content = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(20.0),
        ));
        arena.append_child(viewport, content);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(6.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(arena.layout(viewport).size.height, 4.0);
        assert_eq!(metrics.client_height, 4);
        assert_eq!(metrics.scroll_height, 20);
    }

    #[test]
    fn vertical_scrollbar_reserves_one_cell_for_child_layout_and_metrics() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);

        let child = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(5.0),
        ));
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let viewport_layout = arena.layout(viewport);
        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(viewport_layout.scrollbar_size.width, 1.0);
        assert_eq!(arena.layout(child).size.width, 5.0);
        assert_eq!(metrics.client_width, 5);
        assert_eq!(metrics.client_height, 3);
        assert_eq!(metrics.scroll_height, 5);
    }

    #[test]
    fn horizontal_scrollbar_reserves_one_cell_for_child_layout_and_metrics() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.overflow_x = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);

        let child = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Percent(1.0),
        ));
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let viewport_layout = arena.layout(viewport);
        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(viewport_layout.scrollbar_size.height, 1.0);
        assert_eq!(arena.layout(child).size.height, 2.0);
        assert_eq!(metrics.client_width, 6);
        assert_eq!(metrics.client_height, 2);
        assert_eq!(metrics.scroll_width, 10);
    }

    #[test]
    fn scrollbar_hit_testing_reports_vertical_and_horizontal_rail_geometry() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.overflow_x = LayoutOverflow::Scroll;
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);

        let child = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(5.0),
        ));
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let vertical = arena.scrollbar_hit_for_point(viewport, 5, 1).unwrap();
        assert_eq!(vertical.axis, ScrollbarAxis::Vertical);
        assert_eq!(vertical.rail_start, 0);
        assert_eq!(vertical.rail_length, 2);
        assert_eq!(vertical.thumb_start, 0);
        assert_eq!(vertical.thumb_length, 1);
        assert_eq!(vertical.max_scroll, 3);
        assert_eq!(vertical.client_length, 2);
        assert_eq!(vertical.scroll_length, 5);

        let horizontal = arena.scrollbar_hit_for_point(viewport, 2, 2).unwrap();
        assert_eq!(horizontal.axis, ScrollbarAxis::Horizontal);
        assert_eq!(horizontal.rail_start, 0);
        assert_eq!(horizontal.rail_length, 5);
        assert_eq!(horizontal.thumb_start, 0);
        assert_eq!(horizontal.thumb_length, 2);
        assert_eq!(horizontal.max_scroll, 5);
        assert_eq!(horizontal.client_length, 5);
        assert_eq!(horizontal.scroll_length, 10);

        assert!(arena.scrollbar_hit_for_point(viewport, 5, 2).is_none());
        assert!(arena.scrollbar_hit_for_point(child, 5, 1).is_none());
    }

    #[test]
    fn stable_gutter_reserves_vertical_space_for_hidden_overflow_without_scrolling() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.overflow_y = LayoutOverflow::Hidden;
        viewport_style.scrollbar_gutter = ScrollbarGutter::Stable;
        let viewport = arena.create_element(viewport_style);

        let child = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(5.0),
        ));
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let viewport_layout = arena.layout(viewport);
        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(viewport_layout.scrollbar_size.width, 1.0);
        assert_eq!(arena.layout(child).size.width, 5.0);
        assert_eq!(metrics.client_width, 5);
        assert_eq!(metrics.scroll_left, 0);
        assert_eq!(metrics.scroll_top, 0);
    }

    #[test]
    fn stable_gutter_reserves_horizontal_space_for_hidden_overflow_without_scrolling() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.overflow_x = LayoutOverflow::Hidden;
        viewport_style.scrollbar_gutter = ScrollbarGutter::Stable;
        let viewport = arena.create_element(viewport_style);

        let child = arena.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Percent(1.0),
        ));
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let viewport_layout = arena.layout(viewport);
        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(viewport_layout.scrollbar_size.height, 1.0);
        assert_eq!(arena.layout(child).size.height, 2.0);
        assert_eq!(metrics.client_height, 2);
        assert_eq!(metrics.scroll_left, 0);
        assert_eq!(metrics.scroll_top, 0);
    }

    #[test]
    fn vertical_scrollbar_reduces_inline_wrap_width() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Length(3.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let text = arena.create_text("abcde");
        arena.append_child(viewport, text);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let fragments = arena.inline_fragments(viewport);
        assert_eq!(fragments[3].x, 3);
        assert_eq!(fragments[3].y, 0);
        assert_eq!(fragments[4].x, 0);
        assert_eq!(fragments[4].y, 1);
    }

    #[test]
    fn min_height_zero_visible_wrapper_around_scroll_container_can_shrink() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(8.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let mut header_style = block_style(CssDimension::Length(20.0), CssDimension::Length(3.0));
        header_style.flex_shrink = 0.0;
        let header = arena.create_element(header_style);
        arena.append_child(root, header);

        let mut wrapper_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        wrapper_style.display = LayoutDisplay::Flex;
        wrapper_style.flex_direction = LayoutFlexDirection::Row;
        wrapper_style.flex_shrink = 1.0;
        wrapper_style.min_height = CssDimension::Length(0.0);
        let wrapper = arena.create_element(wrapper_style);
        arena.append_child(root, wrapper);

        let mut viewport_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        viewport_style.display = LayoutDisplay::Flex;
        viewport_style.flex_direction = LayoutFlexDirection::Column;
        viewport_style.flex_shrink = 1.0;
        viewport_style.min_height = CssDimension::Length(0.0);
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        arena.append_child(wrapper, viewport);

        for _ in 0..20 {
            let mut row_style = block_style(CssDimension::Length(20.0), CssDimension::Length(1.0));
            row_style.flex_shrink = 0.0;
            let row = arena.create_element(row_style);
            arena.append_child(viewport, row);
        }

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(8.0),
            },
        );

        assert_eq!(arena.layout(header).size.height, 3.0);
        assert_eq!(arena.layout(wrapper).size.height, 5.0);
        assert_eq!(arena.layout(viewport).size.height, 5.0);
        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_height, 5);
        assert_eq!(metrics.scroll_height, 20);
    }

    #[test]
    fn flex_row_long_text_does_not_shrink_fixed_width_controls() {
        let mut arena = LayoutArena::new();
        let mut row_style = block_style(CssDimension::Length(72.0), CssDimension::Auto);
        row_style.display = LayoutDisplay::Flex;
        row_style.flex_direction = LayoutFlexDirection::Row;
        row_style.align_items = Some(LayoutAlignItems::Center);
        row_style.column_gap = CssLengthPercentage::Length(1.0);
        let row = arena.create_element(row_style);

        let mut checkbox_style = block_style(CssDimension::Length(3.0), CssDimension::Length(3.0));
        checkbox_style.border_top = BorderStyle::ChunkyRounded;
        checkbox_style.border_right = BorderStyle::ChunkyRounded;
        checkbox_style.border_bottom = BorderStyle::ChunkyRounded;
        checkbox_style.border_left = BorderStyle::ChunkyRounded;
        let checkbox = arena.create_element(checkbox_style);
        let checkbox_text = arena.create_text(" ");
        arena.append_child(checkbox, checkbox_text);
        arena.append_child(row, checkbox);

        let mut text_style = block_style(CssDimension::Auto, CssDimension::Auto);
        text_style.flex_grow = 1.0;
        text_style.flex_shrink = 1.0;
        let text_box = arena.create_element(text_style);
        let text =
            arena.create_text("hahahahahahahahahahahahahahahahahahahahahahahahahahahahahaha");
        arena.append_child(text_box, text);
        arena.append_child(row, text_box);

        let mut edit_style = block_style(CssDimension::Length(3.0), CssDimension::Length(3.0));
        edit_style.border_top = BorderStyle::ChunkyRounded;
        edit_style.border_right = BorderStyle::ChunkyRounded;
        edit_style.border_bottom = BorderStyle::ChunkyRounded;
        edit_style.border_left = BorderStyle::ChunkyRounded;
        let edit = arena.create_element(edit_style);
        let edit_text = arena.create_text("e");
        arena.append_child(edit, edit_text);
        arena.append_child(row, edit);

        let mut delete_style = block_style(CssDimension::Length(3.0), CssDimension::Length(3.0));
        delete_style.border_top = BorderStyle::ChunkyRounded;
        delete_style.border_right = BorderStyle::ChunkyRounded;
        delete_style.border_bottom = BorderStyle::ChunkyRounded;
        delete_style.border_left = BorderStyle::ChunkyRounded;
        let delete = arena.create_element(delete_style);
        let delete_text = arena.create_text("x");
        arena.append_child(delete, delete_text);
        arena.append_child(row, delete);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(72.0),
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(arena.layout(checkbox).size.width, 3.0);
        assert_eq!(arena.layout(edit).size.width, 3.0);
        assert_eq!(arena.layout(delete).size.width, 3.0);
    }

    #[test]
    fn border_contributes_to_inline_context_outer_size() {
        let mut arena = LayoutArena::new();
        let mut style = block_style(CssDimension::Auto, CssDimension::Auto);
        style.border_top = BorderStyle::Solid;
        style.border_right = BorderStyle::Solid;
        style.border_bottom = BorderStyle::Solid;
        style.border_left = BorderStyle::Solid;
        let row = arena.create_element(style);
        let text = arena.create_text("hi");
        arena.append_child(row, text);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(arena.layout(row).size.width, 4.0);
        assert_eq!(arena.layout(row).size.height, 3.0);
    }

    #[test]
    fn overflow_scroll_preserves_declared_container_size() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(5.0), CssDimension::Length(2.0));
        root_style.overflow_y = LayoutOverflow::Scroll;
        let root = arena.create_element(root_style);
        let child = fixed_box(&mut arena, 5.0, 10.0);
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(2.0),
            },
        );

        assert_eq!(arena.layout(root).size.width, 5.0);
        assert_eq!(arena.layout(root).size.height, 2.0);
        assert_eq!(arena.layout(child).size.height, 10.0);
    }

    #[test]
    fn image_uses_intrinsic_cell_size() {
        let mut arena = LayoutArena::new();
        let image = arena.create_image(DivStyle::default(), 80, 48, 8, 16);

        arena.compute_layout(
            image,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(image);
        assert_eq!(layout.size.width, 10.0);
        assert_eq!(layout.size.height, 3.0);
    }

    #[test]
    fn image_explicit_size_overrides_intrinsic_size() {
        let mut arena = LayoutArena::new();
        let image = arena.create_image(
            block_style(CssDimension::Length(7.0), CssDimension::Length(2.0)),
            80,
            48,
            8,
            16,
        );

        arena.compute_layout(
            image,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(image);
        assert_eq!(layout.size.width, 7.0);
        assert_eq!(layout.size.height, 2.0);
    }

    #[test]
    fn inline_image_contributes_to_line_size() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let text = arena.create_text("aa");
        let mut image_style = DivStyle::default();
        image_style.display = LayoutDisplay::Inline;
        let image = arena.create_image(image_style, 16, 32, 8, 16);
        arena.append_child(row, text);
        arena.append_child(row, image);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(row);
        assert_eq!(layout.size.width, 4.0);
        assert_eq!(layout.size.height, 2.0);
    }

    #[test]
    fn input_uses_value_intrinsic_size() {
        let mut arena = LayoutArena::new();
        let input = arena.create_input(DivStyle::default(), "hello");

        arena.compute_layout(
            input,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(input);
        assert_eq!(layout.size.width, 5.0);
        assert_eq!(layout.size.height, 1.0);
    }

    #[test]
    fn inline_input_contributes_to_line_size() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let mut input_style = DivStyle::default();
        input_style.display = LayoutDisplay::Inline;
        let text = arena.create_text("x");
        let input = arena.create_input(input_style, "hello");
        arena.append_child(row, text);
        arena.append_child(row, input);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(row);
        assert_eq!(layout.size.width, 6.0);
        assert_eq!(layout.size.height, 1.0);
    }

    #[test]
    fn textarea_wraps_and_auto_sizes() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Length(5.0), CssDimension::Auto),
            "hello world",
        );

        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(textarea);
        assert_eq!(layout.size.width, 5.0);
        assert_eq!(layout.size.height, 3.0);
    }

    #[test]
    fn textarea_force_wraps_long_unbroken_words_at_width() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Length(4.0), CssDimension::Auto),
            "hahahaha",
        );

        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(textarea);
        assert_eq!(layout.size.width, 4.0);
        assert_eq!(layout.size.height, 3.0);
    }

    #[test]
    fn textarea_exact_wrap_boundary_reserves_a_row_for_the_caret() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Length(4.0), CssDimension::Auto),
            "haha",
        );

        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(arena.layout(textarea).size.height, 2.0);
    }

    #[test]
    fn textarea_cursor_visual_position_uses_resolved_soft_wrap_width() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Length(4.0), CssDimension::Auto),
            "hahahaha",
        );
        arena.set_textarea_value(textarea, "hahahaha", 4);
        assert_eq!(arena.textarea_cursor_visual_position(textarea), None);
        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(
            arena.textarea_cursor_visual_position(textarea),
            Some((1, 0))
        );
        assert_eq!(arena.textarea_visual_line_range(textarea, 0), Some((0, 4)));
        assert_eq!(arena.textarea_visual_line_range(textarea, 1), Some((4, 8)));
        assert_eq!(arena.textarea_visual_line_range(textarea, 2), Some((8, 8)));
        assert_eq!(arena.textarea_visual_line_range(textarea, 3), None);

        arena.set_textarea_value(textarea, "hahahaha", 5);
        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );
        assert_eq!(
            arena.textarea_cursor_visual_position(textarea),
            Some((1, 1))
        );
    }

    #[test]
    fn textarea_unknown_width_measurement_matches_its_returned_width() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Auto, CssDimension::Auto),
            "1234567890",
        );

        let max_content = Size {
            width: AvailableSpace::MaxContent,
            height: AvailableSpace::MaxContent,
        };
        let unknown_width = arena.measure_leaf_node(textarea, Size::NONE, max_content);
        let zero_width = arena.measure_leaf_node(
            textarea,
            Size {
                width: Some(0.0),
                height: None,
            },
            Size {
                width: AvailableSpace::Definite(0.0),
                height: AvailableSpace::MaxContent,
            },
        );
        let known_width = arena.measure_leaf_node(
            textarea,
            Size {
                width: Some(unknown_width.width),
                height: None,
            },
            max_content,
        );

        assert_eq!(
            unknown_width,
            Size {
                width: 10.0,
                height: 2.0,
            }
        );
        assert_eq!(zero_width.height, 2.0);
        assert_eq!(known_width, unknown_width);
    }

    #[test]
    fn exact_wrap_boundary_keeps_bordered_flex_textarea_inside_viewport() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(8.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let mut transcript_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        transcript_style.display = LayoutDisplay::Flex;
        transcript_style.flex_direction = LayoutFlexDirection::Column;
        transcript_style.flex_grow = 1.0;
        transcript_style.flex_shrink = 1.0;
        transcript_style.flex_basis = CssDimension::Length(0.0);
        transcript_style.min_height = CssDimension::Length(0.0);
        transcript_style.overflow_y = LayoutOverflow::Scroll;
        let transcript = arena.create_element(transcript_style);

        let mut bottom_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        bottom_style.display = LayoutDisplay::Flex;
        bottom_style.flex_direction = LayoutFlexDirection::Column;
        let bottom = arena.create_element(bottom_style);

        let mut input_row_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        input_row_style.display = LayoutDisplay::Flex;
        input_row_style.border_top = BorderStyle::Solid;
        input_row_style.border_bottom = BorderStyle::Solid;
        input_row_style.border_color = crate::style::Background::Green;
        input_row_style.column_gap = CssLengthPercentage::Length(1.0);
        let input_row = arena.create_element(input_row_style);

        let mut prompt_style = block_style(CssDimension::Auto, CssDimension::Auto);
        prompt_style.display = LayoutDisplay::Inline;
        let prompt = arena.create_element(prompt_style);
        let prompt_text = arena.create_text(">");
        arena.append_child(prompt, prompt_text);

        let mut input_wrapper_style = block_style(CssDimension::Auto, CssDimension::Auto);
        input_wrapper_style.display = LayoutDisplay::Flex;
        input_wrapper_style.flex_direction = LayoutFlexDirection::Column;
        input_wrapper_style.flex_grow = 1.0;
        input_wrapper_style.flex_shrink = 1.0;
        input_wrapper_style.flex_basis = CssDimension::Length(0.0);
        input_wrapper_style.min_width = CssDimension::Length(0.0);
        let input_wrapper = arena.create_element(input_wrapper_style);

        let mut textarea_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        textarea_style.display = LayoutDisplay::Flex;
        textarea_style.min_width = CssDimension::Length(0.0);
        textarea_style.min_height = CssDimension::Length(1.0);
        textarea_style.flex_grow = 1.0;
        textarea_style.overflow_y = LayoutOverflow::Visible;
        textarea_style.white_space = CssWhiteSpace::PreWrap;
        let textarea = arena.create_textarea(textarea_style, "123456789");
        arena.set_textarea_focused(textarea, true);

        let status = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(1.0),
        ));
        let status_text = arena.create_text("status");
        arena.append_child(status, status_text);

        arena.append_child(input_wrapper, textarea);
        arena.append_child(input_row, prompt);
        arena.append_child(input_row, input_wrapper);
        arena.append_child(bottom, input_row);
        arena.append_child(bottom, status);
        arena.append_child(root, transcript);
        arena.append_child(root, bottom);

        for (value, expected_input_height, expected_transcript_height) in [
            ("123456789", 3.0, 4.0),
            ("1234567890", 4.0, 3.0),
            ("12345678901", 4.0, 3.0),
        ] {
            arena.set_textarea_value(textarea, value, value.len() as u32);
            arena.compute_layout(
                root,
                Size {
                    width: AvailableSpace::Definite(12.0),
                    height: AvailableSpace::Definite(8.0),
                },
            );

            assert_eq!(arena.layout(root).size.height, 8.0, "value: {value}");
            assert_eq!(
                arena.layout(transcript).size.height,
                expected_transcript_height,
                "value: {value}"
            );
            assert_eq!(
                arena.layout(input_row).size.height,
                expected_input_height,
                "value: {value}"
            );
            assert_eq!(
                arena.layout(status).location.y + arena.layout(status).size.height,
                arena.layout(bottom).size.height,
                "value: {value}"
            );
            assert_eq!(
                arena.layout(transcript).size.height + arena.layout(bottom).size.height,
                arena.layout(root).size.height,
                "value: {value}"
            );

            if value.len() == 10 {
                let output = crate::paint::paint_arena(&arena, root, 12, 8, false);
                let top_border = output.frame.cell(0, 3).unwrap();
                let cursor = output.frame.cell(2, 5).unwrap();
                let bottom_border = output.frame.cell(0, 6).unwrap();

                assert_eq!(top_border.character, '─');
                assert_eq!(top_border.foreground, crate::style::Background::Green);
                assert!(cursor.reversed);
                assert_eq!(bottom_border.character, '─');
                assert_eq!(bottom_border.foreground, crate::style::Background::Green);
                assert_eq!(output.frame.cell(0, 7).unwrap().character, 's');
            }
        }
    }

    #[test]
    fn textarea_reports_scroll_metrics_and_clamps_offset() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Length(5.0), CssDimension::Length(2.0)),
            "a\nb\nc\nd",
        );

        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let metrics = arena.scroll_metrics(textarea).unwrap();
        assert_eq!(metrics.client_width, 5);
        assert_eq!(metrics.client_height, 2);
        assert_eq!(metrics.scroll_width, 5);
        assert_eq!(metrics.scroll_height, 4);
        assert_eq!(metrics.scroll_top, 0);

        let metrics = arena.set_scroll_offset(textarea, 10, 99).unwrap();
        assert_eq!(metrics.scroll_left, 0);
        assert_eq!(metrics.scroll_top, 2);
    }

    #[test]
    fn bordered_textarea_soft_wrap_measures_against_content_box_width() {
        let mut arena = LayoutArena::new();
        let mut style = block_style(CssDimension::Length(8.0), CssDimension::Auto);
        style.border_top = BorderStyle::Rounded;
        style.border_right = BorderStyle::Rounded;
        style.border_bottom = BorderStyle::Rounded;
        style.border_left = BorderStyle::Rounded;
        let textarea = arena.create_textarea(style, "abcdefg");
        arena.set_textarea_value(textarea, "abcdefg", 6);

        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(textarea);
        assert_eq!(layout.size.width, 8.0);
        assert_eq!(layout.size.height, 4.0);
        assert_eq!(
            arena.textarea_cursor_visual_position(textarea),
            Some((1, 0))
        );
        assert_eq!(arena.textarea_visual_line_range(textarea, 0), Some((0, 6)));
        assert_eq!(arena.textarea_visual_line_range(textarea, 1), Some((6, 7)));
    }

    #[test]
    fn textarea_auto_sizes_inside_centered_flex_column_after_value_change() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(80.0), CssDimension::Length(24.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        root_style.justify_content = Some(LayoutJustifyContent::Center);
        root_style.align_items = Some(LayoutAlignItems::Center);
        root_style.row_gap = crate::style::CssLengthPercentage::Length(1.0);
        let root = arena.create_element(root_style);

        let title = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let mut textarea_style = block_style(CssDimension::Length(48.0), CssDimension::Auto);
        textarea_style.min_height = CssDimension::Length(5.0);
        textarea_style.border_top = BorderStyle::Rounded;
        textarea_style.border_right = BorderStyle::Rounded;
        textarea_style.border_bottom = BorderStyle::Rounded;
        textarea_style.border_left = BorderStyle::Rounded;
        let textarea = arena.create_textarea(textarea_style, "short");
        let submitted = arena.create_element(block_style(
            CssDimension::Length(48.0),
            CssDimension::Length(3.0),
        ));
        arena.append_child(root, title);
        arena.append_child(root, textarea);
        arena.append_child(root, submitted);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(80.0),
                height: AvailableSpace::Definite(24.0),
            },
        );
        assert_eq!(arena.layout(textarea).size.height, 5.0);

        arena.set_textarea_value(textarea, "one\ntwo\nthree\nfour\nfive\nsix\nseven", 0);
        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(80.0),
                height: AvailableSpace::Definite(24.0),
            },
        );

        assert!(arena.layout(textarea).size.height > 5.0);
    }

    #[test]
    fn textarea_auto_sizes_for_soft_wrapped_text_inside_centered_flex_column() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(80.0), CssDimension::Length(24.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        root_style.justify_content = Some(LayoutJustifyContent::Center);
        root_style.align_items = Some(LayoutAlignItems::Center);
        root_style.row_gap = crate::style::CssLengthPercentage::Length(1.0);
        let root = arena.create_element(root_style);

        let title = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let mut textarea_style = block_style(CssDimension::Length(48.0), CssDimension::Auto);
        textarea_style.min_height = CssDimension::Length(5.0);
        textarea_style.border_top = BorderStyle::Rounded;
        textarea_style.border_right = BorderStyle::Rounded;
        textarea_style.border_bottom = BorderStyle::Rounded;
        textarea_style.border_left = BorderStyle::Rounded;
        let textarea = arena.create_textarea(textarea_style, "short");
        let submitted = arena.create_element(block_style(
            CssDimension::Length(48.0),
            CssDimension::Length(3.0),
        ));
        arena.append_child(root, title);
        arena.append_child(root, textarea);
        arena.append_child(root, submitted);

        arena.set_textarea_value(textarea, "word ".repeat(120), 0);
        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(80.0),
                height: AvailableSpace::Definite(24.0),
            },
        );

        assert!(arena.layout(textarea).size.height > 5.0);
    }

    #[test]
    fn inline_textarea_contributes_wrapped_height_to_line_size() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let mut textarea_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
        textarea_style.display = LayoutDisplay::Inline;
        let textarea = arena.create_textarea(textarea_style, "hello world");
        arena.append_child(row, textarea);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        let layout = arena.layout(row);
        assert_eq!(layout.size.width, 5.0);
        assert_eq!(layout.size.height, 3.0);
    }

    #[test]
    fn fixed_scroll_container_reports_metrics_and_clamps_offset() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Length(3.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let child = fixed_box(&mut arena, 5.0, 10.0);
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(3.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_height, 3);
        assert_eq!(metrics.scroll_height, 10);

        let metrics = arena.set_scroll_offset(viewport, 0, 100).unwrap();
        assert_eq!(metrics.scroll_top, 7);
    }

    #[test]
    fn scroll_metrics_include_visible_overflow_from_descendants() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(10.0), CssDimension::Length(4.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let root = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let content = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(10.0),
        ));
        arena.append_child(root, content);
        arena.append_child(viewport, root);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(4.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_height, 4);
        assert_eq!(metrics.scroll_height, 10);
    }

    #[test]
    fn resize_recomputes_visible_overflow_for_percent_descendants() {
        let mut arena = LayoutArena::new();
        let mut viewport_style =
            block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let wrapper = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(1.0),
        ));
        let content = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Percent(2.0),
        ));
        arena.append_child(wrapper, content);
        arena.append_child(viewport, wrapper);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(4.0),
            },
        );
        assert_eq!(arena.scroll_metrics(viewport).unwrap().scroll_height, 8);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(8.0),
            },
        );
        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_height, 8);
        assert_eq!(metrics.scroll_height, 16);
    }

    #[test]
    fn scroll_offset_clamps_when_viewport_grows_after_resize() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Length(3.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style.clone());
        let child = fixed_box(&mut arena, 5.0, 10.0);
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        arena.set_scroll_offset(viewport, 0, 100).unwrap();
        assert_eq!(arena.scroll_offset(viewport).1, 7);

        viewport_style.height = CssDimension::Length(8.0);
        arena.set_style(viewport, viewport_style);
        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(8.0),
            },
        );
        assert_eq!(arena.scroll_metrics(viewport).unwrap().scroll_top, 2);
        assert_eq!(arena.scroll_offset(viewport).1, 7);

        arena.clamp_scroll_offsets();
        assert_eq!(arena.scroll_offset(viewport).1, 2);
    }

    #[test]
    fn percent_scroll_container_reports_resolved_client_size() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(20.0),
            CssDimension::Length(10.0),
        ));
        let mut viewport_style =
            block_style(CssDimension::Percent(0.5), CssDimension::Percent(0.5));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let child = fixed_box(&mut arena, 10.0, 8.0);
        arena.append_child(viewport, child);
        arena.append_child(root, viewport);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(10.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_width, 9);
        assert_eq!(metrics.client_height, 5);
        assert_eq!(metrics.scroll_height, 8);
    }

    #[test]
    fn bordered_padded_scroll_container_uses_padding_box_metrics() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(10.0), CssDimension::Length(6.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.border_top = BorderStyle::Solid;
        viewport_style.border_right = BorderStyle::Solid;
        viewport_style.border_bottom = BorderStyle::Solid;
        viewport_style.border_left = BorderStyle::Solid;
        viewport_style.padding_top = CssLengthPercentage::Length(1.0);
        viewport_style.padding_right = CssLengthPercentage::Length(1.0);
        viewport_style.padding_bottom = CssLengthPercentage::Length(1.0);
        viewport_style.padding_left = CssLengthPercentage::Length(1.0);
        let viewport = arena.create_element(viewport_style);
        let child = fixed_box(&mut arena, 6.0, 8.0);
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(6.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_width, 7);
        assert_eq!(metrics.client_height, 4);
        assert_eq!(metrics.scroll_width, 8);
        assert_eq!(metrics.scroll_height, 10);
    }

    #[test]
    fn image_scroll_demo_layout_constrains_viewport_and_reports_scroll_metrics() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let mut header_style = block_style(CssDimension::Percent(1.0), CssDimension::Length(2.0));
        header_style.flex_shrink = 0.0;
        let header = arena.create_element(header_style);
        let header_text =
            arena.create_text("Image scroll demo. Wheel over the panel. Ctrl-C exits.");
        arena.append_child(header, header_text);

        let mut body_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        body_style.display = LayoutDisplay::Flex;
        body_style.flex_direction = LayoutFlexDirection::Row;
        body_style.flex_grow = 1.0;
        body_style.flex_shrink = 1.0;
        body_style.flex_basis = CssDimension::Length(0.0);
        body_style.min_height = CssDimension::Length(0.0);
        body_style.column_gap = CssLengthPercentage::Length(1.0);
        let body = arena.create_element(body_style);

        let mut viewport_style =
            block_style(CssDimension::Percent(0.8), CssDimension::Percent(1.0));
        viewport_style.min_height = CssDimension::Length(0.0);
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.overflow_x = LayoutOverflow::Hidden;
        viewport_style.border_top = BorderStyle::Rounded;
        viewport_style.border_right = BorderStyle::Rounded;
        viewport_style.border_bottom = BorderStyle::Rounded;
        viewport_style.border_left = BorderStyle::Rounded;
        let viewport = arena.create_element(viewport_style);

        let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        content_style.display = LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        content_style.row_gap = CssLengthPercentage::Length(1.0);
        let content = arena.create_element(content_style);

        for index in 1..=8 {
            let row =
                arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
            let text = arena.create_text(format!("before image row {index}"));
            arena.append_child(row, text);
            arena.append_child(content, row);
        }

        let first_image = image_scroll_demo_image_block(
            &mut arena,
            "image A should clip against the scroll viewport",
        );
        arena.append_child(content, first_image);

        for index in 1..=14 {
            let row =
                arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
            let text = arena.create_text(format!("middle row {index}"));
            arena.append_child(row, text);
            arena.append_child(content, row);
        }

        let second_image =
            image_scroll_demo_image_block(&mut arena, "image B should scroll out like normal text");
        arena.append_child(content, second_image);

        for index in 1..=18 {
            let row =
                arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
            let text = arena.create_text(format!("after image row {index}"));
            arena.append_child(row, text);
            arena.append_child(content, row);
        }

        arena.append_child(viewport, content);

        let rail = arena.create_element(block_style(
            CssDimension::Percent(0.2),
            CssDimension::Percent(1.0),
        ));
        let scrollbar = arena.create_text("#");
        arena.append_child(rail, scrollbar);

        arena.append_child(body, viewport);
        arena.append_child(body, rail);
        arena.append_child(root, header);
        arena.append_child(root, body);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(80.0),
                height: AvailableSpace::Definite(24.0),
            },
        );

        let viewport_metrics = arena.scroll_metrics(viewport).unwrap();
        let rail_metrics = arena.scroll_metrics(rail).unwrap();

        assert_eq!(arena.layout(header).size.height, 2.0);
        assert_eq!(arena.layout(body).size.height, 22.0);
        assert_eq!(arena.layout(viewport).size.height, 22.0);
        assert_eq!(viewport_metrics.client_height, 20);
        assert_eq!(viewport_metrics.scroll_height, 113);
        assert_eq!(rail_metrics.client_height, 22);
    }

    #[test]
    fn ten_thousand_row_scroll_container_reports_content_height() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(20.0), CssDimension::Length(5.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        content_style.display = LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = arena.create_element(content_style);
        for _ in 0..10_000 {
            let row = fixed_box(&mut arena, 20.0, 1.0);
            arena.append_child(content, row);
        }
        arena.append_child(viewport, content);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(20.0),
                height: AvailableSpace::Definite(5.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_height, 5);
        assert_eq!(metrics.scroll_height, 10_000);
        let profile = arena.profile_stats();
        assert_eq!(profile.absolute_layout_visits, 0);
        assert_eq!(profile.stacking_tree_visits, 0);
    }

    #[test]
    fn isolated_text_change_does_not_walk_clean_ten_thousand_row_branch() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(80.0), CssDimension::Length(24.0));
        root_style.display = LayoutDisplay::Flex;
        root_style.flex_direction = LayoutFlexDirection::Column;
        let root = arena.create_element(root_style);

        let header = arena.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(1.0),
        ));
        let status = arena.create_text("scrollTop=0");
        arena.append_child(header, status);
        arena.append_child(root, header);

        let mut viewport_style =
            block_style(CssDimension::Percent(1.0), CssDimension::Length(23.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
        content_style.display = LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = arena.create_element(content_style);
        for index in 0..10_000 {
            let row =
                arena.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
            let text = arena.create_text(format!(
                "percent row {index:05} - resize changes visible content"
            ));
            arena.append_child(row, text);
            arena.append_child(content, row);
        }
        arena.append_child(viewport, content);
        arena.append_child(root, viewport);

        let available = Size {
            width: AvailableSpace::Definite(80.0),
            height: AvailableSpace::Definite(24.0),
        };
        let initial_layout_start = Instant::now();
        arena.compute_layout(root, available);
        if std::env::var_os("PAINTCANNON_PROFILE").is_some() {
            let profile = arena.profile_stats();
            let milliseconds = |nanoseconds: u128| nanoseconds as f64 / 1_000_000.0;
            let extra_width_entries = arena
                .nodes
                .iter()
                .map(|node| node.measure_cache.extra_widths.len())
                .sum::<usize>();
            let extra_height_entries = arena
                .nodes
                .iter()
                .map(|node| node.measure_cache.extra_heights.len())
                .sum::<usize>();
            eprintln!(
                "[paintcannon-profile] event=ten_thousand_row_initial_layout duration_ms={:.3} taffy_ms={:.3} dirty_descendants_ms={:.3} visible_overflow_ms={:.3} inline_width_ms={:.3} inline_height_ms={:.3} inline_fragment_ms={:.3} extra_width_entries={} extra_height_entries={}",
                initial_layout_start.elapsed().as_secs_f64() * 1_000.0,
                milliseconds(profile.taffy_ns),
                milliseconds(profile.dirty_descendants_ns),
                milliseconds(profile.visible_overflow_ns),
                milliseconds(profile.inline_width_ns),
                milliseconds(profile.inline_height_ns),
                milliseconds(profile.inline_fragment_ns),
                extra_width_entries,
                extra_height_entries,
            );
        }
        arena.set_text(status, "scrollTop=3");
        arena.compute_layout(root, available);
        arena.clamp_scroll_offsets();
        arena.ensure_dirty_textareas_visible();

        let profile = arena.profile_stats();
        assert!(
            profile.dirty_descendant_visits < 100 && profile.visible_overflow_visits < 100,
            "visited {} nodes while finding dirty descendants and {} while updating visible overflow",
            profile.dirty_descendant_visits,
            profile.visible_overflow_visits,
        );
        assert!(
            profile.scroll_clamp_visits < 100 && profile.dirty_textarea_visits < 100,
            "visited {} nodes while clamping scroll offsets and {} while finding dirty textareas",
            profile.scroll_clamp_visits,
            profile.dirty_textarea_visits,
        );
    }

    #[test]
    fn setting_scroll_offset_does_not_recompute_layout() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Length(3.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let child = fixed_box(&mut arena, 5.0, 10.0);
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        let passes = arena.layout_passes();
        arena.set_scroll_offset(viewport, 0, 2).unwrap();

        assert_eq!(arena.layout_passes(), passes);
    }

    #[test]
    fn inline_fragments_record_wrapped_text_positions() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
        let text = arena.create_text("hello world");
        arena.append_child(row, text);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::MaxContent,
            },
        );

        let fragments = arena.inline_fragments(row);
        assert_eq!(fragments[0].x, 0);
        assert_eq!(fragments[0].y, 0);
        assert_eq!(fragments[5].x, 0);
        assert_eq!(fragments[5].y, 1);
        assert!(matches!(
            fragments[0].kind,
            InlineFragmentKind::Text {
                character: 'h',
                selection_order: 0
            }
        ));
    }

    #[test]
    fn inline_fragments_preserve_span_targets_across_wrapping() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Length(4.0), CssDimension::Auto));
        let mut span_style = DivStyle::default();
        span_style.display = LayoutDisplay::Inline;
        let span = arena.create_element(span_style);
        let text = arena.create_text("ab cd");
        arena.append_child(span, text);
        arena.append_child(row, span);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(4.0),
                height: AvailableSpace::MaxContent,
            },
        );

        let fragments = arena.inline_fragments(row);
        assert!(fragments.iter().all(|fragment| fragment.node == text));
        assert!(fragments
            .iter()
            .all(|fragment| fragment.hit_node == Some(span)));
        assert!(fragments.iter().any(|fragment| fragment.y == 1));
    }

    #[test]
    fn inline_fragments_keep_selection_order_across_spans() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Length(3.0), CssDimension::Auto));
        let mut span_style = DivStyle::default();
        span_style.display = LayoutDisplay::Inline;
        let first_span = arena.create_element(span_style.clone());
        let second_span = arena.create_element(span_style);
        let first_text = arena.create_text("ab");
        let second_text = arena.create_text("cd");
        arena.append_child(first_span, first_text);
        arena.append_child(second_span, second_text);
        arena.append_child(row, first_span);
        arena.append_child(row, second_span);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(3.0),
                height: AvailableSpace::MaxContent,
            },
        );

        let fragments = arena.inline_fragments(row);
        let chars = fragments
            .iter()
            .filter_map(|fragment| match fragment.kind {
                InlineFragmentKind::Text { character, .. } => Some(character),
                InlineFragmentKind::Replaced => None,
            })
            .collect::<String>();
        let orders = fragments
            .iter()
            .filter_map(|fragment| match fragment.kind {
                InlineFragmentKind::Text {
                    selection_order, ..
                } => Some(selection_order),
                InlineFragmentKind::Replaced => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(chars, "abcd");
        assert_eq!(orders, vec![0, 1, 2, 3]);
        assert_eq!(fragments[0].hit_node, Some(first_span));
        assert_eq!(fragments[2].hit_node, Some(second_span));
        assert!(fragments.iter().any(|fragment| fragment.y == 1));
    }

    #[test]
    fn inline_fragments_respect_whitespace_inheritance() {
        let mut arena = LayoutArena::new();
        let mut row_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
        row_style.white_space = CssWhiteSpace::Pre;
        let row = arena.create_element(row_style);
        let text = arena.create_text("a  b");
        arena.append_child(row, text);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::MaxContent,
            },
        );

        let chars = arena
            .inline_fragments(row)
            .iter()
            .filter_map(|fragment| match fragment.kind {
                InlineFragmentKind::Text { character, .. } => Some(character),
                InlineFragmentKind::Replaced => None,
            })
            .collect::<String>();
        assert_eq!(chars, "a  b");
    }

    #[test]
    fn inline_fragments_record_replaced_nodes_for_hit_testing() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Auto, CssDimension::Auto));
        let mut image_style = DivStyle::default();
        image_style.display = LayoutDisplay::Inline;
        let image = arena.create_image(image_style, 16, 32, 8, 16);
        arena.append_child(row, image);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
        );

        assert_eq!(
            arena.inline_fragments(row),
            &[InlineFragment {
                node: image,
                hit_node: Some(image),
                kind: InlineFragmentKind::Replaced,
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            }]
        );
    }
}
