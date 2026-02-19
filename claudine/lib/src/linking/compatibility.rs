use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::error::{ClaudineError, Result};
use crate::events::Provider;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use super::detector::DiscoveredResource;
use super::model::{ResourceDefinition, ResourceReference, ResourceScope};

#[derive(Debug, Clone)]
struct ParsedMarkdown {
    frontmatter: serde_yaml::Mapping,
    body: String,
    had_frontmatter: bool,
}

/// Parse a canonical markdown candidate, apply deterministic compatibility upgrades,
/// and classify it as `Source` or `PartialSource`.
pub fn classify_canonical_candidate(
    resource: LinkableResource,
    candidate: &DiscoveredResource,
    scope: ResourceScope,
) -> Result<ResourceReference> {
    let canonical_file = canonical_file_path(resource, &candidate.path).ok_or_else(|| {
        ClaudineError::LinkingError(format!(
            "no canonical markdown entrypoint for resource {:?} at {}",
            resource,
            candidate.path.display()
        ))
    })?;

    let content = std::fs::read_to_string(&canonical_file)?;
    let mut parsed = parse_markdown_document(&content)?;

    let mut changed = apply_alias_duplication(resource, &mut parsed.frontmatter);
    changed |= apply_name_derivation(
        resource,
        candidate,
        &canonical_file,
        &mut parsed.frontmatter,
    );

    if changed {
        write_markdown_document(&canonical_file, &parsed)?;
    }

    let frontmatter = mapping_to_string_map(&parsed.frontmatter);
    let missing_required = missing_required_for_all_providers(resource, &frontmatter, &parsed.body);
    let definition = ResourceDefinition {
        name: candidate.name.clone(),
        provider: candidate.provider,
        scope,
        filepath: canonical_file,
        fm_hash: hash_frontmatter(&frontmatter),
        body_hash: biscuit_hash::xx_hash_bytes(parsed.body.as_bytes()),
        frontmatter,
        body: parsed.body,
    };

    if missing_required.is_empty() {
        Ok(ResourceReference::Source(definition))
    } else {
        Ok(ResourceReference::PartialSource(
            definition,
            missing_required,
        ))
    }
}

/// Classify whether a target provider can consume a canonical source as a direct link.
///
/// Returns `LinkMissing` when requirements are met, and `IncompleteLink` when
/// target required properties cannot be satisfied from the canonical source.
pub fn classify_target_reference(
    resource: LinkableResource,
    canonical_source: &ResourceReference,
    target_provider: Provider,
    scope: ResourceScope,
) -> ResourceReference {
    let Some(definition) = canonical_definition(canonical_source) else {
        return ResourceReference::IncompleteLink(target_provider, scope);
    };

    let capabilities = capabilities_for(target_provider);
    let support = capabilities.support_for(resource);
    if !support.level.allows_custom() {
        return ResourceReference::IncompleteLink(target_provider, scope);
    }

    let unmet = support
        .required_properties()
        .iter()
        .filter(|required| {
            !property_is_satisfied(required, &definition.frontmatter, &definition.body)
        })
        .count();

    if unmet > 0 {
        ResourceReference::IncompleteLink(target_provider, scope)
    } else {
        ResourceReference::LinkMissing(target_provider, scope)
    }
}

fn canonical_definition(reference: &ResourceReference) -> Option<&ResourceDefinition> {
    match reference {
        ResourceReference::Source(definition) | ResourceReference::PartialSource(definition, _) => {
            Some(definition)
        }
        _ => None,
    }
}

fn canonical_file_path(resource: LinkableResource, path: &Path) -> Option<PathBuf> {
    match resource {
        LinkableResource::Skill => {
            if path.is_dir() {
                Some(path.join("SKILL.md"))
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("SKILL.md"))
                .unwrap_or(false)
            {
                Some(path.to_path_buf())
            } else {
                None
            }
        }
        LinkableResource::Command | LinkableResource::Agent => {
            if path.is_file() {
                Some(path.to_path_buf())
            } else {
                None
            }
        }
        LinkableResource::Script => None,
    }
}

fn parse_markdown_document(content: &str) -> Result<ParsedMarkdown> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return Ok(ParsedMarkdown {
            frontmatter: serde_yaml::Mapping::new(),
            body: content.to_string(),
            had_frontmatter: false,
        });
    };

    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if trimmed == "---" {
            let frontmatter_raw = &rest[..offset];
            let body = &rest[offset + line.len()..];
            let frontmatter = parse_frontmatter_mapping(frontmatter_raw)?;
            return Ok(ParsedMarkdown {
                frontmatter,
                body: body.to_string(),
                had_frontmatter: true,
            });
        }
        offset += line.len();
    }

    Err(ClaudineError::LinkingError(
        "unclosed YAML frontmatter delimiter".to_string(),
    ))
}

fn parse_frontmatter_mapping(raw: &str) -> Result<serde_yaml::Mapping> {
    if raw.trim().is_empty() {
        return Ok(serde_yaml::Mapping::new());
    }

    let value: serde_yaml::Value = serde_yaml::from_str(raw)?;
    match value {
        serde_yaml::Value::Mapping(mapping) => Ok(mapping),
        serde_yaml::Value::Null => Ok(serde_yaml::Mapping::new()),
        _ => Err(ClaudineError::LinkingError(
            "frontmatter must be a YAML mapping".to_string(),
        )),
    }
}

fn write_markdown_document(path: &Path, parsed: &ParsedMarkdown) -> Result<()> {
    let yaml = serde_yaml::to_string(&parsed.frontmatter)?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    let yaml = yaml.trim_end_matches('\n');

    let rendered = if parsed.frontmatter.is_empty() && !parsed.had_frontmatter {
        parsed.body.clone()
    } else if yaml.is_empty() {
        format!("---\n---\n{}", parsed.body)
    } else {
        format!("---\n{yaml}\n---\n{}", parsed.body)
    };

    std::fs::write(path, rendered)?;
    Ok(())
}

fn apply_alias_duplication(
    resource: LinkableResource,
    frontmatter: &mut serde_yaml::Mapping,
) -> bool {
    let mut changed = false;

    for aliases in property_alias_groups(resource) {
        let source = aliases
            .iter()
            .find_map(|alias| {
                get_frontmatter_value(frontmatter, alias)
                    .filter(|value| yaml_value_has_data(value))
                    .cloned()
            });

        let Some(source_value) = source else {
            continue;
        };

        for alias in aliases {
            if !frontmatter_has_value(frontmatter, &alias) {
                frontmatter.insert(serde_yaml::Value::String(alias), source_value.clone());
                changed = true;
            }
        }
    }

    changed
}

fn apply_name_derivation(
    resource: LinkableResource,
    candidate: &DiscoveredResource,
    canonical_file: &Path,
    frontmatter: &mut serde_yaml::Mapping,
) -> bool {
    if frontmatter_has_value(frontmatter, "name") {
        return false;
    }

    let inferred = inferred_name(resource, candidate, canonical_file);
    let Some(name) = inferred.filter(|name| valid_slug(name)) else {
        return false;
    };

    frontmatter.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(name),
    );
    true
}

fn inferred_name(
    resource: LinkableResource,
    candidate: &DiscoveredResource,
    canonical_file: &Path,
) -> Option<String> {
    match resource {
        LinkableResource::Skill => candidate
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string),
        LinkableResource::Command | LinkableResource::Agent => canonical_file
            .file_stem()
            .and_then(|name| name.to_str())
            .map(ToString::to_string),
        LinkableResource::Script => None,
    }
}

fn valid_slug(name: &str) -> bool {
    let mut chars = name.chars().peekable();
    if chars.peek().is_none() {
        return false;
    }

    let mut previous_dash = false;
    for ch in chars {
        let is_valid_char = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-';
        if !is_valid_char {
            return false;
        }
        if ch == '-' {
            if previous_dash {
                return false;
            }
            previous_dash = true;
        } else {
            previous_dash = false;
        }
    }

    !name.starts_with('-') && !name.ends_with('-')
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

fn missing_required_for_all_providers(
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

fn property_is_satisfied(
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

fn mapping_to_string_map(mapping: &serde_yaml::Mapping) -> BTreeMap<String, String> {
    mapping
        .iter()
        .filter_map(|(key, value)| {
            let key = key.as_str()?.to_string();
            let value = yaml_value_to_string(value);
            Some((key, value))
        })
        .collect()
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null => String::new(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::String(value) => value.clone(),
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => {
            let rendered = serde_yaml::to_string(value).unwrap_or_default();
            rendered
                .strip_prefix("---\n")
                .unwrap_or(&rendered)
                .trim()
                .to_string()
        }
    }
}

fn get_frontmatter_value<'a>(
    frontmatter: &'a serde_yaml::Mapping,
    key: &str,
) -> Option<&'a serde_yaml::Value> {
    frontmatter.get(serde_yaml::Value::String(key.to_string()))
}

fn frontmatter_has_value(frontmatter: &serde_yaml::Mapping, key: &str) -> bool {
    get_frontmatter_value(frontmatter, key)
        .map(yaml_value_has_data)
        .unwrap_or(false)
}

fn yaml_value_has_data(value: &serde_yaml::Value) -> bool {
    match value {
        serde_yaml::Value::Null => false,
        serde_yaml::Value::String(value) => !value.trim().is_empty(),
        serde_yaml::Value::Sequence(values) => !values.is_empty(),
        serde_yaml::Value::Mapping(values) => !values.is_empty(),
        serde_yaml::Value::Tagged(tagged) => yaml_value_has_data(&tagged.value),
        serde_yaml::Value::Bool(_) | serde_yaml::Value::Number(_) => true,
    }
}

fn hash_frontmatter(frontmatter: &BTreeMap<String, String>) -> u64 {
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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn build_candidate(
        name: &str,
        path: PathBuf,
        provider: Provider,
        is_symlink: bool,
    ) -> DiscoveredResource {
        DiscoveredResource {
            name: name.to_string(),
            path,
            provider,
            is_symlink,
            hash: None,
        }
    }

    #[test]
    fn skill_name_is_derived_and_written_in_place() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("build-pipeline");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: Build release artifacts\n---\n# Build\n",
        )
        .unwrap();

        let candidate =
            build_candidate("build-pipeline", skill_dir.clone(), Provider::Claude, false);
        let classified =
            classify_canonical_candidate(LinkableResource::Skill, &candidate, ResourceScope::User)
                .unwrap();

        match classified {
            ResourceReference::Source(definition) => {
                assert_eq!(
                    definition.frontmatter.get("name"),
                    Some(&"build-pipeline".to_string())
                );
            }
            other => panic!("expected Source, got {other:?}"),
        }

        let upgraded = std::fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(upgraded.contains("name: build-pipeline"));
    }

    #[test]
    fn missing_required_properties_mark_source_as_partial() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("missing-description");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\n---\n# Body\n").unwrap();

        let candidate = build_candidate("missing-description", skill_dir, Provider::Claude, false);
        let classified =
            classify_canonical_candidate(LinkableResource::Skill, &candidate, ResourceScope::User)
                .unwrap();

        match classified {
            ResourceReference::PartialSource(_, missing) => {
                assert!(missing.contains(&"description".to_string()));
            }
            other => panic!("expected PartialSource, got {other:?}"),
        }
    }

    #[test]
    fn alias_duplication_adds_equivalent_keys() {
        let tmp = TempDir::new().unwrap();
        let agent_file = tmp.path().join("security-agent.md");
        std::fs::write(
            &agent_file,
            "---\nname: security-agent\ndescription: Security reviewer\nmax_turns: 8\n---\nReview changes for security defects.\n",
        )
        .unwrap();

        let candidate = build_candidate(
            "security-agent",
            agent_file.clone(),
            Provider::Claude,
            false,
        );
        let classified =
            classify_canonical_candidate(LinkableResource::Agent, &candidate, ResourceScope::User)
                .unwrap();

        let definition = match classified {
            ResourceReference::Source(definition)
            | ResourceReference::PartialSource(definition, _) => definition,
            other => panic!("expected Source/PartialSource, got {other:?}"),
        };

        assert_eq!(
            definition.frontmatter.get("max_turns"),
            Some(&"8".to_string())
        );
        assert_eq!(
            definition.frontmatter.get("maxTurns"),
            Some(&"8".to_string())
        );
    }

    #[test]
    fn markdown_command_body_satisfies_prompt_requirement() {
        let tmp = TempDir::new().unwrap();
        let command_file = tmp.path().join("run-tests.md");
        std::fs::write(&command_file, "Run unit tests with coverage.").unwrap();

        let candidate = build_candidate("run-tests", command_file, Provider::Claude, false);
        let classified = classify_canonical_candidate(
            LinkableResource::Command,
            &candidate,
            ResourceScope::User,
        )
        .unwrap();

        assert!(matches!(classified, ResourceReference::Source(_)));
    }

    #[test]
    fn target_links_become_incomplete_when_provider_requirements_unmet() {
        let tmp = TempDir::new().unwrap();
        let agent_file = tmp.path().join("reviewer.md");
        std::fs::write(
            &agent_file,
            "---\nname: reviewer\ndescription: Reviews pull requests\n---\nFocus on maintainability.\n",
        )
        .unwrap();

        let candidate = build_candidate("reviewer", agent_file, Provider::Claude, false);
        let canonical =
            classify_canonical_candidate(LinkableResource::Agent, &candidate, ResourceScope::User)
                .unwrap();

        let for_kimi = classify_target_reference(
            LinkableResource::Agent,
            &canonical,
            Provider::KimiCode,
            ResourceScope::User,
        );
        assert!(matches!(
            for_kimi,
            ResourceReference::IncompleteLink(Provider::KimiCode, ResourceScope::User)
        ));

        let for_opencode = classify_target_reference(
            LinkableResource::Agent,
            &canonical,
            Provider::OpenCode,
            ResourceScope::User,
        );
        assert!(matches!(
            for_opencode,
            ResourceReference::LinkMissing(Provider::OpenCode, ResourceScope::User)
        ));
    }
}
