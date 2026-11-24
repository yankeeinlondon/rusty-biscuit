//! Judging values an editor observed in place of whole-value templates.
//!
//! An editor that evaluated a top-level `{{ … }}` property (an approved,
//! background evaluation) holds its concrete value. Composition would judge
//! that value through the ordinary frontmatter validation stage; this
//! module judges it the same way, on a private copy, with the same
//! conversion, defaults, presence, and constraint rules, so the editor's
//! verdict cannot drift from composition's. What the editor cannot verify
//! (a file check it may not perform, a root union whose other inputs are
//! still templates) is deferred rather than guessed.

use std::collections::BTreeMap;

use serde_json::Value;
use simplified_schema::{ValidationProblem, ValidationProblemCode, scope};

use super::positions::ProblemPositions;
use super::{EffectiveSchema, PositionMap, bound_operands, scan_pending_values};

/// [`EffectiveSchema::validate_observed_frontmatter`]'s verdict, one entry
/// per replaced property.
#[derive(Debug, Clone, Default)]
pub struct ObservedValidation {
    pub properties: BTreeMap<String, ObservedVerdict>,
}

/// The verdict on one observed property.
#[derive(Debug, Clone)]
pub enum ObservedVerdict {
    /// The validator judged the property; no problems means it is valid.
    /// Every problem's instance path starts at the property (a root-level
    /// problem of a fully known candidate, which no single property owns,
    /// is given to every judged property). Problems carry no positions: the
    /// caller places them at the authored expression.
    Judged(Vec<ValidationProblem>),
    /// No verdict is published for the property, for this reason.
    Deferred(DeferReason),
}

/// Why an observed property has no verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeferReason {
    /// The effective schema does not declare the property.
    Undeclared,
    /// The property's declaration is not resolved (a pending schema slot).
    /// Never returned by [`EffectiveSchema::validate_observed_frontmatter`],
    /// which judges a resolved schema; a caller without one reports it.
    UnresolvedDeclaration,
    /// The schema's root ties properties together (a root union, `allOf`,
    /// `if`/`then`/`else`, a dependency), and another of its inputs is still
    /// an authored template or shell value.
    RootUnionUnresolved,
    /// The declaration judges a file (an eager `file`, a `match()` glob),
    /// which the observing caller may not check; with a root that ties
    /// properties together, every observed property is deferred for it.
    FileDependent,
}

/// The root keywords that make one property's verdict depend on another's.
const LINKING_KEYWORDS: &[&str] = &["anyOf", "oneOf", "allOf", "if", "then", "else", "not", "dependentRequired", "dependentSchemas"];

impl EffectiveSchema {
    /// Judges a private copy of the authored frontmatter `seed` (a JSON
    /// object) in which each key of `replacements` holds its observed value
    /// instead of its authored whole-value template. A replaced value is
    /// data: a string holding `{{ … }}` is judged as the text it is. A key
    /// not replaced keeps its authored treatment (its templates pending).
    ///
    /// The complete candidate is judged at once, so a root union and
    /// sibling constraints see what composition would. Only declared
    /// replaced properties get a verdict ([`DeferReason`] says why another
    /// does not). No file is probed or read: file checks run unverified
    /// ([`simplified_schema::validate::with_unverified_file_checks`]) and
    /// the properties they could decide are deferred. Neither input is
    /// changed.
    pub fn validate_observed_frontmatter(&self, seed: &Value, replacements: &BTreeMap<String, Value>) -> ObservedValidation {
        let mut verdicts = ObservedValidation::default();
        let declared: Vec<&String> = replacements
            .keys()
            .filter(|key| {
                let declared = self.declares_top_level_property(key);
                if !declared {
                    verdicts.properties.insert((*key).clone(), ObservedVerdict::Deferred(DeferReason::Undeclared));
                }
                declared
            })
            .collect();
        if declared.is_empty() {
            return verdicts;
        }
        let defer_all = |verdicts: &mut ObservedValidation, reason: DeferReason| {
            for key in &declared {
                verdicts.properties.insert((*key).clone(), ObservedVerdict::Deferred(reason));
            }
        };

        // The authored remainder: what is still a template is pending, as
        // composition's pre-render validation treats it.
        let mut remainder = match seed {
            Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        for key in replacements.keys() {
            remainder.remove(key);
        }
        let remainder = Value::Object(remainder);
        let linked = self.root_links_properties();
        if linked && !scan_pending_values(&remainder).is_empty() {
            defer_all(&mut verdicts, DeferReason::RootUnionUnresolved);
            return verdicts;
        }
        let file_dependent: Vec<&String> = declared.iter().copied().filter(|key| self.property_judges_files(key)).collect();
        if linked && (!file_dependent.is_empty() || self.any_other_property_judges_files(&remainder)) {
            defer_all(&mut verdicts, DeferReason::FileDependent);
            return verdicts;
        }

        let decoded = crate::markdown::literal_token::decode_literal_tokens(&remainder).ok();
        let mut candidate = match decoded {
            Some(Value::Object(map)) => map,
            _ => remainder.as_object().cloned().unwrap_or_default(),
        };
        for (key, value) in replacements {
            candidate.insert(key.clone(), value.clone());
        }
        let candidate = Value::Object(candidate);
        // Templates are found in the authored remainder only, so a replaced
        // value's `{{ … }}` text is never deferred.
        let templates = self.nested_templates(&remainder);
        let positions = PositionMap::new();
        let report = simplified_schema::validate::with_unverified_file_checks(|| {
            let prepared = self.converted_candidate(&candidate, &templates);
            self.validate_instance(
                prepared,
                &candidate,
                &candidate,
                &ProblemPositions::Built(&positions),
                &templates,
                bound_operands::FlagOrigin::Input,
            )
        });

        let mut judged: BTreeMap<&str, Vec<ValidationProblem>> = BTreeMap::new();
        for key in &declared {
            if file_dependent.contains(key) {
                verdicts.properties.insert((*key).clone(), ObservedVerdict::Deferred(DeferReason::FileDependent));
            } else {
                judged.insert(key.as_str(), Vec::new());
            }
        }
        for problem in report.problems {
            match owning_key(&problem) {
                Some(key) => {
                    if let Some(problems) = judged.get_mut(key.as_str()) {
                        problems.push(problem);
                    }
                }
                // A root-level problem of the whole candidate.
                None => {
                    for problems in judged.values_mut() {
                        problems.push(problem.clone());
                    }
                }
            }
        }
        for (key, problems) in judged {
            verdicts.properties.insert(key.to_string(), ObservedVerdict::Judged(problems));
        }
        verdicts
    }

    /// Whether the root's own keywords tie one property's verdict to
    /// another's (a root union included).
    fn root_links_properties(&self) -> bool {
        matches!(self.validators, super::CompiledValidators::RootUnion(_))
            || self.json_schema.as_object().is_some_and(|root| LINKING_KEYWORDS.iter().any(|keyword| root.contains_key(*keyword)))
    }

    /// Whether any declaration of the top-level property `name` can judge a
    /// file: it reaches the eager file format or the `match()` keyword,
    /// through any arm or reference.
    fn property_judges_files(&self, name: &str) -> bool {
        let mut declarations = Vec::new();
        property_declarations(&self.json_schema, &self.json_schema, name, 0, &mut declarations);
        let mut visited = Vec::new();
        declarations.into_iter().any(|declaration| judges_files(&self.json_schema, declaration, 0, &mut visited))
    }

    /// Whether a property of `remainder` other than the observed ones has a
    /// declaration that can judge a file.
    fn any_other_property_judges_files(&self, remainder: &Value) -> bool {
        remainder.as_object().is_some_and(|map| map.keys().any(|key| self.property_judges_files(key)))
    }
}

/// The top-level property a problem belongs to; `None` for a problem of
/// the whole instance.
fn owning_key(problem: &ValidationProblem) -> Option<String> {
    match problem.instance_path.first() {
        Some(key) => Some(key.to_string()),
        None if problem.code == ValidationProblemCode::MissingRequired => problem.property.clone(),
        None => problem.offending_property.clone(),
    }
}

/// Bounds a `$ref` chain or nesting; real schemas nest a handful of levels.
const MAX_DEPTH: usize = 32;

/// Collects every schema declaring the top-level property `name` in
/// `schema`: its `properties` entry, a matching `patternProperties` entry,
/// and those of every arm and reference, as
/// [`EffectiveSchema::declares_top_level_property`] finds them.
fn property_declarations<'a>(root: &'a Value, schema: &'a Value, name: &str, depth: usize, out: &mut Vec<&'a Value>) {
    let Some(object) = schema.as_object() else {
        return;
    };
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(declaration) = object.get("properties").and_then(Value::as_object).and_then(|properties| properties.get(name)) {
        out.push(declaration);
    }
    if let Some(patterns) = object.get("patternProperties").and_then(Value::as_object) {
        for (pattern, declaration) in patterns {
            if regex::Regex::new(pattern).is_ok_and(|re| re.is_match(name)) {
                out.push(declaration);
            }
        }
    }
    let arms = ["allOf", "anyOf", "oneOf"]
        .into_iter()
        .filter_map(|key| object.get(key).and_then(Value::as_array))
        .flatten()
        .chain(["if", "then", "else"].into_iter().filter_map(|key| object.get(key)));
    let referenced = scope::SchemaScope::new(root)
        .chain(schema, "")
        .map(|hops| hops.into_iter().skip(1).collect::<Vec<_>>())
        .unwrap_or_default();
    for arm in arms.chain(referenced) {
        property_declarations(root, arm, name, depth + 1, out);
    }
}

/// Whether `schema`, or anything nested in it or referenced from it, names
/// the eager file format or the `match()` keyword. An unresolvable
/// reference counts, since what it would reach is unknown. A reference
/// target in `visited` was already searched, so a recursive type ends there.
fn judges_files(root: &Value, schema: &Value, depth: usize, visited: &mut Vec<*const Value>) -> bool {
    if depth > MAX_DEPTH {
        return true;
    }
    match schema {
        Value::Object(object) => {
            if object.get("format").and_then(Value::as_str) == Some(simplified_schema::format::DARKMATTER_FILE_FORMAT)
                || object.contains_key(simplified_schema::file_match::DARKMATTER_MATCH_KEYWORD)
            {
                return true;
            }
            if object.contains_key("$ref") {
                match scope::SchemaScope::new(root).chain(schema, "") {
                    Ok(hops) => {
                        for target in hops.into_iter().skip(1) {
                            let identity: *const Value = target;
                            if visited.contains(&identity) {
                                continue;
                            }
                            visited.push(identity);
                            if judges_files(root, target, depth + 1, visited) {
                                return true;
                            }
                        }
                    }
                    Err(_) => return true,
                }
            }
            object
                .iter()
                .filter(|(key, _)| !matches!(key.as_str(), "$defs" | "definitions" | "description" | "title" | "examples" | "default" | "const" | "enum"))
                .any(|(_, value)| judges_files(root, value, depth + 1, visited))
        }
        Value::Array(items) => items.iter().any(|item| judges_files(root, item, depth + 1, visited)),
        _ => false,
    }
}
