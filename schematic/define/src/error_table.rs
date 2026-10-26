//! Declared HTTP error shapes (D38): API defaults overlaid by endpoint or
//! operation entries, resolved by specificity.
//!
//! An API lists default [`ErrorEntry`]s. An endpoint or GraphQL operation adds
//! entries, replaces an API entry whose match is identical, or opts out of all
//! of them. [`ErrorTable::resolve`] builds the effective table and
//! [`ErrorTable::lookup`] picks the most specific entry for a status: an exact
//! status, then the narrowest range. See
//! `schematic/docs/responses-and-errors.md`.

use serde::{Deserialize, Serialize};

use crate::response::{normalize_media_type, ResponseShape};

/// The statuses an error entry matches, within 400–599 (Defaults D-6, D-8).
///
/// Serialized externally tagged: `{"Exact": 404}` or
/// `{"Range": {"lo": 400, "hi": 499}}`.
///
/// ## Examples
///
/// ```
/// use schematic_define::StatusMatch;
///
/// assert_eq!(StatusMatch::class(4), StatusMatch::range(400, 499));
/// assert!(StatusMatch::class(5).contains(503));
/// assert!(!StatusMatch::exact(404).contains(405));
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusMatch {
    /// One status.
    Exact(u16),
    /// Every status from `lo` to `hi`, both inclusive.
    Range {
        /// First matched status.
        lo: u16,
        /// Last matched status.
        hi: u16,
    },
}

#[allow(missing_docs)]
impl StatusMatch {
    pub fn exact(status: u16) -> Self { Self::Exact(status) }
    pub fn range(lo: u16, hi: u16) -> Self { Self::Range { lo, hi } }
    /// The hundred statuses of a class: `class(4)` is `400..=499`.
    pub fn class(class: u16) -> Self { Self::Range { lo: class * 100, hi: class * 100 + 99 } }
    pub fn contains(&self, status: u16) -> bool {
        match *self {
            Self::Exact(exact) => exact == status,
            Self::Range { lo, hi } => (lo..=hi).contains(&status),
        }
    }
    /// Matched status count; an exact match is the narrowest.
    pub fn width(&self) -> u32 {
        match *self {
            Self::Exact(_) => 1,
            Self::Range { lo, hi } => u32::from(hi.saturating_sub(lo)) + 1,
        }
    }

    fn overlaps(&self, other: &Self) -> Option<(u16, u16)> {
        let (a_lo, a_hi) = self.bounds();
        let (b_lo, b_hi) = other.bounds();
        let (lo, hi) = (a_lo.max(b_lo), a_hi.min(b_hi));
        (lo <= hi).then_some((lo, hi))
    }

    fn bounds(&self) -> (u16, u16) {
        match *self {
            Self::Exact(status) => (status, status),
            Self::Range { lo, hi } => (lo, hi),
        }
    }

    fn validate(&self) -> Result<(), String> {
        let (lo, hi) = self.bounds();
        if !(400..=599).contains(&lo) || !(400..=599).contains(&hi) {
            return Err(format!("error status match {self} is outside 400-599"));
        }
        if lo > hi { return Err(format!("error status range {lo}-{hi} is reversed")); }
        Ok(())
    }
}

impl std::fmt::Display for StatusMatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exact(status) => write!(formatter, "{status}"),
            Self::Range { lo, hi } => write!(formatter, "{lo}-{hi}"),
        }
    }
}

/// One declared error: the statuses it matches and the shape their body
/// decodes as. Streams are not allowed on the error path (R16).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEntry {
    /// The statuses this entry matches.
    pub status: StatusMatch,
    /// JSON, text, binary, or empty.
    pub shape: ResponseShape,
    /// A declared concrete media type; `None` uses the shape's default.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

impl ErrorEntry {
    /// An entry with the shape's default media type.
    pub fn new(status: StatusMatch, shape: ResponseShape) -> Self {
        Self { status, shape, content_type: None }
    }
}

/// The effective error table of one endpoint or operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorTable {
    entries: Vec<ErrorEntry>,
}

impl ErrorTable {
    /// The API defaults overlaid by `own`: an entry whose match is identical
    /// to an API entry's replaces it, any other is added, and an opt-out
    /// empties the table (R17).
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ErrorEntry, ErrorTable, StatusMatch, response::ResponseShape};
    ///
    /// let api = [ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Text)];
    /// let own = [ErrorEntry::new(StatusMatch::class(4), ResponseShape::Empty)];
    /// let table = ErrorTable::resolve(&api, &own, false);
    /// assert_eq!(table.lookup(404).unwrap().shape, ResponseShape::Text);
    /// assert_eq!(table.lookup(400).unwrap().shape, ResponseShape::Empty);
    /// assert!(table.lookup(500).is_none());
    /// assert!(ErrorTable::resolve(&api, &[], true).entries().is_empty());
    /// ```
    pub fn resolve(api: &[ErrorEntry], own: &[ErrorEntry], opted_out: bool) -> Self {
        if opted_out { return Self { entries: vec![] }; }
        let mut entries = api.to_vec();
        for entry in own {
            match entries.iter_mut().find(|existing| existing.status == entry.status) {
                Some(existing) => *existing = entry.clone(),
                None => entries.push(entry.clone()),
            }
        }
        Self { entries }
    }

    /// The most specific entry matching `status` (R18): exact, then the
    /// narrowest range. A validated table has no tie.
    pub fn lookup(&self, status: u16) -> Option<&ErrorEntry> {
        self.entries.iter().filter(|entry| entry.status.contains(status)).min_by_key(|entry| entry.status.width())
    }

    /// API entries in declared order, each replaced in place, then additions.
    pub fn entries(&self) -> &[ErrorEntry] { &self.entries }
}

/// Check one level's entries (R16, R18): matches in 400–599 with ordered
/// bounds, no stream shape, valid media types, and no two entries with an
/// identical match or equal-width overlapping ranges.
///
/// ## Errors
///
/// A message naming the offending matches and statuses, without API or
/// endpoint context.
pub fn validate_error_entries(entries: &[ErrorEntry]) -> Result<(), String> {
    for (index, entry) in entries.iter().enumerate() {
        entry.status.validate()?;
        match &entry.shape {
            ResponseShape::Stream { .. } => return Err(format!("error {}: a stream cannot be an error shape", entry.status)),
            ResponseShape::Empty if entry.content_type.is_some() => return Err(format!("error {}: an empty shape has no media type", entry.status)),
            _ => {}
        }
        if let Some(content_type) = &entry.content_type {
            normalize_media_type(content_type).map_err(|message| format!("error {}: {message}", entry.status))?;
        }
        if entries[..index].iter().any(|other| other.status == entry.status) {
            return Err(format!("error status match {} is declared more than once", entry.status));
        }
    }
    check_overlaps(entries)
}

/// Equal-width overlapping matches leave a status with no most specific
/// entry. Run on each level and on the resolved table.
pub(crate) fn check_overlaps(entries: &[ErrorEntry]) -> Result<(), String> {
    for (index, entry) in entries.iter().enumerate() {
        for other in &entries[..index] {
            if other.status != entry.status && other.status.width() == entry.status.width()
                && let Some((lo, hi)) = other.status.overlaps(&entry.status) {
                return Err(format!("error ranges {} and {} have equal width and overlap on statuses {lo}-{hi}", other.status, entry.status));
            }
        }
    }
    Ok(())
}

/// Check one endpoint's or operation's error policy against the API defaults
/// (R17, R18): opting out excludes entries, each level is valid, and the
/// resolved table has a most specific entry for every status.
pub(crate) fn validate_error_policy(api: &[ErrorEntry], own: &[ErrorEntry], opted_out: bool) -> Result<(), String> {
    if opted_out && !own.is_empty() {
        return Err("opts out of declared errors and also declares error entries".into());
    }
    validate_error_entries(own)?;
    check_overlaps(ErrorTable::resolve(api, own, opted_out).entries())
}
