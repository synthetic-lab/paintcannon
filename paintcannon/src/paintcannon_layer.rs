use taffy::{AvailableSpace, NodeId, Size};

use crate::frame::{ClipRect, Frame};
use crate::layout::LayoutArena;
use crate::paint::{paint_arena_into, PaintOptions, PaintOutput};
use crate::style::{
    Background, BorderStyle, CssDimension, CssLengthPercentageAuto, CssOverflowWrap, CssPosition,
    DivStyle, LayoutOverflow,
};

const LINK_HINT: &str = "Ctrl-Shift-Click to open";

/// Renderer-owned UI has a separate layout tree and is painted after the app's
/// entire stacking tree. App z-index, clipping, selection and transitions cannot
/// affect it. Its nodes never enter the public DOM or app hit testing.
pub(crate) struct PaintCannonLayer {
    arena: LayoutArena,
    root: NodeId,
    link_tooltip: NodeId,
    hovered_link: Option<ClipRect>,
    geometry: Option<(ClipRect, usize, usize)>,
    visible: bool,
}

impl PaintCannonLayer {
    pub(crate) fn new() -> Self {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(DivStyle {
            overflow_x: LayoutOverflow::Hidden,
            overflow_y: LayoutOverflow::Hidden,
            ..Default::default()
        });
        let link_tooltip = arena.create_element(DivStyle {
            position: CssPosition::Absolute,
            border_top: BorderStyle::Pill,
            border_right: BorderStyle::Pill,
            border_bottom: BorderStyle::Pill,
            border_left: BorderStyle::Pill,
            background: Background::Rgb(224, 224, 224),
            border_color: Background::Rgb(224, 224, 224),
            color: Background::Black,
            overflow_wrap: CssOverflowWrap::Anywhere,
            overflow_x: LayoutOverflow::Hidden,
            overflow_y: LayoutOverflow::Hidden,
            ..Default::default()
        });
        let text = arena.create_text(LINK_HINT);
        arena.append_child(link_tooltip, text);
        arena.append_child(root, link_tooltip);
        Self {
            arena,
            root,
            link_tooltip,
            hovered_link: None,
            geometry: None,
            visible: false,
        }
    }

    pub(crate) fn set_hovered_link(&mut self, link: Option<ClipRect>) -> bool {
        if self.hovered_link == link {
            return false;
        }
        self.hovered_link = link;
        true
    }

    pub(crate) fn prepare_layout(&mut self, width: usize, height: usize) {
        let geometry = self.hovered_link.map(|link| (link, width, height));
        if self.geometry == geometry {
            return;
        }
        self.geometry = geometry;
        self.visible = false;
        let Some((link, width, height)) = geometry else {
            return;
        };
        if width < 3 || height < 2 {
            return;
        }

        let available = Size {
            width: AvailableSpace::Definite(width as f32),
            height: AvailableSpace::Definite(height as f32),
        };
        let mut root_style = self.arena.style(self.root).clone();
        root_style.width = CssDimension::Length(width as f32);
        root_style.height = CssDimension::Length(height as f32);
        self.arena.set_style(self.root, root_style);
        let tooltip_width = (LINK_HINT.len() + 2).min(width);
        let mut style = self.arena.style(self.link_tooltip).clone();
        style.width = CssDimension::Length(tooltip_width as f32);
        style.left = CssLengthPercentageAuto::Length(
            link.left.max(0).min((width - tooltip_width) as i32) as f32,
        );
        style.top = CssLengthPercentageAuto::Length(0.0);
        style.max_height = CssDimension::Auto;
        self.arena.set_style(self.link_tooltip, style.clone());
        self.arena.compute_layout(self.root, available);
        let tooltip_height = self.arena.layout(self.link_tooltip).size.height as usize;
        let above = (link.top.max(0) as usize).min(height);
        let below = height.saturating_sub(link.bottom.max(0) as usize);
        let (top, room) = if tooltip_height <= above || above >= below {
            (above.saturating_sub(tooltip_height), above)
        } else {
            (height - below, below)
        };
        if room == 0 {
            return;
        }
        // Prefer above, but never cover the link itself when the viewport is small.
        style.top = CssLengthPercentageAuto::Length(top as f32);
        style.max_height = CssDimension::Length(room as f32);
        self.arena.set_style(self.link_tooltip, style);
        self.arena.compute_layout(self.root, available);
        self.arena.prepare_paint(self.root);
        self.visible = true;
    }

    pub(crate) fn paint(&self, frame: Frame, options: PaintOptions<'_>) -> Frame {
        if !self.visible {
            return frame;
        }
        let mut output = PaintOutput {
            frame,
            hit_regions: Vec::new(),
        };
        paint_arena_into(
            &self.arena,
            self.root,
            &mut output,
            false,
            PaintOptions {
                transitions: None,
                ..options
            },
        );
        output.frame
    }
}
