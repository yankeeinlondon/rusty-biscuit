//! Text-utility helpers for classification — case-insensitive substring
//! matching, HTTP status-code extraction, and rate-limit reset-at parsing.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use regex::Regex;
use std::sync::LazyLock;

use chrono::DateTime;

static RESET_AT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"reset at (\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}:\d{2})")
        .expect("opencode reset-at regex must compile")
});

static STATUS_CODE_RES: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [
        Regex::new(r#""statusCode":(\d{3})(?:\D|$)"#).expect("status-code regex 1 must compile"),
        Regex::new(r"statusCode=(\d{3})(?:\D|$)").expect("status-code regex 2 must compile"),
    ]
});

pub(super) fn contains_any_ci(haystack: &str, needles: &[&str]) -> bool {
    let lowered = haystack.to_lowercase();
    needles.iter().any(|n| lowered.contains(&n.to_lowercase()))
}

pub(super) fn extract_status_code(haystack: &str) -> Option<u16> {
    for re in STATUS_CODE_RES.iter() {
        if let Some(caps) = re.captures(haystack)
            && let Some(m) = caps.get(1)
            && let Ok(code) = m.as_str().parse::<u16>()
        {
            return Some(code);
        }
    }
    None
}

pub(super) fn extract_reset_at(haystack: &str) -> Option<DateTime<Utc>> {
    let caps = RESET_AT_RE.captures(haystack)?;
    let date = NaiveDate::parse_from_str(caps.get(1)?.as_str(), "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(caps.get(2)?.as_str(), "%H:%M:%S").ok()?;
    Some(Utc.from_utc_datetime(&NaiveDateTime::new(date, time)))
}
