use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::style::CssWhiteSpace;

pub(crate) const TAB_SIZE_CELLS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalGlyph {
    Character(char),
    Spaces(usize),
}

pub(crate) struct ParsedTextWithSourceMap {
    pub(crate) characters: Vec<char>,
    pub(crate) source_to_parsed_cursor: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextGrapheme<'a> {
    pub(crate) text: &'a str,
    pub(crate) character_start: usize,
    pub(crate) character_end: usize,
    pub(crate) width: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GraphemeSuffix(String);

impl GraphemeSuffix {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn text_graphemes(text: &str) -> impl Iterator<Item = TextGrapheme<'_>> {
    let mut character_index = 0;
    text.graphemes(true).map(move |grapheme| {
        let character_start = character_index;
        character_index += grapheme.chars().count();
        TextGrapheme {
            text: grapheme,
            character_start,
            character_end: character_index,
            width: grapheme_cell_width(grapheme),
        }
    })
}

pub(crate) fn text_cell_width(text: &str) -> usize {
    text_graphemes(text).map(|grapheme| grapheme.width).sum()
}

fn grapheme_cell_width(grapheme: &str) -> usize {
    let base = grapheme.chars().next().expect("graphemes are non-empty");
    if uses_text_presentation(base, &grapheme[base.len_utf8()..]) {
        1
    } else {
        UnicodeWidthStr::width(grapheme)
    }
}

pub(crate) fn uses_text_presentation(character: char, suffix: &str) -> bool {
    if character.is_ascii()
        || !matches!(suffix, "" | "\u{fe0e}" | "\u{fe0f}")
        || UnicodeWidthChar::width(character) != Some(1)
    {
        return false;
    }

    let mut bytes = [0; 7];
    let base_len = character.encode_utf8(&mut bytes).len();
    let selector_len = '\u{fe0f}'.encode_utf8(&mut bytes[base_len..]).len();
    let emoji = std::str::from_utf8(&bytes[..base_len + selector_len])
        .expect("characters were encoded as UTF-8");
    UnicodeWidthStr::width(emoji) == 2
}

pub(crate) fn character_cell_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0; text.chars().count() + 1];
    let mut col = 0;
    for grapheme in text_graphemes(text) {
        offsets[grapheme.character_start..grapheme.character_end].fill(col);
        col += grapheme.width;
        offsets[grapheme.character_end] = col;
    }
    offsets
}

pub(crate) fn cell_offset_for_character(text: &str, character_offset: usize) -> usize {
    text_graphemes(text)
        .take_while(|grapheme| grapheme.character_end <= character_offset)
        .map(|grapheme| grapheme.width)
        .sum()
}

pub(crate) fn character_offset_for_cell(text: &str, cell_offset: usize) -> usize {
    let mut col = 0;
    for grapheme in text_graphemes(text) {
        let end = col + grapheme.width;
        if cell_offset < end {
            return if (cell_offset - col) * 2 >= grapheme.width {
                grapheme.character_end
            } else {
                grapheme.character_start
            };
        }
        col = end;
    }
    text.chars().count()
}

pub(crate) fn grapheme_boundary_at_or_after_cell(text: &str, cell_offset: usize) -> usize {
    let mut col = 0;
    for grapheme in text_graphemes(text) {
        let end = col + grapheme.width;
        if cell_offset > col && cell_offset < end {
            return end;
        }
        col = end;
    }
    cell_offset
}

pub(crate) fn grapheme_parts(grapheme: &str) -> (char, Option<Box<GraphemeSuffix>>) {
    let mut characters = grapheme.chars();
    let character = characters.next().expect("graphemes are non-empty");
    let suffix = characters.collect::<String>();
    let suffix = (!suffix.is_empty()).then(|| Box::new(GraphemeSuffix::new(suffix)));
    (character, suffix)
}

pub(crate) fn parse_text_for_white_space(text: &str, white_space: CssWhiteSpace) -> Vec<char> {
    match white_space {
        CssWhiteSpace::Pre | CssWhiteSpace::PreWrap => preserve_white_space(text),
        CssWhiteSpace::Normal | CssWhiteSpace::NoWrap => collapse_white_space(text, false),
        CssWhiteSpace::PreLine => collapse_white_space(text, true),
    }
}

pub(crate) fn parse_text_for_single_line(text: &str) -> Vec<char> {
    parse_text_for_white_space(text, CssWhiteSpace::Pre)
        .into_iter()
        .map(|character| if character == '\n' { ' ' } else { character })
        .collect()
}

pub(crate) fn parse_text_for_pre_wrap_with_source_map(text: &str) -> ParsedTextWithSourceMap {
    let source = text.chars().collect::<Vec<_>>();
    let mut characters = Vec::new();
    let mut source_to_parsed_cursor = vec![0; source.len() + 1];
    let mut source_index = 0;

    while source_index < source.len() {
        source_to_parsed_cursor[source_index] = characters.len();
        let character = source[source_index];
        let source_end = if character == '\r' && source.get(source_index + 1) == Some(&'\n') {
            source_to_parsed_cursor[source_index + 1] = characters.len();
            source_index + 2
        } else {
            source_index + 1
        };
        let normalized = if character == '\r' { '\n' } else { character };
        match normalized {
            '\t' => characters.extend(std::iter::repeat_n(' ', TAB_SIZE_CELLS)),
            '\n' => characters.push('\n'),
            '\u{000c}' => characters.push(' '),
            character => push_safe_text_character(&mut characters, character),
        }
        source_index = source_end;
        source_to_parsed_cursor[source_index] = characters.len();
    }

    ParsedTextWithSourceMap {
        characters,
        source_to_parsed_cursor,
    }
}

pub(crate) fn terminal_safe_glyph(character: char, cell_width: usize) -> TerminalGlyph {
    match character {
        '\t' | '\n' | '\r' => TerminalGlyph::Spaces(cell_width.max(1)),
        '\0' => TerminalGlyph::Character('\u{fffd}'),
        character if is_c0_control(character) => {
            TerminalGlyph::Character(control_picture(character).unwrap_or('\u{fffd}'))
        }
        character if is_c1_control(character) => TerminalGlyph::Character('\u{fffd}'),
        character => TerminalGlyph::Character(character),
    }
}

fn preserve_white_space(text: &str) -> Vec<char> {
    parse_text_for_pre_wrap_with_source_map(text).characters
}

fn collapse_white_space(text: &str, preserve_newlines: bool) -> Vec<char> {
    let mut chars = Vec::new();
    let mut pending_space = false;
    for character in normalized_line_break_chars(text) {
        if preserve_newlines && character == '\n' {
            chars.push('\n');
            pending_space = false;
            continue;
        }
        if is_css_collapsible_white_space(character) || character == '\n' {
            if !pending_space {
                chars.push(' ');
                pending_space = true;
            }
            continue;
        }
        push_safe_text_character(&mut chars, character);
        pending_space = false;
    }
    chars
}

fn normalized_line_break_chars(text: &str) -> Vec<char> {
    let mut chars = Vec::with_capacity(text.len());
    let mut input = text.chars().peekable();
    while let Some(character) = input.next() {
        if character == '\r' {
            if input.peek() == Some(&'\n') {
                input.next();
            }
            chars.push('\n');
        } else {
            chars.push(character);
        }
    }
    chars
}

fn push_safe_text_character(chars: &mut Vec<char>, character: char) {
    match terminal_safe_glyph(character, 1) {
        TerminalGlyph::Character(character) => chars.push(character),
        TerminalGlyph::Spaces(width) => chars.extend(std::iter::repeat_n(' ', width)),
    }
}

fn is_css_collapsible_white_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\u{000c}')
}

fn is_c0_control(character: char) -> bool {
    (character as u32) <= 0x1f || character == '\u{007f}'
}

fn is_c1_control(character: char) -> bool {
    matches!(character as u32, 0x80..=0x9f)
}

fn control_picture(character: char) -> Option<char> {
    let codepoint = character as u32;
    if codepoint <= 0x1f {
        char::from_u32(0x2400 + codepoint)
    } else if character == '\u{007f}' {
        Some('\u{2421}')
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_white_space_collapses_tabs_and_line_breaks() {
        let chars = parse_text_for_white_space("a\t \r\n b", CssWhiteSpace::Normal);

        assert_eq!(chars.into_iter().collect::<String>(), "a b");
    }

    #[test]
    fn pre_white_space_expands_tabs_and_normalizes_line_breaks() {
        let chars = parse_text_for_white_space("a\tb\r\nc\rd", CssWhiteSpace::Pre);

        assert_eq!(chars.into_iter().collect::<String>(), "a    b\nc\nd");
    }

    #[test]
    fn text_controls_are_rendered_as_safe_visible_characters() {
        let chars = parse_text_for_white_space("\0\x1b]2;title\x07", CssWhiteSpace::Pre);

        assert_eq!(
            chars.into_iter().collect::<String>(),
            "\u{fffd}\u{241b}]2;title\u{2407}"
        );
    }

    #[test]
    fn c1_controls_render_as_replacement_characters() {
        let chars = parse_text_for_white_space("a\u{0085}b", CssWhiteSpace::Pre);

        assert_eq!(chars.into_iter().collect::<String>(), "a\u{fffd}b");
    }

    #[test]
    fn text_presentation_symbols_measure_one_cell_with_either_selector() {
        for symbol in ["⚠", "❤", "♥", "©", "®", "™", "☀", "☘", "▶", "↔", "🏳"] {
            for selector in ["", "\u{fe0e}", "\u{fe0f}"] {
                let text = format!("{symbol}{selector}");
                assert_eq!(text_cell_width(&text), 1, "{text:?}");
                assert_eq!(
                    cell_offset_for_character(&text, text.chars().count()),
                    1,
                    "{text:?}"
                );
            }
        }
    }

    #[test]
    fn compound_emoji_keep_their_grapheme_widths() {
        for text in [
            "🐙",
            "🛑",
            "👍🏽",
            "👩‍💻",
            "👩🏽‍🚀",
            "❤️‍🔥",
            "🏳️‍🌈",
            "👨‍👩‍👧‍👦",
            "🇺🇸",
            "1️⃣",
            "#️⃣",
            "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
        ] {
            assert_eq!(text_cell_width(text), 2, "{text:?}");
            assert_eq!(text_graphemes(text).count(), 1, "{text:?}");
        }
    }

    #[test]
    fn mixed_emoji_offsets_follow_the_rendered_widths() {
        let text = "⚠🐙❤️👩‍💻👍🏽";
        assert_eq!(text_cell_width(text), 8);
        assert_eq!(
            character_cell_offsets(text),
            vec![0, 1, 3, 3, 4, 4, 4, 6, 6, 8]
        );
        assert_eq!(character_offset_for_cell(text, 6), 7);
        assert_eq!(character_offset_for_cell(text, 8), 9);
    }

    #[test]
    fn ordinary_text_does_not_request_emoji_presentation() {
        for character in ['a', '1', '#', '*', 'é', '界', '─', '🐙'] {
            assert!(!uses_text_presentation(character, ""));
        }
        assert_eq!(text_cell_width("1#*e\u{301}界"), 6);
    }

    #[test]
    fn joined_emoji_is_one_two_cell_grapheme() {
        let text = "\u{1f469}\u{200d}\u{1f4bb}x";
        let graphemes = text_graphemes(text).collect::<Vec<_>>();

        assert_eq!(graphemes.len(), 2);
        assert_eq!(graphemes[0].text, "\u{1f469}\u{200d}\u{1f4bb}");
        assert_eq!(graphemes[0].character_start, 0);
        assert_eq!(graphemes[0].character_end, 3);
        assert_eq!(graphemes[0].width, 2);
        assert_eq!(graphemes[1].character_start, 3);
        assert_eq!(character_cell_offsets(text), vec![0, 0, 0, 2, 3]);
        assert_eq!(cell_offset_for_character(text, 3), 2);
        assert_eq!(character_offset_for_cell(text, 0), 0);
        assert_eq!(character_offset_for_cell(text, 1), 3);
        assert_eq!(character_offset_for_cell(text, 2), 3);
        assert_eq!(grapheme_boundary_at_or_after_cell(text, 1), 2);
        assert_eq!(grapheme_boundary_at_or_after_cell(text, 2), 2);
    }

    #[test]
    fn pre_wrap_source_map_preserves_original_character_offsets() {
        let parsed = parse_text_for_pre_wrap_with_source_map("a\tb\r\nc");

        assert_eq!(
            parsed.characters.into_iter().collect::<String>(),
            "a    b\nc"
        );
        assert_eq!(parsed.source_to_parsed_cursor, vec![0, 1, 5, 6, 6, 7, 8]);
    }
}
