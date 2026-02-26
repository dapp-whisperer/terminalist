use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::entities::{project, task};
use terminalist::ui::components::TaskListComponent;
use terminalist::ui::core::{Action, Component, DialogType, SidebarSelection};
use uuid::Uuid;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn sample_project(uuid: Uuid) -> project::Model {
    project::Model {
        uuid,
        backend_uuid: Uuid::new_v4(),
        remote_id: "project-remote-id".to_string(),
        name: "Work".to_string(),
        is_favorite: false,
        is_inbox_project: false,
        order_index: 0,
        parent_uuid: None,
    }
}

fn sample_task(project_uuid: Uuid) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "task-remote-id".to_string(),
        content: "Task with datetime only".to_string(),
        description: Some("desc".to_string()),
        project_uuid,
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: Some("2026-02-26T09:30:00Z".to_string()),
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        is_deleted: false,
    }
}

#[test]
fn test_task_list_component_creation() {
    // Test that TaskListComponent can be created without panicking
    let _task_list = TaskListComponent::new();
}

#[test]
fn edit_key_populates_description_and_due_date() {
    let mut task_list = TaskListComponent::new();
    let project_uuid = Uuid::new_v4();
    let mut task = sample_task(project_uuid);
    task.description = Some("My desc".to_string());
    task.due_date = Some("2026-03-01".to_string());
    task.due_datetime = None;

    task_list.update_data(
        vec![task.clone()],
        Vec::new(),
        vec![sample_project(project_uuid)],
        Vec::new(),
        SidebarSelection::Project(0),
    );

    match task_list.handle_key_events(key(KeyCode::Char('e'))) {
        Action::ShowDialog(DialogType::TaskEdit {
            description,
            due_date,
            ..
        }) => {
            assert_eq!(description, "My desc");
            assert_eq!(due_date, "2026-03-01");
        }
        other => panic!("Expected TaskEdit dialog action, got {other:?}"),
    }
}

#[test]
fn edit_key_with_no_description_sends_empty() {
    let mut task_list = TaskListComponent::new();
    let project_uuid = Uuid::new_v4();
    let mut task = sample_task(project_uuid);
    task.description = None;

    task_list.update_data(
        vec![task.clone()],
        Vec::new(),
        vec![sample_project(project_uuid)],
        Vec::new(),
        SidebarSelection::Project(0),
    );

    match task_list.handle_key_events(key(KeyCode::Char('e'))) {
        Action::ShowDialog(DialogType::TaskEdit { description, .. }) => {
            assert_eq!(description, "");
        }
        other => panic!("Expected TaskEdit dialog action, got {other:?}"),
    }
}

#[test]
fn p_key_emits_cycle_priority() {
    let mut task_list = TaskListComponent::new();
    let project_uuid = Uuid::new_v4();
    let task = sample_task(project_uuid);
    let task_uuid = task.uuid;

    task_list.update_data(
        vec![task],
        Vec::new(),
        vec![sample_project(project_uuid)],
        Vec::new(),
        SidebarSelection::Project(0),
    );

    match task_list.handle_key_events(key(KeyCode::Char('p'))) {
        Action::CyclePriority(uuid_str) => {
            assert_eq!(uuid_str, task_uuid.to_string());
        }
        other => panic!("Expected CyclePriority action, got {other:?}"),
    }
}

#[test]
fn edit_dialog_prefills_due_from_datetime_when_date_missing() {
    let mut task_list = TaskListComponent::new();
    let project_uuid = Uuid::new_v4();
    let task = sample_task(project_uuid);

    task_list.update_data(
        vec![task.clone()],
        Vec::new(),
        vec![sample_project(project_uuid)],
        Vec::new(),
        SidebarSelection::Project(0),
    );

    match task_list.handle_key_events(key(KeyCode::Char('e'))) {
        Action::ShowDialog(DialogType::TaskEdit { due_date, .. }) => {
            assert_eq!(due_date, "2026-02-26T09:30:00Z");
        }
        other => panic!("Expected TaskEdit dialog action, got {other:?}"),
    }
}
