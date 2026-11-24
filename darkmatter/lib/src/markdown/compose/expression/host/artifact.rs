//! The host facts artifact: the JSON companion of a host function catalog
//! that carries the Rust-side registration facts to a process that cannot
//! run the host's registration (the editor).
//!
//! The writer ([`HostFunctions::facts_artifact`]) emits deterministic bytes
//! (keys sorted at every level, pretty-printed, one trailing newline). The
//! reader ([`HostFunctions::analysis_only`]) treats the file as inert data
//! and rebuilds every registration through the same [`RegistrationBuilder`]
//! from the catalog's signatures. The catalog's `narrows(...)` clause is the
//! authority for the narrowing fact; the artifact's `facts` entry is derived
//! output, and a recorded fact that differs from the catalog's (including a
//! missing or extra one) makes the function incomplete. It is strict: a duplicate key, trailing
//! content, an unknown field, or a field that is absent, `null`, or of the
//! wrong type is never defaulted. A problem with the whole file makes every
//! catalog function incomplete; a problem with one function's entry makes
//! that function incomplete.
//!
//! ```json
//! {
//!   "catalog_hash": "<BLAKE3 hex of the catalog's bytes>",
//!   "format_version": 1,
//!   "functions": {
//!     "is_project_file": {
//!       "aliases": [],
//!       "conversion_recovery": null,
//!       "facts": [{ "kind": "true_implies", "parameter": "value", "position": 0, "type": "file" }],
//!       "lazy": false,
//!       "malformed_reference": null,
//!       "source_exclusions": []
//!     }
//!   }
//! }
//! ```

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::super::functions::is_builtin_name;
use super::{FunctionState, HostFunction, HostFunctionCatalog, HostFunctionError, HostFunctions, validate_names};
use simplified_schema::coercion::ValueKind;
use crate::markdown::compose::expression::call_binding::{
    EvaluationMode, FactKind, FactType, FunctionRegistration, RegistrationBuilder,
};

/// The artifact format this library writes and reads.
pub const FACTS_ARTIFACT_FORMAT_VERSION: u64 = 1;

/// A fact's `type` for [`FactType::NonNull`]; no type text has this form.
pub const NON_NULL_FACT_TYPE: &str = "<non-null>";

const TOP_FIELDS: &[&str] = &["catalog_hash", "format_version", "functions"];
const FUNCTION_FIELDS: &[&str] = &[
    "aliases",
    "conversion_recovery",
    "facts",
    "lazy",
    "malformed_reference",
    "source_exclusions",
];
const FACT_FIELDS: &[&str] = &["kind", "parameter", "position", "type"];
const RECOVERY_FIELDS: &[&str] = &["fallback", "subject"];
const EXCLUSION_FIELDS: &[&str] = &["kinds", "parameter"];
const REFERENCE_FIELDS: &[&str] = &["parameter", "position"];

/// The artifact beside `catalog`: its file stem plus `.facts.json`
/// (`functions.yaml` → `functions.facts.json`).
pub fn facts_artifact_path(catalog: &Path) -> PathBuf {
    catalog.with_extension("facts.json")
}

/// A companion artifact (the facts artifact, or a host lifecycle artifact
/// ([`crate::lifecycle::host_profile`])) as the reader received it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactsArtifact<'a> {
    /// The file's text.
    Text(&'a str),
    /// No file beside the catalog.
    Absent,
    /// The file exists but could not be read; the reason.
    Unreadable(&'a str),
}

impl HostFunctions {
    /// The facts artifact for this set, from its registrations: per function
    /// its aliases, narrowing facts, laziness, conversion recovery, source
    /// exclusions, and malformed-reference rule, plus the catalog's version
    /// and [`FACTS_ARTIFACT_FORMAT_VERSION`]. Incomplete functions are
    /// omitted. The bytes depend only on the registrations and the catalog.
    pub fn facts_artifact(&self) -> String {
        let mut functions = Map::new();
        for function in &self.inner.functions {
            if let FunctionState::Complete { registration, .. } = &function.state {
                functions.insert(function.canonical.clone(), function_entry(function, registration));
            }
        }
        let root = json!({
            "catalog_hash": self.inner.catalog.version(),
            "format_version": FACTS_ARTIFACT_FORMAT_VERSION,
            "functions": Value::Object(functions),
        });
        let mut text = serde_json::to_string_pretty(&sorted(root)).expect("a JSON value serializes");
        text.push('\n');
        text
    }

    /// An analysis-only set (no handlers) for `catalog`, with each function's
    /// facts read from `artifact`. A function whose facts cannot be trusted
    /// is incomplete, with a human-readable reason; see the module docs.
    ///
    /// ## Errors
    ///
    /// [`HostFunctionError::BuiltinCollision`] when the catalog declares a
    /// function a built-in already names; the artifact is not consulted.
    pub fn analysis_only(catalog: HostFunctionCatalog, artifact: FactsArtifact<'_>) -> Result<Self, HostFunctionError> {
        let path = facts_artifact_path(catalog.path());
        Self::analysis_only_at(catalog, &path, artifact)
    }

    /// [`Self::analysis_only`] for a facts artifact that a caller located
    /// itself (a host registration manifest names it) rather than beside the
    /// catalog: `facts_path` is where `artifact` was read, and reasons name
    /// it. The artifact must still record the catalog's hash.
    ///
    /// ## Errors
    ///
    /// As [`Self::analysis_only`].
    pub fn analysis_only_at(
        catalog: HostFunctionCatalog,
        facts_path: &Path,
        artifact: FactsArtifact<'_>,
    ) -> Result<Self, HostFunctionError> {
        analysis_only_sited(catalog, facts_path, artifact).map(|(functions, _)| functions)
    }
}

/// [`HostFunctions::analysis_only_at`] plus where in the artifact each
/// incomplete function's reason is, by canonical name, as the reader judged
/// it (never recovered from the reason's text). A function the map does not
/// name is incomplete for the file as a whole (absent or unreadable), which
/// has no position inside it.
pub(crate) fn analysis_only_sited(
    catalog: HostFunctionCatalog,
    facts_path: &Path,
    artifact: FactsArtifact<'_>,
) -> Result<(HostFunctions, HashMap<String, ReasonSite>), HostFunctionError> {
    let names = catalog.function_names();
    validate_names(names.iter().map(|name| (*name, &[][..])))?;
    let file = format!("the facts artifact `{}`", facts_path.display());
    let mut sites = HashMap::new();
    let mut fail = |name: &str, fault: Fault| {
        if let Some(site) = fault.site {
            sites.insert(name.to_string(), site);
        }
        incomplete(name, fault.reason)
    };
    let functions: Vec<HostFunction> = match read_entries(&catalog, &names, artifact) {
        Err(fault) => {
            let fault = fault.prefixed(&file);
            names.iter().map(|name| fail(name, fault.clone())).collect()
        }
        Ok(entries) => names
            .iter()
            .map(|name| match entries.get(*name) {
                None => fail(name, Pointer::root().join("functions").fault("the facts artifact has no entry for it")),
                Some(entry) => match rebuild(&catalog, name, entry) {
                    Ok((aliases, registration)) => HostFunction {
                        canonical: (*name).to_string(),
                        aliases,
                        state: FunctionState::Complete {
                            registration: Box::new(registration),
                            handler: None,
                        },
                    },
                    Err(fault) => fail(name, fault.prefixed("its facts artifact entry")),
                },
            })
            .collect(),
    };
    let named = functions.iter().map(|function| (function.canonical.as_str(), function.aliases.as_slice()));
    let functions = match validate_names(named) {
        Ok(()) => functions,
        Err(error) => {
            let reason = format!("{file} is invalid: {error}");
            let site = refused_name_site(&functions);
            sites = names.iter().map(|name| ((*name).to_string(), site.clone())).collect();
            names.iter().map(|name| incomplete(name, reason.clone())).collect()
        }
    };
    Ok((HostFunctions::new(catalog, functions), sites))
}

fn incomplete(name: &str, reason: String) -> HostFunction {
    HostFunction {
        canonical: name.to_string(),
        aliases: Vec::new(),
        state: FunctionState::Incomplete { reason },
    }
}

/// Where the name [`validate_names`] refuses in `functions` is: the first
/// canonical name or alias, in the order it checks them, that a built-in or
/// an earlier name already claims (compared lowercased, as dispatch resolves
/// them). The artifact root when none is.
fn refused_name_site(functions: &[HostFunction]) -> ReasonSite {
    let mut seen = HashSet::new();
    for function in functions {
        let entry = Pointer::root().join("functions").join(&function.canonical);
        let names = std::iter::once((entry.clone(), &function.canonical))
            .chain(function.aliases.iter().enumerate().map(|(index, alias)| (entry.join("aliases").join(index), alias)));
        for (at, name) in names {
            let lower = name.to_ascii_lowercase();
            if is_builtin_name(&lower) || !seen.insert(lower) {
                return at.site();
            }
        }
    }
    Pointer::root().site()
}

// ── writer ──────────────────────────────────────────────────────────────────

fn function_entry(function: &HostFunction, registration: &FunctionRegistration) -> Value {
    let mut aliases = function.aliases.clone();
    aliases.sort();
    let facts: Vec<Value> = registration
        .narrowing_fact()
        .map(|fact| {
            json!({
                "parameter": fact.parameter(),
                "position": fact.index(),
                "kind": kind_text(fact.kind()),
                "type": match fact.fact_type() {
                    FactType::NonNull => NON_NULL_FACT_TYPE.to_string(),
                    FactType::Type(text) => text.to_string(),
                },
            })
        })
        .into_iter()
        .collect();
    let recovery = registration.numeric_recovery().map_or(Value::Null, |recovery| {
        json!({ "subject": recovery.subject, "fallback": recovery.fallback })
    });
    // Exclusions apply to every overload declaring the parameter, so the
    // first declaration found carries them.
    let mut exclusions: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for signature in registration.overloads() {
        for parameter in signature.parameters() {
            if !parameter.excluded_sources().is_empty() {
                exclusions.entry(parameter.name()).or_insert_with(|| {
                    let mut kinds: Vec<String> = parameter.excluded_sources().iter().map(ToString::to_string).collect();
                    kinds.sort();
                    kinds
                });
            }
        }
    }
    let exclusions: Vec<Value> = exclusions
        .into_iter()
        .map(|(parameter, kinds)| json!({ "parameter": parameter, "kinds": kinds }))
        .collect();
    let malformed = registration.malformed_reference_parameter().map_or(Value::Null, |position| {
        let parameter = registration.overloads()[0].parameters()[position].name();
        json!({ "parameter": parameter, "position": position })
    });
    json!({
        "aliases": aliases,
        "facts": facts,
        "lazy": registration.evaluation() == EvaluationMode::Lazy,
        "conversion_recovery": recovery,
        "source_exclusions": exclusions,
        "malformed_reference": malformed,
    })
}

fn kind_text(kind: FactKind) -> &'static str {
    match kind {
        FactKind::TrueImplies => "true_implies",
        FactKind::TrueIff => "true_iff",
        FactKind::FalseImplies => "false_implies",
    }
}

/// `value` with every object's keys in byte order (the crate's `serde_json`
/// preserves insertion order).
pub(crate) fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(entries.into_iter().map(|(key, value)| (key, sorted(value))).collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

// ── reader ──────────────────────────────────────────────────────────────────

/// Why the reader distrusts an artifact or an entry, and where in the
/// artifact: `None` for a file that has no text to point into.
#[derive(Debug, Clone)]
struct Fault {
    reason: String,
    site: Option<ReasonSite>,
}

impl Fault {
    /// The reason completed by the sentence's subject (`the facts artifact
    /// `…``, `its facts artifact entry`).
    fn prefixed(self, subject: &str) -> Self {
        Self { reason: format!("{subject} {}", self.reason), ..self }
    }
}

/// A JSON pointer into the artifact, as its segments.
#[derive(Debug, Clone, Default)]
struct Pointer(Vec<String>);

impl Pointer {
    fn root() -> Self {
        Self::default()
    }

    fn join(&self, segment: impl ToString) -> Self {
        let mut segments = self.0.clone();
        segments.push(segment.to_string());
        Self(segments)
    }

    fn site(&self) -> ReasonSite {
        ReasonSite::Field(self.0.clone())
    }

    fn fault(&self, reason: impl Into<String>) -> Fault {
        Fault { reason: reason.into(), site: Some(self.site()) }
    }
}

/// The field `key` of `object` (at `at`) by [`required`], or by
/// [`required_in`] for the nested object `owner`: an absent field is a
/// fault at `object`, a `null` one at the field.
fn need<'v>(object: &'v Map<String, Value>, key: &str, owner: Option<&str>, at: &Pointer) -> Result<&'v Value, Fault> {
    let read = match owner {
        None => required(object, key),
        Some(owner) => required_in(object, key, owner),
    };
    read.map_err(|reason| if object.contains_key(key) { at.join(key) } else { at.clone() }.fault(reason))
}

/// [`nullable`] for the field `key` of `object` (at `at`); absence is a
/// fault at `object`.
fn maybe<'v>(object: &'v Map<String, Value>, key: &str, at: &Pointer) -> Result<Option<&'v Value>, Fault> {
    nullable(object, key).map_err(|reason| at.fault(reason))
}

/// [`reject_unknown`] for `object` (at `at`), the fault at the unknown
/// field and its reason completed by `owner` (`` `facts[0]` ``) when given.
fn known(object: &Map<String, Value>, fields: &[&str], owner: Option<&str>, at: &Pointer) -> Result<(), Fault> {
    let Err(reason) = reject_unknown(object, fields) else {
        return Ok(());
    };
    let unknown = object.keys().find(|key| !fields.contains(&key.as_str()));
    let site = unknown.map_or_else(|| at.clone(), |key| at.join(key));
    Err(site.fault(match owner {
        Some(owner) => format!("`{owner}` {reason}"),
        None => reason,
    }))
}

/// The strict parser's refusal, at its own line and byte column.
fn parsed(text: &str) -> Result<Value, Fault> {
    crate::strict_json::parse(text).map_err(|error| {
        let (crate::strict_json::StrictJsonError::Invalid(cause) | crate::strict_json::StrictJsonError::Trailing(cause)) = &error;
        // `serde_json` reports line 0 only for an error that has no position.
        let site = (cause.line() > 0).then(|| ReasonSite::Position(cause.line(), cause.column()));
        Fault { reason: error.to_string(), site }
    })
}

/// The artifact's `functions` map, after every whole-file check; `Err` is
/// why every function is incomplete, its reason completing a sentence about
/// the file.
fn read_entries(
    catalog: &HostFunctionCatalog,
    names: &[&str],
    artifact: FactsArtifact<'_>,
) -> Result<Map<String, Value>, Fault> {
    let unplaced = |reason: String| Fault { reason, site: None };
    let text = match artifact {
        FactsArtifact::Text(text) => text,
        FactsArtifact::Absent => return Err(unplaced("is absent".to_string())),
        FactsArtifact::Unreadable(error) => return Err(unplaced(format!("cannot be read: {error}"))),
    };
    let root_at = Pointer::root();
    let root = parsed(text)?;
    let Value::Object(root) = root else {
        return Err(root_at.fault(format!("is {} instead of an object", describe(&root))));
    };
    known(&root, TOP_FIELDS, None, &root_at)?;
    let version = need(&root, "format_version", None, &root_at)?;
    let version_at = root_at.join("format_version");
    match version.as_u64() {
        Some(FACTS_ARTIFACT_FORMAT_VERSION) => {}
        Some(other) => {
            return Err(version_at.fault(format!(
                "has unknown `format_version` {other}; this reader understands {FACTS_ARTIFACT_FORMAT_VERSION}"
            )));
        }
        None => return Err(version_at.fault(format!("has `format_version` {} instead of an integer", describe(version)))),
    }
    let hash_at = root_at.join("catalog_hash");
    let hash = string(need(&root, "catalog_hash", None, &root_at)?, "catalog_hash").map_err(|reason| hash_at.fault(reason))?;
    if hash != catalog.version() {
        return Err(hash_at.fault(format!(
            "records catalog hash `{hash}`, but the loaded catalog's hash is `{}`; regenerate the artifact",
            catalog.version()
        )));
    }
    let functions_at = root_at.join("functions");
    let functions = need(&root, "functions", None, &root_at)?;
    let Value::Object(functions) = functions else {
        return Err(functions_at.fault(format!("has `functions` {} instead of an object", describe(functions))));
    };
    if let Some(stray) = functions.keys().find(|key| !names.contains(&key.as_str())) {
        return Err(functions_at.join(stray).fault(format!("has an entry for `{stray}`, which the catalog does not declare")));
    }
    Ok(functions.clone())
}

/// One function's aliases and registration, rebuilt from its entry; `Err`
/// completes "its facts artifact entry …".
fn rebuild(
    catalog: &HostFunctionCatalog,
    name: &str,
    entry: &Value,
) -> Result<(Vec<String>, FunctionRegistration), Fault> {
    let at = Pointer::root().join("functions").join(name);
    let Value::Object(entry) = entry else {
        return Err(at.fault(format!("is {} instead of an object", describe(entry))));
    };
    known(entry, FUNCTION_FIELDS, None, &at)?;

    let aliases_at = at.join("aliases");
    let aliases = array(need(entry, "aliases", None, &at)?, "aliases")
        .map_err(|reason| aliases_at.fault(reason))?
        .iter()
        .enumerate()
        .map(|(index, alias)| {
            string(alias, &format!("aliases[{index}]")).map(str::to_string).map_err(|reason| aliases_at.join(index).fault(reason))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let facts_at = at.join("facts");
    let facts = array(need(entry, "facts", None, &at)?, "facts").map_err(|reason| facts_at.fault(reason))?;
    let mut parsed_facts = Vec::with_capacity(facts.len());
    for (index, fact) in facts.iter().enumerate() {
        parsed_facts.push(parse_fact(fact, &format!("facts[{index}]"), &facts_at.join(index))?);
    }
    if parsed_facts.len() > 1 {
        return Err(facts_at.fault(format!("lists {} facts; a function registers at most one", parsed_facts.len())));
    }

    let lazy_at = at.join("lazy");
    match need(entry, "lazy", None, &at)? {
        Value::Bool(false) => {}
        Value::Bool(true) => return Err(lazy_at.fault("has `lazy: true`; host functions are evaluated eagerly")),
        other => return Err(lazy_at.fault(format!("has `lazy` {} instead of a boolean", describe(other)))),
    }

    let recovery_at = at.join("conversion_recovery");
    let recovery = match maybe(entry, "conversion_recovery", &at)? {
        None => None,
        Some(value) => {
            let object = object(value, "conversion_recovery").map_err(|reason| recovery_at.fault(reason))?;
            known(object, RECOVERY_FIELDS, Some("conversion_recovery"), &recovery_at)?;
            let subject = string(need(object, "subject", Some("conversion_recovery"), &recovery_at)?, "conversion_recovery.subject")
                .map_err(|reason| recovery_at.join("subject").fault(reason))?;
            let fallback = string(need(object, "fallback", Some("conversion_recovery"), &recovery_at)?, "conversion_recovery.fallback")
                .map_err(|reason| recovery_at.join("fallback").fault(reason))?;
            Some((subject.to_string(), fallback.to_string()))
        }
    };

    let exclusions_at = at.join("source_exclusions");
    let exclusions =
        array(need(entry, "source_exclusions", None, &at)?, "source_exclusions").map_err(|reason| exclusions_at.fault(reason))?;
    let mut parsed_exclusions = Vec::with_capacity(exclusions.len());
    for (index, exclusion) in exclusions.iter().enumerate() {
        let field = format!("source_exclusions[{index}]");
        let exclusion_at = exclusions_at.join(index);
        let object = object(exclusion, &field).map_err(|reason| exclusion_at.fault(reason))?;
        known(object, EXCLUSION_FIELDS, Some(&field), &exclusion_at)?;
        let parameter = string(need(object, "parameter", Some(&field), &exclusion_at)?, &format!("{field}.parameter"))
            .map_err(|reason| exclusion_at.join("parameter").fault(reason))?;
        let kinds_field = format!("{field}.kinds");
        let kinds_at = exclusion_at.join("kinds");
        let kinds = array(need(object, "kinds", Some(&field), &exclusion_at)?, &kinds_field)
            .map_err(|reason| kinds_at.fault(reason))?
            .iter()
            .enumerate()
            .map(|(kind_index, kind)| {
                value_kind(kind, &format!("{kinds_field}[{kind_index}]")).map_err(|reason| kinds_at.join(kind_index).fault(reason))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if kinds.is_empty() {
            return Err(kinds_at.fault(format!("has an empty `{kinds_field}`; an exclusion names at least one kind")));
        }
        parsed_exclusions.push((parameter.to_string(), kinds));
    }

    let reference_at = at.join("malformed_reference");
    let malformed = match maybe(entry, "malformed_reference", &at)? {
        None => None,
        Some(value) => {
            let object = object(value, "malformed_reference").map_err(|reason| reference_at.fault(reason))?;
            known(object, REFERENCE_FIELDS, Some("malformed_reference"), &reference_at)?;
            let parameter = string(need(object, "parameter", Some("malformed_reference"), &reference_at)?, "malformed_reference.parameter")
                .map_err(|reason| reference_at.join("parameter").fault(reason))?;
            let position = position(need(object, "position", Some("malformed_reference"), &reference_at)?, "malformed_reference.position")
                .map_err(|reason| reference_at.join("position").fault(reason))?;
            Some((parameter.to_string(), position))
        }
    };

    let mut builder: RegistrationBuilder =
        catalog.registration_builder(name).expect("the reader rebuilds only catalog functions");
    for (parameter, kinds) in &parsed_exclusions {
        builder = builder.reject_sources(parameter.clone(), kinds);
    }
    if let Some((subject, fallback)) = &recovery {
        builder = builder.numeric_recovery(subject.clone(), fallback.clone());
    }
    if let Some((parameter, _)) = &malformed {
        builder = builder.false_for_malformed_reference(parameter.clone());
    }
    // The registry judges the entry's facts together, so the refusal is the
    // entry's.
    let registration = catalog.build(builder).map_err(|error| at.fault(format!("does not register: {error}")))?;

    match (parsed_facts.first(), registration.narrowing_fact()) {
        (None, Some(built)) => {
            return Err(facts_at.fault(format!(
                "records no fact, but the catalog declares one for `{}`",
                built.parameter()
            )));
        }
        (Some(fact), None) => {
            return Err(facts_at.fault(format!(
                "records a fact for `{}`, but the catalog declares none",
                fact.parameter
            )));
        }
        _ => {}
    }
    if let (Some(fact), Some(built)) = (parsed_facts.first(), registration.narrowing_fact())
        && fact.position != built.index()
    {
        return Err(facts_at.join(0).join("position").fault(format!(
            "records `facts[0].position` {}, but `{}` is at position {} in the catalog",
            fact.position,
            fact.parameter,
            built.index()
        )));
    }
    if let (Some(fact), Some(built)) = (parsed_facts.first(), registration.narrowing_fact())
        && (fact.parameter != built.parameter() || fact.kind != built.kind() || fact.ty != *built.fact_type())
    {
        return Err(facts_at.fault(format!(
            "records a fact that differs from the one the catalog declares for `{}`",
            built.parameter()
        )));
    }
    if let (Some((parameter, recorded)), Some(built)) = (&malformed, registration.malformed_reference_parameter())
        && *recorded != built
    {
        return Err(reference_at.join("position").fault(format!(
            "records `malformed_reference.position` {recorded}, but `{parameter}` is at position {built} in the catalog"
        )));
    }
    Ok((aliases, registration))
}

struct ParsedFact {
    parameter: String,
    position: usize,
    kind: FactKind,
    ty: FactType,
}

/// The fact `value`, named `field` (`facts[0]`) in reasons and found at `at`.
fn parse_fact(value: &Value, field: &str, at: &Pointer) -> Result<ParsedFact, Fault> {
    let object = object(value, field).map_err(|reason| at.fault(reason))?;
    known(object, FACT_FIELDS, Some(field), at)?;
    let read = |key: &str| need(object, key, Some(field), at);
    let parameter =
        string(read("parameter")?, &format!("{field}.parameter")).map_err(|reason| at.join("parameter").fault(reason))?;
    let position = position(read("position")?, &format!("{field}.position")).map_err(|reason| at.join("position").fault(reason))?;
    let kind_at = at.join("kind");
    let kind = match string(read("kind")?, &format!("{field}.kind")).map_err(|reason| kind_at.fault(reason))? {
        "true_implies" => FactKind::TrueImplies,
        "true_iff" => FactKind::TrueIff,
        "false_implies" => FactKind::FalseImplies,
        other => {
            return Err(kind_at.fault(format!(
                "has `{field}.kind` `{other}`; expected `true_implies`, `true_iff`, or `false_implies`"
            )));
        }
    };
    let ty = match string(read("type")?, &format!("{field}.type")).map_err(|reason| at.join("type").fault(reason))? {
        NON_NULL_FACT_TYPE => FactType::NonNull,
        text => FactType::Type(text.to_string().into()),
    };
    Ok(ParsedFact {
        parameter: parameter.to_string(),
        position,
        kind,
        ty,
    })
}

fn value_kind(value: &Value, field: &str) -> Result<ValueKind, String> {
    Ok(match string(value, field)? {
        "text" => ValueKind::Text,
        "number" => ValueKind::Number,
        "boolean" => ValueKind::Boolean,
        "null" => ValueKind::Null,
        "list" => ValueKind::List,
        "object" => ValueKind::Object,
        other => {
            return Err(format!(
                "has `{field}` `{other}`; expected one of text, number, boolean, null, list, object"
            ));
        }
    })
}

// ── strict field access: absent, null, and wrong type are distinct ─────────

/// "a string", "null", …, for messages.
pub(crate) fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

pub(crate) fn reject_unknown(object: &Map<String, Value>, known: &[&str]) -> Result<(), String> {
    match object.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => Err(format!("has unknown field `{key}`")),
        None => Ok(()),
    }
}

/// The non-null value of a top-level or entry field.
pub(crate) fn required<'v>(object: &'v Map<String, Value>, key: &str) -> Result<&'v Value, String> {
    match object.get(key) {
        None => Err(format!("has no `{key}` field")),
        Some(Value::Null) => Err(format!("has `{key}: null`")),
        Some(value) => Ok(value),
    }
}

/// [`required`] for a field of the nested object `owner`.
pub(crate) fn required_in<'v>(object: &'v Map<String, Value>, key: &str, owner: &str) -> Result<&'v Value, String> {
    match object.get(key) {
        None => Err(format!("has no `{owner}.{key}` field")),
        Some(Value::Null) => Err(format!("has `{owner}.{key}: null`")),
        Some(value) => Ok(value),
    }
}

/// A field whose explicit `null` means "none"; absence is still an error.
pub(crate) fn nullable<'v>(object: &'v Map<String, Value>, key: &str) -> Result<Option<&'v Value>, String> {
    match object.get(key) {
        None => Err(format!("has no `{key}` field (write `null` for none)")),
        Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(value)),
    }
}

pub(crate) fn string<'v>(value: &'v Value, field: &str) -> Result<&'v str, String> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(format!("has `{field}` {} instead of a string", describe(other))),
    }
}

pub(crate) fn array<'v>(value: &'v Value, field: &str) -> Result<&'v Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(format!("has `{field}` {} instead of an array", describe(other))),
    }
}

pub(crate) fn object<'v>(value: &'v Value, field: &str) -> Result<&'v Map<String, Value>, String> {
    match value {
        Value::Object(object) => Ok(object),
        other => Err(format!("has `{field}` {} instead of an object", describe(other))),
    }
}

fn position(value: &Value, field: &str) -> Result<usize, String> {
    value
        .as_u64()
        .and_then(|position| usize::try_from(position).ok())
        .ok_or_else(|| format!("has `{field}` {} instead of a non-negative integer", describe(value)))
}

// ── where a reason points ───────────────────────────────────────────────────

/// What an artifact reader's refusal is about in the artifact's text: the
/// facts reader records it as it judges, the lifecycle reader
/// ([`crate::lifecycle::host_profile`]) through [`reason_site`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReasonSite {
    /// The value at these JSON pointer segments; an absent field is reported
    /// at the object that lacks it.
    Field(Vec<String>),
    /// The parser's own 1-based line and column, for text that is not JSON.
    Position(usize, usize),
}

/// Where `reason`, a reason of the lifecycle artifact reader without its
/// leading file phrase, is about: the inverse of the field-naming forms it
/// writes with the helpers above (`has no `x` field`, `has `events[0].name`
/// a number instead of a string`, `records catalog hash …`). `None` for a
/// reason naming no field (a registration the registry refuses).
pub(crate) fn reason_site(reason: &str) -> Option<ReasonSite> {
    if reason.starts_with("is not valid JSON") || reason.starts_with("has content after its JSON value") {
        // `serde_json` ends its message with ` at line L column C`.
        let (_, at) = reason.rsplit_once(" at line ")?;
        let (line, column) = at.split_once(" column ")?;
        return Some(ReasonSite::Position(line.trim().parse().ok()?, column.trim().parse().ok()?));
    }
    let ticked: Vec<&str> = reason.split('`').skip(1).step_by(2).collect();
    let field = |text: &str| ReasonSite::Field(field_segments(text));
    let first = ticked.first().copied();
    let site = if reason.starts_with("is ") {
        ReasonSite::Field(Vec::new())
    } else if reason.starts_with("records catalog hash") {
        field("catalog_hash")
    } else if reason.starts_with("binds host global") {
        field(ticked.get(1)?)
    } else if reason.starts_with("gives Darkmatter event") {
        field(ticked.last()?)
    } else if reason.starts_with("lists Darkmatter event") || reason.starts_with("does not list every Darkmatter event") {
        field("events")
    } else if reason.starts_with("has unknown field") {
        field(first?)
    } else if let Some(owner) = reason.strip_prefix('`').and_then(|rest| rest.split_once("` has unknown field `")) {
        ReasonSite::Field([field_segments(owner.0), field_segments(owner.1.trim_end_matches('`'))].concat())
    } else if reason.starts_with("has no `") {
        let mut segments = field_segments(first?);
        segments.pop();
        ReasonSite::Field(segments)
    } else if reason.starts_with("has `") || reason.starts_with("has unknown `") {
        field(first?.split(':').next().unwrap_or_default())
    } else {
        return None;
    };
    Some(site)
}

/// `events[0].globals.declarations[1].root` as pointer segments.
fn field_segments(field: &str) -> Vec<String> {
    let mut segments = Vec::new();
    for part in field.split('.') {
        let (name, mut indexes) = part.split_once('[').map_or((part, ""), |(name, rest)| (name, rest));
        if !name.is_empty() {
            segments.push(name.to_string());
        }
        while let Some((index, rest)) = indexes.split_once(']') {
            segments.push(index.to_string());
            indexes = rest.strip_prefix('[').unwrap_or(rest);
        }
    }
    segments
}

// ── strict JSON ─────────────────────────────────────────────────────────────

/// Parses `text` as one JSON value, rejecting a duplicate object key
/// (`serde_json` keeps the last) and anything after the value. `Err`
/// completes a sentence naming the artifact ("the lifecycle artifact `…` …").
pub(crate) fn parse_strict(text: &str) -> Result<Value, String> {
    crate::strict_json::parse(text).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_naming_reason_form_points_at_its_field() {
        let field = |segments: &[&str]| Some(ReasonSite::Field(segments.iter().map(|segment| (*segment).to_string()).collect()));
        for (reason, expected) in [
            ("is not valid JSON: EOF while parsing an object at line 3 column 1", Some(ReasonSite::Position(3, 1))),
            ("has content after its JSON value: trailing characters at line 1 column 4", Some(ReasonSite::Position(1, 4))),
            ("is an array instead of an object", field(&[])),
            ("has no `format_version` field", field(&[])),
            ("has no `outcome_pair` field (write `null` for none)", field(&[])),
            ("has no `events[2].globals` field", field(&["events", "2"])),
            ("has no `events[0].position` for host event `x`", field(&["events", "0"])),
            ("has `effects: null`", field(&["effects"])),
            ("has `events[0].globals.declarations[1].root` a number instead of a string", field(&["events", "0", "globals", "declarations", "1", "root"])),
            ("has `outcome_pair.downgrades[0][1]` a number instead of a string", field(&["outcome_pair", "downgrades", "0", "1"])),
            ("has unknown field `extra`", field(&["extra"])),
            ("`events[1]` has unknown field `extra`", field(&["events", "1", "extra"])),
            ("has unknown `format_version` 3; this reader understands 2", field(&["format_version"])),
            ("records catalog hash `00`, but the loaded catalog's hash is `11`; regenerate the artifact", field(&["catalog_hash"])),
            ("binds host global `stray` (`host_globals[1]`), which no namespace of the host's catalog declares", field(&["host_globals", "1"])),
            ("gives Darkmatter event `composed` a position or role at `events[3]`", field(&["events", "3"])),
            ("lists Darkmatter event `composed` twice", field(&["events"])),
            ("does not list every Darkmatter event", field(&["events"])),
            ("records a registration the registry refuses: boom", None),
        ] {
            assert_eq!(reason_site(reason), expected, "{reason}");
        }
    }

    const CATALOG: &str = "kind: expression-schema\ncategories:\n  - Project\ntypes:\n  pick: |-\n    function(parameters(value: unknown); returns(boolean); category(Project))\n";

    fn catalog() -> HostFunctionCatalog {
        HostFunctionCatalog::load(CATALOG, "host/functions.yaml").expect("the catalog loads")
    }

    /// The artifact the host's registration writes for [`CATALOG`].
    fn written() -> String {
        let handler: super::super::HostHandler = std::sync::Arc::new(|_| Ok(json!(true)));
        HostFunctions::builder(catalog()).function("pick", &[], |registration| registration, handler).build().unwrap().facts_artifact()
    }

    /// The one incomplete function's reason and its site, for `artifact`.
    fn judged(artifact: FactsArtifact<'_>) -> (String, Option<ReasonSite>) {
        let (functions, sites) = analysis_only_sited(catalog(), Path::new("f.json"), artifact).expect("the catalog's names are valid");
        let [(name, reason)] = functions.incomplete_functions()[..] else { panic!("`pick` is incomplete") };
        (reason.to_string(), sites.get(name).cloned())
    }

    fn field(segments: &[&str]) -> Option<ReasonSite> {
        Some(ReasonSite::Field(segments.iter().map(|segment| (*segment).to_string()).collect()))
    }

    #[test]
    fn the_reader_records_where_each_refusal_is_as_it_judges() {
        let written = written();
        let edited = |from: &str, to: &str| {
            assert!(written.contains(from), "{from}");
            written.replacen(from, to, 1)
        };
        for (text, expected, reason) in [
            ("{\n".to_string(), Some(ReasonSite::Position(2, 0)), "the facts artifact `f.json` is not valid JSON"),
            ("{} {}".to_string(), Some(ReasonSite::Position(1, 4)), "the facts artifact `f.json` has content after"),
            ("{\"a\": {\"b\": 1, \"b\": 1}}".to_string(), Some(ReasonSite::Position(1, 18)), "the facts artifact `f.json` is not valid JSON: duplicate key `b`"),
            ("[]".to_string(), field(&[]), "the facts artifact `f.json` is an array instead of an object"),
            (edited("\"catalog_hash\"", "\"stray\": 1, \"catalog_hash\""), field(&["stray"]), "has unknown field `stray`"),
            (edited("\"format_version\": 1", "\"format_version\": 2"), field(&["format_version"]), "has unknown `format_version` 2"),
            (edited("\"catalog_hash\": \"", "\"catalog_hash\": \"0"), field(&["catalog_hash"]), "records catalog hash"),
            (edited("\"functions\": {", "\"functions\": {\"stray\": {},"), field(&["functions", "stray"]), "has an entry for `stray`"),
            (edited("\"lazy\": false", "\"lazy\": 1"), field(&["functions", "pick", "lazy"]), "its facts artifact entry has `lazy` a number instead of a boolean"),
            (edited("\"conversion_recovery\": null,", ""), field(&["functions", "pick"]), "has no `conversion_recovery` field (write `null` for none)"),
            (edited("\"aliases\": []", "\"aliases\": [1]"), field(&["functions", "pick", "aliases", "0"]), "has `aliases[0]` a number instead of a string"),
            (edited("\"aliases\": []", "\"aliases\": [\"MIN\"]"), field(&["functions", "pick", "aliases", "0"]), "the facts artifact `f.json` is invalid"),
            (
                edited("\"source_exclusions\": []", "\"source_exclusions\": [{\"kinds\": [], \"parameter\": \"value\"}]"),
                field(&["functions", "pick", "source_exclusions", "0", "kinds"]),
                "has an empty `source_exclusions[0].kinds`",
            ),
            (
                edited("\"facts\": []", "\"facts\": [{\"kind\": \"true_implies\", \"parameter\": \"value\", \"position\": 0, \"type\": \"string\"}]"),
                field(&["functions", "pick", "facts"]),
                "records a fact for `value`, but the catalog declares none",
            ),
        ] {
            let (judged_reason, site) = judged(FactsArtifact::Text(&text));
            assert_eq!(site, expected, "{text}: {judged_reason}");
            assert!(judged_reason.contains(reason), "{text}: {judged_reason}");
        }
        assert_eq!(judged(FactsArtifact::Absent), ("the facts artifact `f.json` is absent".to_string(), None), "a file-level reason");
        assert_eq!(judged(FactsArtifact::Unreadable("boom")).1, None);
    }

    #[test]
    fn an_artifact_with_no_entry_for_a_function_is_refused_at_its_functions() {
        let text = format!("{{\"catalog_hash\": \"{}\", \"format_version\": 1, \"functions\": {{}}}}", catalog().version());
        assert_eq!(judged(FactsArtifact::Text(&text)), ("the facts artifact has no entry for it".to_string(), field(&["functions"])));
        let complete = written();
        let (functions, sites) = analysis_only_sited(catalog(), Path::new("f.json"), FactsArtifact::Text(&complete)).unwrap();
        assert!(functions.incomplete_functions().is_empty() && sites.is_empty());
    }

    #[test]
    fn the_strict_parser_rejects_duplicate_keys_at_any_depth_and_trailing_content() {
        assert_eq!(parse_strict(r#"{"a": {"b": 1}}"#), Ok(json!({"a": {"b": 1}})));
        for text in [r#"{"a": 1, "a": 2}"#, r#"{"a": {"b": 1, "b": 1}}"#, r#"[{"x": 1, "x": 1}]"#] {
            let error = parse_strict(text).unwrap_err();
            assert!(error.contains("duplicate key"), "{text}: {error}");
        }
        assert!(parse_strict("{} {}").unwrap_err().contains("after its JSON value"));
        assert!(parse_strict("{").unwrap_err().contains("not valid JSON"));
        assert_eq!(parse_strict(" {}\n\n"), Ok(json!({})));
    }

    #[test]
    fn sorting_orders_every_level() {
        let value = sorted(json!({"b": [{"z": 1, "a": 2}], "a": null}));
        assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"a":null,"b":[{"a":2,"z":1}]}"#);
    }

    #[test]
    fn absent_null_and_wrong_type_read_differently() {
        let object = json!({"n": null, "s": 1});
        let object = object.as_object().unwrap();
        assert_eq!(required(object, "x").unwrap_err(), "has no `x` field");
        assert_eq!(required(object, "n").unwrap_err(), "has `n: null`");
        assert_eq!(string(required(object, "s").unwrap(), "s").unwrap_err(), "has `s` a number instead of a string");
        assert_eq!(nullable(object, "n"), Ok(None));
        assert!(nullable(object, "x").is_err());
    }

    #[test]
    fn the_artifact_sits_beside_its_catalog() {
        assert_eq!(facts_artifact_path(Path::new("a/functions.yaml")), PathBuf::from("a/functions.facts.json"));
    }
}
