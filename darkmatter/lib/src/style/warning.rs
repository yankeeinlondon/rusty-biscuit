//! Discriminated warning channel for the `style:` parser.

/// Source-position placeholder. v1 always produces `None` for
/// `StyleWarning::source_span`; the struct exists so later sub-specs can
/// populate it without changing the public surface.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSpan {
    pub line: u32,
    pub column: u32,
    pub length: u32,
}

/// Discriminated category for a `StyleWarning`.
///
/// ## Notes
///
/// `into_strict` promotes `UnknownKey` and `Deprecated` to errors;
/// `KnownButInactive` is informational and never fails strict mode.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleWarningKind {
    /// The path does not appear anywhere in the schema. Likely a typo, or a
    /// removed spelling; [`StyleWarning::suggestion`] names the key to use
    /// when one is known.
    UnknownKey,
    /// The path matched a documented snake-case alias for a renamed key.
    /// The kebab-case canonical spelling is `replacement`.
    Deprecated { replacement: String },
    /// The path parsed successfully and is structurally valid, but the
    /// rendering wiring for this key has not yet been implemented. The
    /// sub-spec number tells the user when it will be.
    KnownButInactive { sub_spec: u8 },
}

/// A warning emitted by the `style:` parser.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleWarning {
    /// Fully-qualified YAML path, e.g., `style.page.lft-margin`.
    pub path: String,
    pub kind: StyleWarningKind,
    /// Canonical key path (without the `style.` prefix) the author should
    /// write instead, e.g. `page.margin-left` for the removed
    /// `page.left-margin`. Set only on `UnknownKey` warnings whose intended
    /// key is known.
    pub suggestion: Option<String>,
    /// Source position, when the parse had the raw frontmatter text.
    pub source_span: Option<StyleSpan>,
}

impl StyleWarning {
    /// Convenience: a warning with no source span.
    pub fn new(path: impl Into<String>, kind: StyleWarningKind) -> Self {
        Self {
            path: path.into(),
            kind,
            suggestion: None,
            source_span: None,
        }
    }

    /// The same warning, suggesting `key` as the spelling to use.
    #[must_use]
    pub fn with_suggestion(mut self, key: impl Into<String>) -> Self {
        self.suggestion = Some(key.into());
        self
    }

    /// `true` if this warning is a schema-validation issue that strict mode
    /// promotes to an error.
    pub fn is_schema_issue(&self) -> bool {
        matches!(
            self.kind,
            StyleWarningKind::UnknownKey | StyleWarningKind::Deprecated { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_is_schema_issue() {
        let w = StyleWarning::new("style.x", StyleWarningKind::UnknownKey);
        assert!(w.is_schema_issue());
    }

    #[test]
    fn deprecated_is_schema_issue() {
        let w = StyleWarning::new(
            "style.block_quote",
            StyleWarningKind::Deprecated {
                replacement: "block-quote".into(),
            },
        );
        assert!(w.is_schema_issue());
    }

    #[test]
    fn known_but_inactive_is_not_schema_issue() {
        let w = StyleWarning::new(
            "style.page.color",
            StyleWarningKind::KnownButInactive { sub_spec: 5 },
        );
        assert!(!w.is_schema_issue());
    }

    #[test]
    fn span_and_suggestion_default_to_none() {
        let w = StyleWarning::new("style.x", StyleWarningKind::UnknownKey);
        assert_eq!(w.source_span, None);
        assert_eq!(w.suggestion, None);
    }

    #[test]
    fn with_suggestion_keeps_the_unknown_key_kind() {
        let w = StyleWarning::new("style.page.left-margin", StyleWarningKind::UnknownKey)
            .with_suggestion("page.margin-left");
        assert_eq!(w.kind, StyleWarningKind::UnknownKey);
        assert_eq!(w.suggestion.as_deref(), Some("page.margin-left"));
        assert!(w.is_schema_issue());
    }
}
