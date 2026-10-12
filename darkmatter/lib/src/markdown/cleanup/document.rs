//! The raw-source clean transaction shared by `md clean` and DMLS formatting.
//!
//! One run, in order:
//!
//! 1. frontmatter repair on the raw text ([`repair_frontmatter`]), before any
//!    `Markdown` exists;
//! 2. one passive read of `style.markdown`, from the authored YAML or, when
//!    that does not parse, the repaired candidate, or else the mapping
//!    Darkmatter's own frontmatter parser loads from the authored block;
//! 3. one resolution of every rule (caller overrides, document, workspace,
//!    default);
//! 4. guarded body formatting ([`clean_body`]), which verifies its final
//!    candidate once;
//! 5. reassembly from the frontmatter bytes being emitted (authored, or
//!    authored plus accepted repairs) and the formatted body.
//!
//! The body check is independent of the frontmatter repair proof. A body
//! refusal discards every candidate edit, frontmatter repairs included, and
//! frontmatter that Darkmatter cannot load is `Invalid` with nothing emitted,
//! so a caller only ever writes the original or a fully accepted result.
//!
//! Strict YAML is not the acceptance test. A block it rejects but
//! Darkmatter's document parser loads (a bare `{{ … }}` value, tab
//! indentation) is a supported document: it is emitted byte-for-byte, with
//! no repair applied, and its body is formatted.

use std::path::Path;

use biscuit_file::{SourceSpan, YamlRepair, serde_yaml_ng};

use super::body::{CleanOutcome, clean_body};
use super::diagnostic::CleanDiagnostic;
use super::frontmatter::{FrontmatterRepair, FrontmatterRepairError, repair_frontmatter};
use super::rules::{Leaf, MarkdownRuleOverrides, ResolvedMarkdownRules, document_rules, resolve};
use crate::markdown::extract_frontmatter_block;
use crate::markdown::frontmatter::load_frontmatter_yaml;
use crate::markdown::schemas::CleanSchemaConfig;

/// The inputs to one [`clean_document`] run.
#[derive(Debug, Clone, Copy)]
pub struct CleanRequest<'a> {
    /// The raw document: frontmatter, body, BOM and line terminators as
    /// authored (an editor's unsaved buffer, a file, or stdin).
    pub source: &'a str,
    /// Explicit caller overrides, the highest precedence level. A command
    /// line's default value is not an override.
    pub overrides: &'a MarkdownRuleOverrides,
    /// DMLS workspace rules, below the document's own; `None` elsewhere.
    pub workspace: Option<&'a MarkdownRuleOverrides>,
    /// The document's path, for schema resolution; `None` for stdin and
    /// untitled buffers, which disables trigger-schema discovery.
    pub document_path: Option<&'a Path>,
}

/// What [`clean_document`] produced.
#[derive(Debug, Clone)]
pub struct CleanResult {
    /// The text to emit; the source itself unless `outcome` is `Changed`.
    pub text: String,
    pub outcome: CleanOutcome,
    /// YAML span in the source (BOM included in the offset).
    pub frontmatter_span: Option<SourceSpan>,
    /// Every finding in authored source coordinates, in pass order: syntax,
    /// schema, `style.markdown` rules, then the fidelity refusal.
    pub diagnostics: Vec<CleanDiagnostic>,
    /// Frontmatter repairs present in `text`; empty unless `outcome` is
    /// `Changed`.
    pub applied: Vec<YamlRepair>,
    /// The rules the run resolved. For `Invalid`, the document level is
    /// missing because it could not be read.
    pub rules: ResolvedMarkdownRules,
}

/// Runs the clean transaction over a raw document.
///
/// `schema` builds the caller's schema configuration from its own request
/// context. It is invoked at most once, only when the schema tier runs (see
/// [`repair_frontmatter`]); nothing here captures a working directory, home,
/// or repository, reads the network, or executes document content.
///
/// ## Returns
///
/// - `Changed`: `text` is the formatted document and `applied` lists the
///   frontmatter repairs in it.
/// - `Unchanged`: nothing needed to change.
/// - `Declined`: the body could not be formatted safely; `text` is the
///   source, `applied` is empty, and the last diagnostic is the
///   `clean.fidelity-fallback` finding.
/// - `Invalid`: Darkmatter's frontmatter parser cannot load the frontmatter
///   that would be emitted (unrepairable, or left unloadable by
///   `frontmatter: false`); `text` is the source and `applied` is empty.
///
/// ## Errors
///
/// The [`FrontmatterRepairError`] cases: a near-miss frontmatter fence, a
/// failure of the caller's `schema` closure, or a missing, unreadable, or
/// invalid referenced schema.
pub fn clean_document<E>(
    request: &CleanRequest<'_>,
    schema: impl FnOnce() -> Result<CleanSchemaConfig, E>,
) -> Result<CleanResult, FrontmatterRepairError<E>> {
    let source = request.source;
    let repair = repair_frontmatter(source, request.document_path, schema)?;
    let mut diagnostics = repair.diagnostics.clone();

    let settings = repair.settings_yaml(source);
    // When strict YAML rejects the authored block, Darkmatter's own parser
    // (the one `Markdown` and compose use) decides whether it is loadable.
    let authored = repair.frontmatter_span.as_ref().and_then(|span| source.get(span.clone()));
    let loaded = match (settings, authored) {
        (Some(settings), _) if !settings.from_candidate => None,
        (_, Some(authored)) => load_frontmatter_yaml(authored),
        _ => None,
    };
    let document = match (settings, &loaded) {
        (Some(settings), _) => {
            let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(settings.yaml)
                .expect("settings_yaml only returns YAML that parses");
            read_document_rules(&value, settings.yaml, |span| settings.authored_span(span), &mut diagnostics)
        }
        (None, Some(map)) => {
            let value = serde_yaml_ng::to_value(map).expect("a loaded frontmatter map is YAML");
            let span = repair.frontmatter_span.clone().expect("a loaded block has a span");
            let yaml = authored.expect("a loaded block has authored text");
            read_document_rules(&value, yaml, |found| found.start + span.start..found.end + span.start, &mut diagnostics)
        }
        (None, None) => None,
    };
    let rules = resolve(request.overrides, document.as_ref(), request.workspace);

    let invalid = |diagnostics: Vec<CleanDiagnostic>, rules: ResolvedMarkdownRules| CleanResult {
        text: source.to_string(),
        outcome: CleanOutcome::Invalid,
        frontmatter_span: repair.frontmatter_span.clone(),
        diagnostics,
        applied: Vec::new(),
        rules,
    };
    // Neither the authored YAML nor a repaired candidate parses, and
    // Darkmatter cannot load the authored block either.
    if repair.frontmatter_span.is_some() && settings.is_none() && loaded.is_none() {
        return Ok(invalid(diagnostics, rules));
    }
    // `frontmatter: false` emits the authored block, which only the candidate
    // could read.
    let repair_enabled = rules.frontmatter.value;
    if !repair_enabled && settings.is_some_and(|settings| settings.from_candidate) && loaded.is_none() {
        return Ok(invalid(diagnostics, rules));
    }

    let frontmatter_span = repair.frontmatter_span.clone();
    // Repairs are emitted only with a candidate that parses; a block only
    // Darkmatter loads is kept as authored.
    let emitted: FrontmatterRepair = if repair_enabled && settings.is_some() {
        repair
    } else {
        repair.discard_repairs(source)
    };
    let body_start = body_offset(&emitted.source);
    let body = &emitted.source[body_start..];
    let cleaned = clean_body(body, &rules);

    if cleaned.outcome == CleanOutcome::Declined {
        let fallback = cleaned
            .fallback
            .expect("a declined body cleanup carries its fallback");
        // The body is a suffix of both the source and the repaired text, so
        // its source offset is unaffected by frontmatter repairs.
        diagnostics.push(fallback.diagnostic(source, source.len() - body.len()));
        return Ok(CleanResult {
            text: source.to_string(),
            outcome: CleanOutcome::Declined,
            frontmatter_span,
            diagnostics,
            applied: Vec::new(),
            rules,
        });
    }

    let mut text = String::with_capacity(emitted.source.len());
    text.push_str(&emitted.source[..body_start]);
    text.push_str(&cleaned.text);
    let changed = text != source;
    Ok(CleanResult {
        outcome: if changed { CleanOutcome::Changed } else { CleanOutcome::Unchanged },
        applied: if changed { emitted.applied } else { Vec::new() },
        text,
        frontmatter_span,
        diagnostics,
        rules,
    })
}

/// The document's `style.markdown` overrides from `value`, read from `yaml`.
/// An unusable object adds its `clean.invalid-rules` findings, with spans
/// mapped to authored coordinates by `authored`, and yields `None`.
fn read_document_rules(
    value: &serde_yaml_ng::Value,
    yaml: &str,
    authored: impl Fn(SourceSpan) -> SourceSpan,
    diagnostics: &mut Vec<CleanDiagnostic>,
) -> Option<MarkdownRuleOverrides> {
    match document_rules(value, yaml, 0) {
        Ok(Leaf::Value(rules)) => Some(rules),
        Ok(Leaf::Absent | Leaf::Null) => None,
        Err(found) => {
            diagnostics.extend(found.into_iter().map(|mut diagnostic| {
                diagnostic.span = authored(diagnostic.span);
                diagnostic
            }));
            None
        }
    }
}

/// Where the body starts: after the frontmatter block and its closing
/// delimiter line, or at 0 without frontmatter.
fn body_offset(text: &str) -> usize {
    match extract_frontmatter_block(text) {
        Ok(Some(extraction)) => extraction.body_span.start,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    //! The transaction's observable results: emitted text, outcome, applied
    //! repairs, and diagnostics in authored coordinates.

    use std::convert::Infallible;

    use biscuit_file::YamlDiagnosticCode;

    use super::*;
    use crate::markdown::cleanup::diagnostic::{CleanCode, CleanStage, FallbackReason};
    use crate::markdown::cleanup::rules::{
        DefaultLanguage, FixedWidth, Indentation, RuleSource,
    };
    use crate::markdown::cleanup::test_hooks::install_broken_pass;

    /// Unparseable as authored (`@` is a reserved indicator); repaired by
    /// quoting.
    const REPAIRABLE: &str = "title: @daily-report\n";
    /// An unlabeled fence: the default rules add `text`.
    const BODY: &str = "\n```\ncode\n```\n";

    fn config() -> Result<CleanSchemaConfig, Infallible> {
        Ok(CleanSchemaConfig::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir())))
    }

    fn clean(source: &str) -> CleanResult {
        clean_levels(source, &MarkdownRuleOverrides::default(), None)
    }

    fn clean_levels(
        source: &str,
        overrides: &MarkdownRuleOverrides,
        workspace: Option<&MarkdownRuleOverrides>,
    ) -> CleanResult {
        let request = CleanRequest { source, overrides, workspace, document_path: None };
        clean_document(&request, config).expect("the transaction runs")
    }

    fn document(yaml: &str, body: &str) -> String {
        format!("---\n{yaml}---\n{body}")
    }

    /// Replaces the first link destination: a meaning change in the body.
    fn retarget_link(candidate: &str) -> String {
        candidate.replacen("(/one)", "(/two)", 1)
    }

    #[track_caller]
    fn assert_declined(result: &CleanResult, source: &str) {
        assert_eq!(result.outcome, CleanOutcome::Declined, "{:?}", result.diagnostics);
        assert_eq!(result.text, source, "a refusal emits the source byte for byte");
        assert!(result.applied.is_empty(), "no repair is reported as applied");
        let last = result.diagnostics.last().expect("a fallback finding");
        assert_eq!(last.code, CleanCode::FidelityFallback);
        assert_eq!(last.span, 0..source.len());
        assert!(last.repairs.is_empty());
    }

    #[test]
    fn repaired_frontmatter_and_formatted_body_are_one_change() {
        let source = document(REPAIRABLE, BODY);
        let result = clean(&source);

        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert_eq!(result.text, "---\ntitle: \"@daily-report\"\n---\n\n```text\ncode\n```\n");
        assert_eq!(result.applied.len(), 1);
        assert_eq!(&source[result.applied[0].span.clone()], "@daily-report");
        assert_eq!(result.frontmatter_span, Some(4..4 + REPAIRABLE.len()));
        let again = clean(&result.text);
        assert_eq!(again.outcome, CleanOutcome::Unchanged, "{:?}", again.text);
        assert!(again.applied.is_empty());
    }

    #[test]
    fn a_body_refusal_discards_the_candidate_frontmatter_repair() {
        let body = "\nSee [a](/one).\n\n```\ncode\n```\n";
        let source = document(REPAIRABLE, body);
        let result = {
            let _broken = install_broken_pass(retarget_link);
            clean(&source)
        };

        assert_declined(&result, &source);
        let fallback = result.diagnostics.last().and_then(|d| d.fallback.clone()).expect("fallback");
        assert_eq!(fallback.reason, FallbackReason::SemanticMismatch);
        // Located in authored coordinates: the diverging link in the
        // paragraph after the (unrepaired) frontmatter and a blank line, line 5.
        assert_eq!(&source[fallback.offset..fallback.offset + 4], "[a](");
        assert!(result.diagnostics.last().unwrap().message.contains("line 5, column 5"));
        // The YAML finding that proposed the discarded repair is still reported.
        assert!(result.diagnostics.iter().any(|d| d.stage == CleanStage::Syntax));
    }

    #[test]
    fn corruption_introduced_by_reflow_is_refused_after_the_rewrap() {
        let yaml = "style:\n  markdown:\n    fixed-width: 20\n";
        let body = "\nOne two three four five six seven [link](/one) nine ten.\n";
        let source = document(yaml, body);
        let accepted = clean(&source);
        assert_eq!(accepted.outcome, CleanOutcome::Changed);
        assert_eq!(accepted.rules.fixed_width.value, FixedWidth::Columns(20));
        assert!(accepted.text.lines().skip(5).all(|line| line.len() <= 20), "{:?}", accepted.text);

        // The pass corrupts only a re-wrapped candidate (one with a break
        // inside the paragraph), so only verification after the re-wrap can
        // catch it.
        let result = {
            let _broken = install_broken_pass(|candidate| {
                if candidate.contains("One two three four\n") { retarget_link(candidate) } else { candidate.to_string() }
            });
            clean(&source)
        };
        assert_declined(&result, &source);
    }

    #[test]
    fn unrepairable_yaml_is_invalid_and_nothing_is_cleaned() {
        let source = document("title: a\ntitle: b\n", BODY);
        let result = clean(&source);

        assert_eq!(result.outcome, CleanOutcome::Invalid);
        assert_eq!(result.text, source, "the body is not cleaned either");
        assert!(result.applied.is_empty());
        assert!(
            result.diagnostics.iter().any(|d| d.code == CleanCode::Yaml(YamlDiagnosticCode::DuplicateKey)),
            "{:?}",
            result.diagnostics
        );
    }

    /// `prompts/_reviews/cycles.md`'s frontmatter: a bare `{{ … }}` value is
    /// a YAML flow mapping followed by garbage, but Darkmatter loads it.
    const BARE_INTERPOLATION: &str = "$schema:\n    review: file\nreview: {{ ctx.area }}/reviews/{{ ctx.today }}-cyclometric-risk/review.md\n";

    #[test]
    fn frontmatter_only_darkmatter_loads_is_kept_and_its_body_is_cleaned() {
        // Tab indentation in a block scalar, as `.claude/skills/lsp/lsp.md`
        // has it.
        let tab_indented = "prompt: |-\n\tYour task is to research.\n\n    Answer the questions.\n";
        for yaml in [BARE_INTERPOLATION, tab_indented] {
            let source = document(yaml, BODY);
            let result = clean(&source);

            assert_eq!(result.outcome, CleanOutcome::Changed, "{yaml:?}: {:?}", result.diagnostics);
            assert_eq!(result.text, document(yaml, "\n```text\ncode\n```\n"), "the block is kept byte for byte");
            assert!(result.applied.is_empty());
            assert!(
                result.diagnostics.iter().any(|d| d.code == CleanCode::Yaml(YamlDiagnosticCode::Parse)),
                "the strict-YAML finding is still reported: {:?}",
                result.diagnostics
            );
            let again = clean(&result.text);
            assert_eq!(again.outcome, CleanOutcome::Unchanged);
        }
    }

    #[test]
    fn rules_are_read_from_frontmatter_only_darkmatter_loads() {
        let yaml = format!("{BARE_INTERPOLATION}style:\n    markdown:\n        default-language: sh\n");
        let result = clean(&document(&yaml, BODY));
        assert_eq!(result.outcome, CleanOutcome::Changed, "{:?}", result.diagnostics);
        assert_eq!(result.rules.default_language.source, RuleSource::Document);
        assert_eq!(result.text, document(&yaml, "\n```sh\ncode\n```\n"));

        let disabled = format!("{BARE_INTERPOLATION}style:\n    markdown:\n        frontmatter: false\n");
        let result = clean(&document(&disabled, BODY));
        assert_eq!(result.outcome, CleanOutcome::Changed, "loadable as authored, so not invalid");
        assert!(!result.rules.frontmatter.value);
        assert_eq!(result.text, document(&disabled, "\n```text\ncode\n```\n"));

        let invalid = format!("{BARE_INTERPOLATION}style:\n    markdown:\n        indentation: 3\n");
        let source = document(&invalid, BODY);
        let result = clean(&source);
        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert_eq!(result.rules.indentation.source, RuleSource::Default);
        let finding = result
            .diagnostics
            .iter()
            .find(|d| d.code == CleanCode::InvalidRules)
            .expect("the unusable object is reported");
        let frontmatter = result.frontmatter_span.clone().expect("frontmatter");
        assert!(finding.span.start >= frontmatter.start && finding.span.end <= frontmatter.end, "{finding:?}");
    }

    #[test]
    fn frontmatter_false_read_from_a_repair_candidate_leaves_invalid_yaml_invalid() {
        let yaml = format!("{REPAIRABLE}style:\n  markdown:\n    frontmatter: false\n");
        let source = document(&yaml, BODY);
        let result = clean(&source);

        assert_eq!(result.outcome, CleanOutcome::Invalid);
        assert_eq!(result.text, source);
        assert!(result.applied.is_empty());
        assert!(!result.rules.frontmatter.value);
        assert_eq!(result.rules.frontmatter.source, RuleSource::Document);
    }

    #[test]
    fn frontmatter_false_keeps_the_block_byte_for_byte_and_still_formats_the_body() {
        let yaml = "title: Doc   \nstyle:\n  markdown:\n    frontmatter: false\n";
        let source = document(yaml, BODY);
        let repaired = clean(&document("title: Doc   \n", BODY));
        assert!(!repaired.applied.is_empty(), "the control repairs the trailing spaces");

        let result = clean(&source);
        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert_eq!(result.text, document(yaml, "\n```text\ncode\n```\n"));
        assert!(result.applied.is_empty());
    }

    #[test]
    fn document_rules_are_read_from_the_repair_candidate_when_the_authored_yaml_does_not_parse() {
        let yaml = format!("{REPAIRABLE}style:\n  markdown:\n    default-language: rust\n");
        let result = clean(&document(&yaml, BODY));

        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert!(result.text.ends_with("\n```rust\ncode\n```\n"), "{:?}", result.text);
        assert_eq!(result.rules.default_language.value, DefaultLanguage::new("rust").unwrap());
    }

    #[test]
    fn an_invalid_rule_in_a_repair_candidate_is_reported_at_its_authored_position() {
        let yaml = format!("{REPAIRABLE}style:\n  markdown:\n    indentation: 3\n");
        let source = document(&yaml, BODY);
        let result = clean(&source);

        let finding = result
            .diagnostics
            .iter()
            .find(|d| d.code == CleanCode::InvalidRules)
            .expect("an invalid-rules finding");
        assert_eq!(&source[finding.span.clone()], "3");
        // The whole object is ignored; the defaults still format the body.
        assert_eq!(result.rules.indentation.source, RuleSource::Default);
        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert!(result.text.ends_with("\n```text\ncode\n```\n"));
    }

    #[test]
    fn each_rule_resolves_overrides_then_document_then_workspace_then_default() {
        let body = "\n- a\n  - b\n\n```\ncode\n```\n";
        let yaml = "style:\n  markdown:\n    indentation: 8\n    default-language: rust\n";
        let source = document(yaml, body);
        let workspace = MarkdownRuleOverrides {
            indentation: Leaf::Value(Indentation::Two),
            default_language: Leaf::Value(DefaultLanguage::new("sh").unwrap()),
            table_layout: Leaf::Value(false),
            ..MarkdownRuleOverrides::default()
        };
        let overrides = MarkdownRuleOverrides {
            indentation: Leaf::Value(Indentation::Two),
            ..MarkdownRuleOverrides::default()
        };

        let result = clean_levels(&source, &overrides, Some(&workspace));
        assert_eq!(result.rules.indentation.source, RuleSource::Cli);
        assert_eq!(result.rules.default_language.source, RuleSource::Document);
        assert_eq!(result.rules.table_layout.source, RuleSource::Workspace);
        assert_eq!(result.rules.ordered_lists.source, RuleSource::Default);
        assert_eq!(result.text, document(yaml, "\n- a\n  - b\n\n```rust\ncode\n```\n"));

        let without_cli = clean_levels(&source, &MarkdownRuleOverrides::default(), Some(&workspace));
        // Eight columns per level: the child moves to column 8, which the
        // parent marker reaches with its maximum padding.
        assert_eq!(without_cli.text, document(yaml, "\n-    a\n        - b\n\n```rust\ncode\n```\n"));
    }

    #[test]
    fn empty_frontmatter_and_terminators_survive() {
        for source in ["---\n---\nbody\n", "---\r\n---\r\n\r\n- a\r\n- b\r\n", "---\n---\n"] {
            let result = clean(source);
            assert_eq!(result.outcome, CleanOutcome::Unchanged, "{source:?}");
            assert_eq!(result.text, source);
        }
    }

    #[test]
    fn a_byte_order_mark_is_kept_without_frontmatter_and_repaired_away_before_it() {
        let plain = "\u{feff}# Title\n";
        assert_eq!(clean(plain).text, plain);

        let with_frontmatter = document("title: Doc\n", "\nBody.\n");
        let source = format!("\u{feff}{with_frontmatter}");
        let result = clean(&source);
        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert_eq!(result.text, with_frontmatter);
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.applied[0].span, 0..3);

        let disabled = format!("\u{feff}{}", document("style:\n  markdown:\n    frontmatter: false\n", "\nBody.\n"));
        let result = clean(&disabled);
        assert_eq!(result.outcome, CleanOutcome::Unchanged);
        assert_eq!(result.text, disabled);
    }

    #[test]
    fn a_frontmatter_only_change_is_a_change_with_every_body_rule_off() {
        let source = document("title: Doc   \n", "\n1. a\n1. b\n");
        let result = clean_levels(&source, &MarkdownRuleOverrides::all_body_rules_off(), None);
        assert_eq!(result.outcome, CleanOutcome::Changed);
        assert_eq!(result.text, document("title: Doc\n", "\n1. a\n1. b\n"));
        assert_eq!(result.applied.len(), 1);
    }

    #[test]
    fn a_malformed_region_declines_the_whole_document() {
        let source = document(REPAIRABLE, "\n::block when=\"x\"\n- a\n");
        let result = clean(&source);
        assert_declined(&result, &source);
        let finding = result.diagnostics.last().expect("a fallback finding");
        assert_eq!(finding.fallback.as_ref().map(|f| f.reason), Some(FallbackReason::MalformedRegion));
        // The location and the detail agree on document lines: the opener is
        // on document line 5 (body line 2), and the detail names no line.
        assert_eq!(
            finding.message,
            "cleanup left the document unchanged because of a malformed directive region at line 5, column 1: \
             `::block when=\"x\"` is never closed by an `::end-block`"
        );
    }
}
