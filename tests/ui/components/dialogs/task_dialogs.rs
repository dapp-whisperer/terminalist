use ratatui::{backend::TestBackend, layout::Rect, style::Color, Terminal};
use terminalist::entities::project;
use terminalist::icons::IconService;
use terminalist::ui::components::ActiveTaskField;

fn sample_project(name: &str, is_inbox: bool) -> project::Model {
    project::Model {
        uuid: uuid::Uuid::new_v4(),
        backend_uuid: uuid::Uuid::new_v4(),
        remote_id: format!("remote-{}", name),
        name: name.to_string(),
        is_favorite: false,
        is_inbox_project: is_inbox,
        order_index: 0,
        parent_uuid: None,
    }
}

#[test]
fn render_task_dialog_4_fields_does_not_panic() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let projects = [sample_project("Work", false), sample_project("Personal", false)];
    let project_refs: Vec<&project::Model> = projects.iter().collect();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            terminalist::ui::components::dialogs::task_dialogs::render_task_creation_dialog(
                f,
                area,
                &IconService::default(),
                "Buy groceries",
                3,
                "Milk",
                4,
                "tomorrow",
                8,
                &project_refs,
                Some(0),
                ActiveTaskField::TaskName,
            );
        })
        .unwrap();
}

#[test]
fn focused_field_uses_cyan_border() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let projects = [sample_project("Work", false)];
    let project_refs: Vec<&project::Model> = projects.iter().collect();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            terminalist::ui::components::dialogs::task_dialogs::render_task_creation_dialog(
                f,
                area,
                &IconService::default(),
                "Task",
                4,
                "",
                0,
                "",
                0,
                &project_refs,
                Some(0),
                ActiveTaskField::TaskName,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let has_cyan = buffer.content().iter().any(|cell| cell.fg == Color::Cyan);
    let has_dark_gray = buffer.content().iter().any(|cell| cell.fg == Color::DarkGray);
    assert!(has_cyan);
    assert!(has_dark_gray);
}

#[test]
fn render_task_edit_dialog_does_not_panic() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let projects = [sample_project("Work", false)];
    let project_refs: Vec<&project::Model> = projects.iter().collect();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            terminalist::ui::components::dialogs::task_dialogs::render_task_edit_dialog(
                f,
                area,
                &IconService::default(),
                "Existing task",
                13,
                "Some description",
                16,
                "2026-03-01",
                10,
                &project_refs,
                Some(0),
                ActiveTaskField::TaskName,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text: String = buffer.content().iter().map(|cell| cell.symbol().to_string()).collect();
    assert!(text.contains("Edit Task"));
}

#[test]
fn focused_description_field_uses_cyan() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let projects = [sample_project("Work", false)];
    let project_refs: Vec<&project::Model> = projects.iter().collect();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            terminalist::ui::components::dialogs::task_dialogs::render_task_creation_dialog(
                f,
                area,
                &IconService::default(),
                "Task",
                4,
                "desc",
                4,
                "",
                0,
                &project_refs,
                None,
                ActiveTaskField::Description,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let has_cyan = buffer.content().iter().any(|cell| cell.fg == Color::Cyan);
    assert!(has_cyan);
}

#[test]
fn focused_project_field_uses_cyan() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let projects = [sample_project("Work", false)];
    let project_refs: Vec<&project::Model> = projects.iter().collect();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            terminalist::ui::components::dialogs::task_dialogs::render_task_creation_dialog(
                f,
                area,
                &IconService::default(),
                "",
                0,
                "",
                0,
                "",
                0,
                &project_refs,
                None,
                ActiveTaskField::Project,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let has_cyan = buffer.content().iter().any(|cell| cell.fg == Color::Cyan);
    assert!(has_cyan);
}

#[test]
fn empty_buffers_render_without_panic() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let projects: Vec<&project::Model> = Vec::new();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            terminalist::ui::components::dialogs::task_dialogs::render_task_creation_dialog(
                f,
                area,
                &IconService::default(),
                "",
                0,
                "",
                0,
                "",
                0,
                &projects,
                None,
                ActiveTaskField::TaskName,
            );
        })
        .unwrap();
}

#[test]
fn render_due_date_dialog_still_works() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 80, 24);
            terminalist::ui::components::dialogs::task_dialogs::render_due_date_input_dialog(
                f,
                area,
                "next friday",
                11,
            );
        })
        .unwrap();
}
