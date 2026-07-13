//! Generate-mode file application: per-file confirmation, decline →
//! override scaffolding, and the unreconciled-drift contract.
//!
//! Design (catalog-generation.md "Generate UX + drift lifecycle"):
//! confirmation granularity is per generated file; decline is not a
//! terminal state — a declined field is pinned back via a field-keyed
//! override (scaffolded here from the committed `catalog.json` values,
//! which carry the exact catalog shape overrides are authored in) or the
//! offending input is reverted. The caller exits non-zero while declined
//! drift remains unreconciled.
//!
//! The decision callback owns ALL interaction (diff printing, prompting),
//! so this module stays testable without a TTY: `--yes` is a callback
//! that always accepts, report-only/`--dry-run` one that always declines.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::catalog::{build_catalog, catalog_path};
use crate::errors::GenError;
use crate::generate::{Generation, committed_data_path, diff_lines};

/// Per-file verdict from the decision callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Write the regenerated file.
    Accept,
    /// Leave the committed file; record unreconciled drift.
    Decline,
    /// Stop asking — this file and every remaining drifted file decline.
    Quit,
}

/// One drifted file the caller chose not to write.
#[derive(Debug)]
pub struct DeclinedDrift {
    pub path: PathBuf,
    /// Provider slug for `data.rs` files; `None` for `catalog.json`.
    pub slug: Option<String>,
    /// Field-keyed override YAML pinning the committed values, when the
    /// committed `catalog.json` still carries them. `None` when no
    /// field-level pin is possible (no committed catalog, no field-level
    /// drift — e.g. an emitter-format change — or the catalog file
    /// itself).
    pub override_snippet: Option<String>,
}

/// What one `generate` application did.
#[derive(Debug)]
pub struct ApplyOutcome {
    /// Files whose committed content already matched.
    pub clean: Vec<PathBuf>,
    /// Files written this run.
    pub written: Vec<PathBuf>,
    /// Drifted files left unreconciled — non-empty means exit non-zero.
    pub declined: Vec<DeclinedDrift>,
}

/// Applies regenerated artifacts under `area`: one `data.rs` per scoped
/// slug, then `catalog.json`, the signals `generated.rs`, the families
/// `families_generated.rs`, and the stream `vocabulary.rs` (all always full
/// scope — `generations` must cover every provider; `signals`, `families`,
/// and `vocabulary` are the prebuilt [`crate::signals::build_signals`] /
/// [`crate::families::build_families`] / [`crate::vocabulary::build_vocabulary`]
/// texts).
///
/// `decide` receives each drifted file's path and line-diff and owns the
/// interaction; after a [`Decision::Quit`] every remaining drifted file is
/// declined without further calls.
pub fn apply_generations(
    area: &Path,
    scope: &[&str],
    generations: &[Generation],
    signals: &str,
    families: &str,
    vocabulary: &str,
    decide: &mut dyn FnMut(&Path, &[String]) -> Decision,
) -> Result<ApplyOutcome, GenError> {
    let committed_catalog = load_committed_catalog(area)?;
    let mut outcome = ApplyOutcome {
        clean: Vec::new(),
        written: Vec::new(),
        declined: Vec::new(),
    };
    let mut quit = false;

    for generation in generations {
        if !scope.contains(&generation.slug.as_str()) {
            continue;
        }
        let path = committed_data_path(area, &generation.slug);
        let snippet = |outcome_slug: &str| {
            committed_catalog
                .as_ref()
                .and_then(|catalog| override_snippet(outcome_slug, generation, catalog))
        };
        apply_one(
            &path,
            &generation.data_rs,
            &mut outcome,
            &mut quit,
            decide,
            Some(generation.slug.clone()),
            || snippet(&generation.slug),
        )?;
    }

    let catalog = build_catalog(generations);
    apply_one(
        &catalog_path(area),
        &catalog,
        &mut outcome,
        &mut quit,
        decide,
        None,
        || None,
    )?;

    apply_one(
        &crate::signals::signals_path(area),
        signals,
        &mut outcome,
        &mut quit,
        decide,
        None,
        || None,
    )?;

    apply_one(
        &crate::families::families_path(area),
        families,
        &mut outcome,
        &mut quit,
        decide,
        None,
        || None,
    )?;

    apply_one(
        &crate::vocabulary::vocabulary_path(area),
        vocabulary,
        &mut outcome,
        &mut quit,
        decide,
        None,
        || None,
    )?;

    Ok(outcome)
}

/// Compare → decide → write/record for a single generated file.
#[allow(clippy::too_many_arguments)]
fn apply_one(
    path: &Path,
    generated: &str,
    outcome: &mut ApplyOutcome,
    quit: &mut bool,
    decide: &mut dyn FnMut(&Path, &[String]) -> Decision,
    slug: Option<String>,
    snippet: impl FnOnce() -> Option<String>,
) -> Result<(), GenError> {
    let committed = if path.is_file() {
        Some(
            std::fs::read_to_string(path).map_err(|source| GenError::Io {
                path: path.to_path_buf(),
                source,
            })?,
        )
    } else {
        None
    };
    if committed.as_deref() == Some(generated) {
        outcome.clean.push(path.to_path_buf());
        return Ok(());
    }

    let diff = diff_lines(committed.as_deref().unwrap_or(""), generated);
    let decision = if *quit {
        Decision::Decline
    } else {
        let decision = decide(path, &diff);
        if decision == Decision::Quit {
            *quit = true;
        }
        decision
    };

    match decision {
        Decision::Accept => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| GenError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            std::fs::write(path, generated).map_err(|source| GenError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            outcome.written.push(path.to_path_buf());
        }
        Decision::Decline | Decision::Quit => {
            outcome.declined.push(DeclinedDrift {
                path: path.to_path_buf(),
                slug,
                override_snippet: snippet(),
            });
        }
    }
    Ok(())
}

/// Parses the committed `catalog.json` when present (absent ⇒ `None`;
/// unparsable ⇒ error — a corrupt committed artifact must fail loudly).
fn load_committed_catalog(area: &Path) -> Result<Option<Value>, GenError> {
    let path = catalog_path(area);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|source| GenError::Io {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| GenError::Json {
            message: format!("committed catalog `{}`: {err}", path.display()),
        })
}

/// The decline → override path: field-keyed YAML pinning each committed
/// catalog value that differs from the regenerated one. Returns `None`
/// when no field-level drift is detectable (emitter-only change, or the
/// slug is absent from the committed catalog).
pub fn override_snippet(
    slug: &str,
    generation: &Generation,
    committed_catalog: &Value,
) -> Option<String> {
    let committed_fields = committed_catalog
        .get("providers")?
        .get(slug)?
        .get("catalog")?
        .as_object()?;

    let mut drifted = serde_yaml_ng::Mapping::new();
    for field in &generation.fields {
        let Some(committed) = committed_fields
            .get(field.field)
            .and_then(|record| record.get("value"))
        else {
            continue;
        };
        if *committed == field.value {
            continue;
        }
        let mut entry = serde_yaml_ng::Mapping::new();
        entry.insert(
            "value".into(),
            serde_yaml_ng::to_value(committed).ok()?,
        );
        entry.insert(
            "reason".into(),
            "TODO — document why the generated value is wrong".into(),
        );
        drifted.insert(
            serde_yaml_ng::Value::String(field.field.to_string()),
            serde_yaml_ng::Value::Mapping(entry),
        );
    }
    if drifted.is_empty() {
        return None;
    }
    let body = serde_yaml_ng::to_string(&serde_yaml_ng::Value::Mapping(drifted)).ok()?;
    Some(format!(
        "# append to docs/providers/overrides/{slug}.yaml to pin the committed value(s):\n{body}"
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::generate::{Provenance, ResolvedField};

    fn generation(slug: &str, data_rs: &str) -> Generation {
        Generation {
            slug: slug.to_string(),
            fields: vec![ResolvedField {
                field: "model_env_vars",
                value: json!(["NEW_VAR"]),
                provenance: Provenance::Declared { kind: "research" },
            }],
            data_rs: data_rs.to_string(),
            skips: Vec::new(),
            research: BTreeMap::new(),
            offering_join: Default::default(),
            artifact_warning: None,
        }
    }

    fn committed_catalog_with(slug: &str, value: Value) -> Value {
        json!({
            "providers": { slug: { "catalog": {
                "model_env_vars": { "value": value, "source": { "kind": "research" } }
            } } }
        })
    }

    #[test]
    fn accept_writes_and_clean_files_skip_the_callback() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        let generation = generation("claude", "new\n");
        // Committed data.rs drifts; catalog.json is absent (drift-from-empty).
        std::fs::create_dir_all(area.join("lib/src/provider/claude")).unwrap();
        std::fs::write(committed_data_path(area, "claude"), "old\n").unwrap();

        let mut asked = Vec::new();
        let outcome = apply_generations(area, &["claude"], std::slice::from_ref(&generation), "signals\n", "families\n", "vocabulary\n", &mut |path,
                diff| {
            asked.push(path.to_path_buf());
            assert!(!diff.is_empty());
            Decision::Accept
        })
        .unwrap();

        assert_eq!(
            outcome.written.len(),
            5,
            "data.rs, catalog.json, signals, families, vocabulary"
        );
        assert!(outcome.declined.is_empty());
        assert_eq!(
            std::fs::read_to_string(committed_data_path(area, "claude")).unwrap(),
            "new\n"
        );
        assert!(catalog_path(area).is_file());
        assert!(crate::signals::signals_path(area).is_file());
        assert!(crate::families::families_path(area).is_file());
        assert!(crate::vocabulary::vocabulary_path(area).is_file());

        // Second run: everything clean, callback never invoked.
        let outcome = apply_generations(area, &["claude"], std::slice::from_ref(&generation), "signals\n", "families\n", "vocabulary\n", &mut |_,
                _| {
            panic!("clean files must not prompt")
        })
        .unwrap();
        assert_eq!(outcome.clean.len(), 5);
        assert!(outcome.written.is_empty() && outcome.declined.is_empty());
    }

    #[test]
    fn decline_leaves_files_and_scaffolds_the_override_pin() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        let generation = generation("claude", "new\n");
        std::fs::create_dir_all(area.join("lib/src/provider/claude")).unwrap();
        std::fs::write(committed_data_path(area, "claude"), "old\n").unwrap();
        std::fs::create_dir_all(area.join("docs/providers")).unwrap();
        std::fs::write(
            catalog_path(area),
            serde_json::to_string(&committed_catalog_with("claude", json!(["OLD_VAR"]))).unwrap(),
        )
        .unwrap();

        let outcome =
            apply_generations(area, &["claude"], std::slice::from_ref(&generation), "signals\n", "families\n", "vocabulary\n", &mut |_, _| {
                Decision::Decline
            })
            .unwrap();

        assert!(outcome.written.is_empty());
        assert_eq!(
            outcome.declined.len(),
            5,
            "data.rs, catalog.json, signals, families, vocabulary"
        );
        assert_eq!(
            std::fs::read_to_string(committed_data_path(area, "claude")).unwrap(),
            "old\n",
            "decline must not write"
        );
        let data_decline = &outcome.declined[0];
        assert_eq!(data_decline.slug.as_deref(), Some("claude"));
        let snippet = data_decline.override_snippet.as_deref().unwrap();
        assert!(snippet.contains("overrides/claude.yaml"), "{snippet}");
        assert!(snippet.contains("model_env_vars:"), "{snippet}");
        assert!(snippet.contains("OLD_VAR"), "{snippet}");
        assert!(snippet.contains("reason: TODO"), "{snippet}");
        // Only data.rs gets a field-keyed pin — catalog.json, signals,
        // families, and vocabulary do not.
        assert!(outcome.declined[1].override_snippet.is_none());
        assert!(outcome.declined[2].override_snippet.is_none());
        assert!(outcome.declined[3].override_snippet.is_none());
        assert!(outcome.declined[4].override_snippet.is_none());
    }

    #[test]
    fn quit_declines_the_rest_without_prompting() {
        let dir = tempfile::tempdir().unwrap();
        let area = dir.path();
        let generations = vec![generation("claude", "new\n"), generation("codex", "new\n")];
        for slug in ["claude", "codex"] {
            std::fs::create_dir_all(area.join(format!("lib/src/provider/{slug}"))).unwrap();
            std::fs::write(committed_data_path(area, slug), "old\n").unwrap();
        }

        let mut calls = 0;
        let outcome =
            apply_generations(area, &["claude", "codex"], &generations, "signals\n", "families\n", "vocabulary\n", &mut |_, _| {
                calls += 1;
                Decision::Quit
            })
            .unwrap();

        assert_eq!(calls, 1, "quit stops further prompting");
        assert_eq!(
            outcome.declined.len(),
            6,
            "both data.rs plus catalog.json, signals, families, and vocabulary"
        );
        assert!(outcome.written.is_empty());
    }

    /// Field values equal but emitted text differs (emitter change): no
    /// field-keyed pin exists, so the snippet is None.
    #[test]
    fn emitter_only_drift_yields_no_override_snippet() {
        let generation = generation("claude", "new\n");
        let catalog = committed_catalog_with("claude", json!(["NEW_VAR"]));
        assert!(override_snippet("claude", &generation, &catalog).is_none());
    }
}
