use terminalist::constants::{ERROR_SYNC_FAILED, ERROR_TASK_UPDATE_FAILED};
use terminalist::ui::app_component::AppState;
use terminalist::ui::core::error_sanitizer::sanitize_user_error;

#[test]
fn test_app_state_default() {
    // Test that AppState can be created with default values
    let state = AppState::default();
    assert!(!state.loading, "Default AppState should not be loading");
    assert!(
        state.error_message.is_none(),
        "Default AppState should have no error message"
    );
}

#[test]
fn test_no_legacy_task_operation_dispatch_remains() {
    let source = include_str!("../../src/ui/app_component.rs");
    assert!(
        !source.contains("TaskOperation::Legacy"),
        "TaskOperation::Legacy should be removed"
    );
    assert!(
        !source.contains("split_once('|')"),
        "Pipe-delimited parsing should be removed from task operation dispatch"
    );
}

#[test]
fn test_sanitize_user_error_keeps_safe_prefix_for_user_message() {
    let raw_error = "❌ Failed to update task: Backend error: token=secret123";
    let message = sanitize_user_error(raw_error, ERROR_SYNC_FAILED);
    assert_eq!(message, ERROR_TASK_UPDATE_FAILED);
}

#[test]
fn test_sanitize_user_error_uses_fallback_for_unknown_error() {
    let raw_error = "todoist api timeout: host=api.todoist.com token=secret123";
    let message = sanitize_user_error(raw_error, ERROR_SYNC_FAILED);
    assert_eq!(message, ERROR_SYNC_FAILED);
}
