//! Binding a generated lifecycle verb's operands through its runtime
//! registration during schema validation.
//!
//! A generated verb type (`number-short`, `length-long`, …) projects its
//! catalog signature into ordinary value types, and those cannot carry what
//! the runtime registration adds: `number`'s and `round`'s numeric recovery
//! (`number: [pear, 7]` binds, the fallback supplies 7), `length`'s refusal
//! of a boolean, and the binder's overload preference (an exact overload over
//! one that needs conversion, otherwise an ambiguous call). Every generated
//! arm therefore carries `x-darkmatter-operands` (SimplifiedSchema
//! `operands(<verb>-short)`), and validation binds each value it types:
//!
//! - the value is read as the lifecycle parser reads it
//!   ([`action_call`]): `null`, a positional list, one scalar, or a long
//!   form whose keys select one overload;
//! - the call is bound passively
//!   ([`validate_action_operands_with`](crate::lifecycle::validate_action_operands_with)),
//!   with every `{{ … }}` operand deferred and nothing evaluated. A stored
//!   literal token is data, not a template: its decoded text is bound and
//!   type-checked like any literal (`abs` refuses a token holding
//!   `{{ fm.count }}`, as it refuses `three`);
//! - an accepted call settles its operands: the schema's own problems on them
//!   are set aside, but not one beside them (an unknown key, a bad
//!   `no_error`), which the core still reports;
//! - a rejected call is a positioned problem at the failing operand, unless
//!   the schema already reports one inside the action value;
//! - a `{{ … }}` span that does not parse, anywhere in an operand, is a
//!   positioned syntax problem at that string, whatever its parameter's type
//!   accepts: the lifecycle parser refuses it ([`action_value_to_expr`]), so
//!   reading it as text would only hide that error. It replaces the problems
//!   already reported at or below the same string, so a number parameter
//!   reports the syntax rather than an incidental type mismatch, and it is
//!   never settled. A stored literal token is data and never scanned.
//!
//! The verbs the lifecycle parser owns itself (communication, `shell`, the
//! flow-control directives, `set`) carry the same marker on their
//! hand-written types. They have no registration to bind, so their value is
//! only scanned for malformed templates, at exactly the operands the parser
//! reads as templates ([`template_operands`]): a `set` value written as a
//! whole-value `$( … )` command, `no_error`, and `break.code` are not.
//!
//! A value whose spelling the type does not offer (a long form where only
//! short forms are typed, as in a bundle) and a long form whose keys select no
//! overload keep the schema's verdict, as the parser rejects them too.
//!
//! The values are found by walking the instance beside the JSON Schema, so
//! every place a host types with a generated verb (event stacks, conditional
//! branches, bundles, loop keys, container events at any depth) is covered
//! without naming any of them.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use super::simplified::{DARKMATTER_OPERANDS_KEYWORD, OperandForm, operands_verb};
use super::{JsonPointer, PositionMap, ValidationProblem, ValidationProblemCode, ValidationProblemKind};
use crate::lifecycle::{
    ActionCall, OperandLocation, OperandVerdict, action_call, action_value_to_expr, operand_form, template_operands,
    validate_action_operands_with,
};
use crate::markdown::coercion::PathSegment;
use crate::markdown::compose::expression::call_binding::{CallBindingError, CallErrorKind};
use crate::markdown::compose::expression::{defer_templates, is_malformed_template};
use crate::markdown::literal_token::decode_leaf;

/// How many `$ref`/combinator hops the walk follows at one instance node
/// before giving up on it: translators close every cycle through a value
/// descent, so this is only a backstop.
const MAX_HOPS: usize = 64;

/// Whether some node of `schema` carries `x-darkmatter-operands`.
pub(super) fn marks_operands(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => map.contains_key(DARKMATTER_OPERANDS_KEYWORD) || map.values().any(marks_operands),
        Value::Array(items) => items.iter().any(marks_operands),
        _ => false,
    }
}

/// The verdicts on every generated verb value of one instance.
#[derive(Debug, Default)]
pub(super) struct BoundOperands {
    /// The operands of every accepted call.
    pub(super) settled: Vec<Vec<PathSegment>>,
    rejected: Vec<Rejection>,
    malformed: Vec<Malformed>,
}

/// An operand string holding a `{{ … }}` span that does not parse.
#[derive(Debug)]
struct Malformed {
    at: Vec<PathSegment>,
    /// The lifecycle parser's own message for the string.
    message: String,
}

/// A call its registration rejects.
#[derive(Debug)]
struct Rejection {
    /// The action value.
    action: Vec<PathSegment>,
    /// Where the problem is reported: the one failing operand, else the
    /// action value.
    at: Vec<PathSegment>,
    error: CallBindingError,
}

impl BoundOperands {
    /// Binds every value of `instance` that `schema` (the whole JSON Schema,
    /// which `$ref`s resolve in) types as a generated verb's operands.
    ///
    /// `stored` is the same document as loaded, before its literal tokens
    /// were decoded into `instance`. A token's decoded text is data: it is
    /// never deferred as a template, so the registration judges it as the
    /// text it is, and it is never scanned for malformed templates.
    pub(super) fn bind(schema: &Value, instance: &Value, stored: &Value) -> Self {
        let mut walker = Walker {
            root: schema,
            seen: HashSet::new(),
            patterns: HashMap::new(),
            sites: Vec::new(),
        };
        walker.walk(schema, instance, &mut Vec::new(), 0);
        let mut bound = Self::default();
        for site in walker.sites {
            let Some(value) = value_at(instance, &site.path) else {
                continue;
            };
            let form = operand_form(&site.verb, value);
            if !site.forms.contains(&form) {
                continue;
            }
            if let Some(operands) = template_operands(&site.verb, form, value) {
                for operand in operands {
                    let mut path = operand_path(&site.path, &operand.location);
                    if let Some(found) = value_at(stored, &path)
                        && (operand.nested || found.is_string())
                    {
                        find_malformed(found, &mut path, &mut bound.malformed);
                    }
                }
                continue;
            }
            let Some(call) = action_call(&site.verb, form, value) else {
                continue;
            };
            let found = bound.malformed.len();
            for location in &call.locations {
                let mut path = operand_path(&site.path, location);
                if let Some(operand) = value_at(stored, &path) {
                    find_malformed(operand, &mut path, &mut bound.malformed);
                }
            }
            let malformed = &bound.malformed[found..];
            let mut data = HashSet::new();
            for (arg, location) in call.args.iter().zip(&call.locations) {
                if let Some(operand) = value_at(stored, &operand_path(&site.path, location)) {
                    collect_stored_data(arg, operand, &mut data);
                }
            }
            let defer = |path: &[PathSegment], value: &Value| {
                !data.contains(&std::ptr::from_ref(value)) && defer_templates(path, value)
            };
            match validate_action_operands_with(&site.verb, &call, &defer) {
                Some(OperandVerdict::Bound { .. } | OperandVerdict::Deferred { .. }) => {
                    bound.settled.extend(
                        call.locations
                            .iter()
                            .map(|location| operand_path(&site.path, location))
                            .filter(|path| !malformed.iter().any(|found| found.at.starts_with(path))),
                    );
                }
                Some(OperandVerdict::Rejected(error)) => bound.rejected.push(Rejection {
                    at: failing_operand(&error, &call)
                        .map_or_else(|| site.path.clone(), |location| operand_path(&site.path, location)),
                    action: site.path,
                    error,
                }),
                None => {}
            }
        }
        bound
    }

    /// Adds a syntax problem for each malformed operand template, in place of
    /// the problems already reported at or below its string.
    pub(super) fn report_malformed(&self, problems: &mut Vec<ValidationProblem>, positions: &PositionMap) {
        for malformed in &self.malformed {
            let path = pointer(&malformed.at);
            problems.retain(|problem| !within(&problem_pointer(problem), &path));
            problems.push(positioned_problem(
                path,
                malformed.message.clone(),
                ValidationProblemKind::Invalid,
                ValidationProblemCode::ConstraintViolation,
                positions,
            ));
        }
    }

    /// Adds a problem for each rejected call that `problems` holds none
    /// inside of. Report [`Self::report_malformed`] first: a call holding a
    /// malformed template is then covered by its syntax problem.
    pub(super) fn report_rejections(&self, problems: &mut Vec<ValidationProblem>, positions: &PositionMap) {
        for rejection in &self.rejected {
            let action = pointer(&rejection.action);
            if problems.iter().any(|problem| within(&problem_pointer(problem), &action)) {
                continue;
            }
            let (kind, code) = match rejection.error.kind() {
                CallErrorKind::Type => (ValidationProblemKind::Type, ValidationProblemCode::TypeMismatch),
                _ => (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation),
            };
            problems.push(positioned_problem(
                pointer(&rejection.at),
                rejection.error.to_string(),
                kind,
                code,
                positions,
            ));
        }
    }
}

/// Where `problem` points: its path, extended by the property it names.
fn problem_pointer(problem: &ValidationProblem) -> String {
    match problem.property.as_ref().or(problem.offending_property.as_ref()) {
        Some(name) => format!("{}/{}", problem.path, name.replace('~', "~0").replace('/', "~1")),
        None => problem.path.clone(),
    }
}

fn positioned_problem(
    path: String,
    message: String,
    kind: ValidationProblemKind,
    code: ValidationProblemCode,
    positions: &PositionMap,
) -> ValidationProblem {
    let instance_path = JsonPointer::parse(&path);
    let (line, column) = positions
        .locate(instance_path.segments(), super::positions::PositionTarget::Value)
        .map_or((None, None), |(line, column)| (Some(line), Some(column)));
    ValidationProblem {
        path,
        message,
        kind,
        property: None,
        line,
        column,
        arm_index: None,
        description: None,
        code,
        instance_path,
        schema_path: None,
        offending_property: None,
        file_reference: None,
        caller_file: None,
    }
}

/// Records the address of every string in `arg` (an operand of the call
/// being bound) whose counterpart in `stored`, the same operand as loaded, is
/// a literal token. A stored leaf with no counterpart is skipped.
fn collect_stored_data(arg: &Value, stored: &Value, data: &mut HashSet<*const Value>) {
    match (arg, stored) {
        (Value::String(_), Value::String(text)) => {
            if matches!(decode_leaf(text), Some(Ok(_))) {
                data.insert(std::ptr::from_ref(arg));
            }
        }
        (Value::Array(items), Value::Array(stored)) => {
            for (item, stored) in items.iter().zip(stored) {
                collect_stored_data(item, stored, data);
            }
        }
        (Value::Object(entries), Value::Object(stored)) => {
            for (key, item) in entries {
                if let Some(stored) = stored.get(key) {
                    collect_stored_data(item, stored, data);
                }
            }
        }
        _ => {}
    }
}

/// Records every string at or below `value` (at `path`) that holds a
/// malformed template, with the message the lifecycle parser gives it.
fn find_malformed(value: &Value, path: &mut Vec<PathSegment>, found: &mut Vec<Malformed>) {
    match value {
        Value::String(text) => {
            if matches!(decode_leaf(text), Some(Ok(_))) || !is_malformed_template(text) {
                return;
            }
            if let Err(error) = action_value_to_expr(value) {
                found.push(Malformed {
                    at: path.clone(),
                    message: format!("malformed template `{}`: {error}", text.trim()),
                });
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(PathSegment::Index(index));
                find_malformed(item, path, found);
                path.pop();
            }
        }
        Value::Object(entries) => {
            for (key, item) in entries {
                path.push(PathSegment::Key(key.clone()));
                find_malformed(item, path, found);
                path.pop();
            }
        }
        _ => {}
    }
}

/// A value the schema types as a generated verb's operands, and the
/// spellings its type offers there.
#[derive(Debug)]
struct Site {
    path: Vec<PathSegment>,
    verb: String,
    forms: Vec<OperandForm>,
}

struct Walker<'s> {
    root: &'s Value,
    /// Each (schema node, instance pointer) pair already walked.
    seen: HashSet<(usize, String)>,
    /// Each `patternProperties` pattern, compiled once; `None` when it does
    /// not compile, so it matches no key here (the validator reports it).
    patterns: HashMap<&'s str, Option<fancy_regex::Regex>>,
    sites: Vec<Site>,
}

impl<'s> Walker<'s> {
    /// Walks `instance` (at `path`) beside every schema that may judge it,
    /// recording each value a generated verb type marks.
    fn walk(&mut self, schema: &'s Value, instance: &Value, path: &mut Vec<PathSegment>, hops: usize) {
        let Value::Object(map) = schema else {
            return;
        };
        if hops > MAX_HOPS || !self.seen.insert((std::ptr::from_ref(schema) as usize, pointer(path))) {
            return;
        }
        if let Some((verb, forms)) = self.marks(schema) {
            match self.sites.iter_mut().find(|site| {
                crate::note_work!(OperandSiteComparison);
                site.path == *path
            }) {
                Some(site) if site.verb == verb => {
                    for form in forms {
                        if !site.forms.contains(&form) {
                            site.forms.push(form);
                        }
                    }
                }
                // Two verbs typing one value: neither registration decides.
                Some(site) => site.forms.clear(),
                None => self.sites.push(Site {
                    path: path.clone(),
                    verb,
                    forms,
                }),
            }
            return;
        }
        for arm in applicators(map) {
            self.walk(arm, instance, path, hops + 1);
        }
        if let Some(target) = self.reference(map) {
            self.walk(target, instance, path, hops + 1);
        }
        match instance {
            Value::Object(entries) => {
                for (key, value) in entries {
                    let mut declared = false;
                    let mut children: Vec<&'s Value> = Vec::new();
                    if let Some(property) = map.get("properties").and_then(|properties| properties.get(key)) {
                        declared = true;
                        children.push(property);
                    }
                    if let Some(Value::Object(patterns)) = map.get("patternProperties") {
                        for (pattern, child) in patterns {
                            if self.matches(pattern, key) {
                                declared = true;
                                children.push(child);
                            }
                        }
                    }
                    if !declared && let Some(additional) = map.get("additionalProperties") {
                        children.push(additional);
                    }
                    path.push(PathSegment::Key(key.clone()));
                    for child in children {
                        self.walk(child, value, path, 0);
                    }
                    path.pop();
                }
            }
            Value::Array(items) => {
                let prefix = map.get("prefixItems").and_then(Value::as_array);
                for (index, item) in items.iter().enumerate() {
                    let child = prefix.and_then(|prefix| prefix.get(index)).or_else(|| map.get("items"));
                    if let Some(child) = child {
                        path.push(PathSegment::Index(index));
                        self.walk(child, item, path, 0);
                        path.pop();
                    }
                }
            }
            _ => {}
        }
    }

    /// The verb and spellings marked on `schema` itself or on the
    /// alternatives it is made of, never below a value descent.
    fn marks(&self, schema: &'s Value) -> Option<(String, Vec<OperandForm>)> {
        crate::note_work!(OperandMarksComputation);
        let mut found: Option<(String, Vec<OperandForm>)> = None;
        let mut ambiguous = false;
        self.collect_marks(schema, 0, &mut found, &mut ambiguous);
        if ambiguous { None } else { found }
    }

    fn collect_marks(
        &self,
        schema: &'s Value,
        hops: usize,
        found: &mut Option<(String, Vec<OperandForm>)>,
        ambiguous: &mut bool,
    ) {
        let Value::Object(map) = schema else {
            return;
        };
        if hops > MAX_HOPS {
            return;
        }
        crate::note_work!(OperandMarksNodeVisit);
        if let Some((verb, form)) = map.get(DARKMATTER_OPERANDS_KEYWORD).and_then(Value::as_str).and_then(operands_verb) {
            match found {
                Some((known, forms)) if known == verb => {
                    if !forms.contains(&form) {
                        forms.push(form);
                    }
                }
                Some(_) => *ambiguous = true,
                None => *found = Some((verb.to_string(), vec![form])),
            }
        }
        for arm in applicators(map) {
            self.collect_marks(arm, hops + 1, found, ambiguous);
        }
        if let Some(target) = self.reference(map) {
            self.collect_marks(target, hops + 1, found, ambiguous);
        }
    }

    /// The local target of a `$ref`; `None` for none, or one that does not
    /// resolve in the root schema.
    fn reference(&self, map: &'s Map<String, Value>) -> Option<&'s Value> {
        let reference = map.get("$ref")?.as_str()?;
        let fragment = reference.strip_prefix('#')?;
        self.root.pointer(fragment)
    }

    fn matches(&mut self, pattern: &'s str, key: &str) -> bool {
        self.patterns
            .entry(pattern)
            .or_insert_with(|| fancy_regex::Regex::new(pattern).ok())
            .as_ref()
            .is_some_and(|regex| regex.is_match(key).unwrap_or(false))
    }
}

/// The subschemas that judge the same instance as `map` itself.
fn applicators(map: &Map<String, Value>) -> impl Iterator<Item = &Value> {
    ["anyOf", "oneOf", "allOf"]
        .into_iter()
        .filter_map(|keyword| map.get(keyword).and_then(Value::as_array))
        .flatten()
        .chain(["then", "else"].into_iter().filter_map(|keyword| map.get(keyword)))
}

/// The one operand every argument error names, when they name only one.
fn failing_operand<'c>(error: &CallBindingError, call: &'c ActionCall) -> Option<&'c OperandLocation> {
    let mut indexes = error.argument_errors().map(|argument| argument.index);
    let first = indexes.next()?;
    if indexes.all(|index| index == first) {
        call.locations.get(first)
    } else {
        None
    }
}

fn operand_path(action: &[PathSegment], location: &OperandLocation) -> Vec<PathSegment> {
    let mut path = action.to_vec();
    match location {
        OperandLocation::Whole => {}
        OperandLocation::Index(index) => path.push(PathSegment::Index(*index)),
        OperandLocation::Key(key) => path.push(PathSegment::Key(key.clone())),
    }
    path
}

fn value_at<'v>(instance: &'v Value, path: &[PathSegment]) -> Option<&'v Value> {
    path.iter().try_fold(instance, |value, segment| match segment {
        PathSegment::Index(index) => value.get(index),
        PathSegment::Key(key) => value.get(key),
    })
}

fn pointer(path: &[PathSegment]) -> String {
    path.iter()
        .map(|segment| match segment {
            PathSegment::Index(index) => format!("/{index}"),
            PathSegment::Key(key) => format!("/{}", key.replace('~', "~0").replace('/', "~1")),
        })
        .collect()
}

/// Whether the JSON Pointer `path` is `ancestor` or lies below it.
fn within(path: &str, ancestor: &str) -> bool {
    path.strip_prefix(ancestor)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}
