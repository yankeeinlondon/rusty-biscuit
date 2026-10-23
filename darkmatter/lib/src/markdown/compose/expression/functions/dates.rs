//! Date formatting, validators, and arithmetic: pure handlers over bound
//! arguments.

use chrono::{Datelike, Duration, Local, Months, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;

use super::{
    FunctionBinding, bound, is_this_month_with, is_this_year_with, is_today_with, is_tomorrow_with, is_yesterday_with,
    parse_date_or_datetime, parse_iso_date, parse_iso_datetime,
};
use crate::markdown::compose::expression::call_binding::BoundCall;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::pure("date", &[], date),
    FunctionBinding::pure("is_date", &["isdate"], is_date),
    FunctionBinding::pure("is_date_utc", &["isdateutc"], is_date),
    FunctionBinding::pure("is_date_time", &["isdatetime", "is_datetime"], is_date_time),
    FunctionBinding::pure("is_date_time_utc", &["isdatetimeutc", "is_datetime_utc"], is_date_time),
    FunctionBinding::pure("is_today", &["istoday"], is_today),
    FunctionBinding::pure("is_today_utc", &["istodayutc"], is_today_utc),
    FunctionBinding::pure("is_yesterday", &["isyesterday"], is_yesterday),
    FunctionBinding::pure("is_yesterday_utc", &["isyesterdayutc"], is_yesterday_utc),
    FunctionBinding::pure("is_tomorrow", &["istomorrow"], is_tomorrow),
    FunctionBinding::pure("is_tomorrow_utc", &["istomorrowutc"], is_tomorrow_utc),
    FunctionBinding::pure("is_this_month", &["isthismonth"], is_this_month),
    FunctionBinding::pure("is_this_month_utc", &["isthismonthutc"], is_this_month_utc),
    FunctionBinding::pure("is_this_year", &["isthisyear"], is_this_year),
    FunctionBinding::pure("is_this_year_utc", &["isthisyearutc"], is_this_year_utc),
    FunctionBinding::pure("date_delta", &["datedelta"], date_delta),
    FunctionBinding::pure("older_than", &["olderthan"], older_than),
    FunctionBinding::pure("newer_than", &["newerthan"], newer_than),
];

/// Naive datetime spellings the engine's `datetime` type admits that
/// [`parse_date_or_datetime`] does not read (no seconds, or a space instead
/// of `T`).
const EXTRA_DATETIME_FORMATS: &[&str] = &["%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M"];

/// The calendar date of a bound `date | datetime` text, read in local time:
/// every spelling the binder admits for those types has one.
fn bound_date(text: &str) -> Option<NaiveDate> {
    parse_date_or_datetime(text, false).or_else(|| {
        EXTRA_DATETIME_FORMATS
            .iter()
            .find_map(|format| NaiveDateTime::parse_from_str(text, format).ok())
            .map(|datetime| datetime.date())
    })
}

/// English ordinal suffix for a day-of-month (1..=31). 11/12/13 are "th".
fn ordinal_suffix(day: u32) -> &'static str {
    match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}

/// Joins non-empty parts with a single space (used for the optional year).
fn join_nonempty(parts: &[String]) -> String {
    parts.iter().filter(|part| !part.is_empty()).cloned().collect::<Vec<_>>().join(" ")
}

/// `date(iso, fmt)`: reformats a bound ISO date or datetime into a named
/// human format; null in either argument passes through.
///
/// The `fmt` enum is `"MMMM Do"`/`short`, `"MMMM Do [YYYY]"`/`short-optional`,
/// `"MMMM Do YYYY"`, `"D MMMM [YYYY]"`, `"D MMMM YYYY"`, and
/// `"ddd, MMMM Do, YYYY"`/`long`. The `[YYYY]` token includes the year only
/// when it differs from the current year.
fn date(call: &BoundCall<'_>) -> Result<Value, String> {
    let args = call.args();
    let (Some(iso), Some(format)) = (args.text(0), args.text(1)) else {
        return Ok(Value::Null);
    };
    let parsed = bound_date(iso).ok_or_else(|| format!("date() invalid ISO date or datetime: {iso:?}"))?;

    let month = parsed.format("%B").to_string();
    let weekday = parsed.format("%a").to_string();
    let day = parsed.day();
    let day_ordinal = format!("{day}{}", ordinal_suffix(day));
    let year = parsed.year();
    let optional_year = if year == Local::now().year() { String::new() } else { year.to_string() };

    let out = match format {
        "MMMM Do" | "short" => format!("{month} {day_ordinal}"),
        "MMMM Do [YYYY]" | "short-optional" => join_nonempty(&[format!("{month} {day_ordinal}"), optional_year]),
        "MMMM Do YYYY" => format!("{month} {day_ordinal} {year}"),
        "D MMMM [YYYY]" => join_nonempty(&[format!("{day} {month}"), optional_year]),
        "D MMMM YYYY" => format!("{day} {month} {year}"),
        "ddd, MMMM Do, YYYY" | "long" => format!("{weekday}, {month} {day_ordinal}, {year}"),
        other => return Err(format!("date() unknown format: {other:?}")),
    };
    Ok(Value::String(out))
}

/// The one `unknown | null` argument a date validator inspects as sent.
fn inspected<'a>(call: &'a BoundCall<'_>) -> Result<&'a Value, String> {
    bound(call.args().get(0), "date validator", 0)
}

/// `is_date(x)` / `is_date_utc(x)`: strict `YYYY-MM-DD` text (the format is
/// timezone-agnostic). Only text can be `true`.
fn is_date(call: &BoundCall<'_>) -> Result<Value, String> {
    let ok = inspected(call)?.as_str().is_some_and(|text| parse_iso_date(text).is_some());
    Ok(Value::Bool(ok))
}

/// `is_date_time(x)` / `is_date_time_utc(x)`: strict ISO datetime text.
fn is_date_time(call: &BoundCall<'_>) -> Result<Value, String> {
    Ok(Value::Bool(inspected(call)?.as_str().is_some_and(parse_iso_datetime)))
}

type RelativeDate = fn(&Value, NaiveDate, bool) -> bool;

fn relative(call: &BoundCall<'_>, test: RelativeDate, utc: bool) -> Result<Value, String> {
    let today = if utc { Utc::now().date_naive() } else { Local::now().date_naive() };
    Ok(Value::Bool(test(inspected(call)?, today, utc)))
}

fn is_today(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_today_with, false)
}

fn is_today_utc(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_today_with, true)
}

fn is_yesterday(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_yesterday_with, false)
}

fn is_yesterday_utc(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_yesterday_with, true)
}

fn is_tomorrow(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_tomorrow_with, false)
}

fn is_tomorrow_utc(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_tomorrow_with, true)
}

fn is_this_month(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_this_month_with, false)
}

fn is_this_month_utc(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_this_month_with, true)
}

fn is_this_year(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_this_year_with, false)
}

fn is_this_year_utc(call: &BoundCall<'_>) -> Result<Value, String> {
    relative(call, is_this_year_with, true)
}

// ── Date arithmetic ─────────────────────────────────────────────────
//
// `date_delta`/`older_than`/`newer_than` share the signature
// `(date1, date2, diff)` where `diff` is a duration string (`14d`, `2mo`,
// `1 hour`). They differ only in direction: `date_delta` is magnitude-only
// (are the two dates at least `diff` apart?), `older_than` asks whether `date1`
// precedes `date2` by at least `diff`, `newer_than` the reverse. The threshold
// is inclusive (`>=`). Month/year components are calendar-aware (chrono
// `Months`); day-and-smaller components are fixed `Duration`s. Dates reduce to
// their local calendar date and anchor at midnight, so a sub-day `diff` still
// compares meaningfully.

/// A parsed `diff` duration: a calendar-month component plus a fixed span.
struct DurationSpec {
    months: u32,
    fixed: Duration,
}

/// Parses a single-term duration string like `14d`, `2mo`, or `1 hour`.
///
/// Grammar: a non-negative integer count, optional whitespace, then a
/// case-insensitive unit. `mo`/`month(s)` are months and `y`/`year(s)` are
/// years (both calendar-aware); `m`/`min`/`minute(s)` are minutes — so the
/// bare `m` always means minutes, never months.
fn parse_duration_spec(s: &str) -> Result<DurationSpec, String> {
    let trimmed = s.trim();
    let digits_end = trimmed.find(|c: char| !c.is_ascii_digit()).unwrap_or(trimmed.len());
    if digits_end == 0 {
        return Err(format!("expected a leading number in {trimmed:?}"));
    }
    let count: i64 = trimmed[..digits_end].parse().map_err(|_| format!("invalid count in {trimmed:?}"))?;
    let unit = trimmed[digits_end..].trim().to_ascii_lowercase();
    let overflow = || format!("duration overflow in {trimmed:?}");
    let (months, fixed): (i64, Duration) = match unit.as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => (0, Duration::try_seconds(count).ok_or_else(overflow)?),
        "m" | "min" | "mins" | "minute" | "minutes" => (0, Duration::try_minutes(count).ok_or_else(overflow)?),
        "h" | "hr" | "hrs" | "hour" | "hours" => (0, Duration::try_hours(count).ok_or_else(overflow)?),
        "d" | "day" | "days" => (0, Duration::try_days(count).ok_or_else(overflow)?),
        "w" | "wk" | "wks" | "week" | "weeks" => (0, Duration::try_weeks(count).ok_or_else(overflow)?),
        "mo" | "mos" | "month" | "months" => (count, Duration::zero()),
        "y" | "yr" | "yrs" | "year" | "years" => (count.checked_mul(12).ok_or_else(overflow)?, Duration::zero()),
        "" => return Err(format!("missing unit in {trimmed:?}")),
        other => return Err(format!("unknown duration unit {other:?}")),
    };
    let months = u32::try_from(months).map_err(|_| overflow())?;
    Ok(DurationSpec { months, fixed })
}

/// Adds a parsed duration to a base instant (calendar months first, then the
/// fixed span). Returns `None` on overflow.
fn add_duration_spec(base: NaiveDateTime, spec: &DurationSpec) -> Option<NaiveDateTime> {
    base.checked_add_months(Months::new(spec.months))?.checked_add_signed(spec.fixed)
}

/// The two midnight-anchored instants and the parsed duration of a bound
/// date-arithmetic call.
fn two_dates_and_duration(
    call: &BoundCall<'_>,
    name: &str,
) -> Result<(NaiveDateTime, NaiveDateTime, DurationSpec), String> {
    let args = call.args();
    let instant = |index: usize| -> Result<NaiveDateTime, String> {
        let text = bound(args.text(index), name, index)?;
        bound_date(text)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .ok_or_else(|| format!("{name}() invalid ISO date or datetime: {text:?}"))
    };
    let diff = bound(args.text(2), name, 2)?;
    let spec = parse_duration_spec(diff).map_err(|reason| format!("{name}() invalid duration {diff:?}: {reason}"))?;
    Ok((instant(0)?, instant(1)?, spec))
}

/// `base + spec`, or the overflow domain error of `name`.
fn threshold(base: NaiveDateTime, spec: &DurationSpec, name: &str) -> Result<NaiveDateTime, String> {
    add_duration_spec(base, spec).ok_or_else(|| format!("{name}() duration arithmetic overflowed"))
}

/// `date_delta(date1, date2, diff)`: true when the two dates are at least
/// `diff` apart, regardless of order.
fn date_delta(call: &BoundCall<'_>) -> Result<Value, String> {
    let (a, b, spec) = two_dates_and_duration(call, "date_delta")?;
    let (earlier, later) = if a <= b { (a, b) } else { (b, a) };
    Ok(Value::Bool(later >= threshold(earlier, &spec, "date_delta")?))
}

/// `older_than(date1, date2, diff)`: true when `date1` is at least `diff`
/// older (earlier) than `date2`.
fn older_than(call: &BoundCall<'_>) -> Result<Value, String> {
    let (first, second, spec) = two_dates_and_duration(call, "older_than")?;
    Ok(Value::Bool(second >= threshold(first, &spec, "older_than")?))
}

/// `newer_than(date1, date2, diff)`: true when `date1` is at least `diff`
/// newer (later) than `date2`.
fn newer_than(call: &BoundCall<'_>) -> Result<Value, String> {
    let (first, second, spec) = two_dates_and_duration(call, "newer_than")?;
    Ok(Value::Bool(first >= threshold(second, &spec, "newer_than")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_suffix_covers_teens_and_units() {
        let cases = [(1, "st"), (2, "nd"), (3, "rd"), (4, "th"), (11, "th"), (12, "th"), (13, "th"), (21, "st")];
        for (day, suffix) in cases {
            assert_eq!(ordinal_suffix(day), suffix, "{day}");
        }
        assert_eq!(ordinal_suffix(22), "nd");
        assert_eq!(ordinal_suffix(23), "rd");
    }

    #[test]
    fn duration_counts_that_overflow_are_domain_errors_not_panics() {
        assert!(parse_duration_spec("99999999999999999d").is_err());
        assert!(parse_duration_spec("9999999999y").is_err());
    }
}
