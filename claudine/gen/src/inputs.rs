//! Input loading for one provider: roster entry, facts, research
//! frontmatter, overrides.
//!
//! All loading is layout-driven from an *area root* (the `claudine/`
//! package-area directory), so tests can point the same pipeline at a
//! fixture tree:
//!
//! ```text
//! <area>/docs/providers.yaml
//! <area>/docs/providers/facts/<slug>.yaml
//! <area>/docs/providers/overrides/<slug>.yaml
//! <area>/docs/research/<topic>/<slug>.md      (+ sibling _schema.yaml)
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::{DarkmatterSchemas, PropertyDef, SimplifiedSchema, coerce};
use serde_json::Value;

use crate::errors::GenError;

/// One override entry: a catalog-shaped replacement value plus the
/// mandatory human rationale.
#[derive(Debug, Clone)]
pub struct OverrideEntry {
    pub value: Value,
    pub reason: String,
}

/// All loaded inputs for one provider slug.
#[derive(Debug)]
pub struct ProviderInputs {
    pub slug: String,
    /// The provider's roster entry as a JSON object.
    pub roster: Value,
    /// Facts-file key → value. Empty when the file is absent.
    pub facts: BTreeMap<String, Value>,
    /// Research topic → schema-validated, coerced frontmatter object.
    pub research: BTreeMap<String, Value>,
    /// Research topic → the same validated frontmatter exactly as authored,
    /// before Darkmatter's coercion, for a coercion that must tell a wrong
    /// type or an explicit null from a valid value.
    pub research_authored: BTreeMap<String, Value>,
    /// Research topic → sidecar path (for the compatibility gate).
    pub sidecars: BTreeMap<String, PathBuf>,
    /// Override field → entry. Empty when the file is absent.
    pub overrides: BTreeMap<String, OverrideEntry>,
}

/// Loads every input for `slug` from the standard layout under `area`.
///
/// `topics` names the research topics the mapping registry consumes; each
/// must exist and validate against its sidecar schema.
/// A relative `area` is joined onto `snapshot`'s request directory. Research
/// schemas resolve through one context built from `snapshot` rebased at the
/// area, so `&` and `^` anchor in the area's repository.
///
/// ## Errors
///
/// Fails loudly when `area` cannot be absolutized; when the area's
/// file-resolution context cannot be built ([`GenError::ResolutionContext`]);
/// when a roster entry is missing or flagged `skip_research: true`; when YAML
/// cannot be parsed; when a research document or sidecar is missing; when
/// research frontmatter does not satisfy its sidecar schema; or when a
/// document's contract revision has no contract to validate it against.
pub fn load(
    area: &Path,
    slug: &str,
    topics: &[&str],
    snapshot: &RequestSnapshot,
) -> Result<ProviderInputs, GenError> {
    let area = std::path::absolute(snapshot.request_dir().join(area)).map_err(|source| {
        GenError::Io {
            path: area.to_path_buf(),
            source,
        }
    })?;
    let roster = load_roster_entry(&area.join("docs/providers.yaml"), slug)?;
    let facts = load_optional_yaml_map(&area.join(format!("docs/providers/facts/{slug}.yaml")))?;
    let overrides =
        load_overrides(&area.join(format!("docs/providers/overrides/{slug}.yaml")))?;
    // Schema resolution otherwise rediscovers the same repository and package
    // area for every research topic in this provider generation pass.
    let schemas = generator_schemas(&area, snapshot)?;

    let mut research = BTreeMap::new();
    let mut research_authored = BTreeMap::new();
    let mut sidecars = BTreeMap::new();
    for topic in topics {
        let doc_path = area.join(format!("docs/research/{topic}/{slug}.md"));
        let sidecar = area.join(format!("docs/research/{topic}/_schema.yaml"));
        if !sidecar.is_file() {
            return Err(GenError::SidecarMissing { path: sidecar });
        }
        let frontmatter = validate_frontmatter(&doc_path, &schemas)?;
        research.insert((*topic).to_string(), frontmatter.coerced);
        research_authored.insert((*topic).to_string(), frontmatter.authored);
        sidecars.insert((*topic).to_string(), sidecar);
    }

    Ok(ProviderInputs {
        slug: slug.to_string(),
        roster,
        facts,
        research,
        research_authored,
        sidecars,
        overrides,
    })
}

/// Finds the roster entry whose `slug:` key equals `slug`, refusing
/// entries flagged `skip_research: true`.
fn load_roster_entry(path: &Path, slug: &str) -> Result<Value, GenError> {
    let value = read_yaml(path)?;
    let entries = value
        .get("sequence")
        .and_then(Value::as_array)
        .ok_or_else(|| GenError::Yaml {
            path: path.to_path_buf(),
            message: "expected a top-level `sequence:` list".into(),
        })?;
    let entry = entries
        .iter()
        .find(|entry| entry.get("slug").and_then(Value::as_str) == Some(slug))
        .cloned()
        .ok_or_else(|| GenError::RosterEntryMissing {
            slug: slug.to_string(),
            path: path.to_path_buf(),
        })?;
    if entry.get("skip_research").and_then(Value::as_bool) == Some(true) {
        return Err(GenError::RosterEntrySkipped {
            slug: slug.to_string(),
            path: path.to_path_buf(),
        });
    }
    Ok(entry)
}

/// Lists roster slugs whose entry is NOT flagged `skip_research: true`, in
/// roster order.
///
/// Feeds the `check` roster ↔ wired-set cross-validation
/// ([`crate::generate::cross_validate_roster`]): an active slug with no
/// wired `Provider` variant is the "researched but not yet code-supported"
/// set.
pub fn roster_active_slugs(area: &Path) -> Result<Vec<String>, GenError> {
    let path = area.join("docs/providers.yaml");
    let value = read_yaml(&path)?;
    let entries = value
        .get("sequence")
        .and_then(Value::as_array)
        .ok_or_else(|| GenError::Yaml {
            path: path.clone(),
            message: "expected a top-level `sequence:` list".into(),
        })?;
    let mut slugs = Vec::new();
    for entry in entries {
        if entry.get("skip_research").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        if let Some(slug) = entry.get("slug").and_then(Value::as_str) {
            slugs.push(slug.to_string());
        }
    }
    Ok(slugs)
}

/// Loads a YAML mapping file into a key → value map; absent file ⇒ empty.
fn load_optional_yaml_map(path: &Path) -> Result<BTreeMap<String, Value>, GenError> {
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let value = read_yaml(path)?;
    let object = value.as_object().ok_or_else(|| GenError::Yaml {
        path: path.to_path_buf(),
        message: "expected a top-level mapping".into(),
    })?;
    Ok(object
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}

/// Loads and shape-checks the overrides file (every entry needs `value:`
/// and a non-empty `reason:`, and nothing else).
fn load_overrides(path: &Path) -> Result<BTreeMap<String, OverrideEntry>, GenError> {
    let raw = load_optional_yaml_map(path)?;
    let mut overrides = BTreeMap::new();
    for (field, entry) in raw {
        // A misspelled key would otherwise be ignored while its entry still
        // applies.
        if let Some(key) = entry
            .as_object()
            .and_then(|keys| keys.keys().find(|key| !matches!(key.as_str(), "value" | "reason")))
        {
            return Err(GenError::OverrideUnknownKey {
                field,
                key: key.clone(),
            });
        }
        let reason = match entry.get("reason") {
            Some(Value::String(reason)) if !reason.trim().is_empty() => reason.trim().to_string(),
            found => {
                return Err(GenError::OverrideInvalidReason {
                    field,
                    found: describe_reason(&entry, found),
                });
            }
        };
        let value = entry
            .get("value")
            .cloned()
            .ok_or_else(|| GenError::OverrideMissingValue {
                field: field.clone(),
            })?;
        overrides.insert(field, OverrideEntry { value, reason });
    }
    Ok(overrides)
}

/// Names what stood where an override's `reason:` string belongs, so a
/// wrong-type reason is never reported as an absent one.
fn describe_reason(entry: &Value, reason: Option<&Value>) -> &'static str {
    match reason {
        _ if !entry.is_object() => "an entry that is not a mapping",
        None => "no `reason:` key",
        Some(Value::Null) => "null",
        Some(Value::String(_)) => "an empty string",
        Some(Value::Bool(_)) => "a Boolean",
        Some(Value::Number(_)) => "a number",
        Some(Value::Array(_)) => "a list",
        Some(Value::Object(_)) => "a mapping",
    }
}

/// Reads a research document, validates its frontmatter against the
/// document's `$schema` sidecar, and returns the schema-coerced
/// frontmatter object (so `boolish`/`numberlike` quirks never reach the
/// mapping layer).
///
/// The `$schema` reference resolves through `context`, normally the area's
/// context from [`area_resolution_context`]. A document written for an older
/// revision of its topic contract is validated against that revision's frozen
/// sidecar (see [`frozen_contract_override`]).
pub fn load_validated_frontmatter(
    path: &Path,
    context: &FileResolutionContext,
) -> Result<Value, GenError> {
    Ok(validate_frontmatter(path, &DarkmatterSchemas::new(context.clone()))?.coerced)
}

/// One validated research document's frontmatter in both forms.
struct ValidatedFrontmatter {
    coerced: Value,
    authored: Value,
}

fn validate_frontmatter(path: &Path, api: &DarkmatterSchemas) -> Result<ValidatedFrontmatter, GenError> {
    let md = Markdown::try_from(path).map_err(|err| GenError::Markdown {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;
    let frontmatter = serde_json::to_value(md.frontmatter().as_map()).map_err(|err| {
        GenError::Json {
            message: err.to_string(),
        }
    })?;
    let schema_override = frozen_contract_override(path, &frontmatter)?;
    let effective = api
        .effective_for_with_override(&md, schema_override.as_ref())
        .map_err(|err| GenError::Markdown {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?
        .ok_or_else(|| GenError::Markdown {
            path: path.to_path_buf(),
            message: "research document declares no `$schema`".into(),
        })?;

    let report = effective.validate(&frontmatter);
    if !report.valid {
        let problems = report
            .problems
            .iter()
            .map(|p| format!("- {}", p.message))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(GenError::ResearchInvalid {
            path: path.to_path_buf(),
            problems,
        });
    }

    let coerced = coerce::coerce_frontmatter(&effective.json_schema, &frontmatter).value;
    Ok(ValidatedFrontmatter {
        coerced,
        authored: frontmatter,
    })
}

/// The frozen sidecar a document written for an older contract revision
/// validates against, as a `$schema` override; `None` when the document
/// matches its contract's current revision or the contract is unversioned.
///
/// Applies to a document whose `$schema` is `./_schema.yaml` and whose
/// sidecar declares `schema_revision: literal(N; …)`. A document without
/// `schema_revision` is revision 1. Revision M ≠ N validates against
/// `_schema.rM.yaml` beside the sidecar, so a contract change does not
/// break generation while the fleet re-researches; the consumer of the
/// changed fields decides how an older document projects.
///
/// ## Errors
///
/// [`GenError::ResearchRevisionUnsupported`] when no frozen sidecar exists
/// for the document's revision.
fn frozen_contract_override(path: &Path, frontmatter: &Value) -> Result<Option<Value>, GenError> {
    if frontmatter.get("$schema").and_then(Value::as_str) != Some("./_schema.yaml") {
        return Ok(None);
    }
    let sidecar = path.with_file_name("_schema.yaml");
    let Some(current) = contract_revision(&sidecar)? else {
        return Ok(None);
    };
    let found = match frontmatter.get("schema_revision") {
        None => 1,
        Some(value) => match value.as_u64() {
            Some(revision) => revision,
            // A null or non-integer revision is the current contract's to reject.
            None => return Ok(None),
        },
    };
    if found == current {
        return Ok(None);
    }
    let frozen = format!("_schema.r{found}.yaml");
    if !path.with_file_name(&frozen).is_file() {
        return Err(GenError::ResearchRevisionUnsupported {
            path: path.to_path_buf(),
            found,
            current,
        });
    }
    Ok(Some(Value::String(format!("./{frozen}"))))
}

/// The `literal` revision a sidecar's `schema_revision` property pins, if any.
fn contract_revision(sidecar: &Path) -> Result<Option<u64>, GenError> {
    let SimplifiedSchema::Single(shape) = crate::schema_compat::load_sidecar_schema(sidecar)? else {
        return Ok(None);
    };
    Ok(match shape.properties.get("schema_revision") {
        Some(PropertyDef::Single(atom)) => atom.literal_value().and_then(Value::as_u64),
        _ => None,
    })
}

fn generator_schemas(area: &Path, snapshot: &RequestSnapshot) -> Result<DarkmatterSchemas, GenError> {
    Ok(DarkmatterSchemas::new(area_resolution_context(area, snapshot)?))
}

/// Builds the file-resolution context for `area` from `snapshot` rebased at
/// the area, so `&` and `^` anchor in the area's repository.
///
/// ## Errors
///
/// Returns [`GenError::ResolutionContext`] when the context cannot be built.
pub fn area_resolution_context(
    area: &Path,
    snapshot: &RequestSnapshot,
) -> Result<FileResolutionContext, GenError> {
    build_resolution_context(&snapshot.at_request_dir(area)).map_err(|source| {
        GenError::ResolutionContext {
            area: area.to_path_buf(),
            source: Box::new(source),
        }
    })
}

/// Parses a YAML file into a `serde_json::Value`.
pub(crate) fn read_yaml(path: &Path) -> Result<Value, GenError> {
    let text = fs::read_to_string(path).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&text).map_err(|err| GenError::Yaml {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
    serde_json::to_value(yaml).map_err(|err| GenError::Yaml {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}
