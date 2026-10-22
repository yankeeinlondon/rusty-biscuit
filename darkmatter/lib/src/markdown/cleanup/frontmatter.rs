//! Raw-source frontmatter repair, shared by `md clean` and DMLS formatting.
//!
//! Frontmatter that does not parse still has to be repairable, so nothing here
//! depends on a constructed [`Markdown`] until the syntax tier has restored
//! parseability. The pipeline locates the frontmatter block by byte span,
//! analyzes only that block, and splices repaired YAML back over the original
//! span. Repairs are therefore format-preserving: comments, key order, and
//! quote style outside the patched ranges survive byte-for-byte, which
//! reserializing a parsed `Value` would destroy.
//!
//! Body ```` ```yaml ```` fenced blocks are never inspected or mutated. They
//! are frequently intentional broken-YAML examples.
//!
//! The library owns the repair passes and their authored-coordinate results;
//! callers own context capture (through the lazy schema closure), terminal
//! reporting, and any wire format.

use std::path::Path;

use biscuit_file::serde_yaml_ng;
use biscuit_file::{SourceSpan, YamlAnalysis, YamlDiagnostic, YamlRepair, analyze_yaml, apply_edit_set};

use super::diagnostic::{CleanCode, CleanDiagnostic, CleanStage, shift, shift_repair};
use crate::markdown::compose::ComposeSource;
use crate::markdown::{FrontmatterExtraction, extract_frontmatter_block};
use crate::markdown::schemas::{CleanSchemaConfig, SchemaError};
use crate::markdown::{Markdown, MarkdownError};

/// Why [`repair_frontmatter`] could not produce a result.
///
/// YAML that cannot be repaired is not an error: it is returned unchanged with
/// its diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum FrontmatterRepairError<E> {
    /// The document opens with a near-miss frontmatter fence.
    #[error(transparent)]
    Extraction(MarkdownError),
    /// A referenced schema is missing, unreadable, or invalid.
    #[error(transparent)]
    Schema(SchemaError),
    /// The caller's schema closure failed to build its configuration.
    #[error("{0}")]
    Context(E),
}

/// Outcome of the raw-source frontmatter pass.
#[derive(Debug, Clone)]
pub struct FrontmatterRepair {
    /// The document with accepted frontmatter repairs spliced in.
    pub source: String,
    /// YAML span in the original document, BOM included in the offset.
    pub frontmatter_span: Option<SourceSpan>,
    /// Whether `source` differs from the original document.
    pub repaired: bool,
    /// Every finding from both tiers, in authored document coordinates, in
    /// pass order: first syntax pass, rescan-only findings, then schema.
    pub diagnostics: Vec<CleanDiagnostic>,
    /// Repairs actually accepted, in authored document coordinates.
    pub applied: Vec<YamlRepair>,
    /// The final repaired YAML, kept so settings can be read from it even
    /// after [`Self::discard_repairs`].
    candidate_yaml: Option<String>,
    coordinate_map: AuthoredCoordinateMap,
}

/// YAML text to read formatting settings from; see
/// [`FrontmatterRepair::settings_yaml`].
#[derive(Debug, Clone, Copy)]
pub struct SettingsYaml<'a> {
    /// The YAML text between the delimiters.
    pub yaml: &'a str,
    /// `true` when the authored YAML does not parse and this is the repaired
    /// candidate, whose offsets are synthetic until passed through
    /// [`Self::authored_span`].
    pub from_candidate: bool,
    /// Document offset of the authored YAML.
    pub authored_offset: usize,
    projection: Option<&'a AuthoredCoordinateMap>,
}

impl SettingsYaml<'_> {
    /// Maps a span relative to [`Self::yaml`] into authored document
    /// coordinates. A candidate span inside synthetic replacement text maps to
    /// the authored range that replacement superseded.
    #[must_use]
    pub fn authored_span(&self, span: SourceSpan) -> SourceSpan {
        let span = match self.projection {
            Some(map) => map.project_span(span),
            None => span,
        };
        shift(&span, self.authored_offset)
    }
}

impl FrontmatterRepair {
    /// The outcome for a document with no frontmatter block.
    fn untouched(source: &str) -> Self {
        Self {
            source: source.to_string(),
            frontmatter_span: None,
            repaired: false,
            diagnostics: Vec::new(),
            applied: Vec::new(),
            candidate_yaml: None,
            coordinate_map: AuthoredCoordinateMap::default(),
        }
    }

    /// Drops every accepted repair while keeping the diagnostics as
    /// report-only findings.
    ///
    /// This is the path for `style.markdown.frontmatter: false` and for a
    /// declined clean: the caller must emit `original` byte-for-byte,
    /// including comments, line endings, BOM, and delimiter spacing, yet still
    /// tell the author what a repair would have fixed.
    #[must_use]
    pub fn discard_repairs(mut self, original: &str) -> Self {
        self.source = original.to_string();
        self.repaired = false;
        self.applied.clear();
        self
    }

    /// The YAML to read `style.markdown` from.
    ///
    /// ## Returns
    ///
    /// The authored YAML in `original` when it parses; otherwise the repaired
    /// candidate when that parses (with `from_candidate` set); `None` without
    /// frontmatter or when neither parses.
    #[must_use]
    pub fn settings_yaml<'a>(&'a self, original: &'a str) -> Option<SettingsYaml<'a>> {
        let span = self.frontmatter_span.as_ref()?;
        let authored = original.get(span.clone())?;
        if parses(authored) {
            return Some(SettingsYaml {
                yaml: authored,
                from_candidate: false,
                authored_offset: span.start,
                projection: None,
            });
        }
        let candidate = self.candidate_yaml.as_deref()?;
        parses(candidate).then_some(SettingsYaml {
            yaml: candidate,
            from_candidate: true,
            authored_offset: span.start,
            projection: Some(&self.coordinate_map),
        })
    }
}

/// Analyzes and repairs a document's frontmatter block in raw source form.
///
/// `schema` is invoked at most once, and only when the schema tier runs:
/// non-empty frontmatter whose YAML parses after syntax repair. A document
/// with absent or empty frontmatter therefore never builds a resolution
/// context or resolves a schema. `document_path: None` (stdin, untitled
/// buffers) disables schema-trigger discovery.
///
/// ## Returns
///
/// The document with every auto-applicable repair spliced over the original
/// frontmatter span, plus every diagnostic both tiers produced. Findings that
/// are not auto-applicable are reported but never mutate the source.
///
/// ## Errors
///
/// - [`FrontmatterRepairError::Extraction`] for a near-miss fence.
/// - [`FrontmatterRepairError::Context`] when `schema` fails.
/// - [`FrontmatterRepairError::Schema`] when schema resolution fails,
///   including a missing or unreadable referenced schema.
///
/// YAML that cannot be repaired is returned unchanged; building a
/// [`Markdown`] from it is what fails.
pub fn repair_frontmatter<E>(
    source: &str,
    document_path: Option<&Path>,
    schema: impl FnOnce() -> Result<CleanSchemaConfig, E>,
) -> Result<FrontmatterRepair, FrontmatterRepairError<E>> {
    let original_source = source;
    let Some(original_extraction) =
        extract_frontmatter_block(source).map_err(FrontmatterRepairError::Extraction)?
    else {
        return Ok(FrontmatterRepair::untouched(source));
    };
    let frontmatter_span = original_extraction.yaml_span;
    let yaml_offset = frontmatter_span.start;
    let bom_repair = source.starts_with('\u{feff}').then(|| YamlRepair {
        span: 0..'\u{feff}'.len_utf8(),
        replacement: String::new(),
        explanation: "remove the UTF-8 byte-order mark at document start".to_string(),
    });
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let extraction = extract_frontmatter_block(source)
        .map_err(FrontmatterRepairError::Extraction)?
        .expect("removing a recognized BOM must preserve the frontmatter block");
    if extraction.yaml.trim().is_empty() {
        let repaired_source = splice_frontmatter(source, &extraction, extraction.yaml);
        return Ok(FrontmatterRepair {
            repaired: repaired_source != original_source,
            source: repaired_source,
            frontmatter_span: Some(frontmatter_span),
            diagnostics: Vec::new(),
            applied: bom_repair.into_iter().collect(),
            candidate_yaml: Some(extraction.yaml.to_string()),
            coordinate_map: AuthoredCoordinateMap::default(),
        });
    }

    let authored = extraction.yaml.to_string();
    let mut coordinate_map = AuthoredCoordinateMap::default();
    let to_document = |map: &AuthoredCoordinateMap, repair: &YamlRepair| {
        shift_repair(map.project_repair(repair), yaml_offset)
    };

    // One analysis serves both the diagnostic and the repair view; calling
    // `diagnose()` and `repair_candidates()` separately would rescan the
    // source. Every repair reaching us has already passed biscuit-file's
    // parse-equivalence proof, and an already-clean block produces no
    // candidates, so it is never reparsed.
    let analysis = analyze_yaml(&authored);
    let mut diagnostics: Vec<CleanDiagnostic> = syntax_diagnostics(&analysis, yaml_offset);
    let outcome = analysis.apply();
    let first_pass_repairs = outcome.audit.applied;
    let mut applied: Vec<YamlRepair> = bom_repair
        .into_iter()
        .chain(
            first_pass_repairs
                .iter()
                .map(|repair| to_document(&coordinate_map, repair)),
        )
        .collect();
    coordinate_map.record(&first_pass_repairs);
    let mut yaml = outcome.source;

    // Parse-equivalence-gated repairs (whitespace, normalization) cannot be
    // proven while the block is unparseable, because there is no original
    // `Value` to compare a candidate against. Restoring parseability unlocks
    // them, so a block whose first pass changed something gets one more scan;
    // without it, clean would not be a fixed point. The rescan is gated on an
    // actual edit, so an already-clean block is still analyzed exactly once.
    if yaml != authored && parses(&yaml) {
        let rescan = analyze_yaml(&yaml);
        // The rescan sees the *repaired* text, so a finding it repeats from
        // the first pass comes back with a shifted span. Only findings the
        // rescan genuinely unlocked are kept, so no suggestion is reported
        // twice.
        diagnostics.extend(new_findings(&rescan, &diagnostics).into_iter().map(
            |diagnostic| {
                CleanDiagnostic::from_yaml(
                    CleanStage::Syntax,
                    coordinate_map.project_diagnostic(diagnostic),
                    yaml_offset,
                )
            },
        ));
        let outcome = rescan.apply();
        let rescan_repairs = outcome.audit.applied;
        applied.extend(
            rescan_repairs
                .iter()
                .map(|repair| to_document(&coordinate_map, repair)),
        );
        coordinate_map.record(&rescan_repairs);
        yaml = outcome.source;
    }

    // The schema tier needs a parsed document, so it is unreachable until the
    // syntax tier has restored parseability. This ordering is also what keeps
    // schema resolution off the hot path for unparseable input.
    if parses(&yaml) {
        let after_syntax = splice_frontmatter(source, &extraction, &yaml);
        if let Ok(markdown) = build_markdown(after_syntax, document_path) {
            let config = schema().map_err(FrontmatterRepairError::Context)?;
            let context = config
                .resolve(document_path)
                .map_err(FrontmatterRepairError::Schema)?;
            let schema_analysis = context
                .analyze(&markdown, &yaml)
                .map_err(FrontmatterRepairError::Schema)?;

            let auto_apply: Vec<YamlRepair> = schema_analysis
                .diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
                .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
                .collect();

            diagnostics.extend(schema_analysis.into_diagnostics().into_iter().map(
                |diagnostic| {
                    CleanDiagnostic::from_yaml(
                        CleanStage::Schema,
                        coordinate_map.project_diagnostic(diagnostic),
                        yaml_offset,
                    )
                },
            ));

            if !auto_apply.is_empty() {
                let outcome = apply_edit_set(&yaml, &auto_apply);
                if outcome.changed() {
                    applied.extend(
                        outcome
                            .audit
                            .applied
                            .iter()
                            .map(|repair| to_document(&coordinate_map, repair)),
                    );
                    coordinate_map.record(&outcome.audit.applied);
                    yaml = outcome.source;
                }
            }
        }
    }

    let repaired_source = splice_frontmatter(source, &extraction, &yaml);
    extract_frontmatter_block(&repaired_source)
        .map_err(FrontmatterRepairError::Extraction)?
        .expect("reconstructing frontmatter must preserve its delimiters");
    Ok(FrontmatterRepair {
        repaired: repaired_source != original_source,
        frontmatter_span: Some(frontmatter_span),
        source: repaired_source,
        diagnostics,
        applied,
        candidate_yaml: Some(yaml),
        coordinate_map,
    })
}

/// The rescan's findings minus everything `seen` already reported.
///
/// A finding is a restatement when its code and message match one already
/// recorded; the message embeds the offending key, so distinct keys with the
/// same code stay distinct.
fn new_findings(rescan: &YamlAnalysis, seen: &[CleanDiagnostic]) -> Vec<YamlDiagnostic> {
    rescan
        .diagnostics()
        .iter()
        .filter(|entry| {
            !seen.iter().any(|prior| {
                prior.code == CleanCode::Yaml(entry.code) && prior.message == entry.message
            })
        })
        .cloned()
        .collect()
}

fn syntax_diagnostics(analysis: &YamlAnalysis, yaml_offset: usize) -> Vec<CleanDiagnostic> {
    analysis
        .diagnostics()
        .iter()
        .cloned()
        .map(|diagnostic| CleanDiagnostic::from_yaml(CleanStage::Syntax, diagnostic, yaml_offset))
        .collect()
}

fn splice_frontmatter(source: &str, extraction: &FrontmatterExtraction<'_>, yaml: &str) -> String {
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
/// `$schema` values and schema-trigger roots relative to the document, and
/// `try_from_content` has no path to anchor them to.
fn build_markdown(source: String, document_path: Option<&Path>) -> Result<Markdown, MarkdownError> {
    let markdown = Markdown::try_from_content(source)?;
    Ok(match document_path {
        Some(path) => markdown.with_source(ComposeSource::File(path.to_path_buf())),
        None => markdown,
    })
}

/// Maps offsets from each repaired YAML revision back to the authored YAML.
///
/// A pass records only accepted edits. Unchanged regions map exactly after
/// accounting for their cumulative length delta. A later span inside synthetic
/// replacement text maps conservatively to the authored range that replacement
/// superseded, because no more precise authored range exists.
#[derive(Debug, Clone, Default)]
pub(crate) struct AuthoredCoordinateMap {
    passes: Vec<EditProjectionPass>,
}

#[derive(Debug, Clone)]
struct EditProjectionPass {
    edits: Vec<ProjectedEdit>,
}

#[derive(Debug, Clone)]
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
    pub(crate) fn project_span(&self, span: SourceSpan) -> SourceSpan {
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

        let delta = self.edits.last().map_or(0, |edit| {
            edit.delta_before + edit.output.len() as isize - edit.input.len() as isize
        });
        offset
            .checked_add_signed(-delta)
            .expect("projected offset must remain within the prior YAML revision")
    }
}
