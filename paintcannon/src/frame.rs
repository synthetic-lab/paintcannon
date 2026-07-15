use std::io::{self, Write};

use termprofile::TermProfile;

use crate::style::{Background, BorderStyle, DivStyle};
use crate::terminal::{write_synchronized_output_begin, write_synchronized_output_end};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClipRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClipBounds {
    left: Option<i32>,
    top: Option<i32>,
    right: Option<i32>,
    bottom: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    pub(crate) anchor: SelectionPoint,
    pub(crate) focus: SelectionPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionPoint {
    pub(crate) order: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Frame {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    painted: Vec<bool>,
    foreground_painted: Vec<bool>,
    background_painted: Vec<bool>,
    selection_units: Vec<SelectionUnit>,
    next_selection_order: usize,
    capture_hidden_selection_units: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) background: Background,
    pub(crate) character: char,
    pub(crate) foreground: Background,
    pub(crate) selection_background: Option<Background>,
    pub(crate) selection_order: Option<usize>,
    pub(crate) reversed: bool,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
    pub(crate) wide_continuation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionUnit {
    order: usize,
    row: i32,
    character: char,
}

impl ClipRect {
    pub(crate) fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width.max(0),
            bottom: y + height.max(0),
        }
    }

    pub(crate) fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    pub(crate) fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
}

impl ClipBounds {
    pub(crate) fn unbounded() -> Self {
        Self {
            left: None,
            top: None,
            right: None,
            bottom: None,
        }
    }

    pub(crate) fn from_rect_axes(rect: ClipRect, clip_x: bool, clip_y: bool) -> Self {
        Self {
            left: clip_x.then_some(rect.left),
            top: clip_y.then_some(rect.top),
            right: clip_x.then_some(rect.right),
            bottom: clip_y.then_some(rect.bottom),
        }
    }

    pub(crate) fn intersect(self, other: Self) -> Self {
        Self {
            left: max_option(self.left, other.left),
            top: max_option(self.top, other.top),
            right: min_option(self.right, other.right),
            bottom: min_option(self.bottom, other.bottom),
        }
    }

    pub(crate) fn clip_rect(self, rect: ClipRect) -> Option<ClipRect> {
        let clipped = ClipRect {
            left: self.left.map_or(rect.left, |left| rect.left.max(left)),
            top: self.top.map_or(rect.top, |top| rect.top.max(top)),
            right: self.right.map_or(rect.right, |right| rect.right.min(right)),
            bottom: self
                .bottom
                .map_or(rect.bottom, |bottom| rect.bottom.min(bottom)),
        };

        (clipped.left < clipped.right && clipped.top < clipped.bottom).then_some(clipped)
    }

    pub(crate) fn contains(self, x: i32, y: i32) -> bool {
        self.left.is_none_or(|left| x >= left)
            && self.right.is_none_or(|right| x < right)
            && self.top.is_none_or(|top| y >= top)
            && self.bottom.is_none_or(|bottom| y < bottom)
    }

    pub(crate) fn vertical_range(self) -> Option<(i32, i32)> {
        Some((self.top?, self.bottom?))
    }
}

impl Frame {
    pub(crate) fn new(width: usize, height: usize, capture_hidden_selection_units: bool) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width.saturating_mul(height)],
            painted: vec![false; width.saturating_mul(height)],
            foreground_painted: vec![false; width.saturating_mul(height)],
            background_painted: vec![false; width.saturating_mul(height)],
            selection_units: Vec::new(),
            next_selection_order: 0,
            capture_hidden_selection_units,
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn layer(&self) -> Self {
        let mut layer = self.clone();
        layer.painted.fill(false);
        layer.foreground_painted.fill(false);
        layer.background_painted.fill(false);
        layer
    }

    pub(crate) fn composite_layer(
        &mut self,
        layer: Self,
        opacity: f32,
        default_foreground: Background,
        default_background: Background,
    ) {
        debug_assert_eq!(self.width, layer.width);
        debug_assert_eq!(self.height, layer.height);

        for index in 0..self.cells.len() {
            if !layer.painted[index] {
                continue;
            }
            let (cell, foreground_painted, background_painted) = composite_cell(
                self.cells[index],
                layer.cells[index],
                layer.foreground_painted[index],
                layer.background_painted[index],
                opacity,
                default_foreground,
                default_background,
            );
            self.cells[index] = cell;
            self.foreground_painted[index] |= foreground_painted;
            self.background_painted[index] |= background_painted;
            self.painted[index] |= foreground_painted || background_painted;
        }

        self.selection_units = layer.selection_units;
        self.next_selection_order = layer.next_selection_order;
    }

    #[cfg(test)]
    pub(crate) fn cell(&self, x: usize, y: usize) -> Option<Cell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.cells[y * self.width + x])
    }

    pub(crate) fn fill_rect(
        &mut self,
        rect: ClipRect,
        background: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        self.fill_rect_internal(rect, background, selection_background, clip, None);
    }

    pub(crate) fn fill_box_background(
        &mut self,
        rect: ClipRect,
        style: &DivStyle,
        background: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
    ) {
        self.fill_rect_internal(rect, background, selection_background, clip, Some(style));
    }

    fn fill_rect_internal(
        &mut self,
        rect: ClipRect,
        background: Background,
        selection_background: Option<Background>,
        clip: ClipBounds,
        box_style: Option<&DivStyle>,
    ) {
        if background == Background::Default && selection_background.is_none() {
            return;
        }

        let Some(bounds) = self.visible_rect(rect, clip) else {
            return;
        };
        let chunky_style = box_style.filter(|style| has_chunky_rounded_corner(style));

        for row in bounds.top as usize..bounds.bottom as usize {
            let start = row * self.width;
            for col in bounds.left as usize..bounds.right as usize {
                if chunky_style.is_some_and(|style| {
                    is_chunky_rounded_corner(rect, style, col as i32, row as i32)
                }) {
                    continue;
                }
                self.cells[start + col] = Cell {
                    background,
                    character: ' ',
                    foreground: Background::Default,
                    selection_background,
                    selection_order: None,
                    reversed: false,
                    bold: false,
                    italic: false,
                    underline: false,
                    strikethrough: false,
                    wide_continuation: false,
                };
                self.painted[start + col] = true;
                self.background_painted[start + col] = true;
            }
        }
    }

    pub(crate) fn set_reversed(&mut self, x: i32, y: i32, reversed: bool, clip: ClipBounds) {
        let Some(index) = self.cell_index(x, y) else {
            return;
        };
        if !clip.contains(x, y) {
            return;
        }
        self.cells[index].reversed = reversed;
        self.painted[index] = true;
        self.foreground_painted[index] = true;
        self.background_painted[index] = true;
    }

    pub(crate) fn write_glyph(
        &mut self,
        x: i32,
        y: i32,
        character: char,
        width: usize,
        style: GlyphStyle,
        clip: ClipBounds,
    ) {
        if width == 0 {
            return;
        }

        let visible = self.cell_index(x, y).is_some() && clip.contains(x, y);
        if !visible && !self.capture_hidden_selection_units {
            return;
        }

        let selection_order = self.push_selection_unit(y, character);
        if !visible {
            return;
        }

        let index = self.cell_index(x, y).expect("cell visibility checked");
        self.cells[index].character = character;
        self.cells[index].foreground = style.foreground;
        self.cells[index].selection_order = Some(selection_order);
        self.cells[index].bold = style.bold;
        self.cells[index].italic = style.italic;
        self.cells[index].underline = style.underline;
        self.cells[index].strikethrough = style.strikethrough;
        self.cells[index].wide_continuation = false;
        self.painted[index] = true;
        self.foreground_painted[index] = true;
        if style.background != Background::Default {
            self.cells[index].background = style.background;
            self.background_painted[index] = true;
        }
        if style.selection_background.is_some() {
            self.cells[index].selection_background = style.selection_background;
        }

        for offset in 1..width {
            let continuation_x = x + offset as i32;
            let Some(continuation_index) = self.cell_index(continuation_x, y) else {
                continue;
            };
            if !clip.contains(continuation_x, y) {
                continue;
            }

            self.cells[continuation_index].character = ' ';
            self.cells[continuation_index].foreground = style.foreground;
            self.cells[continuation_index].selection_order = None;
            self.cells[continuation_index].bold = style.bold;
            self.cells[continuation_index].italic = style.italic;
            self.cells[continuation_index].underline = style.underline;
            self.cells[continuation_index].strikethrough = style.strikethrough;
            self.cells[continuation_index].wide_continuation = true;
            self.painted[continuation_index] = true;
            self.foreground_painted[continuation_index] = true;
            if style.background != Background::Default {
                self.cells[continuation_index].background = style.background;
                self.background_painted[continuation_index] = true;
            }
            if style.selection_background.is_some() {
                self.cells[continuation_index].selection_background = style.selection_background;
            }
        }
    }

    pub(crate) fn write_decoration_glyph(
        &mut self,
        x: i32,
        y: i32,
        character: char,
        style: GlyphStyle,
        clip: ClipBounds,
    ) {
        let Some(index) = self.cell_index(x, y) else {
            return;
        };
        if !clip.contains(x, y) {
            return;
        }

        self.cells[index].character = character;
        self.cells[index].foreground = style.foreground;
        self.cells[index].selection_order = None;
        self.cells[index].bold = style.bold;
        self.cells[index].italic = style.italic;
        self.cells[index].underline = style.underline;
        self.cells[index].strikethrough = style.strikethrough;
        self.cells[index].wide_continuation = false;
        self.painted[index] = true;
        self.foreground_painted[index] = true;
        if style.background != Background::Default {
            self.cells[index].background = style.background;
            self.background_painted[index] = true;
        }
        if style.selection_background.is_some() {
            self.cells[index].selection_background = style.selection_background;
        }
    }

    pub(crate) fn stroke_border(
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

    pub(crate) fn apply_selection(&mut self, selection: Option<&Selection>) {
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

    pub(crate) fn selected_text(&self, selection: &Selection) -> Option<String> {
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
        (!text.is_empty()).then_some(text)
    }

    pub(crate) fn selection_point_for(&self, x: u32, y: u32) -> Option<SelectionPoint> {
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

    pub(crate) fn write_diff_to(
        &self,
        out: &mut impl Write,
        previous: Option<&Frame>,
        color_profile: TermProfile,
        synchronized: bool,
    ) -> io::Result<()> {
        if synchronized {
            write_synchronized_output_begin(out)?;
        }

        let result: io::Result<()> = (|| {
            if synchronized {
                write!(out, "\x1b[?7l")?;
            }
            write!(out, "\x1b[?25l")?;

            let Some(previous) = previous else {
                self.write_full_to(out, color_profile)?;
                return Ok(());
            };

            if previous.width != self.width || previous.height != self.height {
                write!(out, "\x1b[2J")?;
                self.write_full_to(out, color_profile)?;
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

                    self.write_span_to(out, row, start, col, color_profile)?;
                }
            }

            Ok(())
        })();

        let wrap_result = if synchronized {
            write!(out, "\x1b[?7h")
        } else {
            Ok(())
        };
        let end_result = if synchronized {
            write_synchronized_output_end(out)
        } else {
            Ok(())
        };
        result?;
        wrap_result?;
        end_result
    }

    pub(crate) fn write_full_to(
        &self,
        out: &mut impl Write,
        color_profile: TermProfile,
    ) -> io::Result<()> {
        write!(out, "\x1b[H")?;

        for row in 0..self.trailing_empty_rows_start() {
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
        let mut current_bold = false;
        let mut current_italic = false;
        let mut current_underline = false;
        let mut current_strikethrough = false;
        for col in start_col..end_col {
            let cell = self.cells[row * self.width + col];
            if cell.wide_continuation {
                continue;
            }
            if cell.reversed != current_reversed {
                if cell.reversed {
                    write!(out, "\x1b[7m")?;
                } else {
                    write!(out, "\x1b[27m")?;
                }
                current_reversed = cell.reversed;
            }
            if cell.bold != current_bold {
                if cell.bold {
                    write!(out, "\x1b[1m")?;
                } else {
                    write!(out, "\x1b[22m")?;
                }
                current_bold = cell.bold;
            }
            if cell.italic != current_italic {
                if cell.italic {
                    write!(out, "\x1b[3m")?;
                } else {
                    write!(out, "\x1b[23m")?;
                }
                current_italic = cell.italic;
            }
            if cell.underline != current_underline {
                if cell.underline {
                    write!(out, "\x1b[4m")?;
                } else {
                    write!(out, "\x1b[24m")?;
                }
                current_underline = cell.underline;
            }
            if cell.strikethrough != current_strikethrough {
                if cell.strikethrough {
                    write!(out, "\x1b[9m")?;
                } else {
                    write!(out, "\x1b[29m")?;
                }
                current_strikethrough = cell.strikethrough;
            }
            if cell.background != current_background {
                write!(out, "{}", cell.background.ansi_bg(color_profile))?;
                current_background = cell.background;
            }
            if cell.foreground != current_foreground {
                write!(out, "{}", cell.foreground.ansi_fg(color_profile))?;
                current_foreground = cell.foreground;
            }
            write!(out, "{}", cell.character)?;
        }

        write!(
            out,
            "\x1b[27m\x1b[22m\x1b[23m\x1b[24m\x1b[29m\x1b[39m\x1b[49m"
        )
    }

    fn trailing_empty_rows_start(&self) -> usize {
        if self.width == 0 {
            return 0;
        }

        self.cells
            .chunks(self.width)
            .rposition(|row| row.iter().any(|cell| *cell != Cell::default()))
            .map(|row| row + 1)
            .unwrap_or(0)
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
        let Some(index) = self.cell_index(x, y) else {
            return;
        };
        if !clip.contains(x, y) {
            return;
        }

        self.cells[index].character = character;
        self.cells[index].foreground = foreground;
        self.cells[index].selection_order = None;
        self.cells[index].bold = false;
        self.cells[index].italic = false;
        self.cells[index].underline = false;
        self.cells[index].strikethrough = false;
        self.cells[index].wide_continuation = false;
        self.painted[index] = true;
        self.foreground_painted[index] = true;
        if selection_background.is_some() {
            self.cells[index].selection_background = selection_background;
        }
    }

    fn visible_rect(&self, rect: ClipRect, clip: ClipBounds) -> Option<ClipRect> {
        let bounds = clip.clip_rect(rect)?;
        let visible = ClipRect {
            left: bounds.left.max(0),
            top: bounds.top.max(0),
            right: bounds.right.min(self.width as i32).max(0),
            bottom: bounds.bottom.min(self.height as i32).max(0),
        };
        (visible.left < visible.right && visible.top < visible.bottom).then_some(visible)
    }

    fn cell_index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width + x as usize)
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
}

fn composite_cell(
    backdrop: Cell,
    source: Cell,
    source_foreground_painted: bool,
    source_background_painted: bool,
    opacity: f32,
    default_foreground: Background,
    default_background: Background,
) -> (Cell, bool, bool) {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity == 0.0 || (!source_foreground_painted && !source_background_painted) {
        return (backdrop, false, false);
    }

    let (backdrop_foreground, backdrop_background) =
        visual_colors(backdrop, default_foreground, default_background);
    let (source_foreground, source_background) =
        visual_colors(source, default_foreground, default_background);
    let source_has_ink = !source.character.is_whitespace() || source.wide_continuation;

    if source_has_ink && source_foreground_painted {
        let backdrop_foreground_fills_cell = foreground_glyph_fills_cell(backdrop.character);
        let backdrop_surface = if backdrop_foreground_fills_cell {
            backdrop_foreground
        } else {
            backdrop_background
        };
        let mut result = source;
        result.foreground = blend_rgb(source_foreground, backdrop_surface, opacity);
        if source_background_painted {
            result.background = blend_rgb(source_background, backdrop_surface, opacity);
        } else if backdrop_foreground_fills_cell {
            result.background =
                Background::Rgb(backdrop_surface.0, backdrop_surface.1, backdrop_surface.2);
        } else {
            result.background = visual_background(backdrop);
        }
        result.reversed = false;
        (result, true, source_background_painted)
    } else if source_background_painted {
        let mut result = backdrop;
        result.foreground = blend_rgb(source_background, backdrop_foreground, opacity);
        result.background = blend_rgb(source_background, backdrop_background, opacity);
        result.reversed = false;
        (result, true, true)
    } else {
        (backdrop, false, false)
    }
}

fn foreground_glyph_fills_cell(character: char) -> bool {
    matches!(character, '█' | '🭁' | '🭌' | '🭒' | '🭝')
}

fn visual_background(cell: Cell) -> Background {
    if cell.reversed {
        cell.foreground
    } else {
        cell.background
    }
}

fn visual_colors(
    cell: Cell,
    default_foreground: Background,
    default_background: Background,
) -> ((u8, u8, u8), (u8, u8, u8)) {
    let foreground = resolve_rgb(cell.foreground, default_foreground, (255, 255, 255));
    let background = resolve_rgb(cell.background, default_background, (0, 0, 0));
    if cell.reversed {
        (background, foreground)
    } else {
        (foreground, background)
    }
}

fn resolve_rgb(
    color: Background,
    terminal_default: Background,
    fallback: (u8, u8, u8),
) -> (u8, u8, u8) {
    color
        .rgb()
        .or_else(|| terminal_default.rgb())
        .unwrap_or(fallback)
}

fn blend_rgb(source: (u8, u8, u8), backdrop: (u8, u8, u8), opacity: f32) -> Background {
    let blend = |source: u8, backdrop: u8| {
        (f32::from(source) * opacity + f32::from(backdrop) * (1.0 - opacity)).round() as u8
    };
    Background::Rgb(
        blend(source.0, backdrop.0),
        blend(source.1, backdrop.1),
        blend(source.2, backdrop.2),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GlyphStyle {
    pub(crate) background: Background,
    pub(crate) foreground: Background,
    pub(crate) selection_background: Option<Background>,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strikethrough: bool,
}

impl Default for GlyphStyle {
    fn default() -> Self {
        Self {
            background: Background::Default,
            foreground: Background::Default,
            selection_background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
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
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            wide_continuation: false,
        }
    }
}

fn normalized_selection(selection: &Selection) -> (SelectionPoint, SelectionPoint) {
    if selection.anchor.order <= selection.focus.order {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
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

fn has_border(style: &DivStyle) -> bool {
    style.border_top != BorderStyle::None
        || style.border_right != BorderStyle::None
        || style.border_bottom != BorderStyle::None
        || style.border_left != BorderStyle::None
}

fn has_chunky_rounded_corner(style: &DivStyle) -> bool {
    (style.border_top == BorderStyle::ChunkyRounded
        || style.border_bottom == BorderStyle::ChunkyRounded)
        && (style.border_left == BorderStyle::ChunkyRounded
            || style.border_right == BorderStyle::ChunkyRounded)
}

fn is_chunky_rounded_corner(rect: ClipRect, style: &DivStyle, x: i32, y: i32) -> bool {
    let left = rect.left;
    let right = rect.right - 1;
    let top = rect.top;
    let bottom = rect.bottom - 1;
    if left > right || top > bottom {
        return false;
    }

    (x == left
        && y == top
        && style.border_top == BorderStyle::ChunkyRounded
        && style.border_left == BorderStyle::ChunkyRounded)
        || (right != left
            && x == right
            && y == top
            && style.border_top == BorderStyle::ChunkyRounded
            && style.border_right == BorderStyle::ChunkyRounded)
        || (bottom != top
            && x == left
            && y == bottom
            && style.border_bottom == BorderStyle::ChunkyRounded
            && style.border_left == BorderStyle::ChunkyRounded)
        || (right != left
            && bottom != top
            && x == right
            && y == bottom
            && style.border_bottom == BorderStyle::ChunkyRounded
            && style.border_right == BorderStyle::ChunkyRounded)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_border_style() -> DivStyle {
        DivStyle {
            border_top: BorderStyle::Solid,
            border_right: BorderStyle::Solid,
            border_bottom: BorderStyle::Solid,
            border_left: BorderStyle::Solid,
            ..DivStyle::default()
        }
    }

    #[test]
    fn translucent_background_blends_over_both_colors_of_existing_text() {
        let backdrop = Cell {
            character: 'X',
            foreground: Background::Rgb(200, 0, 0),
            background: Background::Rgb(0, 0, 200),
            ..Cell::default()
        };
        let source = Cell {
            background: Background::Rgb(0, 200, 0),
            ..Cell::default()
        };

        let (cell, foreground_painted, background_painted) = composite_cell(
            backdrop,
            source,
            false,
            true,
            0.5,
            Background::White,
            Background::Black,
        );

        assert_eq!(cell.character, 'X');
        assert_eq!(cell.foreground, Background::Rgb(100, 100, 0));
        assert_eq!(cell.background, Background::Rgb(0, 100, 100));
        assert!(foreground_painted);
        assert!(background_painted);
    }

    #[test]
    fn translucent_foreground_blends_with_the_lower_background() {
        let backdrop = Cell {
            background: Background::Rgb(0, 0, 255),
            ..Cell::default()
        };
        let source = Cell {
            character: 'X',
            foreground: Background::Rgb(255, 0, 0),
            ..Cell::default()
        };

        let (cell, foreground_painted, background_painted) = composite_cell(
            backdrop,
            source,
            true,
            false,
            0.5,
            Background::White,
            Background::Black,
        );

        assert_eq!(cell.character, 'X');
        assert_eq!(cell.foreground, Background::Rgb(128, 0, 128));
        assert_eq!(cell.background, Background::Rgb(0, 0, 255));
        assert!(foreground_painted);
        assert!(!background_painted);
    }

    #[test]
    fn opacity_resolves_default_colors_from_the_terminal_palette() {
        let source = Cell {
            character: 'X',
            ..Cell::default()
        };

        let cell = composite_cell(
            Cell::default(),
            source,
            true,
            false,
            0.5,
            Background::Rgb(10, 20, 30),
            Background::Rgb(40, 50, 60),
        )
        .0;

        assert_eq!(cell.foreground, Background::Rgb(25, 35, 45));
        assert_eq!(cell.background, Background::Default);
    }

    #[test]
    fn upper_foreground_always_discards_lower_foreground() {
        let source = Cell {
            character: 'X',
            foreground: Background::White,
            ..Cell::default()
        };
        let first_backdrop = Cell {
            character: 'X',
            foreground: Background::Red,
            background: Background::Blue,
            ..Cell::default()
        };
        let second_backdrop = Cell {
            character: 'Y',
            foreground: Background::Green,
            background: Background::Blue,
            ..Cell::default()
        };

        let first = composite_cell(
            first_backdrop,
            source,
            true,
            false,
            0.5,
            Background::White,
            Background::Black,
        )
        .0;
        let second = composite_cell(
            second_backdrop,
            source,
            true,
            false,
            0.5,
            Background::White,
            Background::Black,
        )
        .0;

        assert_eq!(first.character, 'X');
        assert_eq!(second.character, 'X');
        assert_eq!(first.foreground, second.foreground);
    }

    #[test]
    fn translucent_text_treats_all_chunky_corners_as_full_cell_coverage() {
        let source = Cell {
            character: 't',
            foreground: Background::Rgb(255, 255, 255),
            background: Background::Rgb(0, 0, 200),
            ..Cell::default()
        };

        for character in ['🭁', '🭌', '🭒', '🭝'] {
            let backdrop = Cell {
                character,
                foreground: Background::Rgb(0, 200, 0),
                background: Background::Rgb(200, 0, 0),
                ..Cell::default()
            };
            let cell = composite_cell(
                backdrop,
                source,
                true,
                true,
                0.5,
                Background::White,
                Background::Black,
            )
            .0;

            assert_eq!(cell.character, 't');
            assert_eq!(cell.foreground, Background::Rgb(128, 228, 128));
            assert_eq!(cell.background, Background::Rgb(0, 100, 100));
        }
    }

    #[test]
    fn translucent_text_treats_full_block_as_full_cell_coverage() {
        let backdrop = Cell {
            character: '█',
            foreground: Background::Rgb(0, 200, 0),
            background: Background::Rgb(200, 0, 0),
            ..Cell::default()
        };
        let source = Cell {
            character: 't',
            foreground: Background::Rgb(255, 255, 255),
            ..Cell::default()
        };

        let cell = composite_cell(
            backdrop,
            source,
            true,
            false,
            0.5,
            Background::White,
            Background::Black,
        )
        .0;

        assert_eq!(cell.character, 't');
        assert_eq!(cell.foreground, Background::Rgb(128, 228, 128));
        assert_eq!(cell.background, Background::Rgb(0, 200, 0));
    }

    #[test]
    fn undecorated_whitespace_has_no_foreground_coverage() {
        let backdrop = Cell {
            character: 'X',
            foreground: Background::Red,
            background: Background::Blue,
            ..Cell::default()
        };
        let source = Cell {
            character: ' ',
            foreground: Background::White,
            ..Cell::default()
        };

        assert_eq!(
            composite_cell(
                backdrop,
                source,
                true,
                false,
                0.5,
                Background::White,
                Background::Black,
            ),
            (backdrop, false, false)
        );
    }

    #[test]
    fn nested_opacity_multiplies_group_alpha() {
        let mut frame = Frame::new(1, 1, false);
        frame.fill_rect(
            ClipRect::new(0, 0, 1, 1),
            Background::Blue,
            None,
            ClipBounds::unbounded(),
        );
        let mut outer = frame.layer();
        let mut inner = outer.layer();
        inner.fill_rect(
            ClipRect::new(0, 0, 1, 1),
            Background::Red,
            None,
            ClipBounds::unbounded(),
        );
        outer.composite_layer(inner, 0.5, Background::White, Background::Black);
        frame.composite_layer(outer, 0.5, Background::White, Background::Black);

        assert_eq!(
            frame.cell(0, 0).unwrap().background,
            Background::Rgb(64, 0, 192)
        );
    }

    #[test]
    fn fill_rect_respects_clip_bounds() {
        let mut frame = Frame::new(5, 3, false);
        assert_eq!(frame.width(), 5);
        assert_eq!(frame.height(), 3);
        frame.fill_rect(
            ClipRect::new(0, 0, 5, 3),
            Background::Blue,
            None,
            ClipBounds::from_rect_axes(ClipRect::new(1, 1, 2, 1), true, true),
        );

        assert_eq!(frame.cell(0, 1).unwrap().background, Background::Default);
        assert_eq!(frame.cell(1, 1).unwrap().background, Background::Blue);
        assert_eq!(frame.cell(2, 1).unwrap().background, Background::Blue);
        assert_eq!(frame.cell(3, 1).unwrap().background, Background::Default);
    }

    #[test]
    fn clip_bounds_intersect_on_enabled_axes() {
        let rect = ClipRect::new(0, 0, 10, 4);
        assert_eq!(rect.width(), 10);
        assert_eq!(rect.height(), 4);

        let clip = ClipBounds::from_rect_axes(rect, true, false).intersect(
            ClipBounds::from_rect_axes(ClipRect::new(3, 1, 4, 1), true, true),
        );

        assert_eq!(
            clip.clip_rect(ClipRect::new(0, 0, 10, 4)),
            Some(ClipRect {
                left: 3,
                top: 1,
                right: 7,
                bottom: 2,
            })
        );
    }

    #[test]
    fn border_uses_requested_glyphs() {
        let mut frame = Frame::new(4, 3, false);
        let style = solid_border_style();

        frame.stroke_border(
            ClipRect::new(0, 0, 4, 3),
            &style,
            Background::White,
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cell(0, 0).unwrap().character, '┌');
        assert_eq!(frame.cell(3, 0).unwrap().character, '┐');
        assert_eq!(frame.cell(0, 2).unwrap().character, '└');
        assert_eq!(frame.cell(3, 2).unwrap().character, '┘');
        assert_eq!(frame.cell(1, 0).unwrap().character, '─');
        assert_eq!(frame.cell(0, 1).unwrap().character, '│');
    }

    #[test]
    fn full_width_border_paints_right_edge_inside_frame() {
        let mut frame = Frame::new(80, 4, false);
        let style = solid_border_style();

        frame.stroke_border(
            ClipRect::new(0, 0, 80, 4),
            &style,
            Background::White,
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cell(79, 0).unwrap().character, '┐');
        assert_eq!(frame.cell(79, 1).unwrap().character, '│');
        assert_eq!(frame.cell(79, 3).unwrap().character, '┘');
        assert!(frame.cell(80, 0).is_none());
    }

    #[test]
    fn chunky_rounded_box_background_preserves_underlying_corner_cells() {
        let mut frame = Frame::new(4, 3, false);
        let style = DivStyle {
            border_top: BorderStyle::ChunkyRounded,
            border_right: BorderStyle::ChunkyRounded,
            border_bottom: BorderStyle::ChunkyRounded,
            border_left: BorderStyle::ChunkyRounded,
            ..DivStyle::default()
        };

        frame.fill_rect(
            ClipRect::new(0, 0, 4, 3),
            Background::Blue,
            None,
            ClipBounds::unbounded(),
        );
        frame.fill_box_background(
            ClipRect::new(0, 0, 4, 3),
            &style,
            Background::Red,
            None,
            ClipBounds::unbounded(),
        );
        frame.stroke_border(
            ClipRect::new(0, 0, 4, 3),
            &style,
            Background::Blue,
            None,
            ClipBounds::unbounded(),
        );

        assert_eq!(frame.cell(0, 0).unwrap().character, '🭁');
        assert_eq!(frame.cell(3, 0).unwrap().character, '🭌');
        assert_eq!(frame.cell(0, 2).unwrap().character, '🭒');
        assert_eq!(frame.cell(3, 2).unwrap().character, '🭝');
        assert_eq!(frame.cell(0, 0).unwrap().background, Background::Blue);
        assert_eq!(frame.cell(1, 0).unwrap().background, Background::Red);
        assert_eq!(frame.cell(1, 1).unwrap().background, Background::Red);
    }

    #[test]
    fn selection_text_tracks_rows_and_trimmed_line_endings() {
        let mut frame = Frame::new(6, 2, false);
        let glyph_style = GlyphStyle::default();
        for (index, character) in "abc".chars().enumerate() {
            frame.write_glyph(
                index as i32,
                0,
                character,
                1,
                glyph_style,
                ClipBounds::unbounded(),
            );
        }
        for (index, character) in "de ".chars().enumerate() {
            frame.write_glyph(
                index as i32,
                1,
                character,
                1,
                glyph_style,
                ClipBounds::unbounded(),
            );
        }

        let selection = Selection {
            anchor: SelectionPoint { order: 1 },
            focus: SelectionPoint { order: 5 },
        };

        assert_eq!(frame.selected_text(&selection).as_deref(), Some("bc\nde"));
    }

    #[test]
    fn selection_point_uses_nearest_selectable_cell_on_row() {
        let mut frame = Frame::new(8, 1, false);
        frame.write_glyph(2, 0, 'a', 1, GlyphStyle::default(), ClipBounds::unbounded());
        frame.write_glyph(5, 0, 'b', 1, GlyphStyle::default(), ClipBounds::unbounded());

        assert_eq!(
            frame.selection_point_for(0, 0),
            Some(SelectionPoint { order: 0 })
        );
        assert_eq!(
            frame.selection_point_for(7, 0),
            Some(SelectionPoint { order: 1 })
        );
    }

    #[test]
    fn apply_selection_reverses_selected_cells() {
        let mut frame = Frame::new(3, 1, false);
        for (index, character) in "abc".chars().enumerate() {
            frame.write_glyph(
                index as i32,
                0,
                character,
                1,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
            );
        }

        frame.apply_selection(Some(&Selection {
            anchor: SelectionPoint { order: 1 },
            focus: SelectionPoint { order: 2 },
        }));

        assert!(!frame.cell(0, 0).unwrap().reversed);
        assert!(frame.cell(1, 0).unwrap().reversed);
        assert!(frame.cell(2, 0).unwrap().reversed);
    }

    #[test]
    fn diff_writes_only_changed_spans() {
        let previous = Frame::new(4, 2, false);
        let mut next = Frame::new(4, 2, false);
        next.write_glyph(2, 1, 'x', 1, GlyphStyle::default(), ClipBounds::unbounded());

        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[2;3Hx"));
        assert!(!output.contains("\x1b[H"));
        assert!(!output.contains("\x1b[2J"));
    }

    #[test]
    fn full_frame_write_emits_text_attribute_sgr() {
        let mut frame = Frame::new(2, 1, false);
        frame.write_glyph(
            0,
            0,
            'a',
            1,
            GlyphStyle {
                bold: true,
                italic: true,
                underline: true,
                strikethrough: true,
                ..Default::default()
            },
            ClipBounds::unbounded(),
        );
        frame.write_glyph(1, 0, 'b', 1, GlyphStyle::default(), ClipBounds::unbounded());

        let mut bytes = Vec::new();
        frame
            .write_full_to(&mut bytes, TermProfile::NoColor)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[1m\x1b[3m\x1b[4m\x1b[9ma"));
        assert!(output.contains("a\x1b[22m\x1b[23m\x1b[24m\x1b[29mb"));
        assert!(output.ends_with("\x1b[27m\x1b[22m\x1b[23m\x1b[24m\x1b[29m\x1b[39m\x1b[49m"));
    }

    #[test]
    fn diff_writes_cells_when_only_text_attributes_change() {
        let mut previous = Frame::new(1, 1, false);
        previous.write_glyph(0, 0, 'x', 1, GlyphStyle::default(), ClipBounds::unbounded());
        let mut next = Frame::new(1, 1, false);
        next.write_glyph(
            0,
            0,
            'x',
            1,
            GlyphStyle {
                strikethrough: true,
                ..Default::default()
            },
            ClipBounds::unbounded(),
        );

        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[1;1H\x1b[9mx"));
    }

    #[test]
    fn terminal_frame_writes_disable_autowrap_while_flushing() {
        let mut next = Frame::new(4, 1, false);
        next.write_glyph(3, 0, 'x', 1, GlyphStyle::default(), ClipBounds::unbounded());

        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, None, TermProfile::NoColor, true)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[?7l"));
        assert!(output.contains("\x1b[?7h"));
        assert!(output.find("\x1b[?7l") < output.find("x"));
        assert!(output.find("x") < output.find("\x1b[?7h"));
    }

    #[test]
    fn full_frame_write_skips_trailing_empty_rows() {
        let mut frame = Frame::new(4, 4, false);
        frame.write_glyph(0, 1, 'x', 1, GlyphStyle::default(), ClipBounds::unbounded());

        let mut bytes = Vec::new();
        frame
            .write_full_to(&mut bytes, TermProfile::NoColor)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[1;1H"));
        assert!(output.contains("\x1b[2;1H"));
        assert!(!output.contains("\x1b[3;1H"));
        assert!(!output.contains("\x1b[4;1H"));
    }

    #[test]
    fn diff_clears_and_full_repaints_after_resize() {
        let previous = Frame::new(4, 2, false);
        let next = Frame::new(5, 2, false);

        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[2J"));
        assert!(output.contains("\x1b[H"));
    }
}
