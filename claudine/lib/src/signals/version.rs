//! Lenient provider-version parsing and comparison for detection-record
//! `since`/`until` bounds.
//!
//! Corpus reality the leniency covers: `v1.17.7` / `v1.17.8` (opencode),
//! `0.19.6` (qwen), `rust-v0.142.5` (codex), and the kimi wire protocol
//! `"1.10"`.

use std::cmp::Ordering;

/// A version string reduced to comparable segments.
///
/// A leading non-numeric prefix (`v`, `rust-v`) is stripped; the remainder is
/// split on dots. Numeric segments compare numerically (`10 > 9`);
/// non-parsable segments compare as strings; a missing segment compares as
/// numeric `0`, so `1.10` == `1.10.0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedVersion {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Num(u64),
    Text(String),
}

/// Parse leniently; `None` when the string carries no digits at all (an
/// unparsable observed version keeps the engine in union mode).
pub(crate) fn parse(version: &str) -> Option<ParsedVersion> {
    let start = version.find(|c: char| c.is_ascii_digit())?;
    let segments = version[start..]
        .split('.')
        .map(|s| {
            let s = s.trim();
            s.parse::<u64>()
                .map(Segment::Num)
                .unwrap_or_else(|_| Segment::Text(s.to_string()))
        })
        .collect();
    Some(ParsedVersion { segments })
}

impl ParsedVersion {
    fn segment(&self, index: usize) -> Segment {
        self.segments.get(index).cloned().unwrap_or(Segment::Num(0))
    }
}

impl Ord for ParsedVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.segments.len().max(other.segments.len());
        for i in 0..len {
            let ordering = match (self.segment(i), other.segment(i)) {
                (Segment::Num(a), Segment::Num(b)) => a.cmp(&b),
                // Mixed segments fall back to string comparison so ordering
                // stays total (e.g. `2` vs `beta`).
                (a, b) => segment_text(&a).cmp(&segment_text(&b)),
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for ParsedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn segment_text(segment: &Segment) -> String {
    match segment {
        Segment::Num(n) => n.to_string(),
        Segment::Text(t) => t.clone(),
    }
}

/// Whether `observed` falls inside the inclusive `[since, until]` range.
///
/// An unparsable bound does not restrict (fail-open, mirroring union mode
/// for unparsable observed versions).
pub(crate) fn admits(
    observed: &ParsedVersion,
    since: Option<&str>,
    until: Option<&str>,
) -> bool {
    if let Some(bound) = since.and_then(parse)
        && *observed < bound
    {
        return false;
    }
    if let Some(bound) = until.and_then(parse)
        && *observed > bound
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> ParsedVersion {
        parse(s).expect("parsable version")
    }

    #[test]
    fn strips_leading_prefixes() {
        assert_eq!(v("v1.17.7"), v("1.17.7"));
        assert_eq!(v("rust-v0.142.5"), v("0.142.5"));
    }

    #[test]
    fn numeric_segments_compare_numerically() {
        assert!(v("1.10") > v("1.9"));
        assert!(v("v1.17.8") > v("v1.17.7"));
        assert!(v("0.142.5") > v("0.19.6"));
    }

    #[test]
    fn missing_segments_compare_as_zero() {
        assert_eq!(v("1.10").cmp(&v("1.10.0")), Ordering::Equal);
        assert!(v("1.10.1") > v("1.10"));
    }

    #[test]
    fn non_numeric_segments_compare_as_strings() {
        assert!(v("1.beta") < v("1.rc"));
    }

    #[test]
    fn no_digits_is_unparsable() {
        assert!(parse("beta").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn admits_inclusive_bounds() {
        // Opencode pair: legacy record `until: v1.17.7`.
        assert!(admits(&v("1.17.7"), None, Some("v1.17.7")));
        assert!(!admits(&v("1.17.8"), None, Some("v1.17.7")));
        // 1178 record `since: v1.17.8`.
        assert!(admits(&v("1.17.8"), Some("v1.17.8"), None));
        assert!(!admits(&v("1.17.7"), Some("v1.17.8"), None));
    }

    #[test]
    fn unparsable_bound_does_not_restrict() {
        assert!(admits(&v("1.0"), Some("beta"), None));
    }
}
