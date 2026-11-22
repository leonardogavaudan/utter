use accessibility_sys::{
    kAXFocusedUIElementAttribute, kAXFocusedWindowAttribute, kAXNumberOfCharactersAttribute,
    kAXRoleAttribute, kAXSecureTextFieldSubrole, kAXSelectedTextAttribute,
    kAXSelectedTextRangeAttribute, kAXSubroleAttribute, kAXTitleAttribute, kAXValueAttribute,
};
use std::time::{Duration, Instant};

use core_foundation::base::CFRange;
use objc2_app_kit::NSWorkspace;
use utter_core::context::{
    ContextMode, MAX_CHARS_AFTER_CURSOR, MAX_CHARS_BEFORE_CURSOR, MAX_SCREEN_CHARS, TextContext,
};
use utter_core::screen::{Frame, TextRun, nearby_text};

use super::ax::Element;

const ELECTRON_ACCESSIBILITY_ATTRIBUTE: &str = "AXManualAccessibility";
const STATIC_TEXT_ROLE: &str = "AXStaticText";
const SCROLL_AREA_ROLE: &str = "AXScrollArea";
const TEXT_INPUT_ROLES: [&str; 3] = ["AXTextArea", "AXTextField", "AXComboBox"];
const MAX_SCREEN_ELEMENTS: usize = 4_000;
const SCREEN_WALK_BUDGET: Duration = Duration::from_millis(400);

struct FrontmostApp {
    pid: i32,
    name: Option<String>,
}

pub(crate) fn capture(mode: ContextMode) -> TextContext {
    if !mode.reads_app() {
        return TextContext::default();
    }
    let Some(frontmost) = frontmost_app() else {
        return TextContext::default();
    };

    let app = Element::application(frontmost.pid);
    let window = app.element(kAXFocusedWindowAttribute);
    let mut context = TextContext {
        app_name: frontmost.name.or_else(|| app.string(kAXTitleAttribute)),
        window_title: window
            .as_ref()
            .and_then(|window| window.string(kAXTitleAttribute)),
        ..TextContext::default()
    };
    if !mode.reads_text() {
        return context.normalized();
    }
    let field = focused_element(&app).filter(is_text_input);
    if field.as_ref().is_some_and(is_secure) {
        return context.normalized();
    }
    if let Some(field) = &field {
        read_text_around_cursor(field, &mut context);
    }
    if mode.reads_screen()
        && let Some(window) = &window
    {
        context.screen_text = read_screen_text(window, field.as_ref().and_then(Element::frame));
    }
    context.normalized()
}

pub(crate) fn focused_app_pid() -> Option<i32> {
    frontmost_app().map(|app| app.pid)
}

pub(crate) fn is_text_field_focused() -> bool {
    frontmost_app()
        .map(|frontmost| Element::application(frontmost.pid))
        .and_then(|app| focused_element(&app))
        .is_some_and(|element| is_text_input(&element))
}

fn frontmost_app() -> Option<FrontmostApp> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(FrontmostApp {
        pid: app.processIdentifier(),
        name: app.localizedName().map(|name| name.to_string()),
    })
}

fn focused_element(app: &Element) -> Option<Element> {
    app.set_flag(ELECTRON_ACCESSIBILITY_ATTRIBUTE, true);
    app.element(kAXFocusedUIElementAttribute)
}

fn read_screen_text(window: &Element, focus: Option<Frame>) -> Option<String> {
    let started = Instant::now();
    let mut runs = Vec::new();
    let mut pending = vec![(window.children(), window.frame()?)];
    let mut visited = 0;
    while let Some((elements, clip)) = pending.pop() {
        for element in elements {
            visited += 1;
            if visited > MAX_SCREEN_ELEMENTS || started.elapsed() > SCREEN_WALK_BUDGET {
                return nearby_text(runs, focus, MAX_SCREEN_CHARS);
            }
            match element.string(kAXRoleAttribute).as_deref() {
                Some(STATIC_TEXT_ROLE) => {
                    if let (Some(text), Some(frame)) =
                        (element.string(kAXValueAttribute), element.frame())
                        && clip.intersection(&frame).is_some()
                    {
                        runs.push(TextRun { text, frame });
                    }
                }
                Some(SCROLL_AREA_ROLE) => {
                    if let Some(visible) =
                        element.frame().and_then(|frame| clip.intersection(&frame))
                    {
                        pending.push((element.children(), visible));
                    }
                }
                _ => pending.push((element.children(), clip)),
            }
        }
    }
    nearby_text(runs, focus, MAX_SCREEN_CHARS)
}

fn is_text_input(element: &Element) -> bool {
    element
        .string(kAXRoleAttribute)
        .is_some_and(|role| TEXT_INPUT_ROLES.contains(&role.as_str()))
}

fn is_secure(element: &Element) -> bool {
    [kAXSubroleAttribute, kAXRoleAttribute]
        .iter()
        .any(|attribute| element.string(attribute).as_deref() == Some(kAXSecureTextFieldSubrole))
}

fn read_text_around_cursor(field: &Element, context: &mut TextContext) {
    context.selected_text = field.string(kAXSelectedTextAttribute);
    let Some(selection) = field.range(kAXSelectedTextRangeAttribute) else {
        return;
    };
    let length = field.integer(kAXNumberOfCharactersAttribute);
    let (before, after) = ranges_around(selection, length);
    context.text_before_cursor = before.and_then(|range| field.string_for_range(range));
    context.text_after_cursor = after.and_then(|range| field.string_for_range(range));

    if context.text_before_cursor.is_none()
        && context.text_after_cursor.is_none()
        && let Some(value) = field.string(kAXValueAttribute)
    {
        let units: Vec<u16> = value.encode_utf16().collect();
        let length = Some(units.len() as i64);
        let (before, after) = ranges_around(selection, length);
        context.text_before_cursor = before.map(|range| utf16_slice(&units, range));
        context.text_after_cursor = after.map(|range| utf16_slice(&units, range));
    }
}

fn ranges_around(selection: CFRange, length: Option<i64>) -> (Option<CFRange>, Option<CFRange>) {
    let max_before = MAX_CHARS_BEFORE_CURSOR as i64;
    let max_after = MAX_CHARS_AFTER_CURSOR as i64;
    let start = selection.location.max(0) as i64;
    let end = start + selection.length.max(0) as i64;
    let before_start = (start - max_before).max(0);
    let after_length = length.map_or(max_after, |length| (length - end).clamp(0, max_after));
    let before = (start > before_start)
        .then(|| CFRange::init(before_start as _, (start - before_start) as _));
    let after = (after_length > 0).then(|| CFRange::init(end as _, after_length as _));
    (before, after)
}

fn utf16_slice(units: &[u16], range: CFRange) -> String {
    let start = (range.location.max(0) as usize).min(units.len());
    let end = (start + range.length.max(0) as usize).min(units.len());
    String::from_utf16_lossy(&units[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_stop_at_the_start_and_end_of_the_text() {
        let (before, after) = ranges_around(CFRange::init(5, 0), Some(8));
        let before = before.expect("has text before");
        let after = after.expect("has text after");
        assert_eq!((before.location, before.length), (0, 5));
        assert_eq!((after.location, after.length), (5, 3));
    }

    #[test]
    fn ranges_skip_the_selection_and_cap_the_window() {
        let (before, after) = ranges_around(CFRange::init(1_000, 10), Some(5_000));
        let before = before.expect("has text before");
        let after = after.expect("has text after");
        assert_eq!(before.location, 1_000 - MAX_CHARS_BEFORE_CURSOR as isize);
        assert_eq!(before.length, MAX_CHARS_BEFORE_CURSOR as isize);
        assert_eq!(
            (after.location, after.length),
            (1_010, MAX_CHARS_AFTER_CURSOR as isize)
        );
    }

    #[test]
    fn empty_fields_have_no_ranges() {
        let (before, after) = ranges_around(CFRange::init(0, 0), Some(0));
        assert!(before.is_none());
        assert!(after.is_none());
    }

    #[test]
    fn slices_utf16_without_splitting_past_the_end() {
        let units: Vec<u16> = "héllo 👋".encode_utf16().collect();
        assert_eq!(utf16_slice(&units, CFRange::init(0, 5)), "héllo");
        assert_eq!(utf16_slice(&units, CFRange::init(6, 50)), "👋");
    }
}
