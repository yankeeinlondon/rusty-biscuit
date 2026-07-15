use super::*;
use super::super::format;

pub(super) const KEYS: &[&str] = &[
    "now", "now_utc", "today", "yesterday", "tomorrow", "today_utc", "yesterday_utc",
    "tomorrow_utc", "day", "day_abbr", "day_utc", "day_abbr_utc", "year", "year_utc",
    "month", "month_name", "month_name_abbr", "day_of_month", "day_of_month_suffixed",
    "time", "time_military", "time_utc", "time_military_utc", "timezone", "timezone_offset",
    "timezone_iana", "start_of_week_sun", "end_of_week_sun", "start_of_week_mon",
    "end_of_week_mon", "start_of_week_sun_utc", "end_of_week_sun_utc",
    "start_of_week_mon_utc", "end_of_week_mon_utc", "season", "timestamp", "timestamp_ms",
];

pub(super) const ALIASES: &[&str] = &["utc", "dow", "dow_abbr"];

pub(crate) fn populate_datetime(values: &mut Map<String, Value>) {
    use chrono::{Datelike, Local, Utc};

    let now_local = Local::now();
    let now_utc = Utc::now();

    let today = now_local.date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let tomorrow = today + chrono::Duration::days(1);

    let today_utc = now_utc.date_naive();
    let yesterday_utc = today_utc - chrono::Duration::days(1);
    let tomorrow_utc = today_utc + chrono::Duration::days(1);

    // Legacy fields. `now` carries the host's zone offset (`%:z` → `+01:00`)
    // so the local timestamp is unambiguous — a bare local datetime cannot be
    // resolved to a real instant, and `now_utc` already signals its zone via
    // the trailing `Z`.
    let now_str = now_local.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
    let utc_str = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let tomorrow_str = tomorrow.format("%Y-%m-%d").to_string();
    let year_str = now_local.format("%Y").to_string();
    let month_str = now_local.format("%m").to_string();
    let month_name_str = now_local.format("%B").to_string();
    let month_name_abbr_str = now_local.format("%b").to_string();

    // Core date/time
    values.insert("now".into(), Value::String(now_str));
    values.insert("now_utc".into(), Value::String(utc_str));
    values.insert("today".into(), Value::String(today_str));
    values.insert("yesterday".into(), Value::String(yesterday_str));
    values.insert("tomorrow".into(), Value::String(tomorrow_str));

    // UTC date variants
    values.insert(
        "today_utc".into(),
        Value::String(today_utc.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "yesterday_utc".into(),
        Value::String(yesterday_utc.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "tomorrow_utc".into(),
        Value::String(tomorrow_utc.format("%Y-%m-%d").to_string()),
    );

    // Day of week
    values.insert(
        "day".into(),
        Value::String(now_local.format("%A").to_string()),
    );
    values.insert(
        "day_abbr".into(),
        Value::String(now_local.format("%a").to_string()),
    );

    // UTC day of week
    let dow_utc_str = now_utc.format("%A").to_string();
    let dow_abbr_utc_str = now_utc.format("%a").to_string();
    values.insert("day_utc".into(), Value::String(dow_utc_str));
    values.insert("day_abbr_utc".into(), Value::String(dow_abbr_utc_str));

    // Year/month
    values.insert("year".into(), Value::String(year_str));
    values.insert(
        "year_utc".into(),
        Value::String(now_utc.format("%Y").to_string()),
    );
    values.insert("month".into(), Value::String(month_str));
    values.insert("month_name".into(), Value::String(month_name_str));
    values.insert("month_name_abbr".into(), Value::String(month_name_abbr_str));

    // Day of month
    let day_of_month = today.day();
    values.insert(
        "day_of_month".into(),
        Value::String(day_of_month.to_string()),
    );
    values.insert(
        "day_of_month_suffixed".into(),
        Value::String(format!(
            "{}{}",
            day_of_month,
            format::ordinal_suffix(day_of_month)
        )),
    );

    // Time fields
    values.insert(
        "time".into(),
        Value::String(now_local.format("%I:%M %p").to_string()),
    );
    values.insert(
        "time_military".into(),
        Value::String(now_local.format("%H:%M").to_string()),
    );
    values.insert(
        "time_utc".into(),
        Value::String(format!("{} (UTC)", now_utc.format("%I:%M %p"))),
    );
    values.insert(
        "time_military_utc".into(),
        Value::String(format!("{} (UTC)", now_utc.format("%H:%M"))),
    );

    // Timezone: sniff owns abbreviation derivation (handles chrono's
    // %Z offset fallback on macOS via IANA-to-abbreviation mapping).
    // `probe_ntp: false` — darkmatter only consumes `timezone`/`timezone_iana`;
    // `ntp_status` is never surfaced by any `ctx.*` key, so the live NTP probe
    // (a network round-trip; up to 10s on Linux) must be skipped here.
    let tz_info = sniff::os::detect_timezone_with_options(false);
    values.insert(
        "timezone".into(),
        tz_info.timezone_abbr.map_or(Value::Null, Value::String),
    );
    values.insert(
        "timezone_offset".into(),
        Value::String(now_local.format("%z").to_string()),
    );
    values.insert(
        "timezone_iana".into(),
        tz_info.timezone.map_or(Value::Null, Value::String),
    );

    // Week boundaries (Sunday start)
    let weekday_num = today.weekday().num_days_from_sunday();
    let start_of_week_sun = today - chrono::Duration::days(weekday_num as i64);
    let end_of_week_sun = start_of_week_sun + chrono::Duration::days(6);
    values.insert(
        "start_of_week_sun".into(),
        Value::String(start_of_week_sun.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_sun".into(),
        Value::String(end_of_week_sun.format("%Y-%m-%d").to_string()),
    );

    // Week boundaries (Monday start)
    let weekday_mon = today.weekday().num_days_from_monday();
    let start_of_week_mon = today - chrono::Duration::days(weekday_mon as i64);
    let end_of_week_mon = start_of_week_mon + chrono::Duration::days(6);
    values.insert(
        "start_of_week_mon".into(),
        Value::String(start_of_week_mon.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_mon".into(),
        Value::String(end_of_week_mon.format("%Y-%m-%d").to_string()),
    );

    // UTC week boundaries
    let weekday_utc_sun = today_utc.weekday().num_days_from_sunday();
    let start_utc_sun = today_utc - chrono::Duration::days(weekday_utc_sun as i64);
    let end_utc_sun = start_utc_sun + chrono::Duration::days(6);
    values.insert(
        "start_of_week_sun_utc".into(),
        Value::String(start_utc_sun.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_sun_utc".into(),
        Value::String(end_utc_sun.format("%Y-%m-%d").to_string()),
    );

    let weekday_utc_mon = today_utc.weekday().num_days_from_monday();
    let start_utc_mon = today_utc - chrono::Duration::days(weekday_utc_mon as i64);
    let end_utc_mon = start_utc_mon + chrono::Duration::days(6);
    values.insert(
        "start_of_week_mon_utc".into(),
        Value::String(start_utc_mon.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_mon_utc".into(),
        Value::String(end_utc_mon.format("%Y-%m-%d").to_string()),
    );

    // Season
    values.insert(
        "season".into(),
        Value::String(format::determine_season(today.month(), today.day()).to_string()),
    );

    // Timestamps
    values.insert(
        "timestamp".into(),
        Value::Number(now_utc.timestamp().into()),
    );
    values.insert(
        "timestamp_ms".into(),
        Value::Number(now_utc.timestamp_millis().into()),
    );

    // Backward-compatible aliases (documented in `context-variables.md`).
    // Keep these in sync with the docs: any documented alias must resolve to
    // the same value as its canonical key so interpolation and the
    // `claudine context --values` report agree.
    populate_datetime_aliases(values);
}

/// Insert documented backward-compatible aliases for date/time keys.
///
/// Each alias mirrors the value of an existing canonical key. Aliases are
/// inserted only when the canonical key is present so a missing canonical
/// value cannot leak as a populated alias.
pub(super) fn populate_datetime_aliases(values: &mut Map<String, Value>) {
    const ALIASES: &[(&str, &str)] =
        &[("utc", "now_utc"), ("dow", "day"), ("dow_abbr", "day_abbr")];
    for (alias, canonical) in ALIASES {
        if let Some(value) = values.get(*canonical).cloned() {
            values.insert((*alias).to_string(), value);
        }
    }
}

// ── Repo context ──────────────────────────────────────────────────

/// Wraps a list of strings as a JSON string array.
///
/// List-valued `ctx.*` variables are captured as first-class arrays (spec D6);
/// callers pick a rendering with the D4 formatting functions (`as_csv`,
/// `as_unordered_list`, …). A bare `{{ ctx.some_list }}` renders line-separated.
pub(super) fn string_array(items: Vec<String>) -> Value {
    Value::Array(items.into_iter().map(Value::String).collect())
}

/// Removes a single trailing path separator so `repo_root` is join-safe and
/// matches `sniff repo root` (which also omits the trailing `/`).
pub(super) fn strip_trailing_sep(s: &str) -> String {
    let trimmed = s.strip_suffix('/').or_else(|| s.strip_suffix('\\'));
    trimmed.unwrap_or(s).to_string()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn populate_datetime_produces_all_expected_keys() {
        let mut values = Map::new();
        populate_datetime(&mut values);

        // Legacy fields
        assert!(values.contains_key("now"));
        assert!(values.contains_key("now_utc"));
        assert!(values.contains_key("today"));
        assert!(values.contains_key("yesterday"));
        assert!(values.contains_key("tomorrow"));
        assert!(values.contains_key("day"));
        assert!(values.contains_key("day_abbr"));
        assert!(values.contains_key("year"));
        assert!(values.contains_key("month"));
        assert!(values.contains_key("month_name"));
        assert!(values.contains_key("month_name_abbr"));

        // New date/time fields
        assert!(values.contains_key("today_utc"));
        assert!(values.contains_key("yesterday_utc"));
        assert!(values.contains_key("tomorrow_utc"));
        assert!(values.contains_key("day_utc"));
        assert!(values.contains_key("day_abbr_utc"));
        assert!(values.contains_key("year_utc"));
        assert!(values.contains_key("day_of_month"));
        assert!(values.contains_key("day_of_month_suffixed"));
        assert!(values.contains_key("time"));
        assert!(values.contains_key("time_military"));
        assert!(values.contains_key("timezone"));
        assert!(values.contains_key("timezone_offset"));
        assert!(values.contains_key("start_of_week_sun"));
        assert!(values.contains_key("end_of_week_sun"));
        assert!(values.contains_key("start_of_week_mon"));
        assert!(values.contains_key("end_of_week_mon"));
        assert!(values.contains_key("season"));
        assert!(values.contains_key("timestamp"));
        assert!(values.contains_key("timestamp_ms"));
    }

    /// Regression: `now` must carry the host zone offset (`+HH:MM` / `-HH:MM`)
    /// so the local timestamp is an unambiguous, RFC-3339-parseable instant
    /// rather than a bare local datetime.
    #[test]
    fn now_carries_zone_offset() {
        let mut values = Map::new();
        populate_datetime(&mut values);

        let now = values.get("now").and_then(Value::as_str).unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(now).is_ok(),
            "`now` must be an offset-bearing RFC 3339 datetime, got `{now}`"
        );
    }

    /// Regression: documented backward-compatible aliases (`utc`, `dow`,
    /// `dow_abbr`) must be populated by `populate_datetime` so they
    /// resolve in interpolation and in `claudine context --values` reports
    /// instead of rendering as `null`.
    #[test]
    fn populate_datetime_populates_documented_aliases() {
        let mut values = Map::new();
        populate_datetime(&mut values);

        for (alias, canonical) in [("utc", "now_utc"), ("dow", "day"), ("dow_abbr", "day_abbr")] {
            let alias_value = values
                .get(alias)
                .unwrap_or_else(|| panic!("alias `{alias}` must be present"));
            let canonical_value = values
                .get(canonical)
                .unwrap_or_else(|| panic!("canonical `{canonical}` must be present"));
            assert!(!alias_value.is_null(), "alias `{alias}` must not be null",);
            assert_eq!(
                alias_value, canonical_value,
                "alias `{alias}` must mirror canonical `{canonical}`",
            );
        }
    }

    #[test]
    fn day_of_month_suffixed_formats_correctly() {
        use super::format::ordinal_suffix;
        assert_eq!(ordinal_suffix(1), "st");
        assert_eq!(ordinal_suffix(2), "nd");
        assert_eq!(ordinal_suffix(3), "rd");
        assert_eq!(ordinal_suffix(4), "th");
        assert_eq!(ordinal_suffix(11), "th");
        assert_eq!(ordinal_suffix(12), "th");
        assert_eq!(ordinal_suffix(13), "th");
        assert_eq!(ordinal_suffix(21), "st");
        assert_eq!(ordinal_suffix(22), "nd");
        assert_eq!(ordinal_suffix(23), "rd");
        assert_eq!(ordinal_suffix(31), "st");
    }

    #[test]
    fn populate_datetime_includes_utc_time_variants() {
        let mut values = Map::new();
        populate_datetime(&mut values);
        let tu = values.get("time_utc").and_then(Value::as_str).unwrap_or("");
        let tmu = values
            .get("time_military_utc")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            tu.ends_with(" (UTC)"),
            "time_utc must end with ' (UTC)': {tu:?}"
        );
        assert!(
            tmu.ends_with(" (UTC)"),
            "time_military_utc must end with ' (UTC)': {tmu:?}"
        );
    }

    #[test]
    fn season_determination() {
        use super::format::determine_season;
        // Meteorological seasons
        assert_eq!(determine_season(1, 15), "Winter");
        assert_eq!(determine_season(3, 1), "Spring");
        assert_eq!(determine_season(6, 1), "Summer");
        assert_eq!(determine_season(9, 1), "Fall");
        assert_eq!(determine_season(12, 15), "Winter");
    }
}
