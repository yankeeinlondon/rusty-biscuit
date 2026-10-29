//! A host's lifecycle profile as passive data: the JSON companion that carries
//! a host's Rust event registration and lifecycle parse options to a process
//! that cannot run that registration (the editor).
//!
//! A host whose events, renames, per-event globals, or accepted verbs differ
//! from the `md` host's writes the artifact beside its host global catalog,
//! named `<stem>.lifecycle.json` (`globals.yaml` → `globals.lifecycle.json`),
//! with one call ([`lifecycle_artifact`]). A reader rebuilds an
//! [`EventRegistry`] and [`LifecycleParseOptions`] from it
//! ([`HostLifecycleProfile::from_artifact`]) through the same registration
//! calls the host makes, so a registration the artifact cannot carry fails
//! exactly as it would in the host. The library's one scoped walk
//! ([`super::lifecycle_call_checks`]) then types the document's lifecycle as
//! the host's runtime does. Nothing in a profile runs: it has no services,
//! handlers, or effects.
//!
//! Like the facts artifact of a host function catalog, the file is versioned
//! against its catalog by the BLAKE3 hash of the catalog's bytes, the writer
//! emits deterministic bytes (keys sorted at every level, pretty-printed, one
//! trailing newline), and the reader is strict: a duplicate key, trailing
//! content, an unknown field, or a field that is absent, `null` where `null`
//! means nothing, or of the wrong type is never defaulted. Any problem makes
//! the whole profile unusable, with a human-readable reason.
//!
//! ```json
//! {
//!   "catalog_hash": "<BLAKE3 hex of the catalog's bytes>",
//!   "effects": ["doorbell", "sad-trombone"],
//!   "events": [
//!     { "actions": ["communicate", "expression", "flow"], "core": "initialize",
//!       "globals": { "declarations": [ … ], "scope": "claudine.initialize" },
//!       "name": "initialize", "position": null, "role": null },
//!     { "actions": [ … ], "core": null, "globals": { … }, "name": "success",
//!       "position": "after-composition", "role": "terminal-outcome" }
//!   ],
//!   "format_version": 1,
//!   "global_catalog": true,
//!   "host": "claudine",
//!   "host_globals": ["loop", "group"],
//!   "outcome_pair": { "downgrades": [["success", "failure"]], "failed": "failure", "succeeded": "success" },
//!   "unknown_verbs": "reject"
//! }
//! ```
//!
//! `events` lists Darkmatter's five events in [`CoreEvent::ALL`] order (with
//! the host's spelling as `name`), then the host's own in registration order.
//! A declaration's `availability` is `"available"`, `"execution-dependent"`,
//! or `{ "unavailable": { "code": …, "parameters": { … } } }`. `unknown_verbs`
//! is `"reject"`, `"ignore"`, or `{ "ignore_unprovided": [verbs] }`. `effects`
//! lists the effect names the host's `is_known_effect` accepts; the rebuilt
//! predicate compares names by their lowercase letters and digits alone, the
//! rule Playa's lookup falls back to.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;


use serde_json::{Map, Value, json};

use super::parse::{LifecycleParseOptions, UnknownVerbPolicy};
use super::registry::{
    ActionType, ActionTypes, CoreEvent, EventDescriptor, EventId, EventRegistry, OutcomePair, Position, Role,
};
use crate::markdown::Frontmatter;
use crate::markdown::compose::expression::host::FactsArtifact;
use crate::markdown::compose::expression::host::artifact::{
    array, describe, nullable, object, parse_strict, reject_unknown, required, required_in, sorted, string,
};
use crate::markdown::compose::expression::{Availability, BindingView, ScopeId, UnavailabilityReason};
use crate::markdown::schemas::catalog::CatalogHandle;

/// The artifact format this library writes and reads.
pub const LIFECYCLE_ARTIFACT_FORMAT_VERSION: u64 = 1;

const TOP_FIELDS: &[&str] = &[
    "catalog_hash",
    "effects",
    "events",
    "format_version",
    "global_catalog",
    "host",
    "host_globals",
    "outcome_pair",
    "unknown_verbs",
];
const EVENT_FIELDS: &[&str] = &["actions", "core", "globals", "name", "position", "role"];
const VIEW_FIELDS: &[&str] = &["declarations", "scope"];
const DECLARATION_FIELDS: &[&str] = &["availability", "description", "root"];
const REASON_FIELDS: &[&str] = &["code", "parameters"];
const PAIR_FIELDS: &[&str] = &["downgrades", "failed", "succeeded"];

/// Darkmatter's generic late globals, which every registry declares first.
const GENERIC_GLOBALS: [&str; 2] = ["err", "timing"];

const ACTION_TYPES: [(ActionType, &str); 4] = [
    (ActionType::Communicate, "communicate"),
    (ActionType::Expression, "expression"),
    (ActionType::Flow, "flow"),
    (ActionType::Shell, "shell"),
];

/// The artifact beside `catalog`: its file stem plus `.lifecycle.json`
/// (`globals.yaml` → `globals.lifecycle.json`).
pub fn lifecycle_artifact_path(catalog: &Path) -> PathBuf {
    catalog.with_extension("lifecycle.json")
}

/// The lifecycle artifact of the host whose profile `options` is, for the
/// host global catalog whose bytes are `catalog_text`: its registry's events,
/// renames, per-event globals and action types, outcome pair, and late
/// globals, its unknown-verb policy, and those of `effect_names` its
/// `is_known_effect` accepts, plus the catalog's hash and
/// [`LIFECYCLE_ARTIFACT_FORMAT_VERSION`]. The bytes depend only on these.
pub fn lifecycle_artifact(options: &LifecycleParseOptions<'_>, effect_names: &[&str], catalog_text: &str) -> String {
    let registry = options.registry;
    let mut events: Vec<Value> = CoreEvent::ALL
        .into_iter()
        .map(|event| event_entry(registry, EventId::core(event), Some(event)))
        .collect();
    events.extend(registry.host_events().map(|id| event_entry(registry, id, None)));
    let pair = match (registry.succeeded_alternative(), registry.failed_alternative()) {
        (Some(succeeded), Some(failed)) => {
            let downgrades: Vec<Value> = [succeeded, EventId::core(CoreEvent::Failed)]
                .into_iter()
                .filter_map(|from| {
                    registry.downgrade_target(from).map(|to| json!([registry.name(from), registry.name(to)]))
                })
                .collect();
            json!({
                "succeeded": registry.name(succeeded),
                "failed": registry.name(failed),
                "downgrades": downgrades,
            })
        }
        _ => Value::Null,
    };
    let host_globals: Vec<&str> =
        registry.host_globals().iter().map(AsRef::as_ref).filter(|name| !GENERIC_GLOBALS.contains(name)).collect();
    let unknown_verbs = match &options.unknown_verbs {
        UnknownVerbPolicy::Reject => json!("reject"),
        UnknownVerbPolicy::Ignore => json!("ignore"),
        UnknownVerbPolicy::IgnoreUnprovided(verbs) => json!({ "ignore_unprovided": verbs.iter().collect::<Vec<_>>() }),
    };
    let effects: BTreeSet<&str> = effect_names.iter().copied().filter(|name| (options.is_known_effect)(name)).collect();
    let root = json!({
        "catalog_hash": biscuit_hash::blake3_hash(catalog_text),
        "effects": effects.into_iter().collect::<Vec<_>>(),
        "events": events,
        "format_version": LIFECYCLE_ARTIFACT_FORMAT_VERSION,
        "global_catalog": registry.global_catalog().is_some(),
        "host": registry.host(),
        "host_globals": host_globals,
        "outcome_pair": pair,
        "unknown_verbs": unknown_verbs,
    });
    let mut text = serde_json::to_string_pretty(&sorted(root)).expect("a JSON value serializes");
    text.push('\n');
    text
}

fn event_entry(registry: &EventRegistry, id: EventId, core: Option<CoreEvent>) -> Value {
    let actions: Vec<&str> = ACTION_TYPES
        .iter()
        .filter(|(action, _)| registry.actions(id).contains(*action))
        .map(|(_, name)| *name)
        .collect();
    let view = registry.globals(id);
    let declarations: Vec<Value> = view
        .globals()
        .map(|global| {
            let availability = match global.availability() {
                Availability::Available => json!("available"),
                Availability::ExecutionDependent => json!("execution-dependent"),
                Availability::Unavailable(reason) => {
                    json!({ "unavailable": { "code": reason.code(), "parameters": reason.parameters() } })
                }
            };
            json!({ "root": global.root(), "description": global.description(), "availability": availability })
        })
        .collect();
    json!({
        "name": registry.name(id),
        "core": core.map(CoreEvent::darkmatter_name),
        "position": registry.position(id).map(Position::as_str),
        "role": registry.role(id).map(Role::as_str),
        "actions": actions,
        "globals": { "scope": view.scope().as_str(), "declarations": declarations },
    })
}

/// A host's lifecycle profile rebuilt from its artifact: the event registry
/// and parse options the host's runtime parses and types a lifecycle with.
#[derive(Clone)]
pub struct HostLifecycleProfile {
    registry: Arc<EventRegistry>,
    unknown_verbs: UnknownVerbPolicy,
    is_known_effect: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl std::fmt::Debug for HostLifecycleProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostLifecycleProfile")
            .field("registry", &self.registry)
            .field("unknown_verbs", &self.unknown_verbs)
            .finish_non_exhaustive()
    }
}

/// An effect name reduced to what Playa's fallback lookup compares.
fn effect_key(name: &str) -> String {
    name.chars().filter(|ch| ch.is_alphanumeric()).collect::<String>().to_lowercase()
}

impl HostLifecycleProfile {
    /// The host's event registry.
    pub fn registry(&self) -> &Arc<EventRegistry> {
        &self.registry
    }

    /// Whether `name` is an effect the host knows.
    pub fn is_known_effect(&self, name: &str) -> bool {
        (self.is_known_effect)(name)
    }

    /// The host's parse options, with `authored` giving `set:` mappings their
    /// authored key order.
    pub fn parse_options<'a>(&'a self, authored: Option<&'a Frontmatter>) -> LifecycleParseOptions<'a> {
        LifecycleParseOptions {
            is_known_effect: &*self.is_known_effect,
            authored,
            unknown_verbs: self.unknown_verbs.clone(),
            registry: &self.registry,
        }
    }

    /// The profile `artifact` records for `catalog`, loaded from
    /// `catalog_path` with bytes `catalog_text`; the registry types the host's
    /// globals with `catalog` when the host registered its catalog. Passive:
    /// nothing is run.
    ///
    /// The profile owns what it reads: its registry holds the catalog, the
    /// late-global names, and the `ignore_unprovided` verbs, and releases
    /// them when the last holder of the profile or registry drops.
    ///
    /// ## Errors
    ///
    /// A human-readable reason naming the artifact when it is absent or
    /// unreadable, is not strict JSON, records another catalog's hash or an
    /// unknown format, has a field it should not or lacks one it should, or
    /// records a registration the registry refuses (an outcome alternative
    /// with no event, a renamed fixed event, a reserved root, …).
    pub fn from_artifact(
        catalog: impl Into<CatalogHandle>,
        catalog_path: &Path,
        catalog_text: &str,
        artifact: FactsArtifact<'_>,
    ) -> Result<Self, String> {
        let file = format!("the lifecycle artifact `{}`", lifecycle_artifact_path(catalog_path).display());
        read(catalog.into(), catalog_text, artifact).map_err(|reason| format!("{file} {reason}"))
    }
}

// ── reader ──────────────────────────────────────────────────────────────────

fn read(catalog: CatalogHandle, catalog_text: &str, artifact: FactsArtifact<'_>) -> Result<HostLifecycleProfile, String> {
    let text = match artifact {
        FactsArtifact::Text(text) => text,
        FactsArtifact::Absent => return Err("is absent".to_string()),
        FactsArtifact::Unreadable(error) => return Err(format!("cannot be read: {error}")),
    };
    let root = parse_strict(text)?;
    let Value::Object(root) = root else {
        return Err(format!("is {} instead of an object", describe(&root)));
    };
    reject_unknown(&root, TOP_FIELDS)?;
    let version = required(&root, "format_version")?;
    match version.as_u64() {
        Some(LIFECYCLE_ARTIFACT_FORMAT_VERSION) => {}
        Some(other) => {
            return Err(format!(
                "has unknown `format_version` {other}; this reader understands {LIFECYCLE_ARTIFACT_FORMAT_VERSION}"
            ));
        }
        None => return Err(format!("has `format_version` {} instead of an integer", describe(version))),
    }
    let hash = string(required(&root, "catalog_hash")?, "catalog_hash")?;
    let expected = biscuit_hash::blake3_hash(catalog_text);
    if hash != expected {
        return Err(format!(
            "records catalog hash `{hash}`, but the loaded catalog's hash is `{expected}`; regenerate the artifact"
        ));
    }
    let host = string(required(&root, "host")?, "host")?;
    let mut registry = EventRegistry::new(host);
    let refused = |error: super::registry::RegistrationError| format!("records a registration the registry refuses: {error}");

    let events = array(required(&root, "events")?, "events")?;
    let mut cores = BTreeSet::new();
    for (index, entry) in events.iter().enumerate() {
        let field = format!("events[{index}]");
        let entry = object(entry, &field)?;
        reject_unknown(entry, EVENT_FIELDS).map_err(|error| format!("`{field}` {error}"))?;
        let name = string(required_in(entry, "name", &field)?, &format!("{field}.name"))?;
        let actions = action_types(required_in(entry, "actions", &field)?, &format!("{field}.actions"))?;
        let view = Arc::new(binding_view(required_in(entry, "globals", &field)?, &format!("{field}.globals"))?);
        let core = optional_string(entry, "core", &field)?;
        let position = optional_string(entry, "position", &field)?;
        let role = optional_string(entry, "role", &field)?;
        match core {
            Some(core) => {
                let Some(event) = CoreEvent::ALL.into_iter().find(|event| event.darkmatter_name() == core) else {
                    return Err(format!("has `{field}.core` `{core}`, which is not a Darkmatter event"));
                };
                if !cores.insert(event) {
                    return Err(format!("lists Darkmatter event `{core}` twice"));
                }
                if position.is_some() || role.is_some() {
                    return Err(format!("gives Darkmatter event `{core}` a position or role at `{field}`"));
                }
                if name != core {
                    registry.rename(event, name).map_err(refused)?;
                }
                registry.declare_core_actions(event, actions).map_err(refused)?;
                registry.declare_core_globals(event, view).map_err(refused)?;
            }
            None => {
                let position: Position = position
                    .ok_or_else(|| format!("has no `{field}.position` for host event `{name}`"))?
                    .parse()
                    .map_err(|error| format!("has `{field}.position`: {error}"))?;
                let role: Role = role
                    .ok_or_else(|| format!("has no `{field}.role` for host event `{name}`"))?
                    .parse()
                    .map_err(|error| format!("has `{field}.role`: {error}"))?;
                registry
                    .register(EventDescriptor::new(name, position, role, view).with_actions(actions))
                    .map_err(refused)?;
            }
        }
    }
    if cores.len() != CoreEvent::ALL.len() {
        return Err("does not list every Darkmatter event".to_string());
    }

    if let Some(pair) = nullable(&root, "outcome_pair")? {
        let pair = object(pair, "outcome_pair")?;
        reject_unknown(pair, PAIR_FIELDS).map_err(|error| format!("`outcome_pair` {error}"))?;
        let succeeded = string(required_in(pair, "succeeded", "outcome_pair")?, "outcome_pair.succeeded")?;
        let failed = string(required_in(pair, "failed", "outcome_pair")?, "outcome_pair.failed")?;
        let mut outcome = OutcomePair::new(succeeded, failed);
        for (index, downgrade) in array(required_in(pair, "downgrades", "outcome_pair")?, "outcome_pair.downgrades")?
            .iter()
            .enumerate()
        {
            let field = format!("outcome_pair.downgrades[{index}]");
            match array(downgrade, &field)?.as_slice() {
                [from, to] => {
                    outcome = outcome.downgrade(string(from, &format!("{field}[0]"))?, string(to, &format!("{field}[1]"))?);
                }
                other => return Err(format!("has `{field}` with {} items instead of two", other.len())),
            }
        }
        registry.register_outcome_pair(outcome).map_err(refused)?;
    }

    for (index, name) in array(required(&root, "host_globals")?, "host_globals")?.iter().enumerate() {
        let name = string(name, &format!("host_globals[{index}]"))?;
        registry.declare_host_global(name.to_string()).map_err(refused)?;
    }
    match required(&root, "global_catalog")? {
        Value::Bool(true) => registry.register_global_catalog(catalog).map_err(refused)?,
        Value::Bool(false) => {}
        other => return Err(format!("has `global_catalog` {} instead of a boolean", describe(other))),
    }
    registry.validate().map_err(refused)?;

    let unknown_verbs = match required(&root, "unknown_verbs")? {
        Value::String(policy) if policy == "reject" => UnknownVerbPolicy::Reject,
        Value::String(policy) if policy == "ignore" => UnknownVerbPolicy::Ignore,
        Value::Object(policy) => {
            reject_unknown(policy, &["ignore_unprovided"]).map_err(|error| format!("`unknown_verbs` {error}"))?;
            let verbs = array(required_in(policy, "ignore_unprovided", "unknown_verbs")?, "unknown_verbs.ignore_unprovided")?
                .iter()
                .enumerate()
                .map(|(index, verb)| string(verb, &format!("unknown_verbs.ignore_unprovided[{index}]")).map(str::to_string))
                .collect::<Result<Vec<String>, String>>()?;
            UnknownVerbPolicy::IgnoreUnprovided(verbs.into())
        }
        other => {
            return Err(format!(
                "has `unknown_verbs` {}; expected `reject`, `ignore`, or `{{ ignore_unprovided: [...] }}`",
                describe(other)
            ));
        }
    };
    let effects: BTreeSet<String> = array(required(&root, "effects")?, "effects")?
        .iter()
        .enumerate()
        .map(|(index, name)| string(name, &format!("effects[{index}]")).map(effect_key))
        .collect::<Result<_, _>>()?;

    Ok(HostLifecycleProfile {
        registry: Arc::new(registry),
        unknown_verbs,
        is_known_effect: Arc::new(move |name: &str| effects.contains(&effect_key(name))),
    })
}

/// A field whose explicit `null` means "none" and whose other values are
/// strings; absence is an error.
fn optional_string<'v>(entry: &'v Map<String, Value>, key: &str, owner: &str) -> Result<Option<&'v str>, String> {
    match entry.get(key) {
        None => Err(format!("has no `{owner}.{key}` field (write `null` for none)")),
        Some(Value::Null) => Ok(None),
        Some(value) => string(value, &format!("{owner}.{key}")).map(Some),
    }
}

fn action_types(value: &Value, field: &str) -> Result<ActionTypes, String> {
    let mut types = Vec::new();
    for (index, item) in array(value, field)?.iter().enumerate() {
        let name = string(item, &format!("{field}[{index}]"))?;
        let Some((action, _)) = ACTION_TYPES.iter().find(|(_, spelling)| *spelling == name) else {
            return Err(format!(
                "has `{field}[{index}]` `{name}`; expected communicate, expression, flow, or shell"
            ));
        };
        types.push(*action);
    }
    Ok(ActionTypes::of(types))
}

fn binding_view(value: &Value, field: &str) -> Result<BindingView, String> {
    let view = object(value, field)?;
    reject_unknown(view, VIEW_FIELDS).map_err(|error| format!("`{field}` {error}"))?;
    let scope = string(required_in(view, "scope", field)?, &format!("{field}.scope"))?;
    let mut builder = BindingView::builder(ScopeId::new(scope));
    let declarations_field = format!("{field}.declarations");
    for (index, declaration) in array(required_in(view, "declarations", field)?, &declarations_field)?.iter().enumerate() {
        let entry_field = format!("{declarations_field}[{index}]");
        let declaration = object(declaration, &entry_field)?;
        reject_unknown(declaration, DECLARATION_FIELDS).map_err(|error| format!("`{entry_field}` {error}"))?;
        let root = string(required_in(declaration, "root", &entry_field)?, &format!("{entry_field}.root"))?;
        let description =
            string(required_in(declaration, "description", &entry_field)?, &format!("{entry_field}.description"))?;
        let availability_field = format!("{entry_field}.availability");
        let availability = match required_in(declaration, "availability", &entry_field)? {
            Value::String(text) if text == "available" => Availability::Available,
            Value::String(text) if text == "execution-dependent" => Availability::ExecutionDependent,
            Value::Object(unavailable) => {
                reject_unknown(unavailable, &["unavailable"]).map_err(|error| format!("`{availability_field}` {error}"))?;
                let reason_field = format!("{availability_field}.unavailable");
                let reason = object(required_in(unavailable, "unavailable", &availability_field)?, &reason_field)?;
                reject_unknown(reason, REASON_FIELDS).map_err(|error| format!("`{reason_field}` {error}"))?;
                let code = string(required_in(reason, "code", &reason_field)?, &format!("{reason_field}.code"))?;
                let parameters = object(required_in(reason, "parameters", &reason_field)?, &format!("{reason_field}.parameters"))?;
                let mut built = UnavailabilityReason::new(code);
                for (key, value) in parameters {
                    built = built.with_parameter(key.clone(), value.clone());
                }
                Availability::Unavailable(built)
            }
            other => {
                return Err(format!(
                    "has `{availability_field}` {}; expected `available`, `execution-dependent`, or `{{ unavailable: … }}`",
                    describe(other)
                ));
            }
        };
        builder = builder.declare(root, description, availability);
    }
    builder.build().map_err(|error| format!("has `{field}` that does not build: {error}"))
}

#[cfg(test)]
mod tests;
