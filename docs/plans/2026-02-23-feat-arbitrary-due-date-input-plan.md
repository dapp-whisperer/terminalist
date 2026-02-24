---
title: "feat: Add arbitrary due date input via natural language"
type: feat
status: active
date: 2026-02-23
---

# feat: Add arbitrary due date input via natural language

Press `s` on a selected task to open a text input dialog where you type any date string ("next friday", "march 15", "in 3 days"). The raw string is sent to Todoist's `due_string` API field, which handles all parsing — same as the Todoist app itself. Empty input clears the due date.

## Acceptance Criteria

- [ ] `s` keybinding opens a date input dialog when a task is selected
- [ ] Shows info dialog when no task is selected (reuse `UI_NO_TASK_SELECTED_DUE_DATE`)
- [ ] Dialog accepts free-form text and submits on Enter, cancels on Esc
- [ ] Submitted string is sent to Todoist API via `due_string` field
- [ ] Local DB is updated from the API **response** (resolved `due_date`, `due_datetime`, `is_recurring`) — NOT from the user's input string
- [ ] Empty input submission clears the task's due date (sends `"no date"` as `due_string`)
- [ ] API errors (invalid date string) show an error dialog
- [ ] Help dialog and `docs/KEYBOARD_SHORTCUTS.md` updated with `s` shortcut

## Context

- `todoist-api` crate already supports `due_string` and `due_lang` on `UpdateTaskArgs`
- The app's `backend::UpdateTaskArgs` (`src/backend/mod.rs:119`) does NOT have `due_string` yet
- Current `update_task_due_date()` in `src/sync/tasks.rs:278` only accepts YYYY-MM-DD and writes the caller-provided string to the DB — this won't work for natural language since the resolved date comes back in the API response
- `due_string` and `due_date` are mutually exclusive in the Todoist API; when sending `due_string`, `due_date` and `due_datetime` must be `None`
- `due_lang` left as `None` (uses Todoist account language setting)

## MVP

### 1. Backend layer — add `due_string` field

#### `src/backend/mod.rs`

Add `due_string: Option<String>` to `UpdateTaskArgs` struct (after `due_datetime`).

#### `src/backend/todoist.rs`

Forward `due_string` in the `update_task()` mapping at line 251:

```rust
let todoist_args = crate::todoist::UpdateTaskArgs {
    // ... existing fields ...
    due_string: args.due_string,
    ..Default::default()
};
```

### 2. Sync layer — new method using API response

#### `src/sync/tasks.rs`

Add `update_task_due_string(&self, task_uuid: &Uuid, due_string: &str)`:

```rust
pub async fn update_task_due_string(
    &self,
    task_uuid: &Uuid,
    due_string: &str,
) -> anyhow::Result<()> {
    let remote_id = self.get_task_remote_id(task_uuid).await?;

    let task_args = crate::backend::UpdateTaskArgs {
        due_string: Some(due_string.to_string()),
        due_date: None,
        due_datetime: None,
        // all other fields None
        ..Default::default()
    };

    // API response contains the resolved date
    let backend_task = self.backend.update_task(&remote_id, task_args).await?;

    // Update local storage from response — not from input
    let mut active_model: task::ActiveModel = /* find by uuid */;
    active_model.due_date = Set(backend_task.due_date);
    active_model.due_datetime = Set(backend_task.due_datetime);
    active_model.is_recurring = Set(backend_task.is_recurring);
    active_model.update(&self.db).await?;
    Ok(())
}
```

### 3. Action & DialogType enums

#### `src/ui/core/actions.rs`

```rust
// New action variant (alongside existing SetTaskDueToday etc.)
SetTaskDueString(Uuid, String),

// New dialog variant (in DialogType enum)
TaskDueDateInput { task_uuid: Uuid },
```

### 4. Keybinding — `s` in global handler

#### `src/ui/app_component.rs` — `handle_global_key()` (~line 408)

```rust
KeyCode::Char('s') => {
    if let Some(task) = self.task_list.get_selected_task() {
        Action::ShowDialog(DialogType::TaskDueDateInput { task_uuid: task.uuid })
    } else {
        Action::ShowDialog(DialogType::Info(UI_NO_TASK_SELECTED_DUE_DATE.to_string()))
    }
}
```

### 5. Dialog handling

#### `src/ui/components/dialog_component.rs`

- **`update()`**: Handle `ShowDialog(TaskDueDateInput { .. })` — clear `input_buffer`, store dialog type
- **`handle_key_events()`**: Reuse the existing text input pattern (Char/Backspace/Delete/Left/Right/Enter/Esc) — the generic fallback branch already handles this for most dialog types
- **`handle_submit()`**: New match arm:

```rust
DialogType::TaskDueDateInput { task_uuid } => {
    if self.input_buffer.is_empty() {
        Action::SetTaskDueString(task_uuid, "no date".to_string())
    } else {
        Action::SetTaskDueString(task_uuid, self.input_buffer.clone())
    }
}
```

### 6. Dialog rendering

#### `src/ui/components/dialogs/task_dialogs.rs`

New render function — simple single-field dialog:
- Title: "Set Due Date"
- Input label: "Due Date"
- Instructions line: examples like `"next friday" | "march 15" | "in 3 days" | Esc: cancel`
- Size: `centered_rect_lines(65, 8, area)`

### 7. App action handler

#### `src/ui/app_component.rs` — `handle_app_action()` (~line 583)

```rust
Action::SetTaskDueString(task_uuid, due_string) => {
    self.spawn_task_operation(
        "Set task due string".to_string(),
        format!("{}|{}", task_uuid, due_string),
    );
}
```

And in `spawn_task_operation()` (~line 878), new match arm:

```rust
"Set task due string" => {
    let (uuid_str, due_string) = task_info.split_once('|').unwrap();
    let task_uuid = Uuid::parse_str(uuid_str)?;
    sync_service.update_task_due_string(&task_uuid, due_string).await?;
    format!("Due date set")
}
```

### 8. Constants

#### `src/constants.rs`

```rust
pub const SUCCESS_TASK_DUE_STRING_SET: &str = "Task due date updated";
```

### 9. Documentation updates

#### `docs/KEYBOARD_SHORTCUTS.md`

Add under Task Management: `s   Set task due date (natural language input)`

#### `src/ui/components/dialogs/system_dialogs.rs`

Add `s` to help dialog shortcuts list (~line 216).

## Sources

- Todoist REST API `due_string` field: already supported by `todoist-api` crate's `UpdateTaskArgs`
- Existing dialog patterns: `src/ui/components/dialog_component.rs`, `src/ui/components/dialogs/task_dialogs.rs`
- Existing date shortcuts: `src/ui/app_component.rs:369-408`
- Sync layer: `src/sync/tasks.rs:278-312`
