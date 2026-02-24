use chrono::{NaiveDate, Weekday};
use terminalist::utils::datetime::*;

#[test]
fn test_normalize_due_string_abbreviations() {
    assert_eq!(normalize_due_string("tmrw"), "tomorrow");
    assert_eq!(normalize_due_string("tmr"), "tomorrow");
    assert_eq!(normalize_due_string("tom"), "tomorrow");
    assert_eq!(normalize_due_string("tmw"), "tomorrow");
    assert_eq!(normalize_due_string("tod"), "today");
    assert_eq!(normalize_due_string("tdy"), "today");
    assert_eq!(normalize_due_string("yday"), "yesterday");
    assert_eq!(normalize_due_string("yest"), "yesterday");
    assert_eq!(normalize_due_string("mon"), "monday");
    assert_eq!(normalize_due_string("tue"), "tuesday");
    assert_eq!(normalize_due_string("tues"), "tuesday");
    assert_eq!(normalize_due_string("wed"), "wednesday");
    assert_eq!(normalize_due_string("thu"), "thursday");
    assert_eq!(normalize_due_string("thur"), "thursday");
    assert_eq!(normalize_due_string("thurs"), "thursday");
    assert_eq!(normalize_due_string("fri"), "friday");
    assert_eq!(normalize_due_string("sat"), "saturday");
    assert_eq!(normalize_due_string("sun"), "sunday");
}

#[test]
fn test_normalize_due_string_case_insensitive() {
    assert_eq!(normalize_due_string("TMRW"), "tomorrow");
    assert_eq!(normalize_due_string("Fri"), "friday");
    assert_eq!(normalize_due_string("NEXT FRI"), "NEXT friday"); // "NEXT" preserved, "FRI" expanded
    assert_eq!(normalize_due_string("Tod"), "today");
}

#[test]
fn test_normalize_due_string_multi_word() {
    assert_eq!(normalize_due_string("next fri"), "next friday");
    assert_eq!(normalize_due_string("next thurs"), "next thursday");
    assert_eq!(normalize_due_string("next tues"), "next tuesday");
    assert_eq!(normalize_due_string("every mon"), "every monday");
}

#[test]
fn test_normalize_due_string_passthrough() {
    assert_eq!(normalize_due_string("tomorrow"), "tomorrow");
    assert_eq!(normalize_due_string("next friday"), "next friday");
    assert_eq!(normalize_due_string("march 15"), "march 15");
    assert_eq!(normalize_due_string("in 3 days"), "in 3 days");
    assert_eq!(normalize_due_string("March 15"), "March 15");
}

#[test]
fn test_normalize_due_string_empty_and_whitespace() {
    assert_eq!(normalize_due_string(""), "");
    // Whitespace-only returns input unchanged (caller handles trim)
    assert_eq!(normalize_due_string("   "), "   ");
}

#[test]
fn test_normalize_due_string_collapses_extra_spaces() {
    assert_eq!(normalize_due_string("next   fri"), "next friday");
}

#[test]
fn test_format_ymd() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
    assert_eq!(format_ymd(date), "2025-01-15");
}

#[test]
fn test_next_weekday() {
    let monday = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(); // Monday
    let friday = next_weekday(monday, Weekday::Fri);
    assert_eq!(friday, NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
}

#[test]
fn test_next_weekday_monday() {
    let friday = NaiveDate::from_ymd_opt(2023, 12, 22).unwrap(); // Friday
    let next_monday = next_weekday(friday, Weekday::Mon);
    let expected = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap(); // Next Monday
    assert_eq!(next_monday, expected);
}

#[test]
fn test_next_weekday_same_day() {
    let monday = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap(); // Monday
    let next_monday = next_weekday(monday, Weekday::Mon);
    let expected = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Next Monday (7 days later)
    assert_eq!(next_monday, expected);
}

#[test]
fn test_format_human_date_today() {
    let today = format_today();
    assert_eq!(format_human_date(&today), "today");
}

#[test]
fn test_format_human_date_tomorrow() {
    let tomorrow = format_date_with_offset(1);
    assert_eq!(format_human_date(&tomorrow), "tomorrow");
}

#[test]
fn test_format_human_date_yesterday() {
    let yesterday = format_date_with_offset(-1);
    assert_eq!(format_human_date(&yesterday), "yesterday");
}

#[test]
fn test_format_human_datetime_iso_format() {
    // Test the specific format from the user's example
    let datetime_str = "2025-09-16T09:00:00";
    let formatted = format_human_datetime(datetime_str);

    // Should contain time information and be human-readable
    assert!(formatted.contains("at"));
    assert!(formatted.contains("09:00"));
}
