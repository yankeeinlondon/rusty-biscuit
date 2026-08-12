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
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType,
    TypeExpr,
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
    /// `true` when the property value is supplied by the host runtime rather
    /// than authored in static frontmatter.
    pub generated: bool,
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
/// For a property-level union, the first atom whose type is completable wins —
/// the spec's intent of "treat the property as that type for the purpose of
/// completion" without merging competing arms.
///
/// For a **root-level union**, `property` is gathered from every inline arm
/// that declares it. When every contributing arm is a `file`, their
/// `match(...)` globs are combined (deduplicated) so the union offers the union
/// of its arms' candidates; otherwise the first completable arm wins. See
/// [`suggestion_from_union`].
pub fn for_property(effective: &EffectiveSchema, property: &str) -> Option<CompletionSuggestion> {
    match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => {
            let def = shape.properties.get(property)?;
            suggestion_from_def(property, def)
        }
        SimplifiedSchema::Union(arms) => suggestion_from_union(property, arms),
    }
}

/// Returns the list of properties in `effective` that have a completable
/// type. Order matches the SimplifiedSchema's declaration order.
///
/// For a root-level union, the union of every inline arm's completable
/// properties is returned, deduplicated in first-seen order. Returns an empty
/// vector when the effective schema was supplied as raw JSON Schema (no
/// SimplifiedSchema projection is available).
pub fn completable_properties(effective: &EffectiveSchema) -> Vec<String> {
    let Some(simplified) = effective.simplified.as_ref() else {
        return Vec::new();
    };
    match simplified {
        SimplifiedSchema::Single(shape) => completable_in_shape(shape),
        SimplifiedSchema::Union(arms) => {
            let mut out: Vec<String> = Vec::new();
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    for name in completable_in_shape(shape) {
                        if !out.contains(&name) {
                            out.push(name);
                        }
                    }
                }
            }
            out
        }
    }
}

fn completable_in_shape(shape: &SchemaShape) -> Vec<String> {
    shape
        .properties
        .iter()
        .filter_map(|(name, def)| first_completable_atom(def).map(|_| name.clone()))
        .collect()
}

/// Builds a completion suggestion for `property` across the inline arms of a
/// root-level union.
///
/// Each arm that declares `property` contributes its first completable atom.
/// When every contributing atom is a `file`, their `match(...)` globs are
/// merged (deduplicated, first-seen order) so a union of file-typed arms
/// offers the union of their candidates instead of only the first arm's.
/// Otherwise the first contributing atom wins, matching the single-shape
/// "first completable arm" rule. Unresolved [`SchemaArm::FileRef`] arms are
/// skipped.
fn suggestion_from_union(property: &str, arms: &[SchemaArm]) -> Option<CompletionSuggestion> {
    let atoms: Vec<&PropertyAtom> = arms
        .iter()
        .filter_map(|arm| match arm {
            SchemaArm::Inline(shape) => shape.properties.get(property),
            SchemaArm::FileRef(_) => None,
        })
        .filter_map(first_completable_atom)
        .collect();

    let first = *atoms.first()?;
    let all_file = atoms
        .iter()
        .all(|atom| matches!(&atom.ty, TypeExpr::Primitive(SimplifiedType::File)));
    if all_file {
        let mut patterns: Vec<String> = Vec::new();
        for atom in &atoms {
            for constraint in &atom.constraints {
                if let Constraint::Match(arm_patterns) = constraint {
                    for pattern in arm_patterns {
                        if !patterns.contains(pattern) {
                            patterns.push(pattern.clone());
                        }
                    }
                }
            }
        }
        return Some(CompletionSuggestion {
            property: property.to_string(),
            is_array: atoms.iter().any(|atom| atom.is_array),
            description: first.description.clone(),
            generated: atoms.iter().all(|atom| atom_is_generated(atom)),
            kind: CompletionKind::File { patterns },
        });
    }

    Some(CompletionSuggestion {
        property: property.to_string(),
        is_array: first.is_array,
        description: first.description.clone(),
        generated: atom_is_generated(first),
        kind: kind_for_atom(first)?,
    })
}

fn suggestion_from_def(property: &str, def: &PropertyDef) -> Option<CompletionSuggestion> {
    let atom = first_completable_atom(def)?;
    let kind = kind_for_atom(atom)?;
    Some(CompletionSuggestion {
        property: property.to_string(),
        is_array: atom.is_array,
        description: atom.description.clone(),
        generated: atom_is_generated(atom),
        kind,
    })
}

fn first_completable_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    match def {
        PropertyDef::Single(atom) => is_completable(&atom.ty).then_some(atom),
        PropertyDef::Union(atoms) => atoms.iter().find(|a| is_completable(&a.ty)),
    }
}

fn atom_is_generated(atom: &PropertyAtom) -> bool {
    atom.constraints
        .iter()
        .chain(atom.array_constraints.iter())
        .any(|constraint| matches!(constraint, Constraint::Generated))
}

fn is_completable(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Primitive(p) => matches!(
            *p,
            SimplifiedType::File
                | SimplifiedType::Enum
                | SimplifiedType::Url
                | SimplifiedType::Email
                | SimplifiedType::Date
                | SimplifiedType::DateTime
                | SimplifiedType::Time
        ),
        // Inline objects don't have a single completable value at the
        // property level — they describe a typed shape. Imported types are
        // resolved before completion runs; an unresolved one is not
        // completable here.
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } => false,
    }
}

fn kind_for_atom(atom: &PropertyAtom) -> Option<CompletionKind> {
    let ty = match &atom.ty {
        TypeExpr::Primitive(ty) => *ty,
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } => return None,
    };
    match ty {
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
        assert!(!suggestion.generated);
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
    fn eager_file_match_patterns_still_reach_completion() {
        // Regression: `match(...)` is no longer lowered into the compiled JSON
        // Schema (the `x-darkmatter-match` keyword was removed), but completion
        // reads patterns from the SimplifiedSchema atom, so they must still
        // surface — including on an eager `file(eager; match(...))` field.
        let eff = effective("$schema:\n  review: \"file(eager; match('**/*review*.md'))\"\n");
        let suggestion = for_property(&eff, "review").expect("review should be completable");
        match suggestion.kind {
            CompletionKind::File { patterns } => {
                assert_eq!(patterns, vec!["**/*review*.md"]);
            }
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
    fn generated_file_property_marks_completion_metadata() {
        let eff = effective("$schema:\n  cover: \"file(generated; match('*.png'))\"\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { patterns } => assert_eq!(patterns, vec!["*.png"]),
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn generated_enum_property_marks_completion_metadata() {
        let eff = effective("$schema:\n  status: enum(draft, published; generated)\n");
        let suggestion = for_property(&eff, "status").expect("status should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Enum { members } => assert_eq!(members, vec!["draft", "published"]),
            other => panic!("expected Enum completion, got {other:?}"),
        }
    }

    #[test]
    fn generated_hint_property_marks_completion_metadata() {
        let eff = effective("$schema:\n  published: date(generated)\n");
        let suggestion = for_property(&eff, "published").expect("published should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("date")),
            other => panic!("expected Hint, got {other:?}"),
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

    #[test]
    fn property_union_uses_selected_arm_generated_metadata() {
        let eff = effective("$schema:\n  href:\n    - url\n    - file(generated)\n");
        let suggestion = for_property(&eff, "href").expect("href should be completable");
        assert!(!suggestion.generated);
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected URL hint from first arm, got {other:?}"),
        }

        let eff = effective("$schema:\n  href:\n    - url(generated)\n    - file\n");
        let suggestion = for_property(&eff, "href").expect("href should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected URL hint from first arm, got {other:?}"),
        }
    }

    #[test]
    fn root_union_offers_completable_properties_from_every_arm() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - spec: \"file(match('**/spec*.md'))\"\n",
            "  - design: \"file(match('**/design*.md'))\"\n",
        ));
        let names = completable_properties(&eff);
        assert_eq!(names, vec!["spec", "design"]);
    }

    #[test]
    fn root_union_for_property_returns_per_arm_file_patterns() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - spec: \"file(match('**/spec*.md'))\"\n",
            "  - design: \"file(match('**/design*.md'))\"\n",
        ));
        match for_property(&eff, "spec").expect("spec completable").kind {
            CompletionKind::File { patterns } => assert_eq!(patterns, vec!["**/spec*.md"]),
            other => panic!("expected File completion for spec, got {other:?}"),
        }
        match for_property(&eff, "design").expect("design completable").kind {
            CompletionKind::File { patterns } => assert_eq!(patterns, vec!["**/design*.md"]),
            other => panic!("expected File completion for design, got {other:?}"),
        }
    }

    #[test]
    fn root_union_combines_file_patterns_for_shared_property() {
        // The same property in two file-typed arms must combine its match
        // globs (deduped) instead of surfacing only the first arm's.
        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(match('**/spec*.md'))\"\n",
            "  - doc: \"file(match('**/design*.md', '**/spec*.md'))\"\n",
        ));
        match for_property(&eff, "doc").expect("doc completable").kind {
            CompletionKind::File { patterns } => {
                assert_eq!(patterns, vec!["**/spec*.md", "**/design*.md"]);
            }
            other => panic!("expected combined File completion, got {other:?}"),
        }
    }

    #[test]
    fn root_union_file_property_is_generated_only_when_every_arm_is_generated() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(generated; match('**/spec*.md'))\"\n",
            "  - doc: \"file(generated; match('**/design*.md'))\"\n",
        ));
        let suggestion = for_property(&eff, "doc").expect("doc completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { patterns } => {
                assert_eq!(patterns, vec!["**/spec*.md", "**/design*.md"]);
            }
            other => panic!("expected combined File completion, got {other:?}"),
        }

        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(generated; match('**/spec*.md'))\"\n",
            "  - doc: \"file(match('**/design*.md'))\"\n",
        ));
        let suggestion = for_property(&eff, "doc").expect("doc completable");
        assert!(!suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { patterns } => {
                assert_eq!(patterns, vec!["**/spec*.md", "**/design*.md"]);
            }
            other => panic!("expected combined File completion, got {other:?}"),
        }
    }

    #[test]
    fn root_union_non_file_property_uses_selected_arm_generated_metadata() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - status: enum(draft, published; generated)\n",
            "  - status: enum(archived)\n",
        ));
        let suggestion = for_property(&eff, "status").expect("status completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Enum { members } => assert_eq!(members, vec!["draft", "published"]),
            other => panic!("expected Enum completion, got {other:?}"),
        }
    }
}
