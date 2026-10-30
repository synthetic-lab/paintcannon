use std::collections::HashMap;
use std::io::{self, Write};

use termprofile::TermProfile;

use crate::style::{Background, BorderStyle, DivStyle};
use crate::terminal::{write_synchronized_output_begin, write_synchronized_output_end};
use crate::text::GraphemeSuffix;

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
    pub(crate) source: usize,
    pub(crate) offset: usize,
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
    grapheme_suffixes: HashMap<usize, String>,
    pub(crate) hyperlinks: HashMap<taffy::NodeId, std::sync::Arc<str>>,
    capture_hidden_selection_units: bool,
    pub(crate) powerline_extended: bool,
    pub(crate) legacy_symbols: bool,
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
    pub(crate) hyperlink: Option<taffy::NodeId>,
    pub(crate) wide_continuation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectionUnit {
    point: SelectionPoint,
    row: i32,
    character: char,
    suffix: Option<Box<GraphemeSuffix>>,
    logical_line: Option<LogicalLine>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LogicalLine {
    pub(crate) text_flow_id: usize,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GraphemeGlyph<'a> {
    pub(crate) character: char,
    pub(crate) suffix: Option<&'a str>,
    pub(crate) width: usize,
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
            grapheme_suffixes: HashMap::new(),
            hyperlinks: HashMap::new(),
            capture_hidden_selection_units,
            powerline_extended: false,
            legacy_symbols: false,
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    /// The visible fragment under the pointer, not the link's unclipped layout box.
    pub(crate) fn hyperlink_region_at(&self, x: u32, y: u32) -> Option<ClipRect> {
        let x = i32::try_from(x).ok()?;
        let y = i32::try_from(y).ok()?;
        let index = self.cell_index(x, y)?;
        let link = self.cells[index].hyperlink?;
        let row = &self.cells[y as usize * self.width..(y as usize + 1) * self.width];
        let mut left = x as usize;
        let mut right = left + 1;
        while left > 0 && row[left - 1].hyperlink == Some(link) {
            left -= 1;
        }
        while right < row.len() && row[right].hyperlink == Some(link) {
            right += 1;
        }
        Some(ClipRect::new(left as i32, y, (right - left) as i32, 1))
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
            let source_replaces_glyph = layer.foreground_painted[index]
                && (!layer.cells[index].character.is_whitespace()
                    || layer.cells[index].wide_continuation);
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
            if source_replaces_glyph {
                if let Some(suffix) = layer.grapheme_suffixes.get(&index) {
                    self.grapheme_suffixes.insert(index, suffix.clone());
                } else {
                    self.grapheme_suffixes.remove(&index);
                }
            }
            self.foreground_painted[index] |= foreground_painted;
            self.background_painted[index] |= background_painted;
            self.painted[index] |= foreground_painted || background_painted;
        }

        self.hyperlinks.extend(layer.hyperlinks);
        self.selection_units = layer.selection_units;
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
        let borders = box_style
            .map(|style| ResolvedBorders::new(style, self.powerline_extended, self.legacy_symbols));

        for row in bounds.top as usize..bounds.bottom as usize {
            let start = row * self.width;
            for col in bounds.left as usize..bounds.right as usize {
                if borders.as_ref().is_some_and(|borders| {
                    borders.preserves_background_at(rect, col as i32, row as i32)
                }) {
                    continue;
                }
                let index = start + col;
                self.grapheme_suffixes.remove(&index);
                self.cells[index] = Cell {
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
                    hyperlink: None,
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
        point: SelectionPoint,
    ) {
        self.write_grapheme(
            x,
            y,
            GraphemeGlyph {
                character,
                suffix: None,
                width,
            },
            style,
            clip,
            point,
            None,
        );
    }

    pub(crate) fn write_grapheme(
        &mut self,
        x: i32,
        y: i32,
        glyph: GraphemeGlyph<'_>,
        style: GlyphStyle,
        clip: ClipBounds,
        point: SelectionPoint,
        logical_line: Option<LogicalLine>,
    ) {
        let visible = self.cell_index(x, y).is_some() && clip.contains(x, y);
        if !visible && !self.capture_hidden_selection_units {
            return;
        }

        let selection_order =
            self.push_selection_unit(point, y, glyph.character, glyph.suffix, logical_line);
        if !visible || glyph.width == 0 {
            return;
        }

        let index = self.cell_index(x, y).expect("cell visibility checked");
        if let Some(suffix) = glyph.suffix.filter(|suffix| !suffix.is_empty()) {
            self.grapheme_suffixes.insert(index, suffix.to_owned());
        } else {
            self.grapheme_suffixes.remove(&index);
        }
        self.cells[index].character = glyph.character;
        self.cells[index].foreground = style.foreground;
        self.cells[index].selection_order = Some(selection_order);
        self.cells[index].bold = style.bold;
        self.cells[index].italic = style.italic;
        self.cells[index].underline = style.underline;
        self.cells[index].strikethrough = style.strikethrough;
        self.cells[index].hyperlink = style.hyperlink;
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

        for offset in 1..glyph.width {
            let continuation_x = x + offset as i32;
            let Some(continuation_index) = self.cell_index(continuation_x, y) else {
                continue;
            };
            if !clip.contains(continuation_x, y) {
                continue;
            }

            self.grapheme_suffixes.remove(&continuation_index);
            self.cells[continuation_index].character = ' ';
            self.cells[continuation_index].foreground = style.foreground;
            self.cells[continuation_index].selection_order = Some(selection_order);
            self.cells[continuation_index].bold = style.bold;
            self.cells[continuation_index].italic = style.italic;
            self.cells[continuation_index].underline = style.underline;
            self.cells[continuation_index].strikethrough = style.strikethrough;
            self.cells[continuation_index].hyperlink = style.hyperlink;
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

        self.grapheme_suffixes.remove(&index);
        self.cells[index].character = character;
        self.cells[index].foreground = style.foreground;
        self.cells[index].selection_order = None;
        self.cells[index].bold = style.bold;
        self.cells[index].italic = style.italic;
        self.cells[index].underline = style.underline;
        self.cells[index].strikethrough = style.strikethrough;
        self.cells[index].hyperlink = style.hyperlink;
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

        let borders = ResolvedBorders::new(style, self.powerline_extended, self.legacy_symbols);
        let left = rect.left;
        let right = rect.right - 1;
        let top = rect.top;
        let bottom = rect.bottom - 1;

        if borders.top.is_some() {
            for x in left..=right {
                self.write_border_cell(
                    x,
                    top,
                    borders.character_at(x == left, x == right, true, false),
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }

        if borders.bottom.is_some() && bottom != top {
            for x in left..=right {
                self.write_border_cell(
                    x,
                    bottom,
                    borders.character_at(x == left, x == right, false, true),
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }

        if let Some(glyphs) = borders.left {
            let start = if borders.top.is_none() { top } else { top + 1 };
            let end = if borders.bottom.is_none() {
                bottom
            } else {
                bottom - 1
            };
            for y in start..=end {
                self.write_border_cell(
                    left,
                    y,
                    glyphs.left,
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }

        if let Some(glyphs) = borders.right.filter(|_| right != left) {
            let start = if borders.top.is_none() { top } else { top + 1 };
            let end = if borders.bottom.is_none() {
                bottom
            } else {
                bottom - 1
            };
            for y in start..=end {
                self.write_border_cell(
                    right,
                    y,
                    glyphs.right,
                    border_color,
                    selection_background,
                    clip,
                );
            }
        }
    }

    pub(crate) fn order_selection_units(&mut self, source_order: &HashMap<usize, usize>) {
        if self.selection_units.is_sorted_by(|left, right| {
            if left.point.source == right.point.source {
                left.point.offset <= right.point.offset
            } else {
                source_order[&left.point.source] < source_order[&right.point.source]
            }
        }) {
            return;
        }

        let mut units = std::mem::take(&mut self.selection_units)
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>();
        units.sort_by_cached_key(|(_, unit)| (source_order[&unit.point.source], unit.point.offset));
        let mut new_orders = vec![0; units.len()];
        self.selection_units = units
            .into_iter()
            .enumerate()
            .map(|(new_order, (old_order, unit))| {
                new_orders[old_order] = new_order;
                unit
            })
            .collect();
        for cell in &mut self.cells {
            cell.selection_order = cell.selection_order.map(|order| new_orders[order]);
        }
    }

    pub(crate) fn selection_range(
        &self,
        selection: &Selection,
    ) -> Option<std::ops::RangeInclusive<usize>> {
        let anchor = self.selection_order_for(selection.anchor)?;
        let focus = self.selection_order_for(selection.focus)?;
        Some(anchor.min(focus)..=anchor.max(focus))
    }

    fn selection_order_for(&self, point: SelectionPoint) -> Option<usize> {
        let mut closest = None;
        for (order, unit) in self.selection_units.iter().enumerate() {
            if unit.point.source != point.source {
                continue;
            }
            let distance = unit.point.offset.abs_diff(point.offset);
            if distance == 0 {
                return Some(order);
            }
            if closest.is_none_or(|(_, best_distance)| distance < best_distance) {
                closest = Some((order, distance));
            }
        }
        closest.map(|(order, _)| order)
    }

    pub(crate) fn apply_selection(&mut self, selection: Option<&Selection>) {
        let Some(selection) = selection else {
            return;
        };

        let Some(range) = self.selection_range(selection) else {
            return;
        };
        for cell in &mut self.cells {
            if cell
                .selection_order
                .is_some_and(|order| range.contains(&order))
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
        let range = self.selection_range(selection)?;
        let mut lines = Vec::new();
        let mut previous_unit: Option<&SelectionUnit> = None;
        let mut current_line = String::new();

        for unit in &self.selection_units[range] {
            if let Some(previous) = previous_unit {
                let line_breaks = match (previous.logical_line, unit.logical_line) {
                    (Some(previous), Some(current))
                        if previous.text_flow_id == current.text_flow_id =>
                    {
                        current.index.saturating_sub(previous.index)
                    }
                    _ => u32::from(previous.row != unit.row),
                };
                if line_breaks > 0 {
                    lines.push(current_line.trim_end().to_string());
                    current_line.clear();
                    for _ in 1..line_breaks {
                        lines.push(String::new());
                    }
                }
            }

            current_line.push(unit.character);
            if let Some(suffix) = &unit.suffix {
                current_line.push_str(suffix.as_str());
            }
            previous_unit = Some(unit);
        }

        if previous_unit.is_some() {
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
            return Some(self.selection_units[order].point);
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
            return Some(self.selection_units[first_order].point);
        }
        if col >= last_col {
            return Some(self.selection_units[last_order].point);
        }

        selectable_cols
            .into_iter()
            .min_by_key(|(candidate_col, _)| candidate_col.abs_diff(col))
            .map(|(_, order)| self.selection_units[order].point)
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
            write!(out, "\x1b[?7l")?;
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
                    if self.cell_matches(previous, index) {
                        col += 1;
                        continue;
                    }

                    let start = col;
                    while col < self.width {
                        let index = row * self.width + col;
                        if self.cell_matches(previous, index) {
                            break;
                        }
                        if self.cell_uses_complex_grapheme(index)
                            || previous.cell_uses_complex_grapheme(index)
                        {
                            col = self.width;
                            break;
                        }
                        col += 1;
                    }

                    self.write_span_to(out, row, start, col, color_profile)?;
                }
            }

            Ok(())
        })();

        let link_result = if result.is_err() && !self.hyperlinks.is_empty() {
            write!(out, "\x1b]8;;\x1b\\")
        } else {
            Ok(())
        };
        let wrap_result = write!(out, "\x1b[?7h");
        let end_result = if synchronized {
            write_synchronized_output_end(out)
        } else {
            Ok(())
        };
        result?;
        link_result?;
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

        let mut current_hyperlink = None;
        let mut current_background = Background::Default;
        let mut current_foreground = Background::Default;
        let mut current_reversed = false;
        let mut current_bold = false;
        let mut current_italic = false;
        let mut current_underline = false;
        let mut current_strikethrough = false;
        let mut next_glyph_needs_positioning = false;
        for col in start_col..end_col {
            let cell = self.cells[row * self.width + col];
            if cell.wide_continuation {
                next_glyph_needs_positioning = true;
                continue;
            }
            if next_glyph_needs_positioning {
                write!(out, "\x1b[{};{}H", row + 1, col + 1)?;
            }
            next_glyph_needs_positioning = self.cell_uses_complex_grapheme(row * self.width + col);
            if cell.hyperlink != current_hyperlink {
                if let Some(id) = cell.hyperlink {
                    let href = &self.hyperlinks[&id];
                    write!(
                        out,
                        "\x1b]8;id=pc-{}-{};{href}\x1b\\",
                        std::process::id(),
                        usize::from(id)
                    )?;
                } else {
                    write!(out, "\x1b]8;;\x1b\\")?;
                }
                current_hyperlink = cell.hyperlink;
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
            let mut first_col_after_glyph = col + 1;
            while first_col_after_glyph < end_col
                && self.cells[row * self.width + first_col_after_glyph].wide_continuation
            {
                first_col_after_glyph += 1;
            }
            if first_col_after_glyph > col + 1 {
                for _ in col..first_col_after_glyph {
                    write!(out, " ")?;
                }
                write!(out, "\x1b[{};{}H", row + 1, col + 1)?;
            }
            write!(out, "{}", cell.character)?;
            if let Some(suffix) = self.grapheme_suffixes.get(&(row * self.width + col)) {
                write!(out, "{suffix}")?;
            }
        }

        if current_hyperlink.is_some() {
            write!(out, "\x1b]8;;\x1b\\")?;
        }
        write!(
            out,
            "\x1b[27m\x1b[22m\x1b[23m\x1b[24m\x1b[29m\x1b[39m\x1b[49m"
        )
    }

    fn cell_uses_complex_grapheme(&self, index: usize) -> bool {
        !self.cells[index].character.is_ascii() || self.grapheme_suffixes.contains_key(&index)
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

        self.grapheme_suffixes.remove(&index);
        self.cells[index].background = visual_background(self.cells[index]);
        self.cells[index].reversed = false;
        self.cells[index].character = character;
        self.cells[index].foreground = foreground;
        self.cells[index].selection_order = None;
        self.cells[index].bold = false;
        self.cells[index].italic = false;
        self.cells[index].underline = false;
        self.cells[index].strikethrough = false;
        self.cells[index].hyperlink = None;
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

    fn cell_matches(&self, other: &Self, index: usize) -> bool {
        self.cells[index] == other.cells[index]
            && self.cells[index]
                .hyperlink
                .and_then(|id| self.hyperlinks.get(&id))
                == other.cells[index]
                    .hyperlink
                    .and_then(|id| other.hyperlinks.get(&id))
            && self.grapheme_suffixes.get(&index) == other.grapheme_suffixes.get(&index)
    }

    fn push_selection_unit(
        &mut self,
        point: SelectionPoint,
        row: i32,
        character: char,
        suffix: Option<&str>,
        logical_line: Option<LogicalLine>,
    ) -> usize {
        let order = self.selection_units.len();
        self.selection_units.push(SelectionUnit {
            point,
            row,
            character,
            suffix: suffix.map(|suffix| Box::new(GraphemeSuffix::new(suffix))),
            logical_line,
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
    character == '█' || is_shaped_block(character)
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
    pub(crate) hyperlink: Option<taffy::NodeId>,
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
            hyperlink: None,
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
            hyperlink: None,
            wide_continuation: false,
        }
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
    style.border_top.height() > 0.0
        || style.border_right != BorderStyle::None
        || style.border_bottom.height() > 0.0
        || style.border_left != BorderStyle::None
}

struct ResolvedBorders {
    top: Option<BorderGlyphs>,
    right: Option<BorderGlyphs>,
    bottom: Option<BorderGlyphs>,
    left: Option<BorderGlyphs>,
}

impl ResolvedBorders {
    fn new(style: &DivStyle, powerline_extended: bool, legacy_symbols: bool) -> Self {
        let resolve = |side| border_glyphs(side, powerline_extended, legacy_symbols);
        Self {
            top: (style.border_top.height() > 0.0).then(|| resolve(style.border_top)),
            right: (style.border_right.width() > 0.0).then(|| resolve(style.border_right)),
            bottom: (style.border_bottom.height() > 0.0).then(|| resolve(style.border_bottom)),
            left: (style.border_left.width() > 0.0).then(|| resolve(style.border_left)),
        }
    }

    fn preserves_background_at(&self, rect: ClipRect, x: i32, y: i32) -> bool {
        let left = rect.left;
        let right = rect.right - 1;
        let top = rect.top;
        let bottom = rect.bottom - 1;
        if x != left && x != right {
            return false;
        }

        // Pill caps expose the backdrop along the whole side. Other shaped
        // corners require both adjacent border sides to support that shape.
        (x == left && self.left.is_some_and(|glyphs| is_shaped_block(glyphs.left)))
            || (x == right
                && self
                    .right
                    .is_some_and(|glyphs| is_shaped_block(glyphs.right)))
            || (x == left
                && y == top
                && self
                    .top
                    .is_some_and(|glyphs| is_shaped_block(glyphs.top_left))
                && self
                    .left
                    .is_some_and(|glyphs| is_shaped_block(glyphs.top_left)))
            || (right != left
                && x == right
                && y == top
                && self
                    .top
                    .is_some_and(|glyphs| is_shaped_block(glyphs.top_right))
                && self
                    .right
                    .is_some_and(|glyphs| is_shaped_block(glyphs.top_right)))
            || (bottom != top
                && x == left
                && y == bottom
                && self
                    .bottom
                    .is_some_and(|glyphs| is_shaped_block(glyphs.bottom_left))
                && self
                    .left
                    .is_some_and(|glyphs| is_shaped_block(glyphs.bottom_left)))
            || (right != left
                && bottom != top
                && x == right
                && y == bottom
                && self
                    .bottom
                    .is_some_and(|glyphs| is_shaped_block(glyphs.bottom_right))
                && self
                    .right
                    .is_some_and(|glyphs| is_shaped_block(glyphs.bottom_right)))
    }

    fn character_at(&self, at_left: bool, at_right: bool, at_top: bool, at_bottom: bool) -> char {
        let top = self.top.filter(|_| at_top);
        let right = self.right.filter(|_| at_right);
        let bottom = self.bottom.filter(|_| at_bottom);
        let left = self.left.filter(|_| at_left);
        let Some(glyphs) = top.or(bottom).or(left).or(right) else {
            return ' ';
        };
        match (
            top.is_some(),
            right.is_some(),
            bottom.is_some(),
            left.is_some(),
        ) {
            (true, true, _, _) => glyphs.top_right,
            (true, _, _, true) => glyphs.top_left,
            (_, true, true, _) => glyphs.bottom_right,
            (_, _, true, true) => glyphs.bottom_left,
            (true, _, _, _) | (_, _, true, _) => glyphs.horizontal,
            (_, _, _, true) => glyphs.left,
            (_, true, _, _) => glyphs.right,
            _ => ' ',
        }
    }
}

#[derive(Clone, Copy)]
struct BorderGlyphs {
    horizontal: char,
    left: char,
    right: char,
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
}

impl BorderGlyphs {
    fn corners(self) -> [char; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
        ]
    }
}

const CHUNKY_GLYPHS: BorderGlyphs = BorderGlyphs {
    horizontal: '█',
    left: '█',
    right: '█',
    top_left: '\u{e0ba}',
    top_right: '\u{e0b8}',
    bottom_left: '\u{e0be}',
    bottom_right: '\u{e0bc}',
};

const CHUNKY_ROUNDED_GLYPHS: BorderGlyphs = BorderGlyphs {
    horizontal: '█',
    left: '█',
    right: '█',
    top_left: '🭁',
    top_right: '🭌',
    bottom_left: '🭒',
    bottom_right: '🭝',
};

const PILL_GLYPHS: BorderGlyphs = BorderGlyphs {
    horizontal: '█',
    left: '\u{e0b6}',
    right: '\u{e0b4}',
    top_left: '\u{e0b6}',
    top_right: '\u{e0b4}',
    bottom_left: '\u{e0b6}',
    bottom_right: '\u{e0b4}',
};

const SQUARE_GLYPHS: BorderGlyphs = BorderGlyphs {
    horizontal: '█',
    left: '█',
    right: '█',
    top_left: '█',
    top_right: '█',
    bottom_left: '█',
    bottom_right: '█',
};

fn is_shaped_block(character: char) -> bool {
    CHUNKY_GLYPHS.corners().contains(&character)
        || CHUNKY_ROUNDED_GLYPHS.corners().contains(&character)
        || PILL_GLYPHS.corners().contains(&character)
}

fn border_glyphs(
    style: BorderStyle,
    powerline_extended: bool,
    legacy_symbols: bool,
) -> BorderGlyphs {
    match style {
        BorderStyle::None | BorderStyle::Solid => BorderGlyphs {
            horizontal: '─',
            left: '│',
            right: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        },
        BorderStyle::Double => BorderGlyphs {
            horizontal: '═',
            left: '║',
            right: '║',
            top_left: '╔',
            top_right: '╗',
            bottom_left: '╚',
            bottom_right: '╝',
        },
        BorderStyle::Heavy => BorderGlyphs {
            horizontal: '━',
            left: '┃',
            right: '┃',
            top_left: '┏',
            top_right: '┓',
            bottom_left: '┗',
            bottom_right: '┛',
        },
        BorderStyle::Rounded => BorderGlyphs {
            horizontal: '─',
            left: '│',
            right: '│',
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
        },
        BorderStyle::ChunkyRounded | BorderStyle::ChunkyBestEffort if legacy_symbols => {
            CHUNKY_ROUNDED_GLYPHS
        }
        BorderStyle::Chunky | BorderStyle::ChunkyBestEffort if powerline_extended => CHUNKY_GLYPHS,
        BorderStyle::Pill if powerline_extended => PILL_GLYPHS,
        BorderStyle::Chunky
        | BorderStyle::ChunkyRounded
        | BorderStyle::ChunkyBestEffort
        | BorderStyle::Pill => SQUARE_GLYPHS,
        BorderStyle::Ascii => BorderGlyphs {
            horizontal: '-',
            left: '|',
            right: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection_point(offset: usize) -> SelectionPoint {
        SelectionPoint { source: 0, offset }
    }

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
    fn translucent_text_treats_shaped_border_glyphs_as_full_cell_coverage() {
        let source = Cell {
            character: 't',
            foreground: Background::Rgb(255, 255, 255),
            background: Background::Rgb(0, 0, 200),
            ..Cell::default()
        };

        for character in [
            '🭁', '🭌', '🭒', '🭝', '\u{e0b4}', '\u{e0b6}', '\u{e0b8}', '\u{e0ba}', '\u{e0bc}',
            '\u{e0be}',
        ] {
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
    fn translucent_text_does_not_treat_partial_blocks_as_full_cell_coverage() {
        for character in ['▀', '▄', '▌', '▐', '▖', '─', 'A'] {
            let backdrop = Cell {
                character,
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
            assert_eq!(
                cell.foreground,
                Background::Rgb(228, 128, 128),
                "{character}"
            );
            assert_eq!(cell.background, backdrop.background, "{character}");
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
    fn pill_caps_use_the_visible_backdrop_even_over_selected_text() {
        let mut frame = Frame::new(5, 1, false);
        frame.powerline_extended = true;
        let clip = ClipBounds::unbounded();
        frame.write_glyph(
            0,
            0,
            'x',
            1,
            GlyphStyle {
                foreground: Background::White,
                background: Background::Blue,
                ..Default::default()
            },
            clip,
            selection_point(frame.selection_units.len()),
        );
        frame.set_reversed(0, 0, true, clip);
        let style = DivStyle {
            border_left: BorderStyle::Pill,
            border_right: BorderStyle::Pill,
            ..Default::default()
        };
        let rect = ClipRect::new(0, 0, 5, 1);
        frame.fill_box_background(rect, &style, Background::Green, None, clip);
        frame.stroke_border(rect, &style, Background::Green, None, clip);
        let cap = frame.cell(0, 0).unwrap();
        assert_eq!(cap.character, '\u{e0b6}');
        assert_eq!(cap.foreground, Background::Green);
        assert_eq!(cap.background, Background::White);
        assert!(!cap.reversed);
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
    fn unknown_legacy_support_uses_filled_square_corners() {
        let mut frame = Frame::new(4, 3, false);
        let style = DivStyle {
            border_top: BorderStyle::ChunkyRounded,
            border_right: BorderStyle::ChunkyRounded,
            border_bottom: BorderStyle::ChunkyRounded,
            border_left: BorderStyle::ChunkyRounded,
            ..DivStyle::default()
        };
        let rect = ClipRect::new(0, 0, 4, 3);
        let clip = ClipBounds::unbounded();
        frame.fill_rect(rect, Background::Blue, None, clip);
        frame.fill_box_background(rect, &style, Background::Red, None, clip);
        frame.stroke_border(rect, &style, Background::Green, None, clip);
        for (x, y) in [(0, 0), (3, 0), (0, 2), (3, 2)] {
            assert_eq!(frame.cell(x, y).unwrap().character, '█');
            assert_eq!(frame.cell(x, y).unwrap().background, Background::Red);
        }
    }

    #[test]
    fn chunky_rounded_box_background_preserves_underlying_corner_cells() {
        let mut frame = Frame::new(4, 3, false);
        frame.legacy_symbols = true;
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
    fn chunky_best_effort_prefers_legacy_then_powerline_then_squares() {
        let border = crate::style::parse_border_style("chunky-best-effort").unwrap();
        for (legacy, powerline, corners) in [
            (true, true, ['🭁', '🭌', '🭒', '🭝']),
            (true, false, ['🭁', '🭌', '🭒', '🭝']),
            (
                false,
                true,
                ['\u{e0ba}', '\u{e0b8}', '\u{e0be}', '\u{e0bc}'],
            ),
            (false, false, ['█'; 4]),
        ] {
            let mut frame = Frame::new(4, 3, false);
            frame.legacy_symbols = legacy;
            frame.powerline_extended = powerline;
            let style = DivStyle {
                border_top: border,
                border_right: border,
                border_bottom: border,
                border_left: border,
                ..DivStyle::default()
            };
            let rect = ClipRect::new(0, 0, 4, 3);
            let clip = ClipBounds::unbounded();
            frame.fill_rect(rect, Background::Blue, None, clip);
            frame.fill_box_background(rect, &style, Background::Red, None, clip);
            frame.stroke_border(rect, &style, Background::Green, None, clip);
            for ((x, y), expected) in [(0, 0), (3, 0), (0, 2), (3, 2)].into_iter().zip(corners) {
                let cell = frame.cell(x, y).unwrap();
                assert_eq!(cell.character, expected);
                assert_eq!(
                    cell.background,
                    if legacy || powerline {
                        Background::Blue
                    } else {
                        Background::Red
                    }
                );
            }
            assert_eq!(frame.cell(1, 0).unwrap().character, '█');
            assert_eq!(frame.cell(0, 1).unwrap().character, '█');
        }
    }

    #[test]
    fn border_fallbacks_and_backgrounds_agree_for_all_capability_combinations() {
        for (legacy, powerline) in [(false, false), (false, true), (true, false), (true, true)] {
            for border in [
                BorderStyle::Chunky,
                BorderStyle::ChunkyRounded,
                BorderStyle::ChunkyBestEffort,
                BorderStyle::Pill,
            ] {
                let expected = match border {
                    BorderStyle::ChunkyRounded if legacy => ['🭁', '🭌', '🭒', '🭝'],
                    BorderStyle::ChunkyBestEffort if legacy => ['🭁', '🭌', '🭒', '🭝'],
                    BorderStyle::Chunky | BorderStyle::ChunkyBestEffort if powerline => {
                        ['\u{e0ba}', '\u{e0b8}', '\u{e0be}', '\u{e0bc}']
                    }
                    BorderStyle::Pill if powerline => {
                        ['\u{e0b6}', '\u{e0b4}', '\u{e0b6}', '\u{e0b4}']
                    }
                    _ => ['█'; 4],
                };
                let style = DivStyle {
                    border_top: border,
                    border_bottom: border,
                    border_left: border,
                    border_right: border,
                    ..Default::default()
                };
                for clip in [
                    ClipBounds::unbounded(),
                    ClipBounds::from_rect_axes(ClipRect::new(2, 0, 3, 4), true, true),
                ] {
                    let mut frame = Frame::new(6, 5, false);
                    frame.legacy_symbols = legacy;
                    frame.powerline_extended = powerline;
                    frame.fill_rect(
                        ClipRect::new(0, 0, 6, 5),
                        Background::Blue,
                        None,
                        ClipBounds::unbounded(),
                    );
                    let rect = ClipRect::new(1, 1, 4, 3);
                    frame.fill_box_background(rect, &style, Background::Red, None, clip);
                    frame.stroke_border(rect, &style, Background::Green, None, clip);
                    for ((x, y), character) in
                        [(1, 1), (4, 1), (1, 3), (4, 3)].into_iter().zip(expected)
                    {
                        let cell = frame.cell(x, y).unwrap();
                        let visible = clip.contains(x as i32, y as i32);
                        assert_eq!(cell.character, if visible { character } else { ' ' });
                        assert_eq!(
                            cell.background,
                            if visible && character == '█' {
                                Background::Red
                            } else {
                                Background::Blue
                            },
                            "glyph={character}, legacy={legacy}, powerline={powerline}, ({x},{y})"
                        );
                    }
                    assert_eq!(frame.cell(2, 2).unwrap().background, Background::Red);
                    assert_eq!(frame.cell(0, 0).unwrap().background, Background::Blue);
                }
            }
        }
    }

    #[test]
    fn narrow_borders_preserve_corner_and_pill_precedence() {
        for (border, width, rows) in [
            (BorderStyle::Pill, 1, &["\u{e0b6}"][..]),
            (BorderStyle::Chunky, 1, &["\u{e0b8}", "█", "\u{e0bc}"][..]),
            (BorderStyle::ChunkyRounded, 4, &["🭁██🭌"][..]),
        ] {
            for supported in [false, true] {
                let mut frame = Frame::new(width, rows.len(), false);
                frame.powerline_extended = supported;
                frame.legacy_symbols = supported;
                let style = DivStyle {
                    border_top: border,
                    border_right: border,
                    border_bottom: border,
                    border_left: border,
                    ..Default::default()
                };
                let rect = ClipRect::new(0, 0, width as i32, rows.len() as i32);
                let clip = ClipBounds::unbounded();
                frame.fill_rect(rect, Background::Blue, None, clip);
                frame.fill_box_background(rect, &style, Background::Red, None, clip);
                frame.stroke_border(rect, &style, Background::Green, None, clip);
                for (y, row) in rows.iter().enumerate() {
                    for (x, character) in row.chars().enumerate() {
                        let cell = frame.cell(x, y).unwrap();
                        assert_eq!(cell.character, if supported { character } else { '█' });
                        assert_eq!(
                            cell.background,
                            if supported && character != '█' {
                                Background::Blue
                            } else {
                                Background::Red
                            }
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mixed_border_sides_keep_their_own_glyphs_and_fallbacks() {
        for supported in [false, true] {
            let mut frame = Frame::new(4, 3, false);
            frame.legacy_symbols = supported;
            frame.powerline_extended = supported;
            let style = DivStyle {
                border_top: BorderStyle::ChunkyBestEffort,
                border_left: BorderStyle::Pill,
                border_bottom: BorderStyle::Double,
                ..Default::default()
            };
            let rect = ClipRect::new(0, 0, 4, 3);
            let clip = ClipBounds::unbounded();
            frame.fill_rect(rect, Background::Blue, None, clip);
            frame.fill_box_background(rect, &style, Background::Red, None, clip);
            frame.stroke_border(rect, &style, Background::Green, None, clip);
            let expected = if supported {
                ["🭁███", "\u{e0b6}   ", "╚═══"]
            } else {
                ["████", "█   ", "╚═══"]
            };
            for (y, row) in expected.iter().enumerate() {
                for (x, character) in row.chars().enumerate() {
                    let cell = frame.cell(x, y).unwrap();
                    assert_eq!(cell.character, character);
                    assert_eq!(
                        cell.background,
                        if supported && x == 0 {
                            Background::Blue
                        } else {
                            Background::Red
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn chunky_border_uses_inward_facing_powerline_corners() {
        let glyphs = border_glyphs(BorderStyle::Chunky, true, false);
        assert_eq!(glyphs.top_left, '\u{e0ba}');
        assert_eq!(glyphs.top_right, '\u{e0b8}');
        assert_eq!(glyphs.bottom_left, '\u{e0be}');
        assert_eq!(glyphs.bottom_right, '\u{e0bc}');
    }

    #[test]
    fn supported_chunky_corners_preserve_underlying_background() {
        let mut frame = Frame::new(4, 3, false);
        frame.powerline_extended = true;
        let style = DivStyle {
            border_top: BorderStyle::Chunky,
            border_right: BorderStyle::Chunky,
            border_bottom: BorderStyle::Chunky,
            border_left: BorderStyle::Chunky,
            ..DivStyle::default()
        };
        let rect = ClipRect::new(0, 0, 4, 3);
        let clip = ClipBounds::unbounded();
        frame.fill_rect(rect, Background::Blue, None, clip);
        frame.fill_box_background(rect, &style, Background::Red, None, clip);
        frame.stroke_border(rect, &style, Background::Green, None, clip);
        for ((x, y), expected) in [(0, 0), (3, 0), (0, 2), (3, 2)]
            .into_iter()
            .zip(['\u{e0ba}', '\u{e0b8}', '\u{e0be}', '\u{e0bc}'])
        {
            let cell = frame.cell(x, y).unwrap();
            assert_eq!(cell.character, expected);
            assert_eq!(cell.background, Background::Blue);
        }
        assert_eq!(frame.cell(1, 0).unwrap().character, '█');
        assert_eq!(frame.cell(1, 1).unwrap().background, Background::Red);
        assert!(frame.layer().powerline_extended);
    }

    #[test]
    fn chunky_border_has_filled_edges_and_square_corners() {
        let mut frame = Frame::new(4, 3, false);
        let style = DivStyle {
            border_top: BorderStyle::Chunky,
            border_right: BorderStyle::Chunky,
            border_bottom: BorderStyle::Chunky,
            border_left: BorderStyle::Chunky,
            ..DivStyle::default()
        };
        let rect = ClipRect::new(0, 0, 4, 3);
        let clip = ClipBounds::unbounded();
        frame.fill_rect(rect, Background::Blue, None, clip);
        frame.fill_box_background(rect, &style, Background::Red, None, clip);
        frame.stroke_border(rect, &style, Background::Green, None, clip);
        for (y, row) in ["████", "█  █", "████"].iter().enumerate() {
            for (x, character) in row.chars().enumerate() {
                let cell = frame.cell(x, y).unwrap();
                assert_eq!(cell.character, character);
                assert_eq!(cell.background, Background::Red);
            }
        }
    }

    #[test]
    fn chunky_border_respects_individual_border_sides() {
        let mut frame = Frame::new(4, 3, false);
        let style = DivStyle {
            border_left: BorderStyle::Chunky,
            border_bottom: BorderStyle::Chunky,
            ..DivStyle::default()
        };
        frame.stroke_border(
            ClipRect::new(0, 0, 4, 3),
            &style,
            Background::Green,
            None,
            ClipBounds::unbounded(),
        );
        assert_eq!(frame.cell(0, 0).unwrap().character, '█');
        assert_eq!(frame.cell(0, 2).unwrap().character, '█');
        assert_eq!(frame.cell(3, 2).unwrap().character, '█');
        assert_eq!(frame.cell(3, 0).unwrap().character, ' ');
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
                selection_point(frame.selection_units.len()),
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
                selection_point(frame.selection_units.len()),
            );
        }

        let selection = Selection {
            anchor: selection_point(1),
            focus: selection_point(5),
        };

        assert_eq!(frame.selected_text(&selection).as_deref(), Some("bc\nde"));
    }

    #[test]
    fn hyperlinks_close_at_row_boundaries_and_dont_leak_to_plain_cells() {
        let mut frame = Frame::new(3, 2, false);
        let id = taffy::NodeId::from(7usize);
        frame
            .hyperlinks
            .insert(id, crate::hyperlink::encode_href("https://example.com"));
        let style = GlyphStyle {
            hyperlink: Some(id),
            ..Default::default()
        };
        frame.write_grapheme(
            0,
            0,
            GraphemeGlyph {
                character: '\u{26a0}',
                suffix: Some("\u{fe0f}"),
                width: 2,
            },
            style,
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
            None,
        );
        frame.write_glyph(
            0,
            1,
            'x',
            1,
            style,
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );
        let mut bytes = Vec::new();
        frame
            .write_full_to(&mut bytes, TermProfile::NoColor)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();
        assert_eq!(output.matches("https://example.com").count(), 2);
        assert_eq!(output.matches("\x1b]8;;\x1b\\").count(), 2);
        assert!(output.contains("x\x1b]8;;\x1b\\ "), "{output:?}");
        assert_eq!(frame.cell(1, 0).unwrap().hyperlink, Some(id));
        assert_eq!(frame.cell(2, 0).unwrap().hyperlink, None);
    }

    #[test]
    fn hyperlink_metadata_follows_visible_glyphs_through_opacity_and_overpainting() {
        let mut frame = Frame::new(2, 1, false);
        let id = taffy::NodeId::from(7usize);
        frame
            .hyperlinks
            .insert(id, crate::hyperlink::encode_href("https://example.com"));
        frame.write_glyph(
            0,
            0,
            'a',
            1,
            GlyphStyle {
                hyperlink: Some(id),
                ..Default::default()
            },
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );
        let mut layer = frame.layer();
        layer.write_glyph(
            0,
            0,
            'b',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(layer.selection_units.len()),
        );
        frame.composite_layer(layer, 0.5, Background::White, Background::Black);
        assert_eq!(frame.cell(0, 0).unwrap().character, 'b');
        assert_eq!(frame.cell(0, 0).unwrap().hyperlink, None);

        let mut layer = frame.layer();
        layer.write_glyph(
            0,
            0,
            'c',
            1,
            GlyphStyle {
                hyperlink: Some(id),
                ..Default::default()
            },
            ClipBounds::unbounded(),
            selection_point(layer.selection_units.len()),
        );
        frame.composite_layer(layer, 0.5, Background::White, Background::Black);
        assert_eq!(frame.cell(0, 0).unwrap().hyperlink, Some(id));
        frame.write_border_cell(0, 0, '|', Background::White, None, ClipBounds::unbounded());
        assert_eq!(
            frame.cell(0, 0).unwrap().hyperlink,
            None,
            "an unrelated border owns its cell"
        );
        frame.write_glyph(
            0,
            0,
            'c',
            1,
            GlyphStyle {
                hyperlink: Some(id),
                ..Default::default()
            },
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );
        frame.fill_rect(
            ClipRect::new(0, 0, 1, 1),
            Background::Red,
            None,
            ClipBounds::unbounded(),
        );
        assert_eq!(frame.cell(0, 0).unwrap().hyperlink, None);
    }

    #[test]
    fn selection_text_preserves_grapheme_suffixes() {
        let mut frame = Frame::new(2, 1, false);
        frame.write_grapheme(
            0,
            0,
            GraphemeGlyph {
                character: '\u{26a0}',
                suffix: Some("\u{fe0f}"),
                width: 2,
            },
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
            None,
        );
        let selection = Selection {
            anchor: selection_point(0),
            focus: selection_point(0),
        };

        assert_eq!(
            frame.selected_text(&selection).as_deref(),
            Some("\u{26a0}\u{fe0f}")
        );
    }

    #[test]
    fn terminal_output_preserves_the_requested_symbol_presentation() {
        for character in ['⚠', '❤', '♥', '☀', '™', '🏳'] {
            for (suffix, width) in [(None, 1), (Some("\u{fe0e}"), 1), (Some("\u{fe0f}"), 2)] {
                let mut frame = Frame::new(3, 1, false);
                frame.write_grapheme(
                    0,
                    0,
                    GraphemeGlyph {
                        character,
                        suffix,
                        width,
                    },
                    GlyphStyle::default(),
                    ClipBounds::unbounded(),
                    selection_point(frame.selection_units.len()),
                    None,
                );
                let mut bytes = Vec::new();
                frame
                    .write_full_to(&mut bytes, TermProfile::NoColor)
                    .unwrap();
                let rendered = String::from_utf8(bytes).unwrap();

                assert!(
                    rendered.contains(&format!(
                        "{character}{}\x1b[1;{}H",
                        suffix.unwrap_or(""),
                        width + 1
                    )),
                    "{rendered:?}"
                );
            }
        }
    }

    #[test]
    fn selection_point_uses_nearest_selectable_cell_on_row() {
        let mut frame = Frame::new(8, 1, false);
        frame.write_glyph(
            2,
            0,
            'a',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );
        frame.write_glyph(
            5,
            0,
            'b',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );

        assert_eq!(
            frame.selection_point_for(0, 0),
            Some(SelectionPoint {
                source: 0,
                offset: 0
            })
        );
        assert_eq!(
            frame.selection_point_for(7, 0),
            Some(SelectionPoint {
                source: 0,
                offset: 1
            })
        );
    }

    #[test]
    fn selection_ordering_preserves_pointer_targets_and_copied_text() {
        let [a, b, c] = [(7, 0, 'a'), (7, 1, 'b'), (3, 0, 'c')]
            .map(|(source, offset, character)| (SelectionPoint { source, offset }, character));
        for glyphs in [[a, b, c], [c, a, b], [b, a, c]] {
            let mut frame = Frame::new(3, 1, true);
            for (x, (point, character)) in glyphs.into_iter().enumerate() {
                frame.write_glyph(
                    x as i32,
                    0,
                    character,
                    1,
                    GlyphStyle::default(),
                    ClipBounds::unbounded(),
                    point,
                );
            }
            frame.order_selection_units(&HashMap::from([(7, 0), (3, 1)]));
            let selection = Selection {
                anchor: b.0,
                focus: c.0,
            };
            assert_eq!(frame.selected_text(&selection).as_deref(), Some("bc"));
            frame.apply_selection(Some(&selection));
            for (x, (point, character)) in glyphs.into_iter().enumerate() {
                assert_eq!(frame.selection_point_for(x as u32, 0), Some(point));
                assert_eq!(frame.cell(x, 0).unwrap().reversed, character != 'a');
            }
        }
    }

    #[test]
    fn selection_endpoints_keep_nearest_offset_and_first_tie() {
        let mut frame = Frame::new(3, 1, true);
        for (x, (offset, character)) in [(2, 'a'), (4, 'b'), (8, 'c')].into_iter().enumerate() {
            frame.write_glyph(
                x as i32,
                0,
                character,
                1,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
                selection_point(offset),
            );
        }
        for (anchor, focus, expected) in [(3, 7, "abc"), (4, 4, "b"), (usize::MAX, 8, "c")] {
            assert_eq!(
                frame
                    .selected_text(&Selection {
                        anchor: selection_point(anchor),
                        focus: selection_point(focus),
                    })
                    .as_deref(),
                Some(expected),
            );
        }
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
                selection_point(frame.selection_units.len()),
            );
        }

        frame.apply_selection(Some(&Selection {
            anchor: selection_point(1),
            focus: selection_point(2),
        }));

        assert!(!frame.cell(0, 0).unwrap().reversed);
        assert!(frame.cell(1, 0).unwrap().reversed);
        assert!(frame.cell(2, 0).unwrap().reversed);
    }

    #[test]
    fn diff_writes_only_changed_spans() {
        let previous = Frame::new(4, 2, false);
        let mut next = Frame::new(4, 2, false);
        next.write_glyph(
            2,
            1,
            'x',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(next.selection_units.len()),
        );

        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("\x1b[2;3Hx"));
        assert!(!output.contains("\x1b[H"));
        assert!(!output.contains("\x1b[2J"));
    }

    #[test]
    fn diff_writes_when_only_grapheme_suffix_changes() {
        let mut previous = Frame::new(1, 1, false);
        previous.write_grapheme(
            0,
            0,
            GraphemeGlyph {
                character: 'e',
                suffix: Some("\u{301}"),
                width: 1,
            },
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(previous.selection_units.len()),
            None,
        );
        let mut next = Frame::new(1, 1, false);
        next.write_grapheme(
            0,
            0,
            GraphemeGlyph {
                character: 'e',
                suffix: Some("\u{308}"),
                width: 1,
            },
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(next.selection_units.len()),
            None,
        );

        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
            .unwrap();
        let output = String::from_utf8(bytes).unwrap();

        assert!(output.contains("e\u{308}"));
        assert!(!output.contains("e\u{301}"));
    }

    #[test]
    fn changing_an_emoji_repaints_the_following_border_at_its_column() {
        for (character, suffix) in [('👍', "\u{1f3fd}"), ('👩', "\u{200d}\u{1f4bb}")] {
            let mut previous = Frame::new(6, 1, false);
            previous.write_glyph(
                0,
                0,
                '│',
                1,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
                selection_point(previous.selection_units.len()),
            );
            previous.write_glyph(
                1,
                0,
                '🐙',
                2,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
                selection_point(previous.selection_units.len()),
            );
            previous.write_glyph(
                3,
                0,
                'x',
                1,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
                selection_point(previous.selection_units.len()),
            );
            previous.write_glyph(
                4,
                0,
                '│',
                1,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
                selection_point(previous.selection_units.len()),
            );
            let mut next = previous.clone();
            next.write_grapheme(
                1,
                0,
                GraphemeGlyph {
                    character,
                    suffix: Some(suffix),
                    width: 2,
                },
                GlyphStyle::default(),
                ClipBounds::unbounded(),
                selection_point(next.selection_units.len()),
                None,
            );
            let mut bytes = Vec::new();
            next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
                .unwrap();
            let output = String::from_utf8(bytes).unwrap();

            assert!(
                output.contains(&format!("{character}{suffix}\x1b[1;4Hx│")),
                "{output:?}"
            );
            assert!(!output.contains('\u{fe0f}'), "{output:?}");
        }
    }

    #[test]
    fn diff_starting_in_a_continuation_cell_positions_the_following_text() {
        let mut previous = Frame::new(3, 1, false);
        previous.write_glyph(
            0,
            0,
            '🐙',
            2,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(previous.selection_units.len()),
        );
        previous.write_glyph(
            2,
            0,
            'x',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(previous.selection_units.len()),
        );
        let mut next = previous.clone();
        next.cells[1].background = Background::Red;
        next.write_glyph(
            2,
            0,
            'y',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(next.selection_units.len()),
        );
        let mut bytes = Vec::new();
        next.write_diff_to(&mut bytes, Some(&previous), TermProfile::NoColor, false)
            .unwrap();

        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("\x1b[1;3Hy"), "{output:?}");
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
            selection_point(frame.selection_units.len()),
        );
        frame.write_glyph(
            1,
            0,
            'b',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );

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
        previous.write_glyph(
            0,
            0,
            'x',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(previous.selection_units.len()),
        );
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
            selection_point(next.selection_units.len()),
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
        next.write_glyph(
            3,
            0,
            'x',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(next.selection_units.len()),
        );

        for synchronized in [false, true] {
            let mut bytes = Vec::new();
            next.write_diff_to(&mut bytes, None, TermProfile::NoColor, synchronized)
                .unwrap();
            let output = String::from_utf8(bytes).unwrap();

            assert!(output.contains("\x1b[?7l"));
            assert!(output.contains("\x1b[?7h"));
            assert!(output.find("\x1b[?7l") < output.find("x"));
            assert!(output.find("x") < output.find("\x1b[?7h"));
        }
    }

    #[test]
    fn full_frame_write_skips_trailing_empty_rows() {
        let mut frame = Frame::new(4, 4, false);
        frame.write_glyph(
            0,
            1,
            'x',
            1,
            GlyphStyle::default(),
            ClipBounds::unbounded(),
            selection_point(frame.selection_units.len()),
        );

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
