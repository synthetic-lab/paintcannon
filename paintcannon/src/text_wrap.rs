use std::ops::Range;

use crate::text::{character_cell_width, parse_text_for_pre_wrap_with_source_map};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WrappedText {
    pub(crate) glyphs: Vec<TextGlyph>,
    cursor_positions: Vec<(usize, usize)>,
    visual_line_ranges: Vec<Range<usize>>,
    end_position: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextGlyph {
    pub(crate) character: char,
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) width: usize,
}

impl WrappedText {
    pub(crate) fn new(text: &str, wrap_width: usize) -> Self {
        let wrap_width = wrap_width.max(1);
        let parsed = parse_text_for_pre_wrap_with_source_map(text);
        let chars = &parsed.characters;
        let mut glyphs = Vec::new();
        let mut parsed_cursor_positions = Vec::with_capacity(chars.len() + 1);
        let mut row = 0;
        let mut col = 0;
        let mut index = 0;

        while index < chars.len() {
            let character = chars[index];
            if character == '\r' {
                parsed_cursor_positions.push(normalize_cursor_position(row, col, wrap_width));
                index += 1;
                continue;
            }
            if character == '\n' {
                parsed_cursor_positions.push(normalize_cursor_position(row, col, wrap_width));
                row += 1;
                col = 0;
                index += 1;
                continue;
            }
            if is_word_start(chars, index) {
                let word_end = next_word_end(chars, index);
                let word_width = text_width(&chars[index..word_end]);
                if word_width <= wrap_width && col > 0 && col + word_width > wrap_width {
                    row += 1;
                    col = 0;
                }
            }
            let width = character_cell_width(character);
            if col > 0 && width > 0 && col + width > wrap_width {
                row += 1;
                col = 0;
                if character == ' ' || character == '\t' {
                    parsed_cursor_positions.push((row, col));
                    index += 1;
                    continue;
                }
            }
            parsed_cursor_positions.push(normalize_cursor_position(row, col, wrap_width));
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
        let end_position = normalize_cursor_position(row, col, wrap_width);
        parsed_cursor_positions.push(end_position);
        let cursor_positions = parsed
            .source_to_parsed_cursor
            .iter()
            .map(|parsed_cursor| parsed_cursor_positions[*parsed_cursor])
            .collect::<Vec<_>>();
        let visual_line_ranges = visual_line_ranges(
            chars,
            &parsed.source_to_parsed_cursor,
            &parsed_cursor_positions,
            &cursor_positions,
            end_position.0 + 1,
        );

        Self {
            glyphs,
            cursor_positions,
            visual_line_ranges,
            end_position,
        }
    }

    pub(crate) fn cursor_position(&self, cursor: usize) -> (usize, usize) {
        self.cursor_positions
            .get(cursor)
            .copied()
            .unwrap_or(self.end_position)
    }

    pub(crate) fn row_count(&self) -> usize {
        self.end_position.0 + 1
    }

    pub(crate) fn max_line_width(&self) -> usize {
        self.glyphs
            .iter()
            .map(|glyph| glyph.col + glyph.width)
            .max()
            .unwrap_or(1)
    }

    pub(crate) fn visual_line_range(&self, row: usize) -> Option<Range<usize>> {
        self.visual_line_ranges.get(row).cloned()
    }

    pub(crate) fn cursor_after_vertical_move(&self, cursor: usize, direction: i32) -> usize {
        if direction == 0 {
            return cursor.min(self.cursor_positions.len().saturating_sub(1));
        }

        let cursor = cursor.min(self.cursor_positions.len().saturating_sub(1));
        let (row, col) = self.cursor_position(cursor);
        let Some(target_row) = row.checked_add_signed(direction as isize) else {
            return cursor;
        };
        self.cursor_for_visual_position(target_row, col)
            .unwrap_or(cursor)
    }

    pub(crate) fn cursor_for_visual_position(&self, row: usize, col: usize) -> Option<usize> {
        self.cursor_positions
            .iter()
            .enumerate()
            .filter(|(_, (candidate_row, _))| *candidate_row == row)
            .min_by_key(|(_, (_, candidate_col))| {
                (
                    candidate_col.abs_diff(col),
                    if *candidate_col < col { 1 } else { 0 },
                )
            })
            .map(|(index, _)| index)
    }
}

fn visual_line_ranges(
    characters: &[char],
    source_to_parsed_cursor: &[usize],
    parsed_cursor_positions: &[(usize, usize)],
    source_cursor_positions: &[(usize, usize)],
    row_count: usize,
) -> Vec<Range<usize>> {
    let mut ranges = vec![None::<Range<usize>>; row_count];
    for source_index in 0..source_to_parsed_cursor.len().saturating_sub(1) {
        let parsed_start = source_to_parsed_cursor[source_index];
        let parsed_end = source_to_parsed_cursor[source_index + 1];
        for parsed_index in parsed_start..parsed_end {
            if matches!(characters[parsed_index], '\n' | '\r') {
                continue;
            }
            let row = parsed_cursor_positions[parsed_index].0;
            let range = &mut ranges[row];
            match range {
                Some(range) => {
                    range.start = range.start.min(source_index);
                    range.end = range.end.max(source_index + 1);
                }
                None => *range = Some(source_index..source_index + 1),
            }
        }
    }

    ranges
        .into_iter()
        .enumerate()
        .map(|(row, range)| {
            range.unwrap_or_else(|| {
                let offset = source_cursor_positions
                    .iter()
                    .position(|position| position.0 == row)
                    .unwrap_or(source_cursor_positions.len().saturating_sub(1));
                offset..offset
            })
        })
        .collect()
}

fn normalize_cursor_position(row: usize, col: usize, wrap_width: usize) -> (usize, usize) {
    if col >= wrap_width {
        (row + 1, 0)
    } else {
        (row, col)
    }
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

fn text_width(chars: &[char]) -> usize {
    chars
        .iter()
        .map(|character| character_cell_width(*character))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_move_uses_soft_wrapped_visual_rows() {
        let layout = WrappedText::new("abcd efgh", 6);

        assert_eq!(layout.cursor_position(5), (1, 0));
        assert_eq!(layout.cursor_position(7), (1, 2));
        assert_eq!(layout.cursor_after_vertical_move(7, -1), 2);
        assert_eq!(layout.cursor_after_vertical_move(2, 1), 7);
    }

    #[test]
    fn long_unbroken_word_wraps_at_width() {
        let layout = WrappedText::new("hahahaha", 4);
        let row_text = |row| {
            layout
                .glyphs
                .iter()
                .filter(|glyph| glyph.row == row)
                .map(|glyph| glyph.character)
                .collect::<String>()
        };

        assert_eq!(row_text(0), "haha");
        assert_eq!(row_text(1), "haha");
        assert_eq!(layout.cursor_position(1), (0, 1));
        assert_eq!(layout.cursor_position(4), (1, 0));
        assert_eq!(layout.cursor_position(5), (1, 1));
        assert_eq!(layout.row_count(), 3);
    }

    #[test]
    fn vertical_move_stays_put_at_visual_edges() {
        let layout = WrappedText::new("abc def", 4);

        assert_eq!(layout.cursor_after_vertical_move(1, -1), 1);
        assert_eq!(layout.cursor_after_vertical_move(6, 1), 6);
    }

    #[test]
    fn visual_line_ranges_follow_soft_wraps_and_exclude_line_breaks() {
        let wrapped = WrappedText::new("abcd efgh", 6);
        assert_eq!(wrapped.visual_line_range(0), Some(0..5));
        assert_eq!(wrapped.visual_line_range(1), Some(5..9));
        assert_eq!(wrapped.visual_line_range(2), None);

        let explicit_lines = WrappedText::new("a\n\nb", 10);
        assert_eq!(explicit_lines.visual_line_range(0), Some(0..1));
        assert_eq!(explicit_lines.visual_line_range(1), Some(2..2));
        assert_eq!(explicit_lines.visual_line_range(2), Some(3..4));
    }

    #[test]
    fn exact_wrap_boundary_has_an_empty_final_visual_line() {
        let wrapped = WrappedText::new("haha", 4);

        assert_eq!(wrapped.visual_line_range(0), Some(0..4));
        assert_eq!(wrapped.visual_line_range(1), Some(4..4));
    }

    #[test]
    fn cursor_positions_and_ranges_use_original_offsets_after_text_normalization() {
        let tabbed = WrappedText::new("a\tb", 8);
        assert_eq!(tabbed.cursor_position(2), (0, 5));
        assert_eq!(tabbed.visual_line_range(0), Some(0..3));

        let crlf = WrappedText::new("a\r\nb", 8);
        assert_eq!(crlf.cursor_position(1), (0, 1));
        assert_eq!(crlf.cursor_position(2), (0, 1));
        assert_eq!(crlf.cursor_position(3), (1, 0));
        assert_eq!(crlf.visual_line_range(0), Some(0..1));
        assert_eq!(crlf.visual_line_range(1), Some(3..4));

        let wide = WrappedText::new("a界b", 3);
        assert_eq!(wide.visual_line_range(0), Some(0..2));
        assert_eq!(wide.visual_line_range(1), Some(2..3));
    }
}
