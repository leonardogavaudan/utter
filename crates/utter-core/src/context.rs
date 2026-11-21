use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_CHARS_BEFORE_CURSOR: usize = 400;
pub const MAX_CHARS_AFTER_CURSOR: usize = 200;
pub const MAX_SELECTED_CHARS: usize = 2_000;
pub const MAX_WINDOW_TITLE_CHARS: usize = 200;
pub const MAX_SCREEN_CHARS: usize = 2_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ContextMode {
    Off,
    AppOnly,
    Cursor,
    #[default]
    Screen,
}

impl ContextMode {
    pub fn reads_app(self) -> bool {
        self != ContextMode::Off
    }

    pub fn reads_text(self) -> bool {
        matches!(self, ContextMode::Cursor | ContextMode::Screen)
    }

    pub fn reads_screen(self) -> bool {
        self == ContextMode::Screen
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TextContext {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub text_before_cursor: Option<String>,
    pub selected_text: Option<String>,
    pub text_after_cursor: Option<String>,
    pub screen_text: Option<String>,
}

impl TextContext {
    pub fn is_empty(&self) -> bool {
        [
            &self.app_name,
            &self.window_title,
            &self.text_before_cursor,
            &self.selected_text,
            &self.text_after_cursor,
            &self.screen_text,
        ]
        .iter()
        .all(|field| field.is_none())
    }

    pub fn restricted_to(self, mode: ContextMode) -> Self {
        match mode {
            ContextMode::Off => TextContext::default(),
            ContextMode::AppOnly => TextContext {
                app_name: self.app_name,
                window_title: self.window_title,
                ..TextContext::default()
            },
            ContextMode::Cursor => TextContext {
                screen_text: None,
                ..self
            },
            ContextMode::Screen => self,
        }
    }

    pub fn normalized(self) -> Self {
        TextContext {
            app_name: non_blank(self.app_name),
            window_title: non_blank(self.window_title)
                .map(|title| head_chars(&title, MAX_WINDOW_TITLE_CHARS)),
            text_before_cursor: non_empty(self.text_before_cursor)
                .map(|text| tail_chars(&text, MAX_CHARS_BEFORE_CURSOR)),
            selected_text: non_empty(self.selected_text)
                .map(|text| head_chars(&text, MAX_SELECTED_CHARS)),
            text_after_cursor: non_empty(self.text_after_cursor)
                .map(|text| head_chars(&text, MAX_CHARS_AFTER_CURSOR)),
            screen_text: non_empty(self.screen_text)
                .map(|text| head_chars(&text, MAX_SCREEN_CHARS)),
        }
    }
}

fn non_blank(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

pub fn head_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

pub fn tail_chars(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    text.chars().skip(total.saturating_sub(max_chars)).collect()
}

const OPENING_PUNCTUATION: [char; 15] = [
    '(', '[', '{', '"', '\'', '/', '@', '#', '“', '‘', '„', '«', '‹', '¿', '¡',
];
const CLOSING_PUNCTUATION: [char; 13] = [
    '.', ',', ';', ':', '!', '?', ')', ']', '}', '”', '’', '»', '…',
];

pub fn fit_to_cursor(text_before_cursor: Option<&str>, text: &str) -> String {
    let Some(previous) = text_before_cursor.and_then(|before| before.chars().last()) else {
        return text.to_owned();
    };
    let Some(first) = text.chars().next() else {
        return String::new();
    };
    let needs_space = !previous.is_whitespace()
        && !first.is_whitespace()
        && !OPENING_PUNCTUATION.contains(&previous)
        && !CLOSING_PUNCTUATION.contains(&first)
        && !writes_without_spaces(previous)
        && !writes_without_spaces(first);
    if needs_space {
        format!(" {text}")
    } else {
        text.to_owned()
    }
}

fn writes_without_spaces(character: char) -> bool {
    matches!(
        character,
        '\u{3000}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FF00}'..='\u{FFEF}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_context() -> TextContext {
        TextContext {
            app_name: Some("Slack".into()),
            window_title: Some("#general".into()),
            text_before_cursor: Some("Hey Priyanka,".into()),
            selected_text: Some("draft".into()),
            text_after_cursor: Some("Thanks".into()),
            screen_text: Some("Priyanka: can you send the Tauri demo?".into()),
        }
    }

    #[test]
    fn restricting_drops_the_fields_the_mode_does_not_allow() {
        assert!(full_context().restricted_to(ContextMode::Off).is_empty());
        let app_only = full_context().restricted_to(ContextMode::AppOnly);
        assert_eq!(app_only.app_name.as_deref(), Some("Slack"));
        assert_eq!(app_only.window_title.as_deref(), Some("#general"));
        assert_eq!(app_only.text_before_cursor, None);
        assert_eq!(app_only.selected_text, None);
        assert_eq!(app_only.screen_text, None);
        let cursor = full_context().restricted_to(ContextMode::Cursor);
        assert_eq!(cursor.text_before_cursor.as_deref(), Some("Hey Priyanka,"));
        assert_eq!(cursor.screen_text, None);
        assert_eq!(
            full_context().restricted_to(ContextMode::Screen),
            full_context()
        );
    }

    #[test]
    fn normalizing_trims_blank_fields_and_caps_lengths() {
        let context = TextContext {
            app_name: Some("  ".into()),
            window_title: Some(" Notes ".into()),
            text_before_cursor: Some("é".repeat(MAX_CHARS_BEFORE_CURSOR + 10) + "end"),
            selected_text: Some(String::new()),
            text_after_cursor: Some("a".repeat(MAX_CHARS_AFTER_CURSOR * 2)),
            screen_text: Some("s".repeat(MAX_SCREEN_CHARS + 50)),
        }
        .normalized();
        assert_eq!(context.app_name, None);
        assert_eq!(context.window_title.as_deref(), Some("Notes"));
        let before = context.text_before_cursor.expect("before text is kept");
        assert_eq!(before.chars().count(), MAX_CHARS_BEFORE_CURSOR);
        assert!(before.ends_with("end"));
        assert_eq!(context.selected_text, None);
        assert_eq!(
            context.text_after_cursor.map(|t| t.chars().count()),
            Some(MAX_CHARS_AFTER_CURSOR)
        );
        assert_eq!(
            context.screen_text.map(|t| t.chars().count()),
            Some(MAX_SCREEN_CHARS)
        );
    }

    #[test]
    fn adds_a_space_when_continuing_after_a_word() {
        assert_eq!(fit_to_cursor(Some("Hello"), "world"), " world");
        assert_eq!(fit_to_cursor(Some("Hey Priyanka,"), "thanks"), " thanks");
    }

    #[test]
    fn leaves_text_alone_after_whitespace_openers_or_with_no_context() {
        assert_eq!(fit_to_cursor(Some("Hello "), "world"), "world");
        assert_eq!(fit_to_cursor(Some("line\n"), "Next"), "Next");
        assert_eq!(fit_to_cursor(Some("call ("), "maybe"), "maybe");
        assert_eq!(fit_to_cursor(Some("@"), "priyanka"), "priyanka");
        assert_eq!(fit_to_cursor(None, "Hello"), "Hello");
        assert_eq!(fit_to_cursor(Some(""), "Hello"), "Hello");
    }

    #[test]
    fn treats_curly_quotes_and_inverted_marks_as_openers_and_closers() {
        assert_eq!(fit_to_cursor(Some("He said “"), "hello"), "hello");
        assert_eq!(fit_to_cursor(Some("¿"), "Qué"), "Qué");
        assert_eq!(fit_to_cursor(Some("word"), "” next"), "” next");
        assert_eq!(fit_to_cursor(Some("quoted”"), "next"), " next");
    }

    #[test]
    fn never_adds_spaces_in_chinese_or_japanese() {
        assert_eq!(fit_to_cursor(Some("今天"), "天气"), "天气");
        assert_eq!(fit_to_cursor(Some("あ"), "Tokyo"), "Tokyo");
        assert_eq!(fit_to_cursor(Some("Tokyo"), "です"), "です");
    }

    #[test]
    fn does_not_space_before_punctuation_or_empty_text() {
        assert_eq!(fit_to_cursor(Some("Hello"), ", world"), ", world");
        assert_eq!(fit_to_cursor(Some("Hello"), "."), ".");
        assert_eq!(fit_to_cursor(Some("Hello"), ""), "");
    }
}
