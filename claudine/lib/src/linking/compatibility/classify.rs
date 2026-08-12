//! Canonical-source and target-reference classification: parse a candidate,
//! apply deterministic compatibility upgrades, and decide whether it can serve
//! as a `Source`/`PartialSource` or a `LinkMissing`/`IncompleteLink` target.

use std::path::{Path, PathBuf};

use crate::error::{ClaudineError, Result};
use crate::linking::capabilities::{LinkableResource, capabilities_for};
use crate::linking::detector::DiscoveredResource;
use crate::linking::model::{
    IncompleteCause, ResourceDefinition, ResourceReference, ResourceScope,
};
use crate::provider::Provider;

use super::frontmatter_io::{parse_markdown_document, write_markdown_document};
use super::properties::{
    apply_alias_duplication, apply_name_derivation, hash_frontmatter, mapping_to_string_map,
    missing_required_for_all_providers, property_is_satisfied,
};

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
            biscuit_file::to_portable_string(&candidate.path)
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

pub(super) fn inferred_name(
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

pub(super) fn valid_slug(name: &str) -> bool {
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
