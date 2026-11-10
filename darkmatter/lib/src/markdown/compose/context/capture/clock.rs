//! The request's one reading of the clock and of the host's UTC offset.
//!
//! `ctx.now`, `ctx.today`, and the date functions (`is_today()`, `date()`'s
//! optional year, offset conversion in `date_delta()`) all read the same
//! reading, so a request that crosses midnight sees one date, and a schema
//! slot that calls a date function depends on a captured value rather than
//! on the live clock.

use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, Utc};

/// One instant and the host's UTC offset at that instant.
///
/// The offset is fixed for the request: an instant from another season is
/// converted to a local date with this offset, not with the host's daylight
/// saving rules for that season.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClockReading {
    utc: DateTime<Utc>,
    offset: FixedOffset,
}

impl ClockReading {
    /// Reads the live clock and the host's current offset.
    pub(crate) fn now() -> Self {
        let local = Local::now();
        Self { utc: local.with_timezone(&Utc), offset: *local.offset() }
    }

    /// A reading at `utc` with the host offset `offset`.
    #[cfg(test)]
    pub(crate) fn at(utc: DateTime<Utc>, offset: FixedOffset) -> Self {
        Self { utc, offset }
    }

    /// The instant in the host's captured offset.
    pub(crate) fn local(&self) -> DateTime<FixedOffset> {
        self.utc.with_timezone(&self.offset)
    }

    /// The instant in UTC.
    pub(crate) fn utc(&self) -> DateTime<Utc> {
        self.utc
    }

    /// Today's calendar date, in UTC or in the captured offset.
    pub(crate) fn today(&self, utc: bool) -> NaiveDate {
        if utc { self.utc.date_naive() } else { self.local().date_naive() }
    }

    /// The local calendar year.
    pub(crate) fn local_year(&self) -> i32 {
        self.local().year()
    }

    /// The offset a dated instant is converted with: UTC, or the captured
    /// host offset.
    pub(crate) fn zone(&self, utc: bool) -> FixedOffset {
        if utc { FixedOffset::east_opt(0).expect("zero is a valid offset") } else { self.offset }
    }
}

/// A clock reading shared by every clone: taken when first needed (or
/// supplied by the capture), then the same for the rest of the request.
#[derive(Debug, Clone, Default)]
pub(crate) struct CapturedClock(Arc<OnceLock<ClockReading>>);

impl CapturedClock {
    /// A clock that already holds `reading`.
    pub(crate) fn seeded(reading: ClockReading) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(reading);
        Self(Arc::new(cell))
    }

    /// The request's reading, taken now if no one has read it yet.
    pub(crate) fn reading(&self) -> ClockReading {
        *self.0.get_or_init(ClockReading::now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_clone_sees_the_first_reading() {
        let clock = CapturedClock::default();
        let first = clock.clone().reading();
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert_eq!(clock.reading(), first);
    }

    #[test]
    fn a_reading_crossing_midnight_in_its_offset_has_a_different_local_date() {
        let utc = DateTime::parse_from_rfc3339("2024-06-15T23:30:00Z").unwrap().with_timezone(&Utc);
        let reading = ClockReading::at(utc, FixedOffset::east_opt(3600).unwrap());
        assert_eq!(reading.today(true), NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
        assert_eq!(reading.today(false), NaiveDate::from_ymd_opt(2024, 6, 16).unwrap());
        assert_eq!(reading.local_year(), 2024);
    }
}
