use super::common::{self, shortcuts};
use crate::entities::project;
use crate::icons::IconService;
use crate::ui::components::dialog_component::ActiveTaskField;
use crate::ui::layout::LayoutManager;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::Clear,
    Frame,
};

#[allow(clippy::too_many_arguments)]
pub fn render_task_dialog(
    f: &mut Frame,
    area: Rect,
    _icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    description_buffer: &str,
    description_cursor: usize,
    due_date_buffer: &str,
    due_date_cursor: usize,
    task_projects: &[&project::Model],
    selected_project_index: Option<usize>,
    is_editing: bool,
    active_field: ActiveTaskField,
) {
    let title = if is_editing { "Edit Task" } else { "New Task" };
    let dialog_area = LayoutManager::centered_rect_lines(65, 20, area);
    f.render_widget(Clear, dialog_area);

    let main_block = common::create_dialog_block(title, Color::Cyan);

    // Create layout for content
    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Task content input field
            Constraint::Length(3), // Description input field
            Constraint::Length(3), // Due date input field
            Constraint::Length(3), // Project selection field
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_paragraph = common::create_input_paragraph_styled(
        input_buffer,
        cursor_position,
        "Task Name",
        active_field == ActiveTaskField::TaskName,
    );
    let description_paragraph = common::create_input_paragraph_styled(
        description_buffer,
        description_cursor,
        "Description",
        active_field == ActiveTaskField::Description,
    );
    let due_date_paragraph = common::create_input_paragraph_styled(
        due_date_buffer,
        due_date_cursor,
        "Due Date",
        active_field == ActiveTaskField::DueDate,
    );

    // Project selection field
    let project_name = match selected_project_index {
        None => "None (Inbox)".to_string(),
        Some(index) => {
            if index < task_projects.len() {
                task_projects[index].name.clone()
            } else {
                "None (Inbox)".to_string()
            }
        }
    };

    let project_paragraph =
        common::create_selection_paragraph_styled(project_name, "Project", active_field == ActiveTaskField::Project);

    // Instructions based on mode
    let action = if is_editing {
        ("Enter", Color::Green, " Save Task")
    } else {
        ("Enter", Color::Green, " Create Task")
    };

    let instructions = [
        action,
        shortcuts::SEPARATOR,
        ("Tab", Color::Cyan, " Next"),
        shortcuts::SEPARATOR,
        ("Shift+Tab", Color::Cyan, " Prev"),
        shortcuts::SEPARATOR,
        ("↑↓", Color::Cyan, " Project"),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    // Render all components
    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(description_paragraph, chunks[1]);
    f.render_widget(due_date_paragraph, chunks[2]);
    f.render_widget(project_paragraph, chunks[3]);
    f.render_widget(instructions_paragraph, chunks[5]);

    match active_field {
        ActiveTaskField::TaskName => {
            f.set_cursor_position((chunks[0].x + 1 + cursor_position as u16, chunks[0].y + 1));
        }
        ActiveTaskField::Description => {
            f.set_cursor_position((chunks[1].x + 1 + description_cursor as u16, chunks[1].y + 1));
        }
        ActiveTaskField::DueDate => {
            f.set_cursor_position((chunks[2].x + 1 + due_date_cursor as u16, chunks[2].y + 1));
        }
        ActiveTaskField::Project => {}
    }
}

pub fn render_due_date_input_dialog(f: &mut Frame, area: Rect, input_buffer: &str, cursor_position: usize) {
    let dialog_area = LayoutManager::centered_rect_lines(65, 8, area);
    f.render_widget(Clear, dialog_area);

    let main_block = common::create_dialog_block("Set Due Date", Color::Cyan);

    let inner_area = main_block.inner(dialog_area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // Input field
            Constraint::Length(1), // Instructions
        ])
        .split(inner_area);

    let input_paragraph = common::create_input_paragraph(input_buffer, cursor_position, "Due Date");

    let instructions = [
        ("Enter", Color::Green, " Set Date"),
        shortcuts::SEPARATOR,
        ("Empty", Color::Yellow, " Clear Date"),
        shortcuts::SEPARATOR,
        shortcuts::ESC_CANCEL,
    ];
    let instructions_paragraph = common::create_instructions_paragraph(&instructions);

    f.render_widget(main_block, dialog_area);
    f.render_widget(input_paragraph, chunks[0]);
    f.render_widget(instructions_paragraph, chunks[1]);

    f.set_cursor_position((chunks[0].x + 1 + cursor_position as u16, chunks[0].y + 1));
}

// Legacy wrapper functions for backward compatibility
#[allow(clippy::too_many_arguments)]
pub fn render_task_creation_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    description_buffer: &str,
    description_cursor: usize,
    due_date_buffer: &str,
    due_date_cursor: usize,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
    active_field: ActiveTaskField,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        cursor_position,
        description_buffer,
        description_cursor,
        due_date_buffer,
        due_date_cursor,
        task_projects,
        selected_task_project_index,
        false, // is_editing = false for creation
        active_field,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn render_task_edit_dialog(
    f: &mut Frame,
    area: Rect,
    icons: &IconService,
    input_buffer: &str,
    cursor_position: usize,
    description_buffer: &str,
    description_cursor: usize,
    due_date_buffer: &str,
    due_date_cursor: usize,
    task_projects: &[&project::Model],
    selected_task_project_index: Option<usize>,
    active_field: ActiveTaskField,
) {
    render_task_dialog(
        f,
        area,
        icons,
        input_buffer,
        cursor_position,
        description_buffer,
        description_cursor,
        due_date_buffer,
        due_date_cursor,
        task_projects,
        selected_task_project_index,
        true, // is_editing = true for editing
        active_field,
    );
}
