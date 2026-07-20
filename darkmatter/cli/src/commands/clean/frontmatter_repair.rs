//! Raw-source frontmatter analysis and repair for `md clean`.
//!
//! `md clean` has to fix frontmatter that does not parse, so nothing here may
//! depend on a constructed [`Markdown`]. The pipeline runs on the document's
//! raw text: it locates the frontmatter block by byte span, analyzes only that
//! block, and splices repaired YAML back over the original span. Repairs are
//! therefore format-preserving — comments, key order, and quote style outside
//! the patched ranges survive byte-for-byte, which reserializing a parsed
//! `Value` would destroy.
//!
//! Body ```` ```yaml ```` fenced blocks are never inspected or mutated. They
//! are frequently intentional broken-YAML examples, and repairing them would
//! corrupt the document.
//!
//! See `darkmatter/features/2026-07-14-invalid-frontmatter/spec.md`.

use biscuit_file::serde_yaml_ng;
use biscuit_file::{YamlDiagnostic, YamlRepair, analyze_yaml, apply_edit_set};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::schemas::{CleanSchemaConfig, CleanSchemaContext};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Schema controls for `md clean`, mirroring the `md compose` flags of the
/// same name.
#[derive(Debug, Clone, Default)]
pub struct CleanSchemaFlags {
    /// `--schema PATH`: an explicit schema that replaces the document's own
    /// `$schema` layer. Baseline and trigger layers still apply beneath it.
    pub schema: Option<PathBuf>,
    /// `--baseline-schema PATH`
    pub baseline_schema: Option<PathBuf>,
    /// `--no-baseline-schema`
    pub no_baseline_schema: bool,
    /// `--no-trigger-schemas`
    pub no_trigger_schemas: bool,
}

/// Which analysis tier produced a diagnostic.
///
/// The two tiers run against different text, so this is what tells a consumer
/// which string a diagnostic's `span` indexes: `syntax` spans index the
/// frontmatter YAML as authored, `schema` spans index that YAML after the
/// syntax tier's repairs were applied.
///
/// The syntax tier can run a second pass once its own repairs restore
/// parseability. A finding that pass already reported is dropped as a
/// restatement, so the only `syntax` spans carrying post-repair coordinates
/// belong to findings the first pass could not see at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanStage {
    /// Schema-free analysis from `biscuit-file`.
    Syntax,
    /// Schema-aware analysis from Darkmatter's `SimplifiedSchema` engine.
    Schema,
}

/// A `biscuit-file` diagnostic tagged with the tier that produced it.
#[derive(Debug, Clone, Serialize)]
pub struct CleanDiagnostic {
    /// Which text `diagnostic.span` indexes. See [`CleanStage`].
    pub stage: CleanStage,
    #[serde(flatten)]
    pub diagnostic: YamlDiagnostic,
}

/// The `md clean --json` STDOUT envelope.
///
/// The per-diagnostic shape (`code`, `span`, `classification`, `message`,
/// `repairs[]`) is `biscuit_file::YamlDiagnostic`'s own `Serialize`, so the
/// wire format stays pinned to the shared diagnostic vocabulary rather than a
/// CLI-local copy of it. Spans serialize as `{"start": N, "end": M}` byte
/// offsets.
#[derive(Debug, Serialize)]
pub struct CleanJsonReport {
    /// Document analyzed, or `null` when the input came from stdin.
    pub path: Option<PathBuf>,
    /// Byte offset of the frontmatter YAML block within the document. Add it
    /// to a `syntax`-stage span to project that span into document
    /// coordinates. `null` when the document has no frontmatter.
    pub frontmatter_offset: Option<usize>,
    /// Whether any repair changed the frontmatter text.
    pub repaired: bool,
    /// Every finding from both tiers, in the order they were produced.
    pub diagnostics: Vec<CleanDiagnostic>,
}

/// Outcome of the raw-source frontmatter pass.
pub struct FrontmatterRepair {
    /// The document with deterministic frontmatter repairs spliced in.
    pub source: String,
    /// Byte offset of the frontmatter YAML block within [`Self::source`].
    pub frontmatter_offset: Option<usize>,
    /// Whether any repair changed the frontmatter text.
    pub repaired: bool,
    /// Every finding from both tiers.
    pub diagnostics: Vec<CleanDiagnostic>,
}

impl FrontmatterRepair {
    /// The zero-cost outcome for documents with no frontmatter, or an empty
    /// frontmatter block: no YAML analysis, no schema resolution, and — most
    /// expensively — no trigger-schema git-root ancestor walk.
    fn untouched(source: &str) -> Self {
        Self {
            source: source.to_string(),
            frontmatter_offset: None,
            repaired: false,
            diagnostics: Vec::new(),
        }
    }

    /// Builds the `--json` envelope for this outcome.
    pub fn json_report(self, path: Option<PathBuf>) -> CleanJsonReport {
        CleanJsonReport {
            path,
            frontmatter_offset: self.frontmatter_offset,
            repaired: self.repaired,
            diagnostics: self.diagnostics,
        }
    }
}

/// Analyzes and repairs a document's frontmatter block in raw source form.
///
/// ## Returns
///
/// The document with all auto-applicable repairs spliced over the original
/// frontmatter span, plus every diagnostic both tiers produced. Findings that
/// are not auto-applicable are reported but never mutate the source.
///
/// ## Errors
///
/// Propagates [`extract_frontmatter_block`]'s near-miss fence error and any
/// schema-resolution failure. YAML that cannot be repaired is *not* an error
/// here — it is returned unchanged, and the caller fails when it tries to
/// build a [`Markdown`] from it, which keeps the existing exit-code contract.
pub fn repair_frontmatter(
    source: &str,
    document_path: Option<&Path>,
    flags: &CleanSchemaFlags,
) -> Result<FrontmatterRepair> {
    let Some(extraction) = extract_frontmatter_block(source)? else {
        return Ok(FrontmatterRepair::untouched(source));
    };
    if extraction.yaml.trim().is_empty() {
        return Ok(FrontmatterRepair::untouched(source));
    }

    let yaml_span = extraction.yaml_span.clone();
    let authored = extraction.yaml.to_string();

    // One analysis serves both the diagnostic and the repair view; calling
    // `diagnose()` and `repair_candidates()` separately would rescan the
    // source. Every repair reaching us has already passed biscuit-file's
    // parse-equivalence proof, and an already-clean block produces no
    // candidates, so it is never reparsed.
    let analysis = analyze_yaml(&authored);
    let mut diagnostics: Vec<CleanDiagnostic> = syntax_diagnostics(&analysis);
    let mut yaml = analysis.apply().source;

    // Parse-equivalence-gated repairs (whitespace, normalization) cannot be
    // proven while the block is unparseable, because there is no original
    // `Value` to compare a candidate against. Restoring parseability unlocks
    // them, so a block whose first pass changed something gets one more scan —
    // without it, `md clean` would not be a fixed point: the repairs skipped
    // on run 1 would land on run 2. The rescan is gated on an actual edit, so
    // an already-clean block is still analyzed exactly once.
    if yaml != authored && parses(&yaml) {
        let rescan = analyze_yaml(&yaml);
        // The rescan sees the *repaired* text, so any finding it repeats from
        // the first pass comes back with a shifted span. Emitting both would
        // print every report-only suggestion twice and leave a consumer unable
        // to tell which of two `syntax` spans is in authored coordinates. Only
        // findings the rescan genuinely unlocked are kept.
        diagnostics.extend(new_findings(&rescan, &diagnostics));
        yaml = rescan.apply().source;
    }

    // The schema tier needs a parsed document, so it is unreachable until the
    // syntax tier has restored parseability. This ordering is also what keeps
    // schema resolution off the hot path for unparseable input.
    if parses(&yaml) {
        let after_syntax = splice(source, &yaml_span, &yaml);
        if let Ok(markdown) = build_markdown(after_syntax, document_path) {
            let context = resolve_schema_context(flags, document_path)?;
            let schema_analysis = context.analyze(&markdown, &yaml)?;

            let auto_apply: Vec<YamlRepair> = schema_analysis
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
                .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
                .collect();

            diagnostics.extend(schema_analysis.into_diagnostics().into_iter().map(
                |diagnostic| CleanDiagnostic {
                    stage: CleanStage::Schema,
                    diagnostic,
                },
            ));

            if !auto_apply.is_empty() {
                let outcome = apply_edit_set(&yaml, &auto_apply);
                if outcome.changed() {
                    yaml = outcome.source;
                }
            }
        }
    }

    Ok(FrontmatterRepair {
        repaired: yaml != authored,
        source: splice(source, &yaml_span, &yaml),
        frontmatter_offset: Some(yaml_span.start),
        diagnostics,
    })
}

/// Renders report-only findings to STDERR as suggestions.
///
/// Suggestions deliberately do not affect the exit code: `md clean` runs in
/// pre-commit hooks and agent loops, where a new failure mode would be more
/// disruptive than the smells it reports.
pub fn report_suggestions(diagnostics: &[CleanDiagnostic], terminal: &Terminal) {
    let mut list = UnorderedList::empty();
    let mut reported = 0_usize;

    for entry in diagnostics {
        if entry.diagnostic.classification.is_auto_apply_eligible() {
            continue;
        }
        reported += 1;
        list.add(Prose::new(format!(
            "<b>{}</b>. {}",
            escape_prose(entry.diagnostic.code.as_str()),
            escape_prose(&entry.diagnostic.message)
        )));
    }

    if reported == 0 {
        return;
    }

    // `Prose`/`UnorderedList` render without a trailing newline, so the header
    // and the list need their own line breaks or they arrive glued together.
    eprintln!(
        "{}",
        Prose::new("frontmatter suggestions (not applied)").render(terminal)
    );
    eprintln!("{}", list.render(terminal));
}

/// The rescan's findings minus everything `seen` already reported.
///
/// A finding is a restatement when its code and message match one already
/// recorded; the message embeds the offending key, so distinct keys with the
/// same code stay distinct.
fn new_findings(
    rescan: &biscuit_file::YamlAnalysis,
    seen: &[CleanDiagnostic],
) -> Vec<CleanDiagnostic> {
    syntax_diagnostics(rescan)
        .into_iter()
        .filter(|entry| {
            !seen.iter().any(|prior| {
                prior.diagnostic.code == entry.diagnostic.code
                    && prior.diagnostic.message == entry.diagnostic.message
            })
        })
        .collect()
}

fn syntax_diagnostics(analysis: &biscuit_file::YamlAnalysis) -> Vec<CleanDiagnostic> {
    analysis
        .diagnostics()
        .iter()
        .cloned()
        .map(|diagnostic| CleanDiagnostic {
            stage: CleanStage::Syntax,
            diagnostic,
        })
        .collect()
}

/// Replaces `span` in `source` with `replacement`.
fn splice(source: &str, span: &std::ops::Range<usize>, replacement: &str) -> String {
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..span.start]);
    out.push_str(replacement);
    out.push_str(&source[span.end..]);
    out
}

fn parses(yaml: &str) -> bool {
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml).is_ok()
}

/// Builds the document the schema tier validates against.
///
/// The path is reattached because the schema layer resolves file-reference
/// `$schema` values and trigger-schema roots relative to the document, and
/// `try_from_content` has no path to anchor them to.
fn build_markdown(source: String, document_path: Option<&Path>) -> Result<Markdown> {
    let markdown = Markdown::try_from_content(source)?;
    Ok(match document_path {
        Some(path) => markdown.with_source(ComposeSource::File(path.to_path_buf())),
        None => markdown,
    })
}

/// Resolves the effective schema state for this `clean` invocation.
///
/// Called at most once per run and only behind a non-empty frontmatter block,
/// so the returned context *is* the per-invocation cache the performance
/// contract requires — trigger discovery's ancestor walk to the Git root
/// happens here or not at all.
fn resolve_schema_context(
    flags: &CleanSchemaFlags,
    document_path: Option<&Path>,
) -> Result<CleanSchemaContext> {
    let mut config = CleanSchemaConfig::new();

    if flags.no_baseline_schema || crate::commands::compose::env_disables_baseline_schema() {
        config = config.without_baseline_schema();
    } else if let Some(path) = &flags.baseline_schema {
        config = config.with_baseline_schema_file(crate::io::resolve_file_path(path)?);
    }

    if let Some(schema) = &flags.schema {
        // A string `$schema` is biscuit-file's file-reference form, which is
        // exactly what a path-valued `--schema` means.
        config = config.with_schema_override(serde_json::Value::String(
            schema.to_string_lossy().into_owned(),
        ));
    }

    config = config.with_trigger_schemas(!flags.no_trigger_schemas);
    Ok(config.resolve(document_path)?)
}

/// Neutralizes Prose markup so diagnostic text renders literally.
fn escape_prose(text: &str) -> String {
    text.replace('<', "&lt;").replace('>', "&gt;")
}
