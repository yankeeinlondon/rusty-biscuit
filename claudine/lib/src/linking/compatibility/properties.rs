//! Property satisfaction, alias duplication, name derivation, and the YAML
//! value/string helpers — the frontmatter-property half of the compatibility
//! layer, plus Claude-specific property detection.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use biscuit_file::serde_yaml_ng;

use crate::linking::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use crate::linking::detector::DiscoveredResource;

use super::classify::{inferred_name, valid_slug};
use super::frontmatter_io::parse_markdown_document;
use super::table;

pub(super) fn apply_alias_duplication(
    resource: LinkableResource,
    frontmatter: &mut serde_yaml_ng::Mapping,
) -> bool {
    let mut changed = false;

    for aliases in property_alias_groups(resource) {
        let source = aliases.iter().find_map(|alias| {
            get_frontmatter_value(frontmatter, alias)
                .filter(|value| yaml_value_has_data(value))
                .cloned()
        });

        let Some(source_value) = source else {
            continue;
        };

        for alias in aliases {
            if !frontmatter_has_value(frontmatter, &alias) {
                frontmatter.insert(serde_yaml_ng::Value::String(alias), source_value.clone());
                changed = true;
            }
        }
    }

    changed
}

pub(super) fn apply_name_derivation(
    resource: LinkableResource,
    candidate: &DiscoveredResource,
    canonical_file: &Path,
    frontmatter: &mut serde_yaml_ng::Mapping,
) -> bool {
    if frontmatter_has_value(frontmatter, "name") {
        return false;
    }

    let inferred = inferred_name(resource, candidate, canonical_file);
    let Some(name) = inferred.filter(|name| valid_slug(name)) else {
        return false;
    };

    frontmatter.insert(
        serde_yaml_ng::Value::String("name".to_string()),
        serde_yaml_ng::Value::String(name),
    );
    true
}

fn property_alias_groups(resource: LinkableResource) -> Vec<Vec<String>> {
    let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for provider in ALL_PROVIDERS {
        let capabilities = capabilities_for(provider);
        let support = capabilities.support_for(resource);
        if !support.level.allows_custom() {
            continue;
        }

        for property in support
            .required_properties()
            .iter()
            .chain(support.optional_properties().iter())
        {
            grouped
                .entry(normalize_key(property))
                .or_default()
                .insert((*property).to_string());
        }
    }

    grouped
        .into_values()
        .filter(|values| values.len() > 1)
        .map(|values| values.into_iter().collect::<Vec<_>>())
        .collect()
}

pub(super) fn missing_required_for_all_providers(
    resource: LinkableResource,
    frontmatter: &BTreeMap<String, String>,
    body: &str,
) -> Vec<String> {
    let mut required = BTreeSet::new();
    for provider in ALL_PROVIDERS {
        let capabilities = capabilities_for(provider);
        let support = capabilities.support_for(resource);
        if !support.level.allows_custom() {
            continue;
        }
        for key in support.required_properties() {
            required.insert((*key).to_string());
        }
    }

    required
        .into_iter()
        .filter(|property| !property_is_satisfied(property, frontmatter, body))
        .collect()
}

pub(super) fn property_is_satisfied(
    property: &str,
    frontmatter: &BTreeMap<String, String>,
    body: &str,
) -> bool {
    if property.eq_ignore_ascii_case("prompt") && !body.trim().is_empty() {
        return true;
    }

    let normalized = normalize_key(property);
    frontmatter
        .iter()
        .any(|(key, value)| normalize_key(key) == normalized && !value.trim().is_empty())
}

pub(super) fn mapping_to_string_map(mapping: &serde_yaml_ng::Mapping) -> BTreeMap<String, String> {
    mapping
        .iter()
        .filter_map(|(key, value)| {
            let key = key.as_str()?.to_string();
            let value = yaml_value_to_string(value);
            Some((key, value))
        })
        .collect()
}

fn yaml_value_to_string(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::Null => String::new(),
        serde_yaml_ng::Value::Bool(value) => value.to_string(),
        serde_yaml_ng::Value::Number(value) => value.to_string(),
        serde_yaml_ng::Value::String(value) => value.clone(),
        serde_yaml_ng::Value::Sequence(_)
        | serde_yaml_ng::Value::Mapping(_)
        | serde_yaml_ng::Value::Tagged(_) => {
            let rendered = serde_yaml_ng::to_string(value).unwrap_or_default();
            rendered
                .strip_prefix("---\n")
                .unwrap_or(&rendered)
                .trim()
                .to_string()
        }
    }
}

fn get_frontmatter_value<'a>(
    frontmatter: &'a serde_yaml_ng::Mapping,
    key: &str,
) -> Option<&'a serde_yaml_ng::Value> {
    frontmatter.get(serde_yaml_ng::Value::String(key.to_string()))
}

fn frontmatter_has_value(frontmatter: &serde_yaml_ng::Mapping, key: &str) -> bool {
    get_frontmatter_value(frontmatter, key)
        .map(yaml_value_has_data)
        .unwrap_or(false)
}

fn yaml_value_has_data(value: &serde_yaml_ng::Value) -> bool {
    match value {
        serde_yaml_ng::Value::Null => false,
        serde_yaml_ng::Value::String(value) => !value.trim().is_empty(),
        serde_yaml_ng::Value::Sequence(values) => !values.is_empty(),
        serde_yaml_ng::Value::Mapping(values) => !values.is_empty(),
        serde_yaml_ng::Value::Tagged(tagged) => yaml_value_has_data(&tagged.value),
        serde_yaml_ng::Value::Bool(_) | serde_yaml_ng::Value::Number(_) => true,
    }
}

pub(super) fn hash_frontmatter(frontmatter: &BTreeMap<String, String>) -> u64 {
    let mut bytes = Vec::new();
    for (key, value) in frontmatter {
        bytes.extend_from_slice(key.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
    }
    biscuit_hash::xx_hash_bytes(&bytes)
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

/// Check whether a markdown file contains Claude-specific frontmatter properties
/// that make it unsafe to share with other providers via symlink.
///
/// Returns the list of Claude-specific property names found, or an empty vec
/// if the file is shareable.
pub(crate) fn claude_specific_properties(path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    table::NON_PORTABLE_PROPERTIES
        .iter()
        .filter(|prop| {
            parsed
                .frontmatter
                .contains_key(serde_yaml_ng::Value::String(prop.to_string()))
        })
        .map(|prop| prop.to_string())
        .collect()
}

/// Returns true if the file contains any Claude-specific frontmatter properties.
pub(crate) fn has_claude_specific_properties(path: &Path) -> bool {
    !claude_specific_properties(path).is_empty()
}
