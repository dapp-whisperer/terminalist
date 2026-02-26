use terminalist::logger;

#[test]
fn test_memory_logs() {
    // Clear any existing logs
    logger::clear_memory_logs();

    // Test that we can get empty logs
    let logs = logger::get_memory_logs();
    assert!(logs.is_empty());
}

#[test]
fn test_log_file_path() {
    // Test that we can get the log file path
    let path = logger::get_log_file_path();
    assert!(path.is_ok());
    let path = path.unwrap();
    assert!(path.to_string_lossy().contains("terminalist.log"));
}

#[test]
fn test_sanitize_for_log_escapes_control_characters() {
    let value = "line1\nline2\t\u{001b}[31m";
    let sanitized = logger::sanitize_for_log(value);

    assert_eq!(sanitized, "line1\\u{000A}line2\\u{0009}\\u{001B}[31m");
}

#[test]
fn test_redact_user_text_for_log_reports_length_only() {
    let redacted = logger::redact_user_text_for_log("do taxes tomorrow");
    assert_eq!(redacted, "[redacted len=17]");
}
