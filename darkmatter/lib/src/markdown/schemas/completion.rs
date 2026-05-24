//! Schema-aware completion hints for external consumers (e.g. shell-completion
//! generators).
//!
//! This module is a **read-only consumer** of an [`EffectiveSchema`]: it
//! inspects the SimplifiedSchema projection (when available) and yields
//! completion data for a property, without mutating or re-resolving the
//! schema. See the *Shell Completions Integration* section of the spec.
//!
//! Three property categories are surfaced:
//!
//! - [`CompletionKind::File`] — filesystem paths filtered by zero or more
//!   `match=` globs. The caller (shell-completion generator) walks the
//!   filesystem; this module only returns the patterns.
//! - [`CompletionKind::Enum`] — the enumerated members of an `enum` property.
//! - [`CompletionKind::Hint`] — `url` / `email` / date-family properties, where
//!   completion is a one-line format hint (no values).
//!
//! All entry points are pure functions over already-resolved schema state —
//! no I/O, no re-parsing.
//!
//! ## Example
//!
//! ```ignore
//! use darkmatter::markdown::schemas::{DarkmatterSchemas, completion};
//!
//! let api = DarkmatterSchemas::new();
//! if let Some(effective) = api.effective_for(&md)? {
//!     for property in completion::completable_properties(&effective) {
//!         let suggestion = completion::for_property(&effective, &property);
//!         // dispatch to shell-completion backend...
//!     }
//! }
//! ```

use super::EffectiveSchema;
use super::simplified::{
    Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType,
};

/// Completion data derived from a single property.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionSuggestion {
    /// Property name (key in frontmatter).
    pub property: String,
    /// `true` when the property is declared with `[]`, i.e. the value is an
    /// array of completable atoms.
    pub is_array: bool,
    /// Optional description from `-> ...` syntax.
    pub description: Option<String>,
    /// The completion category and payload.
    pub kind: CompletionKind,
}

/// The category of completion produced for a property.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    /// Filesystem-path completion. Patterns are the raw glob strings from
    /// `match(...)` (including any `!`-prefixed negations). Empty when the
    /// property is a bare `file` with no `match` constraint.
    File { patterns: Vec<String> },
    /// Enum-member completion. The members come from the constraint payload
    /// in declaration order.
    Enum { members: Vec<String> },
    /// One-line format hint for value-completion-unfriendly types
    /// (`url`, `email`, `date`, `datetime`, `time`).
    Hint { format: &'static str },
}

/// Returns the completion suggestion for `property` if its declared type is
/// in the completable set, otherwise `None`.
///
/// For property-level unions, the first atom whose type is completable wins.
/// This matches the spec's intent of "treat the property as that type for
/// the purpose of completion" without trying to merge competing arms.
pub fn for_property(effective: &EffectiveSchema, property: &str) -> Option<CompletionSuggestion> {
    let simplified = effective.simplified.as_ref()?;
    let shape = single_shape(simplified)?;
    let def = shape.properties.get(property)?;
    suggestion_from_def(property, def)
}

/// Returns the list of properties in `effective` that have a completable
/// type. Order matches the SimplifiedSchema's declaration order.
///
/// Returns an empty vector when the effective schema is a root union or was
/// supplied as raw JSON Schema (no SimplifiedSchema projection is available).
pub fn completable_properties(effective: &EffectiveSchema) -> Vec<String> {
    let Some(simplified) = effective.simplified.as_ref() else {
        return Vec::new();
    };
    let Some(shape) = single_shape(simplified) else {
        return Vec::new();
    };
    shape
        .properties
        .iter()
        .filter_map(|(name, def)| {
            if first_completable_atom(def).is_some() {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect()
}

fn single_shape(schema: &SimplifiedSchema) -> Option<&SchemaShape> {
    match schema {
        SimplifiedSchema::Single(shape) => Some(shape),
        SimplifiedSchema::Union(_) => None,
    }
}

fn suggestion_from_def(property: &str, def: &PropertyDef) -> Option<CompletionSuggestion> {
    let atom = first_completable_atom(def)?;
    let kind = kind_for_atom(atom)?;
    Some(CompletionSuggestion {
        property: property.to_string(),
        is_array: atom.is_array,
        description: atom.description.clone(),
        kind,
    })
}

fn first_completable_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    match def {
        PropertyDef::Single(atom) => is_completable(atom.ty).then_some(atom),
        PropertyDef::Union(atoms) => atoms.iter().find(|a| is_completable(a.ty)),
    }
}

fn is_completable(ty: SimplifiedType) -> bool {
    matches!(
        ty,
        SimplifiedType::File
            | SimplifiedType::Enum
            | SimplifiedType::Url
            | SimplifiedType::Email
            | SimplifiedType::Date
            | SimplifiedType::DateTime
            | SimplifiedType::Time
    )
}

fn kind_for_atom(atom: &PropertyAtom) -> Option<CompletionKind> {
    match atom.ty {
        SimplifiedType::File => Some(CompletionKind::File {
            patterns: atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Match(patterns) => Some(patterns.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
        }),
        SimplifiedType::Enum => Some(CompletionKind::Enum {
            members: atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Members(members) => Some(members.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
        }),
        SimplifiedType::Url => Some(CompletionKind::Hint {
            format: "URL (e.g. https://example.com/...)",
        }),
        SimplifiedType::Email => Some(CompletionKind::Hint {
            format: "email address (e.g. name@example.com)",
        }),
        SimplifiedType::Date => Some(CompletionKind::Hint {
            format: "ISO-8601 date (YYYY-MM-DD)",
        }),
        SimplifiedType::DateTime => Some(CompletionKind::Hint {
            format: "ISO-8601 datetime (e.g. 2026-05-11T12:00:00Z)",
        }),
        SimplifiedType::Time => Some(CompletionKind::Hint {
            format: "time-of-day (HH:MM[:SS][±HH:MM])",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::schemas::DarkmatterSchemas;

    fn effective(yaml_body: &str) -> EffectiveSchema {
        let md_text = format!("---\n{yaml_body}---\nbody\n");
        let md: Markdown = md_text.as_str().into();
        let api = DarkmatterSchemas::new();
        api.effective_for(&md)
            .expect("effective_for")
            .expect("effective schema present")
    }

    #[test]
    fn file_property_with_match_patterns() {
        let eff = effective("$schema:\n  cover: \"file(match('*.png', '*.jpg'))\"\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        assert_eq!(suggestion.property, "cover");
        assert!(!suggestion.is_array);
        match suggestion.kind {
            CompletionKind::File { patterns } => {
                assert_eq!(patterns, vec!["*.png", "*.jpg"]);
            }
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn file_property_without_match_yields_empty_patterns() {
        let eff = effective("$schema:\n  cover: file\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        match suggestion.kind {
            CompletionKind::File { patterns } => assert!(patterns.is_empty()),
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn enum_property_returns_members() {
        let eff = effective("$schema:\n  status: enum(draft, published, archived)\n");
        let suggestion = for_property(&eff, "status").expect("status should be completable");
        match suggestion.kind {
            CompletionKind::Enum { members } => {
                assert_eq!(members, vec!["draft", "published", "archived"]);
            }
            other => panic!("expected Enum completion, got {other:?}"),
        }
    }

    #[test]
    fn url_property_returns_hint() {
        let eff = effective("$schema:\n  homepage: url\n");
        let suggestion = for_property(&eff, "homepage").expect("homepage should be completable");
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected Hint, got {other:?}"),
        }
    }

    #[test]
    fn non_completable_type_returns_none() {
        let eff = effective("$schema:\n  title: string\n");
        assert!(for_property(&eff, "title").is_none());
    }

    #[test]
    fn unknown_property_returns_none() {
        let eff = effective("$schema:\n  title: string\n");
        assert!(for_property(&eff, "nope").is_none());
    }

    #[test]
    fn array_file_property_is_marked_as_array() {
        let eff = effective("$schema:\n  attachments: \"file[]\"\n");
        let suggestion =
            for_property(&eff, "attachments").expect("attachments should be completable");
        assert!(suggestion.is_array);
        matches!(suggestion.kind, CompletionKind::File { .. });
    }

    #[test]
    fn completable_properties_filters_to_completable_types() {
        let eff = effective(
            "$schema:\n  title: string\n  cover: file\n  status: enum(a, b)\n  homepage: url\n",
        );
        let mut names = completable_properties(&eff);
        names.sort();
        assert_eq!(names, vec!["cover", "homepage", "status"]);
    }

    #[test]
    fn description_is_preserved() {
        let eff = effective("$schema:\n  cover: \"file -> The cover image for this post\"\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        assert_eq!(
            suggestion.description.as_deref(),
            Some("The cover image for this post")
        );
    }

    #[test]
    fn property_union_picks_first_completable_arm() {
        let eff = effective("$schema:\n  href:\n    - url\n    - file\n");
        let suggestion = for_property(&eff, "href").expect("href should be completable");
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected URL hint from first arm, got {other:?}"),
        }
    }
}
