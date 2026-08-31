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

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, coerce};
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
    /// Research topic → sidecar path (for the compatibility gate).
    pub sidecars: BTreeMap<String, PathBuf>,
    /// Override field → entry. Empty when the file is absent.
    pub overrides: BTreeMap<String, OverrideEntry>,
}

/// Loads every input for `slug` from the standard layout under `area`.
///
/// `topics` names the research topics the mapping registry consumes; each
/// must exist and validate against its sidecar schema.
/// Relative area paths have the same meaning as their absolute form.
///
/// ## Errors
///
/// Fails loudly when `area` cannot be absolutized; when a roster entry is
/// missing or flagged `skip_research: true`; when YAML cannot be parsed; when
/// a research document or sidecar is missing; or when research frontmatter
/// does not satisfy its sidecar schema.
pub fn load(area: &Path, slug: &str, topics: &[&str]) -> Result<ProviderInputs, GenError> {
    let area = std::path::absolute(area).map_err(|source| GenError::Io {
        path: area.to_path_buf(),
        source,
    })?;
    let roster = load_roster_entry(&area.join("docs/providers.yaml"), slug)?;
    let facts = load_optional_yaml_map(&area.join(format!("docs/providers/facts/{slug}.yaml")))?;
    let overrides =
        load_overrides(&area.join(format!("docs/providers/overrides/{slug}.yaml")))?;
    // Schema resolution otherwise rediscovers the same repository and package
    // area for every research topic in this provider generation pass.
    let schemas = generator_schemas(&area);

    let mut research = BTreeMap::new();
    let mut sidecars = BTreeMap::new();
    for topic in topics {
        let doc_path = area.join(format!("docs/research/{topic}/{slug}.md"));
        let sidecar = area.join(format!("docs/research/{topic}/_schema.yaml"));
        if !sidecar.is_file() {
            return Err(GenError::SidecarMissing { path: sidecar });
        }
        let frontmatter = load_validated_frontmatter_with_api(&doc_path, &schemas)?;
        research.insert((*topic).to_string(), frontmatter);
        sidecars.insert((*topic).to_string(), sidecar);
    }

    Ok(ProviderInputs {
        slug: slug.to_string(),
        roster,
        facts,
        research,
        sidecars,
        overrides,
    })
}

/// Finds the roster entry whose `slug:` key equals `slug`, refusing
/// entries flagged `skip_research: true`.
fn load_roster_entry(path: &Path, slug: &str) -> Result<Value, GenError> {
    let value = read_yaml(path)?;
    let entries = value
        .get("list")
        .and_then(Value::as_array)
        .ok_or_else(|| GenError::Yaml {
            path: path.to_path_buf(),
            message: "expected a top-level `list:` sequence".into(),
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
        .get("list")
        .and_then(Value::as_array)
        .ok_or_else(|| GenError::Yaml {
            path: path.clone(),
            message: "expected a top-level `list:` sequence".into(),
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
/// and a non-empty `reason:`).
fn load_overrides(path: &Path) -> Result<BTreeMap<String, OverrideEntry>, GenError> {
    let raw = load_optional_yaml_map(path)?;
    let mut overrides = BTreeMap::new();
    for (field, entry) in raw {
        let reason = entry
            .get("reason")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .ok_or_else(|| GenError::OverrideMissingReason {
                field: field.clone(),
            })?
            .to_string();
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

/// Reads a research document, validates its frontmatter against the
/// document's `$schema` sidecar, and returns the schema-coerced
/// frontmatter object (so `boolish`/`numberlike` quirks never reach the
/// mapping layer).
pub(crate) fn load_validated_frontmatter(path: &Path) -> Result<Value, GenError> {
    load_validated_frontmatter_with_api(path, &DarkmatterSchemas::new())
}

fn load_validated_frontmatter_with_api(
    path: &Path,
    api: &DarkmatterSchemas,
) -> Result<Value, GenError> {
    let md = Markdown::try_from(path).map_err(|err| GenError::Markdown {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;
    let effective = api
        .effective_for(&md)
        .map_err(|err| GenError::Markdown {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?
        .ok_or_else(|| GenError::Markdown {
            path: path.to_path_buf(),
            message: "research document declares no `$schema`".into(),
        })?;

    let frontmatter = serde_json::to_value(md.frontmatter().as_map()).map_err(|err| {
        GenError::Json {
            message: err.to_string(),
        }
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

    Ok(coerce::coerce_frontmatter(&effective.json_schema, &frontmatter).value)
}

fn generator_schemas(area: &Path) -> DarkmatterSchemas {
    let Some(repository_root) = sniff::filesystem::git::GitRepo::discover(area)
        .ok()
        .flatten()
        .map(|repository| repository.repo_root().to_path_buf())
    else {
        return DarkmatterSchemas::new();
    };
    let context = FileResolutionContext::new(area)
        .with_repository_root(repository_root)
        .with_package_area(area);
    DarkmatterSchemas::new().with_file_resolution_context(context)
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
