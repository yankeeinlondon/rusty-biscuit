use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::events::Provider;

use super::capabilities::{LinkableResource, ResourceFormat, capabilities_for};
use super::compatibility::{classify_canonical_candidate, classify_target_reference};
use super::detector::{
    AgentDefinitionsDetector, DiscoveredResource, LinkDetector, SharedScriptsDetector,
    SkillsDetector, SlashCommandsDetector,
};
use super::model::{
    Resource, ResourceDefinition, ResourceFormatConversion, ResourceReference,
    ResourceScope as ModelScope, ResourceType,
};
use super::paths::{ProviderSkillPaths, ResourceScope as PathScope};
use super::symlink::{LinkResult, category_link_target, create_skill_link, relative_path};

const DERIVED_FM_HASH_KEY: &str = "_claudine_fm_hash";
const DERIVED_BODY_HASH_KEY: &str = "_claudine_body_hash";

/// Apply execution results for fixable states.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ApplySummary {
    /// Number of direct links created or confirmed.
    pub links_created: usize,
    /// Number of derived artifacts generated or regenerated.
    pub derived_written: usize,
    /// Number of operations skipped due safety or incompatibility checks.
    pub skipped: usize,
}

struct DetectionSnapshot {
    providers: Vec<Provider>,
    canonical_by_name: HashMap<String, DiscoveredResource>,
    resources_by_provider_name: HashMap<(Provider, String), DiscoveredResource>,
    names: Vec<String>,
}

/// Analyze provider resource states for one resource type and scope.
pub fn analyze_resource_links(
    paths: &ProviderSkillPaths,
    resource: LinkableResource,
    scope: PathScope,
    canonical_provider: Provider,
) -> Result<Vec<Resource>> {
    let snapshot = detect_snapshot(paths, resource, scope, canonical_provider)?;
    let model_scope = to_model_scope(scope);
    let kind = to_resource_type(resource);

    let mut analyzed = Vec::new();
    for name in &snapshot.names {
        let canonical = snapshot.canonical_by_name.get(name);
        let Some(canonical) = canonical else {
            for provider in &snapshot.providers {
                if let Some(existing) = snapshot
                    .resources_by_provider_name
                    .get(&(*provider, name.clone()))
                {
                    analyzed.push(Resource {
                        name: name.clone(),
                        kind,
                        scope: model_scope,
                        provider: *provider,
                        definition: ResourceReference::Isolated(isolated_definition(
                            existing,
                            model_scope,
                        )),
                    });
                }
            }
            continue;
        };

        let canonical_ref = classify_canonical_candidate(resource, canonical, model_scope)?;
        analyzed.push(Resource {
            name: name.clone(),
            kind,
            scope: model_scope,
            provider: canonical.provider,
            definition: canonical_ref.clone(),
        });

        for provider in &snapshot.providers {
            if *provider == canonical.provider {
                continue;
            }

            let existing = snapshot
                .resources_by_provider_name
                .get(&(*provider, name.clone()));
            let state = classify_target_state(
                resource,
                scope,
                model_scope,
                &canonical_ref,
                *provider,
                existing,
            );
            analyzed.push(Resource {
                name: name.clone(),
                kind,
                scope: model_scope,
                provider: *provider,
                definition: state,
            });
        }
    }

    analyzed.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.provider.as_slug().cmp(right.provider.as_slug()))
    });

    Ok(analyzed)
}

/// Apply fixable link/derived states for a previously analyzed resource set.
///
/// The input should come from [`analyze_resource_links`] for the same
/// `(resource, scope)` tuple.
pub fn apply_fixable_resources(
    paths: &ProviderSkillPaths,
    resource: LinkableResource,
    scope: PathScope,
    resources: &[Resource],
) -> Result<ApplySummary> {
    let mut summary = ApplySummary::default();
    let mut canonical_by_name: HashMap<&str, &ResourceDefinition> = HashMap::new();

    for item in resources {
        match &item.definition {
            ResourceReference::Source(definition)
            | ResourceReference::PartialSource(definition, _) => {
                canonical_by_name
                    .entry(item.name.as_str())
                    .or_insert(definition);
            }
            _ => {}
        }
    }

    for item in resources {
        let Some(canonical) = canonical_by_name.get(item.name.as_str()) else {
            continue;
        };

        match item.definition {
            ResourceReference::LinkMissing(provider, _) => {
                if apply_direct_link(paths, resource, scope, provider, canonical)? {
                    summary.links_created += 1;
                } else {
                    summary.skipped += 1;
                }
            }
            ResourceReference::DerivedMissing(provider, _)
            | ResourceReference::DerivedStale(provider, _) => {
                if apply_derived_write(paths, resource, scope, provider, canonical, &item.name)? {
                    summary.derived_written += 1;
                } else {
                    summary.skipped += 1;
                }
            }
            _ => {}
        }
    }

    Ok(summary)
}

fn detect_snapshot(
    paths: &ProviderSkillPaths,
    resource: LinkableResource,
    scope: PathScope,
    canonical_provider: Provider,
) -> Result<DetectionSnapshot> {
    let include_repo = matches!(scope, PathScope::Repo);
    let (providers, canonical_resources, all_resources) = match resource {
        LinkableResource::Skill => {
            let detector = SkillsDetector::new(
                paths,
                user_base_for_scope(scope, canonical_provider),
                repo_base_for_scope(scope, canonical_provider),
                include_repo,
            )?;
            (
                detector.supported_providers().to_vec(),
                scope_canonical_map(scope, &detector),
                scope_resources(scope, &detector),
            )
        }
        LinkableResource::Command => {
            let detector = SlashCommandsDetector::new(
                paths,
                user_base_for_scope(scope, canonical_provider),
                repo_base_for_scope(scope, canonical_provider),
                include_repo,
            )?;
            (
                detector.supported_providers().to_vec(),
                scope_canonical_map(scope, &detector),
                scope_resources(scope, &detector),
            )
        }
        LinkableResource::Agent => {
            let detector = AgentDefinitionsDetector::new(
                paths,
                user_base_for_scope(scope, canonical_provider),
                repo_base_for_scope(scope, canonical_provider),
                include_repo,
            )?;
            (
                detector.supported_providers().to_vec(),
                scope_canonical_map(scope, &detector),
                scope_resources(scope, &detector),
            )
        }
        LinkableResource::Script => {
            let detector = SharedScriptsDetector::new(
                paths,
                user_base_for_scope(scope, canonical_provider),
                repo_base_for_scope(scope, canonical_provider),
                include_repo,
            )?;
            (
                detector.supported_providers().to_vec(),
                scope_canonical_map(scope, &detector),
                scope_resources(scope, &detector),
            )
        }
    };

    let mut resources_by_provider_name = HashMap::new();
    let mut names = BTreeSet::new();

    for (name, resource) in &canonical_resources {
        names.insert(name.clone());
        resources_by_provider_name
            .entry((resource.provider, name.clone()))
            .or_insert_with(|| resource.clone());
    }

    for resource in &all_resources {
        names.insert(resource.name.clone());
        resources_by_provider_name
            .entry((resource.provider, resource.name.clone()))
            .or_insert_with(|| resource.clone());
    }

    Ok(DetectionSnapshot {
        providers,
        canonical_by_name: canonical_resources,
        resources_by_provider_name,
        names: names.into_iter().collect(),
    })
}

fn scope_canonical_map<T>(scope: PathScope, detector: &T) -> HashMap<String, DiscoveredResource>
where
    T: LinkDetector,
{
    match scope {
        PathScope::User => detector.user_canonical_resources().clone(),
        PathScope::Repo => detector
            .repo_canonical_resources()
            .cloned()
            .unwrap_or_default(),
    }
}

fn scope_resources<T>(scope: PathScope, detector: &T) -> Vec<DiscoveredResource>
where
    T: LinkDetector,
{
    match scope {
        PathScope::User => detector.user_resources().to_vec(),
        PathScope::Repo => detector.repo_resources().unwrap_or(&[]).to_vec(),
    }
}

fn user_base_for_scope(scope: PathScope, canonical_provider: Provider) -> Option<Provider> {
    match scope {
        PathScope::User => Some(canonical_provider),
        PathScope::Repo => None,
    }
}

fn repo_base_for_scope(scope: PathScope, canonical_provider: Provider) -> Option<Provider> {
    match scope {
        PathScope::User => None,
        PathScope::Repo => Some(canonical_provider),
    }
}

fn classify_target_state(
    resource: LinkableResource,
    path_scope: PathScope,
    model_scope: ModelScope,
    canonical_ref: &ResourceReference,
    target_provider: Provider,
    existing: Option<&DiscoveredResource>,
) -> ResourceReference {
    let requirement_state =
        classify_target_reference(resource, canonical_ref, target_provider, model_scope);
    if matches!(requirement_state, ResourceReference::IncompleteLink(_, _)) {
        return requirement_state;
    }

    let capabilities = capabilities_for(target_provider);
    let support = capabilities.support_for(resource);
    let Some(target_format) = support.format else {
        return ResourceReference::IncompleteLink(target_provider, model_scope);
    };

    if target_format == ResourceFormat::Markdown
        || (resource == LinkableResource::Script && target_format == ResourceFormat::Executable)
    {
        return classify_direct_state(
            resource,
            path_scope,
            model_scope,
            canonical_ref,
            target_provider,
            existing,
        );
    }

    let conversion = conversion_for_formats(ResourceFormat::Markdown, target_format);
    if conversion.is_none() {
        return ResourceReference::IncompleteLink(target_provider, model_scope);
    }

    classify_derived_state(
        model_scope,
        canonical_ref,
        target_provider,
        target_format,
        existing,
    )
}

fn classify_direct_state(
    resource: LinkableResource,
    path_scope: PathScope,
    model_scope: ModelScope,
    canonical_ref: &ResourceReference,
    target_provider: Provider,
    existing: Option<&DiscoveredResource>,
) -> ResourceReference {
    let Some(existing) = existing else {
        return ResourceReference::LinkMissing(target_provider, model_scope);
    };

    if !existing.is_symlink {
        return ResourceReference::Isolated(isolated_definition(existing, model_scope));
    }

    if symlink_points_to_canonical(resource, path_scope, existing, canonical_ref) {
        ResourceReference::Link(target_provider, model_scope)
    } else {
        ResourceReference::IncompleteLink(target_provider, model_scope)
    }
}

fn classify_derived_state(
    model_scope: ModelScope,
    canonical_ref: &ResourceReference,
    target_provider: Provider,
    target_format: ResourceFormat,
    existing: Option<&DiscoveredResource>,
) -> ResourceReference {
    let Some(existing) = existing else {
        return ResourceReference::DerivedMissing(target_provider, model_scope);
    };

    if existing.is_symlink {
        return ResourceReference::IncompleteLink(target_provider, model_scope);
    }

    if derived_hashes_match(canonical_ref, &existing.path, target_format) {
        ResourceReference::DerivedLink(target_provider, model_scope)
    } else {
        ResourceReference::DerivedStale(target_provider, model_scope)
    }
}

fn apply_direct_link(
    paths: &ProviderSkillPaths,
    resource: LinkableResource,
    scope: PathScope,
    provider: Provider,
    canonical: &ResourceDefinition,
) -> Result<bool> {
    let Some(dest_dir) = paths.resource_dir(provider, resource, scope) else {
        return Ok(false);
    };
    if category_link_target(&dest_dir)?.is_some() {
        return Ok(false);
    }

    let source = canonical_link_source_path(resource, canonical);
    match create_skill_link(&source, &dest_dir, scope)? {
        LinkResult::Linked { .. } | LinkResult::AlreadyLinked => Ok(true),
        LinkResult::Skipped { .. } => Ok(false),
    }
}

fn apply_derived_write(
    paths: &ProviderSkillPaths,
    resource: LinkableResource,
    scope: PathScope,
    provider: Provider,
    canonical: &ResourceDefinition,
    name: &str,
) -> Result<bool> {
    let capabilities = capabilities_for(provider);
    let support = capabilities.support_for(resource);
    let Some(target_format) = support.format else {
        return Ok(false);
    };
    let Some(conversion) = conversion_for_formats(ResourceFormat::Markdown, target_format) else {
        return Ok(false);
    };

    let Some(dest_dir) = paths.resource_dir(provider, resource, scope) else {
        return Ok(false);
    };
    let target_path = expected_resource_path(&dest_dir, name, resource, target_format);

    if target_path
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Ok(false);
    }

    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let rendered = render_derived_content(canonical, conversion)?;
    std::fs::write(target_path, rendered)?;
    Ok(true)
}

fn render_derived_content(
    canonical: &ResourceDefinition,
    conversion: ResourceFormatConversion,
) -> Result<String> {
    match conversion {
        ResourceFormatConversion::Toml => render_toml_derived(canonical),
        ResourceFormatConversion::Yaml => render_yaml_derived(canonical),
        ResourceFormatConversion::Bespoke((_, out)) => Ok(out(render_markdown(canonical))),
    }
}

fn render_toml_derived(canonical: &ResourceDefinition) -> Result<String> {
    let mut doc = toml_edit::DocumentMut::new();

    for (key, value) in &canonical.frontmatter {
        if key == DERIVED_FM_HASH_KEY || key == DERIVED_BODY_HASH_KEY {
            continue;
        }
        doc[key] = toml_edit::value(value.as_str());
    }

    if !canonical.body.trim().is_empty() && !doc.contains_key("prompt") {
        doc["prompt"] = toml_edit::value(canonical.body.as_str());
    }

    doc[DERIVED_FM_HASH_KEY] = toml_edit::value(canonical.fm_hash.to_string());
    doc[DERIVED_BODY_HASH_KEY] = toml_edit::value(canonical.body_hash.to_string());

    Ok(doc.to_string())
}

fn render_yaml_derived(canonical: &ResourceDefinition) -> Result<String> {
    let mut map = serde_yaml::Mapping::new();
    for (key, value) in &canonical.frontmatter {
        if key == DERIVED_FM_HASH_KEY || key == DERIVED_BODY_HASH_KEY {
            continue;
        }
        map.insert(
            serde_yaml::Value::String(key.clone()),
            serde_yaml::Value::String(value.clone()),
        );
    }

    let prompt_key = serde_yaml::Value::String("prompt".to_string());
    if !canonical.body.trim().is_empty() && !map.contains_key(&prompt_key) {
        map.insert(
            prompt_key,
            serde_yaml::Value::String(canonical.body.clone()),
        );
    }

    map.insert(
        serde_yaml::Value::String(DERIVED_FM_HASH_KEY.to_string()),
        serde_yaml::Value::String(canonical.fm_hash.to_string()),
    );
    map.insert(
        serde_yaml::Value::String(DERIVED_BODY_HASH_KEY.to_string()),
        serde_yaml::Value::String(canonical.body_hash.to_string()),
    );

    Ok(serde_yaml::to_string(&map)?)
}

fn render_markdown(definition: &ResourceDefinition) -> String {
    if definition.frontmatter.is_empty() {
        return definition.body.clone();
    }

    let mut map = serde_yaml::Mapping::new();
    for (key, value) in &definition.frontmatter {
        map.insert(
            serde_yaml::Value::String(key.clone()),
            serde_yaml::Value::String(value.clone()),
        );
    }
    let yaml = serde_yaml::to_string(&map).unwrap_or_default();
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml).trim_end();
    format!("---\n{yaml}\n---\n{}", definition.body)
}

fn derived_hashes_match(
    canonical_ref: &ResourceReference,
    path: &Path,
    target_format: ResourceFormat,
) -> bool {
    let Some(canonical) = canonical_definition(canonical_ref) else {
        return false;
    };
    let Some((fm_hash, body_hash)) = read_derived_hashes(path, target_format) else {
        return false;
    };

    fm_hash == canonical.fm_hash && body_hash == canonical.body_hash
}

fn read_derived_hashes(path: &Path, format: ResourceFormat) -> Option<(u64, u64)> {
    match format {
        ResourceFormat::Toml => read_toml_hashes(path),
        ResourceFormat::Yaml => read_yaml_hashes(path),
        _ => None,
    }
}

fn read_toml_hashes(path: &Path) -> Option<(u64, u64)> {
    let content = std::fs::read_to_string(path).ok()?;
    let doc: toml_edit::DocumentMut = content.parse().ok()?;
    let fm_hash = toml_hash_value(doc.get(DERIVED_FM_HASH_KEY)?)?;
    let body_hash = toml_hash_value(doc.get(DERIVED_BODY_HASH_KEY)?)?;
    Some((fm_hash, body_hash))
}

fn toml_hash_value(item: &toml_edit::Item) -> Option<u64> {
    if let Some(value) = item.as_integer() {
        return u64::try_from(value).ok();
    }
    item.as_str()?.parse::<u64>().ok()
}

fn read_yaml_hashes(path: &Path) -> Option<(u64, u64)> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let map = value.as_mapping()?;
    let fm_hash =
        yaml_hash_value(map.get(serde_yaml::Value::String(DERIVED_FM_HASH_KEY.to_string()))?)?;
    let body_hash =
        yaml_hash_value(map.get(serde_yaml::Value::String(DERIVED_BODY_HASH_KEY.to_string()))?)?;
    Some((fm_hash, body_hash))
}

fn yaml_hash_value(value: &serde_yaml::Value) -> Option<u64> {
    match value {
        serde_yaml::Value::Number(number) => number.as_u64(),
        serde_yaml::Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    }
}

fn expected_resource_path(
    dest_dir: &Path,
    name: &str,
    resource: LinkableResource,
    format: ResourceFormat,
) -> PathBuf {
    match resource {
        LinkableResource::Skill => dest_dir.join(name),
        LinkableResource::Script => dest_dir.join(name),
        LinkableResource::Command | LinkableResource::Agent => {
            let extension = match format {
                ResourceFormat::Markdown => "md",
                ResourceFormat::Toml => "toml",
                ResourceFormat::Yaml => "yaml",
                _ => "",
            };

            if extension.is_empty() {
                dest_dir.join(name)
            } else {
                dest_dir.join(format!("{name}.{extension}"))
            }
        }
    }
}

fn symlink_points_to_canonical(
    resource: LinkableResource,
    path_scope: PathScope,
    existing: &DiscoveredResource,
    canonical_ref: &ResourceReference,
) -> bool {
    let Some(canonical) = canonical_definition(canonical_ref) else {
        return false;
    };

    let canonical_source = canonical_link_source_path(resource, canonical);
    let Ok(link_target) = std::fs::read_link(&existing.path) else {
        return false;
    };

    let expected = match path_scope {
        PathScope::User => canonical_source,
        PathScope::Repo => {
            let Some(parent) = existing.path.parent() else {
                return false;
            };
            relative_path(parent, &canonical_source)
        }
    };

    link_target == expected
}

fn canonical_link_source_path(
    resource: LinkableResource,
    definition: &ResourceDefinition,
) -> PathBuf {
    match resource {
        LinkableResource::Skill => definition
            .filepath
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| definition.filepath.clone()),
        _ => definition.filepath.clone(),
    }
}

fn isolated_definition(existing: &DiscoveredResource, scope: ModelScope) -> ResourceDefinition {
    let bytes = std::fs::read(&existing.path).unwrap_or_default();
    let body = String::from_utf8(bytes.clone()).unwrap_or_default();

    ResourceDefinition {
        name: existing.name.clone(),
        provider: existing.provider,
        scope,
        filepath: existing.path.clone(),
        frontmatter: BTreeMap::new(),
        body_hash: biscuit_hash::xx_hash_bytes(&bytes),
        fm_hash: 0,
        body,
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

fn conversion_for_formats(
    source_format: ResourceFormat,
    target_format: ResourceFormat,
) -> Option<ResourceFormatConversion> {
    match (source_format, target_format) {
        (ResourceFormat::Markdown, ResourceFormat::Toml) => Some(ResourceFormatConversion::Toml),
        (ResourceFormat::Markdown, ResourceFormat::Yaml) => Some(ResourceFormatConversion::Yaml),
        _ => None,
    }
}

fn to_model_scope(scope: PathScope) -> ModelScope {
    match scope {
        PathScope::User => ModelScope::User,
        PathScope::Repo => ModelScope::Repo,
    }
}

fn to_resource_type(resource: LinkableResource) -> ResourceType {
    match resource {
        LinkableResource::Skill => ResourceType::Skill,
        LinkableResource::Command => ResourceType::SlashCommand,
        LinkableResource::Agent => ResourceType::AgentDefinition,
        LinkableResource::Script => ResourceType::SharedScripts,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn find_reference<'a>(
        resources: &'a [Resource],
        name: &str,
        provider: Provider,
    ) -> &'a ResourceReference {
        &resources
            .iter()
            .find(|resource| resource.name == name && resource.provider == provider)
            .unwrap()
            .definition
    }

    #[cfg(unix)]
    #[test]
    fn direct_markdown_links_are_created_from_link_missing() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        write(
            &home.path().join(".claude/commands/build.md"),
            "---\ndescription: build\n---\nRun build.\n",
        );

        let analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&analyzed, "build", Provider::Codex),
            ResourceReference::LinkMissing(Provider::Codex, ModelScope::User)
        ));

        let summary = apply_fixable_resources(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            &analyzed,
        )
        .unwrap();
        assert!(summary.links_created > 0);

        let reanalyzed = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&reanalyzed, "build", Provider::Codex),
            ResourceReference::Link(Provider::Codex, ModelScope::User)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn repo_scope_skill_links_are_created_with_relative_targets() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        let canonical = repo.path().join(".claude/skills/repo-skill/SKILL.md");
        write(
            &canonical,
            "---\ndescription: Repo skill\n---\nUse this skill in repo scope.\n",
        );

        let analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Skill,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&analyzed, "repo-skill", Provider::Codex),
            ResourceReference::LinkMissing(Provider::Codex, ModelScope::Repo)
        ));

        let summary =
            apply_fixable_resources(&paths, LinkableResource::Skill, PathScope::Repo, &analyzed)
                .unwrap();
        assert!(summary.links_created > 0);

        let codex_link = repo.path().join(".codex/skills/repo-skill");
        assert!(
            codex_link
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let target = std::fs::read_link(&codex_link).unwrap();
        assert!(target.is_relative());
        let expected = relative_path(codex_link.parent().unwrap(), canonical.parent().unwrap());
        assert_eq!(target, expected);

        let reanalyzed = analyze_resource_links(
            &paths,
            LinkableResource::Skill,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&reanalyzed, "repo-skill", Provider::Codex),
            ResourceReference::Link(Provider::Codex, ModelScope::Repo)
        ));
    }

    #[test]
    fn derived_toml_artifacts_are_created_and_marked_in_sync() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        write(
            &home.path().join(".claude/commands/test.md"),
            "---\ndescription: run tests\n---\nRun tests with coverage.\n",
        );

        let analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&analyzed, "test", Provider::Gemini),
            ResourceReference::DerivedMissing(Provider::Gemini, ModelScope::User)
        ));

        let summary = apply_fixable_resources(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            &analyzed,
        )
        .unwrap();
        assert!(summary.derived_written > 0);

        let reanalyzed = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&reanalyzed, "test", Provider::Gemini),
            ResourceReference::DerivedLink(Provider::Gemini, ModelScope::User)
        ));
    }

    #[test]
    fn repo_scope_derived_artifacts_transition_missing_to_stale_to_synced() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        let canonical = repo.path().join(".claude/commands/check.md");
        write(
            &canonical,
            "---\ndescription: repo check\n---\nRun repo checks.\n",
        );

        let missing = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&missing, "check", Provider::Gemini),
            ResourceReference::DerivedMissing(Provider::Gemini, ModelScope::Repo)
        ));

        let summary =
            apply_fixable_resources(&paths, LinkableResource::Command, PathScope::Repo, &missing)
                .unwrap();
        assert!(summary.derived_written > 0);

        let synced = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&synced, "check", Provider::Gemini),
            ResourceReference::DerivedLink(Provider::Gemini, ModelScope::Repo)
        ));

        write(
            &canonical,
            "---\ndescription: repo check\n---\nRun repo checks with strict mode.\n",
        );

        let stale = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&stale, "check", Provider::Gemini),
            ResourceReference::DerivedStale(Provider::Gemini, ModelScope::Repo)
        ));

        let summary =
            apply_fixable_resources(&paths, LinkableResource::Command, PathScope::Repo, &stale)
                .unwrap();
        assert!(summary.derived_written > 0);

        let resynced = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&resynced, "check", Provider::Gemini),
            ResourceReference::DerivedLink(Provider::Gemini, ModelScope::Repo)
        ));
    }

    #[test]
    fn derived_hash_changes_mark_and_fix_stale_artifacts() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        let canonical = home.path().join(".claude/commands/lint.md");
        write(
            &canonical,
            "---\ndescription: lint\n---\nRun lint checks.\n",
        );

        let initial = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        apply_fixable_resources(&paths, LinkableResource::Command, PathScope::User, &initial)
            .unwrap();

        write(
            &canonical,
            "---\ndescription: lint\n---\nRun lint checks with strict mode.\n",
        );

        let stale = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&stale, "lint", Provider::Gemini),
            ResourceReference::DerivedStale(Provider::Gemini, ModelScope::User)
        ));

        let summary =
            apply_fixable_resources(&paths, LinkableResource::Command, PathScope::User, &stale)
                .unwrap();
        assert!(summary.derived_written > 0);

        let fixed = analyze_resource_links(
            &paths,
            LinkableResource::Command,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&fixed, "lint", Provider::Gemini),
            ResourceReference::DerivedLink(Provider::Gemini, ModelScope::User)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn repo_scope_agent_links_are_created_with_relative_targets() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        let canonical = repo.path().join(".claude/agents/reviewer.md");
        write(
            &canonical,
            "---\nname: reviewer\ndescription: Reviews pull requests\n---\nReview all changes.\n",
        );

        let analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Agent,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&analyzed, "reviewer", Provider::OpenCode),
            ResourceReference::LinkMissing(Provider::OpenCode, ModelScope::Repo)
        ));

        let summary =
            apply_fixable_resources(&paths, LinkableResource::Agent, PathScope::Repo, &analyzed)
                .unwrap();
        assert!(summary.links_created > 0);

        let opencode_link = repo.path().join(".opencode/agents/reviewer.md");
        assert!(
            opencode_link
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let target = std::fs::read_link(&opencode_link).unwrap();
        assert!(target.is_relative());
        let expected = relative_path(opencode_link.parent().unwrap(), &canonical);
        assert_eq!(target, expected);

        let reanalyzed = analyze_resource_links(
            &paths,
            LinkableResource::Agent,
            PathScope::Repo,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&reanalyzed, "reviewer", Provider::OpenCode),
            ResourceReference::Link(Provider::OpenCode, ModelScope::Repo)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn script_links_are_created_for_user_and_repo_scopes() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        let user_canonical = home.path().join(".codex/scripts/tool.sh");
        write(&user_canonical, "#!/usr/bin/env bash\necho user\n");

        let user_analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Script,
            PathScope::User,
            Provider::Codex,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&user_analyzed, "tool.sh", Provider::Goose),
            ResourceReference::LinkMissing(Provider::Goose, ModelScope::User)
        ));

        let summary = apply_fixable_resources(
            &paths,
            LinkableResource::Script,
            PathScope::User,
            &user_analyzed,
        )
        .unwrap();
        assert!(summary.links_created > 0);

        let goose_user_link = home.path().join(".config/goose/scripts/tool.sh");
        assert!(
            goose_user_link
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let user_target = std::fs::read_link(&goose_user_link).unwrap();
        assert!(user_target.is_absolute());
        assert_eq!(user_target, user_canonical);

        let repo_canonical = repo.path().join(".codex/scripts/tool.sh");
        write(&repo_canonical, "#!/usr/bin/env bash\necho repo\n");

        let repo_analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Script,
            PathScope::Repo,
            Provider::Codex,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&repo_analyzed, "tool.sh", Provider::Goose),
            ResourceReference::LinkMissing(Provider::Goose, ModelScope::Repo)
        ));

        let summary = apply_fixable_resources(
            &paths,
            LinkableResource::Script,
            PathScope::Repo,
            &repo_analyzed,
        )
        .unwrap();
        assert!(summary.links_created > 0);

        let goose_repo_link = repo.path().join(".goose/scripts/tool.sh");
        assert!(
            goose_repo_link
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let repo_target = std::fs::read_link(&goose_repo_link).unwrap();
        assert!(repo_target.is_relative());
        let expected = relative_path(goose_repo_link.parent().unwrap(), &repo_canonical);
        assert_eq!(repo_target, expected);
    }

    #[test]
    fn incomplete_links_are_not_derived_when_requirements_fail() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        let paths = ProviderSkillPaths::from_roots_for_test(
            home.path().to_path_buf(),
            repo.path().to_path_buf(),
        );

        write(
            &home.path().join(".claude/agents/reviewer.md"),
            "---\nname: reviewer\ndescription: Reviews changes\n---\nUse strict review standards.\n",
        );

        let analyzed = analyze_resource_links(
            &paths,
            LinkableResource::Agent,
            PathScope::User,
            Provider::Claude,
        )
        .unwrap();
        assert!(matches!(
            find_reference(&analyzed, "reviewer", Provider::KimiCode),
            ResourceReference::IncompleteLink(Provider::KimiCode, ModelScope::User)
        ));

        let _summary =
            apply_fixable_resources(&paths, LinkableResource::Agent, PathScope::User, &analyzed)
                .unwrap();
        assert!(!home.path().join(".kimi/agents/reviewer.yaml").exists());
    }
}
