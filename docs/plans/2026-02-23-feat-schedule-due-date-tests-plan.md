---
title: Add Tests for Schedule Due Date Feature
type: feat
status: active
date: 2026-02-23
---

# Add Tests for Schedule Due Date Feature

The `s` keybinding / arbitrary due date feature has almost zero test coverage beyond 3 `normalize_due_string()` tests. This plan covers what to add, grouped by priority.

## Bugs Found During Analysis

- [ ] **Whitespace-only input bug**: `"   "` is not treated as empty -- gets sent to API as literal whitespace instead of `"no date"`. Fix: change `self.input_buffer.is_empty()` to `self.input_buffer.trim().is_empty()` at `src/ui/components/dialog_component.rs:253`
- [ ] **Missing `deadline` field update**: `update_task_due_string()` updates `due_date`, `due_datetime`, `is_recurring` from API response but skips `deadline`. Compare to `create_task` which does store it. Fix: add `active_model.deadline = ActiveValue::Set(backend_task.deadline);` at `src/sync/tasks.rs:348`

## Acceptance Criteria

- [ ] `normalize_due_string()` edge cases covered (empty, whitespace, case, all abbreviations)
- [ ] `DialogComponent` behavior tested for `TaskDueDateInput` (open, type, submit, cancel, cursor ops)
- [ ] Dialog rendering smoke tests (no panics, small terminal)
- [ ] Whitespace-only input bug fixed and tested
- [ ] Deadline field bug fixed
- [ ] All tests pass on CI (clippy clean, 3 OS matrix)

## Priority 1: `normalize_due_string()` Edge Cases

No infrastructure needed. Add to `tests/utils/datetime.rs`.

| Test | Input | Expected |
|------|-------|----------|
| Empty string | `""` | `""` |
| Whitespace only | `"   "` | `"   "` (or `""` after trim -- see bug above) |
| Case insensitive | `"TMRW"` | `"tomorrow"` |
| Case insensitive multi | `"NEXT FRI"` | `"next friday"` |
| Missing abbrevs: yday | `"yday"` | `"yesterday"` |
| Missing abbrevs: yest | `"yest"` | `"yesterday"` |
| Missing abbrevs: tmw | `"tmw"` | `"tomorrow"` |
| Missing abbrevs: wed/sat/sun | `"wed"`, `"sat"`, `"sun"` | full names |
| Passthrough preserves case | `"March 15"` | `"March 15"` |
| Multi-space collapsed | `"next   fri"` | `"next friday"` |
| Pipe in input | `"3\|pm"` | `"3\|pm"` (passthrough) |

## Priority 2: `DialogComponent` Behavior Tests

No mocks needed -- `DialogComponent` is synchronous. Add to `tests/ui/components/dialog_component.rs`.

- [ ] **Opens with empty input**: `update(ShowDialog(TaskDueDateInput { .. }))` -> `input_buffer` empty, `cursor_position` 0
- [ ] **Submit normalizes**: type `"tmrw"` + Enter -> `Action::SetTaskDueString(uuid, "tomorrow")`
- [ ] **Submit empty -> "no date"**: Enter immediately -> `Action::SetTaskDueString(uuid, "no date")`
- [ ] **Cancel returns HideDialog**: type something + Esc -> `Action::HideDialog`
- [ ] **Cancel clears state**: after HideDialog, `input_buffer` empty, not visible
- [ ] **Backspace**: type `"fri"` + Backspace -> `"fr"`, cursor 2
- [ ] **Left/Right cursor**: verify cursor_position tracks correctly
- [ ] **Submit whitespace-only -> "no date"** (after bug fix): `"   "` + Enter -> `"no date"`

Note: `Action` doesn't derive `PartialEq` -- use `match` arms in assertions, not `assert_eq!`.

## Priority 3: Render Smoke Tests

Uses ratatui `TestBackend`. Add to `tests/ui/components/dialogs/task_dialogs.rs`.

- [ ] **Normal render**: `TestBackend::new(80, 24)`, render with `"next friday"` input -- no panic
- [ ] **Empty input render**: render with `""` -- no panic
- [ ] **Small terminal**: `TestBackend::new(30, 5)` (smaller than 65x8 dialog) -- no panic

## Priority 4: Mock Backend (Future)

Blocked on creating a `MockBackend` implementing the `Backend` trait (`src/backend/mod.rs:145`). Once available, test:

- [ ] `update_task_due_string()` happy path: backend returns resolved date, local DB updated
- [ ] Backend error: error message surfaces correctly
- [ ] Clear date: `"no date"` -> backend returns `None` for date fields -> local DB cleared
- [ ] Task not found locally after backend success: silent no-op (or should it error?)

This is a separate effort -- don't block Priority 1-3 on it.

## Context

### Key Files
- `src/utils/datetime.rs:167` - `normalize_due_string()`
- `src/ui/components/dialog_component.rs:252` - `TaskDueDateInput` submit handler
- `src/ui/components/dialogs/task_dialogs.rs:85` - `render_due_date_input_dialog()`
- `src/sync/tasks.rs:318` - `update_task_due_string()`
- `src/ui/app_component.rs:409` - `'s'` keybinding
- `src/ui/app_component.rs:969` - `spawn_task_operation` pipe-delimited parsing

### Existing Test Files to Extend
- `tests/utils/datetime.rs` - add Priority 1 tests
- `tests/ui/components/dialog_component.rs` - add Priority 2 tests
- `tests/ui/components/dialogs/task_dialogs.rs` - add Priority 3 tests

## Sources

- Feature plan: `docs/plans/2026-02-23-feat-arbitrary-due-date-input-plan.md`
- Architecture guide: `RATATUI_ARCHITECTURE_GUIDELINES.md` (TestBackend patterns)
