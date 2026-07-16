use unicode_linebreak::{break_property, linebreaks, BreakClass, BreakOpportunity};
use unicode_segmentation::UnicodeSegmentation;

use crate::style::CssWordBreak;

pub(crate) struct LineBreakPlan {
    soft_before: Vec<bool>,
    grapheme_before: Vec<bool>,
}

impl LineBreakPlan {
    pub(crate) fn new(characters: &[char], word_break: CssWordBreak) -> Self {
        let text = characters.iter().collect::<String>();
        let byte_to_character = byte_to_character_indices(&text);
        let mut grapheme_before = vec![false; characters.len() + 1];
        for (byte_index, _) in text.grapheme_indices(true) {
            if let Some(character_index) = byte_to_character[byte_index] {
                grapheme_before[character_index] = true;
            }
        }
        grapheme_before[characters.len()] = true;

        let mut soft_before = vec![false; characters.len() + 1];
        for (byte_index, opportunity) in linebreaks(&text) {
            if !matches!(
                opportunity,
                BreakOpportunity::Allowed | BreakOpportunity::Mandatory
            ) {
                continue;
            }
            let Some(character_index) = byte_to_character[byte_index] else {
                continue;
            };
            soft_before[character_index] = true;
        }

        match word_break {
            CssWordBreak::BreakAll => soft_before.clone_from(&grapheme_before),
            CssWordBreak::KeepAll => {
                for index in 1..characters.len() {
                    if is_keep_all_unit(characters[index - 1])
                        && is_keep_all_unit(characters[index])
                    {
                        soft_before[index] = false;
                    }
                }
            }
            CssWordBreak::Inherit | CssWordBreak::Normal | CssWordBreak::BreakWord => {}
        }

        soft_before[0] = false;
        Self {
            soft_before,
            grapheme_before,
        }
    }

    pub(crate) fn is_soft_break_before(&self, index: usize) -> bool {
        self.soft_before.get(index).copied().unwrap_or(false)
    }

    pub(crate) fn is_grapheme_break_before(&self, index: usize) -> bool {
        self.grapheme_before.get(index).copied().unwrap_or(false)
    }

    pub(crate) fn next_soft_break(&self, characters: &[char], start: usize) -> usize {
        ((start + 1)..characters.len())
            .find(|index| {
                self.is_soft_break_before(*index) || matches!(characters[*index], '\r' | '\n')
            })
            .unwrap_or(characters.len())
    }
}

fn byte_to_character_indices(text: &str) -> Vec<Option<usize>> {
    let mut indices = vec![None; text.len() + 1];
    for (character_index, (byte_index, _)) in text.char_indices().enumerate() {
        indices[byte_index] = Some(character_index);
    }
    indices[text.len()] = Some(text.chars().count());
    indices
}

fn is_keep_all_unit(character: char) -> bool {
    matches!(
        break_property(character as u32),
        BreakClass::Alphabetic
            | BreakClass::Ambiguous
            | BreakClass::HebrewLetter
            | BreakClass::Ideographic
            | BreakClass::Numeric
            | BreakClass::HangulLJamo
            | BreakClass::HangulVJamo
            | BreakClass::HangulTJamo
            | BreakClass::HangulLvSyllable
            | BreakClass::HangulLvtSyllable
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_uses_unicode_breaks_and_keep_all_suppresses_cjk_breaks() {
        let hyphenated = "ab-cd".chars().collect::<Vec<_>>();
        let normal = LineBreakPlan::new(&hyphenated, CssWordBreak::Normal);
        assert!(normal.is_soft_break_before(3));

        let cjk = "你好".chars().collect::<Vec<_>>();
        assert!(LineBreakPlan::new(&cjk, CssWordBreak::Normal).is_soft_break_before(1));
        assert!(!LineBreakPlan::new(&cjk, CssWordBreak::KeepAll).is_soft_break_before(1));
    }

    #[test]
    fn break_all_uses_grapheme_boundaries() {
        let characters = "a\u{0301}b".chars().collect::<Vec<_>>();
        let plan = LineBreakPlan::new(&characters, CssWordBreak::BreakAll);

        assert!(!plan.is_soft_break_before(1));
        assert!(plan.is_soft_break_before(2));
    }
}
