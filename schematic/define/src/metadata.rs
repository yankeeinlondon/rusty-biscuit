//! Descriptive rate-limit and retry metadata for REST APIs.
//!
//! These types only describe how a service reports quota and which responses
//! a caller may retry; nothing in this crate reads the headers, sleeps, or
//! retries. Every constructor validates, and deserialization runs the same
//! validation, so a [`RateLimit`] or [`RetryGuidance`] obtained either way is
//! valid. Because the fields are public, a value mutated in place should be
//! re-checked with `validate`.
//!
//! An endpoint-level declaration replaces the API-level one as a whole; the
//! two are never merged field by field.

use serde::{Deserialize, Serialize};

/// Lowest and highest HTTP status codes a selector may name.
const STATUS_MIN: u16 = 100;
const STATUS_MAX: u16 = 599;

/// Why a rate-limit or retry declaration is invalid.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MetadataError {
    /// A header field is empty or is not a legal HTTP token.
    #[error("`{field}` is not a valid HTTP header name: {value:?}")]
    InvalidHeaderName { field: &'static str, value: String },
    /// A status code lies outside `100..=599`.
    #[error("`{field}` status code {value} is outside 100..=599")]
    StatusOutOfRange { field: &'static str, value: u16 },
    /// A status range's `start` is greater than its `end`.
    #[error("status range `start` {start} is greater than `end` {end}")]
    ReversedRange { start: u16, end: u16 },
    /// `max_attempts` is zero; it counts the initial attempt, so at least 1.
    #[error("`max_attempts` must be at least 1, got 0")]
    ZeroMaxAttempts,
    /// A backoff duration that must be positive is zero.
    #[error("backoff `{field}` must be greater than 0 ms")]
    ZeroDelay { field: &'static str },
    /// An exponential backoff caps below its base delay.
    #[error("backoff `cap_ms` {cap_ms} is less than `base_ms` {base_ms}")]
    CapBelowBase { base_ms: u64, cap_ms: u64 },
}

fn check_header(field: &'static str, value: &str) -> Result<(), MetadataError> {
    if crate::builders::valid_header_name(value) {
        Ok(())
    } else {
        Err(MetadataError::InvalidHeaderName { field, value: value.to_owned() })
    }
}

fn check_status(field: &'static str, value: u16) -> Result<(), MetadataError> {
    if (STATUS_MIN..=STATUS_MAX).contains(&value) {
        Ok(())
    } else {
        Err(MetadataError::StatusOutOfRange { field, value })
    }
}

/// How the value of [`RateLimit::reset_header`] expresses the reset moment.
///
/// It applies only to the reset header; a `Retry-After` header carries its
/// own format defined by HTTP.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetUnit {
    /// Seconds remaining until the window resets.
    Seconds,
    /// Unix timestamp, in seconds, at which the window resets.
    EpochSeconds,
    /// An HTTP-date (RFC 9110) at which the window resets.
    HttpDate,
}

/// The response headers a service uses to report its rate limit.
///
/// Serialized as an object with every member required except
/// `retry_after_header`, which may be omitted but never `null`.
///
/// ## Examples
///
/// ```
/// use schematic_define::metadata::{RateLimit, ResetUnit};
///
/// let github = RateLimit::new(
///     "x-ratelimit-limit",
///     "x-ratelimit-remaining",
///     "x-ratelimit-reset",
///     ResetUnit::EpochSeconds,
/// )?
/// .with_retry_after_header("retry-after")?;
///
/// assert_eq!(github.retry_after_header.as_deref(), Some("retry-after"));
/// assert!(RateLimit::new("", "r", "s", ResetUnit::Seconds).is_err());
/// # Ok::<(), schematic_define::metadata::MetadataError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawRateLimit")]
#[non_exhaustive]
pub struct RateLimit {
    /// Header carrying the request quota for the current window.
    pub limit_header: String,
    /// Header carrying the requests left in the current window.
    pub remaining_header: String,
    /// Header carrying when the window resets, in [`RateLimit::reset_unit`].
    pub reset_header: String,
    /// Header a throttled response uses to say how long to wait, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_header: Option<String>,
    /// Format of the reset header's value.
    pub reset_unit: ResetUnit,
}

impl RateLimit {
    /// Declares the three quota headers with no retry-after header.
    ///
    /// ## Errors
    ///
    /// [`MetadataError::InvalidHeaderName`] when any name is empty or not an
    /// HTTP token.
    pub fn new(
        limit_header: impl Into<String>,
        remaining_header: impl Into<String>,
        reset_header: impl Into<String>,
        reset_unit: ResetUnit,
    ) -> Result<Self, MetadataError> {
        let rate_limit = Self {
            limit_header: limit_header.into(),
            remaining_header: remaining_header.into(),
            reset_header: reset_header.into(),
            retry_after_header: None,
            reset_unit,
        };
        rate_limit.validate()?;
        Ok(rate_limit)
    }

    /// Adds the header a throttled response uses to say how long to wait.
    ///
    /// ## Errors
    ///
    /// [`MetadataError::InvalidHeaderName`] when `name` is empty or not an
    /// HTTP token.
    pub fn with_retry_after_header(mut self, name: impl Into<String>) -> Result<Self, MetadataError> {
        let name = name.into();
        check_header("retry_after_header", &name)?;
        self.retry_after_header = Some(name);
        Ok(self)
    }

    /// Checks every header name.
    ///
    /// ## Errors
    ///
    /// [`MetadataError::InvalidHeaderName`] naming the first invalid field.
    pub fn validate(&self) -> Result<(), MetadataError> {
        check_header("limit_header", &self.limit_header)?;
        check_header("remaining_header", &self.remaining_header)?;
        check_header("reset_header", &self.reset_header)?;
        if let Some(name) = &self.retry_after_header {
            check_header("retry_after_header", name)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRateLimit {
    limit_header: String,
    remaining_header: String,
    reset_header: String,
    #[serde(default, deserialize_with = "crate::serde_presence::non_null")]
    retry_after_header: Option<String>,
    reset_unit: ResetUnit,
}

impl TryFrom<RawRateLimit> for RateLimit {
    type Error = MetadataError;

    fn try_from(raw: RawRateLimit) -> Result<Self, Self::Error> {
        let rate_limit = Self {
            limit_header: raw.limit_header,
            remaining_header: raw.remaining_header,
            reset_header: raw.reset_header,
            retry_after_header: raw.retry_after_header,
            reset_unit: raw.reset_unit,
        };
        rate_limit.validate()?;
        Ok(rate_limit)
    }
}

/// Response statuses a caller may retry.
///
/// Serialized externally tagged: `{"code": 429}` or
/// `{"range": {"start": 500, "end": 599}}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StatusSelector {
    /// One status code.
    Code(u16),
    /// Every status from `start` to `end`, both inclusive.
    Range { start: u16, end: u16 },
}

impl StatusSelector {
    /// Checks that codes lie in `100..=599` and a range is not reversed.
    ///
    /// ## Errors
    ///
    /// [`MetadataError::StatusOutOfRange`] or [`MetadataError::ReversedRange`].
    pub fn validate(&self) -> Result<(), MetadataError> {
        match *self {
            Self::Code(code) => check_status("code", code),
            Self::Range { start, end } => {
                check_status("range.start", start)?;
                check_status("range.end", end)?;
                if start > end {
                    return Err(MetadataError::ReversedRange { start, end });
                }
                Ok(())
            }
        }
    }
}

/// Delay between retry attempts.
///
/// Serialized externally tagged: `"none"`, `{"fixed": {"delay_ms": 500}}`, or
/// `{"exponential": {"base_ms": 500, "cap_ms": 30000}}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Backoff {
    /// Retry immediately; an explicit choice, not an absent value.
    None,
    /// The same delay before every retry.
    Fixed { delay_ms: u64 },
    /// A delay that grows from `base_ms` and never exceeds `cap_ms`.
    Exponential { base_ms: u64, cap_ms: u64 },
}

impl Backoff {
    /// Checks that delays are positive and the cap is not below the base.
    ///
    /// ## Errors
    ///
    /// [`MetadataError::ZeroDelay`] or [`MetadataError::CapBelowBase`].
    pub fn validate(&self) -> Result<(), MetadataError> {
        match *self {
            Self::None => Ok(()),
            Self::Fixed { delay_ms: 0 } => Err(MetadataError::ZeroDelay { field: "delay_ms" }),
            Self::Fixed { .. } => Ok(()),
            Self::Exponential { base_ms: 0, .. } => Err(MetadataError::ZeroDelay { field: "base_ms" }),
            Self::Exponential { base_ms, cap_ms } if cap_ms < base_ms => {
                Err(MetadataError::CapBelowBase { base_ms, cap_ms })
            }
            Self::Exponential { .. } => Ok(()),
        }
    }
}

/// Which failures a caller may retry and how.
///
/// An empty `retry_on` is valid and selects no status. Serialized as an
/// object whose members are all required.
///
/// ## Examples
///
/// ```
/// use schematic_define::metadata::{Backoff, RetryGuidance, StatusSelector};
///
/// let guidance = RetryGuidance::new(
///     [StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }],
///     true,
///     3,
///     Backoff::Exponential { base_ms: 1_000, cap_ms: 60_000 },
/// )?;
///
/// assert_eq!(guidance.max_attempts, 3);
/// assert!(RetryGuidance::new([], false, 0, Backoff::None).is_err());
/// # Ok::<(), schematic_define::metadata::MetadataError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawRetryGuidance")]
#[non_exhaustive]
pub struct RetryGuidance {
    /// Statuses that may be retried.
    pub retry_on: Vec<StatusSelector>,
    /// Whether a server-sent retry-after delay takes precedence over `backoff`.
    pub honor_retry_after: bool,
    /// Total attempts including the first; at least 1.
    pub max_attempts: u8,
    /// Delay between attempts.
    pub backoff: Backoff,
}

impl RetryGuidance {
    /// Declares retry guidance.
    ///
    /// ## Errors
    ///
    /// Any [`MetadataError`] that [`RetryGuidance::validate`] reports.
    pub fn new(
        retry_on: impl IntoIterator<Item = StatusSelector>,
        honor_retry_after: bool,
        max_attempts: u8,
        backoff: Backoff,
    ) -> Result<Self, MetadataError> {
        let guidance = Self { retry_on: retry_on.into_iter().collect(), honor_retry_after, max_attempts, backoff };
        guidance.validate()?;
        Ok(guidance)
    }

    /// Checks every selector, the attempt count, and the backoff.
    ///
    /// ## Errors
    ///
    /// The first [`MetadataError`] found.
    pub fn validate(&self) -> Result<(), MetadataError> {
        for selector in &self.retry_on {
            selector.validate()?;
        }
        if self.max_attempts == 0 {
            return Err(MetadataError::ZeroMaxAttempts);
        }
        self.backoff.validate()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRetryGuidance {
    retry_on: Vec<StatusSelector>,
    honor_retry_after: bool,
    max_attempts: u8,
    backoff: Backoff,
}

impl TryFrom<RawRetryGuidance> for RetryGuidance {
    type Error = MetadataError;

    fn try_from(raw: RawRetryGuidance) -> Result<Self, Self::Error> {
        let guidance = Self {
            retry_on: raw.retry_on,
            honor_retry_after: raw.honor_retry_after,
            max_attempts: raw.max_attempts,
            backoff: raw.backoff,
        };
        guidance.validate()?;
        Ok(guidance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn github_rate_limit() -> RateLimit {
        RateLimit::new("x-ratelimit-limit", "x-ratelimit-remaining", "x-ratelimit-reset", ResetUnit::EpochSeconds)
            .and_then(|rate_limit| rate_limit.with_retry_after_header("retry-after"))
            .expect("GitHub rate limit is valid")
    }

    fn full_guidance() -> RetryGuidance {
        RetryGuidance::new(
            [StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }],
            true,
            5,
            Backoff::Exponential { base_ms: 250, cap_ms: 8_000 },
        )
        .expect("guidance is valid")
    }

    fn guidance_with(selector: StatusSelector) -> Result<RetryGuidance, MetadataError> {
        RetryGuidance::new([selector], false, 1, Backoff::None)
    }

    #[test]
    fn status_code_boundaries() {
        for (code, ok) in [(99, false), (100, true), (599, true), (600, false)] {
            assert_eq!(guidance_with(StatusSelector::Code(code)).is_ok(), ok, "code {code}");
            assert_eq!(guidance_with(StatusSelector::Range { start: code, end: 599 }).is_ok(), ok, "start {code}");
            assert_eq!(guidance_with(StatusSelector::Range { start: 100, end: code }).is_ok(), ok, "end {code}");
        }
        assert_eq!(
            guidance_with(StatusSelector::Code(600)),
            Err(MetadataError::StatusOutOfRange { field: "code", value: 600 })
        );
    }

    #[test]
    fn range_order() {
        assert_eq!(
            guidance_with(StatusSelector::Range { start: 503, end: 502 }),
            Err(MetadataError::ReversedRange { start: 503, end: 502 })
        );
        assert!(guidance_with(StatusSelector::Range { start: 503, end: 503 }).is_ok());
    }

    #[test]
    fn max_attempts_boundaries() {
        for (attempts, ok) in [(0, false), (1, true), (255, true)] {
            assert_eq!(RetryGuidance::new([], false, attempts, Backoff::None).is_ok(), ok, "attempts {attempts}");
        }
    }

    #[test]
    fn empty_retry_on_is_valid() {
        let guidance = RetryGuidance::new([], true, 1, Backoff::None).unwrap();
        assert!(guidance.retry_on.is_empty());
    }

    #[test]
    fn backoff_boundaries() {
        let cases = [
            (Backoff::Fixed { delay_ms: 0 }, Some(MetadataError::ZeroDelay { field: "delay_ms" })),
            (Backoff::Fixed { delay_ms: 1 }, None),
            (Backoff::Exponential { base_ms: 0, cap_ms: 10 }, Some(MetadataError::ZeroDelay { field: "base_ms" })),
            (Backoff::Exponential { base_ms: 10, cap_ms: 9 }, Some(MetadataError::CapBelowBase { base_ms: 10, cap_ms: 9 })),
            (Backoff::Exponential { base_ms: 10, cap_ms: 10 }, None),
            (Backoff::Exponential { base_ms: 10, cap_ms: 11 }, None),
            (Backoff::None, None),
        ];
        for (backoff, expected) in cases {
            assert_eq!(RetryGuidance::new([], false, 1, backoff).err(), expected, "{backoff:?}");
        }
    }

    #[test]
    fn header_names_are_validated_per_field() {
        let valid = "x-ratelimit-limit";
        for bad in ["", "x y", "x:y", "x-ré"] {
            let expect = |field| Err(MetadataError::InvalidHeaderName { field, value: bad.to_owned() });
            assert_eq!(RateLimit::new(bad, valid, valid, ResetUnit::Seconds), expect("limit_header"));
            assert_eq!(RateLimit::new(valid, bad, valid, ResetUnit::Seconds), expect("remaining_header"));
            assert_eq!(RateLimit::new(valid, valid, bad, ResetUnit::Seconds), expect("reset_header"));
            let base = RateLimit::new(valid, valid, valid, ResetUnit::Seconds).unwrap();
            assert_eq!(base.with_retry_after_header(bad), expect("retry_after_header"));
        }
        assert!(RateLimit::new(valid, valid, valid, ResetUnit::HttpDate).is_ok());
    }

    #[test]
    fn mutated_value_is_caught_by_validate() {
        let mut rate_limit = github_rate_limit();
        rate_limit.reset_header.clear();
        assert!(rate_limit.validate().is_err());
    }

    #[test]
    fn error_messages_name_field_and_value() {
        let message = RateLimit::new("x y", "a", "b", ResetUnit::Seconds).unwrap_err().to_string();
        assert!(message.contains("limit_header") && message.contains("x y"), "{message}");
        let message = guidance_with(StatusSelector::Code(99)).unwrap_err().to_string();
        assert!(message.contains("code") && message.contains("99"), "{message}");
    }

    fn round_trip<T>(value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let first = serde_json::to_string(value).unwrap();
        let read: T = serde_json::from_str(&first).unwrap();
        assert_eq!(&read, value);
        assert_eq!(serde_json::to_string(&read).unwrap(), first);
    }

    #[test]
    fn complete_values_are_preserved() {
        round_trip(&github_rate_limit());
        round_trip(&RateLimit::new("a", "b", "c", ResetUnit::Seconds).unwrap());
        round_trip(&RateLimit::new("a", "b", "c", ResetUnit::HttpDate).unwrap());
        round_trip(&full_guidance());
        for backoff in [Backoff::None, Backoff::Fixed { delay_ms: 1 }] {
            round_trip(&RetryGuidance::new([StatusSelector::Code(100)], false, 255, backoff).unwrap());
        }
    }

    #[test]
    fn serialized_representation() {
        assert_eq!(
            serde_json::to_value(github_rate_limit()).unwrap(),
            json!({
                "limit_header": "x-ratelimit-limit",
                "remaining_header": "x-ratelimit-remaining",
                "reset_header": "x-ratelimit-reset",
                "retry_after_header": "retry-after",
                "reset_unit": "epoch_seconds",
            })
        );
        assert_eq!(
            serde_json::to_value(full_guidance()).unwrap(),
            json!({
                "retry_on": [{"code": 429}, {"range": {"start": 500, "end": 599}}],
                "honor_retry_after": true,
                "max_attempts": 5,
                "backoff": {"exponential": {"base_ms": 250, "cap_ms": 8000}},
            })
        );
        assert_eq!(serde_json::to_value(Backoff::None).unwrap(), json!("none"));
        assert_eq!(serde_json::to_value(Backoff::Fixed { delay_ms: 5 }).unwrap(), json!({"fixed": {"delay_ms": 5}}));
    }

    /// Applies `edit` to a valid document and reports whether it still parses.
    fn parses<T: for<'de> Deserialize<'de>>(base: &Value, edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> bool {
        let mut document = base.clone();
        edit(document.as_object_mut().unwrap());
        serde_json::from_value::<T>(document).is_ok()
    }

    #[test]
    fn rate_limit_serde_matrix() {
        let base = serde_json::to_value(github_rate_limit()).unwrap();
        assert!(parses::<RateLimit>(&base, |_| {}));
        for field in ["limit_header", "remaining_header", "reset_header", "retry_after_header", "reset_unit"] {
            let optional = field == "retry_after_header";
            assert_eq!(parses::<RateLimit>(&base, |o| { o.remove(field); }), optional, "{field} absent");
            assert!(!parses::<RateLimit>(&base, |o| { o.insert(field.into(), Value::Null); }), "{field} null");
            assert!(!parses::<RateLimit>(&base, |o| { o.insert(field.into(), json!(7)); }), "{field} wrong type");
            assert!(!parses::<RateLimit>(&base, |o| { o.insert(field.into(), json!("")); }), "{field} empty");
        }
        let without: RateLimit = serde_json::from_value({
            let mut document = base.clone();
            document.as_object_mut().unwrap().remove("retry_after_header");
            document
        })
        .unwrap();
        assert_eq!(without.retry_after_header, None);
        assert!(!parses::<RateLimit>(&base, |o| { o.insert("limit_header".into(), json!("x:y")); }));
        assert!(!parses::<RateLimit>(&base, |o| { o.insert("reset_unit".into(), json!("minutes")); }));
        assert!(!parses::<RateLimit>(&base, |o| { o.insert("extra".into(), json!(1)); }));
    }

    #[test]
    fn retry_guidance_serde_matrix() {
        let base = serde_json::to_value(full_guidance()).unwrap();
        assert!(parses::<RetryGuidance>(&base, |_| {}));
        for field in ["retry_on", "honor_retry_after", "max_attempts", "backoff"] {
            assert!(!parses::<RetryGuidance>(&base, |o| { o.remove(field); }), "{field} absent");
            assert!(!parses::<RetryGuidance>(&base, |o| { o.insert(field.into(), Value::Null); }), "{field} null");
        }
        let invalid = [
            ("max_attempts", json!("3")),
            ("max_attempts", json!(0)),
            ("max_attempts", json!(256)),
            ("honor_retry_after", json!("true")),
            ("retry_on", json!({"code": 429})),
            ("retry_on", json!([{"code": 429}, "500"])),
            ("retry_on", json!(["429", 500])),
            ("retry_on", json!([{"code": 600}])),
            ("retry_on", json!([{"range": {"start": 500, "end": 400}}])),
            ("retry_on", json!([{"range": {"start": 500, "end": 599, "step": 1}}])),
            ("retry_on", json!([{"status": 429}])),
            ("backoff", json!("fixed")),
            ("backoff", json!({"fixed": {"delay_ms": 0}})),
            ("backoff", json!({"fixed": {"delay_ms": 1, "jitter": true}})),
            ("backoff", json!({"exponential": {"base_ms": 10, "cap_ms": 1}})),
            ("backoff", json!({"exponential": {"base_ms": 10}})),
            ("backoff", json!("linear")),
        ];
        for (field, value) in invalid {
            assert!(!parses::<RetryGuidance>(&base, |o| { o.insert(field.into(), value.clone()); }), "{field} = {value}");
        }
        assert!(!parses::<RetryGuidance>(&base, |o| { o.insert("extra".into(), json!(1)); }));

        let empty: RetryGuidance = serde_json::from_value({
            let mut document = base.clone();
            document.as_object_mut().unwrap().insert("retry_on".into(), json!([]));
            document
        })
        .unwrap();
        assert!(empty.retry_on.is_empty());
        assert!(parses::<RetryGuidance>(&base, |o| { o.insert("backoff".into(), json!("none")); }));
    }
}
