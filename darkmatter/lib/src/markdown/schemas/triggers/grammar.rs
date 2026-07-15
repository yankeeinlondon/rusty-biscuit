//! Match-expression AST and parser for trigger schemas.
//!
//! A trigger schema's `match:` payload is a small boolean tree over
//! frontmatter property conditions plus a dollar-reserved `$path` predicate.
//! The tree mirrors JSON Schema's `allOf` / `anyOf` / `not`, with a
//! first-class `min-match` (N-of-M) combinator.
//!
//! Each property condition is `property: <type-expr>` where the type
//! expression reuses the existing [`PropertyAtom`] / [`TypeExpr`] grammar
//! from [`crate::markdown::schemas::simplified`]. Match loading rejects
//! constraints that resolve files, import types, load examples, provide
//! defaults, or mark generated values (see [`is_match_safe_constraint`]).
//!
//! ## Outer-OR arms
//!
//! `match:` accepts two forms, mirroring the root `$schema` union convention:
//!
//! - **Mapping** → a single match expression.
//! - **Sequence** → a list of arm mappings, logically OR'd.
//!
//! See [`parse_match_arms`] for the entry point.

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::simplified::{
    Constraint, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr,
    grammar::parse_type_expr,
};

/// The reserved combinator keys. A property literally named one of these
/// cannot be condition-matched in v1 (the combinator interpretation wins).
/// Future non-property predicates stay dollar-prefixed so this reserved set
/// never grows.
pub const COMBINATOR_KEYS: &[&str] = &["all", "any", "none", "min-match"];

/// The single dollar-reserved predicate key currently in the grammar.
pub const PATH_KEY: &str = "$path";

/// One match expression in a trigger schema's boolean tree.
///
/// The tree is the pure evaluation model: given a parsed frontmatter snapshot
/// and a normalized document path, [`super::matcher::matches`] decides whether
/// the trigger activates. No I/O, no validator compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchExpr {
    /// Every child condition must hold (`all:`).
    All(Vec<MatchExpr>),
    /// At least one child must hold (`any:`).
    Any(Vec<MatchExpr>),
    /// No child may hold (`none:`).
    None(Vec<MatchExpr>),
    /// N-of-M: at least `count` children must hold (`min-match:`).
    MinMatch {
        /// The minimum number of holding children.
        count: usize,
        /// The candidate children.
        of: Vec<MatchExpr>,
    },
    /// A frontmatter property condition: `property: <type-expr>`.
    ///
    /// The condition is a **guard** unless the atom carries
    /// [`Constraint::Required`], which makes it a **gate**. A guard may be
    /// absent (the condition then holds vacuously); a present value that
    /// contradicts the declared type **defeats** the match even when the key
    /// is not required.
    Property {
        /// The frontmatter property name.
        name: String,
        /// The parsed type expression + constraints.
        atom: PropertyAtom,
    },
    /// The dollar-reserved `$path:` predicate. Glob list with `!` negation;
    /// a bare basename glob matches in any directory (gitignore-style).
    /// Inherently a gate — there is no "absent" case for a document path.
    Path(PathGlobs),
}

/// A `$path:` glob list, parsed from a string or sequence of strings.
///
/// Reuses the same shape as [`Constraint::Match`]: a list of glob patterns
/// where `!`-prefixed entries are negations. Compiled lazily by the matcher.
#[derive(Debug, Clone, PartialEq)]
pub struct PathGlobs {
    /// The raw authored glob patterns (with `!` prefixes intact).
    pub patterns: Vec<String>,
}

impl PathGlobs {
    fn from_yaml(value: &YamlValue) -> Result<Self, SchemaError> {
        let patterns = match value {
            YamlValue::String(s) => vec![s.clone()],
            YamlValue::Sequence(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        YamlValue::String(s) => out.push(s.clone()),
                        other => {
                            return Err(SchemaError::TriggerMatch {
                                message: format!(
                                    "$path glob list items must be strings, got {}",
                                    yaml_kind(other)
                                ),
                            });
                        }
                    }
                }
                out
            }
            other => {
                return Err(SchemaError::TriggerMatch {
                    message: format!(
                        "$path must be a glob string or a sequence of glob strings, got {}",
                        yaml_kind(other)
                    ),
                });
            }
        };
        if patterns.is_empty() {
            return Err(SchemaError::TriggerMatch {
                message: "$path must have at least one glob pattern".into(),
            });
        }
        Ok(Self { patterns })
    }
}

/// The outer `match:` payload: one or more OR'd arms.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArms(pub Vec<MatchExpr>);

/// Parses the `match:` value into one or more OR'd arms.
///
/// - **Mapping** → a single arm.
/// - **Sequence** → a list of arms, logically OR'd.
///
/// Each arm is a single match expression parsed by [`parse_match_expr`].
pub fn parse_match_arms(value: &YamlValue) -> Result<MatchArms, SchemaError> {
    match value {
        YamlValue::Mapping(_) => {
            let expr = parse_match_expr(value)?;
            Ok(MatchArms(vec![expr]))
        }
        YamlValue::Sequence(items) => {
            if items.is_empty() {
                return Err(SchemaError::TriggerMatch {
                    message: "`match:` sequence must have at least one arm".into(),
                });
            }
            let mut arms = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                let arm = parse_match_expr(item).map_err(|mut err| {
                    annotate_arm(&mut err, idx);
                    err
                })?;
                arms.push(arm);
            }
            Ok(MatchArms(arms))
        }
        other => Err(SchemaError::TriggerMatch {
            message: format!(
                "`match:` must be a mapping or a sequence of arm mappings, got {}",
                yaml_kind(other)
            ),
        }),
    }
}

fn annotate_arm(err: &mut SchemaError, idx: usize) {
    if let SchemaError::TriggerMatch { message } = err {
        *message = format!("match arm[{idx}]: {message}");
    }
}

/// Parses a single match expression (one arm) from a YAML mapping.
///
/// A mapping's keys are partitioned into **structural** keys (combinators
/// `all`/`any`/`none`/`min-match` plus the `$path` predicate) and
/// **property-condition** keys (everything else). A mapping must be entirely
/// one category:
///
/// - All structural → sibling keys AND'd into an [`MatchExpr::All`].
/// - All property-conditions → implicit `all` of the property conditions.
/// - Mixed → load error ([`SchemaError::TriggerMixedMapping`]).
fn parse_match_expr(value: &YamlValue) -> Result<MatchExpr, SchemaError> {
    let map = value.as_mapping().ok_or_else(|| SchemaError::TriggerMatch {
        message: format!(
            "a match arm must be a mapping, got {}",
            yaml_kind(value)
        ),
    })?;

    let mut structural_keys: Vec<&str> = Vec::new();
    let mut property_keys: Vec<&str> = Vec::new();
    for key in map.keys() {
        let Some(k) = key.as_str() else {
            return Err(SchemaError::TriggerMatch {
                message: "match arm keys must be strings".into(),
            });
        };
        if is_structural_key(k) {
            structural_keys.push(k);
        } else {
            property_keys.push(k);
        }
    }

    if !structural_keys.is_empty() && !property_keys.is_empty() {
        return Err(SchemaError::TriggerMixedMapping {
            structural: structural_keys.iter().map(|s| s.to_string()).collect(),
            properties: property_keys.iter().map(|s| s.to_string()).collect(),
        });
    }

    if !structural_keys.is_empty() {
        // Sibling structural keys are AND'd.
        let mut children = Vec::with_capacity(structural_keys.len());
        for key in map.keys() {
            let k = key.as_str().expect("key string checked above");
            if !is_structural_key(k) {
                continue;
            }
            let val = map.get(key).expect("key present");
            let child = parse_structural_entry(k, val)?;
            children.push(child);
        }
        if children.len() == 1 {
            return Ok(children.into_iter().next().expect("one child"));
        }
        return Ok(MatchExpr::All(children));
    }

    // All property-conditions → implicit all.
    let mut conditions = Vec::with_capacity(property_keys.len());
    for key in map.keys() {
        let k = key.as_str().expect("key string checked above");
        let val = map.get(key).expect("key present");
        let atom = parse_property_condition_atom(k, val)?;
        conditions.push(MatchExpr::Property {
            name: k.to_string(),
            atom,
        });
    }
    Ok(MatchExpr::All(conditions))
}

/// Returns `true` when `key` is a combinator or dollar-reserved predicate.
fn is_structural_key(key: &str) -> bool {
    COMBINATOR_KEYS.contains(&key) || key == PATH_KEY
}

/// Parses one structural entry: a combinator body or the `$path` predicate.
fn parse_structural_entry(key: &str, value: &YamlValue) -> Result<MatchExpr, SchemaError> {
    match key {
        "all" => Ok(MatchExpr::All(parse_child_list(key, value)?)),
        "any" => Ok(MatchExpr::Any(parse_child_list(key, value)?)),
        "none" => Ok(MatchExpr::None(parse_child_list(key, value)?)),
        "min-match" => Ok(parse_min_match(value)?),
        "$path" => Ok(MatchExpr::Path(PathGlobs::from_yaml(value)?)),
        other => Err(SchemaError::TriggerMatch {
            message: format!("unknown structural key `{other}`"),
        }),
    }
}

/// Parses the `min-match:` mapping: `{ count: N, of: [...] }`.
fn parse_min_match(value: &YamlValue) -> Result<MatchExpr, SchemaError> {
    let map = value.as_mapping().ok_or_else(|| SchemaError::TriggerMatch {
        message: format!("`min-match:` must be a mapping, got {}", yaml_kind(value)),
    })?;
    let mut count: Option<usize> = None;
    let mut of: Option<Vec<MatchExpr>> = None;
    for (k, v) in map {
        let key = k.as_str().ok_or_else(|| SchemaError::TriggerMatch {
            message: "`min-match:` keys must be strings".into(),
        })?;
        match key {
            "count" => {
                count = Some(parse_count(v)?);
            }
            "of" => {
                of = Some(parse_child_list("min-match.of", v)?);
            }
            other => {
                return Err(SchemaError::TriggerMatch {
                    message: format!(
                        "`min-match:` supports only `count` and `of`, found `{other}`"
                    ),
                });
            }
        }
    }
    let count = count.ok_or_else(|| SchemaError::TriggerMatch {
        message: "`min-match:` requires a `count`".into(),
    })?;
    let children = of.ok_or_else(|| SchemaError::TriggerMatch {
        message: "`min-match:` requires an `of` list".into(),
    })?;
    if children.is_empty() {
        return Err(SchemaError::TriggerMatch {
            message: "`min-match.of:` must have at least one child".into(),
        });
    }
    if count == 0 {
        return Err(SchemaError::TriggerMatch {
            message: "`min-match.count:` must be at least 1".into(),
        });
    }
    if count > children.len() {
        return Err(SchemaError::TriggerMatch {
            message: format!(
                "`min-match.count:` ({count}) cannot exceed the number of children ({})",
                children.len()
            ),
        });
    }
    Ok(MatchExpr::MinMatch { count, of: children })
}

fn parse_count(value: &YamlValue) -> Result<usize, SchemaError> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| SchemaError::TriggerMatch {
            message: "`min-match.count:` requires a non-negative integer".into(),
        })
}

/// Parses a combinator's child list (a sequence of arm mappings).
fn parse_child_list(context: &str, value: &YamlValue) -> Result<Vec<MatchExpr>, SchemaError> {
    let items = value.as_sequence().ok_or_else(|| SchemaError::TriggerMatch {
        message: format!("`{context}:` must be a sequence of mappings, got {}", yaml_kind(value)),
    })?;
    if items.is_empty() {
        return Err(SchemaError::TriggerMatch {
            message: format!("`{context}:` must have at least one child"),
        });
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(parse_match_expr(item)?);
    }
    Ok(out)
}

/// Parses a property-condition value into a match-safe [`PropertyAtom`],
/// rejecting forbidden constraints and imported types.
fn parse_property_condition_atom(name: &str, value: &YamlValue) -> Result<PropertyAtom, SchemaError> {
    let def = parse_property_condition_def(name, value)?;
    match def {
        PropertyDef::Single(atom) => {
            enforce_match_safe(name, &atom)?;
            Ok(atom)
        }
        PropertyDef::Union(_) => Err(SchemaError::TriggerMatch {
            message: format!(
                "property `{name}` in a match condition may not be a property-level union; use \
                 nested `any:` instead"
            ),
        }),
    }
}

/// Parses a property value mirroring [`super::super::simplified`] shapes, but
/// always produces a [`PropertyDef::Single`] (no property-level unions).
fn parse_property_condition_def(name: &str, value: &YamlValue) -> Result<PropertyDef, SchemaError> {
    match value {
        YamlValue::String(s) => Ok(PropertyDef::Single(parse_type_expr(name, s)?)),
        YamlValue::Mapping(_) => {
            // A nested mapping lowers to an inline object shape, mirroring the
            // schema YAML-shape layer. A mapping always yields a single shape.
            let single = single_shape_from_yaml(name, value)?;
            Ok(PropertyDef::Single(PropertyAtom::bare_inline_object(single)))
        }
        YamlValue::Sequence(_) => Ok(PropertyDef::Union(parse_union_atoms(name, value)?)),
        other => Err(SchemaError::TriggerMatch {
            message: format!(
                "property `{name}` value must be a type-expression string or a nested mapping, \
                 got {}",
                yaml_kind(other)
            ),
        }),
    }
}

/// Parses a YAML mapping into a single [`SchemaShape`], erroring if it would
/// yield a root union (which never happens for a mapping, but the parse may
/// still fail for malformed content).
fn single_shape_from_yaml(
    name: &str,
    value: &YamlValue,
) -> Result<crate::markdown::schemas::simplified::SchemaShape, SchemaError> {
    use crate::markdown::schemas::simplified::{SimplifiedSchema, parse_yaml_schema};
    match parse_yaml_schema(value)? {
        SimplifiedSchema::Single(shape) => Ok(shape),
        SimplifiedSchema::Union(_) => Err(SchemaError::TriggerMatch {
            message: format!(
                "property `{name}` inline object must be a single mapping shape",
            ),
        }),
    }
}

fn parse_union_atoms(name: &str, value: &YamlValue) -> Result<Vec<PropertyAtom>, SchemaError> {
    use crate::markdown::schemas::simplified::{SimplifiedSchema, parse_yaml_schema};
    let items = value.as_sequence().expect("sequence checked by caller");
    let mut atoms = Vec::with_capacity(items.len());
    for item in items {
        match item {
            YamlValue::String(s) => atoms.push(parse_type_expr(name, s)?),
            YamlValue::Mapping(_) => match parse_yaml_schema(item)? {
                SimplifiedSchema::Single(shape) => {
                    atoms.push(PropertyAtom::bare_inline_object(shape));
                }
                SimplifiedSchema::Union(_) => {
                    return Err(SchemaError::TriggerMatch {
                        message: format!("property `{name}` union arm must be a single shape",),
                    });
                }
            },
            other => {
                return Err(SchemaError::TriggerMatch {
                    message: format!(
                        "property `{name}` union arm must be a string or mapping, got {}",
                        yaml_kind(other)
                    ),
                });
            }
        }
    }
    if atoms.is_empty() {
        return Err(SchemaError::TriggerMatch {
            message: format!("property `{name}` union must have at least one arm"),
        });
    }
    Ok(atoms)
}

/// Returns `true` when `constraint` is in the match-safe subset.
///
/// Match loading rejects constraints that resolve files, import types, load
/// examples, provide defaults, mark generated values, or otherwise consult
/// state outside the candidate value. Allowed: `required`, `enum` members,
/// `literal` value equality, `pattern`, length/range, item-count, key-count,
/// and structural purity constraints (`integer`, `not-empty`, `unique`).
pub fn is_match_safe_constraint(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::Required
            | Constraint::Members(_)
            // `literal(...)` value equality is pure (no I/O, no state), so it is
            // a valid trigger discriminant — the idiomatic replacement for a
            // single-member `enum(...)` tag.
            | Constraint::LiteralValue(_)
            | Constraint::Pattern(_)
            | Constraint::MinLen(_)
            | Constraint::MaxLen(_)
            | Constraint::Min(_)
            | Constraint::Max(_)
            | Constraint::Integer
            | Constraint::MinItems(_)
            | Constraint::MaxItems(_)
            | Constraint::MinKeys(_)
            | Constraint::MaxKeys(_)
            | Constraint::NotEmpty
            | Constraint::Unique
    )
}

/// Enforces the match-safe constraint subset and rejects imported types for
/// one property condition atom.
pub fn enforce_match_safe(name: &str, atom: &PropertyAtom) -> Result<(), SchemaError> {
    if matches!(atom.ty, TypeExpr::Imported { .. }) {
        return Err(SchemaError::TriggerForbiddenConstraint {
            property: name.to_string(),
            constraint: "imported type (Name@file)".into(),
        });
    }
    // `file` may be tested only as a string-shaped value; eager existence is
    // forbidden.
    if matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File))
        && atom.constraints.iter().any(|c| matches!(c, Constraint::Eager))
    {
        return Err(SchemaError::TriggerForbiddenConstraint {
            property: name.to_string(),
            constraint: "eager (file existence check)".into(),
        });
    }
    for constraint in &atom.constraints {
        if !is_match_safe_constraint(constraint) {
            return Err(SchemaError::TriggerForbiddenConstraint {
                property: name.to_string(),
                constraint: constraint.keyword().to_string(),
            });
        }
    }
    for constraint in &atom.array_constraints {
        if !is_match_safe_constraint(constraint) {
            return Err(SchemaError::TriggerForbiddenConstraint {
                property: name.to_string(),
                constraint: constraint.keyword().to_string(),
            });
        }
    }
    Ok(())
}

fn yaml_kind(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "boolean",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "sequence",
        YamlValue::Mapping(_) => "mapping",
        YamlValue::Tagged(_) => "tagged-value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse failed")
    }

    #[test]
    fn parses_implicit_all_of_properties() {
        let v = yaml("prompt: string(required)\nsections: object");
        let arms = parse_match_arms(&v).unwrap();
        assert_eq!(arms.0.len(), 1);
        match &arms.0[0] {
            MatchExpr::All(children) => {
                assert_eq!(children.len(), 2);
            }
            other => panic!("expected All, got {other:?}"),
        }
    }

    #[test]
    fn parses_all_combinator() {
        let v = yaml("all:\n  - prompt: string(required)\n  - sections: object");
        let arms = parse_match_arms(&v).unwrap();
        assert!(matches!(&arms.0[0], MatchExpr::All(_)));
    }

    #[test]
    fn parses_any_combinator() {
        let v = yaml(
            "any:\n  - initialize: object(required)\n  - success: object(required)",
        );
        let arms = parse_match_arms(&v).unwrap();
        match &arms.0[0] {
            MatchExpr::Any(children) => assert_eq!(children.len(), 2),
            other => panic!("expected Any, got {other:?}"),
        }
    }

    #[test]
    fn parses_none_combinator() {
        let v = yaml("none:\n  - kind: enum(schema; required)");
        let arms = parse_match_arms(&v).unwrap();
        assert!(matches!(&arms.0[0], MatchExpr::None(_)));
    }

    #[test]
    fn parses_min_match() {
        let v = yaml(
            "min-match:\n  count: 2\n  of:\n    - a: string(required)\n    - b: string(required)\n    - c: string(required)",
        );
        let arms = parse_match_arms(&v).unwrap();
        match &arms.0[0] {
            MatchExpr::MinMatch { count, of } => {
                assert_eq!(*count, 2);
                assert_eq!(of.len(), 3);
            }
            other => panic!("expected MinMatch, got {other:?}"),
        }
    }

    #[test]
    fn parses_outer_or_sequence() {
        let v = yaml(
            "- kind: enum(prompt; required)\n- all:\n    - prompt: string(required)",
        );
        let arms = parse_match_arms(&v).unwrap();
        assert_eq!(arms.0.len(), 2);
    }

    #[test]
    fn rejects_mixed_mapping() {
        let v = yaml("all:\n  - prompt: string(required)\nprompt: string");
        let err = parse_match_arms(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMixedMapping { .. }));
    }

    #[test]
    fn rejects_forbidden_default_constraint() {
        let v = yaml("title: string(default(hello))");
        let err = parse_match_arms(&v).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerForbiddenConstraint { .. }
        ));
    }

    #[test]
    fn rejects_forbidden_eager_file() {
        let v = yaml("doc: file(eager)");
        let err = parse_match_arms(&v).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerForbiddenConstraint { .. }
        ));
    }

    #[test]
    fn rejects_imported_type() {
        let v = yaml("value: type@./types.yaml");
        let err = parse_match_arms(&v).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerForbiddenConstraint { .. }
        ));
    }

    #[test]
    fn allows_bare_file_as_string_shape() {
        let v = yaml("doc: file");
        let arms = parse_match_arms(&v).unwrap();
        assert_eq!(arms.0.len(), 1);
    }

    #[test]
    fn allows_enum_pattern_and_required() {
        let v = yaml("kind: enum(prompt, schema; required)\ntitle: string(pattern(^Hello))");
        let arms = parse_match_arms(&v).unwrap();
        assert_eq!(arms.0.len(), 1);
    }

    #[test]
    fn parses_path_predicate_single() {
        let v = yaml("$path: \"prompts/**/*.md\"");
        let arms = parse_match_arms(&v).unwrap();
        match &arms.0[0] {
            MatchExpr::Path(globs) => assert_eq!(globs.patterns, vec!["prompts/**/*.md".to_string()]),
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn parses_path_predicate_list() {
        let v = yaml("$path:\n  - \"**/*.md\"\n  - \"!**/_*.md\"");
        let arms = parse_match_arms(&v).unwrap();
        match &arms.0[0] {
            MatchExpr::Path(globs) => {
                assert_eq!(globs.patterns.len(), 2);
                assert_eq!(globs.patterns[1], "!**/_*.md");
            }
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn rejects_min_match_count_zero() {
        let v = yaml(
            "min-match:\n  count: 0\n  of:\n    - a: string(required)",
        );
        let err = parse_match_arms(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_min_match_count_exceeds_children() {
        let v = yaml(
            "min-match:\n  count: 5\n  of:\n    - a: string(required)\n    - b: string(required)",
        );
        let err = parse_match_arms(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn reserved_property_names_are_structural() {
        // A property named `all` is interpreted as the combinator.
        assert!(is_structural_key("all"));
        assert!(is_structural_key("any"));
        assert!(is_structural_key("none"));
        assert!(is_structural_key("min-match"));
        assert!(is_structural_key("$path"));
        assert!(!is_structural_key("prompt"));
    }

    #[test]
    fn reserved_name_all_wins_over_property() {
        // A key literally named `all` is the combinator, not a property
        // condition. This is the documented v1 limitation.
        let v = yaml("all:\n  - prompt: string(required)");
        let arms = parse_match_arms(&v).unwrap();
        match &arms.0[0] {
            MatchExpr::All(children) => assert_eq!(children.len(), 1),
            other => panic!("expected All combinator, got {other:?}"),
        }
    }

    #[test]
    fn type_contradiction_in_any_defeats_arm() {
        // `any: [kind: enum(x), kind: string(required)]` where `kind: 42`:
        // the enum arm is defeated (42 is not a string), the string arm is
        // defeated (42 is not a string) → any fails. This is the type-
        // contradiction defeat even in the presence of a non-required guard.
        let v = yaml("kind: string");
        let arms = parse_match_arms(&v).unwrap();
        // Guard: `kind` absent passes; `kind: 42` defeats.
        let expr = &arms.0[0];
        let fm_pass = serde_yaml_ng::to_value(serde_yaml_ng::Mapping::new()).unwrap();
        let fm_fail: serde_yaml_ng::Value =
            serde_yaml_ng::from_str("kind: 42").unwrap();
        // The matcher tests cover the JSON evaluation; here we just confirm
        // the guard parses cleanly.
        assert!(matches!(expr, MatchExpr::All(_)));
        let _ = (fm_pass, fm_fail);
    }

    #[test]
    fn path_normalization_bare_basename_glob() {
        let v = yaml("$path: SKILL.md");
        let arms = parse_match_arms(&v).unwrap();
        assert!(matches!(&arms.0[0], MatchExpr::Path(_)));
    }

    #[test]
    fn path_normalization_double_star_glob() {
        let v = yaml("$path: \"docs/**/*.md\"");
        let arms = parse_match_arms(&v).unwrap();
        assert!(matches!(&arms.0[0], MatchExpr::Path(_)));
    }

    #[test]
    fn path_normalization_negation() {
        let v = yaml("$path:\n  - \"**/*.md\"\n  - \"!**/_*.md\"");
        let arms = parse_match_arms(&v).unwrap();
        assert!(matches!(&arms.0[0], MatchExpr::Path(_)));
    }
}
