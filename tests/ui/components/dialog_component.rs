use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::ui::components::DialogComponent;
use terminalist::ui::core::{Action, Component, DialogType};
use uuid::Uuid;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_dialog_component_creation() {
    let _dialog = DialogComponent::new();
}

// --- TaskDueDateInput dialog behavior tests ---

#[test]
fn test_due_date_dialog_opens_with_empty_input() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));
    assert!(dialog.is_visible());
    assert!(dialog.input_buffer.is_empty());
    assert_eq!(dialog.cursor_position, 0);
}

#[test]
fn test_due_date_submit_normalizes_abbreviation() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "tmrw".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    let action = dialog.handle_key_events(key(KeyCode::Enter));
    match action {
        Action::SetTaskDueString(uuid, due) => {
            assert_eq!(uuid, task_uuid);
            assert_eq!(due, "tomorrow");
        }
        other => panic!("Expected SetTaskDueString, got {:?}", other),
    }
}

#[test]
fn test_due_date_submit_empty_sends_no_date() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    let action = dialog.handle_key_events(key(KeyCode::Enter));
    match action {
        Action::SetTaskDueString(uuid, due) => {
            assert_eq!(uuid, task_uuid);
            assert_eq!(due, "no date");
        }
        other => panic!("Expected SetTaskDueString with 'no date', got {:?}", other),
    }
}

#[test]
fn test_due_date_submit_whitespace_only_sends_no_date() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "   ".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    let action = dialog.handle_key_events(key(KeyCode::Enter));
    match action {
        Action::SetTaskDueString(uuid, due) => {
            assert_eq!(uuid, task_uuid);
            assert_eq!(due, "no date");
        }
        other => panic!("Expected SetTaskDueString with 'no date', got {:?}", other),
    }
}

#[test]
fn test_due_date_cancel_returns_hide_dialog() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    dialog.handle_key_events(key(KeyCode::Char('t')));
    let action = dialog.handle_key_events(key(KeyCode::Esc));
    assert!(matches!(action, Action::HideDialog));
}

#[test]
fn test_due_date_cancel_clears_state() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    dialog.handle_key_events(key(KeyCode::Char('x')));
    dialog.update(Action::HideDialog);
    assert!(!dialog.is_visible());
    assert!(dialog.input_buffer.is_empty());
}

#[test]
fn test_due_date_typing_updates_buffer_and_cursor() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "fri".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }
    assert_eq!(dialog.input_buffer, "fri");
    assert_eq!(dialog.cursor_position, 3);
}

#[test]
fn test_due_date_backspace() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "fri".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    dialog.handle_key_events(key(KeyCode::Backspace));
    assert_eq!(dialog.input_buffer, "fr");
    assert_eq!(dialog.cursor_position, 2);
}

#[test]
fn test_due_date_cursor_movement() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "fri".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }
    assert_eq!(dialog.cursor_position, 3);

    dialog.handle_key_events(key(KeyCode::Left));
    assert_eq!(dialog.cursor_position, 2);

    dialog.handle_key_events(key(KeyCode::Left));
    assert_eq!(dialog.cursor_position, 1);

    dialog.handle_key_events(key(KeyCode::Right));
    assert_eq!(dialog.cursor_position, 2);

    // Right at end of string should not go past length
    dialog.handle_key_events(key(KeyCode::Right));
    dialog.handle_key_events(key(KeyCode::Right));
    assert_eq!(dialog.cursor_position, 3);

    // Left at start should not go negative
    dialog.handle_key_events(key(KeyCode::Left));
    dialog.handle_key_events(key(KeyCode::Left));
    dialog.handle_key_events(key(KeyCode::Left));
    dialog.handle_key_events(key(KeyCode::Left));
    assert_eq!(dialog.cursor_position, 0);
}

#[test]
fn test_due_date_submit_passthrough_natural_language() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "march 15".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    let action = dialog.handle_key_events(key(KeyCode::Enter));
    match action {
        Action::SetTaskDueString(uuid, due) => {
            assert_eq!(uuid, task_uuid);
            assert_eq!(due, "march 15");
        }
        other => panic!("Expected SetTaskDueString, got {:?}", other),
    }
}

#[test]
fn test_due_date_submit_clears_dialog() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput {
        task_uuid,
    }));

    for c in "fri".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    let _action = dialog.handle_key_events(key(KeyCode::Enter));
    assert!(!dialog.is_visible());
    assert!(dialog.input_buffer.is_empty());
    assert_eq!(dialog.cursor_position, 0);
}
