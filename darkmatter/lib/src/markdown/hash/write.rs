//! The write-back path: turning a [`SaveDecision`] into serialized Markdown.
//!
//! The decision layer ([`Markdown::plan_hash_save`]) chooses *what* to persist;
//! this layer applies that choice to a clone of the document and re-serializes
//! it. The split mirrors `md clean --save`: the library mutates and serializes,
//! the CLI owns the `fs::write`.
//!
//! ## Body fidelity
//!
//! Serialization reuses [`Markdown::as_string`], which appends
//! [`Markdown::content`] verbatim after the frontmatter block. Hash-save runs no
//! cleanup or body normalization, so the body bytes are preserved exactly —
//! irregular spacing, code fences, and trailing newlines included.

use super::options::{LAST_UPDATED_KEY, MdHashOptions};
use super::save::SaveDecision;
use crate::markdown::Markdown;

impl Markdown {
    /// Applies a [`SaveDecision`] and returns the serialized Markdown to write,
    /// or `None` when the decision requires no change.
    ///
    /// The active hash property is set from `options.property`, preserving its
    /// existing position in the frontmatter when present and appending it when
    /// new (`IndexMap` insertion semantics). When the decision bumps
    /// `last_updated`, that key is set to `today`, which the caller supplies as a
    /// `YYYY-MM-DD` string so the library stays deterministic and clock-free.
    ///
    /// The source document is never mutated; mutation happens on a clone.
    ///
    /// ## Returns
    ///
    /// `Some(markdown)` to write, or `None` when [`SaveDecision::new_stored`] is
    /// `None` (nothing changed, so the file is left untouched).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::{Markdown, MdHashOptions};
    ///
    /// let doc: Markdown = "---\ntitle: T\n---\n# H\n\nBody.".into();
    /// let opts = MdHashOptions::default();
    /// let decision = doc.plan_hash_save(None, &opts).unwrap();
    /// let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();
    /// assert!(written.ends_with("# H\n\nBody."));
    /// ```
    pub fn apply_hash_save(
        &self,
        decision: &SaveDecision,
        options: &MdHashOptions,
        today: &str,
    ) -> Option<String> {
        let new_stored = decision.new_stored.as_ref()?;

        let mut updated = self.clone();
        let map = updated.frontmatter_mut().as_map_mut();
        // Direct map insertion keeps the serialized value identical to the
        // parsed `StoredHash` shape and preserves key position for an existing
        // hash property; a new property appends at the end.
        map.insert(options.property.clone(), new_stored.to_frontmatter_value());
        if decision.bump_last_updated {
            map.insert(
                LAST_UPDATED_KEY.to_string(),
                serde_json::Value::String(today.to_string()),
            );
        }
        Some(updated.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::MdHashKind;

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// The serialized body must equal the original `content()` byte-for-byte,
    /// so the written document always ends with the untouched body.
    fn assert_body_preserved(original: &Markdown, serialized: &str) {
        assert!(
            serialized.ends_with(original.content()),
            "body not preserved verbatim.\noriginal body: {:?}\nserialized: {:?}",
            original.content(),
            serialized
        );
    }

    #[test]
    fn no_change_returns_none() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = StoredFromDoc::stored(&doc, MdHashKind::Simple, &opts);

        let decision = doc.plan_hash_save(Some(&stored), &opts).unwrap();
        assert!(decision.new_stored.is_none());
        assert!(
            doc.apply_hash_save(&decision, &opts, "2026-05-28")
                .is_none()
        );
    }

    #[test]
    fn first_baseline_adds_hash_and_preserves_body() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("hash:"));
        assert!(written.contains("title: T"));
        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn first_baseline_does_not_bump_last_updated() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(!written.contains("last_updated"));
    }

    #[test]
    fn document_without_frontmatter_gains_a_valid_block() {
        let doc = md("# H\n\nBody only, no frontmatter.\n");
        assert!(doc.frontmatter().is_empty());
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        // A fresh frontmatter block wraps the body, which is left untouched
        // after the closing delimiter.
        assert!(written.starts_with("---\n"));
        assert!(written.contains("\n---\n"));
        assert!(written.contains("hash:"));
        assert_body_preserved(&doc, &written);

        // The written document re-parses with a populated frontmatter block.
        let reparsed: Markdown = written.into();
        assert!(!reparsed.frontmatter().is_empty());
        assert!(reparsed.content().contains("Body only, no frontmatter."));
    }

    #[test]
    fn content_change_bumps_last_updated() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = StoredFromDoc::stored(&original, MdHashKind::Simple, &opts);

        let edited = md("---\ntitle: T\n---\n# H\n\nNew body.");
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = edited
            .apply_hash_save(&decision, &opts, "2026-05-28")
            .unwrap();

        assert!(written.contains("last_updated: 2026-05-28"));
        assert_body_preserved(&edited, &written);
    }

    #[test]
    fn updating_existing_hash_preserves_key_order() {
        // `hash` sits in the middle: its position must be preserved on update.
        let opts = MdHashOptions::default();
        let stored = crate::markdown::hash::StoredHash::parse(
            &serde_json::json!("aaaa111111111111-bbbb222222222222"),
            "hash",
        )
        .unwrap();

        // Force a body change so a rewrite is required.
        let edited = md(
            "---\ntitle: T\nhash: aaaa111111111111-bbbb222222222222\nauthor: A\n---\n# H\n\nChanged body.",
        );
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = edited
            .apply_hash_save(&decision, &opts, "2026-05-28")
            .unwrap();

        let reparsed: Markdown = written.into();
        let keys: Vec<&str> = reparsed
            .frontmatter()
            .as_map()
            .keys()
            .map(String::as_str)
            .collect();
        // Original three keys keep their relative order; `last_updated` is the
        // only newcomer and lands at the end.
        assert_eq!(keys, vec!["title", "hash", "author", "last_updated"]);
    }

    #[test]
    fn body_with_irregular_spacing_and_code_fences_is_byte_exact() {
        let body = "#   Heading\n\n\n   indented prose   \n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n\n\ntrailing text\n\n\n";
        let source = format!("---\ntitle: T\n---\n{body}");
        let doc = md(&source);
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn body_without_trailing_newline_is_preserved() {
        let doc = md("---\ntitle: T\n---\n# H\n\nNo trailing newline.");
        assert!(!doc.content().ends_with('\n'));
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn custom_property_name_is_used_for_write() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions {
            property: "fingerprint".to_string(),
            ..MdHashOptions::default()
        };

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("fingerprint:"));
        assert!(!written.contains("\nhash:"));
        assert_body_preserved(&doc, &written);
    }

    /// Test helper: a stored hash computed from a document at a kind.
    struct StoredFromDoc;
    impl StoredFromDoc {
        fn stored(
            doc: &Markdown,
            kind: MdHashKind,
            opts: &MdHashOptions,
        ) -> crate::markdown::hash::StoredHash {
            crate::markdown::hash::StoredHash {
                kind,
                value: doc.compute_hash(kind, opts).to_stored_value(),
                ignored: Vec::new(),
            }
        }
    }
}
