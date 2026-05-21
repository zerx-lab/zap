use super::{get_input_key, text_fallback_event_for_unconverted_key};
use winit::event::ElementState;
use winit::keyboard::{Key::Character, ModifiersState, SmolStr};

#[test]
fn test_get_input_key() {
    // Tests all visible ASCII characters
    // TODO: it would be nice to test the following:
    // - non-Character keys (ex: named keys, dead keys)
    // - non-ascii characters to ensure shift behavior is appropriate
    for ascii_code in 32u8..127u8 {
        let input = ascii_code as char;
        let key = Character(SmolStr::from(input.to_string()));

        for shift in [false, true] {
            match get_input_key(&key, shift) {
                Character(new_value) => {
                    let new_char = new_value
                        .chars()
                        .next()
                        .expect("string should be non-empty");

                    let expected = match (input, shift) {
                        ('A'..='Z', false) => input
                            .to_lowercase()
                            .next()
                            .expect("string should be non-empty"),
                        // Case 2: a lower case letter when shift is true
                        // Should turn into upper case version
                        ('a'..='z', true) => input
                            .to_uppercase()
                            .next()
                            .expect("string should be non-empty"),
                        // Case 3: a character that should be unchanged by caps lock
                        // - An upper-case letter when shift is true
                        // - A lower-case letter when shift is false,
                        // - A non-alpha character
                        _ => input,
                    };
                    assert_eq!(
                        expected, new_char,
                        "Expected '{input}' -> '{expected}' when shift={shift}, but got '{new_char}'"
                    )
                }
                unexpected => {
                    panic!("Key '{key:?}' somehow became non-character {unexpected:?}")
                }
            }
        }
    }
}

#[test]
fn text_fallback_emits_typed_characters_for_unmodified_text() {
    let event = text_fallback_event_for_unconverted_key(
        Some("mobile".to_string()),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
    );

    match event.expect("text input should produce an event") {
        crate::Event::TypedCharacters { chars } => assert_eq!("mobile", chars),
        unexpected => panic!("expected typed characters, got {unexpected:?}"),
    }
}

#[test]
fn text_fallback_maps_enter_to_key_down() {
    let event = text_fallback_event_for_unconverted_key(
        Some("\r".to_string()),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
    );

    match event.expect("enter should produce a key event") {
        crate::Event::KeyDown {
            keystroke, chars, ..
        } => {
            assert_eq!("enter", keystroke.key);
            assert_eq!("\r", chars);
        }
        unexpected => panic!("expected key down, got {unexpected:?}"),
    }
}

#[test]
fn text_fallback_preserves_control_key_chars() {
    for (input, expected_key) in [
        ("\u{8}", "backspace"),
        ("\u{7f}", "delete"),
        ("\u{1b}", "escape"),
    ] {
        let event = text_fallback_event_for_unconverted_key(
            Some(input.to_string()),
            ElementState::Pressed,
            ModifiersState::empty(),
            false,
        );

        match event.expect("control key should produce a key event") {
            crate::Event::KeyDown {
                keystroke, chars, ..
            } => {
                assert_eq!(expected_key, keystroke.key);
                assert_eq!(input, chars);
            }
            unexpected => panic!("expected key down, got {unexpected:?}"),
        }
    }
}

#[test]
fn text_fallback_ignores_shortcuts() {
    let event = text_fallback_event_for_unconverted_key(
        Some("v".to_string()),
        ElementState::Pressed,
        ModifiersState::CONTROL,
        false,
    );

    assert!(event.is_none());
}

#[test]
fn text_fallback_ignores_synthetic_events() {
    let event = text_fallback_event_for_unconverted_key(
        Some("a".to_string()),
        ElementState::Pressed,
        ModifiersState::empty(),
        true,
    );

    assert!(event.is_none());
}

#[test]
fn text_fallback_ignores_key_releases() {
    let event = text_fallback_event_for_unconverted_key(
        Some("a".to_string()),
        ElementState::Released,
        ModifiersState::empty(),
        false,
    );

    assert!(event.is_none());
}
