use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;

use super::catalog::{MatchTier, McpCatalogStore, Resolution};
use super::defaults::effective_defaults;
use super::types::McpServer;

/// A tag and how it resolved into the session set.
#[derive(Debug, Clone)]
pub struct ResolvedTag {
    pub tag: String,
    pub resolved_to: String,
    pub match_tier: MatchTier,
}

/// An ambiguous tag plus the candidate catalog IDs it could map to.
#[derive(Debug, Clone)]
pub struct AmbiguousTag {
    pub tag: String,
    pub match_tier: MatchTier,
    pub candidates: Vec<String>,
}

/// The computed session set — defaults + explicit values + resolved tags.
#[derive(Debug, Default)]
pub struct SessionSet {
    pub servers: Vec<McpServer>,
    pub cleaned_prompt: Option<String>,
    pub default_servers: Vec<String>,
    pub explicit_servers: Vec<String>,
    pub tag_servers: Vec<String>,
    pub resolved_tags: Vec<ResolvedTag>,
    pub missing_tags: Vec<String>,
    pub ambiguous_tags: Vec<AmbiguousTag>,
    pub warnings: Vec<String>,
}

/// Compute the effective session set.
///
/// The caller controls ambiguity handling via `resolve_ambiguous`. Returning
/// `Some(id)` selects a candidate; returning `None` leaves the tag ambiguous.
pub fn compute_session_set<F>(
    catalog: &McpCatalogStore,
    repo_root: Option<&Path>,
    explicit_use: &[String],
    prompt_tags: &[String],
    mut resolve_ambiguous: F,
) -> Result<SessionSet>
where
    F: FnMut(&str, MatchTier, &[String]) -> Option<String>,
{
    let mut session = SessionSet::default();
    let mut seen_ids = HashSet::new();

    for id in effective_defaults(repo_root, catalog)? {
        match catalog.resolve(&id) {
            Ok(server) => push_server(
                &mut session.default_servers,
                &mut session.servers,
                &mut seen_ids,
                server.clone(),
            ),
            Err(_) => session
                .warnings
                .push(format!("default `{id}` not found in catalog")),
        }
    }

    for query in explicit_use {
        match catalog.resolve(query) {
            Ok(server) => push_server(
                &mut session.explicit_servers,
                &mut session.servers,
                &mut seen_ids,
                server.clone(),
            ),
            Err(error) => session
                .warnings
                .push(format!("`--use {query}` could not be resolved: {error}")),
        }
    }

    for tag in prompt_tags {
        match catalog.resolve_outcome(tag) {
            Resolution::Resolved(resolved) => {
                session.resolved_tags.push(ResolvedTag {
                    tag: tag.clone(),
                    resolved_to: resolved.server.id.clone(),
                    match_tier: resolved.tier,
                });
                push_server(
                    &mut session.tag_servers,
                    &mut session.servers,
                    &mut seen_ids,
                    resolved.server.clone(),
                );
            }
            Resolution::Missing => session.missing_tags.push(tag.clone()),
            Resolution::Ambiguous { tier, candidates } => {
                let candidate_ids: Vec<String> =
                    candidates.iter().map(|server| server.id.clone()).collect();
                if let Some(selected) = resolve_ambiguous(tag, tier, &candidate_ids) {
                    match catalog.resolve(&selected) {
                        Ok(server) => {
                            session.resolved_tags.push(ResolvedTag {
                                tag: tag.clone(),
                                resolved_to: server.id.clone(),
                                match_tier: tier,
                            });
                            push_server(
                                &mut session.tag_servers,
                                &mut session.servers,
                                &mut seen_ids,
                                server.clone(),
                            );
                        }
                        Err(error) => session.warnings.push(format!(
                            "tag `#{tag}` selected invalid candidate `{selected}`: {error}"
                        )),
                    }
                } else {
                    session.ambiguous_tags.push(AmbiguousTag {
                        tag: tag.clone(),
                        match_tier: tier,
                        candidates: candidate_ids,
                    });
                }
            }
        }
    }

    Ok(session)
}

fn push_server(
    source_list: &mut Vec<String>,
    servers: &mut Vec<McpServer>,
    seen_ids: &mut HashSet<String>,
    server: McpServer,
) {
    source_list.push(server.id.clone());
    if seen_ids.insert(server.id.clone()) {
        servers.push(server);
    }
}

/// Lex `#tags` from a prompt string according to the MCP CLI spec.
///
/// A valid tag:
/// - starts with `#` at the start of the prompt or after whitespace
/// - has an alphabetic first character after `#`
/// - contains only alphanumeric, `-`, or `_` afterwards
/// - ends at whitespace or end-of-line
pub fn lex_tags(prompt: &str) -> (String, Vec<String>) {
    let chars: Vec<(usize, char)> = prompt.char_indices().collect();
    let mut cleaned = String::with_capacity(prompt.len());
    let mut tags = Vec::new();
    let mut cursor = 0usize;
    let len = chars.len();

    while cursor < len {
        let (byte_index, ch) = chars[cursor];

        if ch != '#' {
            cleaned.push(ch);
            cursor += 1;
            continue;
        }

        let has_valid_prefix = cursor == 0 || chars[cursor - 1].1.is_whitespace();
        let Some((_, first_tag_char)) = chars.get(cursor + 1) else {
            cleaned.push('#');
            cursor += 1;
            continue;
        };

        if !has_valid_prefix || !first_tag_char.is_ascii_alphabetic() {
            cleaned.push('#');
            cursor += 1;
            continue;
        }

        let mut end_index = cursor + 2;
        while end_index < len {
            let candidate = chars[end_index].1;
            if candidate.is_ascii_alphanumeric() || candidate == '-' || candidate == '_' {
                end_index += 1;
                continue;
            }
            break;
        }

        let terminal_char = chars.get(end_index).map(|(_, value)| *value);
        if terminal_char.is_some_and(|value| !value.is_whitespace()) {
            cleaned.push('#');
            cursor += 1;
            continue;
        }

        let tag_start = chars[cursor + 1].0;
        let tag_end = chars
            .get(end_index)
            .map(|(index, _)| *index)
            .unwrap_or(prompt.len());
        tags.push(prompt[tag_start..tag_end].to_string());

        cursor = end_index;
        while cursor < len && chars[cursor].1.is_whitespace() {
            let previous_was_whitespace =
                cleaned.chars().last().is_some_and(char::is_whitespace);
            if !previous_was_whitespace {
                cleaned.push(chars[cursor].1);
            }
            cursor += 1;
        }

        let _ = byte_index;
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
    fn lex_tags_extracts_spec_compliant_tags() {
        let (cleaned, tags) = lex_tags("fix #calendar integration #slack");
        assert_eq!(tags, vec!["calendar", "slack"]);
        assert_eq!(cleaned, "fix integration");
    }

    #[test]
    fn lex_tags_requires_whitespace_or_start_before_hash() {
        let (cleaned, tags) = lex_tags("abc#calendar and #slack");
        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned, "abc#calendar and");
    }

    #[test]
    fn lex_tags_requires_alphabetic_first_character() {
        let (cleaned, tags) = lex_tags("fix #123 and #calendar");
        assert_eq!(tags, vec!["calendar"]);
        assert_eq!(cleaned, "fix #123 and");
    }

    #[test]
    fn lex_tags_rejects_punctuation_terminated_candidates() {
        let (cleaned, tags) = lex_tags("check #calendar, then #slack");
        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned, "check #calendar, then");
    }

    #[test]
    fn session_set_deduplicates_and_tracks_sources() {
        let catalog = make_catalog_with_servers(&["calendar", "slack"]);
        let session = compute_session_set(
            &catalog,
            None,
            &["calendar".into(), "slack".into(), "calendar".into()],
            &["slack".into()],
            |_, _, _| None,
        )
        .unwrap();
        assert_eq!(session.servers.len(), 2);
        assert_eq!(session.explicit_servers.len(), 3);
        assert_eq!(session.tag_servers, vec!["slack"]);
    }

    #[test]
    fn session_set_records_missing_tags() {
        let catalog = make_catalog_with_servers(&["calendar"]);
        let session =
            compute_session_set(&catalog, None, &[], &["missing".into()], |_, _, _| None).unwrap();
        assert_eq!(session.missing_tags, vec!["missing"]);
    }

    #[test]
    fn session_set_can_select_ambiguous_tags() {
        let mut catalog = make_catalog_with_servers(&["calendar", "calendar-beta"]);
        catalog.add_alias("calendar", "team-calendar").unwrap();
        let session = compute_session_set(
            &catalog,
            None,
            &[],
            &["calendar".into()],
            |tag, _, candidates| {
                if tag == "calendar" && candidates.iter().any(|candidate| candidate == "calendar") {
                    Some("calendar".into())
                } else {
                    None
                }
            },
        )
        .unwrap();

        assert_eq!(session.ambiguous_tags.len(), 0);
        assert_eq!(session.tag_servers, vec!["calendar"]);
    }

    #[test]
    fn session_set_keeps_ambiguous_tags_when_not_selected() {
        let catalog = make_catalog_with_servers(&["calendar", "calendar-beta"]);
        let session =
            compute_session_set(&catalog, None, &[], &["cal".into()], |_, _, _| None)
                .unwrap();
        assert_eq!(session.ambiguous_tags.len(), 1);
        assert_eq!(session.ambiguous_tags[0].candidates.len(), 2);
    }
}
