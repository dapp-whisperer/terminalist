use terminalist::constants::{ERROR_OPERATION_FAILED, ERROR_TASK_CREATE_FAILED};
use terminalist::ui::core::actions::{Action, DialogType};
use terminalist::ui::core::task_manager::TaskManager;

#[test]
fn test_task_manager_creation() {
    // Test that TaskManager can be created without panicking
    let _task_manager = TaskManager::new();
}

#[tokio::test]
async fn test_task_operation_error_dialog_is_sanitized_from_known_prefix() {
    let (mut task_manager, mut action_rx) = TaskManager::new();

    task_manager.spawn_task_operation(
        || async {
            Err(anyhow::anyhow!(
                "{}: Backend error: token=secret123",
                ERROR_TASK_CREATE_FAILED
            ))
        },
        "Create task: demo".to_string(),
    );

    let action = action_rx.recv().await.expect("expected background action");
    match action {
        Action::ShowDialog(DialogType::Error(message)) => {
            assert_eq!(message, ERROR_TASK_CREATE_FAILED);
            assert!(!message.contains("secret123"));
        }
        other => panic!("expected error dialog action, got {:?}", other),
    }
}

#[tokio::test]
async fn test_task_operation_error_dialog_uses_generic_fallback_for_unknown_errors() {
    let (mut task_manager, mut action_rx) = TaskManager::new();

    task_manager.spawn_task_operation(
        || async { Err(anyhow::anyhow!("database timeout token=secret123")) },
        "Create task: demo".to_string(),
    );

    let action = action_rx.recv().await.expect("expected background action");
    match action {
        Action::ShowDialog(DialogType::Error(message)) => {
            assert_eq!(message, ERROR_OPERATION_FAILED);
            assert!(!message.contains("secret123"));
        }
        other => panic!("expected error dialog action, got {:?}", other),
    }
}
