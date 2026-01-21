//! Time and delay parsing utilities.

use chrono::{Duration as ChronoDuration, NaiveTime};

/// Parses a time string into a `NaiveTime`.
///
/// ## Supported Formats
///
/// - 12-hour format: `7:00am`, `11:30pm`
/// - 24-hour format: `19:30`, `07:00`
/// - Short 12-hour: `7am`, `11pm`
///
/// ## Errors
///
/// Returns an error string if the time cannot be parsed.
///
/// ## Examples
///
/// ```
/// use queue_lib::parse_at_time;
/// use chrono::NaiveTime;
///
/// let time = parse_at_time("7:00am").unwrap();
/// assert_eq!(time, NaiveTime::from_hms_opt(7, 0, 0).unwrap());
///
/// let time = parse_at_time("19:30").unwrap();
/// assert_eq!(time, NaiveTime::from_hms_opt(19, 30, 0).unwrap());
/// ```
pub fn parse_at_time(value: &str) -> Result<NaiveTime, String> {
    let normalized = value.trim().to_lowercase().replace(' ', "");

    if normalized.is_empty() {
        return Err("time cannot be empty".to_string());
    }

    let formats = ["%H:%M", "%I:%M%P", "%I%P"];

    for format in formats {
        if let Ok(time) = NaiveTime::parse_from_str(&normalized, format) {
            return Ok(time);
        }
    }

    Err("expected time like 7:00am or 19:30".to_string())
}

/// Parses a delay string into a `chrono::Duration`.
///
/// ## Supported Units
///
/// - `s` - seconds
/// - `m` - minutes (default if no unit specified)
/// - `h` - hours
/// - `d` - days
///
/// ## Errors
///
/// Returns an error string if the delay cannot be parsed.
///
/// ## Examples
///
/// ```
/// use queue_lib::parse_delay;
/// use chrono::Duration;
///
/// let delay = parse_delay("15").unwrap();
/// assert_eq!(delay, Duration::minutes(15));
///
/// let delay = parse_delay("30s").unwrap();
/// assert_eq!(delay, Duration::seconds(30));
/// ```
pub fn parse_delay(value: &str) -> Result<ChronoDuration, String> {
    let normalized = value.trim().to_lowercase().replace(' ', "");

    if normalized.is_empty() {
        return Err("delay cannot be empty".to_string());
    }

    let split_index = normalized
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(normalized.len());
    let (amount, unit) = normalized.split_at(split_index);

    if amount.is_empty() {
        return Err("delay must start with a number".to_string());
    }

    let amount: i64 = amount
        .parse()
        .map_err(|_| "delay must be a number".to_string())?;

    if amount <= 0 {
        return Err("delay must be greater than zero".to_string());
    }

    let duration = match unit {
        "" | "m" => ChronoDuration::minutes(amount),
        "s" => ChronoDuration::seconds(amount),
        "h" => ChronoDuration::hours(amount),
        "d" => ChronoDuration::days(amount),
        _ => {
            return Err("delay units must be s, m, h, or d".to_string());
        }
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_at_time_accepts_12_hour_format() {
        let time = parse_at_time("7:00am").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(7, 0, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_accepts_24_hour_format() {
        let time = parse_at_time("19:30").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(19, 30, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_rejects_empty() {
        assert!(parse_at_time("").is_err());
        assert!(parse_at_time("   ").is_err());
    }

    #[test]
    fn parse_delay_defaults_to_minutes() {
        let delay = parse_delay("15").expect("valid delay");
        assert_eq!(delay, ChronoDuration::minutes(15));
    }

    #[test]
    fn parse_delay_supports_seconds() {
        let delay = parse_delay("10s").expect("valid delay");
        assert_eq!(delay, ChronoDuration::seconds(10));
    }

    #[test]
    fn parse_delay_supports_hours() {
        let delay = parse_delay("2h").expect("valid delay");
        assert_eq!(delay, ChronoDuration::hours(2));
    }

    #[test]
    fn parse_delay_supports_days() {
        let delay = parse_delay("1d").expect("valid delay");
        assert_eq!(delay, ChronoDuration::days(1));
    }

    #[test]
    fn parse_delay_rejects_invalid_units() {
        assert!(parse_delay("1w").is_err());
    }

    #[test]
    fn parse_delay_rejects_zero() {
        assert!(parse_delay("0").is_err());
        assert!(parse_delay("0s").is_err());
    }

    #[test]
    fn parse_delay_rejects_negative() {
        assert!(parse_delay("-5").is_err());
    }

    #[test]
    fn parse_delay_rejects_empty() {
        assert!(parse_delay("").is_err());
        assert!(parse_delay("   ").is_err());
    }

    #[test]
    fn parse_at_time_accepts_pm_times() {
        let time = parse_at_time("11:30pm").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(23, 30, 0).expect("time"));

        let time = parse_at_time("7:30pm").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(19, 30, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_accepts_midnight_and_noon() {
        let time = parse_at_time("12:00am").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));

        let time = parse_at_time("12:00pm").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(12, 0, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_trims_whitespace() {
        let time = parse_at_time("  7:00am  ").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(7, 0, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_is_case_insensitive() {
        let time = parse_at_time("7:00AM").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(7, 0, 0).expect("time"));

        let time = parse_at_time("7:00Pm").expect("valid time");
        assert_eq!(time, NaiveTime::from_hms_opt(19, 0, 0).expect("time"));
    }

    #[test]
    fn parse_at_time_rejects_invalid() {
        assert!(parse_at_time("invalid").is_err());
        assert!(parse_at_time("25:00").is_err());
        assert!(parse_at_time("13:00am").is_err());
    }

    #[test]
    fn parse_delay_trims_whitespace() {
        let delay = parse_delay("  15m  ").expect("valid delay");
        assert_eq!(delay, ChronoDuration::minutes(15));
    }

    #[test]
    fn parse_delay_is_case_insensitive() {
        let delay = parse_delay("15M").expect("valid delay");
        assert_eq!(delay, ChronoDuration::minutes(15));

        let delay = parse_delay("2H").expect("valid delay");
        assert_eq!(delay, ChronoDuration::hours(2));
    }

    #[test]
    fn parse_delay_handles_large_values() {
        let delay = parse_delay("1000").expect("valid delay");
        assert_eq!(delay, ChronoDuration::minutes(1000));
    }

    #[test]
    fn parse_delay_rejects_no_number() {
        assert!(parse_delay("m").is_err());
        assert!(parse_delay("h").is_err());
    }
}
