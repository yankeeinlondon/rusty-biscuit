//! Top-level `claudine sequence <file>` command.

use clap::Args;
use claudine::composition::{
    self, CompositionError, ResolvedCompositionSource, SequenceExecutionOptions,
    parse_interactive_hint,
};
use color_eyre::eyre::{Result, eyre};
use tracing::info_span;

use super::compose::SharedComposeArgs;
use super::schema_interactive::emit_dropped_optional_warnings;

/// Run a Markdown document as a serial sequence of composition steps.
///
/// Positional tokens are one file reference plus optional `key=value` setters
/// in any order. Inline setters override `--set` on overlapping keys; reserved
/// per-step overlay keys still win over both.
#[derive(Debug, Clone, Args)]
pub struct SequenceArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference and/or `key=value` setters.
    #[arg(value_name = "ARG", num_args = 1.., required = true)]
    pub args: Vec<String>,

    /// Override the document's fail-fast behavior for this run.
    /// Accepts: true, false, 1, 0, yes, no.
    #[arg(long = "fail-fast", value_name = "BOOL", value_parser = parse_boolish)]
    pub fail_fast: Option<bool>,
}

fn parse_boolish(s: &str) -> Result<bool, String> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(format!(
            "invalid boolean value '{s}'; expected true/false, 1/0, or yes/no"
        )),
    }
}

/// Entry point for `claudine sequence`.
///
/// Errors returned here bubble up to the top-level walker in `main.rs`,
/// which renders darkmatter `BlockError` reports for typed Markdown
/// failures and falls back to `color_eyre` otherwise.
pub fn run_sequence(
    args: SequenceArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    let code = run_sequence_inner(args, verbose, startup_timings)?;
    std::process::exit(code);
}

#[allow(clippy::result_large_err)]
fn reject_sequence_interactive(
    source: &claudine::composition::ResolvedCompositionSource,
) -> std::result::Result<(), CompositionError> {
    if let Some(Some(true)) = source
        .markdown
        .frontmatter()
        .as_map()
        .get("interactive")
        .map(parse_interactive_hint)
        .transpose()?
    {
        return Err(CompositionError::SequenceInteractiveRejected(
            source.resolved_path.clone(),
        ));
    }
    Ok(())
}

/// Resolve a sequence source, accepting both Markdown files and raw YAML
/// sequence files.
///
/// Markdown files follow the standard [`composition::resolve_composition_source`]
/// path. YAML files (`.yaml` / `.yml`) are treated as frontmatter without a
/// Markdown body: their top-level mapping becomes the document frontmatter so
/// that `sequence`, `prompt`, `name`, `description`, `$schema`, and other keys
/// behave exactly as they would in a Markdown frontmatter block.
#[allow(clippy::result_large_err)]
fn resolve_sequence_source(file_ref: &str) -> Result<ResolvedCompositionSource, CompositionError> {
    // Markdown files use the standard resolution path.
    match composition::resolve_composition_source(file_ref) {
        Ok(source) => return Ok(source),
        Err(CompositionError::NotMarkdown(_)) => {}
        Err(e) => return Err(e),
    }

    // YAML sequence files are treated as frontmatter without a Markdown body.
    let reference = biscuit_file::FileReference::new(file_ref)
        .map_err(|e| CompositionError::InvalidReference(format!("{file_ref}: {e}")))?
        .with_package_area_magic_path();
    let resolved_path = reference
        .resolve()
        .map_err(|e| CompositionError::InvalidReference(format!("{file_ref}: {e}")))?
        .ok_or_else(|| CompositionError::FileNotFound(file_ref.to_string()))?;

    let ext = resolved_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext.to_ascii_lowercase().as_str(), "yaml" | "yml") {
        return Err(CompositionError::NotMarkdown(resolved_path.display().to_string()));
    }

    let yaml = biscuit_file::Yaml::new(&resolved_path).map_err(|e| {
        CompositionError::MarkdownLoad(format!("{}: {e}", resolved_path.display()))
    })?;
    let json_value = yaml.as_json().map_err(|e| {
        CompositionError::MarkdownLoad(format!(
            "YAML-to-JSON conversion failed for {}: {e}",
            resolved_path.display()
        ))
    })?;
    let root = json_value.as_object().ok_or_else(|| {
        CompositionError::SequenceExternalWrongType(
            "YAML sequence file root must be an object".to_string(),
        )
    })?;

    let mut frontmatter = darkmatter::markdown::Frontmatter::new();
    for (key, value) in root {
        frontmatter.insert(key, value.clone()).map_err(|e| {
            CompositionError::MarkdownLoad(format!("{}: {e}", resolved_path.display()))
        })?;
    }

    let original_text = std::fs::read_to_string(&resolved_path)
        .map_err(|e| CompositionError::MarkdownLoad(format!("{}: {e}", resolved_path.display())))?;
    let markdown = darkmatter::markdown::Markdown::with_frontmatter(frontmatter, "");

    Ok(ResolvedCompositionSource {
        original_ref: file_ref.to_string(),
        resolved_path,
        original_text,
        markdown,
    })
}

fn run_sequence_inner(
    args: SequenceArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let SequenceArgs {
        shared,
        args,
        fail_fast,
    } = args;

    if shared.timeout.is_some() && shared.interactive {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }
    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    // Early validation: both flags share the same duration grammar.
    if let Some(ref raw) = shared.timeout {
        claudine::harness::parse_timeout(raw, std::path::Path::new("<--timeout>"))
            .map_err(|e| eyre!("invalid --timeout value: {e}"))?;
    }
    shared.step_timeout_secs()?;

    let parsed = super::compose::parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

    let source = match resolve_sequence_source(&file) {
        Ok(source) => source,
        Err(CompositionError::FileNotFound(_)) => {
            let selected = crate::completion::operation_file::autocomplete_operation_file(
                &file,
                crate::completion::scopes::ComposeMode::Sequence,
            )?;
            resolve_sequence_source(&selected)?
        }
        Err(e) => return Err(e.into()),
    };

    reject_sequence_interactive(&source)?;

    let _sequence_span = info_span!(
        "sequence",
        file = %source.resolved_path.display(),
        fail_fast = ?fail_fast,
    )
    .entered();

    let plan = composition::resolve_sequence_plan(&source)?.ok_or_else(|| {
        eyre!(
            "file '{}' does not define a `sequence` frontmatter property",
            file
        )
    })?;

    let set_overrides =
        super::compose::merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;

    // Schema-aware doc-level scrub: drop invalid optional values only.
    // Required-value validation is deferred to per-step pre-validation
    // (`wrap::sequence::run_phase_1c_attempt`) so a missing required
    // property is aggregated across ALL failing steps and surfaced as
    // `SequenceMissingProperties` — not short-circuited here with a
    // single-document `MissingProperties` error. Interactive collection
    // for missing required values is driven by
    // `wrap::sequence::run_phase_1c_with_schema` against the
    // deduplicated cross-step set.
    let (source, set_overrides, dropped_optionals) =
        composition::drop_invalid_optionals(source, set_overrides);
    emit_dropped_optional_warnings(&dropped_optionals);

    let execution_options = SequenceExecutionOptions {
        fail_fast_override: fail_fast,
    };

    super::wrap::sequence::execute_sequence(
        &source,
        plan,
        &shared,
        set_overrides,
        execution_options,
        verbose,
        shared.perf,
        startup_timings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::{Frontmatter, Markdown};
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn source_with_frontmatter(
        dir: &TempDir,
        frontmatter: &[(&str, serde_json::Value)],
    ) -> claudine::composition::ResolvedCompositionSource {
        let file = dir.path().join("seq.md");
        let mut fm = Frontmatter::new();
        for (key, value) in frontmatter {
            fm.insert(key, value.clone()).unwrap();
        }
        // Provide a minimal sequence so the document is otherwise valid.
        if !frontmatter.iter().any(|(k, _)| *k == "sequence") {
            fm.insert("sequence", json!(["step-1"])).unwrap();
        }
        let md = Markdown::with_frontmatter(fm, "body");
        fs::write(&file, md.as_string()).unwrap();
        claudine::composition::resolve_composition_source(file.to_string_lossy().as_ref())
            .unwrap()
    }

    #[test]
    fn sequence_rejects_interactive_true() {
        let dir = TempDir::new().unwrap();
        let source = source_with_frontmatter(&dir, &[("interactive", json!(true))]);
        let err = reject_sequence_interactive(&source)
            .expect_err("interactive: true must be rejected for sequence");
        assert!(
            matches!(err, CompositionError::SequenceInteractiveRejected(_)),
            "expected SequenceInteractiveRejected, got {err:?}"
        );
    }

    #[test]
    fn sequence_allows_interactive_false_null_and_absent() {
        let dir = TempDir::new().unwrap();
        let cases: Vec<Vec<(&str, serde_json::Value)>> = vec![
            vec![("interactive", json!(false))],
            vec![("interactive", json!(null))],
            vec![],
        ];
        for case in cases {
            let source = source_with_frontmatter(&dir, &case);
            assert!(
                reject_sequence_interactive(&source).is_ok(),
                "sequence should proceed for frontmatter {case:?}"
            );
        }
    }

    #[test]
    fn sequence_rejects_interactive_wrong_type() {
        let dir = TempDir::new().unwrap();
        let source = source_with_frontmatter(&dir, &[("interactive", json!("yes"))]);
        let err = reject_sequence_interactive(&source)
            .expect_err("interactive string must be rejected for sequence");
        assert!(
            matches!(err, CompositionError::InteractiveHintWrongType(_)),
            "expected InteractiveHintWrongType, got {err:?}"
        );
    }
}
