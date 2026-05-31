#![allow(dead_code)]

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
        if background == Background::Default && selection_background.is_none() {
            return;
        }

        let Some(bounds) = self.visible_rect(rect, clip) else {
            return;
        };

        for row in bounds.top as usize..bounds.bottom as usize {
            let start = row * self.width;
            for col in bounds.left as usize..bounds.right as usize {
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

    pub(crate) fn clear_chunky_rounded_corners(
        &mut self,
        rect: ClipRect,
        style: &DivStyle,
        clip: ClipBounds,
    ) {
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

    pub(crate) fn clear_cell(&mut self, x: i32, y: i32, clip: ClipBounds) {
        let Some(index) = self.cell_index(x, y) else {
            return;
        };
        if !clip.contains(x, y) {
            return;
        }
        self.cells[index] = Cell::default();
    }

    pub(crate) fn set_reversed(&mut self, x: i32, y: i32, reversed: bool, clip: ClipBounds) {
        let Some(index) = self.cell_index(x, y) else {
            return;
        };
        if !clip.contains(x, y) {
            return;
        }
        self.cells[index].reversed = reversed;
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
        self.cells[index].wide_continuation = false;
        if style.background != Background::Default {
            self.cells[index].background = style.background;
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
            self.cells[continuation_index].wide_continuation = true;
            if style.background != Background::Default {
                self.cells[continuation_index].background = style.background;
            }
            if style.selection_background.is_some() {
                self.cells[continuation_index].selection_background = style.selection_background;
            }
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
            if cell.reversed != current_reversed {
                if cell.reversed {
                    write!(out, "\x1b[7m")?;
                } else {
                    write!(out, "\x1b[27m")?;
                }
                current_reversed = cell.reversed;
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

        write!(out, "\x1b[27m\x1b[39m\x1b[49m")
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
        self.cells[index].wide_continuation = false;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GlyphStyle {
    pub(crate) background: Background,
    pub(crate) foreground: Background,
    pub(crate) selection_background: Option<Background>,
}

impl Default for GlyphStyle {
    fn default() -> Self {
        Self {
            background: Background::Default,
            foreground: Background::Default,
            selection_background: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_border_style() -> DivStyle {
        let mut style = DivStyle::default();
        style.border_top = BorderStyle::Solid;
        style.border_right = BorderStyle::Solid;
        style.border_bottom = BorderStyle::Solid;
        style.border_left = BorderStyle::Solid;
        style
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
    fn chunky_rounded_corners_can_clear_background_bleed() {
        let mut frame = Frame::new(4, 3, false);
        let mut style = DivStyle::default();
        style.border_top = BorderStyle::ChunkyRounded;
        style.border_right = BorderStyle::ChunkyRounded;
        style.border_bottom = BorderStyle::ChunkyRounded;
        style.border_left = BorderStyle::ChunkyRounded;

        frame.fill_rect(
            ClipRect::new(0, 0, 4, 3),
            Background::Blue,
            None,
            ClipBounds::unbounded(),
        );
        frame.clear_chunky_rounded_corners(
            ClipRect::new(0, 0, 4, 3),
            &style,
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
        assert_eq!(frame.cell(0, 0).unwrap().background, Background::Default);
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
