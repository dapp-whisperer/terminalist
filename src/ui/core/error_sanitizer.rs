use crate::constants::*;

const SAFE_ERROR_PREFIXES: &[&str] = &[
    ERROR_TASK_COMPLETION_FAILED,
    ERROR_TASK_DELETE_FAILED,
    ERROR_TASK_UPDATE_FAILED,
    ERROR_TASK_CREATE_FAILED,
    ERROR_TASK_DUE_DATE_FAILED,
    ERROR_TASK_PRIORITY_FAILED,
    ERROR_PROJECT_CREATE_FAILED,
    ERROR_PROJECT_DELETE_FAILED,
    ERROR_PROJECT_UPDATE_FAILED,
    ERROR_LABEL_CREATE_FAILED,
    ERROR_LABEL_DELETE_FAILED,
    ERROR_LABEL_UPDATE_FAILED,
    ERROR_TASK_RESTORE_FAILED,
    ERROR_INVALID_PRIORITY_FORMAT,
    ERROR_INVALID_PRIORITY_INFO,
    ERROR_INVALID_DATE_FORMAT,
    ERROR_INVALID_TASK_EDIT_FORMAT,
    ERROR_INVALID_PROJECT_EDIT_FORMAT,
    ERROR_INVALID_LABEL_EDIT_FORMAT,
    ERROR_UNKNOWN_OPERATION,
];

pub fn sanitize_user_error(raw_error: &str, fallback_message: &str) -> String {
    let trimmed = raw_error.trim();

    for safe_prefix in SAFE_ERROR_PREFIXES {
        if trimmed == *safe_prefix || trimmed.starts_with(safe_prefix) || trimmed.contains(safe_prefix) {
            return (*safe_prefix).to_string();
        }
    }

    fallback_message.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_internal_context_when_known_error_prefix_exists() {
        let raw = "Operation failed: ❌ Failed to create task: Backend error: 401 unauthorized token=secret";
        let message = sanitize_user_error(raw, ERROR_OPERATION_FAILED);
        assert_eq!(message, ERROR_TASK_CREATE_FAILED);
    }

    #[test]
    fn falls_back_for_unknown_errors() {
        let message = sanitize_user_error("database timeout: connection reset", ERROR_SYNC_FAILED);
        assert_eq!(message, ERROR_SYNC_FAILED);
    }
}
