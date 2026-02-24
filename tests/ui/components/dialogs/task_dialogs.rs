use ratatui::{backend::TestBackend, layout::Rect, Terminal};

#[test]
fn test_task_dialogs_module_exists() {
    // Test that the task dialogs module compiles and is accessible
}

#[test]
fn test_render_due_date_dialog_does_not_panic() {
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

#[test]
fn test_render_due_date_dialog_empty_input() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 80, 24);
            terminalist::ui::components::dialogs::task_dialogs::render_due_date_input_dialog(
                f, area, "", 0,
            );
        })
        .unwrap();
}

#[test]
fn test_render_due_date_dialog_small_terminal() {
    let backend = TestBackend::new(30, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 30, 5);
            terminalist::ui::components::dialogs::task_dialogs::render_due_date_input_dialog(
                f, area, "", 0,
            );
        })
        .unwrap();
}
