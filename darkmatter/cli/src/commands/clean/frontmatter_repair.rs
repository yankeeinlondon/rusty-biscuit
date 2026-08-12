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
use biscuit_file::{
    SourceSpan, YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlRepair, analyze_yaml,
    apply_edit_set,
};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::schemas::{CleanSchemaConfig, CleanSchemaContext};
use darkmatter::markdown::span::line_col_of_offset;
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
/// This identifies the diagnostic's owner, not its coordinate space. All
/// diagnostics are projected into authored YAML coordinates before they are
/// retained, including findings unlocked by a syntax repair or schema pass.
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
    /// The analysis tier that owns this authored-coordinate diagnostic.
    pub stage: CleanStage,
    #[serde(flatten)]
    pub diagnostic: YamlDiagnostic,
}

/// The version-1 `md clean --json` STDOUT envelope.
#[derive(Debug, Serialize)]
pub struct CleanJsonReport {
    pub version: u8,
    pub source: CleanJsonSource,
    pub frontmatter: CleanJsonFrontmatter,
    pub diagnostics: Vec<CleanJsonDiagnostic>,
    pub applied: Vec<CleanJsonRepair>,
    /// Whether either frontmatter repair or Markdown cleanup changed the
    /// document emitted by this invocation.
    pub changed: bool,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonSource {
    pub kind: CleanJsonSourceKind,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanJsonSourceKind {
    File,
    Stdin,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonFrontmatter {
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<CleanJsonSpan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanJsonSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonPositionedSpan {
    pub start: usize,
    pub end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonDiagnostic {
    pub code: YamlDiagnosticCode,
    pub classification: YamlCertainty,
    pub message: String,
    pub span: CleanJsonPositionedSpan,
    pub repairs: Vec<CleanJsonRepair>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanJsonRepair {
    pub span: CleanJsonSpan,
    pub replacement: String,
    pub explanation: String,
}

/// Outcome of the raw-source frontmatter pass.
pub struct FrontmatterRepair {
    /// The document with deterministic frontmatter repairs spliced in.
    pub source: String,
    /// YAML span in the exact document supplied by the caller.
    pub frontmatter_span: Option<SourceSpan>,
    /// Whether any repair changed the frontmatter text.
    pub repaired: bool,
    /// Every finding from both tiers.
    pub diagnostics: Vec<CleanDiagnostic>,
    /// Every repair actually accepted by the edit-set utility.
    applied: Vec<AppliedRepair>,
}

struct AppliedRepair {
    repair: YamlRepair,
    document_relative: bool,
}

/// Maps offsets from each repaired YAML revision back to the authored YAML.
///
/// A pass records only accepted edits. Unchanged regions map exactly after
/// accounting for their cumulative length delta. A later span inside synthetic
/// replacement text maps conservatively to the authored range that replacement
/// superseded, because no more precise authored range exists.
#[derive(Default)]
struct AuthoredCoordinateMap {
    passes: Vec<EditProjectionPass>,
}

struct EditProjectionPass {
    edits: Vec<ProjectedEdit>,
}

struct ProjectedEdit {
    input: SourceSpan,
    output: SourceSpan,
    delta_before: isize,
}

#[derive(Clone, Copy)]
enum OffsetBias {
    Start,
    End,
}

impl AuthoredCoordinateMap {
    fn project_span(&self, span: SourceSpan) -> SourceSpan {
        let mut start = span.start;
        let mut end = span.end;
        for pass in self.passes.iter().rev() {
            start = pass.project_offset(start, OffsetBias::Start);
            end = pass.project_offset(end, OffsetBias::End);
        }
        start..end
    }

    fn project_repair(&self, repair: &YamlRepair) -> YamlRepair {
        let mut repair = repair.clone();
        repair.span = self.project_span(repair.span);
        repair
    }

    fn project_diagnostic(&self, diagnostic: YamlDiagnostic) -> YamlDiagnostic {
        let mut diagnostic = diagnostic;
        diagnostic.span = self.project_span(diagnostic.span);
        diagnostic.repairs = diagnostic
            .repairs
            .iter()
            .map(|repair| self.project_repair(repair))
            .collect();
        diagnostic
    }

    fn record(&mut self, repairs: &[YamlRepair]) {
        if !repairs.is_empty() {
            self.passes.push(EditProjectionPass::new(repairs));
        }
    }
}

impl EditProjectionPass {
    fn new(repairs: &[YamlRepair]) -> Self {
        let mut delta = 0_isize;
        let edits = repairs
            .iter()
            .map(|repair| {
                let output_start = repair
                    .span
                    .start
                    .checked_add_signed(delta)
                    .expect("accepted edit deltas must remain within the YAML source");
                let output_end = output_start + repair.replacement.len();
                let edit = ProjectedEdit {
                    input: repair.span.clone(),
                    output: output_start..output_end,
                    delta_before: delta,
                };
                delta += repair.replacement.len() as isize
                    - (repair.span.end - repair.span.start) as isize;
                edit
            })
            .collect();
        Self { edits }
    }

    fn project_offset(&self, offset: usize, bias: OffsetBias) -> usize {
        for edit in &self.edits {
            if offset < edit.output.start {
                return offset
                    .checked_add_signed(-edit.delta_before)
                    .expect("projected offset must remain within the prior YAML revision");
            }
            if offset <= edit.output.end {
                if offset == edit.output.start {
                    return match bias {
                        OffsetBias::Start if edit.output.is_empty() => edit.input.end,
                        OffsetBias::Start => edit.input.start,
                        OffsetBias::End if edit.output.is_empty() => edit.input.end,
                        OffsetBias::End => edit.input.start,
                    };
                }
                if offset == edit.output.end {
                    return edit.input.end;
                }
                return match bias {
                    OffsetBias::Start => edit.input.start,
                    OffsetBias::End => edit.input.end,
                };
            }
        }

        let delta = self
            .edits
            .last()
            .map_or(0, |edit| {
                edit.delta_before + edit.output.len() as isize - edit.input.len() as isize
            });
        offset
            .checked_add_signed(-delta)
            .expect("projected offset must remain within the prior YAML revision")
    }
}

impl FrontmatterRepair {
    /// The zero-cost outcome for documents with no frontmatter, or an empty
    /// frontmatter block: no YAML analysis, no schema resolution, and — most
    /// expensively — no trigger-schema git-root ancestor walk.
    fn untouched(source: &str) -> Self {
        Self {
            source: source.to_string(),
            frontmatter_span: None,
            repaired: false,
            diagnostics: Vec::new(),
            applied: Vec::new(),
        }
    }

    /// Builds the `--json` envelope for this outcome.
    pub fn json_report(
        &self,
        path: Option<PathBuf>,
        document_source: &str,
        changed: bool,
    ) -> CleanJsonReport {
        let source = CleanJsonSource {
            kind: if path.is_some() {
                CleanJsonSourceKind::File
            } else {
                CleanJsonSourceKind::Stdin
            },
            path,
        };
        let frontmatter = CleanJsonFrontmatter {
            present: self.frontmatter_span.is_some(),
            span: self.frontmatter_span.as_ref().map(json_span),
        };
        let yaml_offset = self
            .frontmatter_span
            .as_ref()
            .map_or(0, |span| span.start);

        CleanJsonReport {
            version: 1,
            source,
            frontmatter,
            diagnostics: self
                .diagnostics
                .iter()
                .map(|entry| json_diagnostic(entry, yaml_offset, document_source))
                .collect(),
            applied: self
                .applied
                .iter()
                .map(|applied| {
                    json_repair(
                        &applied.repair,
                        if applied.document_relative {
                            0
                        } else {
                            yaml_offset
                        },
                    )
                })
                .collect(),
            changed,
        }
    }
}

fn json_diagnostic(
    entry: &CleanDiagnostic,
    yaml_offset: usize,
    document_source: &str,
) -> CleanJsonDiagnostic {
    let start = yaml_offset + entry.diagnostic.span.start;
    let end = yaml_offset + entry.diagnostic.span.end;
    let (start_line, start_column) = line_col_of_offset(document_source, start);
    let (end_line, end_column) = line_col_of_offset(document_source, end);

    CleanJsonDiagnostic {
        code: entry.diagnostic.code,
        classification: entry.diagnostic.classification,
        message: entry.diagnostic.message.clone(),
        span: CleanJsonPositionedSpan {
            start,
            end,
            start_line,
            start_column,
            end_line,
            end_column,
        },
        repairs: entry
            .diagnostic
            .repairs
            .iter()
            .map(|repair| json_repair(repair, yaml_offset))
            .collect(),
    }
}

fn json_repair(repair: &YamlRepair, yaml_offset: usize) -> CleanJsonRepair {
    CleanJsonRepair {
        span: CleanJsonSpan {
            start: yaml_offset + repair.span.start,
            end: yaml_offset + repair.span.end,
        },
        replacement: repair.replacement.clone(),
        explanation: repair.explanation.clone(),
    }
}

fn json_span(span: &SourceSpan) -> CleanJsonSpan {
    CleanJsonSpan {
        start: span.start,
        end: span.end,
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
    let original_source = source;
    let Some(original_extraction) = extract_frontmatter_block(source)? else {
        return Ok(FrontmatterRepair::untouched(source));
    };
    let frontmatter_span = original_extraction.yaml_span;
    let bom_repair = source.starts_with('\u{feff}').then(|| AppliedRepair {
        repair: YamlRepair {
            span: 0..'\u{feff}'.len_utf8(),
            replacement: String::new(),
            explanation: "remove the UTF-8 byte-order mark at document start".to_string(),
        },
        document_relative: true,
    });
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let extraction = extract_frontmatter_block(source)?
        .expect("removing a recognized BOM must preserve the frontmatter block");
    if extraction.yaml.trim().is_empty() {
        let repaired_source = splice_frontmatter(source, &extraction, extraction.yaml);
        return Ok(FrontmatterRepair {
            repaired: repaired_source != original_source,
            source: repaired_source,
            frontmatter_span: Some(frontmatter_span),
            diagnostics: Vec::new(),
            applied: bom_repair.into_iter().collect(),
        });
    }

    let authored = extraction.yaml.to_string();
    let mut coordinate_map = AuthoredCoordinateMap::default();

    // One analysis serves both the diagnostic and the repair view; calling
    // `diagnose()` and `repair_candidates()` separately would rescan the
    // source. Every repair reaching us has already passed biscuit-file's
    // parse-equivalence proof, and an already-clean block produces no
    // candidates, so it is never reparsed.
    let analysis = analyze_yaml(&authored);
    let mut diagnostics: Vec<CleanDiagnostic> = syntax_diagnostics(&analysis);
    let outcome = analysis.apply();
    let first_pass_repairs = outcome.audit.applied;
    let mut applied: Vec<AppliedRepair> = bom_repair
        .into_iter()
        .chain(first_pass_repairs.iter().map(|repair| AppliedRepair {
            repair: coordinate_map.project_repair(repair),
            document_relative: false,
        }))
        .collect();
    coordinate_map.record(&first_pass_repairs);
    let mut yaml = outcome.source;

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
        diagnostics.extend(
            new_findings(&rescan, &diagnostics)
                .into_iter()
                .map(|mut entry| {
                    entry.diagnostic = coordinate_map.project_diagnostic(entry.diagnostic);
                    entry
                }),
        );
        let outcome = rescan.apply();
        let rescan_repairs = outcome.audit.applied;
        applied.extend(rescan_repairs.iter().map(|repair| AppliedRepair {
            repair: coordinate_map.project_repair(repair),
            document_relative: false,
        }));
        coordinate_map.record(&rescan_repairs);
        yaml = outcome.source;
    }

    // The schema tier needs a parsed document, so it is unreachable until the
    // syntax tier has restored parseability. This ordering is also what keeps
    // schema resolution off the hot path for unparseable input.
    if parses(&yaml) {
        let after_syntax = splice_frontmatter(source, &extraction, &yaml);
        if let Ok(markdown) = build_markdown(after_syntax, document_path) {
            let context = resolve_schema_context(flags, document_path)?;
            let schema_analysis = context.analyze(&markdown, &yaml)?;

            let auto_apply: Vec<YamlRepair> = schema_analysis
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
                .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
                .collect();

            diagnostics.extend(schema_analysis.into_diagnostics().into_iter().map(|diagnostic| {
                CleanDiagnostic {
                    stage: CleanStage::Schema,
                    diagnostic: coordinate_map.project_diagnostic(diagnostic),
                }
            }));

            if !auto_apply.is_empty() {
                let outcome = apply_edit_set(&yaml, &auto_apply);
                if outcome.changed() {
                    applied.extend(outcome.audit.applied.iter().map(|repair| {
                        AppliedRepair {
                            repair: coordinate_map.project_repair(repair),
                            document_relative: false,
                        }
                    }));
                    coordinate_map.record(&outcome.audit.applied);
                    yaml = outcome.source;
                }
            }
        }
    }

    let repaired_source = splice_frontmatter(source, &extraction, &yaml);
    extract_frontmatter_block(&repaired_source)?
        .expect("reconstructing frontmatter must preserve its delimiters");
    Ok(FrontmatterRepair {
        repaired: repaired_source != original_source,
        frontmatter_span: Some(frontmatter_span),
        source: repaired_source,
        diagnostics,
        applied,
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

fn splice_frontmatter(
    source: &str,
    extraction: &darkmatter::markdown::FrontmatterExtraction<'_>,
    yaml: &str,
) -> String {
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..extraction.yaml_span.start]);
    out.push_str(yaml);
    out.push_str(&source[extraction.yaml_span.end..extraction.body_span.start]);
    out.push_str(&source[extraction.body_span.clone()]);
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
