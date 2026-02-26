use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::entities::project;
use terminalist::sync::tasks::ProjectUpdateIntent;
use terminalist::ui::components::DialogComponent;
use terminalist::ui::core::{Action, Component, DialogType};
use uuid::Uuid;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn sample_project(name: &str, is_inbox: bool) -> project::Model {
    project::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: format!("remote-{}", name),
        name: name.to_string(),
        is_favorite: false,
        is_inbox_project: is_inbox,
        order_index: 0,
        parent_uuid: None,
    }
}

#[test]
fn task_fields_cycle_with_tab_and_backtab() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::TaskName
    ));
    dialog.handle_key_events(key(KeyCode::Tab));
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::Description
    ));
    dialog.handle_key_events(key(KeyCode::Tab));
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::DueDate
    ));
    dialog.handle_key_events(key(KeyCode::Tab));
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::Project
    ));
    dialog.handle_key_events(key(KeyCode::Tab));
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::TaskName
    ));

    dialog.handle_key_events(key(KeyCode::BackTab));
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::Project
    ));
    dialog.handle_key_events(key(KeyCode::BackTab));
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::DueDate
    ));
}

#[test]
fn up_down_cycles_project_when_project_field_is_focused() {
    let mut dialog = DialogComponent::new();
    let work = sample_project("Work", false);
    let personal = sample_project("Personal", false);
    dialog.projects = vec![sample_project("Inbox", true), work.clone(), personal.clone()];
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));

    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, Some(0));
    assert_eq!(dialog.selected_task_project_uuid, Some(work.uuid));
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, Some(1));
    assert_eq!(dialog.selected_task_project_uuid, Some(personal.uuid));
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, None);
    assert_eq!(dialog.selected_task_project_uuid, None);
    dialog.handle_key_events(key(KeyCode::Up));
    assert_eq!(dialog.selected_task_project_index, Some(1));
    assert_eq!(dialog.selected_task_project_uuid, Some(personal.uuid));
}

#[test]
fn project_cycle_with_single_project_toggles_project_and_inbox() {
    let mut dialog = DialogComponent::new();
    let work = sample_project("Work", false);
    dialog.projects = vec![sample_project("Inbox", true), work.clone()];
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));

    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, Some(0));
    assert_eq!(dialog.selected_task_project_uuid, Some(work.uuid));

    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, None);
    assert_eq!(dialog.selected_task_project_uuid, None);
}

#[test]
fn project_cycle_with_no_non_inbox_projects_keeps_selection_empty() {
    let mut dialog = DialogComponent::new();
    dialog.projects = vec![sample_project("Inbox", true)];
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));

    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, None);
    assert_eq!(dialog.selected_task_project_uuid, None);
}

#[test]
fn submit_create_task_with_all_fields() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    dialog.input_buffer = "Buy groceries".to_string();
    dialog.description_buffer = "Milk and eggs".to_string();
    dialog.due_date_buffer = "tmrw".to_string();
    let project_id = Uuid::new_v4();
    dialog.selected_task_project_uuid = Some(project_id);
    dialog.task_project_explicitly_selected = true;

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::CreateTask {
            content,
            description,
            due_string,
            project_uuid,
        } => {
            assert_eq!(content, "Buy groceries");
            assert_eq!(description.as_deref(), Some("Milk and eggs"));
            assert_eq!(due_string.as_deref(), Some("tomorrow"));
            assert_eq!(project_uuid, Some(project_id));
        }
        other => panic!("Expected CreateTask action, got {other:?}"),
    }
}

#[test]
fn submit_create_task_empty_optional_fields() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));
    dialog.input_buffer = "Title only".to_string();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::CreateTask {
            description,
            due_string,
            ..
        } => {
            assert_eq!(description, None);
            assert_eq!(due_string, None);
        }
        other => panic!("Expected CreateTask action, got {other:?}"),
    }
}

#[test]
fn edit_dialog_prepopulates_all_buffers() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    let project_uuid = Uuid::new_v4();
    let mut work_project = sample_project("Work", false);
    work_project.uuid = project_uuid;
    dialog.projects = vec![sample_project("Inbox", true), work_project];

    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing name".to_string(),
        description: "Existing description".to_string(),
        due_date: "2026-02-26".to_string(),
        project_uuid: Some(project_uuid),
    }));

    assert_eq!(dialog.input_buffer, "Existing name");
    assert_eq!(dialog.description_buffer, "Existing description");
    assert_eq!(dialog.due_date_buffer, "2026-02-26");
    assert_eq!(dialog.cursor_position, "Existing name".chars().count());
    assert_eq!(dialog.description_cursor, "Existing description".chars().count());
    assert_eq!(dialog.due_date_cursor, "2026-02-26".chars().count());
    assert!(matches!(dialog.dialog_type, Some(DialogType::TaskEdit { task_uuid: id, .. }) if id == task_uuid));

    dialog.input_buffer = "Updated name".to_string();
    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask {
            task_uuid: id,
            project_update,
            ..
        } => {
            assert_eq!(id, task_uuid);
            assert_eq!(project_update, ProjectUpdateIntent::Unchanged);
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn edit_dialog_missing_project_uuid_falls_back_to_inbox() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    let stale_project_uuid = Uuid::new_v4();
    dialog.projects = vec![sample_project("Inbox", true), sample_project("Work", false)];

    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing name".to_string(),
        description: "Existing description".to_string(),
        due_date: "2026-02-26".to_string(),
        project_uuid: Some(stale_project_uuid),
    }));

    assert_eq!(dialog.selected_task_project_index, None);
    assert_eq!(dialog.selected_task_project_uuid, None);

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask {
            task_uuid: id,
            project_update,
            ..
        } => {
            assert_eq!(id, task_uuid);
            assert_eq!(project_update, ProjectUpdateIntent::Unchanged);
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn edit_dialog_explicit_project_selection_emits_set_intent() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    let project_uuid = Uuid::new_v4();
    let mut work = sample_project("Work", false);
    work.uuid = project_uuid;
    dialog.projects = vec![sample_project("Inbox", true), work];

    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing".to_string(),
        description: "desc".to_string(),
        due_date: "2026-02-26".to_string(),
        project_uuid: None,
    }));

    dialog.input_buffer = "Updated".to_string();
    dialog.selected_task_project_uuid = Some(project_uuid);
    dialog.task_project_explicitly_selected = true;

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { project_update, .. } => {
            assert_eq!(project_update, ProjectUpdateIntent::Set(project_uuid));
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn create_dialog_missing_default_project_uuid_falls_back_to_inbox() {
    let mut dialog = DialogComponent::new();
    let stale_default_project_uuid = Uuid::new_v4();
    dialog.projects = vec![sample_project("Inbox", true), sample_project("Work", false)];

    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: Some(stale_default_project_uuid),
    }));

    assert_eq!(dialog.selected_task_project_index, None);
    assert_eq!(dialog.selected_task_project_uuid, None);
    assert!(!dialog.task_project_explicitly_selected);

    dialog.input_buffer = "Title".to_string();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::CreateTask { project_uuid, .. } => {
            assert_eq!(project_uuid, None);
        }
        other => panic!("Expected CreateTask action, got {other:?}"),
    }
}

#[test]
fn submit_edit_task_keeps_due_when_unchanged() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing".to_string(),
        description: "desc".to_string(),
        due_date: "2026-02-26".to_string(),
        project_uuid: None,
    }));

    dialog.input_buffer = "Updated".to_string();
    dialog.description_buffer = "Updated description".to_string();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask {
            task_uuid: id,
            description,
            due_string,
            ..
        } => {
            assert_eq!(id, task_uuid);
            assert_eq!(description.as_deref(), Some("Updated description"));
            assert_eq!(due_string, None);
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn submit_edit_task_clears_due_when_field_is_emptied() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing".to_string(),
        description: "desc".to_string(),
        due_date: "2026-02-26".to_string(),
        project_uuid: None,
    }));

    dialog.input_buffer = "Updated".to_string();
    dialog.due_date_buffer.clear();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { due_string, .. } => {
            assert_eq!(due_string.as_deref(), Some("no date"));
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn submit_edit_task_updates_due_when_changed() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing".to_string(),
        description: "desc".to_string(),
        due_date: "2026-02-26".to_string(),
        project_uuid: None,
    }));

    dialog.input_buffer = "Updated".to_string();
    dialog.due_date_buffer = "tmrw".to_string();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { due_string, .. } => {
            assert_eq!(due_string.as_deref(), Some("tomorrow"));
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn due_date_input_dialog_submit_behavior_unchanged() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDueDateInput { task_uuid }));

    for c in "tmrw".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::SetTaskDueString(uuid, due) => {
            assert_eq!(uuid, task_uuid);
            assert_eq!(due, "tomorrow");
        }
        other => panic!("Expected SetTaskDueString, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Suite 1: Character Input Routing
// ---------------------------------------------------------------------------

#[test]
fn typing_into_description_field_populates_description_buffer() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Tab to Description
    dialog.handle_key_events(key(KeyCode::Tab));

    for c in "hello".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    assert_eq!(dialog.description_buffer, "hello");
    assert!(dialog.input_buffer.is_empty());
    assert!(dialog.due_date_buffer.is_empty());
}

#[test]
fn typing_into_due_date_field_populates_due_date_buffer() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Tab×2 to DueDate
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));

    for c in "tmrw".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }

    assert_eq!(dialog.due_date_buffer, "tmrw");
    assert!(dialog.input_buffer.is_empty());
    assert!(dialog.description_buffer.is_empty());
}

#[test]
fn char_input_ignored_when_project_field_focused() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Tab×3 to Project
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));

    dialog.handle_key_events(key(KeyCode::Char('x')));

    assert!(dialog.input_buffer.is_empty());
    assert!(dialog.description_buffer.is_empty());
    assert!(dialog.due_date_buffer.is_empty());
}

#[test]
fn backspace_in_description_removes_from_description_buffer() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Tab to Description
    dialog.handle_key_events(key(KeyCode::Tab));

    for c in "abc".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }
    dialog.handle_key_events(key(KeyCode::Backspace));

    assert_eq!(dialog.description_buffer, "ab");
    assert_eq!(dialog.description_cursor, 2);
}

#[test]
fn left_right_arrows_move_cursor_in_active_field() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Tab to Description
    dialog.handle_key_events(key(KeyCode::Tab));

    for c in "abc".chars() {
        dialog.handle_key_events(key(KeyCode::Char(c)));
    }
    assert_eq!(dialog.description_cursor, 3);

    dialog.handle_key_events(key(KeyCode::Left));
    assert_eq!(dialog.description_cursor, 2);

    dialog.handle_key_events(key(KeyCode::Right));
    assert_eq!(dialog.description_cursor, 3);

    // Left×4 should clamp at 0
    for _ in 0..4 {
        dialog.handle_key_events(key(KeyCode::Left));
    }
    assert_eq!(dialog.description_cursor, 0);
}

#[test]
fn up_down_arrows_noop_when_not_on_project_field() {
    let mut dialog = DialogComponent::new();
    let work = sample_project("Work", false);
    dialog.projects = vec![sample_project("Inbox", true), work];
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Focus on TaskName (default)
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, None);

    dialog.handle_key_events(key(KeyCode::Up));
    assert_eq!(dialog.selected_task_project_index, None);
}

// ---------------------------------------------------------------------------
// Suite 2: Submit Boundary Cases
// ---------------------------------------------------------------------------

#[test]
fn submit_with_empty_task_name_returns_none() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::None => {}
        other => panic!("Expected Action::None, got {other:?}"),
    }
}

#[test]
fn submit_with_whitespace_only_task_name_returns_none() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));
    dialog.input_buffer = "   ".to_string();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::None => {}
        other => panic!("Expected Action::None, got {other:?}"),
    }
}

#[test]
fn edit_submit_empty_description_yields_some_empty_string() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Task".to_string(),
        description: "Old desc".to_string(),
        due_date: String::new(),
        project_uuid: None,
    }));

    // Clear the description buffer
    dialog.description_buffer.clear();
    dialog.description_cursor = 0;

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { description, .. } => {
            assert_eq!(description.as_deref(), Some(""));
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn edit_submit_move_to_inbox_when_user_cycles_to_none() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    let work = sample_project("Work", false);
    dialog.projects = vec![sample_project("Inbox", true), work.clone()];

    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Task".to_string(),
        description: String::new(),
        due_date: String::new(),
        project_uuid: Some(work.uuid),
    }));

    // Tab to Project
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));
    dialog.handle_key_events(key(KeyCode::Tab));

    // Cycle Down past last project to None slot
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.selected_task_project_index, None);

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { project_update, .. } => {
            assert_eq!(project_update, ProjectUpdateIntent::MoveToInbox);
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn create_dialog_valid_default_project_preselects_it() {
    let mut dialog = DialogComponent::new();
    let work = sample_project("Work", false);
    let project_uuid = work.uuid;
    dialog.projects = vec![sample_project("Inbox", true), work];

    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: Some(project_uuid),
    }));

    assert_eq!(dialog.selected_task_project_uuid, Some(project_uuid));
    assert_eq!(dialog.selected_task_project_index, Some(0));
    assert!(!dialog.task_project_explicitly_selected);

    // Submit without Tab → project_uuid should come from default (not explicit selection)
    dialog.input_buffer = "Task".to_string();
    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::CreateTask { project_uuid: puid, .. } => {
            // Default project was set but user didn't explicitly select via Tab,
            // so it uses the default_project_uuid path which checks it exists
            assert_eq!(puid, Some(project_uuid));
        }
        other => panic!("Expected CreateTask action, got {other:?}"),
    }
}

#[test]
fn edit_submit_whitespace_padded_due_unchanged() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Task".to_string(),
        description: String::new(),
        due_date: "2026-02-26".to_string(),
        project_uuid: None,
    }));

    // Add trailing whitespace to buffer
    dialog.due_date_buffer = "2026-02-26 ".to_string();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { due_string, .. } => {
            // Trimmed buffer matches trimmed original → None (unchanged)
            assert_eq!(due_string, None);
        }
        other => panic!("Expected EditTask action, got {other:?}"),
    }
}

#[test]
fn edit_empty_name_returns_none() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Existing".to_string(),
        description: String::new(),
        due_date: String::new(),
        project_uuid: None,
    }));

    dialog.input_buffer.clear();

    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::None => {}
        other => panic!("Expected Action::None, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Suite 3: State Lifecycle
// ---------------------------------------------------------------------------

#[test]
fn clear_dialog_resets_all_new_fields() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Fill in all buffers
    dialog.input_buffer = "title".to_string();
    dialog.description_buffer = "desc".to_string();
    dialog.due_date_buffer = "tmrw".to_string();
    dialog.cursor_position = 5;
    dialog.description_cursor = 4;
    dialog.due_date_cursor = 4;
    dialog.active_task_field = terminalist::ui::components::ActiveTaskField::DueDate;
    dialog.task_project_explicitly_selected = true;

    // Submit to trigger clear_dialog
    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::CreateTask { .. } => {}
        other => panic!("Expected CreateTask, got {other:?}"),
    }

    // Now open a fresh TaskCreation
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    assert!(dialog.input_buffer.is_empty());
    assert!(dialog.description_buffer.is_empty());
    assert!(dialog.due_date_buffer.is_empty());
    assert_eq!(dialog.cursor_position, 0);
    assert_eq!(dialog.description_cursor, 0);
    assert_eq!(dialog.due_date_cursor, 0);
    assert!(matches!(
        dialog.active_task_field,
        terminalist::ui::components::ActiveTaskField::TaskName
    ));
    assert!(!dialog.task_project_explicitly_selected);
}

#[test]
fn esc_cancels_and_resets_state() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));

    // Type data
    dialog.input_buffer = "Something".to_string();
    dialog.description_buffer = "desc".to_string();
    dialog.due_date_buffer = "tmrw".to_string();

    // Press Esc
    let action = dialog.handle_key_events(key(KeyCode::Esc));
    assert!(matches!(action, Action::HideDialog));

    // HideDialog triggers clear via update
    dialog.update(Action::HideDialog);

    // Reopen and verify clean
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
    }));
    assert!(dialog.input_buffer.is_empty());
    assert!(dialog.description_buffer.is_empty());
    assert!(dialog.due_date_buffer.is_empty());
}

#[test]
fn edit_dialog_sets_original_due_date_buffer() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Task".to_string(),
        description: String::new(),
        due_date: "2026-03-01".to_string(),
        project_uuid: None,
    }));

    // original_due_date_buffer is private, but we can verify its effect:
    // If we don't change the due_date_buffer, due_string should be None (unchanged)
    match dialog.handle_key_events(key(KeyCode::Enter)) {
        Action::EditTask { due_string, .. } => {
            assert_eq!(due_string, None, "Unmodified due date should yield None");
        }
        other => panic!("Expected EditTask, got {other:?}"),
    }
}
