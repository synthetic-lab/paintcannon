#![allow(dead_code)]

use std::time::Instant;

use taffy::prelude::TaffyMaxContent;
use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_root_layout, AvailableSpace, Cache,
    CacheTree, CoreStyle, Layout, LayoutInput, LayoutOutput, LayoutPartialTree, MaybeMath,
    MaybeResolve, NodeId, Point, ResolveOrZero, RoundTree, RunMode, Size, SizingMode, Style,
    TraversePartialTree, TraverseTree,
};
use unicode_width::UnicodeWidthChar;

use crate::style::{CssDimension, CssWhiteSpace, DivStyle, LayoutDisplay, LayoutOverflow};

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
    pub(crate) cursor: u32,
    pub(crate) focused: bool,
}

#[derive(Clone)]
pub(crate) struct TextAreaLayoutData {
    pub(crate) value: String,
    pub(crate) cursor: u32,
    pub(crate) focused: bool,
}

pub(crate) struct LayoutArena {
    nodes: Vec<LayoutNode>,
    layout_passes: u64,
    layout_mode_stack: Vec<RunMode>,
    profile: LayoutProfileStats,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LayoutProfileStats {
    pub(crate) inline_width_calls: u64,
    pub(crate) inline_height_calls: u64,
    pub(crate) inline_fragment_calls: u64,
    pub(crate) inline_width_ns: u128,
    pub(crate) inline_height_ns: u128,
    pub(crate) inline_fragment_ns: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutStats {
    pub(crate) node_count: usize,
    pub(crate) inline_context_count: usize,
    pub(crate) inline_fragment_count: usize,
}

struct LayoutNode {
    kind: LayoutNodeKind,
    style: DivStyle,
    taffy_style: Style,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
    layout: Layout,
    cache: Cache,
    scroll_left: u32,
    scroll_top: u32,
    fragments: Vec<InlineFragment>,
    measure_cache: InlineMeasureCache,
}

#[derive(Clone, Copy)]
struct ContentWidths {
    min: f32,
    max: f32,
}

#[derive(Default)]
struct InlineMeasureCache {
    widths: Vec<InlineWidthCacheEntry>,
    heights: Vec<InlineHeightCacheEntry>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArenaScrollMetrics {
    pub(crate) scroll_left: u32,
    pub(crate) scroll_top: u32,
    pub(crate) scroll_width: u32,
    pub(crate) scroll_height: u32,
    pub(crate) client_width: u32,
    pub(crate) client_height: u32,
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
            layout_passes: 0,
            layout_mode_stack: Vec::new(),
            profile: LayoutProfileStats::default(),
        }
    }

    pub(crate) fn create_element(&mut self, style: DivStyle) -> NodeId {
        self.push_node(LayoutNodeKind::Element, style)
    }

    pub(crate) fn reserve_nodes(&mut self, additional: usize) {
        self.nodes.reserve(additional);
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
                cursor: 0,
                focused: false,
            }),
            style,
        )
    }

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
            self.clear_cache_from(node);
        }
    }

    pub(crate) fn set_textarea_focused(&mut self, node: NodeId, focused: bool) {
        if let LayoutNodeKind::TextArea(textarea) = &mut self.nodes[node_index(node)].kind {
            textarea.focused = focused;
        }
    }

    pub(crate) fn append_child(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[node_index(parent)].children.push(child);
        self.nodes[node_index(child)].parent = Some(parent);
        self.clear_cache_from(parent);
    }

    pub(crate) fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        let children = &mut self.nodes[node_index(parent)].children;
        if let Some(index) = children.iter().position(|id| *id == child) {
            children.remove(index);
            self.nodes[node_index(child)].parent = None;
            self.clear_cache_from(parent);
        }
    }

    pub(crate) fn set_style(&mut self, node: NodeId, style: DivStyle) {
        let item = &mut self.nodes[node_index(node)];
        item.taffy_style = style.to_taffy();
        item.style = style;
        self.clear_cache_subtree_and_ancestors(node);
    }

    pub(crate) fn compute_layout(&mut self, root: NodeId, available: Size<AvailableSpace>) {
        self.layout_passes += 1;
        self.profile = LayoutProfileStats::default();
        self.clear_measure_caches();
        compute_root_layout(self, root, available);
    }

    pub(crate) fn layout(&self, node: NodeId) -> Layout {
        self.nodes[node_index(node)].layout
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

    pub(crate) fn scroll_offset(&self, node: NodeId) -> (u32, u32) {
        let node = &self.nodes[node_index(node)];
        (node.scroll_left, node.scroll_top)
    }

    pub(crate) fn layout_passes(&self) -> u64 {
        self.layout_passes
    }

    pub(crate) fn stats(&self) -> LayoutStats {
        LayoutStats {
            node_count: self.nodes.len(),
            inline_context_count: self
                .nodes
                .iter()
                .enumerate()
                .filter(|(index, _)| self.is_inline_context(NodeId::from(*index)))
                .count(),
            inline_fragment_count: self.nodes.iter().map(|node| node.fragments.len()).sum(),
        }
    }

    pub(crate) fn profile_stats(&self) -> LayoutProfileStats {
        self.profile
    }

    fn push_node(&mut self, kind: LayoutNodeKind, style: DivStyle) -> NodeId {
        let id = NodeId::from(self.nodes.len());
        self.nodes.push(LayoutNode {
            kind,
            taffy_style: style.to_taffy(),
            style,
            children: Vec::new(),
            parent: None,
            layout: Layout::new(),
            cache: Cache::new(),
            scroll_left: 0,
            scroll_top: 0,
            fragments: Vec::new(),
            measure_cache: InlineMeasureCache::default(),
        });
        id
    }

    pub(crate) fn inline_fragments(&self, node: NodeId) -> &[InlineFragment] {
        &self.nodes[node_index(node)].fragments
    }

    pub(crate) fn scroll_metrics(&mut self, node: NodeId) -> Option<ArenaScrollMetrics> {
        self.scroll_metrics_for_node(node)
    }

    pub(crate) fn set_scroll_offset(
        &mut self,
        node: NodeId,
        scroll_left: u32,
        scroll_top: u32,
    ) -> Option<ArenaScrollMetrics> {
        let metrics = self.scroll_metrics_for_node(node)?;
        let max_left = axis_max_scroll(
            self.nodes[node_index(node)].style.overflow_x,
            metrics.scroll_width.saturating_sub(metrics.client_width),
        );
        let max_top = axis_max_scroll(
            self.nodes[node_index(node)].style.overflow_y,
            metrics.scroll_height.saturating_sub(metrics.client_height),
        );
        let item = &mut self.nodes[node_index(node)];
        item.scroll_left = scroll_left.min(max_left);
        item.scroll_top = scroll_top.min(max_top);
        self.scroll_metrics_for_node(node)
    }

    fn scroll_metrics_for_node(&mut self, node_id: NodeId) -> Option<ArenaScrollMetrics> {
        let index = node_index(node_id);
        if !matches!(self.nodes[index].kind, LayoutNodeKind::Element) {
            return None;
        }

        let layout = self.nodes[index].layout;
        let white_space = self.nodes[index].style.white_space;
        let overflow_x = self.nodes[index].style.overflow_x;
        let overflow_y = self.nodes[index].style.overflow_y;
        let scroll_left = self.nodes[index].scroll_left;
        let scroll_top = self.nodes[index].scroll_top;
        let content_size = layout.content_box_size();
        let content_origin = Point {
            x: layout.border.left + layout.padding.left,
            y: layout.border.top + layout.padding.top,
        };
        let client_width = float_to_cells(content_size.width);
        let client_height = float_to_cells(content_size.height);
        let mut scroll_width = client_width;
        let mut scroll_height = client_height;

        if self.is_inline_context(node_id) {
            let widths = self.inline_content_widths(node_id, white_space);
            let height = self.inline_content_height(node_id, white_space, client_width.max(1));
            scroll_width = scroll_width.max(float_to_cells(widths.max));
            scroll_height = scroll_height.max(float_to_cells(height));
        } else {
            let child_count = self.nodes[index].children.len();
            for child_index in 0..child_count {
                let child = self.nodes[index].children[child_index];
                let child_layout = self.nodes[node_index(child)].layout;
                scroll_width = scroll_width.max(float_to_cells(
                    child_layout.location.x + child_layout.size.width - content_origin.x,
                ));
                scroll_height = scroll_height.max(float_to_cells(
                    child_layout.location.y + child_layout.size.height - content_origin.y,
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

    fn clear_cache(&mut self) {
        for node in &mut self.nodes {
            node.cache.clear();
        }
    }

    fn clear_cache_from(&mut self, node: NodeId) {
        let mut current = Some(node);
        while let Some(node_id) = current {
            let item = &mut self.nodes[node_index(node_id)];
            item.cache.clear();
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
        let child_count = self.nodes[index].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            self.clear_cache_subtree(child);
        }
    }

    fn clear_measure_caches(&mut self) {
        for node in &mut self.nodes {
            node.measure_cache.widths.clear();
            node.measure_cache.heights.clear();
        }
    }

    fn should_store_layout(&self) -> bool {
        !self
            .layout_mode_stack
            .iter()
            .any(|mode| *mode == RunMode::ComputeSize)
    }

    fn is_inline_context(&self, node: NodeId) -> bool {
        let node = &self.nodes[node_index(node)];
        matches!(node.kind, LayoutNodeKind::Element)
            && node.style.display == LayoutDisplay::Block
            && self.has_only_inline_children(&node.children)
    }

    fn has_only_inline_children(&self, children: &[NodeId]) -> bool {
        let mut has_inline = false;
        for child in children {
            let node = &self.nodes[node_index(*child)];
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

        let style = self.nodes[node_index(node_id)].taffy_style.clone();
        let paint_style = self.nodes[node_index(node_id)].style.clone();

        let margin = style
            .margin()
            .resolve_or_zero(parent_size.width, |_, _| 0.0);
        let padding = style
            .padding()
            .resolve_or_zero(parent_size.width, |_, _| 0.0);
        let border = style
            .border()
            .resolve_or_zero(parent_size.width, |_, _| 0.0);
        let padding_border = padding + border;
        let padding_border_size = padding_border.sum_axes();
        let box_sizing_adjustment = if style.box_sizing() == taffy::BoxSizing::ContentBox {
            padding_border_size
        } else {
            Size::ZERO
        };

        let (node_size, node_min_size, node_max_size, aspect_ratio) = match sizing_mode {
            SizingMode::ContentSize => (known_dimensions, Size::NONE, Size::NONE, None),
            SizingMode::InherentSize => {
                let aspect_ratio = style.aspect_ratio();
                let style_size = style
                    .size()
                    .maybe_resolve(parent_size, |_, _| 0.0)
                    .maybe_apply_aspect_ratio(aspect_ratio)
                    .maybe_add(box_sizing_adjustment);
                let style_min_size = style
                    .min_size()
                    .maybe_resolve(parent_size, |_, _| 0.0)
                    .maybe_apply_aspect_ratio(aspect_ratio)
                    .maybe_add(box_sizing_adjustment);
                let style_max_size = style
                    .max_size()
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

        let content_box_inset = padding_border;
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
        let content_widths = self.inline_content_widths(node_id, paint_style.white_space);
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
        let content_height = self.inline_content_height(
            node_id,
            paint_style.white_space,
            content_width.max(1.0).round() as u32,
        );
        self.profile.inline_height_calls += 1;
        self.profile.inline_height_ns += inline_height_start.elapsed().as_nanos();
        if run_mode == RunMode::PerformLayout && self.should_store_layout() {
            let inline_fragment_start = Instant::now();
            self.compute_inline_fragments(
                node_id,
                paint_style.white_space,
                content_width.max(1.0).round() as u32,
            );
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
        if let Some(entry) = self.nodes[index]
            .measure_cache
            .widths
            .iter()
            .find(|entry| entry.white_space == white_space)
        {
            return entry.widths;
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
            .widths
            .push(InlineWidthCacheEntry {
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
                let size = self.replaced_node_size(node_id, Size::NONE, Size::MAX_CONTENT);
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
        if let Some(entry) = self.nodes[index]
            .measure_cache
            .heights
            .iter()
            .find(|entry| entry.white_space == white_space && entry.width == width)
        {
            return entry.height;
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
            .heights
            .push(InlineHeightCacheEntry {
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
                let size = self.replaced_node_size(node, Size::NONE, Size::MAX_CONTENT);
                measure_inline_replaced(size, cursor);
            }
            LayoutNodeKind::Element
            | LayoutNodeKind::Image(_)
            | LayoutNodeKind::Input(_)
            | LayoutNodeKind::TextArea(_) => {}
        }
    }

    fn compute_inline_fragments(&mut self, node: NodeId, white_space: CssWhiteSpace, width: u32) {
        let mut cursor = InlineLayoutCursor {
            col: 0,
            row: 0,
            width: width.max(1),
            max_col: 0,
            selection_order: 0,
            fragments: Vec::new(),
        };
        let index = node_index(node);
        let child_count = self.nodes[index].children.len();
        for child_index in 0..child_count {
            let child = self.nodes[index].children[child_index];
            self.layout_inline_node(child, white_space, None, &mut cursor);
        }
        self.nodes[index].fragments = cursor.fragments;
    }

    fn layout_inline_node(
        &self,
        node: NodeId,
        inherited_white_space: CssWhiteSpace,
        hit_target: Option<NodeId>,
        cursor: &mut InlineLayoutCursor,
    ) {
        let item = &self.nodes[node_index(node)];
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
                let size = self.replaced_node_size(node, Size::NONE, Size::MAX_CONTENT);
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
                let wrap_width = explicit_width_cells(node.style.width)
                    .or_else(|| available_space.width.into_option().map(float_to_cells));
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
            self.nodes[node_index(node_id)].layout = *layout;
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
    fragments: Vec<InlineFragment>,
}

fn node_index(node: NodeId) -> usize {
    node.into()
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
        width: input.value.chars().count().max(1) as f32,
        height: 1.0,
    }
}

fn textarea_natural_size(textarea: &TextAreaLayoutData, wrap_width: Option<u32>) -> Size<f32> {
    let mut cursor = InlineMeasureCursor {
        col: 0,
        row: 0,
        width: wrap_width.unwrap_or(u32::MAX).max(1),
        max_col: 0,
    };
    measure_inline_text(&textarea.value, CssWhiteSpace::PreWrap, &mut cursor);
    Size {
        width: cursor.max_col.max(cursor.col).max(1) as f32,
        height: (cursor.row + 1).max(1) as f32,
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

fn float_to_cells(value: f32) -> u32 {
    value.max(0.0).round() as u32
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
    let chars = normalize_text_for_white_space(text, white_space);
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
    let chars = normalize_text_for_white_space(text, white_space);
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
            if cursor.col > 0 && cursor.col + word_width > cursor.width {
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
            if character == ' ' || character == '\t' {
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
    let chars = normalize_text_for_white_space(text, white_space);
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
            if cursor.col > 0 && cursor.col + word_width > cursor.width {
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
            if character == ' ' || character == '\t' {
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

fn normalize_text_for_white_space(text: &str, white_space: CssWhiteSpace) -> Vec<char> {
    match white_space {
        CssWhiteSpace::Pre | CssWhiteSpace::PreWrap => text.chars().collect(),
        CssWhiteSpace::Normal | CssWhiteSpace::NoWrap => collapse_whitespace(text, false),
        CssWhiteSpace::PreLine => collapse_whitespace(text, true),
    }
}

fn collapse_whitespace(text: &str, preserve_newlines: bool) -> Vec<char> {
    let mut chars = Vec::new();
    let mut pending_space = false;
    for character in text.chars() {
        if character == '\r' {
            continue;
        }
        if preserve_newlines && character == '\n' {
            chars.push('\n');
            pending_space = false;
            continue;
        }
        if matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}') {
            if !pending_space {
                chars.push(' ');
                pending_space = true;
            }
            continue;
        }
        chars.push(character);
        pending_space = false;
    }
    chars
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

fn character_cell_width(character: char) -> usize {
    if character == '\t' {
        return 4;
    }
    UnicodeWidthChar::width(character).unwrap_or(0)
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
        BorderStyle, CssDimension, CssGridTemplateTrack, CssTrackSizing, LayoutFlexDirection,
        LayoutOverflow,
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
        assert_eq!(layout.size.height, 2.0);
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
        assert_eq!(layout.size.height, 2.0);
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
        assert_eq!(metrics.client_width, 10);
        assert_eq!(metrics.client_height, 5);
        assert_eq!(metrics.scroll_height, 8);
    }

    #[test]
    fn bordered_scroll_container_uses_content_box_metrics() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(10.0), CssDimension::Length(6.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.border_top = BorderStyle::Solid;
        viewport_style.border_right = BorderStyle::Solid;
        viewport_style.border_bottom = BorderStyle::Solid;
        viewport_style.border_left = BorderStyle::Solid;
        let viewport = arena.create_element(viewport_style);
        let child = fixed_box(&mut arena, 8.0, 8.0);
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(10.0),
                height: AvailableSpace::Definite(6.0),
            },
        );

        let metrics = arena.scroll_metrics(viewport).unwrap();
        assert_eq!(metrics.client_width, 8);
        assert_eq!(metrics.client_height, 4);
        assert_eq!(metrics.scroll_width, 8);
        assert_eq!(metrics.scroll_height, 8);
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
