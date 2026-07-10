//! Library-facing completion query over resolved SimplifiedSchema suggestions.
//!
//! The public entry point [`suggestions_for_path`] returns lint-valid
//! `suggest(...)` candidates for a property path, preserving declaration order
//! and retaining scalar/array-item type information. This is the single source
//! DMLS completion reads from — it does not consult raw generated
//! `x-darkmatter-suggest` annotations.
//!
//! ## Union selection
//!
//! - **Property-level union**: the sole eligible suggestion-bearing atom is
//!   used (cardinality guarantees at most one). All atoms remain linted.
//! - **Root-level union**: the first declaration-order arm with an eligible
//!   suggestion-bearing definition for the property is used. All arms remain
//!   linted.
//!
//! ## Side-effect freedom
//!
//! The query performs no filesystem discovery, shell execution, composition
//! directive evaluation, or network access. It reads the SimplifiedSchema AST
//! and the in-memory lint results only.

use super::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema,
    SuggestionCandidate, SuggestionLintProblem, TypeExpr, lint_suggestions,
};

/// One lint-valid suggestion candidate available for value completion.
///
/// Produced by [`suggestions_for_path`]. Each item carries YAML-safe
/// insertion text ready for a text edit: double-quoted for strings (with
/// required escaping), canonical decimal for numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionItem {
    /// Decoded argument text (after quote/escape processing).
    pub decoded: String,
    /// Target-directed interpreted JSON value.
    pub value: serde_json::Value,
    /// Canonical decimal text for numeric candidates. `None` for strings.
    pub canonical_decimal: Option<String>,
    /// YAML-safe insertion text: double-quoted for strings, canonical decimal
    /// for numbers.
    pub insert_text: String,
    /// Display label (decoded text for strings, canonical decimal for numbers).
    pub label: String,
    /// Whether this candidate targets a `string` property.
    pub is_string: bool,
    /// Whether this candidate targets a `number` property.
    pub is_number: bool,
}

/// The result of querying suggestions for one property path.
///
/// Returned by [`suggestions_for_path`]. Candidates are in declaration order
/// with invalid candidates (those identified by [`super::lint_suggestions`])
/// omitted; valid siblings are retained.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionQuery {
    /// Whether the eligible type is an array form (`string[]` / `number[]`).
    ///
    /// When `true`, completion should insert one candidate element at the
    /// cursor — not a collection of suggestions or an entire array.
    pub is_array: bool,
    /// Lint-valid candidates in declaration order.
    pub items: Vec<SuggestionItem>,
}

/// Queries lint-valid suggestion candidates for `property_path` in `schema`.
///
/// `property_path` is a slice of segments (e.g. `["settings", "mode"]`) that
/// descends through inline objects. A single segment queries a top-level
/// property.
///
/// Returns `None` when the property has no `suggest(...)` constraint, when the
/// schema is a raw JSON Schema (`SimplifiedSchema` unavailable), or when the
/// path does not resolve.
///
/// Invalid candidates (those with a lint problem) are omitted; valid siblings
/// are retained in declaration order.
///
/// ## Union selection
///
/// For a property-level union, the sole eligible suggestion-bearing atom is
/// used (cardinality guarantees at most one). For a root-level union, the
/// first declaration-order arm with an eligible suggestion-bearing definition
/// for the property is used. All arms and atoms remain linted regardless of
/// which is selected for completion.
///
/// ## Side-effect freedom
///
/// No filesystem discovery, shell execution, composition directive evaluation,
/// or network access occurs.
pub fn suggestions_for_path(
    schema: &SimplifiedSchema,
    property_path: &[&str],
) -> Option<SuggestionQuery> {
    let (leaf, ancestors) = property_path.split_last()?;
    let atom = suggestion_atom_for_path(schema, ancestors, leaf)?;
    let candidates = atom.constraints.iter().find_map(|c| match c {
        Constraint::Suggest(candidates) => Some(candidates.as_slice()),
        _ => None,
    })?;

    let is_array = atom.is_array;
    let is_string = matches!(atom.ty, TypeExpr::Primitive(super::SimplifiedType::String));
    let is_number = matches!(atom.ty, TypeExpr::Primitive(super::SimplifiedType::Number));

    let lints = lint_suggestions(schema).unwrap_or_default();
    let (root_arm, property_arm) = locate_atom(schema, ancestors, leaf);

    let items = candidates
        .iter()
        .filter(|candidate| !is_lint_problem(candidates, &lints, &root_arm, &property_arm, candidate))
        .map(|candidate| build_item(candidate, is_string, is_number))
        .collect();

    Some(SuggestionQuery { is_array, items })
}

/// Resolves the [`PropertyAtom`] carrying `suggest(...)` for a path, applying
/// the union-selection rules.
fn suggestion_atom_for_path<'a>(
    schema: &'a SimplifiedSchema,
    ancestors: &[&str],
    leaf: &str,
) -> Option<&'a PropertyAtom> {
    match schema {
        SimplifiedSchema::Single(shape) => {
            let shape = descend(shape, ancestors)?;
            suggest_atom_in_def(shape.properties.get(leaf)?)
        }
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    let shape = descend(shape, ancestors)?;
                    if let Some(def) = shape.properties.get(leaf)
                        && let Some(atom) = suggest_atom_in_def(def)
                    {
                        return Some(atom);
                    }
                }
            }
            None
        }
    }
}

/// Descends through inline-object ancestors to reach the leaf's shape.
fn descend<'a>(root: &'a SchemaShape, ancestors: &[&str]) -> Option<&'a SchemaShape> {
    let mut shape = root;
    for segment in ancestors {
        let def = shape.properties.get(*segment)?;
        shape = inline_object_shape(def)?;
    }
    Some(shape)
}

/// The inline-object [`SchemaShape`] of the first inline-object arm.
fn inline_object_shape(def: &PropertyDef) -> Option<&SchemaShape> {
    atoms_of(def).iter().find_map(|atom| match &atom.ty {
        TypeExpr::InlineObject(inner) => Some(inner),
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => None,
    })
}

/// Finds the atom carrying `suggest(...)` in a [`PropertyDef`].
fn suggest_atom_in_def(def: &PropertyDef) -> Option<&PropertyAtom> {
    atoms_of(def).iter().find(|atom| {
        atom.constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Suggest(_)))
    })
}

/// Returns the arm indices for lint-problem matching.
fn locate_atom(
    schema: &SimplifiedSchema,
    ancestors: &[&str],
    leaf: &str,
) -> (Option<usize>, Option<usize>) {
    match schema {
        SimplifiedSchema::Single(shape) => {
            if let Some(def) = resolve_def(shape, ancestors, leaf) {
                (None, property_arm_index(def))
            } else {
                (None, None)
            }
        }
        SimplifiedSchema::Union(arms) => {
            for (root_arm, arm) in arms.iter().enumerate() {
                if let SchemaArm::Inline(shape) = arm
                    && let Some(def) = resolve_def(shape, ancestors, leaf)
                {
                    return (Some(root_arm), property_arm_index(def));
                }
            }
            (None, None)
        }
    }
}

fn resolve_def<'a>(
    shape: &'a SchemaShape,
    ancestors: &[&str],
    leaf: &str,
) -> Option<&'a PropertyDef> {
    let shape = descend(shape, ancestors)?;
    shape.properties.get(leaf)
}

fn property_arm_index(def: &PropertyDef) -> Option<usize> {
    match def {
        PropertyDef::Single(_) => None,
        PropertyDef::Union(atoms) => atoms
            .iter()
            .position(|atom| atom.constraints.iter().any(|c| matches!(c, Constraint::Suggest(_)))),
    }
}

fn atoms_of(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// Whether a candidate is identified as invalid by the lint problems.
fn is_lint_problem(
    candidates: &[SuggestionCandidate],
    lints: &[SuggestionLintProblem],
    root_arm: &Option<usize>,
    property_arm: &Option<usize>,
    candidate: &SuggestionCandidate,
) -> bool {
    let index = candidates
        .iter()
        .position(|c| c.decoded == candidate.decoded && c.interpreted == candidate.interpreted);
    let Some(index) = index else {
        return false;
    };
    lints.iter().any(|problem| {
        problem.root_arm == *root_arm
            && problem.property_arm == *property_arm
            && problem.decoded == candidate.decoded
            && problem.interpreted == candidate.interpreted
            && candidate_index_matches(problem, candidates, index)
    })
}

fn candidate_index_matches(
    problem: &SuggestionLintProblem,
    candidates: &[SuggestionCandidate],
    target_index: usize,
) -> bool {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.decoded == problem.decoded && c.interpreted == problem.interpreted && problem.span == c.span
        })
        .any(|(i, _)| i == target_index)
}

/// Builds a [`SuggestionItem`] from a candidate.
fn build_item(candidate: &SuggestionCandidate, is_string: bool, is_number: bool) -> SuggestionItem {
    let (label, insert_text) = if is_number {
        let text = candidate
            .canonical_decimal
            .as_deref()
            .unwrap_or(candidate.decoded.as_str());
        (text.to_string(), text.to_string())
    } else {
        (
            candidate.decoded.clone(),
            yaml_double_quote(&candidate.decoded),
        )
    };
    SuggestionItem {
        decoded: candidate.decoded.clone(),
        value: candidate.interpreted.clone(),
        canonical_decimal: candidate.canonical_decimal.clone(),
        insert_text,
        label,
        is_string,
        is_number,
    }
}

/// Produces a YAML-safe double-quoted scalar from `text`.
///
/// Escapes backslash, double quote, and common control characters using YAML
/// 1.2 double-quoted escape sequences (compatible with JSON escaping).
pub fn yaml_double_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\x0b' => out.push_str("\\v"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Prefix-matches a candidate's display label against `prefix`, case-sensitive.
pub fn matches_prefix(item: &SuggestionItem, prefix: &str) -> bool {
    item.label.starts_with(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(yaml_body: &str) -> SimplifiedSchema {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_body).unwrap();
        let schema_value = value.get("$schema").unwrap();
        super::super::parse_yaml_schema(schema_value).unwrap()
    }

    #[test]
    fn string_suggestions_preserve_order() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_path(&s, &["color"]).unwrap();
        assert!(!q.is_array);
        assert_eq!(q.items.len(), 3);
        assert_eq!(q.items[0].label, "red");
        assert_eq!(q.items[0].insert_text, "\"red\"");
        assert_eq!(q.items[1].label, "green");
        assert_eq!(q.items[1].insert_text, "\"green\"");
        assert_eq!(q.items[2].label, "blue");
    }

    #[test]
    fn number_suggestions_use_canonical_decimal() {
        let s = schema("$schema:\n  ratio: number(suggest(0.25, 0.5, 1))\n");
        let q = suggestions_for_path(&s, &["ratio"]).unwrap();
        assert!(!q.is_array);
        assert_eq!(q.items.len(), 3);
        assert_eq!(q.items[0].label, "0.25");
        assert_eq!(q.items[0].insert_text, "0.25");
        assert_eq!(q.items[1].label, "0.5");
        assert_eq!(q.items[1].insert_text, "0.5");
        assert_eq!(q.items[2].label, "1");
        assert_eq!(q.items[2].insert_text, "1");
    }

    #[test]
    fn array_form_sets_is_array() {
        let s = schema("$schema:\n  tags: string(suggest(alpha, beta))[]\n");
        let q = suggestions_for_path(&s, &["tags"]).unwrap();
        assert!(q.is_array);
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "alpha");
        assert_eq!(q.items[0].insert_text, "\"alpha\"");
    }

    #[test]
    fn invalid_candidates_are_omitted() {
        let s = schema("$schema:\n  count: number(min(0); suggest(1, many, 2))\n");
        let q = suggestions_for_path(&s, &["count"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "1");
        assert_eq!(q.items[1].label, "2");
    }

    #[test]
    fn no_suggest_returns_none() {
        let s = schema("$schema:\n  title: string\n");
        assert!(suggestions_for_path(&s, &["title"]).is_none());
    }

    #[test]
    fn unknown_property_returns_none() {
        let s = schema("$schema:\n  title: string\n");
        assert!(suggestions_for_path(&s, &["nope"]).is_none());
    }

    #[test]
    fn nested_inline_object_descends() {
        let s = schema("$schema:\n  settings: \"{ mode: string(suggest(fast, slow)) }\"\n");
        let q = suggestions_for_path(&s, &["settings", "mode"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "fast");
        assert_eq!(q.items[1].label, "slow");
    }

    #[test]
    fn property_union_uses_sole_suggestion_arm() {
        let s = schema("$schema:\n  choice:\n    - boolean\n    - string(suggest(first, second))\n");
        let q = suggestions_for_path(&s, &["choice"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "first");
        assert_eq!(q.items[1].label, "second");
    }

    #[test]
    fn root_union_uses_first_suggestion_arm() {
        let s = schema("$schema:\n  - root: string(suggest(arm-one))\n  - root: string(suggest(arm-two))\n");
        let q = suggestions_for_path(&s, &["root"]).unwrap();
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].label, "arm-one");
    }

    #[test]
    fn prefix_filter_matches_label() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_path(&s, &["color"]).unwrap();
        let filtered: Vec<&SuggestionItem> =
            q.items.iter().filter(|item| matches_prefix(item, "gr")).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "green");
    }

    #[test]
    fn empty_prefix_matches_all() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_path(&s, &["color"]).unwrap();
        let count = q.items.iter().filter(|item| matches_prefix(item, "")).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn yaml_double_quote_escapes_special_chars() {
        assert_eq!(yaml_double_quote("simple"), "\"simple\"");
        assert_eq!(yaml_double_quote("with \"quotes\""), "\"with \\\"quotes\\\"\"");
        assert_eq!(yaml_double_quote("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(yaml_double_quote("line\nbreak"), "\"line\\nbreak\"");
    }

    #[test]
    fn negative_number_uses_canonical_decimal() {
        let s = schema("$schema:\n  score: number(min(-100); max(100); suggest(-1, 50, 101))\n");
        let q = suggestions_for_path(&s, &["score"]).unwrap();
        assert_eq!(q.items[0].label, "-1");
        assert_eq!(q.items[0].insert_text, "-1");
    }

    #[test]
    fn number_integer_constraint_suggestions() {
        let s = schema("$schema:\n  port: number(integer; suggest(80, 443))\n");
        let q = suggestions_for_path(&s, &["port"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "80");
        assert_eq!(q.items[1].label, "443");
    }

    #[test]
    fn string_with_spaces_is_quoted() {
        let s = schema("$schema:\n  v: string(suggest(\"blue gray\", plain))\n");
        let q = suggestions_for_path(&s, &["v"]).unwrap();
        assert_eq!(q.items[0].label, "blue gray");
        assert_eq!(q.items[0].insert_text, "\"blue gray\"");
    }
}
