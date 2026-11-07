//! Command-line setters: `md compose doc.md author.name=Alice`.
//!
//! A setter is kept as the person wrote it until the document it lands on is
//! known. [`SetterPlan::new`] settles overlaps before any document is read, then
//! [`apply_setter_plan`] reads each surviving value once per document, through
//! that document's own schema. A setter therefore reaches a transcluded child
//! as a value of the child's declared type, not of the parent's.

use std::collections::{HashMap, HashSet};

use serde_json::Value;
use simplified_schema::completion::PathStep;
use simplified_schema::{PathEditError, PropertyPath, SetterValue, set_at};

use super::context::effective_state::{ConflictWinner, deep_merge};
use super::schema_validation::{PreparedSchemas, prepare_schemas};
use super::value_origin::{FrontmatterProvenance, ValuePathSegment};
use crate::markdown::Markdown;
use crate::markdown::schemas::completion::declared_shape;
use crate::markdown::schemas::frontmatter_as_json;
use crate::markdown::types::{MarkdownError, MarkdownResult};

/// How `md compose` reads one positional word.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenClass {
    /// `key=value` with a key the setter grammar accepts.
    Setter(RawSetter),
    /// The input document: a word with no `=`, or whose key the setter grammar
    /// refuses, such as `./foo=bar.md`, `9key=value`, or `a..b=1`.
    Path,
    /// `=value`, which names nothing.
    EmptyKey,
}

/// One setter as written, split at its first `=`.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSetter {
    /// The whole word, for messages.
    pub argument: String,
    /// The key, parsed.
    pub path: PropertyPath,
    /// Everything after the first `=`.
    pub raw: String,
}

/// Classifies a positional word of `md compose`.
///
/// This is the one definition of the setter grammar, shared by execution and
/// shell completion. The first segment follows `[A-Za-z_][A-Za-z0-9_-]*`, and a
/// later segment follows it or is all digits. A `.` decoded from `\.` is allowed
/// inside a segment.
#[must_use]
pub fn classify_token(token: &str) -> TokenClass {
    let Some((key, raw)) = token.split_once('=') else {
        return TokenClass::Path;
    };
    if key.is_empty() {
        return TokenClass::EmptyKey;
    }
    let Ok(path) = PropertyPath::parse(key) else {
        return TokenClass::Path;
    };
    let mut segments = path.segments().iter();
    let first_fits = segments.next().is_some_and(|segment| is_named_segment(segment));
    if !first_fits || !segments.all(|segment| is_named_segment(segment) || is_all_digits(segment)) {
        return TokenClass::Path;
    }
    TokenClass::Setter(RawSetter {
        argument: token.to_string(),
        path,
        raw: raw.to_string(),
    })
}

fn is_named_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    characters.next().is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
}

fn is_all_digits(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq)]
struct PlannedSetter {
    argument: String,
    path: PropertyPath,
    value: SetterValue,
}

/// A command line's setters with their overlaps settled: ancestors come before
/// descendants, and independent paths at one depth keep command-line order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetterPlan {
    setters: Vec<PlannedSetter>,
}

impl SetterPlan {
    /// Settles the overlaps among `setters`, in command-line order.
    ///
    /// The same decoded path given twice takes the last value at the first
    /// position; the value it overrides is never read, so it may be invalid.
    ///
    /// ## Errors
    ///
    /// [`MarkdownError::Setter`] for a value that cannot be admitted, and for a
    /// scalar or `null` value at an ancestor of another setter, in either order.
    pub fn new(setters: impl IntoIterator<Item = RawSetter>) -> MarkdownResult<Self> {
        let mut planned: Vec<PlannedSetter> = Vec::new();
        for setter in settle_overlaps(setters) {
            let setter = plan_setter(setter, &planned)?;
            planned.push(setter);
        }
        Ok(Self { setters: planned })
    }

    /// As [`Self::new`], dropping what `new` would refuse and keeping the rest.
    ///
    /// A value that cannot be admitted is removed before overlaps are settled, so
    /// it does not take away an earlier valid value of the same path.
    fn tolerant(setters: impl IntoIterator<Item = RawSetter>) -> Self {
        let readable = setters.into_iter().filter(|setter| SetterValue::parse(&setter.raw).is_ok());
        let mut planned: Vec<PlannedSetter> = Vec::new();
        for setter in settle_overlaps(readable) {
            if let Ok(setter) = plan_setter(setter, &planned) {
                planned.push(setter);
            }
        }
        Self { setters: planned }
    }

    /// Whether the command line had no setters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.setters.is_empty()
    }

    /// The surviving arguments as written, in application order.
    pub(crate) fn arguments(&self) -> impl ExactSizeIterator<Item = &str> {
        self.setters.iter().map(|setter| setter.argument.as_str())
    }

    /// Whether a setter replaces top-level `key` whole.
    pub(crate) fn names_whole_key(&self, key: &str) -> bool {
        self.setters.iter().any(|setter| matches!(setter.path.segments(), [only] if only == key))
    }

    /// The setter whose path holds the value at `location`, the most specific
    /// when several do.
    pub(crate) fn argument_covering(&self, location: &[ValuePathSegment]) -> Option<&str> {
        let location: Vec<String> = location
            .iter()
            .map(|step| match step {
                ValuePathSegment::Key(key) => key.clone(),
                ValuePathSegment::Index(index) => index.to_string(),
            })
            .collect();
        self.covering(&location).map(|setter| setter.argument.as_str())
    }

    /// Gives a schema problem on a setter's value to that setter: the message
    /// names it, and the position goes, because the value was never at the on-disk
    /// scalar the position would point to.
    pub(crate) fn attribute_problem(&self, problem: &mut simplified_schema::ValidationProblem) {
        // A required or unknown-key problem points at the parent, and names its
        // children separately. One naming none is about the value at its path.
        let parent = problem.instance_path.segments();
        let children: Vec<&String> = problem.property.iter().chain(&problem.offending_properties).collect();
        let covering = if children.is_empty() {
            self.covering(parent)
        } else {
            children.iter().find_map(|child| self.covering(&[parent, std::slice::from_ref(*child)].concat()))
        };
        if let Some(setter) = covering {
            problem.message = format!("{} (from the setter `{}` at `{}`)", problem.message, setter.argument, setter.path);
            problem.line = None;
            problem.column = None;
        }
    }

    fn covering(&self, location: &[String]) -> Option<&PlannedSetter> {
        self.setters.iter().rev().find(|setter| location.starts_with(setter.path.segments()))
    }

    /// What caller records see before any document has bound the plan: the
    /// top-level keys it replaces whole. Only the keys matter until binding, when
    /// schema validators learn which properties came from the caller.
    pub(crate) fn provisional_bindings(&self) -> SetterBindings {
        let whole_keys = self
            .setters
            .iter()
            .filter_map(|setter| match setter.path.segments() {
                [only] => Some((only.clone(), setter.value.bind(None))),
                _ => None,
            })
            .collect();
        SetterBindings {
            whole_keys: Some(Value::Object(whole_keys)),
            nested: Vec::new(),
        }
    }
}

/// One entry per decoded path, the last value at the first position, shortest
/// paths first.
fn settle_overlaps(setters: impl IntoIterator<Item = RawSetter>) -> Vec<RawSetter> {
    let mut surviving: Vec<RawSetter> = Vec::new();
    for setter in setters {
        match surviving.iter_mut().find(|kept| kept.path == setter.path) {
            Some(kept) => *kept = setter,
            None => surviving.push(setter),
        }
    }
    surviving.sort_by_key(|setter| setter.path.segments().len());
    surviving
}

/// Reads one surviving value, given the shorter setters already planned.
fn plan_setter(setter: RawSetter, planned: &[PlannedSetter]) -> MarkdownResult<PlannedSetter> {
    let value = SetterValue::parse(&setter.raw).map_err(|error| setter_error(&setter.argument, &setter.path, error.to_string()))?;
    if let Some(ancestor) = planned.iter().find(|ancestor| {
        is_proper_prefix(ancestor.path.segments(), setter.path.segments()) && !matches!(ancestor.value, SetterValue::Container(_))
    }) {
        let reason = format!("`{}` replaces `{}` with a value that cannot hold it", ancestor.argument, ancestor.path);
        return Err(setter_error(&setter.argument, &setter.path, reason));
    }
    Ok(PlannedSetter {
        argument: setter.argument,
        path: setter.path,
        value,
    })
}

/// Writes a setter's unbound guess: the value before any schema has been chosen.
fn apply_guess(target: &mut Markdown, setter: &PlannedSetter) -> Result<(), PathEditError> {
    set_at(target.frontmatter_mut().as_map_mut(), &setter.path, setter.value.bind(None))
}

fn is_proper_prefix(ancestor: &[String], descendant: &[String]) -> bool {
    ancestor.len() < descendant.len() && descendant.starts_with(ancestor)
}

fn setter_error(argument: &str, path: &PropertyPath, reason: String) -> MarkdownError {
    MarkdownError::Setter {
        argument: argument.to_string(),
        path: path.to_string(),
        reason,
    }
}

/// What a plan wrote into one document, as caller input.
///
/// A record covers exactly what the caller supplied, so an authored value beside
/// a nested setter keeps resolving from its own document.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SetterBindings {
    /// Top-level keys the caller supplied whole, as an override object: a setter
    /// naming the key, or a nested setter into a key `--set` supplied.
    pub(crate) whole_keys: Option<Value>,
    /// The outermost nested setters into document-authored values.
    pub(crate) nested: Vec<NestedSetterRecord>,
}

/// One nested setter's bound value and where it landed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NestedSetterRecord {
    /// The argument as written, naming the setter in a failure.
    pub(crate) argument: String,
    /// The typed path from the frontmatter root.
    pub(crate) path: Vec<ValuePathSegment>,
    /// The value as bound.
    pub(crate) raw: Value,
}

/// Applies the plan to `markdown`, reading each value through this document's
/// schema, and updates `provenance` for what changed.
///
/// A provisional copy with every value guessed selects the schema, since a setter
/// can decide which trigger applies. Each value is then read once through the
/// type that schema declares; there is no second selection, and the schema stage
/// judges the result. A key a data override supplied keeps it: data overrides
/// come from an operation and outrank what a person typed.
///
/// ## Errors
///
/// [`MarkdownError::Setter`] when a path cannot be applied to this document.
pub(crate) fn apply_setter_plan(
    markdown: &mut Markdown,
    options: &crate::markdown::compose::ComposeRequest,
    prepared: &PreparedSchemas,
    provenance: &mut FrontmatterProvenance,
) -> MarkdownResult<SetterBindings> {
    let data_overrides = options.data_overrides.as_ref().and_then(Value::as_object);
    let active: Vec<&PlannedSetter> = options
        .setters
        .setters
        .iter()
        .filter(|setter| !data_overrides.is_some_and(|overrides| overrides.contains_key(&setter.path.segments()[0])))
        .collect();
    if active.is_empty() {
        return Ok(SetterBindings::default());
    }
    let located_failure = |setter: &PlannedSetter, error: PathEditError| {
        let reason = match document_name(markdown) {
            Some(document) => format!("{error} in `{document}`"),
            None => error.to_string(),
        };
        setter_error(&setter.argument, &setter.path, reason)
    };

    let mut overlay = markdown.clone();
    for setter in &active {
        apply_guess(&mut overlay, setter).map_err(|error| located_failure(setter, error))?;
    }
    let effective = prepared.effective_for(&overlay);
    let overlay_json = frontmatter_as_json(&overlay);
    let bound: Vec<Value> = active
        .iter()
        .map(|setter| {
            let walked = setter.path.resolve_map(overlay.frontmatter().as_map());
            let declared = effective
                .as_ref()
                .and_then(|effective| declared_shape(effective, &overlay_json, walked.steps(), Some(&setter.value)));
            setter.value.bind(declared.as_ref())
        })
        .collect();

    let mut applied = markdown.clone();
    for (setter, value) in active.iter().zip(bound) {
        set_at(applied.frontmatter_mut().as_map_mut(), &setter.path, value).map_err(|error| located_failure(setter, error))?;
    }
    *markdown = applied;

    let frontmatter = markdown.frontmatter().as_map();
    let as_json = Value::Object(frontmatter.iter().map(|(key, value)| (key.clone(), value.clone())).collect());
    for setter in &active {
        let path = typed_path(setter, frontmatter);
        match path.as_slice() {
            [ValuePathSegment::Key(key)] => {
                provenance.data_mut().clear_key(key);
                provenance.set_authored_source(key, frontmatter.get(key));
            }
            nested => provenance.data_mut().clear_path(nested, &as_json),
        }
    }
    Ok(bindings_of(&active, frontmatter, options))
}

/// Applies the plan on a pre-flight walk's copy of a document, preparing the
/// schemas that walk has not needed yet.
pub(crate) fn apply_setter_plan_for_preflight(
    markdown: &mut Markdown,
    options: &crate::markdown::compose::ComposeRequest,
    provenance: &mut FrontmatterProvenance,
) -> MarkdownResult<()> {
    if options.setters.is_empty() {
        return Ok(());
    }
    let mut trigger_registry = None;
    let prepared = prepare_schemas(markdown, options, &mut trigger_registry)?;
    apply_setter_plan(markdown, options, &prepared, provenance).map(drop)
}

/// A document as shell completion should read it, with the words before the cursor
/// applied.
#[derive(Debug, Clone)]
pub struct CompletionOverlay {
    /// A temporary copy of the document. Select its schema from this copy, too.
    pub markdown: Markdown,
    /// The setters that took effect, which completion should not offer again.
    pub paths: Vec<PropertyPath>,
}

/// Builds the copy of `markdown` that completion reads: `--state` defaults under
/// the document, `--set` over it, then `setters` through the same ordered plan
/// composition uses, each value read as an unbound guess.
///
/// Completion is advisory, so one setter that composition would refuse is dropped
/// here and the others still apply. Nothing is evaluated.
#[must_use]
pub fn completion_overlay(
    markdown: &Markdown,
    defaults: Option<&Value>,
    overrides: Option<&Value>,
    setters: impl IntoIterator<Item = RawSetter>,
) -> CompletionOverlay {
    let mut overlay = markdown.clone();
    if let Some(defaults) = defaults {
        let frontmatter = overlay.frontmatter_mut().as_map_mut();
        let current = Value::Object(frontmatter.iter().map(|(key, value)| (key.clone(), value.clone())).collect());
        if let Value::Object(merged) = deep_merge(&current, defaults, ConflictWinner::Base) {
            *frontmatter = merged.into_iter().collect();
        }
    }
    if let Some(overrides) = overrides.and_then(Value::as_object) {
        let frontmatter = overlay.frontmatter_mut().as_map_mut();
        for (key, value) in overrides {
            frontmatter.insert(key.clone(), value.clone());
        }
    }
    let plan = SetterPlan::tolerant(setters);
    let paths = plan
        .setters
        .iter()
        .filter(|setter| apply_guess(&mut overlay, setter).is_ok())
        .map(|setter| setter.path.clone())
        .collect();
    CompletionOverlay { markdown: overlay, paths }
}

fn document_name(markdown: &Markdown) -> Option<String> {
    use crate::markdown::compose::ComposeSource;
    match markdown.source() {
        Some(ComposeSource::File(path)) => Some(path.display().to_string()),
        _ => None,
    }
}

fn typed_path(setter: &PlannedSetter, frontmatter: &indexmap::IndexMap<String, Value>) -> Vec<ValuePathSegment> {
    setter
        .path
        .resolve_map(frontmatter)
        .steps()
        .iter()
        .map(|step| match step {
            PathStep::Key(key) => ValuePathSegment::Key((*key).to_string()),
            PathStep::Index(index) => ValuePathSegment::Index(*index),
        })
        .collect()
}

fn bindings_of(
    active: &[&PlannedSetter],
    frontmatter: &indexmap::IndexMap<String, Value>,
    options: &crate::markdown::compose::ComposeRequest,
) -> SetterBindings {
    let set_overrides = options.set_overrides.as_ref().and_then(Value::as_object);
    let whole_keys: HashSet<&str> = active
        .iter()
        .filter_map(|setter| {
            let segments = setter.path.segments();
            let first = segments[0].as_str();
            (segments.len() == 1 || set_overrides.is_some_and(|overrides| overrides.contains_key(first))).then_some(first)
        })
        .collect();
    let values: HashMap<&str, &Value> = whole_keys
        .iter()
        .filter_map(|key| Some((*key, frontmatter.get(*key)?)))
        .collect();

    let nested = active
        .iter()
        .filter(|setter| !whole_keys.contains(setter.path.segments()[0].as_str()))
        .filter(|setter| !active.iter().any(|other| is_proper_prefix(other.path.segments(), setter.path.segments())))
        .filter_map(|setter| {
            let raw = setter.path.resolve_map(frontmatter).value()?.clone();
            Some(NestedSetterRecord {
                argument: setter.argument.clone(),
                path: typed_path(setter, frontmatter),
                raw,
            })
        })
        .collect();
    SetterBindings {
        whole_keys: Some(Value::Object(
            values.into_iter().map(|(key, value)| (key.to_string(), value.clone())).collect(),
        )),
        nested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(token: &str) -> RawSetter {
        match classify_token(token) {
            TokenClass::Setter(setter) => setter,
            other => panic!("`{token}` is not a setter: {other:?}"),
        }
    }

    fn plan(tokens: &[&str]) -> MarkdownResult<SetterPlan> {
        SetterPlan::new(tokens.iter().map(|token| raw(token)))
    }

    fn order(plan: &SetterPlan) -> Vec<&str> {
        plan.arguments().collect()
    }

    #[test]
    fn a_word_that_fits_the_grammar_is_a_setter() {
        for (token, segments, value) in [
            ("title=x", vec!["title"], "x"),
            ("_a-b=x", vec!["_a-b"], "x"),
            ("author.name=Alice", vec!["author", "name"], "Alice"),
            ("items.1.name=z", vec!["items", "1", "name"], "z"),
            ("a.b=c.md", vec!["a", "b"], "c.md"),
            ("a=b=c", vec!["a"], "b=c"),
            ("a=", vec!["a"], ""),
            (r"a\.b=1", vec!["a.b"], "1"),
            (r"dir\.config=1.md", vec!["dir.config"], "1.md"),
        ] {
            let setter = raw(token);
            assert_eq!(setter.path.segments(), segments.as_slice(), "{token}");
            assert_eq!(setter.raw, value, "{token}");
            assert_eq!(setter.argument, token);
        }
    }

    #[test]
    fn a_word_that_does_not_fit_is_a_path() {
        for token in [
            "doc.md",
            "-",
            "./foo=bar.md",
            "../foo=bar.md",
            "/abs/a=1.md",
            "9key=value",
            "@x=1.md",
            "a..b=1",
            "a.=1",
            ".a=1",
            "a:b=1",
            "1.a=1",
            r".\dir\.config=1.md",
            r"\.a=1",
            "a b=1",
            "é=1",
            r"a\\.b=1",
        ] {
            assert_eq!(classify_token(token), TokenClass::Path, "{token}");
        }
    }

    #[test]
    fn an_empty_key_is_its_own_class() {
        assert_eq!(classify_token("=value"), TokenClass::EmptyKey);
        assert_eq!(classify_token("="), TokenClass::EmptyKey);
    }

    #[test]
    fn ancestors_come_first_and_independent_paths_keep_their_order() {
        let settled = plan(&["b.x=1", "a={}", "c=2", "a.y=3"]).unwrap();
        assert_eq!(order(&settled), ["a={}", "c=2", "b.x=1", "a.y=3"]);
    }

    #[test]
    fn the_last_value_of_a_path_wins_and_decoded_paths_are_compared() {
        let settled = plan(&["a.b=1", "x=2", "a.b=3"]).unwrap();
        assert_eq!(order(&settled), ["x=2", "a.b=3"]);

        // A literal dotted key and two nested segments are different paths.
        assert_eq!(order(&plan(&[r"a\.b=1", "a.b=2"]).unwrap()).len(), 2);
    }

    #[test]
    fn an_overridden_value_is_never_read() {
        assert_eq!(order(&plan(&["a={x:1,x:2}", "a=1"]).unwrap()), ["a=1"]);
        let error = plan(&["a=1", "a={x:1,x:2}"]).unwrap_err();
        assert!(matches!(error, MarkdownError::Setter { .. }), "{error}");
        assert!(error.to_string().contains("duplicate key `x`"), "{error}");
    }

    #[test]
    fn a_scalar_or_null_ancestor_fails_in_both_orders() {
        for (ancestor, descendant) in [("a=1", "a.b=2"), ("a=null", "a.b=2"), ("a=\"t\"", "a.b=2"), ("x.y=text", "x.y.z.w=1")] {
            for tokens in [[ancestor, descendant], [descendant, ancestor]] {
                let error = plan(&tokens).unwrap_err();
                let MarkdownError::Setter { argument, path, reason } = &error else {
                    panic!("{error}");
                };
                assert_eq!(argument, descendant);
                assert_eq!(path, descendant.split_once('=').unwrap().0);
                assert!(reason.contains(ancestor), "{reason}");
            }
        }
    }

    #[test]
    fn a_replaced_scalar_ancestor_does_not_block_the_value_that_won() {
        assert!(plan(&["a=1", "a={}", "a.b=2"]).is_ok());
    }

    #[test]
    fn an_empty_plan_is_empty() {
        assert!(plan(&[]).unwrap().is_empty());
        assert!(SetterPlan::default().is_empty());
    }

    fn overlay_of(document: &str, tokens: &[&str]) -> (Value, Vec<String>) {
        let overlay = completion_overlay(&Markdown::from(document), None, None, tokens.iter().map(|token| raw(token)));
        let paths = overlay.paths.iter().map(ToString::to_string).collect();
        (frontmatter_as_json(&overlay.markdown), paths)
    }

    #[test]
    fn an_overlay_applies_the_plan_in_its_settled_order() {
        let (frontmatter, paths) = overlay_of("---\nkept: 1\n---\n", &["a.b=1", "a={c:2}", "n=1.10"]);
        assert_eq!(frontmatter, serde_json::json!({"kept": 1, "a": {"c": 2, "b": 1}, "n": 1.1}));
        assert_eq!(paths, ["a", "n", "a.b"]);
    }

    #[test]
    fn an_overlay_drops_only_the_setter_composition_would_refuse() {
        // An inadmissible value, a child of a scalar ancestor, and an index past
        // the end each fail `SetterPlan::new` or the edit, one at a time.
        let (frontmatter, paths) = overlay_of(
            "---\nitems: [a]\ntext: t\n---\n",
            &["bad={x:1,x:2}", "s=1", "s.child=2", "items.9=z", "text.deep=1", "ok=yes"],
        );
        assert_eq!(frontmatter, serde_json::json!({"items": ["a"], "text": "t", "s": 1, "ok": "yes"}));
        assert_eq!(paths, ["s", "ok"]);
    }

    #[test]
    fn an_inadmissible_repeat_keeps_the_earlier_valid_value() {
        let (frontmatter, paths) = overlay_of("", &["a=1", "a={x:1,x:2}"]);
        assert_eq!(frontmatter, serde_json::json!({"a": 1}));
        assert_eq!(paths, ["a"]);
    }

    #[test]
    fn an_overlay_layers_defaults_under_and_overrides_over_the_document() {
        let defaults = serde_json::json!({"a": 9, "deep": {"x": 1, "y": 2}, "only_default": 1});
        let overrides = serde_json::json!({"b": 8, "c": {"replaced": true}});
        let overlay = completion_overlay(
            &Markdown::from("---\na: 1\nb: 2\nc: {kept: 1}\ndeep: {y: 3}\n---\n"),
            Some(&defaults),
            Some(&overrides),
            [raw("c.more=1")],
        );
        assert_eq!(
            frontmatter_as_json(&overlay.markdown),
            serde_json::json!({
                "a": 1,
                "b": 8,
                "c": {"replaced": true, "more": 1},
                "deep": {"y": 3, "x": 1},
                "only_default": 1,
            })
        );
    }

    #[test]
    fn only_a_whole_key_setter_names_the_key() {
        let settled = plan(&["a={}", "b.c=1"]).unwrap();
        assert!(settled.names_whole_key("a"));
        assert!(!settled.names_whole_key("b"));
        assert!(!settled.names_whole_key("c"));
    }

    #[test]
    fn the_covering_setter_is_the_most_specific_and_reads_indexes() {
        let settled = plan(&["a={}", "a.b=1", "items.0=x"]).unwrap();
        let key = |name: &str| ValuePathSegment::Key(name.to_string());
        assert_eq!(settled.argument_covering(&[key("a"), key("b"), key("c")]), Some("a.b=1"));
        assert_eq!(settled.argument_covering(&[key("a"), key("z")]), Some("a={}"));
        assert_eq!(settled.argument_covering(&[key("items"), ValuePathSegment::Index(0)]), Some("items.0=x"));
        assert_eq!(settled.argument_covering(&[key("items")]), None);
        assert_eq!(settled.argument_covering(&[key("other")]), None);
    }
}
