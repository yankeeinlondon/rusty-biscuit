//! Stage 1 transform pipeline for markdown document preparation.
//!
//! This module provides the `transform()` family of methods on `Markdown`
//! for running preparation transforms in a fixed order:
//!
//! 1. **Text Replacement** - Replace literal strings from frontmatter `replace` map
//! 2. **Interpolation** - Expand `{{variable}}` expressions
//! 3. **Cleanup** - Normalize markdown formatting
//! 4. **Normalization** - Adjust heading levels
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::transform::TransformOptions;
//!
//! let content = "# Hello\nWorld";
//! let mut md: Markdown = content.into();
//!
//! // Transform with default options
//! let report = md.transform_mut().unwrap();
//!
//! // Transform with custom options
//! let options = TransformOptions::new()
//!     .with_fail_fast(true);
//! let report = md.transform_with(options).unwrap();
//! ```

mod state;
mod types;

pub mod interpolation;
pub mod replacement;

pub use state::{EffectiveState, EffectiveStateBuilder};
pub use types::{
    Stage1Stages, TransformContext, TransformOptions, TransformReport, TransformWarning,
};

use super::cleanup;
use super::normalize::{self, NormalizationError};
use super::types::{MarkdownError, MarkdownResult};
use super::Markdown;

// Re-export HeadingLevel for tests
#[cfg(test)]
pub use super::normalize::HeadingLevel;

impl Markdown {
    /// Transforms the document using default options.
    ///
    /// This is equivalent to `transform_with(TransformOptions::new())`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    /// let (transformed, report) = md.transform().unwrap();
    /// ```
    pub fn transform(&self) -> MarkdownResult<(Markdown, TransformReport)> {
        self.transform_with(TransformOptions::new())
    }

    /// Transforms the document with custom options.
    ///
    /// Returns a new `Markdown` document and a report of changes made.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::transform::{TransformOptions, Stage1Stages};
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    ///
    /// let options = TransformOptions::new()
    ///     .with_stages(Stage1Stages {
    ///         normalization: false,
    ///         ..Default::default()
    ///     });
    ///
    /// let (transformed, report) = md.transform_with(options).unwrap();
    /// ```
    pub fn transform_with(
        &self,
        options: TransformOptions,
    ) -> MarkdownResult<(Markdown, TransformReport)> {
        let mut result = self.clone();
        let report = result.run_transform_pipeline(options)?;
        Ok((result, report))
    }

    /// Transforms the document in place, returning only the report.
    ///
    /// This is more efficient than `transform()` when you don't need
    /// to preserve the original document.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Test\nContent";
    /// let mut md: Markdown = content.into();
    /// let report = md.transform_mut().unwrap();
    ///
    /// // md is now transformed
    /// ```
    pub fn transform_mut(&mut self) -> MarkdownResult<TransformReport> {
        self.run_transform_pipeline(TransformOptions::new())
    }

    /// Internal pipeline runner.
    fn run_transform_pipeline(
        &mut self,
        options: TransformOptions,
    ) -> MarkdownResult<TransformReport> {
        let mut report = TransformReport::new();

        // Build effective state for replacement and interpolation
        let _effective_state = EffectiveStateBuilder::new()
            .with_frontmatter(self.frontmatter().as_map().clone())
            .with_external_state(options.external_state.clone().unwrap_or_default())
            .with_context(options.context().clone())
            .build();

        // Stage 1: Text Replacement
        if options.stages.replacement {
            let replacements = self.run_replacement_stage(&_effective_state, &options);
            report.replacements_applied = replacements;
        }

        // Stage 2: Interpolation
        if options.stages.interpolation {
            let interpolations = self.run_interpolation_stage(&_effective_state, &options);
            report.interpolations_applied = interpolations;
        }

        // Stage 3: Cleanup
        if options.stages.cleanup {
            let original_content = self.content.clone();
            self.content = cleanup::cleanup_content(&self.content);
            report.cleanup_changed = self.content != original_content;
        }

        // Stage 4: Normalization
        if options.stages.normalization {
            match self.run_normalization_stage() {
                Ok(norm_report) => {
                    if norm_report.has_changes() {
                        report.normalization_report = Some(norm_report);
                    }
                }
                Err(NormalizationError::LevelOverflow { .. }) if !options.fail_fast => {
                    // Add warning instead of failing
                    report.add_warning(TransformWarning::new(
                        "normalization",
                        "Skipped normalization: would overflow H6",
                    ));
                }
                Err(e) => {
                    return Err(MarkdownError::Transform(format!(
                        "Normalization failed: {}",
                        e
                    )));
                }
            }
        }

        Ok(report)
    }

    /// Runs the text replacement stage.
    ///
    /// Applies text replacements from the `replace` map in effective state.
    /// See [`replacement::apply_replacements`] for algorithm details.
    fn run_replacement_stage(&mut self, state: &EffectiveState, _options: &TransformOptions) -> usize {
        let (new_content, count) = replacement::apply_replacements(&self.content, state);
        if count > 0 {
            self.content = new_content;
        }
        count
    }

    /// Runs the interpolation stage.
    ///
    /// Finds `{{ expression }}` patterns in content and evaluates them
    /// against the effective state. Expressions in code blocks are skipped.
    fn run_interpolation_stage(
        &mut self,
        state: &EffectiveState,
        options: &TransformOptions,
    ) -> usize {
        use interpolation::{parse, EvalResult, Evaluator, ExpressionFinder};

        let finder = ExpressionFinder::new(&self.content);
        let locations = finder.find_all();

        if locations.is_empty() {
            return 0;
        }

        let evaluator = Evaluator::new(state);
        let mut count = 0;
        let mut new_content = self.content.clone();

        // Apply replacements from end to start to preserve offsets
        for loc in locations.into_iter().rev() {
            match parse(&loc.expression) {
                Ok(expr) => match evaluator.eval(&expr) {
                    EvalResult::Value(replacement) => {
                        new_content.replace_range(loc.start..loc.end, &replacement);
                        count += 1;
                    }
                    EvalResult::Error { message, .. } if options.fail_fast => {
                        // TODO: In the future, return Err from pipeline
                        // For now, log and continue
                        tracing::warn!(
                            expression = %loc.expression,
                            error = %message,
                            "Interpolation evaluation failed"
                        );
                    }
                    EvalResult::Error { .. } => {
                        // Leave original expression in place
                    }
                },
                Err(e) if options.fail_fast => {
                    tracing::warn!(
                        expression = %loc.expression,
                        error = %e,
                        "Interpolation parse failed"
                    );
                }
                Err(_) => {
                    // Parse error - leave original
                }
            }
        }

        self.content = new_content;
        count
    }

    /// Runs the normalization stage.
    ///
    /// Uses `None` as target level, which means headings are not re-leveled
    /// but the document structure is validated.
    fn run_normalization_stage(
        &mut self,
    ) -> Result<normalize::NormalizationReport, NormalizationError> {
        let (new_content, report) = normalize::normalize(&self.content, None)?;
        self.content = new_content;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use super::HeadingLevel;

    #[test]
    fn test_transform_returns_unchanged_document() {
        let content = "# Hello\n\nWorld";
        let md: Markdown = content.into();

        let (transformed, _report) = md.transform().unwrap();

        // Content should still match (cleanup may add spacing)
        assert!(transformed.content().contains("Hello"));
        assert!(transformed.content().contains("World"));
    }

    #[test]
    fn test_transform_mut_modifies_in_place() {
        let content = "# Hello\n\nWorld";
        let mut md: Markdown = content.into();

        let _report = md.transform_mut().unwrap();

        assert!(md.content().contains("Hello"));
        assert!(md.content().contains("World"));
    }

    #[test]
    fn test_transform_with_custom_options() {
        let content = "# Hello\n\nWorld";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages {
            cleanup: false,
            normalization: false,
            ..Default::default()
        });

        let (transformed, report) = md.transform_with(options).unwrap();

        // With cleanup disabled, content should be unchanged
        assert_eq!(transformed.content(), md.content());
        assert!(!report.cleanup_changed);
    }

    #[test]
    fn test_transform_cleanup_stage() {
        // Content without proper spacing
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_cleanup());

        let (transformed, report) = md.transform_with(options).unwrap();

        // Cleanup should add blank line between header and paragraph
        assert!(transformed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_transform_normalization_stage_no_change() {
        let content = "# Hello\n\n## World";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_normalization());

        let (_, report) = md.transform_with(options).unwrap();

        // Well-formed document, no normalization needed
        assert!(report.normalization_report.is_none());
    }

    #[test]
    fn test_transform_preserves_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello";
        let md: Markdown = content.into();

        let (transformed, _) = md.transform().unwrap();

        let title: Option<String> = transformed.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
    }

    #[test]
    fn test_transform_report_summary() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let (_, report) = md.transform().unwrap();

        // Should have a meaningful summary
        let summary = report.summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_transform_report_has_changes() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let (_, report) = md.transform().unwrap();

        // Cleanup should have made changes
        assert!(report.has_changes());
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_transform_stages_all_disabled() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::none());

        let (transformed, report) = md.transform_with(options).unwrap();

        // No changes should be made
        assert_eq!(transformed.content(), md.content());
        assert!(!report.has_changes());
    }

    #[test]
    fn test_transform_stages_run_in_order() {
        // This test verifies that stages run in the expected order
        // by observing their effects

        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        // Run all stages
        let (_, report) = md.transform().unwrap();

        // Verify stages ran (via report)
        assert_eq!(report.replacements_applied, 0); // No replace map in frontmatter
        assert_eq!(report.interpolations_applied, 0); // Stub (Phase 2)
        // Cleanup should have run
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_transform_with_external_state() {
        let content = "# Hello";
        let md: Markdown = content.into();

        let options =
            TransformOptions::new().with_external_state(serde_json::json!({"key": "value"}));

        // Should not fail
        let result = md.transform_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transform_options_context_captured() {
        let options = TransformOptions::new();
        let ctx = options.context();

        // Context should have been captured
        assert!(!ctx.today.is_empty());
        assert!(!ctx.year.is_empty());
    }

    #[test]
    fn test_transform_fail_fast_false_continues_on_warning() {
        // Document that would cause normalization warning
        // (but for now normalization doesn't fail with None target)
        let content = "# Hello";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_fail_fast(false);

        let result = md.transform_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_effective_state_available_to_stages() {
        let content = "---\nkey: value\n---\n# Hello";
        let md: Markdown = content.into();

        // External state should merge with frontmatter
        let options = TransformOptions::new()
            .with_external_state(serde_json::json!({"external": "data"}));

        let result = md.transform_with(options);
        assert!(result.is_ok());
    }

    // ============================================
    // Replacement stage integration tests
    // ============================================

    #[test]
    fn test_replacement_stage_with_frontmatter() {
        let content = "---\nreplace:\n  foo: bar\n---\n# Hello foo\n\nContent with foo here.";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("Hello bar"));
        assert!(transformed.content().contains("Content with bar here."));
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_overlap_resolution() {
        // Longest key wins: "foobar" before "foo"
        let content = "---\nreplace:\n  foo: short\n  foobar: long\n---\nfoobar and foo";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "long and short");
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_non_recursive() {
        // Replacement output should NOT be re-scanned
        let content = "---\nreplace:\n  foo: foobar\n  foobar: baz\n---\nfoo";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (transformed, report) = md.transform_with(options).unwrap();

        // "foo" -> "foobar" but NOT -> "baz"
        assert_eq!(transformed.content(), "foobar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_null_value() {
        let content = "---\nreplace:\n  remove_me: null\n---\nHello remove_me world";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Hello  world");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_number_value() {
        let content = "---\nreplace:\n  VERSION: 42\n---\nVersion: VERSION";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Version: 42");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_no_replace_in_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello foo";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (transformed, report) = md.transform_with(options).unwrap();

        // No changes when no replace map
        assert_eq!(transformed.content(), md.content());
        assert_eq!(report.replacements_applied, 0);
    }

    #[test]
    fn test_replacement_stage_with_external_state() {
        // External state can provide replace map
        let content = "# Hello foo";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages::only_replacement())
            .with_external_state(serde_json::json!({
                "replace": {"foo": "bar"}
            }));

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "# Hello bar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_external_overrides_frontmatter() {
        // External state replace map should merge/override frontmatter
        let content = "---\nreplace:\n  foo: from_fm\n  baz: qux\n---\nfoo baz";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages::only_replacement())
            .with_external_state(serde_json::json!({
                "replace": {"foo": "from_external"}
            }));

        let (transformed, report) = md.transform_with(options).unwrap();

        // External wins on "foo", but we lose "baz" because replace is replaced wholesale
        // This is the correct behavior based on PreferExternal merge strategy
        assert!(transformed.content().contains("from_external"));
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_report_summary() {
        let content = "---\nreplace:\n  a: b\n---\na a a";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_replacement());

        let (_, report) = md.transform_with(options).unwrap();

        assert_eq!(report.replacements_applied, 3);
        let summary = report.summary();
        assert!(summary.contains("3 replacement(s)"));
    }

    #[test]
    fn test_replacement_then_cleanup() {
        // Test that replacement runs before cleanup
        let content = "---\nreplace:\n  foo: bar\n---\n# Header\nfoo here";
        let md: Markdown = content.into();

        // Enable both replacement and cleanup
        let options = TransformOptions::new().with_stages(Stage1Stages {
            replacement: true,
            interpolation: false,
            cleanup: true,
            normalization: false,
        });

        let (transformed, report) = md.transform_with(options).unwrap();

        // Replacement happened
        assert!(transformed.content().contains("bar here"));
        assert_eq!(report.replacements_applied, 1);

        // Cleanup added blank line
        assert!(transformed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    // ============================================
    // Interpolation stage integration tests
    // ============================================

    #[test]
    fn test_interpolation_simple_variable() {
        let content = "---\nname: Alice\n---\n# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "# Hello Alice!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_nested_variable() {
        let content = "---\nuser:\n  name: Bob\n---\nWelcome {{ user.name }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Welcome Bob");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_missing_variable() {
        let content = "---\ntitle: Test\n---\nHello {{ missing }}!";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        // Missing variables become empty string
        assert_eq!(transformed.content(), "Hello !");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_default() {
        let content = "---\ntitle: Test\n---\nColor: {{ color | \"unknown\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Color: unknown");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_primary() {
        let content = "---\ncolor: blue\n---\nColor: {{ color | \"unknown\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Color: blue");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_true() {
        let content = "---\nactive: true\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Status: on");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_false() {
        let content = "---\nactive: false\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Status: off");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_equal() {
        let content = "---\ncount: 5\n---\n{{ count == 5 ? \"five\" : \"other\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "five");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_greater_than() {
        let content = "---\ncount: 10\n---\n{{ count > 5 ? \"many\" : \"few\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "many");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_multiple_expressions() {
        let content = "---\nfirst: Alice\nlast: Smith\n---\n{{ first }} {{ last }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Alice Smith");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_skips_code_span() {
        let content = "---\nname: Alice\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        // Only the first expression is expanded, code span preserved
        assert_eq!(transformed.content(), "Hello Alice! Code: `{{ name }}`");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_skips_fenced_code() {
        let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        // Only the first expression is expanded, code block preserved
        assert!(transformed.content().contains("Hello Alice!"));
        assert!(transformed.content().contains("```\n{{ name }}\n```"));
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_no_expressions() {
        let content = "---\nname: Alice\n---\n# Just plain text";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), md.content());
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_with_external_state() {
        let content = "# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages::only_interpolation())
            .with_external_state(serde_json::json!({"name": "External"}));

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "# Hello External!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_external_overrides_frontmatter() {
        let content = "---\nname: Frontmatter\n---\n# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages::only_interpolation())
            .with_external_state(serde_json::json!({"name": "External"}));

        let (transformed, report) = md.transform_with(options).unwrap();

        // External state wins
        assert_eq!(transformed.content(), "# Hello External!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_chained_fallback() {
        let content = "---\nbackup: second\n---\nValue: {{ missing | backup | \"default\" }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Value: second");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_parse_error_preserves_original() {
        // Malformed expression should be left as-is (not fail_fast)
        let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages::only_interpolation())
            .with_fail_fast(false);

        let (transformed, report) = md.transform_with(options).unwrap();

        // Invalid expression left unchanged
        assert_eq!(transformed.content(), "Hello {{ @invalid }}!");
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_full_transform_with_interpolation() {
        // Integration test: frontmatter + interpolation + cleanup
        let content = "---\nname: Alice\ncount: 3\n---\n# Welcome {{ name }}\nYou have {{ count > 0 ? \"items\" : \"nothing\" }}";
        let md: Markdown = content.into();

        let (transformed, report) = md.transform().unwrap();

        assert!(transformed.content().contains("Welcome Alice"));
        assert!(transformed.content().contains("You have items"));
        assert_eq!(report.interpolations_applied, 2);
        assert!(report.cleanup_changed); // Cleanup adds blank line
    }

    #[test]
    fn test_interpolation_report_summary() {
        let content = "---\na: 1\nb: 2\n---\n{{ a }} {{ b }}";
        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (_, report) = md.transform_with(options).unwrap();

        assert_eq!(report.interpolations_applied, 2);
        let summary = report.summary();
        assert!(summary.contains("2 interpolation(s)"));
    }

    // ============================================
    // E2E Integration tests
    // ============================================

    #[test]
    fn test_e2e_all_stages_with_external_state() {
        // Full pipeline: replacement -> interpolation -> cleanup -> normalization
        let content = r#"---
replace:
  PLACEHOLDER: actual
name: Alice
count: 5
---
# Welcome {{ name }}
PLACEHOLDER content here.
{{ count > 3 ? "many items" : "few items" }}"#;

        let md: Markdown = content.into();

        let options = TransformOptions::new().with_external_state(serde_json::json!({
            "extra": "external_value"
        }));

        let (transformed, report) = md.transform_with(options).unwrap();

        // Replacement happened
        assert!(transformed.content().contains("actual content here"));
        assert_eq!(report.replacements_applied, 1);

        // Interpolation happened
        assert!(transformed.content().contains("Welcome Alice"));
        assert!(transformed.content().contains("many items"));
        assert_eq!(report.interpolations_applied, 2);

        // Cleanup happened (blank line added)
        assert!(report.cleanup_changed);

        // Full summary
        let summary = report.summary();
        assert!(summary.contains("1 replacement(s)"));
        assert!(summary.contains("2 interpolation(s)"));
    }

    #[test]
    fn test_e2e_unicode_content() {
        // Test Unicode handling in replacement and interpolation
        let content = r#"---
replace:
  ":smile:": "😊"
  ":wave:": "👋"
greeting: こんにちは
---
Hello :wave: {{ greeting }} :smile:"#;

        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages {
            replacement: true,
            interpolation: true,
            cleanup: false,
            normalization: false,
        });

        let (transformed, report) = md.transform_with(options).unwrap();

        assert_eq!(transformed.content(), "Hello 👋 こんにちは 😊");
        assert_eq!(report.replacements_applied, 2);
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_e2e_helper_functions() {
        let content = r#"---
items:
  - a
  - b
  - c
value: "42"
pi: 3.14159
---
Items: {{ length(items) }}
Number: {{ number(value) }}
Rounded: {{ round(pi) }}"#;

        let md: Markdown = content.into();

        let options = TransformOptions::new().with_stages(Stage1Stages::only_interpolation());

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("Items: 3"));
        assert!(transformed.content().contains("Number: 42"));
        assert!(transformed.content().contains("Rounded: 3"));
        assert_eq!(report.interpolations_applied, 3);
    }
}
