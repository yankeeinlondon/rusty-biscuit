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
