#![allow(dead_code)]

use std::time::Instant;

use taffy::NodeId;
use unicode_width::UnicodeWidthChar;

use crate::frame::{ClipBounds, ClipRect, Frame, GlyphStyle};
use crate::layout::{
    ImageLayoutData, InlineFragmentKind, InputLayoutData, LayoutArena, LayoutNodeKind,
    TextAreaLayoutData,
};
use crate::style::{Background, ColorTransitionProperty, DivStyle, ImageRendering, LayoutOverflow};
use crate::transition::TransitionState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HitRegion {
    pub(crate) id: NodeId,
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

pub(crate) struct PaintOutput {
    pub(crate) frame: Frame,
    pub(crate) hit_regions: Vec<HitRegion>,
}

#[derive(Clone, Copy)]
pub(crate) struct PaintOptions<'a> {
    pub(crate) transitions: Option<&'a TransitionState>,
    pub(crate) now: Instant,
    pub(crate) truecolor_enabled: bool,
}

impl PaintOutput {
    pub(crate) fn target_at(&self, x: u32, y: u32) -> Option<NodeId> {
        let x = x.min(i32::MAX as u32) as i32;
        let y = y.min(i32::MAX as u32) as i32;
        self.hit_regions
            .iter()
            .rev()
            .find(|region| region.contains(x, y))
            .map(|region| region.id)
    }
}

impl HitRegion {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

pub(crate) fn paint_arena(
    arena: &LayoutArena,
    root: NodeId,
    width: usize,
    height: usize,
    capture_hidden_selection_units: bool,
) -> PaintOutput {
    paint_arena_with_options(
        arena,
        root,
        width,
        height,
        capture_hidden_selection_units,
        PaintOptions {
            transitions: None,
            now: Instant::now(),
            truecolor_enabled: false,
        },
    )
}

pub(crate) fn paint_arena_with_options(
    arena: &LayoutArena,
    root: NodeId,
    width: usize,
    height: usize,
    capture_hidden_selection_units: bool,
    options: PaintOptions<'_>,
) -> PaintOutput {
    let mut output = PaintOutput {
        frame: Frame::new(width, height, capture_hidden_selection_units),
        hit_regions: Vec::new(),
    };
    {
        let mut painter = Painter {
            arena,
            options,
            output: &mut output,
        };
        painter.paint_node(
            root,
            PaintState {
                parent_x: 0,
                parent_y: 0,
                background: Background::Default,
                selection_background: None,
                foreground: Background::Default,
                clip: ClipBounds::unbounded(),
            },
        );
    }
    output
}

struct Painter<'a, 'out> {
    arena: &'a LayoutArena,
    options: PaintOptions<'a>,
    output: &'out mut PaintOutput,
}

#[derive(Clone, Copy)]
struct PaintState {
    parent_x: i32,
    parent_y: i32,
    background: Background,
    selection_background: Option<Background>,
    foreground: Background,
    clip: ClipBounds,
}

impl<'a, 'out> Painter<'a, 'out> {
    fn paint_node(&mut self, id: NodeId, state: PaintState) {
        let layout = self.arena.layout(id);
        let x = state.parent_x + layout.location.x.round() as i32;
        let y = state.parent_y + layout.location.y.round() as i32;
        let width = layout.size.width.round().max(0.0) as i32;
        let height = layout.size.height.round().max(0.0) as i32;
        let bounds = ClipRect::new(x, y, width, height);

        match self.arena.kind(id) {
            LayoutNodeKind::Element => self.paint_element(id, bounds, state),
            LayoutNodeKind::Text(text) => self.paint_text(text, x, y, state),
            LayoutNodeKind::Image(image) => self.paint_image_node(id, image, bounds, state),
            LayoutNodeKind::Input(input) => self.paint_input_node(id, input, bounds, state),
            LayoutNodeKind::TextArea(textarea) => {
                self.paint_textarea_node(id, textarea, bounds, state)
            }
        }
    }

    fn paint_element(&mut self, id: NodeId, bounds: ClipRect, state: PaintState) {
        let style = self.arena.style(id);
        let background = effective_background(
            self.paint_color(
                id,
                ColorTransitionProperty::BackgroundColor,
                style.background,
            ),
            state.background,
        );
        let foreground = effective_background(
            self.paint_color(id, ColorTransitionProperty::Color, style.color),
            state.foreground,
        );
        let selection_background = style.selection_background.or(state.selection_background);

        push_hit_region(&mut self.output.hit_regions, id, bounds, state.clip);
        self.output
            .frame
            .fill_rect(bounds, background, selection_background, state.clip);
        self.output
            .frame
            .clear_chunky_rounded_corners(bounds, style, state.clip);

        let content = content_box_rect(bounds, style);
        let child_clip = child_clip_for(style.overflow_x, style.overflow_y, content, state.clip);
        let (scroll_left, scroll_top) = self.arena.scroll_offset(id);
        let child_state = PaintState {
            parent_x: bounds.left - scroll_offset_cells(style.overflow_x, scroll_left),
            parent_y: bounds.top - scroll_offset_cells(style.overflow_y, scroll_top),
            background,
            selection_background,
            foreground,
            clip: child_clip,
        };

        if self.arena.inline_fragments(id).is_empty() {
            for child in self.arena.children(id) {
                self.paint_node(*child, child_state);
            }
        } else {
            self.paint_inline_fragments(
                id,
                content.left - bounds.left,
                content.top - bounds.top,
                child_state,
            );
        }

        let border_color =
            self.paint_color(id, ColorTransitionProperty::BorderColor, style.border_color);
        self.output.frame.stroke_border(
            bounds,
            style,
            border_color,
            selection_background,
            state.clip,
        );
    }

    fn paint_image_node(
        &mut self,
        id: NodeId,
        image: &ImageLayoutData,
        bounds: ClipRect,
        state: PaintState,
    ) {
        let style = self.arena.style(id);
        let background = effective_background(
            self.paint_color(
                id,
                ColorTransitionProperty::BackgroundColor,
                style.background,
            ),
            state.background,
        );
        let selection_background = style.selection_background.or(state.selection_background);
        let content = content_box_rect(bounds, style);
        let child_clip = child_clip_for(style.overflow_x, style.overflow_y, content, state.clip);

        push_hit_region(&mut self.output.hit_regions, id, bounds, state.clip);
        self.output
            .frame
            .fill_rect(bounds, background, selection_background, state.clip);
        self.output
            .frame
            .clear_chunky_rounded_corners(bounds, style, state.clip);
        paint_image(
            &mut self.output.frame,
            image,
            style.image_rendering,
            content,
            selection_background,
            child_clip,
        );
        let border_color =
            self.paint_color(id, ColorTransitionProperty::BorderColor, style.border_color);
        self.output.frame.stroke_border(
            bounds,
            style,
            border_color,
            selection_background,
            state.clip,
        );
    }

    fn paint_input_node(
        &mut self,
        id: NodeId,
        input: &InputLayoutData,
        bounds: ClipRect,
        state: PaintState,
    ) {
        let style = self.arena.style(id);
        let background = effective_background(
            self.paint_color(
                id,
                ColorTransitionProperty::BackgroundColor,
                style.background,
            ),
            state.background,
        );
        let foreground = effective_background(
            self.paint_color(id, ColorTransitionProperty::Color, style.color),
            state.foreground,
        );
        let selection_background = style.selection_background.or(state.selection_background);
        let content = content_box_rect(bounds, style);

        push_hit_region(&mut self.output.hit_regions, id, bounds, state.clip);
        self.output
            .frame
            .fill_rect(bounds, background, selection_background, state.clip);
        self.output
            .frame
            .clear_chunky_rounded_corners(bounds, style, state.clip);
        paint_input(
            &mut self.output.frame,
            input,
            content,
            background,
            selection_background,
            foreground,
            state.clip,
        );
        let border_color =
            self.paint_color(id, ColorTransitionProperty::BorderColor, style.border_color);
        self.output.frame.stroke_border(
            bounds,
            style,
            border_color,
            selection_background,
            state.clip,
        );
    }

    fn paint_textarea_node(
        &mut self,
        id: NodeId,
        textarea: &TextAreaLayoutData,
        bounds: ClipRect,
        state: PaintState,
    ) {
        let style = self.arena.style(id);
        let background = effective_background(
            self.paint_color(
                id,
                ColorTransitionProperty::BackgroundColor,
                style.background,
            ),
            state.background,
        );
        let foreground = effective_background(
            self.paint_color(id, ColorTransitionProperty::Color, style.color),
            state.foreground,
        );
        let selection_background = style.selection_background.or(state.selection_background);
        let content = content_box_rect(bounds, style);

        push_hit_region(&mut self.output.hit_regions, id, bounds, state.clip);
        self.output
            .frame
            .fill_rect(bounds, background, selection_background, state.clip);
        self.output
            .frame
            .clear_chunky_rounded_corners(bounds, style, state.clip);
        paint_textarea(
            &mut self.output.frame,
            textarea,
            content,
            background,
            selection_background,
            foreground,
            state.clip,
        );
        let border_color =
            self.paint_color(id, ColorTransitionProperty::BorderColor, style.border_color);
        self.output.frame.stroke_border(
            bounds,
            style,
            border_color,
            selection_background,
            state.clip,
        );
    }

    fn paint_text(&mut self, text: &str, x: i32, y: i32, state: PaintState) {
        let style = GlyphStyle {
            background: state.background,
            foreground: state.foreground,
            selection_background: state.selection_background,
        };
        for (offset, character) in text.chars().enumerate() {
            self.output
                .frame
                .write_glyph(x + offset as i32, y, character, 1, style, state.clip);
        }
    }

    fn paint_inline_fragments(&mut self, id: NodeId, x: i32, y: i32, state: PaintState) {
        let glyph_style = GlyphStyle {
            background: state.background,
            foreground: state.foreground,
            selection_background: state.selection_background,
        };

        for fragment in self.arena.inline_fragments(id) {
            let rect = ClipRect::new(
                state.parent_x + x + fragment.x as i32,
                state.parent_y + y + fragment.y as i32,
                fragment.width as i32,
                fragment.height as i32,
            );
            if let Some(hit_node) = fragment.hit_node {
                push_hit_region(&mut self.output.hit_regions, hit_node, rect, state.clip);
            }

            match fragment.kind {
                InlineFragmentKind::Text { character, .. } => {
                    self.output.frame.write_glyph(
                        rect.left,
                        rect.top,
                        character,
                        fragment.width as usize,
                        glyph_style,
                        state.clip,
                    );
                }
                InlineFragmentKind::Replaced => {
                    self.paint_inline_replaced(fragment.node, rect, state);
                }
            }
        }
    }

    fn paint_inline_replaced(&mut self, node: NodeId, rect: ClipRect, state: PaintState) {
        match self.arena.kind(node) {
            LayoutNodeKind::Image(image) => {
                self.paint_image_node(node, image, rect, state);
            }
            LayoutNodeKind::Input(input) => {
                self.paint_input_node(node, input, rect, state);
            }
            LayoutNodeKind::TextArea(textarea) => {
                self.paint_textarea_node(node, textarea, rect, state);
            }
            LayoutNodeKind::Element | LayoutNodeKind::Text(_) => {
                self.output.frame.fill_rect(
                    rect,
                    state.background,
                    state.selection_background,
                    state.clip,
                );
            }
        }
    }

    fn paint_color(
        &self,
        node: NodeId,
        property: ColorTransitionProperty,
        style_color: Background,
    ) -> Background {
        self.options
            .transitions
            .map(|transitions| {
                transitions.paint_color(
                    node,
                    property,
                    style_color,
                    self.options.now,
                    self.options.truecolor_enabled,
                )
            })
            .unwrap_or(style_color)
    }
}

fn effective_background(value: Background, inherited: Background) -> Background {
    if value == Background::Default {
        inherited
    } else {
        value
    }
}

fn paint_image(
    frame: &mut Frame,
    image: &ImageLayoutData,
    rendering: ImageRendering,
    rect: ClipRect,
    selection_background: Option<Background>,
    clip: ClipBounds,
) {
    let Some(rgb) = image.rgb.as_ref() else {
        return;
    };
    if image.width_px == 0 || image.height_px == 0 || rect.width() <= 0 || rect.height() <= 0 {
        return;
    }

    match rendering {
        ImageRendering::Ascii => {
            let rect_width = rect.width().max(1) as u32;
            let rect_height = rect.height().max(1) as u32;
            for y in rect.top..rect.bottom {
                for x in rect.left..rect.right {
                    let local_x = (x - rect.left).max(0) as u32;
                    let local_y = (y - rect.top).max(0) as u32;
                    let source_x = (local_x.saturating_mul(image.width_px) / rect_width)
                        .min(image.width_px.saturating_sub(1));
                    let source_y = (local_y.saturating_mul(image.height_px) / rect_height)
                        .min(image.height_px.saturating_sub(1));
                    let Some(pixel) = image_pixel(rgb, image.width_px, source_x, source_y) else {
                        continue;
                    };
                    frame.write_glyph(
                        x,
                        y,
                        ascii_pixel_char(pixel[0], pixel[1], pixel[2]),
                        1,
                        GlyphStyle {
                            background: Background::Default,
                            foreground: Background::Rgb(pixel[0], pixel[1], pixel[2]),
                            selection_background,
                        },
                        clip,
                    );
                }
            }
        }
        ImageRendering::HalfBlock => {
            let rect_width = rect.width().max(1) as u32;
            let virtual_height = (rect.height().max(1) as u32).saturating_mul(2);
            for y in rect.top..rect.bottom {
                for x in rect.left..rect.right {
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

                    let Some(top_pixel) = image_pixel(rgb, image.width_px, source_x, top_y) else {
                        continue;
                    };
                    let Some(bottom_pixel) = image_pixel(rgb, image.width_px, source_x, bottom_y)
                    else {
                        continue;
                    };

                    frame.write_glyph(
                        x,
                        y,
                        '▄',
                        1,
                        GlyphStyle {
                            background: Background::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]),
                            foreground: Background::Rgb(
                                bottom_pixel[0],
                                bottom_pixel[1],
                                bottom_pixel[2],
                            ),
                            selection_background,
                        },
                        clip,
                    );
                }
            }
        }
    }
}

fn paint_input(
    frame: &mut Frame,
    input: &InputLayoutData,
    rect: ClipRect,
    background: Background,
    selection_background: Option<Background>,
    foreground: Background,
    clip: ClipBounds,
) {
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }
    frame.fill_rect(rect, background, selection_background, clip);

    let width = rect.width() as usize;
    let chars = input.value.chars().collect::<Vec<_>>();
    let cursor = (input.cursor as usize).min(chars.len());
    let start = if input.focused && cursor >= width {
        cursor + 1 - width
    } else {
        0
    };
    let cursor_col = input.focused.then_some(cursor.saturating_sub(start));
    let glyph_style = GlyphStyle {
        background,
        foreground,
        selection_background,
    };

    for col in 0..width {
        let Some(character) = chars.get(start + col).copied() else {
            continue;
        };
        frame.write_glyph(
            rect.left + col as i32,
            rect.top,
            character,
            1,
            glyph_style,
            clip,
        );
    }
    if let Some(cursor_col) = cursor_col {
        frame.set_reversed(rect.left + cursor_col as i32, rect.top, true, clip);
    }
}

fn paint_textarea(
    frame: &mut Frame,
    textarea: &TextAreaLayoutData,
    rect: ClipRect,
    background: Background,
    selection_background: Option<Background>,
    foreground: Background,
    clip: ClipBounds,
) {
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }
    frame.fill_rect(rect, background, selection_background, clip);

    let layout = WrappedText::new(&textarea.value, rect.width() as usize);
    let glyph_style = GlyphStyle {
        background,
        foreground,
        selection_background,
    };
    for glyph in &layout.glyphs {
        if glyph.row as i32 >= rect.height() {
            continue;
        }
        frame.write_glyph(
            rect.left + glyph.col as i32,
            rect.top + glyph.row as i32,
            glyph.character,
            glyph.width,
            glyph_style,
            clip,
        );
    }
    if textarea.focused {
        let (row, col) = layout.cursor_position(textarea.cursor as usize);
        if row as i32 >= 0
            && (row as i32) < rect.height()
            && col as i32 >= 0
            && (col as i32) < rect.width()
        {
            frame.set_reversed(rect.left + col as i32, rect.top + row as i32, true, clip);
        }
    }
}

fn image_pixel(rgb: &[u8], width_px: u32, x: u32, y: u32) -> Option<[u8; 3]> {
    let index = (y as usize * width_px as usize + x as usize) * 3;
    let pixel = rgb.get(index..index + 3)?;
    Some([pixel[0], pixel[1], pixel[2]])
}

fn ascii_pixel_char(red: u8, green: u8, blue: u8) -> char {
    const CHARS: &[u8] = b" .:-=+*#%@";
    let luminance = 0.2126 * f32::from(red) + 0.7152 * f32::from(green) + 0.0722 * f32::from(blue);
    let index = ((luminance / 255.0) * (CHARS.len() - 1) as f32).round() as usize;
    CHARS[index.min(CHARS.len() - 1)] as char
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WrappedText {
    glyphs: Vec<TextGlyph>,
    cursor_positions: Vec<(usize, usize)>,
    end_position: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextGlyph {
    character: char,
    row: usize,
    col: usize,
    width: usize,
}

impl WrappedText {
    fn new(text: &str, wrap_width: usize) -> Self {
        let wrap_width = wrap_width.max(1);
        let chars = text.chars().collect::<Vec<_>>();
        let mut glyphs = Vec::new();
        let mut cursor_positions = Vec::with_capacity(chars.len() + 1);
        let mut row = 0;
        let mut col = 0;
        let mut index = 0;

        while index < chars.len() {
            cursor_positions.push((row, col));
            let character = chars[index];
            if character == '\r' {
                index += 1;
                continue;
            }
            if character == '\n' {
                row += 1;
                col = 0;
                index += 1;
                continue;
            }
            if !character.is_whitespace() {
                let word_end = next_word_end(&chars, index);
                let word_width = text_width(&chars[index..word_end]);
                if col > 0 && col + word_width > wrap_width {
                    row += 1;
                    col = 0;
                }
            }
            let width = character_cell_width(character);
            if col > 0 && width > 0 && col + width > wrap_width {
                row += 1;
                col = 0;
                if character == ' ' || character == '\t' {
                    index += 1;
                    continue;
                }
            }
            if width > 0 {
                glyphs.push(TextGlyph {
                    character,
                    row,
                    col,
                    width,
                });
            }
            col += width;
            index += 1;
        }
        cursor_positions.push((row, col));

        Self {
            glyphs,
            cursor_positions,
            end_position: (row, col),
        }
    }

    fn cursor_position(&self, cursor: usize) -> (usize, usize) {
        self.cursor_positions
            .get(cursor)
            .copied()
            .unwrap_or(self.end_position)
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

fn push_hit_region(
    hit_regions: &mut Vec<HitRegion>,
    id: NodeId,
    bounds: ClipRect,
    clip: ClipBounds,
) {
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

fn border_extent_cells(style: crate::style::BorderStyle) -> i32 {
    if style == crate::style::BorderStyle::None {
        0
    } else {
        1
    }
}

fn scroll_offset_cells(overflow: LayoutOverflow, value: u32) -> i32 {
    if overflow == LayoutOverflow::Scroll {
        value.min(i32::MAX as u32) as i32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::{AvailableSpace, Size};

    use crate::style::{BorderStyle, CssDimension, ImageRendering, LayoutDisplay};

    fn block_style(width: CssDimension, height: CssDimension) -> DivStyle {
        let mut style = DivStyle::default();
        style.width = width;
        style.height = height;
        style
    }

    #[test]
    fn paints_block_background_and_text() {
        let mut arena = LayoutArena::new();
        let mut root_style = block_style(CssDimension::Length(8.0), CssDimension::Length(3.0));
        root_style.background = Background::Blue;
        root_style.color = Background::White;
        let root = arena.create_element(root_style);
        let text = arena.create_text("hi");
        arena.append_child(root, text);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(8.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        let output = paint_arena(&arena, root, 8, 3, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, 'h');
        assert_eq!(output.frame.cell(1, 0).unwrap().character, 'i');
        assert_eq!(
            output.frame.cell(2, 0).unwrap().background,
            Background::Blue
        );
        assert_eq!(
            output.frame.cell(0, 0).unwrap().foreground,
            Background::White
        );
    }

    #[test]
    fn paints_border_outside_scroll_clip() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
        viewport_style.border_top = BorderStyle::Solid;
        viewport_style.border_right = BorderStyle::Solid;
        viewport_style.border_bottom = BorderStyle::Solid;
        viewport_style.border_left = BorderStyle::Solid;
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let mut child_style = block_style(CssDimension::Length(4.0), CssDimension::Length(4.0));
        child_style.background = Background::Red;
        let child = arena.create_element(child_style);
        arena.append_child(viewport, child);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        arena.set_scroll_offset(viewport, 0, 1);
        let output = paint_arena(&arena, viewport, 6, 3, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, '┌');
        assert_eq!(
            output.frame.cell(0, 0).unwrap().background,
            Background::Default
        );
        assert_eq!(output.frame.cell(1, 0).unwrap().character, '─');
        assert_eq!(output.frame.cell(1, 1).unwrap().background, Background::Red);
    }

    #[test]
    fn scrolling_inline_fragments_moves_painted_text() {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Length(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        viewport_style.white_space = crate::style::CssWhiteSpace::Pre;
        let viewport = arena.create_element(viewport_style);

        let mut span_style = DivStyle::default();
        span_style.display = LayoutDisplay::Inline;
        let span = arena.create_element(span_style);
        let text = arena.create_text("aaaaa\nbbbbb");
        arena.append_child(span, text);
        arena.append_child(viewport, span);

        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(1.0),
            },
        );
        let first = paint_arena(&arena, viewport, 5, 1, false);
        assert_eq!(first.frame.cell(0, 0).unwrap().character, 'a');

        arena.set_scroll_offset(viewport, 0, 1);
        let second = paint_arena(&arena, viewport, 5, 1, false);

        assert_eq!(second.frame.cell(0, 0).unwrap().character, 'b');
    }

    #[test]
    fn pre_whitespace_text_node_paints_multiple_rows() {
        let mut arena = LayoutArena::new();
        let mut panel_style = block_style(CssDimension::Length(5.0), CssDimension::Length(3.0));
        panel_style.white_space = crate::style::CssWhiteSpace::Pre;
        let panel = arena.create_element(panel_style);
        let text = arena.create_text("top\nmid\nbot");
        arena.append_child(panel, text);

        arena.compute_layout(
            panel,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        let output = paint_arena(&arena, panel, 5, 3, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, 't');
        assert_eq!(output.frame.cell(0, 1).unwrap().character, 'm');
        assert_eq!(output.frame.cell(0, 2).unwrap().character, 'b');
    }

    #[test]
    fn inline_span_hit_region_targets_span_while_text_paints() {
        let mut arena = LayoutArena::new();
        let row = arena.create_element(block_style(CssDimension::Length(6.0), CssDimension::Auto));
        let mut span_style = DivStyle::default();
        span_style.display = LayoutDisplay::Inline;
        let span = arena.create_element(span_style);
        let text = arena.create_text("hello");
        arena.append_child(span, text);
        arena.append_child(row, span);

        arena.compute_layout(
            row,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::MaxContent,
            },
        );
        let output = paint_arena(&arena, row, 6, 1, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, 'h');
        assert!(output.hit_regions.iter().any(|region| region.id == span
            && region.left == 0
            && region.top == 0
            && region.right == 1
            && region.bottom == 1));
    }

    #[test]
    fn paints_half_block_image_pixels() {
        let mut arena = LayoutArena::new();
        let image = arena.create_image(
            block_style(CssDimension::Length(1.0), CssDimension::Length(1.0)),
            1,
            2,
            8,
            16,
        );
        arena.set_image_pixels(image, 1, 2, vec![255, 0, 0, 0, 255, 0]);

        arena.compute_layout(
            image,
            Size {
                width: AvailableSpace::Definite(1.0),
                height: AvailableSpace::Definite(1.0),
            },
        );
        let output = paint_arena(&arena, image, 1, 1, false);
        let cell = output.frame.cell(0, 0).unwrap();

        assert_eq!(cell.character, '▄');
        assert_eq!(cell.background, Background::Rgb(255, 0, 0));
        assert_eq!(cell.foreground, Background::Rgb(0, 255, 0));
    }

    #[test]
    fn paints_ascii_image_pixels() {
        let mut arena = LayoutArena::new();
        let mut style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
        style.image_rendering = ImageRendering::Ascii;
        let image = arena.create_image(style, 2, 1, 8, 16);
        arena.set_image_pixels(image, 2, 1, vec![0, 0, 0, 255, 255, 255]);

        arena.compute_layout(
            image,
            Size {
                width: AvailableSpace::Definite(2.0),
                height: AvailableSpace::Definite(1.0),
            },
        );
        let output = paint_arena(&arena, image, 2, 1, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, ' ');
        assert_eq!(output.frame.cell(1, 0).unwrap().character, '@');
        assert_eq!(
            output.frame.cell(1, 0).unwrap().foreground,
            Background::Rgb(255, 255, 255)
        );
    }

    #[test]
    fn paints_focused_input_with_horizontal_cursor_scroll() {
        let mut arena = LayoutArena::new();
        let input = arena.create_input(
            block_style(CssDimension::Length(4.0), CssDimension::Length(1.0)),
            "abcdef",
        );
        arena.set_input_value(input, "abcdef", 5);
        arena.set_input_focused(input, true);

        arena.compute_layout(
            input,
            Size {
                width: AvailableSpace::Definite(4.0),
                height: AvailableSpace::Definite(1.0),
            },
        );
        let output = paint_arena(&arena, input, 4, 1, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, 'c');
        assert_eq!(output.frame.cell(1, 0).unwrap().character, 'd');
        assert_eq!(output.frame.cell(2, 0).unwrap().character, 'e');
        assert_eq!(output.frame.cell(3, 0).unwrap().character, 'f');
        assert!(output.frame.cell(3, 0).unwrap().reversed);
    }

    #[test]
    fn paints_input_own_background_color_and_border() {
        let mut arena = LayoutArena::new();
        let mut style = block_style(CssDimension::Length(8.0), CssDimension::Length(3.0));
        style.background = Background::Black;
        style.color = Background::White;
        style.border_top = BorderStyle::Rounded;
        style.border_right = BorderStyle::Rounded;
        style.border_bottom = BorderStyle::Rounded;
        style.border_left = BorderStyle::Rounded;
        style.border_color = Background::Cyan;
        let input = arena.create_input(style, "name");

        arena.compute_layout(
            input,
            Size {
                width: AvailableSpace::Definite(8.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        let output = paint_arena(&arena, input, 8, 3, false);

        let border = output.frame.cell(0, 0).unwrap();
        assert_eq!(border.character, '╭');
        assert_eq!(border.foreground, Background::Cyan);
        assert_eq!(border.background, Background::Black);

        let first_char = output.frame.cell(1, 1).unwrap();
        assert_eq!(first_char.character, 'n');
        assert_eq!(first_char.foreground, Background::White);
        assert_eq!(first_char.background, Background::Black);

        assert_eq!(
            output.frame.cell(6, 1).unwrap().background,
            Background::Black
        );
    }

    #[test]
    fn paints_textarea_with_wrapping_and_cursor() {
        let mut arena = LayoutArena::new();
        let textarea = arena.create_textarea(
            block_style(CssDimension::Length(5.0), CssDimension::Length(2.0)),
            "hello world",
        );
        arena.set_textarea_value(textarea, "hello world", 6);
        arena.set_textarea_focused(textarea, true);

        arena.compute_layout(
            textarea,
            Size {
                width: AvailableSpace::Definite(5.0),
                height: AvailableSpace::Definite(2.0),
            },
        );
        let output = paint_arena(&arena, textarea, 5, 2, false);

        assert_eq!(output.frame.cell(0, 0).unwrap().character, 'h');
        assert_eq!(output.frame.cell(4, 0).unwrap().character, 'o');
        assert_eq!(output.frame.cell(0, 1).unwrap().character, 'w');
        assert!(output.frame.cell(0, 1).unwrap().reversed);
    }

    #[test]
    fn hit_testing_returns_topmost_painted_region() {
        let mut arena = LayoutArena::new();
        let root = arena.create_element(block_style(
            CssDimension::Length(6.0),
            CssDimension::Length(3.0),
        ));
        let child = arena.create_element(block_style(
            CssDimension::Length(2.0),
            CssDimension::Length(1.0),
        ));
        arena.append_child(root, child);

        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(3.0),
            },
        );
        let output = paint_arena(&arena, root, 6, 3, false);

        assert_eq!(output.target_at(0, 0), Some(child));
        assert_eq!(output.target_at(5, 2), Some(root));
        assert_eq!(output.target_at(6, 0), None);
    }
}
