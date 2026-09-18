//! Runtime options for collecting and reporting recent commits.

use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, TimeZone, Utc};

use crate::filesystem::path_kind::ChangeCategory;
use crate::{Result, SniffError};

/// The number of commits selected when no selection is given.
pub const DEFAULT_RECENT_COMMIT_COUNT: usize = 10;

/// A calendar day named relative to the options' timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedDate {
    Today,
    Yesterday,
}

/// Which commits a collection starts from, before filters apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The newest `n` commits that pass every filter.
    Count(usize),
    /// Commits within this duration before now.
    Duration(Duration),
    /// A local calendar day; `Today` runs from local midnight to now.
    NamedDate(NamedDate),
    /// Exactly one local calendar day.
    Date(NaiveDate),
    /// From the tip back to and including this commit.
    Hash(String),
}

impl Default for Selection {
    fn default() -> Self {
        Self::Count(DEFAULT_RECENT_COMMIT_COUNT)
    }
}

impl Selection {
    /// Parse a CLI-style scope string.
    ///
    /// Precedence: `today`/`yesterday` (case-insensitive), `YYYY-MM-DD`, a
    /// positive all-digit count, a duration (`6h`, `3d`, `2 weeks`, `1mo`,
    /// `1y`), then a hexadecimal hash of at least 7 characters. An all-digit
    /// input is always a count, never a hash.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::filesystem::git::{NamedDate, Selection};
    ///
    /// assert_eq!(Selection::parse("25").unwrap(), Selection::Count(25));
    /// assert_eq!(
    ///     Selection::parse("Yesterday").unwrap(),
    ///     Selection::NamedDate(NamedDate::Yesterday)
    /// );
    /// assert!(Selection::parse("0").is_err());
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::InvalidPeriod`] for a zero count, a duration or
    /// count too large to represent, or input matching no form.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower == "today" {
            return Ok(Self::NamedDate(NamedDate::Today));
        }
        if lower == "yesterday" {
            return Ok(Self::NamedDate(NamedDate::Yesterday));
        }

        if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
            return Ok(Self::Date(date));
        }

        if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(count) = trimmed.parse::<usize>()
                && count > 0
            {
                return Ok(Self::Count(count));
            }
            return Err(SniffError::InvalidPeriod(trimmed.to_string()));
        }

        match parse_duration(&lower) {
            DurationScope::Parsed(duration) => return Ok(Self::Duration(duration)),
            DurationScope::OutOfRange => {
                return Err(SniffError::InvalidPeriod(trimmed.to_string()));
            }
            DurationScope::NoMatch => {}
        }

        if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(Self::Hash(trimmed.to_string()));
        }

        Err(SniffError::InvalidPeriod(trimmed.to_string()))
    }

    /// The UTC instants this selection covers, or `None` for count and hash
    /// selections, which are not time-bounded.
    ///
    /// Calendar days are computed in `timezone`; durations are relative to
    /// `now` and ignore it.
    pub(crate) fn time_window(
        &self,
        now: DateTime<Utc>,
        timezone: FixedOffset,
    ) -> Option<TimeWindow> {
        let local_today = now.with_timezone(&timezone).date_naive();
        match self {
            Self::Count(_) | Self::Hash(_) => None,
            Self::Duration(duration) => Some(TimeWindow {
                // A duration reaching past the earliest representable instant
                // covers all of history; plain subtraction would panic.
                since: now
                    .checked_sub_signed(*duration)
                    .unwrap_or(DateTime::<Utc>::MIN_UTC),
                until: WindowEnd::Inclusive(now),
            }),
            Self::NamedDate(NamedDate::Today) => Some(TimeWindow {
                since: local_midnight(local_today, timezone),
                until: WindowEnd::Inclusive(now),
            }),
            Self::NamedDate(NamedDate::Yesterday) => {
                Some(single_day(local_today.pred_opt()?, timezone))
            }
            Self::Date(date) => Some(single_day(*date, timezone)),
        }
    }
}

impl FromStr for Selection {
    type Err = SniffError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

/// A half-open or closed span of commit times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimeWindow {
    pub(crate) since: DateTime<Utc>,
    pub(crate) until: WindowEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowEnd {
    /// Up to and including this instant ("now").
    Inclusive(DateTime<Utc>),
    /// Strictly before this instant (the next local midnight).
    Exclusive(DateTime<Utc>),
}

impl TimeWindow {
    pub(crate) fn contains(&self, instant: DateTime<Utc>) -> bool {
        instant >= self.since
            && match self.until {
                WindowEnd::Inclusive(end) => instant <= end,
                WindowEnd::Exclusive(end) => instant < end,
            }
    }
}

fn local_midnight(date: NaiveDate, timezone: FixedOffset) -> DateTime<Utc> {
    // A fixed offset has no gaps or folds, so every local time maps to exactly
    // one instant.
    timezone
        .from_local_datetime(&date.and_time(chrono::NaiveTime::MIN))
        .single()
        .expect("a fixed offset maps every local time to one instant")
        .with_timezone(&Utc)
}

fn single_day(date: NaiveDate, timezone: FixedOffset) -> TimeWindow {
    let since = local_midnight(date, timezone);
    let until = date
        .succ_opt()
        .map(|next| local_midnight(next, timezone))
        .unwrap_or(DateTime::<Utc>::MAX_UTC);
    TimeWindow {
        since,
        until: WindowEnd::Exclusive(until),
    }
}

/// What reading a scope as a duration produced.
enum DurationScope {
    Parsed(Duration),
    /// Duration-shaped, but beyond what [`Duration`] can represent. Distinct
    /// from [`NoMatch`](Self::NoMatch) because the remaining selection forms
    /// must not claim such an input — `9223372036854775807d` is every
    /// character a hash allows.
    OutOfRange,
    /// Not duration-shaped; later selection forms may still match.
    NoMatch,
}

fn parse_duration(input: &str) -> DurationScope {
    let input = input.trim();

    let (num_str, unit): (&str, &str) = if let Some((n, u)) = split_number_unit(input) {
        (n, u)
    } else {
        let Some(pos) = input.find(char::is_alphabetic) else {
            return DurationScope::NoMatch;
        };
        input.split_at(pos)
    };
    let num_str = num_str.trim();

    // The unit is recognized before the count so that an unrecognized suffix
    // stays a non-match whatever its digits are.
    let seconds_per_unit: i64 = match unit.trim() {
        "h" | "hour" | "hours" => 3_600,
        "d" | "day" | "days" => 86_400,
        "w" | "wk" | "week" | "weeks" => 604_800,
        "mo" | "m" | "month" | "months" => 30 * 86_400,
        "y" | "yr" | "year" | "years" => 365 * 86_400,
        _ => return DurationScope::NoMatch,
    };

    let Ok(count) = num_str.parse::<i64>() else {
        return if !num_str.is_empty() && num_str.chars().all(|c| c.is_ascii_digit()) {
            DurationScope::OutOfRange
        } else {
            DurationScope::NoMatch
        };
    };
    if count <= 0 {
        return DurationScope::NoMatch;
    }

    match count
        .checked_mul(seconds_per_unit)
        .and_then(Duration::try_seconds)
    {
        Some(duration) => DurationScope::Parsed(duration),
        None => DurationScope::OutOfRange,
    }
}

fn split_number_unit(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let pos = input
        .find(char::is_whitespace)
        .or_else(|| input.find(char::is_alphabetic))?;
    let num_part = &input[..pos];
    let unit_part = input[pos..].trim_start();
    if unit_part.is_empty() || num_part.is_empty() {
        return None;
    }
    Some((num_part, unit_part))
}

/// How much of each commit a text report shows. JSON output ignores it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RecentCommitsVerbosity {
    /// The header line only.
    Compact,
    /// Header plus changed files.
    #[default]
    Normal,
    /// Header, description, bullet points, and changed files.
    Verbose,
}

/// Which files a text report lists; commits with no file in the projection
/// are omitted from that report. `to_json()` ignores it; apply it to JSON with
/// [`RecentCommits::projected`](super::RecentCommits::projected).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RecentCommitsProjection {
    #[default]
    All,
    SourceCode,
    Documentation,
}

/// Options for [`RecentCommits`](super::RecentCommits) collection and reporting.
///
/// Built with chained methods. The five selection methods ([`count`],
/// [`duration`], [`named_date`], [`date`], [`hash`]) replace one another —
/// the last call wins. Filters of different kinds combine with AND; repeated
/// [`operation`] calls accumulate and match any of the operations, while every
/// other single-valued filter keeps its last value. Repeated
/// [`has_file_type`] calls require every listed category.
///
/// [`count`]: Self::count
/// [`duration`]: Self::duration
/// [`named_date`]: Self::named_date
/// [`date`]: Self::date
/// [`hash`]: Self::hash
/// [`operation`]: Self::operation
/// [`has_file_type`]: Self::has_file_type
///
/// ## Examples
///
/// ```
/// use chrono::{Duration, FixedOffset};
/// use sniff::filesystem::git::{RecentCommitsOptions, RecentCommitsVerbosity};
///
/// let options = RecentCommitsOptions::new()
///     .duration(Duration::days(5))
///     .operation("fix")
///     .operation("feat")
///     .verbosity(RecentCommitsVerbosity::Compact)
///     .timezone(FixedOffset::west_opt(8 * 3600).unwrap());
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentCommitsOptions {
    pub(crate) selection: Selection,
    pub(crate) branch: Option<String>,
    pub(crate) operations: Vec<String>,
    pub(crate) scope: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) package: Option<String>,
    pub(crate) package_area: Option<String>,
    pub(crate) file_types: BTreeSet<ChangeCategory>,
    pub(crate) verbosity: RecentCommitsVerbosity,
    pub(crate) show_author: bool,
    pub(crate) projection: RecentCommitsProjection,
    pub(crate) timezone: FixedOffset,
}

impl Default for RecentCommitsOptions {
    /// The last [`DEFAULT_RECENT_COMMIT_COUNT`] commits from `HEAD`, no
    /// filters, normal verbosity, and the host's current local UTC offset.
    fn default() -> Self {
        Self {
            selection: Selection::default(),
            branch: None,
            operations: Vec::new(),
            scope: None,
            author: None,
            package: None,
            package_area: None,
            file_types: BTreeSet::new(),
            verbosity: RecentCommitsVerbosity::default(),
            show_author: false,
            projection: RecentCommitsProjection::default(),
            // The offset in effect now. A fixed offset cannot follow a DST
            // transition inside the selected range; see `timezone`.
            timezone: *Local::now().offset(),
        }
    }
}

impl RecentCommitsOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the selection with an already-built [`Selection`], such as one
    /// from [`Selection::parse`].
    pub fn selection(mut self, selection: Selection) -> Self {
        self.selection = selection;
        self
    }

    /// Select the newest `count` matching commits. Zero is rejected at
    /// collection time.
    pub fn count(self, count: usize) -> Self {
        self.selection(Selection::Count(count))
    }

    pub fn duration(self, duration: Duration) -> Self {
        self.selection(Selection::Duration(duration))
    }

    pub fn named_date(self, name: NamedDate) -> Self {
        self.selection(Selection::NamedDate(name))
    }

    /// Select exactly one local calendar day.
    pub fn date(self, date: NaiveDate) -> Self {
        self.selection(Selection::Date(date))
    }

    /// Select from the tip back to and including `hash`.
    pub fn hash(self, hash: impl Into<String>) -> Self {
        self.selection(Selection::Hash(hash.into()))
    }

    /// Walk history from this branch's tip instead of `HEAD`. A local branch
    /// wins over a remote-tracking branch of the same name.
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Keep conventional commits whose operation equals `operation`
    /// (case-insensitive). Repeated calls match any listed operation.
    pub fn operation(mut self, operation: impl Into<String>) -> Self {
        self.operations.push(operation.into());
        self
    }

    /// Keep conventional commits whose scope equals `scope`
    /// (case-insensitive).
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Keep commits whose author name or email contains `author`
    /// (case-insensitive).
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Keep commits touching a file owned by this monorepo package.
    pub fn package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// Keep commits touching a file owned by a package in this package area.
    pub fn package_area(mut self, package_area: impl Into<String>) -> Self {
        self.package_area = Some(package_area.into());
        self
    }

    /// Keep commits touching at least one file in `category`.
    pub fn has_file_type(mut self, category: ChangeCategory) -> Self {
        self.file_types.insert(category);
        self
    }

    pub fn verbosity(mut self, verbosity: RecentCommitsVerbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Add the author to each commit header in text reports.
    pub fn show_author(mut self, show_author: bool) -> Self {
        self.show_author = show_author;
        self
    }

    pub fn projection(mut self, projection: RecentCommitsProjection) -> Self {
        self.projection = projection;
        self
    }

    /// The offset used for calendar selections and human date labels. JSON
    /// datetimes stay UTC.
    ///
    /// A fixed offset cannot follow daylight-saving transitions, so a calendar
    /// day that spans one is bounded by a single offset.
    pub fn timezone(mut self, timezone: FixedOffset) -> Self {
        self.timezone = timezone;
        self
    }

    /// Reject selections no collection can satisfy.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::InvalidPeriod`] for `Count(0)`.
    pub(crate) fn validate(&self) -> Result<()> {
        if self.selection == Selection::Count(0) {
            return Err(SniffError::InvalidPeriod("0".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap()
    }

    fn east(hours: i32, minutes: i32) -> FixedOffset {
        FixedOffset::east_opt(hours * 3600 + minutes * 60).unwrap()
    }

    mod parsing {
        use super::*;

        #[test]
        fn named_dates_are_case_insensitive() {
            assert_eq!(
                Selection::parse("today").unwrap(),
                Selection::NamedDate(NamedDate::Today)
            );
            assert_eq!(
                Selection::parse(" YESTERDAY ").unwrap(),
                Selection::NamedDate(NamedDate::Yesterday)
            );
        }

        #[test]
        fn iso_date_wins_over_other_forms() {
            assert_eq!(
                Selection::parse("2026-04-01").unwrap(),
                Selection::Date(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap())
            );
        }

        #[test]
        fn all_digit_input_is_a_count_not_a_hash() {
            assert_eq!(Selection::parse("10").unwrap(), Selection::Count(10));
            assert_eq!(
                Selection::parse("12345678").unwrap(),
                Selection::Count(12_345_678)
            );
        }

        #[test]
        fn zero_count_is_rejected() {
            for input in ["0", "000"] {
                assert!(
                    matches!(Selection::parse(input), Err(SniffError::InvalidPeriod(_))),
                    "{input}"
                );
            }
        }

        #[test]
        fn overflowing_count_is_rejected() {
            assert!(matches!(
                Selection::parse("99999999999999999999999999"),
                Err(SniffError::InvalidPeriod(_))
            ));
        }

        #[test]
        fn durations_accept_every_unit_alias() {
            for (input, expected) in [
                ("6h", Duration::hours(6)),
                ("1 hour", Duration::hours(1)),
                ("3d", Duration::days(3)),
                ("2 days", Duration::days(2)),
                ("1w", Duration::weeks(1)),
                ("2wk", Duration::weeks(2)),
                ("1mo", Duration::days(30)),
                ("2m", Duration::days(60)),
                ("1y", Duration::days(365)),
                ("2 years", Duration::days(730)),
                ("3D", Duration::days(3)),
            ] {
                assert_eq!(
                    Selection::parse(input).unwrap(),
                    Selection::Duration(expected),
                    "{input}"
                );
            }
        }

        /// Each unit's largest accepted count and its seconds per unit.
        /// `Duration` tops out at `i64::MAX` milliseconds, so the accepted
        /// counts are `i64::MAX / 1000 / seconds_per_unit`.
        const DURATION_BOUNDS: [(&str, i64, i64); 5] = [
            ("h", 2_562_047_788_015, 3_600),
            ("d", 106_751_991_167, 86_400),
            ("w", 15_250_284_452, 604_800),
            ("mo", 3_558_399_705, 2_592_000),
            ("y", 292_471_208, 31_536_000),
        ];

        #[test]
        fn each_unit_accepts_its_largest_representable_count() {
            for (unit, largest, seconds_per_unit) in DURATION_BOUNDS {
                let input = format!("{largest}{unit}");
                assert_eq!(
                    Selection::parse(&input).unwrap(),
                    Selection::Duration(Duration::try_seconds(largest * seconds_per_unit).unwrap()),
                    "{input}"
                );
            }
        }

        #[test]
        fn each_unit_rejects_counts_past_its_largest_representable_one() {
            for (unit, largest, _) in DURATION_BOUNDS {
                // `d` is also a hex digit, so an oversized day count must not
                // fall through to the hash form. `u64::MAX` overflows the
                // count's own `i64` parse, one step before the multiplication.
                for count in [
                    (largest + 1).to_string(),
                    i64::MAX.to_string(),
                    u64::MAX.to_string(),
                ] {
                    let input = format!("{count}{unit}");
                    assert!(
                        matches!(
                            Selection::parse(&input),
                            Err(SniffError::InvalidPeriod(ref value)) if *value == input
                        ),
                        "{input} parsed as {:?}",
                        Selection::parse(&input)
                    );
                }
            }
        }

        #[test]
        fn short_hex_is_invalid_and_long_hex_is_a_hash() {
            assert!(Selection::parse("abc12").is_err());
            assert_eq!(
                Selection::parse("abc1234").unwrap(),
                Selection::Hash("abc1234".to_string())
            );
            assert_eq!(
                Selection::parse("ABCDEF0123").unwrap(),
                Selection::Hash("ABCDEF0123".to_string())
            );
        }

        #[test]
        fn malformed_inputs_are_invalid_periods() {
            for input in ["", "   ", "-3d", "0d", "3 fortnights", "2026-13-01", "not-a-scope"] {
                assert!(
                    matches!(Selection::parse(input), Err(SniffError::InvalidPeriod(_))),
                    "{input:?}"
                );
            }
        }

        #[test]
        fn from_str_matches_parse() {
            assert_eq!("3d".parse::<Selection>().unwrap(), Selection::parse("3d").unwrap());
            assert!("nope".parse::<Selection>().is_err());
        }
    }

    mod builder {
        use super::*;

        #[test]
        fn default_selects_the_last_ten_commits_unfiltered() {
            let options = RecentCommitsOptions::default();
            assert_eq!(options.selection, Selection::Count(10));
            assert_eq!(options.branch, None);
            assert!(options.operations.is_empty());
            assert_eq!(options.scope, None);
            assert_eq!(options.author, None);
            assert_eq!(options.package, None);
            assert_eq!(options.package_area, None);
            assert!(options.file_types.is_empty());
            assert_eq!(options.verbosity, RecentCommitsVerbosity::Normal);
            assert!(!options.show_author);
            assert_eq!(options.projection, RecentCommitsProjection::All);
            assert_eq!(options.timezone, *Local::now().offset());
            assert_eq!(RecentCommitsOptions::new(), options);
        }

        #[test]
        fn selection_methods_are_last_wins() {
            let date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
            let options = RecentCommitsOptions::new()
                .count(3)
                .duration(Duration::days(2))
                .hash("abc1234")
                .named_date(NamedDate::Today)
                .date(date);
            assert_eq!(options.selection, Selection::Date(date));

            let options = options.selection(Selection::parse("25").unwrap());
            assert_eq!(options.selection, Selection::Count(25));
        }

        #[test]
        fn operations_accumulate_and_single_valued_filters_keep_the_last_value() {
            let options = RecentCommitsOptions::new()
                .operation("fix")
                .operation("Planning")
                .scope("lib")
                .scope("cli")
                .author("ada")
                .author("grace")
                .package("a")
                .package("b")
                .package_area("x")
                .package_area("y")
                .branch("main")
                .branch("feature");

            assert_eq!(options.operations, ["fix", "Planning"]);
            assert_eq!(options.scope.as_deref(), Some("cli"));
            assert_eq!(options.author.as_deref(), Some("grace"));
            assert_eq!(options.package.as_deref(), Some("b"));
            assert_eq!(options.package_area.as_deref(), Some("y"));
            assert_eq!(options.branch.as_deref(), Some("feature"));
        }

        #[test]
        fn file_type_filters_accumulate_without_duplicates() {
            let options = RecentCommitsOptions::new()
                .has_file_type(ChangeCategory::Cicd)
                .has_file_type(ChangeCategory::SourceCode)
                .has_file_type(ChangeCategory::Cicd);
            assert_eq!(
                options.file_types.into_iter().collect::<Vec<_>>(),
                [ChangeCategory::SourceCode, ChangeCategory::Cicd]
            );
        }

        #[test]
        fn display_options_and_timezone_are_settable() {
            let offset = east(5, 30);
            let options = RecentCommitsOptions::new()
                .verbosity(RecentCommitsVerbosity::Verbose)
                .show_author(true)
                .projection(RecentCommitsProjection::Documentation)
                .timezone(offset);
            assert_eq!(options.verbosity, RecentCommitsVerbosity::Verbose);
            assert!(options.show_author);
            assert_eq!(options.projection, RecentCommitsProjection::Documentation);
            assert_eq!(options.timezone, offset);
        }

        #[test]
        fn validate_rejects_only_a_zero_count() {
            assert!(matches!(
                RecentCommitsOptions::new().count(0).validate(),
                Err(SniffError::InvalidPeriod(value)) if value == "0"
            ));
            for options in [
                RecentCommitsOptions::new(),
                RecentCommitsOptions::new().count(1),
                RecentCommitsOptions::new().duration(Duration::zero()),
                RecentCommitsOptions::new().hash("abc1234"),
            ] {
                assert!(options.validate().is_ok(), "{:?}", options.selection);
            }
        }
    }

    mod calendar_bounds {
        use super::*;

        #[test]
        fn count_and_hash_are_not_time_bounded() {
            let now = utc(2026, 9, 17, 12, 0, 0);
            assert_eq!(Selection::Count(5).time_window(now, east(0, 0)), None);
            assert_eq!(
                Selection::Hash("abc1234".into()).time_window(now, east(0, 0)),
                None
            );
        }

        #[test]
        fn today_starts_at_local_midnight_for_a_positive_offset() {
            // 2026-09-17T20:00Z is 2026-09-18T01:30 at +05:30, so "today" is
            // the 18th locally even though it is still the 17th in UTC.
            let now = utc(2026, 9, 17, 20, 0, 0);
            let window = Selection::NamedDate(NamedDate::Today)
                .time_window(now, east(5, 30))
                .unwrap();

            assert_eq!(window.since, utc(2026, 9, 17, 18, 30, 0));
            assert_eq!(window.until, WindowEnd::Inclusive(now));
            assert!(window.contains(utc(2026, 9, 17, 18, 30, 0)));
            assert!(!window.contains(utc(2026, 9, 17, 18, 29, 59)));
            assert!(window.contains(now));
            assert!(!window.contains(now + Duration::seconds(1)));
        }

        #[test]
        fn yesterday_is_one_local_day_ending_at_local_midnight() {
            // 2026-09-17T03:00Z is 2026-09-16T19:00 at -08:00.
            let now = utc(2026, 9, 17, 3, 0, 0);
            let window = Selection::NamedDate(NamedDate::Yesterday)
                .time_window(now, east(-8, 0))
                .unwrap();

            let start = utc(2026, 9, 15, 8, 0, 0);
            let end = utc(2026, 9, 16, 8, 0, 0);
            assert_eq!(window.since, start);
            assert_eq!(window.until, WindowEnd::Exclusive(end));
            assert!(window.contains(start));
            assert!(window.contains(end - Duration::seconds(1)), "23:59:59 local");
            assert!(!window.contains(end), "00:00 today local is excluded");
            assert!(!window.contains(start - Duration::seconds(1)));
            assert!(!window.contains(now));
        }

        #[test]
        fn specific_date_is_one_local_day_for_positive_and_negative_offsets() {
            let date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
            let now = utc(2026, 9, 17, 12, 0, 0);

            for (offset, start, end) in [
                (east(5, 30), utc(2026, 3, 31, 18, 30, 0), utc(2026, 4, 1, 18, 30, 0)),
                (east(-8, 0), utc(2026, 4, 1, 8, 0, 0), utc(2026, 4, 2, 8, 0, 0)),
                (east(0, 0), utc(2026, 4, 1, 0, 0, 0), utc(2026, 4, 2, 0, 0, 0)),
            ] {
                let window = Selection::Date(date).time_window(now, offset).unwrap();
                assert_eq!(window.since, start, "{offset}");
                assert_eq!(window.until, WindowEnd::Exclusive(end), "{offset}");
                assert!(window.contains(start), "{offset}");
                assert!(window.contains(end - Duration::seconds(1)), "{offset}");
                assert!(!window.contains(end), "{offset}");
                assert!(!window.contains(start - Duration::seconds(1)), "{offset}");
            }
        }

        #[test]
        fn boundary_commit_moves_between_days_with_the_offset() {
            // 2026-04-01T23:30 at -08:00 == 2026-04-02T07:30Z.
            let commit = utc(2026, 4, 2, 7, 30, 0);
            let now = utc(2026, 9, 17, 12, 0, 0);
            let april_first = Selection::Date(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
            let april_second = Selection::Date(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap());

            assert!(april_first.time_window(now, east(-8, 0)).unwrap().contains(commit));
            assert!(!april_second.time_window(now, east(-8, 0)).unwrap().contains(commit));
            assert!(!april_first.time_window(now, east(0, 0)).unwrap().contains(commit));
            assert!(april_second.time_window(now, east(0, 0)).unwrap().contains(commit));
        }

        #[test]
        fn durations_ignore_the_offset() {
            let now = utc(2026, 9, 17, 12, 0, 0);
            let selection = Selection::Duration(Duration::hours(6));
            let expected = TimeWindow {
                since: utc(2026, 9, 17, 6, 0, 0),
                until: WindowEnd::Inclusive(now),
            };

            assert_eq!(selection.time_window(now, east(5, 30)), Some(expected));
            assert_eq!(selection.time_window(now, east(-8, 0)), Some(expected));
        }

        #[test]
        fn a_duration_longer_than_history_starts_at_the_earliest_instant() {
            let now = utc(2026, 9, 17, 12, 0, 0);
            for input in ["2562047788015h", "106751991167d", "292471208y"] {
                let window = Selection::parse(input)
                    .unwrap()
                    .time_window(now, east(0, 0))
                    .unwrap();
                assert_eq!(window.since, DateTime::<Utc>::MIN_UTC, "{input}");
                assert_eq!(window.until, WindowEnd::Inclusive(now), "{input}");
                assert!(window.contains(DateTime::<Utc>::MIN_UTC), "{input}");
                assert!(window.contains(now), "{input}");
                assert!(!window.contains(now + Duration::seconds(1)), "{input}");
            }
        }

        #[test]
        fn zero_duration_includes_only_the_current_instant() {
            let now = utc(2026, 9, 17, 12, 0, 0);
            let window = Selection::Duration(Duration::zero())
                .time_window(now, east(0, 0))
                .unwrap();
            assert!(window.contains(now));
            assert!(!window.contains(now - Duration::seconds(1)));
        }
    }
}
