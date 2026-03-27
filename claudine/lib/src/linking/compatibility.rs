use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::error::{ClaudineError, Result};
use crate::events::Provider;
use biscuit_file::serde_yaml_ng;

use super::capabilities::{ALL_PROVIDERS, LinkableResource, capabilities_for};
use super::detector::DiscoveredResource;
use super::model::{IncompleteCause, ResourceDefinition, ResourceReference, ResourceScope};

#[derive(Debug, Clone)]
pub(crate) struct ParsedMarkdown {
    pub(crate) frontmatter: serde_yaml_ng::Mapping,
    pub(crate) body: String,
    pub(crate) had_frontmatter: bool,
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
        return ResourceReference::IncompleteLink(
            target_provider,
            scope,
            IncompleteCause::NoCanonicalDefinition,
        );
    };

    let capabilities = capabilities_for(target_provider);
    let support = capabilities.support_for(resource);
    if !support.level.allows_custom() {
        return ResourceReference::IncompleteLink(
            target_provider,
            scope,
            IncompleteCause::CustomNotSupported,
        );
    }

    let missing: Vec<String> = support
        .required_properties()
        .iter()
        .filter(|required| {
            !property_is_satisfied(required, &definition.frontmatter, &definition.body)
        })
        .map(|s| s.to_string())
        .collect();

    if !missing.is_empty() {
        ResourceReference::IncompleteLink(
            target_provider,
            scope,
            IncompleteCause::MissingProperties(missing),
        )
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
        LinkableResource::Script => {
            if path.is_file() {
                Some(path.to_path_buf())
            } else {
                None
            }
        }
    }
}

pub(crate) fn parse_markdown_document(content: &str) -> Result<ParsedMarkdown> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return Ok(ParsedMarkdown {
            frontmatter: serde_yaml_ng::Mapping::new(),
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

fn parse_frontmatter_mapping(raw: &str) -> Result<serde_yaml_ng::Mapping> {
    if raw.trim().is_empty() {
        return Ok(serde_yaml_ng::Mapping::new());
    }

    let value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(raw) {
        Ok(value) => value,
        Err(_) => return Ok(parse_frontmatter_lines(raw)),
    };
    match value {
        serde_yaml_ng::Value::Mapping(mapping) => Ok(mapping),
        serde_yaml_ng::Value::Null => Ok(serde_yaml_ng::Mapping::new()),
        _ => Err(ClaudineError::LinkingError(
            "frontmatter must be a YAML mapping".to_string(),
        )),
    }
}

/// Line-by-line fallback for frontmatter that isn't strict YAML.
///
/// Handles values like `argument-hint: [--force] [msg]` where square brackets
/// are literal text, not YAML flow sequences.
fn parse_frontmatter_lines(raw: &str) -> serde_yaml_ng::Mapping {
    let mut mapping = serde_yaml_ng::Mapping::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(": ") {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() && !key.contains(' ') {
                mapping.insert(
                    serde_yaml_ng::Value::String(key.to_string()),
                    serde_yaml_ng::Value::String(value.to_string()),
                );
            }
        }
    }
    mapping
}

fn write_markdown_document(path: &Path, parsed: &ParsedMarkdown) -> Result<()> {
    let yaml = serde_yaml_ng::to_string(&parsed.frontmatter)?;
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

fn apply_name_derivation(
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

fn mapping_to_string_map(mapping: &serde_yaml_ng::Mapping) -> BTreeMap<String, String> {
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

/// Frontmatter properties that are non-portable across providers. Resources
/// containing any of these should not be symlinked into non-Claude provider
/// directories.
///
/// - `model`: shared property name but values are provider-specific (e.g.,
///   `sonnet` vs `gemini-2.5-pro`). Present in Claude, Gemini, and OpenCode
///   agent schemas but with incompatible value semantics.
/// - `tools`: shared property name but Claude uses a comma-separated string
///   while other providers (OpenCode, KimiCode) expect structured records.
///   Value format mismatch causes parse errors.
/// - `skills`: Claude-only. No other provider has a skill auto-loading
///   mechanism. Unknown key — likely ignored but not guaranteed.
///
/// Note: `allowed-tools` / `allowed_tools` is intentionally excluded. It is
/// Claude-only (used in skills/commands) and other providers simply ignore
/// unrecognized frontmatter keys.
const NON_PORTABLE_PROPERTIES: &[&str] = &["model", "tools", "skills"];

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

    NON_PORTABLE_PROPERTIES
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
    fn script_file_is_classified_as_source() {
        let tmp = TempDir::new().unwrap();
        let script_file = tmp.path().join("run.sh");
        std::fs::write(&script_file, "#!/usr/bin/env bash\necho hi\n").unwrap();

        let candidate = build_candidate("run.sh", script_file, Provider::Codex, false);
        let classified =
            classify_canonical_candidate(LinkableResource::Script, &candidate, ResourceScope::User)
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
            ResourceReference::IncompleteLink(Provider::KimiCode, ResourceScope::User, _)
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

    #[test]
    fn frontmatter_with_yaml_incompatible_brackets_uses_fallback_parser() {
        let content = "---\ndescription: Create commits\nargument-hint: [--force] [commit-message]\n---\nDo the thing.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert!(parsed.had_frontmatter);
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("description".to_string())),
            Some(&serde_yaml_ng::Value::String("Create commits".to_string()))
        );
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
            Some(&serde_yaml_ng::Value::String(
                "[--force] [commit-message]".to_string()
            ))
        );
        assert_eq!(parsed.body, "Do the thing.\n");
    }

    #[test]
    fn single_bracket_value_parsed_as_yaml_sequence() {
        // serde_yaml interprets `[bug-description]` as a flow sequence.
        // This is correct YAML behavior but means the value is a Sequence,
        // not the literal string `[bug-description]`. Downstream consumers
        // (mapping_to_string_map) re-serialize it as `- bug-description`.
        let content = "---\nargument-hint: [bug-description]\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        let hint = parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("argument-hint".to_string()))
            .unwrap();
        assert!(
            hint.is_sequence(),
            "YAML parses [value] as a flow sequence, not a string"
        );
    }

    #[test]
    fn angle_bracket_placeholders_are_plain_strings() {
        let content = "---\nargument-hint: <skill-name>\ndescription: Do something\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
            Some(&serde_yaml_ng::Value::String("<skill-name>".to_string()))
        );
    }

    #[test]
    fn mixed_angle_and_square_brackets_are_plain_strings() {
        // `<source-file> [test-glob]` starts with `<`, so YAML treats
        // the whole value as a plain scalar (the `[` is not at value start).
        let content = "---\nargument-hint: <source-file> [test-glob]\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
            Some(&serde_yaml_ng::Value::String(
                "<source-file> [test-glob]".to_string()
            ))
        );
    }

    #[test]
    fn allowed_tools_with_colons_and_parens_parse_as_strings() {
        let content = "---\nallowed-tools: Bash(git:*), Read\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("allowed-tools".to_string())),
            Some(&serde_yaml_ng::Value::String(
                "Bash(git:*), Read".to_string()
            ))
        );
    }

    #[test]
    fn allowed_tools_with_multiple_colons_parse_as_strings() {
        let content =
            "---\nallowed-tools: Bash(ping :*), Bash(traceroute :*), Bash(dig :*)\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("allowed-tools".to_string())),
            Some(&serde_yaml_ng::Value::String(
                "Bash(ping :*), Bash(traceroute :*), Bash(dig :*)".to_string()
            ))
        );
    }

    #[test]
    fn quoted_field_value_strips_yaml_quotes() {
        let content = "---\nname: \"code-review\"\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("name".to_string())),
            Some(&serde_yaml_ng::Value::String("code-review".to_string()))
        );
    }

    #[test]
    fn single_quoted_brackets_are_plain_strings() {
        let content = "---\nargument-hint: '[--force] [commit-message-hint]'\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("argument-hint".to_string())),
            Some(&serde_yaml_ng::Value::String(
                "[--force] [commit-message-hint]".to_string()
            ))
        );
    }

    #[test]
    fn description_with_nested_quotes_and_parens() {
        let content = "---\ndescription: Evaluate \"drift\" (aka, docs out of sync)\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        let desc = parsed
            .frontmatter
            .get(serde_yaml_ng::Value::String("description".to_string()))
            .unwrap();
        assert!(desc.is_string(), "description should parse as string");
    }

    #[test]
    fn crlf_line_endings_in_frontmatter() {
        let content = "---\r\ndescription: test\r\nname: example\r\n---\r\nBody.\r\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert!(parsed.had_frontmatter);
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("description".to_string())),
            Some(&serde_yaml_ng::Value::String("test".to_string()))
        );
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("name".to_string())),
            Some(&serde_yaml_ng::Value::String("example".to_string()))
        );
    }

    #[test]
    fn bom_prefix_is_stripped_before_parsing() {
        let content = "\u{feff}---\ndescription: test\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert!(parsed.had_frontmatter);
        assert_eq!(
            parsed
                .frontmatter
                .get(serde_yaml_ng::Value::String("description".to_string())),
            Some(&serde_yaml_ng::Value::String("test".to_string()))
        );
    }

    #[test]
    fn no_frontmatter_returns_full_body() {
        let content = "Just a body with no frontmatter.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert!(!parsed.had_frontmatter);
        assert!(parsed.frontmatter.is_empty());
        assert_eq!(parsed.body, content);
    }

    #[test]
    fn empty_frontmatter_returns_empty_mapping() {
        let content = "---\n---\nBody.\n";
        let parsed = parse_markdown_document(content).unwrap();
        assert!(parsed.had_frontmatter);
        assert!(parsed.frontmatter.is_empty());
        assert_eq!(parsed.body, "Body.\n");
    }

    #[test]
    fn fallback_parser_skips_comments_and_empty_lines() {
        let mapping = parse_frontmatter_lines("# comment\n\nname: test\n\ndescription: hello\n");
        assert_eq!(mapping.len(), 2);
        assert_eq!(
            mapping.get(serde_yaml_ng::Value::String("name".to_string())),
            Some(&serde_yaml_ng::Value::String("test".to_string()))
        );
        assert_eq!(
            mapping.get(serde_yaml_ng::Value::String("description".to_string())),
            Some(&serde_yaml_ng::Value::String("hello".to_string()))
        );
    }

    #[test]
    fn fallback_parser_ignores_lines_without_separator() {
        let mapping = parse_frontmatter_lines("name: valid\nno-separator\nother: also-valid\n");
        assert_eq!(mapping.len(), 2);
        assert!(
            mapping
                .get(serde_yaml_ng::Value::String("name".to_string()))
                .is_some()
        );
        assert!(
            mapping
                .get(serde_yaml_ng::Value::String("other".to_string()))
                .is_some()
        );
    }

    #[test]
    fn fallback_parser_rejects_keys_with_spaces() {
        let mapping = parse_frontmatter_lines("good-key: value\nbad key: value\n");
        assert_eq!(mapping.len(), 1);
        assert!(
            mapping
                .get(serde_yaml_ng::Value::String("good-key".to_string()))
                .is_some()
        );
    }

    #[test]
    fn classify_target_reference_captures_missing_property_names() {
        use crate::linking::model::IncompleteCause;

        let tmp = TempDir::new().unwrap();
        let agent_file = tmp.path().join("reviewer.md");
        std::fs::write(
            &agent_file,
            "---\nname: reviewer\ndescription: Reviews changes\n---\nUse strict review standards.\n",
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
        match for_kimi {
            ResourceReference::IncompleteLink(_, _, IncompleteCause::MissingProperties(props)) => {
                assert!(!props.is_empty(), "should capture missing property names");
            }
            ResourceReference::IncompleteLink(_, _, cause) => {
                // CustomNotSupported is also valid if Kimi doesn't support custom agents
                assert!(
                    matches!(cause, IncompleteCause::CustomNotSupported),
                    "unexpected cause: {cause}"
                );
            }
            other => panic!("expected IncompleteLink, got {other:?}"),
        }
    }

    #[test]
    fn claude_specific_detects_model() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        std::fs::write(&file, "---\ndescription: Test\nmodel: sonnet\n---\nBody\n").unwrap();
        let props = claude_specific_properties(&file);
        assert_eq!(props, vec!["model"]);
        assert!(has_claude_specific_properties(&file));
    }

    #[test]
    fn claude_specific_detects_tools() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        std::fs::write(&file, "---\ndescription: Test\ntools: Bash, Read\n---\nBody\n").unwrap();
        let props = claude_specific_properties(&file);
        assert_eq!(props, vec!["tools"]);
    }

    #[test]
    fn claude_specific_detects_skills() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        std::fs::write(&file, "---\ndescription: Test\nskills: rust\n---\nBody\n").unwrap();
        let props = claude_specific_properties(&file);
        assert_eq!(props, vec!["skills"]);
    }

    #[test]
    fn claude_specific_ignores_allowed_tools() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        std::fs::write(
            &file,
            "---\ndescription: Test\nallowed-tools: Bash(git:*), Read\n---\nBody\n",
        )
        .unwrap();
        // allowed-tools is not in the non-portable list — other providers ignore it
        assert!(claude_specific_properties(&file).is_empty());
        assert!(!has_claude_specific_properties(&file));
    }

    #[test]
    fn claude_specific_detects_multiple() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        std::fs::write(
            &file,
            "---\ndescription: Test\nmodel: opus\ntools: Bash\nskills: rust\n---\nBody\n",
        )
        .unwrap();
        let props = claude_specific_properties(&file);
        assert_eq!(props, vec!["model", "tools", "skills"]);
    }

    #[test]
    fn claude_specific_returns_empty_for_shareable() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        std::fs::write(&file, "---\ndescription: A shareable agent\n---\nBody\n").unwrap();
        assert!(claude_specific_properties(&file).is_empty());
        assert!(!has_claude_specific_properties(&file));
    }

    #[test]
    fn claude_specific_returns_empty_for_missing_file() {
        let path = std::path::PathBuf::from("/nonexistent/file.md");
        assert!(claude_specific_properties(&path).is_empty());
        assert!(!has_claude_specific_properties(&path));
    }
}
