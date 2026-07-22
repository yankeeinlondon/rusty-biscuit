//! Passive classification of authored schema file references.

use biscuit_file::FileReference;

use super::SchemaError;

/// The resolution policy selected by an authored local schema reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaReferenceKind {
    /// A name with no path separator or special file-reference prefix.
    BareName,
    /// A relative, absolute, magic, package, vault, or recursive reference.
    PathQualified,
}

/// A syntactically valid local schema reference and its resolution policy.
#[derive(Debug, Clone)]
pub struct SchemaReference {
    file_reference: FileReference,
    kind: SchemaReferenceKind,
}

impl SchemaReference {
    /// Borrows the syntax-checked [`FileReference`].
    ///
    /// Its `raw()` is the canonical (trimmed) reference — the same string used
    /// for classification, resolution, and error reporting.
    pub fn file_reference(&self) -> &FileReference {
        &self.file_reference
    }

    /// Returns the schema-root classification of this reference.
    pub fn kind(&self) -> SchemaReferenceKind {
        self.kind
    }

    pub(crate) fn into_file_reference(self) -> FileReference {
        self.file_reference
    }
}

// Identity is the canonical trimmed string plus the resolution policy; the
// resolution-time state a `FileReference` may accumulate (magic paths, vault
// roots) is absent from a freshly classified reference.
impl PartialEq for SchemaReference {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.file_reference.raw() == other.file_reference.raw()
    }
}

/// Classifies one authored schema reference without resolving it.
///
/// The authored string is trimmed once here, and every downstream product —
/// the classification, the [`FileReference`], and the `reference` carried on
/// errors — is derived from that one trimmed string. Callers must not re-trim
/// or re-classify: a padded declaration such as `" ./schemas/post.yaml "` must
/// never be checked as one string and resolved as another.
///
/// Construction delegates syntax to [`FileReference::new`]. HTTP(S) values
/// are rejected by Darkmatter's local-only schema policy before construction.
/// This function performs no filesystem or network access.
///
/// ## Errors
///
/// Returns [`SchemaError::RemoteUnsupported`] for HTTP(S) values and
/// [`SchemaError::Unresolved`] carrying the syntax error for malformed local
/// file references, including a value that is empty or entirely whitespace.
pub fn classify_schema_reference(reference: &str) -> Result<SchemaReference, SchemaError> {
    let trimmed = reference.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Err(SchemaError::RemoteUnsupported {
            reference: trimmed.to_string(),
        });
    }
    // A whitespace-only value is an empty reference, not a path made of
    // spaces; reject it here rather than handing the padding to biscuit-file.
    if trimmed.is_empty() {
        return Err(SchemaError::Unresolved {
            reference: String::new(),
            source: biscuit_file::FileReferenceError::InvalidSyntax(
                "empty reference string".to_string(),
            ),
        });
    }
    let kind = if is_bare_schema_name(trimmed) {
        SchemaReferenceKind::BareName
    } else {
        SchemaReferenceKind::PathQualified
    };
    let file_reference = FileReference::new(trimmed).map_err(|source| SchemaError::Unresolved {
        reference: trimmed.to_string(),
        source,
    })?;
    Ok(SchemaReference { file_reference, kind })
}

pub(crate) fn is_bare_schema_name(reference: &str) -> bool {
    if reference.is_empty() || reference.contains('/') || reference.contains('\\') {
        return false;
    }
    !reference.starts_with('@')
        && !reference.starts_with('!')
        && !reference.starts_with('%')
        && !reference.starts_with("vault:")
        && !reference.starts_with("vault::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_bare_and_path_qualified_local_references() {
        assert_eq!(
            classify_schema_reference("schema.yaml").unwrap().kind(),
            SchemaReferenceKind::BareName,
        );
        for reference in ["./schema.yaml", "docs/schema.yaml", r"docs\schema.yaml", "@schema.yaml"] {
            assert_eq!(
                classify_schema_reference(reference).unwrap().kind(),
                SchemaReferenceKind::PathQualified,
            );
        }
    }

    #[test]
    fn rejects_remote_references_without_resolution() {
        for reference in ["http://example.com/schema.yaml", "https://example.com/schema.yaml"] {
            assert!(matches!(
                classify_schema_reference(reference),
                Err(SchemaError::RemoteUnsupported { .. })
            ));
        }
    }
}
