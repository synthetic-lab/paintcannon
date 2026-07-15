use unicode_width::UnicodeWidthChar;

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

pub(crate) fn character_cell_width(character: char) -> usize {
    UnicodeWidthChar::width(character).unwrap_or(0)
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
    fn pre_wrap_source_map_preserves_original_character_offsets() {
        let parsed = parse_text_for_pre_wrap_with_source_map("a\tb\r\nc");

        assert_eq!(
            parsed.characters.into_iter().collect::<String>(),
            "a    b\nc"
        );
        assert_eq!(parsed.source_to_parsed_cursor, vec![0, 1, 5, 6, 6, 7, 8]);
    }
}
