//! One entry point for recording a document's new baseline: the managed hash
//! and `last_updated`, stamped together with a UTC date.
//!
//! Every writer that persists a baseline (`md hash --save`, the effect engine,
//! Claudine's inline closure) goes through [`Markdown::stamp_baseline`], so the
//! stored-hash parse, the save decision, the `last_updated` policy, and the UTC
//! date are decided in one place.

use chrono::{DateTime, Utc};

use super::options::{MdHashOptions, last_updated_stamp};
use super::save::SaveDecision;
use super::stored::StoredHash;
use super::write::apply_hash_save_text;
use crate::markdown::{Markdown, MarkdownResult};

/// What the caller knows about the content it is stamping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The caller did not edit the content and is asking whether it changed
    /// (`md hash --save`). `last_updated` advances only when the hash moved.
    Detect,
    /// The caller just edited the content itself, so `last_updated` advances
    /// even when there was no stored hash to compare against.
    Known,
}

/// The outcome of [`Markdown::stamp_baseline`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineStamp {
    /// The decision that was applied, after the [`Change`] policy.
    pub decision: SaveDecision,
    /// The document text to write, or `None` when the file needs no change.
    pub text: Option<String>,
}

impl Markdown {
    /// Reads the document's stored hash property, or `None` when it is absent
    /// or null.
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::MalformedStoredHash`] when the property is
    /// present but not a valid stored hash.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    pub fn stored_hash(&self, options: &MdHashOptions) -> MarkdownResult<Option<StoredHash>> {
        match self.frontmatter().as_map().get(options.property.as_str()) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => StoredHash::parse(value, &options.property).map(Some),
        }
    }

    /// Records this document's current content as its baseline in `source`.
    ///
    /// `source` is the authored text this document was parsed from; only the
    /// hash node and, when the decision bumps it, the `last_updated` scalar are
    /// changed, so every other byte survives. The date written is the UTC date
    /// of `now`, never the local one.
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::MalformedStoredHash`] for an invalid stored
    /// hash and [`MarkdownError::FrontmatterTextEdit`] when the frontmatter
    /// cannot be edited unambiguously.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    /// [`MarkdownError::FrontmatterTextEdit`]: crate::markdown::MarkdownError::FrontmatterTextEdit
    pub fn stamp_baseline(
        &self,
        source: &str,
        options: &MdHashOptions,
        now: DateTime<Utc>,
        change: Change,
    ) -> MarkdownResult<BaselineStamp> {
        let stored = self.stored_hash(options)?;
        let mut decision = self.plan_hash_save(stored.as_ref(), options)?;
        if change == Change::Known {
            decision.bump_last_updated = true;
        }
        let text = apply_hash_save_text(source, &decision, options, &last_updated_stamp(now))?;
        Ok(BaselineStamp { decision, text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// 23:30 UTC on the 28th is already the 29th in any zone east of UTC+0:30,
    /// so a local-date implementation writes the wrong day.
    fn utc_evening() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 28, 23, 30, 0).unwrap()
    }

    fn stamped(source: &str, change: Change) -> BaselineStamp {
        let md: Markdown = source.into();
        md.stamp_baseline(source, &MdHashOptions::default(), utc_evening(), change)
            .unwrap()
    }

    fn with_stale_hash(body: &str) -> String {
        let original = format!("---\ntitle: T\nlast_updated: 2020-01-01\n---\n{body}");
        let md: Markdown = original.as_str().into();
        let opts = MdHashOptions::default();
        let first = md.plan_hash_save(None, &opts).unwrap();
        let stamped = md.apply_hash_save(&first, &opts, "2020-01-01").unwrap();
        stamped.replace(body, "Edited.")
    }

    #[test]
    fn detect_bumps_when_the_hash_moved() {
        let source = with_stale_hash("Original.");
        let text = stamped(&source, Change::Detect).text.unwrap();
        assert!(text.contains("last_updated: 2026-09-28"), "{text}");
    }

    #[test]
    fn detect_leaves_a_first_baseline_undated() {
        let source = "---\ntitle: T\nlast_updated: 2020-01-01\n---\nBody.";
        let text = stamped(source, Change::Detect).text.unwrap();
        assert!(text.contains("hash:"));
        assert!(text.contains("last_updated: 2020-01-01"), "{text}");
    }

    #[test]
    fn known_dates_a_first_baseline_with_the_utc_date() {
        let source = "---\ntitle: T\nlast_updated: 2020-01-01\n---\nBody.";
        let text = stamped(source, Change::Known).text.unwrap();
        assert!(text.contains("last_updated: 2026-09-28"), "{text}");
    }

    #[test]
    fn unchanged_content_needs_no_write() {
        let source = with_stale_hash("Original.");
        let once = stamped(&source, Change::Known).text.unwrap();
        assert_eq!(stamped(&once, Change::Known).text, None);
    }

    #[test]
    fn malformed_stored_hash_is_an_error() {
        let source = "---\nhash: 42\n---\nBody.";
        let md: Markdown = source.into();
        let result =
            md.stamp_baseline(source, &MdHashOptions::default(), utc_evening(), Change::Known);
        assert!(result.is_err());
    }
}
