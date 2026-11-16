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
//! `edit`'s target and flags are also judged by the parser's own rules
//! ([`edit_target_problem`](crate::lifecycle::edit_target_problem),
//! [`edit_flag_problem`](crate::lifecycle::edit_flag_problem)), which convert
//! as the schema does: a YAML number or boolean target names a file by its
//! text, and boolean-like flag text such as `"yes"` is a flag. What remains
//! refused is a blank target, `edit: null`, and a flag that is a number, a
//! list, or a template.
//!
//! A value whose spelling the type does not offer (a long form where only
//! short forms are typed, as in a bundle) and a long form whose keys select no
//! overload keep the schema's verdict, as the parser rejects them too.
//!
//! The values are found by walking the instance beside the JSON Schema, so
//! every place a host types with a generated verb (event stacks, conditional
//! branches, bundles, loop keys, container events at any depth) is covered
//! without naming any of them.
//!
//! The same walk finds every action dictionary: an object the schema types
//! with both `when` and `no_error` (a list item, a stack root, a `then`/`else`
//! body), or with `no_error` beside `while`/`until` (a `loop:` block). Each
//! is judged by the lifecycle parser's own placement and branch-shape rules
//! ([`placement_rule`]): a `no_error` beside `then`, two placements of one
//! flag, a flag on `set`, a `when` with nothing to guard, a flag that is not
//! a literal boolean (also inside the long form of a side effect or
//! expression function, which md's baseline leaves untyped), and
//! `when`/`then`/`else` in a dictionary of verbs. Whether a flag counts is a
//! question of who wrote it, so the walk reads it from the instance's
//! authored counterpart ([`FlagOrigin`]): a `no_error: false` the schema
//! completion filled is never a placement. A dictionary the schema already
//! reports a problem in keeps that verdict. A `then`/`else` body no type
//! reaches (md's baseline stops typing below its third conditional level) is
//! read as the parser reads a body, so its dictionaries get the same rules.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use super::simplified::{DARKMATTER_OPERANDS_KEYWORD, OperandForm, operands_verb};
use super::{JsonPointer, ValidationProblem, ValidationProblemCode, ValidationProblemKind};
use crate::lifecycle::actions::{is_known_expression_function_verb, is_known_side_effect};
use crate::lifecycle::{
    ActionCall, ActionMapPosition, AuthoredSource, OperandLocation, OperandVerdict, action_call, action_value_to_expr,
    edit_flag_problem, edit_target_problem, operand_form, placement_rule, template_operands,
    validate_action_operands_with,
};
use simplified_schema::coercion::PathSegment;
use crate::markdown::compose::expression::call_binding::{CallBindingError, CallErrorKind};
use crate::markdown::compose::expression::{defer_templates, is_malformed_template};
use crate::markdown::literal_token::decode_leaf;

/// How many `$ref`/combinator hops the walk follows at one instance node
/// before giving up on it: translators close every cycle through a value
/// descent, so this is only a backstop.
const MAX_HOPS: usize = 64;

/// The verdicts on every generated verb value of one instance.
#[derive(Debug, Default)]
pub(super) struct BoundOperands {
    /// The operands of every accepted call.
    pub(super) settled: Vec<Vec<PathSegment>>,
    rejected: Vec<Rejection>,
    malformed: Vec<Malformed>,
    refused_targets: Vec<RefusedTarget>,
    placements: Vec<Placement>,
}

/// Which `no_error` flags of a validated instance its author wrote.
///
/// A validated value may have been completed by the schema first (composition
/// inserts every default before its verdict), and a completed flag is
/// `false`, indistinguishable from an authored one; only the authored
/// counterpart tells them apart.
#[derive(Debug, Clone, Copy)]
pub(super) enum FlagOrigin<'a> {
    /// The instance is itself authored.
    Input,
    /// The instance's authored counterpart; a flag counts where this value
    /// holds one at the same place. `Value::Null` means nothing is known to
    /// be authored.
    Authored(&'a Value),
}

/// An action dictionary that breaks one of the lifecycle parser's placement
/// or branch-shape rules.
#[derive(Debug)]
struct Placement {
    /// The dictionary.
    map: Vec<PathSegment>,
    /// The offending value.
    at: Vec<PathSegment>,
    /// The lifecycle parser's own message.
    message: String,
    /// Whether the value has the wrong type rather than the wrong place.
    mistyped: bool,
}

/// An `edit` operand the lifecycle parser refuses: a blank target, a missing
/// one (`edit: null`), or a flag the shared conversion does not read as a
/// boolean (a number, a list, a template).
#[derive(Debug)]
struct RefusedTarget {
    at: Vec<PathSegment>,
    /// The lifecycle parser's own message.
    message: String,
    /// Whether the operand has the wrong type rather than a blank value.
    mistyped: bool,
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
    ///
    /// `origin` tells which of `instance`'s `no_error` flags were authored
    /// (see [`FlagOrigin`]).
    pub(super) fn bind(schema: &Value, instance: &Value, stored: &Value, origin: FlagOrigin<'_>) -> Self {
        let mut walker = Walker {
            root: schema,
            seen: HashSet::new(),
            patterns: HashMap::new(),
            marks: HashMap::new(),
            sites: Vec::new(),
            site_index: HashMap::new(),
            actions: Vec::new(),
            action_index: HashMap::new(),
        };
        walker.walk(schema, instance, &mut Vec::new(), 0);
        untyped_bodies(instance, &mut walker.actions);
        let mut bound = Self {
            placements: placements(instance, &walker.actions, &walker.sites, origin),
            ..Self::default()
        };
        for site in walker.sites {
            let Some(value) = value_at(instance, &site.path) else {
                continue;
            };
            let form = operand_form(&site.verb, value);
            if !site.forms.contains(&form) {
                continue;
            }
            // `edit: null` supplies no target. The schema's optional
            // wrapper admits a null value, so the parser's refusal is
            // reported here, where the two share one rule.
            if site.verb == "edit" && value.is_null() {
                bound.refused_targets.push(RefusedTarget {
                    at: site.path.clone(),
                    message: EDIT_WITHOUT_TARGET.to_string(),
                    mistyped: true,
                });
                continue;
            }
            if site.verb == "edit"
                && let Some(location) = edit_target(form, value)
            {
                let path = operand_path(&site.path, &location);
                if let Some(found) = value_at(stored, &path)
                    && let Some((message, mistyped)) = edit_target_problem(found)
                {
                    bound.refused_targets.push(RefusedTarget {
                        at: path,
                        message: message.to_string(),
                        mistyped,
                    });
                }
            }
            if site.verb == "edit"
                && form == OperandForm::Long
                && let Some(Value::Object(entries)) = value_at(stored, &site.path)
            {
                for (key, flag) in entries {
                    if let Some(message) = edit_flag_problem(key, flag) {
                        let mut at = site.path.clone();
                        at.push(PathSegment::Key(key.clone()));
                        bound.refused_targets.push(RefusedTarget { at, message, mistyped: true });
                    }
                }
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
    pub(super) fn report_malformed(&self, problems: &mut Vec<ValidationProblem>) {
        for malformed in &self.malformed {
            let path = pointer(&malformed.at);
            problems.retain(|problem| !within(&problem_pointer(problem), &path));
            problems.push(operand_problem(
                path,
                malformed.message.clone(),
                ValidationProblemKind::Invalid,
                ValidationProblemCode::ConstraintViolation,
            ));
        }
    }

    /// Adds the lifecycle parser's problem for each refused `edit` operand, in
    /// place of the problems already reported at or below it.
    pub(super) fn report_refused_targets(&self, problems: &mut Vec<ValidationProblem>) {
        for refused in &self.refused_targets {
            let path = pointer(&refused.at);
            problems.retain(|problem| !within(&problem_pointer(problem), &path));
            let (kind, code) = if refused.mistyped {
                (ValidationProblemKind::Type, ValidationProblemCode::TypeMismatch)
            } else {
                (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation)
            };
            problems.push(operand_problem(path, refused.message.clone(), kind, code));
        }
    }

    /// Adds the lifecycle parser's problem for each action dictionary that
    /// breaks a placement or branch-shape rule, unless `problems` already
    /// holds one inside that dictionary. Report this last: a dictionary is
    /// left to every other verdict on it.
    pub(super) fn report_placements(&self, problems: &mut Vec<ValidationProblem>) {
        let reported: Vec<String> = problems.iter().map(problem_pointer).collect();
        for placement in &self.placements {
            let map = pointer(&placement.map);
            if reported.iter().any(|path| within(path, &map)) {
                continue;
            }
            let (kind, code) = if placement.mistyped {
                (ValidationProblemKind::Type, ValidationProblemCode::TypeMismatch)
            } else {
                (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation)
            };
            problems.push(operand_problem(pointer(&placement.at), placement.message.clone(), kind, code));
        }
    }

    /// Adds a problem for each rejected call that `problems` holds none
    /// inside of. Report [`Self::report_malformed`] first: a call holding a
    /// malformed template is then covered by its syntax problem.
    pub(super) fn report_rejections(&self, problems: &mut Vec<ValidationProblem>) {
        for rejection in &self.rejected {
            let action = pointer(&rejection.action);
            if problems.iter().any(|problem| within(&problem_pointer(problem), &action)) {
                continue;
            }
            let (kind, code) = match rejection.error.kind() {
                CallErrorKind::Type => (ValidationProblemKind::Type, ValidationProblemCode::TypeMismatch),
                _ => (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation),
            };
            problems.push(operand_problem(
                pointer(&rejection.at),
                rejection.error.to_string(),
                kind,
                code,
            ));
        }
    }
}

/// The rule each of `actions` (action dictionaries of `instance`) breaks.
///
/// A verb's long form is read only where its host parses it: a verb the
/// schema types (one of `sites`), or a side effect or expression function,
/// which every host provides. md leaves a verb it does not provide untyped
/// and its parser never reads its value, so its inner flag breaks no rule.
fn placements(instance: &Value, actions: &[ActionMap], sites: &[Site], origin: FlagOrigin<'_>) -> Vec<Placement> {
    let mut found = Vec::new();
    for ActionMap { path, position, verbs, untyped } in actions {
        let Some(Value::Object(map)) = value_at(instance, path) else {
            continue;
        };
        let authored = match origin {
            FlagOrigin::Input => AuthoredSource::Input,
            FlagOrigin::Authored(authored) => authored_at(authored, path),
        };
        let provided = |verb: &str| {
            is_known_side_effect(verb)
                || is_known_expression_function_verb(verb)
                || (*untyped && verbs.contains(verb))
                || sites.iter().any(|site| {
                    !site.forms.is_empty()
                        && site.path.len() == path.len() + 1
                        && site.path.starts_with(path)
                        && site.path.last() == Some(&PathSegment::Key(verb.to_string()))
                })
        };
        if let Some(rule) = placement_rule(map, *position, authored, &provided) {
            let mut at = path.clone();
            at.extend(rule.at.into_iter().map(PathSegment::Key));
            found.push(Placement {
                map: path.clone(),
                at,
                message: rule.message,
                mistyped: rule.mistyped,
            });
        }
    }
    found
}

/// Adds to `actions` every action dictionary inside a `then`/`else` body
/// that no type reached, as the lifecycle parser reads a body: each element
/// of a list is an item, a dictionary is a dictionary of verbs, and an item
/// that is a conditional holds bodies of its own.
///
/// md's baseline types three conditional levels and leaves the bodies below
/// them untyped, but the parser judges them with the same rules, so they get
/// the same placement checks. The conditional level is never counted here:
/// a stack nested deeper than the parser allows is left to the parser, as
/// in a recursive profile that types every level. A dictionary found here
/// provides the verbs of the conditional enclosing its body.
fn untyped_bodies(instance: &Value, actions: &mut Vec<ActionMap>) {
    let mut known: HashSet<Vec<PathSegment>> = actions.iter().map(|action| action.path.clone()).collect();
    let mut next = 0;
    while next < actions.len() {
        let (path, verbs) = (actions[next].path.clone(), actions[next].verbs.clone());
        let is_item = actions[next].position == ActionMapPosition::Item;
        next += 1;
        let Some(Value::Object(map)) = value_at(instance, &path).filter(|_| is_item) else {
            continue;
        };
        if !map.contains_key("when") || !map.contains_key("then") {
            continue;
        }
        for branch in ["then", "else"] {
            let mut body = path.clone();
            body.push(PathSegment::Key(branch.to_string()));
            let found: Vec<(Vec<PathSegment>, ActionMapPosition)> = match map.get(branch) {
                Some(Value::Array(items)) => items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.is_object())
                    .map(|(index, _)| {
                        let mut item = body.clone();
                        item.push(PathSegment::Index(index));
                        (item, ActionMapPosition::Item)
                    })
                    .collect(),
                Some(Value::Object(_)) => vec![(body, ActionMapPosition::Dictionary)],
                _ => Vec::new(),
            };
            for (path, position) in found {
                if known.insert(path.clone()) {
                    actions.push(ActionMap {
                        path,
                        position,
                        verbs: verbs.clone(),
                        untyped: true,
                    });
                }
            }
        }
    }
}

/// The authored cursor at `path` of `authored`.
fn authored_at<'v>(authored: &'v Value, path: &[PathSegment]) -> AuthoredSource<'v> {
    path.iter().fold(AuthoredSource::from_option(Some(authored)), |cursor, segment| match segment {
        PathSegment::Index(index) => cursor.at(*index),
        PathSegment::Key(key) => cursor.child(key),
    })
}

/// Where `problem` points: its path, extended by the property it names.
fn problem_pointer(problem: &ValidationProblem) -> String {
    match problem.property.as_ref().or(problem.offending_property.as_ref()) {
        Some(name) => format!("{}/{}", problem.path, name.replace('~', "~0").replace('/', "~1")),
        None => problem.path.clone(),
    }
}

/// An operand problem at `path`, unlocated like every problem before its
/// report settles.
fn operand_problem(
    path: String,
    message: String,
    kind: ValidationProblemKind,
    code: ValidationProblemCode,
) -> ValidationProblem {
    let instance_path = JsonPointer::parse(&path);
    ValidationProblem {
        path,
        message,
        kind,
        property: None,
        line: None,
        column: None,
        arm_index: None,
        description: None,
        code,
        instance_path,
        schema_path: None,
        offending_property: None,
        offending_properties: Vec::new(),
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

/// The problem for an `edit` action whose value is `null`.
const EDIT_WITHOUT_TARGET: &str = "`edit` needs a target file; `null` names none";

/// Where the target of an `edit` action value sits, when the value has the
/// shape the parser reads one target from: a scalar, a one-item list, or a
/// long form's `file`. Any other shape is an arity or shape problem the
/// schema already reports.
fn edit_target(form: OperandForm, value: &Value) -> Option<OperandLocation> {
    match (form, value) {
        (OperandForm::Short, Value::String(_) | Value::Number(_) | Value::Bool(_)) => Some(OperandLocation::Whole),
        (OperandForm::Short, Value::Array(items)) if items.len() == 1 => Some(OperandLocation::Index(0)),
        (OperandForm::Long, Value::Object(entries)) if entries.contains_key("file") => {
            Some(OperandLocation::Key("file".to_string()))
        }
        _ => None,
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
    /// [`Self::marks`] of each schema node, which depends on the node alone:
    /// a node judges every item of a long list.
    marks: HashMap<usize, Option<(String, Vec<OperandForm>)>>,
    sites: Vec<Site>,
    /// The index in `sites` of each recorded path.
    site_index: HashMap<Vec<PathSegment>, usize>,
    /// Every action dictionary, once each.
    actions: Vec<ActionMap>,
    /// The index in `actions` of each recorded path.
    action_index: HashMap<Vec<PathSegment>, usize>,
}

/// An action dictionary of the instance.
#[derive(Debug)]
struct ActionMap {
    path: Vec<PathSegment>,
    position: ActionMapPosition,
    /// The verbs its types declare by name: the verbs its host provides at
    /// this place.
    verbs: HashSet<String>,
    /// Whether no type reaches it: it lies in a `then`/`else` body the schema
    /// leaves untyped (md's `md-loose-body`), found by
    /// [`untyped_bodies`], and takes `verbs` from the conditional enclosing
    /// that body.
    untyped: bool,
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
            let known = self.site_index.get(path.as_slice()).copied();
            if known.is_some() {
                crate::note_work!(OperandSiteComparison);
            }
            match known.map(|index| &mut self.sites[index]) {
                Some(site) if site.verb == verb => {
                    for form in forms {
                        if !site.forms.contains(&form) {
                            site.forms.push(form);
                        }
                    }
                }
                // Two verbs typing one value: neither registration decides.
                Some(site) => site.forms.clear(),
                None => {
                    self.site_index.insert(path.clone(), self.sites.len());
                    self.sites.push(Site {
                        path: path.clone(),
                        verb,
                        forms,
                    });
                }
            }
            return;
        }
        if instance.is_object() {
            self.note_action_map(map, path);
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

    /// Records `path` as an action dictionary when `map` types one there. A
    /// value one of whose types is a conditional is a list item wherever it
    /// sits, since only a list item may be a conditional.
    fn note_action_map(&mut self, map: &'s Map<String, Value>, path: &[PathSegment]) {
        let Some(properties) = map.get("properties").and_then(Value::as_object) else {
            return;
        };
        let conditional = properties.contains_key("when")
            && (properties.contains_key("then")
                || map.get("patternProperties").and_then(Value::as_object).is_some_and(|patterns| {
                    patterns.keys().any(|pattern| self.matches(pattern, "then"))
                }));
        let position = if conditional {
            Some(ActionMapPosition::Item)
        } else {
            action_map_position(properties, path)
        };
        let Some(position) = position else {
            return;
        };
        let verbs = properties.keys().filter(|key| !matches!(key.as_str(), "when" | "then" | "else" | "no_error")).cloned();
        match self.action_index.get(path).map(|&index| &mut self.actions[index]) {
            Some(known) => {
                if position == ActionMapPosition::Item {
                    known.position = position;
                }
                known.verbs.extend(verbs);
            }
            None => {
                self.action_index.insert(path.to_vec(), self.actions.len());
                self.actions.push(ActionMap {
                    path: path.to_vec(),
                    position,
                    verbs: verbs.collect(),
                    untyped: false,
                });
            }
        }
    }

    /// The verb and spellings marked on `schema` itself or on the
    /// alternatives it is made of, never below a value descent.
    fn marks(&mut self, schema: &'s Value) -> Option<(String, Vec<OperandForm>)> {
        let key = std::ptr::from_ref(schema) as usize;
        if let Some(known) = self.marks.get(&key) {
            return known.clone();
        }
        crate::note_work!(OperandMarksComputation);
        let mut found: Option<(String, Vec<OperandForm>)> = None;
        let mut ambiguous = false;
        self.collect_marks(schema, 0, &mut found, &mut ambiguous);
        let marks = if ambiguous { None } else { found };
        self.marks.insert(key, marks.clone());
        marks
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

/// Where an action dictionary whose type declares `properties` sits, when it
/// is one: it declares `no_error` beside `when` (a list item, or a stack root
/// or body dictionary) or beside `while`/`until` (a `loop:` block). An element
/// of a list is a list item.
fn action_map_position(properties: &Map<String, Value>, path: &[PathSegment]) -> Option<ActionMapPosition> {
    if !properties.contains_key("no_error") {
        return None;
    }
    if properties.contains_key("when") {
        Some(match path.last() {
            Some(PathSegment::Index(_)) => ActionMapPosition::Item,
            _ => ActionMapPosition::Dictionary,
        })
    } else if properties.contains_key("while") || properties.contains_key("until") {
        Some(ActionMapPosition::Loop)
    } else {
        None
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
