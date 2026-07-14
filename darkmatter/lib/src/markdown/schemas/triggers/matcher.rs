//! Pure trigger-expression matcher.
//!
//! [`matches`] evaluates a [`MatchExpr`] against a parsed frontmatter snapshot
//! and a normalized document path. It is a **pure function**: no I/O, no
//! validator compilation, no filesystem access. This makes it viable
//! per-keystroke in DMLS and composes with the content-hash effective-schema
//! cache.
//!
//! ## Guard vs gate semantics
//!
//! - A bare type-expr key (no `required`) is a **guard**: the key may be
//!   absent, in which case the condition holds vacuously. A present value that
//!   contradicts the declared type **defeats** the match.
//! - `required` makes the condition a **gate**: the key must be present *and*
//!   conform.
//! - `enum(...)` members and `pattern(...)` are value predicates checked when
//!   the value is present (or always, under a gate).
//! - `$path` is inherently a gate — there is no "absent" case for a document
//!   path.
//!
//! ## `$path` glob matching
//!
//! `$path` globs are evaluated against the boundary-relative `/`-separated
//! path (case-sensitive on every platform). A bare basename glob (no `/`)
//! matches in any directory (`SKILL.md` ≡ `**/SKILL.md`, gitignore-style).
//! `!`-prefixed patterns are negations: a path matches when it matches at
//! least one include pattern and no exclusion.

use globset::Glob;
use serde_json::Value;

use crate::markdown::schemas::simplified::{Constraint, PropertyAtom, SimplifiedType, TypeExpr};

use super::grammar::MatchExpr;

/// A compiled glob matcher with its negation flag.
type CompiledGlob = (bool, globset::GlobMatcher);

thread_local! {
    /// Per-thread glob-matcher cache keyed by the joined pattern signature.
    /// Avoids recompiling glob matchers on every per-keystroke evaluation of
    /// the same trigger registry.
    static GLOB_CACHE: std::cell::RefCell<Vec<(String, Vec<CompiledGlob>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Evaluates `expr` against `frontmatter` and `normalized_path`.
///
/// Returns `true` when the trigger activates. Side-effect-free by
/// construction: the signature carries no filesystem, environment, or process
/// types.
pub fn matches(expr: &MatchExpr, frontmatter: &Value, normalized_path: &str) -> bool {
    match expr {
        MatchExpr::All(children) => children.iter().all(|c| matches(c, frontmatter, normalized_path)),
        MatchExpr::Any(children) => children.iter().any(|c| matches(c, frontmatter, normalized_path)),
        MatchExpr::None(children) => !children.iter().any(|c| matches(c, frontmatter, normalized_path)),
        MatchExpr::MinMatch { count, of } => {
            let met = of
                .iter()
                .filter(|c| matches(c, frontmatter, normalized_path))
                .count();
            met >= *count
        }
        MatchExpr::Property { name, atom } => {
            property_matches(frontmatter, name, atom)
        }
        MatchExpr::Path(globs) => path_matches(normalized_path, &globs.patterns),
    }
}

/// Returns the first defeating condition for a non-matching arm, for the
/// `md schema triggers` trace. Returns `None` when the arm matches.
pub fn first_defeat(
    expr: &MatchExpr,
    frontmatter: &Value,
    normalized_path: &str,
) -> Option<String> {
    if matches(expr, frontmatter, normalized_path) {
        return None;
    }
    Some(describe_defeat(expr, frontmatter, normalized_path))
}

fn describe_defeat(expr: &MatchExpr, frontmatter: &Value, path: &str) -> String {
    match expr {
        MatchExpr::Property { name, atom } => {
            let value = frontmatter.get(name);
            match value {
                None if is_required(atom) => format!("`{name}` is required but absent"),
                None => format!("`{name}` absent (guard not satisfied)"),
                Some(v) if !type_conforms(atom, v) => {
                    format!("`{name}` has value `{}` which does not conform", summarize(v))
                }
                Some(v) => format!("`{name}` value `{}` failed a constraint", summarize(v)),
            }
        }
        MatchExpr::Path(globs) => {
            format!("path `{path}` does not match globs {}", fmt_globs(&globs.patterns))
        }
        MatchExpr::All(children) => children
            .iter()
            .find_map(|c| first_defeat(c, frontmatter, path))
            .unwrap_or_else(|| "all-children".into()),
        MatchExpr::Any(_) => "no any-child matched".into(),
        MatchExpr::None(_) => "a none-child matched".into(),
        MatchExpr::MinMatch { count, of } => {
            let met = of.iter().filter(|c| matches(c, frontmatter, path)).count();
            format!("min-match met {met}/{count}")
        }
    }
}

fn summarize(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn fmt_globs(globs: &[String]) -> String {
    globs.iter().map(|g| format!("{g:?}")).collect::<Vec<_>>().join(", ")
}

/// Evaluates one property condition against the frontmatter root.
fn property_matches(frontmatter: &Value, name: &str, atom: &PropertyAtom) -> bool {
    let value = frontmatter.get(name);
    let required = is_required(atom);
    match value {
        None => {
            // Absent: a gate (`required`) defeats; a guard passes.
            !required
        }
        Some(v) => {
            // A present null is treated like an absent value (SimplifiedSchema
            // nullable semantics): required fails, guard passes.
            if v.is_null() {
                return !required;
            }
            type_conforms(atom, v)
        }
    }
}

/// Returns `true` when `value` conforms to the atom's type-expr and the
/// match-safe constraints.
fn type_conforms(atom: &PropertyAtom, value: &Value) -> bool {
    // Array semantics: when `is_array`, the value must be an array and each
    // item must conform to the item type + item constraints; array-level
    // constraints then apply to the array as a whole.
    if atom.is_array {
        let Some(items) = value.as_array() else {
            return false;
        };
        for item in items {
            if !scalar_conforms(atom, item) {
                return false;
            }
        }
        return array_constraints_hold(atom, items);
    }
    scalar_conforms(atom, value)
}

/// Checks a non-array value against the type-expr and value constraints.
fn scalar_conforms(atom: &PropertyAtom, value: &Value) -> bool {
    if !value_matches_type(&atom.ty, value) {
        return false;
    }
    for constraint in &atom.constraints {
        if !constraint_holds(constraint, value) {
            return false;
        }
    }
    true
}

/// Checks the declared structural type against a value.
fn value_matches_type(ty: &TypeExpr, value: &Value) -> bool {
    match ty {
        TypeExpr::Primitive(p) => primitive_matches(*p, value),
        TypeExpr::InlineObject(shape) => {
            let Some(map) = value.as_object() else {
                return false;
            };
            for (name, def) in &shape.properties {
                let atom = match def {
                    crate::markdown::schemas::simplified::PropertyDef::Single(a) => a,
                    crate::markdown::schemas::simplified::PropertyDef::Union(arms) => {
                        let v = map.get(name);
                        match v {
                            None => {
                                // Guard/pass if none required; a union with a required
                                // atom needs the key present.
                                let any_required = arms.iter().any(is_required);
                                if any_required {
                                    return false;
                                }
                                continue;
                            }
                            Some(val) => {
                                if val.is_null() {
                                    let any_required = arms.iter().any(is_required);
                                    if any_required {
                                        return false;
                                    }
                                    continue;
                                }
                                if !arms.iter().any(|a| {
                                    if a.is_array {
                                        val.as_array().is_some_and(|items| {
                                            items.iter().all(|i| scalar_conforms(a, i))
                                        }) && array_constraints_hold(a, val.as_array().expect("checked"))
                                    } else {
                                        scalar_conforms(a, val)
                                    }
                                }) {
                                    return false;
                                }
                                continue;
                            }
                        }
                    }
                };
                let v = map.get(name);
                match v {
                    None => {
                        if is_required(atom) {
                            return false;
                        }
                    }
                    Some(val) if val.is_null() => {
                        if is_required(atom) {
                            return false;
                        }
                    }
                    Some(val) => {
                        if atom.is_array {
                            let Some(items) = val.as_array() else {
                                return false;
                            };
                            for item in items {
                                if !scalar_conforms(atom, item) {
                                    return false;
                                }
                            }
                            if !array_constraints_hold(atom, items) {
                                return false;
                            }
                        } else if !scalar_conforms(atom, val) {
                            return false;
                        }
                    }
                }
            }
            true
        }
        TypeExpr::Imported { .. } => {
            // Imports are rejected at load time; unreachable in a loaded trigger.
            false
        }
    }
}

fn primitive_matches(ty: SimplifiedType, value: &Value) -> bool {
    match ty {
        SimplifiedType::String
        | SimplifiedType::Date
        | SimplifiedType::DateTime
        | SimplifiedType::Time
        | SimplifiedType::Url
        | SimplifiedType::Email
        | SimplifiedType::Yaml
        | SimplifiedType::Json => value.is_string(),
        SimplifiedType::File => {
            // `file` is testable only as a string-shaped value.
            value.is_string()
        }
        SimplifiedType::Number | SimplifiedType::NumberLike => value.is_number(),
        SimplifiedType::Boolean => value.is_boolean(),
        SimplifiedType::Boolish => value.is_boolean() || value.is_string(),
        SimplifiedType::Enum => value.is_string(),
        // `expression` is a content-format string type (like `yaml` / `json`):
        // any string is shape-compatible; parseability is a format concern.
        SimplifiedType::Expression => value.is_string(),
        // `literal` accepts any scalar shape here; typed identity is enforced by
        // the `LiteralValue` equality constraint in `constraint_holds`.
        SimplifiedType::Literal => true,
        SimplifiedType::Object => value.is_object(),
        SimplifiedType::Any => true,
    }
}

/// Checks a single match-safe constraint against a value.
fn constraint_holds(constraint: &Constraint, value: &Value) -> bool {
    match constraint {
        Constraint::Required => {
            // Presence is handled at the property level; for inline-object
            // nested properties the type walk already enforced it. Treat as
            // satisfied here.
            true
        }
        Constraint::Members(members) => value
            .as_str()
            .is_some_and(|s| members.iter().any(|m| m == s)),
        // Typed equality for `literal(...)` discriminants: a document value
        // matches only when it equals the authored scalar, type-sensitively
        // (string `'2'` never satisfies number `literal(2)`).
        Constraint::LiteralValue(expected) => literal_value_matches(value, expected),
        Constraint::Pattern(pattern) => {
            let Some(s) = value.as_str() else { return false };
            regex_match(pattern, s)
        }
        Constraint::MinLen(n) => value.as_str().is_some_and(|s| s.chars().count() >= *n),
        Constraint::MaxLen(n) => value.as_str().is_some_and(|s| s.chars().count() <= *n),
        Constraint::Min(n) => value.as_f64().is_some_and(|x| x >= *n),
        Constraint::Max(n) => value.as_f64().is_some_and(|x| x <= *n),
        Constraint::Integer => value.as_f64().is_some_and(|x| x.fract() == 0.0),
        Constraint::NotEmpty => value.as_str().is_some_and(|s| !s.trim().is_empty()),
        // `Unique`, item/key-count constraints are array/object-level and
        // checked in `array_constraints_hold` / object walks; they are no-ops
        // on a scalar value.
        Constraint::Unique
        | Constraint::MinItems(_)
        | Constraint::MaxItems(_)
        | Constraint::MinKeys(_)
        | Constraint::MaxKeys(_) => true,
        // The remaining constraints are forbidden at load time; unreachable.
        _ => true,
    }
}

/// Type-sensitive equality between a document value and a `literal(...)`
/// discriminant. Equal JSON values match directly; two numbers additionally
/// match when their representations differ only in integer-vs-float encoding
/// (`literal(2)` authored as `i64` versus a decoded `2` scalar). A string and a
/// number never match — that type sensitivity is what makes discriminated
/// unions unambiguous.
fn literal_value_matches(value: &Value, expected: &Value) -> bool {
    if value == expected {
        return true;
    }
    match (value, expected) {
        (Value::Number(a), Value::Number(b)) => numbers_equal(a, b),
        _ => false,
    }
}

/// Exact numeric equality across integer/float JSON encodings. Integer pairs
/// compare exactly (never through `f64`, which cannot represent integers past
/// `2^53`); an integer matches a float only when the float equals it exactly.
fn numbers_equal(a: &serde_json::Number, b: &serde_json::Number) -> bool {
    match (json_int(a), json_int(b)) {
        (Some(x), Some(y)) => x == y,
        (Some(x), None) => float_eq_int(b.as_f64(), x),
        (None, Some(y)) => float_eq_int(a.as_f64(), y),
        (None, None) => a.as_f64() == b.as_f64(),
    }
}

/// Exact integer value of a JSON number, or `None` for a float-encoded number.
fn json_int(n: &serde_json::Number) -> Option<i128> {
    n.as_u64().map(|u| u as i128).or_else(|| n.as_i64().map(|i| i as i128))
}

/// True when `f` is present, integral, and exactly equals integer `x`.
/// (float→int `as` casts saturate in Rust, so an out-of-range float simply
/// fails the equality rather than misbehaving.)
fn float_eq_int(f: Option<f64>, x: i128) -> bool {
    f.is_some_and(|f| f.fract() == 0.0 && f as i128 == x)
}

/// Checks array-level constraints against an array value.
fn array_constraints_hold(atom: &PropertyAtom, items: &[Value]) -> bool {
    for constraint in &atom.array_constraints {
        let ok = match constraint {
            Constraint::Unique => {
                let mut seen = Vec::with_capacity(items.len());
                for item in items {
                    if seen.contains(item) {
                        return false;
                    }
                    seen.push(item.clone());
                }
                true
            }
            Constraint::MinItems(n) => items.len() >= *n,
            Constraint::MaxItems(n) => items.len() <= *n,
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

fn is_required(atom: &PropertyAtom) -> bool {
    atom.constraints.iter().any(|c| matches!(c, Constraint::Required))
}

/// Evaluates a `$path` glob list against a normalized path.
fn path_matches(normalized_path: &str, patterns: &[String]) -> bool {
    let Some(matchers) = compile_globs(patterns) else {
        return false;
    };
    // A path matches when it matches at least one include pattern and no
    // exclusion. This is the gitignore-style include/exclude semantics
    // shared with the `file` type's `match()` constraint.
    let mut included = false;
    for (is_negation, matcher) in &matchers {
        if matcher.is_match(normalized_path) {
            if *is_negation {
                return false;
            }
            included = true;
        }
    }
    included
}

/// Compiles a glob list into matchers, applying gitignore-style basename
/// expansion and preserving `!` negation flags.
fn compile_globs(patterns: &[String]) -> Option<Vec<CompiledGlob>> {
    let signature = patterns.join("\n");
    let cached = GLOB_CACHE.with(|cell| {
        cell.borrow()
            .iter()
            .find(|(sig, _)| sig == &signature)
            .map(|(_, ms)| ms.clone())
    });
    if let Some(ms) = cached {
        return Some(ms);
    }

    let mut out = Vec::with_capacity(patterns.len());
    for raw in patterns {
        let (is_negation, body) = raw
            .strip_prefix('!')
            .map(|b| (true, b))
            .unwrap_or((false, raw.as_str()));
        // Bare basename glob (no `/`) matches in any directory.
        let expanded = if body.contains('/') {
            body.to_string()
        } else {
            format!("**/{body}")
        };
        let glob = Glob::new(&expanded).ok()?;
        out.push((is_negation, glob.compile_matcher()));
    }

    GLOB_CACHE.with(|cell| {
        cell.borrow_mut().push((signature, out.clone()));
    });
    Some(out)
}

fn regex_match(pattern: &str, input: &str) -> bool {
    thread_local! {
        static CACHE: std::cell::RefCell<Vec<(String, regex::Regex)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    CACHE.with(|cell| {
        if let Some(re) = cell.borrow().iter().find(|(p, _)| p == pattern) {
            return re.1.is_match(input);
        }
        match regex::Regex::new(pattern) {
            Ok(re) => {
                let ok = re.is_match(input);
                cell.borrow_mut().push((pattern.to_string(), re));
                ok
            }
            Err(_) => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::{
        Constraint, PropertyAtom, SimplifiedType, TypeExpr,
    };
    use crate::markdown::schemas::triggers::grammar::{MatchExpr, PathGlobs};
    use serde_json::json;

    fn prop(name: &str, ty: SimplifiedType, constraints: Vec<Constraint>) -> MatchExpr {
        MatchExpr::Property {
            name: name.to_string(),
            atom: PropertyAtom {
                ty: TypeExpr::Primitive(ty),
                is_array: false,
                constraints,
                array_constraints: vec![],
                description: None,
            },
        }
    }

    fn frontmatter(pairs: &[(&str, Value)]) -> Value {
        let mut map = serde_json::Map::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        Value::Object(map)
    }

    // ── combinators ──────────────────────────────────────────────────────

    #[test]
    fn all_requires_every_child() {
        let expr = MatchExpr::All(vec![
            prop("a", SimplifiedType::String, vec![Constraint::Required]),
            prop("b", SimplifiedType::String, vec![]),
        ]);
        let fm = frontmatter(&[("a", json!("x")), ("b", json!("y"))]);
        assert!(matches(&expr, &fm, ""));
        // `a` is required but absent → all fails.
        let fm = frontmatter(&[("b", json!("y"))]);
        assert!(!matches(&expr, &fm, ""));
    }

    #[test]
    fn any_needs_one_child() {
        let expr = MatchExpr::Any(vec![
            prop("a", SimplifiedType::String, vec![Constraint::Required]),
            prop("b", SimplifiedType::String, vec![Constraint::Required]),
        ]);
        // `a` present but type-contradicts → defeats; `b` absent → required
        // gate fails → any fails.
        let fm = frontmatter(&[("a", json!(42))]);
        assert!(!matches(&expr, &fm, ""));
        let fm = frontmatter(&[("b", json!("y"))]);
        assert!(matches(&expr, &fm, ""));
    }

    #[test]
    fn none_inverts() {
        let expr = MatchExpr::None(vec![
            prop("a", SimplifiedType::String, vec![Constraint::Required]),
        ]);
        // `a` present → child holds → none fails.
        let fm = frontmatter(&[("a", json!("x"))]);
        assert!(!matches(&expr, &fm, ""));
        // `a` absent → required gate fails → child does not hold → none holds.
        let fm = frontmatter(&[]);
        assert!(matches(&expr, &fm, ""));
    }

    #[test]
    fn guard_in_none_does_not_forbid_absence() {
        // A guard inside `none` holds when absent, so `none:[guard]` forbids
        // the key from being absent-as-conforming. Use `required` to express
        // "forbid presence".
        let expr = MatchExpr::None(vec![
            prop("a", SimplifiedType::String, vec![]),
        ]);
        // `a` present + conforms → child holds → none fails.
        let fm = frontmatter(&[("a", json!("x"))]);
        assert!(!matches(&expr, &fm, ""));
        // `a` absent → guard holds → child holds → none fails.
        let fm = frontmatter(&[]);
        assert!(!matches(&expr, &fm, ""));
    }

    #[test]
    fn min_match_n_of_m() {
        let expr = MatchExpr::MinMatch {
            count: 2,
            of: vec![
                prop("a", SimplifiedType::String, vec![Constraint::Required]),
                prop("b", SimplifiedType::String, vec![Constraint::Required]),
                prop("c", SimplifiedType::String, vec![Constraint::Required]),
            ],
        };
        let fm = frontmatter(&[("a", json!("x")), ("b", json!("y"))]);
        assert!(matches(&expr, &fm, ""));
        let fm = frontmatter(&[("a", json!("x"))]);
        assert!(!matches(&expr, &fm, ""));
    }

    #[test]
    fn free_nesting() {
        let expr = MatchExpr::All(vec![MatchExpr::Any(vec![
            MatchExpr::None(vec![prop("kind", SimplifiedType::String, vec![])]),
            prop("tag", SimplifiedType::String, vec![Constraint::Required]),
        ])]);
        let fm = frontmatter(&[("tag", json!("x"))]);
        assert!(matches(&expr, &fm, ""));
    }

    // ── guard vs gate ────────────────────────────────────────────────────

    #[test]
    fn guard_absent_passes() {
        let expr = prop("maybe", SimplifiedType::String, vec![]);
        let fm = frontmatter(&[]);
        assert!(matches(&expr, &fm, ""));
    }

    #[test]
    fn guard_present_contradiction_defeats() {
        let expr = prop("maybe", SimplifiedType::String, vec![]);
        let fm = frontmatter(&[("maybe", json!(42))]);
        assert!(!matches(&expr, &fm, ""));
    }

    #[test]
    fn gate_required_absent_defeats() {
        let expr = prop("must", SimplifiedType::String, vec![Constraint::Required]);
        let fm = frontmatter(&[]);
        assert!(!matches(&expr, &fm, ""));
    }

    #[test]
    fn gate_required_present_conforms() {
        let expr = prop("must", SimplifiedType::String, vec![Constraint::Required]);
        let fm = frontmatter(&[("must", json!("hi"))]);
        assert!(matches(&expr, &fm, ""));
    }

    #[test]
    fn null_treated_as_absent() {
        let expr = prop("must", SimplifiedType::String, vec![Constraint::Required]);
        let fm = frontmatter(&[("must", Value::Null)]);
        assert!(!matches(&expr, &fm, ""));

        let guard = prop("maybe", SimplifiedType::String, vec![]);
        assert!(matches(&guard, &fm, ""));
    }

    // ── value predicates ─────────────────────────────────────────────────

    #[test]
    fn enum_predicate() {
        let expr = MatchExpr::Property {
            name: "kind".into(),
            atom: PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::Enum),
                is_array: false,
                constraints: vec![Constraint::Members(vec!["prompt".into(), "schema".into()])],
                array_constraints: vec![],
                description: None,
            },
        };
        let fm = frontmatter(&[("kind", json!("prompt"))]);
        assert!(matches(&expr, &fm, ""));
        let fm = frontmatter(&[("kind", json!("other"))]);
        assert!(!matches(&expr, &fm, ""));
    }

    #[test]
    fn pattern_predicate() {
        let expr = prop(
            "title",
            SimplifiedType::String,
            vec![Constraint::Pattern("^Hello".into())],
        );
        let fm = frontmatter(&[("title", json!("Hello world"))]);
        assert!(matches(&expr, &fm, ""));
        let fm = frontmatter(&[("title", json!("Goodbye"))]);
        assert!(!matches(&expr, &fm, ""));
    }

    // ── $path ────────────────────────────────────────────────────────────

    #[test]
    fn path_bare_basename_matches_any_dir() {
        let expr = MatchExpr::Path(PathGlobs {
            patterns: vec!["SKILL.md".into()],
        });
        assert!(matches(&expr, &json!({}), "docs/SKILL.md"));
        assert!(matches(&expr, &json!({}), "SKILL.md"));
        assert!(!matches(&expr, &json!({}), "docs/OTHER.md"));
    }

    #[test]
    fn path_glob_with_separator() {
        let expr = MatchExpr::Path(PathGlobs {
            patterns: vec!["prompts/**/*.md".into()],
        });
        assert!(matches(&expr, &json!({}), "prompts/foo/bar.md"));
        assert!(!matches(&expr, &json!({}), "docs/foo.md"));
    }

    #[test]
    fn path_negation() {
        let expr = MatchExpr::Path(PathGlobs {
            patterns: vec!["**/*.md".into(), "!**/_*.md".into()],
        });
        assert!(matches(&expr, &json!({}), "docs/foo.md"));
        assert!(!matches(&expr, &json!({}), "docs/_draft.md"));
    }

    #[test]
    fn path_case_sensitive() {
        let expr = MatchExpr::Path(PathGlobs {
            patterns: vec!["SKILL.md".into()],
        });
        assert!(matches(&expr, &json!({}), "dir/SKILL.md"));
        assert!(!matches(&expr, &json!({}), "dir/skill.md"));
    }

    // ── first_defeat ─────────────────────────────────────────────────────

    #[test]
    fn first_defeat_describes_missing_required() {
        let expr = prop("must", SimplifiedType::String, vec![Constraint::Required]);
        let defeat = first_defeat(&expr, &frontmatter(&[]), "");
        assert!(defeat.as_deref().unwrap().contains("required"));
    }

    // ── literal / expression discriminants ───────────────────────────────

    #[test]
    fn literal_matches_typed_equality() {
        // `kind: literal(spec)` behaves as a value-equality discriminant.
        let expr = prop(
            "kind",
            SimplifiedType::Literal,
            vec![Constraint::LiteralValue(json!("spec"))],
        );
        assert!(matches(&expr, &frontmatter(&[("kind", json!("spec"))]), ""));
        assert!(!matches(&expr, &frontmatter(&[("kind", json!("other"))]), ""));
    }

    #[test]
    fn literal_number_equality_is_type_sensitive() {
        // `version: literal(2)` (number) must not match the string `'2'`.
        let expr = prop(
            "version",
            SimplifiedType::Literal,
            vec![Constraint::LiteralValue(json!(2))],
        );
        assert!(matches(&expr, &frontmatter(&[("version", json!(2))]), ""));
        assert!(matches(&expr, &frontmatter(&[("version", json!(2.0))]), ""));
        assert!(!matches(&expr, &frontmatter(&[("version", json!("2"))]), ""));
    }

    #[test]
    fn literal_boolean_equality() {
        let expr = prop(
            "archived",
            SimplifiedType::Literal,
            vec![Constraint::LiteralValue(json!(false))],
        );
        assert!(matches(&expr, &frontmatter(&[("archived", json!(false))]), ""));
        assert!(!matches(&expr, &frontmatter(&[("archived", json!(true))]), ""));
        // Type-sensitive: the string "false" is not the boolean literal.
        assert!(!matches(&expr, &frontmatter(&[("archived", json!("false"))]), ""));
    }

    #[test]
    fn optional_literal_guard_passes_when_absent() {
        // A bare (non-required) literal is a guard: absence holds vacuously.
        let expr = prop(
            "kind",
            SimplifiedType::Literal,
            vec![Constraint::LiteralValue(json!("spec"))],
        );
        assert!(matches(&expr, &frontmatter(&[]), ""));
        // Present-but-wrong still defeats.
        assert!(!matches(&expr, &frontmatter(&[("kind", json!("plan"))]), ""));
    }

    #[test]
    fn literal_number_matches_float_encoded_equivalent() {
        // AC7 regression: an integer literal still matches its float encoding for
        // small values (`literal(2)` accepts `2.0`).
        let expr = prop(
            "version",
            SimplifiedType::Literal,
            vec![Constraint::LiteralValue(json!(2))],
        );
        assert!(matches(&expr, &frontmatter(&[("version", json!(2.0))]), ""));
    }

    #[test]
    fn literal_large_integer_rejects_f64_neighbor() {
        // Boundary integers past f64's 2^53 exact range: 2^53+1, i64::MAX,
        // i64::MAX + 1 (a u64), and u64::MAX. Each neighbor collides with the
        // literal under f64 rounding but must be rejected by exact comparison.
        let cases: &[(Value, Value)] = &[
            (json!(9007199254740993_u64), json!(9007199254740992_u64)),
            (json!(9223372036854775807_u64), json!(9223372036854775806_u64)),
            (json!(9223372036854775808_u64), json!(9223372036854775807_u64)),
            (json!(18446744073709551615_u64), json!(18446744073709551614_u64)),
        ];
        for (lit, neighbor) in cases {
            let expr = prop(
                "version",
                SimplifiedType::Literal,
                vec![Constraint::LiteralValue(lit.clone())],
            );
            assert!(
                matches(&expr, &frontmatter(&[("version", lit.clone())]), ""),
                "literal({lit}) must accept {lit}"
            );
            assert!(
                !matches(&expr, &frontmatter(&[("version", neighbor.clone())]), ""),
                "literal({lit}) must reject f64-colliding neighbor {neighbor}"
            );
        }
    }

    #[test]
    fn expression_is_string_shaped_like_yaml_json() {
        // `expression` mirrors the content-format string types in triggers: any
        // string is shape-compatible; a non-string defeats.
        let expr = prop("when", SimplifiedType::Expression, vec![]);
        assert!(matches(&expr, &frontmatter(&[("when", json!("is_agent()"))]), ""));
        assert!(!matches(&expr, &frontmatter(&[("when", json!(3))]), ""));
    }
}
