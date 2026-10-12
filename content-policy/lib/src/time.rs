//! Date arithmetic and the 00:00 UTC boundary.
//!
//! Every date takes effect at the start of its day, 00:00 UTC, so the named
//! day is not included and no result depends on the host's timezone. Units
//! match Darkmatter's expression durations (`parse_duration_spec`): `mo` and
//! `yr` are calendar months and years, clamped to the destination month's
//! last day; `d` and `wk` are day increments.

use chrono::{DateTime, Days, Months, NaiveDate, Utc};

use crate::model::{Duration, DurationUnit};

/// The instant a date takes effect: 00:00 UTC on that day.
#[must_use]
pub fn start_of_day(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .expect("midnight exists on every date")
        .and_utc()
}

/// Adds `duration` to `date`, or `None` outside chrono's date range.
#[must_use]
pub fn add_duration(date: NaiveDate, duration: Duration) -> Option<NaiveDate> {
    let count = duration.count();
    match duration.unit() {
        DurationUnit::Days => date.checked_add_days(Days::new(u64::from(count))),
        DurationUnit::Weeks => date.checked_add_days(Days::new(u64::from(count) * 7)),
        DurationUnit::Months => date.checked_add_months(Months::new(count)),
        DurationUnit::Years => date.checked_add_months(Months::new(count.checked_mul(12)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn duration(count: u32, unit: DurationUnit) -> Duration {
        Duration::new(count, unit).unwrap()
    }

    /// Cross-check against Darkmatter's `add_duration_spec` rule (calendar
    /// months first, clamped, then a fixed day span), written out as a table
    /// because content-policy may not depend on Darkmatter.
    #[test]
    fn calendar_arithmetic_matches_darkmatter_durations() {
        use DurationUnit::*;
        for (start, count, unit, expected) in [
            ("2026-01-31", 1, Months, "2026-02-28"),
            ("2024-01-31", 1, Months, "2024-02-29"),
            ("2026-03-31", 1, Months, "2026-04-30"),
            ("2026-08-31", 3, Months, "2026-11-30"),
            ("2026-09-28", 3, Months, "2026-12-28"),
            ("2026-12-29", 3, Months, "2027-03-29"),
            ("2026-11-30", 3, Months, "2027-02-28"),
            ("2024-02-29", 1, Years, "2025-02-28"),
            ("2024-02-29", 4, Years, "2028-02-29"),
            ("2026-09-28", 1, Years, "2027-09-28"),
            ("2026-12-31", 1, Days, "2027-01-01"),
            ("2024-02-28", 1, Days, "2024-02-29"),
            ("2026-02-28", 1, Days, "2026-03-01"),
            ("2026-09-28", 2, Weeks, "2026-10-12"),
            ("2026-12-28", 1, Weeks, "2027-01-04"),
        ] {
            assert_eq!(
                add_duration(date(start), duration(count, unit)),
                Some(date(expected)),
                "{start} + {count}{}",
                unit.suffix()
            );
        }
    }

    #[test]
    fn out_of_range_is_none() {
        assert_eq!(add_duration(date("2026-01-01"), duration(u32::MAX, DurationUnit::Years)), None);
        assert_eq!(add_duration(NaiveDate::MAX, duration(1, DurationUnit::Days)), None);
    }

    #[test]
    fn a_date_starts_at_utc_midnight() {
        assert_eq!(
            start_of_day(date("2027-01-01")).to_rfc3339(),
            "2027-01-01T00:00:00+00:00"
        );
    }
}
