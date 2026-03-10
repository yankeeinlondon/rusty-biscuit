use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;

use super::catalog::{MatchTier, McpCatalogStore};
use super::defaults::effective_defaults;
use super::types::McpServer;

/// A tag and how it was resolved.
#[derive(Debug, Clone)]
pub struct ResolvedTag {
    /// The original tag string (without `#`).
    pub tag: String,
    /// The catalog ID it resolved to.
    pub resolved_to: String,
    /// Which match tier was used.
    pub match_tier: MatchTier,
}

/// The computed session set — defaults + explicit + tags, deduplicated.
#[derive(Debug)]
pub struct SessionSet {
    /// Resolved server definitions.
    pub servers: Vec<McpServer>,
    /// Prompt with tags stripped (if tags were extracted).
    pub cleaned_prompt: Option<String>,
    /// Tags that were resolved.
    pub resolved_tags: Vec<ResolvedTag>,
    /// Warnings (e.g. unresolved IDs).
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Session computation
// ---------------------------------------------------------------------------

/// Compute the effective session set.
///
/// 1. Start with effective defaults (repo replaces user)
/// 2. Add explicit `--use` values
/// 3. Add `#tag` values from prompt
/// 4. Resolve all through the catalog
/// 5. Deduplicate
pub fn compute_session_set(
    catalog: &McpCatalogStore,
    repo_root: Option<&Path>,
    explicit_use: &[String],
    prompt_tags: &[String],
) -> Result<SessionSet> {
    let mut warnings = Vec::new();
    let mut resolved_tags = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut servers = Vec::new();

    // 1. Effective defaults
    let defaults = effective_defaults(repo_root, catalog)?;
    for id in &defaults {
        if let Ok(server) = catalog.resolve(id) {
            if seen_ids.insert(server.id.clone()) {
                servers.push(server.clone());
            }
        } else {
            warnings.push(format!("default `{id}` not found in catalog"));
        }
    }

    // 2. Explicit --use
    for id in explicit_use {
        if let Ok(server) = catalog.resolve(id) {
            if seen_ids.insert(server.id.clone()) {
                servers.push(server.clone());
            }
        } else {
            warnings.push(format!("`--use {id}` not found in catalog"));
        }
    }

    // 3. Tags
    for tag in prompt_tags {
        match catalog.resolve_match(tag) {
            Ok(resolved) => {
                resolved_tags.push(ResolvedTag {
                    tag: tag.clone(),
                    resolved_to: resolved.server.id.clone(),
                    match_tier: resolved.tier,
                });
                if seen_ids.insert(resolved.server.id.clone()) {
                    servers.push(resolved.server.clone());
                }
            }
            Err(e) => {
                warnings.push(format!("tag `#{tag}`: {e}"));
            }
        }
    }

    Ok(SessionSet {
        servers,
        cleaned_prompt: None,
        resolved_tags,
        warnings,
    })
}

/// Extract `#tag` tokens from a prompt string.
///
/// Returns `(cleaned_prompt, extracted_tags)` where matched tags are stripped
/// from the outgoing prompt.
pub fn extract_tags(prompt: &str, catalog: &McpCatalogStore) -> (String, Vec<String>) {
    let mut tags = Vec::new();
    let mut cleaned = String::with_capacity(prompt.len());
    let mut chars = prompt.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '#' {
            // Extract the word after #
            let start = i + 1;
            let mut end = start;
            while let Some(&(j, c)) = chars.peek() {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    end = j + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }

            if end > start {
                let word = &prompt[start..end];
                // Only extract if it resolves to a catalog entry
                if catalog.resolve(word).is_ok() {
                    tags.push(word.to_string());
                    // Skip the tag (don't add to cleaned)
                    // Also skip trailing whitespace
                    while let Some(&(_, c)) = chars.peek() {
                        if c == ' ' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }
            // Not a valid tag, keep as-is
            cleaned.push('#');
            cleaned.push_str(&prompt[start..end]);
        } else {
            cleaned.push(ch);
        }
    }

    (cleaned.trim().to_string(), tags)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;
    use crate::mcp::types::{McpServerMetadata, McpTransport};

    fn make_catalog_with_servers(names: &[&str]) -> McpCatalogStore {
        let mut catalog = McpCatalogStore::load_from(std::path::Path::new("/nonexistent")).unwrap();
        for name in names {
            let server = McpServer {
                id: name.to_string(),
                aliases: Vec::new(),
                transport: McpTransport::Stdio,
                command: Some("npx".into()),
                args: vec![format!("@test/{name}")],
                cwd: None,
                env: HashMap::new(),
                url: None,
                headers: HashMap::new(),
                enabled_tools: Vec::new(),
                disabled_tools: Vec::new(),
                required: false,
                metadata: McpServerMetadata {
                    description: None,
                    created_from: None,
                    fingerprint: String::new(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
                provider_overrides: HashMap::new(),
            };
            catalog.add_server(server);
        }
        catalog
    }

    #[test]
    fn extract_tags_finds_catalog_matches() {
        let catalog = make_catalog_with_servers(&["calendar", "slack"]);
        let (cleaned, tags) = extract_tags("fix the #calendar integration #slack", &catalog);
        assert_eq!(tags, vec!["calendar", "slack"]);
        assert_eq!(cleaned, "fix the integration");
    }

    #[test]
    fn extract_tags_preserves_non_catalog_hashes() {
        let catalog = make_catalog_with_servers(&["calendar"]);
        let (cleaned, tags) = extract_tags("fix #123 and #calendar", &catalog);
        assert_eq!(tags, vec!["calendar"]);
        assert!(cleaned.contains("#123"));
    }

    #[test]
    fn extract_tags_empty_prompt() {
        let catalog = make_catalog_with_servers(&["calendar"]);
        let (cleaned, tags) = extract_tags("", &catalog);
        assert!(cleaned.is_empty());
        assert!(tags.is_empty());
    }

    #[test]
    fn session_set_deduplicates() {
        let catalog = make_catalog_with_servers(&["calendar", "slack"]);
        let session = compute_session_set(
            &catalog,
            None,
            &["calendar".into(), "slack".into(), "calendar".into()],
            &[],
        )
        .unwrap();
        assert_eq!(session.servers.len(), 2);
    }

    #[test]
    fn session_set_with_tags() {
        let catalog = make_catalog_with_servers(&["calendar", "slack"]);
        let session = compute_session_set(&catalog, None, &[], &["calendar".into()]).unwrap();
        assert_eq!(session.servers.len(), 1);
        assert_eq!(session.resolved_tags.len(), 1);
        assert_eq!(session.resolved_tags[0].match_tier, MatchTier::ExactId);
    }

    #[test]
    fn session_set_warnings_for_missing() {
        let catalog = make_catalog_with_servers(&["calendar"]);
        let session = compute_session_set(&catalog, None, &["nonexistent".into()], &[]).unwrap();
        assert_eq!(session.warnings.len(), 1);
        assert!(session.warnings[0].contains("nonexistent"));
    }
}
