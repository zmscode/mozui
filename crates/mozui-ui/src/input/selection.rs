use std::{char, ops::Range};

use mozui::{Context, Window};
use ropey::Rope;
use sum_tree::Bias;

use crate::{RopeExt as _, input::InputState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharType {
    /// a-z, A-Z, 0-9, _
    Word,
    /// '\t', ' ', '\u{00A0}' etc.
    Whitespace,
    /// \n, \r
    Newline,
    /// . , ; : ( ) [ ] { } ... or CJK characters: `汉`, `🎉` etc.
    Other,
}

/// Implementation based on <https://github.com/zed-industries/zed/blob/main/crates/gpui/src/text_system/line_wrapper.rs>
fn is_word_char(c: char) -> bool {
    matches!(c, '_' ) ||
    // ASCII alphanumeric characters, for English, numbers: `Hello123`, etc.
    c.is_ascii_alphanumeric() ||
    // Latin script in Unicode for French, German, Spanish, etc.
    // Latin-1 Supplement
    // https://en.wikipedia.org/wiki/Latin-1_Supplement
    matches!(c, '\u{00C0}'..='\u{00FF}') ||
    // Latin Extended-A
    // https://en.wikipedia.org/wiki/Latin_Extended-A
    matches!(c, '\u{0100}'..='\u{017F}') ||
    // Latin Extended-B
    // https://en.wikipedia.org/wiki/Latin_Extended-B
    matches!(c, '\u{0180}'..='\u{024F}') ||
    // Cyrillic for Russian, Ukrainian, etc.
    // https://en.wikipedia.org/wiki/Cyrillic_script_in_Unicode
    matches!(c, '\u{0400}'..='\u{04FF}') ||

    // Vietnamese (https://vietunicode.sourceforge.net/charset/)
    matches!(c, '\u{1E00}'..='\u{1EFF}') || // Latin Extended Additional
    matches!(c, '\u{0300}'..='\u{036F}') // Combining Diacritical Marks
}

impl From<char> for CharType {
    fn from(c: char) -> Self {
        match c {
            c if is_word_char(c) => CharType::Word,
            c if c == '\n' || c == '\r' => CharType::Newline,
            c if c.is_whitespace() => CharType::Whitespace,
            _ => CharType::Other,
        }
    }
}

impl CharType {
    /// Check if two CharTypes are connectable
    fn is_connectable(self, c: char) -> bool {
        let other = CharType::from(c);
        match (self, other) {
            (CharType::Word, CharType::Word) => true,
            (CharType::Whitespace, CharType::Whitespace) => true,
            _ => false,
        }
    }
}

impl InputState {
    /// Select the word at the given offset on double-click.
    ///
    /// The offset is the UTF-8 offset.
    pub(super) fn select_word(&mut self, offset: usize, _: &mut Window, cx: &mut Context<Self>) {
        let Some(range) = TextSelector::word_range(&self.text, offset) else {
            return;
        };

        self.selected_range = (range.start..range.end).into();
        self.selected_word_range = Some(self.selected_range);
        cx.notify()
    }

    /// Select the line at the given offset on triple-click.
    ///
    /// The offset is the UTF-8 offset.
    pub(super) fn select_line(&mut self, offset: usize, _: &mut Window, cx: &mut Context<Self>) {
        let range = TextSelector::line_range(&self.text, offset);
        self.selected_range = (range.start..range.end).into();
        self.selected_word_range = None;
        cx.notify()
    }
}

struct TextSelector;
impl TextSelector {
    /// Select a line in the given text at the specified offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// Returns the start and end offsets of the selected line.
    pub fn line_range(text: &Rope, offset: usize) -> Range<usize> {
        let offset = text.clip_offset(offset, Bias::Left);
        let row = text.offset_to_point(offset).row;
        let start = text.line_start_offset(row);
        let end = text.line_end_offset(row);

        start..end
    }

    /// Select a word in the given text at the specified offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// Returns the start and end offsets of the selected word.
    pub fn word_range(text: &Rope, offset: usize) -> Option<Range<usize>> {
        let offset = text.clip_offset(offset, Bias::Left);
        let Some(char) = text.char_at(offset) else {
            return None;
        };

        let char_type = CharType::from(char);
        let mut start = offset;
        let mut end = offset + char.len_utf8();
        let prev_chars = text.chars_at(start).reversed().take(128);
        let next_chars = text.chars_at(end).take(128);

        for ch in prev_chars {
            if char_type.is_connectable(ch) {
                start -= ch.len_utf8();
            } else {
                break;
            }
        }

        for ch in next_chars {
            if char_type.is_connectable(ch) {
                end += ch.len_utf8();
            } else {
                break;
            }
        }

        Some(start..end)
    }
}
