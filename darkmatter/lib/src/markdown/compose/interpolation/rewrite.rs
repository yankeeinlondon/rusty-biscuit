//! Shared string rewrite helper for interpolation.
//!
//! Provides `interpolate_text`, which scans a string for `{{ }}` expressions,
//! evaluates them against an [`EvaluationLookup`] implementation, and
//! returns the rewritten string. Supports both markdown-aware scanning
//! (skipping code regions) and plain-text scanning.

use super::{EvalResult, Evaluator, ExpressionFinder, ExpressionLocation, parse};
use crate::markdown::compose::expression::{Expr, InlineCodeSpan, InterpolationContext};
use crate::markdown::compose::expression::{EvaluationLookup, ExpressionError};
use crate::markdown::compose::ComposeWarning;
use crate::markdown::types::{MarkdownError, SourceRef};
use serde_json::Value;
use std::collections::HashMap;

/// Wraps a typed evaluation `cause` in [`MarkdownError::Interpolation`], the
/// single construction point for the live compose interpolation path.
///
/// The `key` is left `None` here (body interpolation has none); frontmatter
/// whole-value callers attach it afterward via `key_scoped_error`. The `source`
/// starts as [`SourceRef::Effective`] — the on-disk excerpt is layered on at the
/// pipeline boundary where the document's [`SourceContext`] is in scope.
///
/// [`SourceContext`]: biscuit_terminal::errors::SourceContext
fn interpolation_error(expression: &str, cause: ExpressionError) -> MarkdownError {
    MarkdownError::Interpolation {
        key: None,
        expression: expression.to_string(),
        source: Box::new(SourceRef::Effective {
            rendered: expression.to_string(),
            origin_key: None,
        }),
        cause: Box::new(cause),
    }
}

/// Controls how `interpolate_text` scans for `{{ }}` expressions.
#[derive(Clone, Copy)]
pub(crate) enum ScanMode {
    /// Scan the entire string with no exclusions.
    /// Used by frontmatter interpolation and other non-Markdown surfaces.
    Plain,
    /// Scan a Markdown body and retain the syntax position of each replacement.
    Body {
        /// Whether opted-in fenced and indented code blocks are scanned.
        include_code_blocks: bool,
    },
}

/// Result of rewriting interpolation expressions in a string.
pub(crate) struct InterpolationRewrite {
    /// The rewritten output string.
    pub output: String,
    /// Number of expressions successfully replaced.
    pub replacements: usize,
    /// Warnings generated during rewrite (non-fatal issues).
    pub warnings: Vec<ComposeWarning>,
}

/// Maximum number of rescan iterations to prevent infinite loops.
const MAX_INTERPOLATION_DEPTH: usize = 10;

/// Converts `{{{ ... }}}` interpolation literals in `input` to the literal
/// text `{{ ... }}`.
///
/// Literals are recognized using the shared scanner and are converted
/// from end to start so byte offsets remain stable. This runs **after**
/// the final interpolation pass so replacement values that introduce new
/// literals are also converted.
pub(crate) fn convert_literals(input: &str, scan_mode: ScanMode) -> String {
    let scan = match scan_mode {
        ScanMode::Plain => ExpressionFinder::scan_plain(input),
        ScanMode::Body { include_code_blocks } => {
            if include_code_blocks {
                ExpressionFinder::new_including_code_blocks(input).scan()
            } else {
                ExpressionFinder::new(input).scan()
            }
        }
    };
    let mut output = input.to_string();
    for lit in scan.literals.iter().rev() {
        let replacement = format!("{}{}{}", "{{", lit.content, "}}");
        output.replace_range(lit.start..lit.end, &replacement);
    }
    output
}

/// Scans `input` for `{{ }}` expressions, evaluates them, and returns
/// the rewritten string.
///
/// After each pass of replacements, the output is rescanned for newly
/// introduced `{{ }}` expressions (e.g. from a ternary branch that
/// contains interpolation placeholders).  Loop-depth protection prevents
/// runaway recursion when a replacement re-introduces the same expression.
///
/// ## Arguments
///
/// - `input` — the text to scan
/// - `evaluator` — evaluates parsed expressions against state
/// - `scan_mode` — whether to respect code regions or scan everything
/// - `fail_fast` — if `true`, return an error on the first parse/eval failure
/// - `warning_stage` — label attached to any warnings produced
pub(crate) fn interpolate_text<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    fail_fast: bool,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, MarkdownError> {
    // Fast path (F14): a `{{ … }}` expression and a `{{{ … }}}` literal both
    // require the `{{` sequence. When the input contains none, no expression or
    // literal can be present, so the whole scan pipeline — including any
    // pulldown-cmark code-region parse, every rescan
    // pass, and `convert_literals` — is provably a no-op. Skip it and return the
    // input verbatim. Byte-identical: the scan would find zero locations and
    // `convert_literals` zero literals either way.
    if !input.contains("{{") {
        return Ok(InterpolationRewrite {
            output: input.to_string(),
            replacements: 0,
            warnings: Vec::new(),
        });
    }

    let mut output = input.to_string();
    let mut total_count = 0;
    let mut all_warnings = Vec::new();

    for depth in 0..MAX_INTERPOLATION_DEPTH {
        let locations: Vec<ExpressionLocation> = match scan_mode {
            ScanMode::Plain => ExpressionFinder::find_all_plain(&output),
            ScanMode::Body { include_code_blocks } => {
                if include_code_blocks {
                    ExpressionFinder::new_including_code_blocks(&output).find_all()
                } else {
                    ExpressionFinder::new(&output).find_all()
                }
            }
        };

        if locations.is_empty() {
            break;
        }

        let mut count = 0;
        let mut warnings = Vec::new();
        let mut spans = CodeSpanRefencer::new(&locations);

        for loc in locations.into_iter().rev() {
            match parse(&loc.expression) {
                Ok(expr) => {
                    let mut ctx_warnings = evaluator.collect_context_warnings(
                        &expr,
                        warning_stage,
                    );
                    warnings.append(&mut ctx_warnings);
                    match evaluator.eval(&expr) {
                    EvalResult::Value(replacement) => {
                        let replacement = project_body_replacement(
                            replacement,
                            &expr,
                            loc.context,
                            scan_mode,
                        );
                        // Inherit line indentation for multiline replacements
                        let replacement = if replacement.contains('\n') {
                            let line_start =
                                output[..loc.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                            let indent: String = output[line_start..loc.start]
                                .chars()
                                .take_while(|c| c.is_whitespace())
                                .collect();
                            if indent.is_empty() {
                                replacement
                            } else {
                                replacement.replace('\n', &format!("\n{indent}"))
                            }
                        } else {
                            replacement
                        };
                        let replaced = loc.end - loc.start;
                        output.replace_range(loc.start..loc.end, &replacement);
                        spans.record(&loc, replacement.len(), replaced, &mut output);
                        count += 1;
                    }
                    EvalResult::Error { error, .. }
                        if fail_fast || error.is_authoring_fatal() =>
                    {
                        return Err(interpolation_error(&loc.expression, error));
                    }
                    EvalResult::Error { error, original } => {
                        warnings.push(ComposeWarning::new(
                            warning_stage,
                            format!("failed to evaluate '{}': {}", original, error),
                        ));
                    }
                }
                }
                Err(e) if fail_fast => {
                    return Err(interpolation_error(
                        &loc.expression,
                        ExpressionError::Parse(e.to_string()),
                    ));
                }
                Err(e) => {
                    warnings.push(ComposeWarning::new(
                        warning_stage,
                        format!("failed to parse '{}': {}", loc.expression, e),
                    ));
                }
            }
        }

        total_count += count;
        all_warnings.extend(warnings);

        if count == 0 {
            break;
        }

        // If we hit the max depth with replacements still pending, add a warning.
        if depth == MAX_INTERPOLATION_DEPTH - 1 {
            all_warnings.push(ComposeWarning::new(
                warning_stage,
                format!(
                    "interpolation depth limit ({}) reached; possible infinite loop",
                    MAX_INTERPOLATION_DEPTH
                ),
            ));
        }
    }

    // `convert_literals` runs a full expression scan (a pulldown-cmark parse in
    // Markdown body mode) plus a copy. A `{{{ … }}}` literal is impossible
    // without the `{{{` sequence, so skip that work entirely when it's absent
    // (F14) — byte-identical: the scan would find no literals either way.
    let output = if output.contains("{{{") {
        convert_literals(&output, scan_mode)
    } else {
        output
    };

    Ok(InterpolationRewrite {
        output,
        replacements: total_count,
        warnings: all_warnings,
    })
}

/// Serializes `replacement` for the Markdown syntax position it lands in.
///
/// Only body scanning is syntax-aware. Frontmatter, directive arguments, and
/// opted-in code blocks keep the raw evaluated bytes so their own parser or
/// executor observes exactly what the expression produced.
fn project_body_replacement(
    replacement: String,
    expression: &Expr,
    context: InterpolationContext,
    scan_mode: ScanMode,
) -> String {
    if !matches!(scan_mode, ScanMode::Body { .. }) {
        return replacement;
    }
    match context {
        InterpolationContext::Prose => {
            if matches!(expression, Expr::FunctionCall { name, .. } if name == "raw_markdown") {
                replacement
            } else {
                escape_prose_literal(&replacement)
            }
        }
        // A code span's content is literal by construction, but its line
        // structure is not: a blank line ends the enclosing paragraph and
        // strands both delimiters. `CodeSpanRefencer` handles the delimiters.
        InterpolationContext::InlineCode => flatten_code_span_lines(&replacement),
        InterpolationContext::Raw
        | InterpolationContext::CodeBlock
        | InterpolationContext::Directive => replacement,
    }
}

/// Serializes `value` as CommonMark inline text.
///
/// Two hazard classes are neutralized. ASCII punctuation is backslash-escaped
/// so no inline construct — emphasis, a link, a code span, an autolink — can be
/// spelled by data. Line structure is flattened to single `&#10;` references so
/// no value can close the enclosing paragraph, open an indented code block, or
/// leave trailing spaces that CommonMark reads as a hard break. The numeric
/// reference keeps the break as text rather than source line structure, so the
/// parsed value is identical whether or not the cleanup pass runs.
fn escape_prose_literal(value: &str) -> String {
    let flattened = flatten_prose_lines(value);
    let contains_nested_expression = flattened.contains("{{");
    let content = flattened.trim_matches(|character| matches!(character, ' ' | '\t'));
    let edge_start = flattened.len() - flattened.trim_start_matches([' ', '\t']).len();
    let edge_end = edge_start + content.len();

    let mut escaped = String::with_capacity(flattened.len() + 8);
    for (offset, character) in flattened.char_indices() {
        match character {
            '\n' => escaped.push_str("&#10;"),
            // Horizontal whitespace at either edge of the value abuts document
            // text, where a leading run can open an indented code block and a
            // trailing run can become a hard break.
            ' ' if offset < edge_start || offset >= edge_end => escaped.push_str("&#32;"),
            '\t' if offset < edge_start || offset >= edge_end => escaped.push_str("&#9;"),
            // Nested replacements must remain visible to the fixpoint rescan;
            // the braces themselves are literal CommonMark text after it.
            character
                if character.is_ascii_punctuation()
                    && !(contains_nested_expression && matches!(character, '{' | '}')) =>
            {
                escaped.push('\\');
                escaped.push(character);
            }
            character => escaped.push(character),
        }
    }
    escaped
}

/// Collapses every line-break run — with the horizontal whitespace that
/// surrounds it — to a single `\n`.
fn flatten_prose_lines(value: &str) -> String {
    let mut flattened = String::with_capacity(value.len());
    let mut pending_break = false;
    let mut pending_spaces = String::new();
    for character in value.chars() {
        match character {
            '\n' | '\r' => {
                pending_break = true;
                pending_spaces.clear();
            }
            ' ' | '\t' if pending_break => {}
            ' ' | '\t' => pending_spaces.push(character),
            character => {
                if pending_break {
                    flattened.push('\n');
                    pending_break = false;
                } else {
                    flattened.push_str(&pending_spaces);
                }
                pending_spaces.clear();
                flattened.push(character);
            }
        }
    }
    if pending_break {
        flattened.push('\n');
    } else {
        flattened.push_str(&pending_spaces);
    }
    flattened
}

/// Collapses every line-break run in a code-span value to the single space
/// CommonMark itself substitutes for a line ending inside a code span.
fn flatten_code_span_lines(value: &str) -> String {
    let mut flattened = String::with_capacity(value.len());
    let mut pending_break = false;
    for character in value.chars() {
        match character {
            '\n' | '\r' => pending_break = true,
            character => {
                if std::mem::take(&mut pending_break) {
                    flattened.push(' ');
                }
                flattened.push(character);
            }
        }
    }
    if pending_break {
        flattened.push(' ');
    }
    flattened
}

/// Re-delimits every inline code span that received a replacement.
///
/// CommonMark closes a code span on the first backtick run whose length equals
/// the opening run, and strips one space from each end of the content. Neither
/// rule is expressible as a replacement string, so the span's own delimiters
/// must be rewritten once all of its replacements have landed.
struct CodeSpanRefencer {
    /// Lowest expression start inside each span. Replacements are applied from
    /// the end of the document backwards, so reaching this offset means every
    /// replacement inside the span is already in place.
    heads: HashMap<InlineCodeSpan, usize>,
    /// Net length change applied inside each span so far.
    deltas: HashMap<InlineCodeSpan, isize>,
}

impl CodeSpanRefencer {
    fn new(locations: &[ExpressionLocation]) -> Self {
        let mut heads: HashMap<InlineCodeSpan, usize> = HashMap::new();
        for location in locations {
            if let Some(span) = location.code_span {
                heads
                    .entry(span)
                    .and_modify(|head| *head = (*head).min(location.start))
                    .or_insert(location.start);
            }
        }
        Self { heads, deltas: HashMap::new() }
    }

    fn record(
        &mut self,
        location: &ExpressionLocation,
        inserted: usize,
        replaced: usize,
        output: &mut String,
    ) {
        let Some(span) = location.code_span else {
            return;
        };
        let delta = self.deltas.entry(span).or_insert(0);
        *delta += inserted as isize - replaced as isize;
        if self.heads.get(&span) != Some(&location.start) {
            return;
        }
        let Ok(end) = usize::try_from(span.end as isize + *delta) else {
            return;
        };
        if span.fence == 0 || end > output.len() || span.start + 2 * span.fence > end {
            return;
        }
        let (open, close) = (span.start + span.fence, end - span.fence);
        if !output.is_char_boundary(open) || !output.is_char_boundary(close) {
            return;
        }
        let content = &output[open..close];
        let refenced = fence_code_span(strip_code_span_padding(content));
        output.replace_range(span.start..end, &refenced);
    }
}

/// Undoes CommonMark's code-span padding rule to recover the content the
/// authored span denoted.
fn strip_code_span_padding(content: &str) -> &str {
    if content.starts_with(' ')
        && content.ends_with(' ')
        && content.chars().any(|character| character != ' ')
    {
        &content[1..content.len() - 1]
    } else {
        content
    }
}

/// Spells `content` as a code span whose delimiters cannot be closed early and
/// whose padding rule cannot consume the content's own spaces.
fn fence_code_span(content: &str) -> String {
    let mut longest_run = 0;
    let mut run = 0;
    for character in content.chars() {
        run = if character == '`' { run + 1 } else { 0 };
        longest_run = longest_run.max(run);
    }
    let fence = "`".repeat(longest_run + 1);
    let padded = content.starts_with('`')
        || content.ends_with('`')
        || (content.starts_with(' ')
            && content.ends_with(' ')
            && content.chars().any(|character| character != ' '));
    if padded {
        format!("{fence} {content} {fence}")
    } else {
        format!("{fence}{content}{fence}")
    }
}

/// Interpolates a single frontmatter value.
///
/// When `input`'s trimmed content is exactly one `{{ expr }}` (a *whole-value*
/// interpolation), the value is executable state, not text: it is parsed and
/// evaluated directly, and the typed `serde_json::Value` result is returned —
/// so `{{ false }}` stays the boolean `false` (falsy) rather than the string
/// `"false"` (truthy), `{{ file_index(x) }}` stays a number, and a whole-value
/// expression that yields an array or object is preserved as that typed value.
/// A whole-value parse or evaluation failure is **fatal regardless of
/// `fail_fast`**, so malformed expansion syntax (e.g. a mismatched paren) can
/// never leak downstream as a raw `{{ … }}` string.
///
/// Mixed text (`"a {{ x }}"`), strings holding more than one expression, and
/// plain strings fall through to [`interpolate_text`], keeping the established
/// lenient string-rewrite behavior — including leaving an unresolved `{{ … }}`
/// in place for a later pass and demoting parse/eval failures to warnings when
/// `fail_fast` is off.
///
/// ## Errors
///
/// Returns `MarkdownError::Transform` when a whole-value expression fails to
/// parse or evaluate, and propagates the same error as [`interpolate_text`]
/// when `fail_fast` is set or a fatal evaluation error occurs on the string
/// path. Whole-value undefined variables resolve to `null` (not an error), so
/// `{{ missing }}` stays lenient.
pub(crate) fn interpolate_value<L: EvaluationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    fail_fast: bool,
    warning_stage: &'static str,
) -> Result<(Value, usize, Vec<ComposeWarning>), MarkdownError> {
    if is_whole_value_literal(input) {
        let result = interpolate_text(input, evaluator, ScanMode::Plain, fail_fast, warning_stage)?;
        return Ok((Value::String(result.output), result.replacements, result.warnings));
    }
    if let Some(loc) = whole_value_span(input) {
        let expr = parse(&loc.expression).map_err(|e| {
            interpolation_error(&loc.expression, ExpressionError::Parse(e.to_string()))
        })?;
        // The whole-value path bypasses `interpolate_text`, so it must still run
        // the context-typo check on its single parsed expression — otherwise
        // `phase: "{{ ctx.toady }}"` (resolving to null) would warn in body text
        // but stay silent in frontmatter.
        let warnings = evaluator.collect_context_warnings(&expr, warning_stage);
        let value = evaluator
            .eval_json(&expr)
            .map_err(|cause| interpolation_error(&loc.expression, cause))?;
        return Ok((value, 1, warnings));
    }
    let result = interpolate_text(input, evaluator, ScanMode::Plain, fail_fast, warning_stage)?;
    Ok((Value::String(result.output), result.replacements, result.warnings))
}

/// Returns the single interpolation span when `input`'s trimmed content is
/// exactly one `{{ expr }}` (only whitespace before and after the span).
///
/// Returns `None` for plain strings, mixed text (`"a {{ x }}"`), and strings
/// holding more than one expression — those route to the lenient
/// [`interpolate_text`] string path. Detection is independent of parse/eval
/// outcome, so a malformed whole-value `{{ … }}` is still recognized as
/// whole-value and held to the strict parse-and-evaluate contract.
fn whole_value_span(input: &str) -> Option<ExpressionLocation> {
    let mut locations = ExpressionFinder::find_all_plain(input);
    if locations.len() != 1 {
        return None;
    }
    let loc = locations.remove(0);
    if !input[..loc.start].trim().is_empty() || !input[loc.end..].trim().is_empty() {
        return None;
    }
    Some(loc)
}

/// Returns `true` when `input`'s trimmed content is exactly one interpolation
/// literal (`{{{ ... }}}`).
///
/// Such values take the string-rewrite path rather than the typed whole-value
/// expression path, so they resolve to the literal text `{{ ... }}`.
fn is_whole_value_literal(input: &str) -> bool {
    let scan = ExpressionFinder::scan_plain(input);
    if scan.expressions.is_empty() && scan.literals.len() == 1 {
        let lit = &scan.literals[0];
        input[..lit.start].trim().is_empty() && input[lit.end..].trim().is_empty()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::EffectiveStateBuilder;
    use crate::markdown::compose::ComposeContext;
    use serde_json::json;
    use std::collections::HashMap;

    fn make_state(data: serde_json::Value) -> crate::markdown::compose::EffectiveState {
        let fm: HashMap<String, serde_json::Value> = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => HashMap::new(),
        };
        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(ComposeContext::fixed_for_testing())
            .build()
            .unwrap()
    }

    #[test]
    fn plain_mode_does_not_skip_code_spans() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let result =
            interpolate_text("`{{ name }}`", &evaluator, ScanMode::Plain, false, "test").unwrap();
        assert_eq!(result.output, "`Alice`");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn markdown_aware_scans_inline_code_spans() {
        // Inline code spans are scanned in body mode — only
        // fenced/indented code blocks are skipped.
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "`{{ name }}`",
            &evaluator,
            ScanMode::Body { include_code_blocks: false },
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "`Alice`");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn markdown_aware_skips_fenced_code_blocks() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        let input = "before {{ name }}\n\n```\n{{ name }}\n```\nafter";
        let result =
            interpolate_text(input, &evaluator, ScanMode::Body { include_code_blocks: false }, false, "test").unwrap();
        assert!(result.output.contains("before Alice"));
        assert!(result.output.contains("```\n{{ name }}\n```"));
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn multiline_indentation_inherited() {
        let state = make_state(json!({"items": "a\nb\nc"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "  list: {{ items }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "  list: a\n  b\n  c");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn fail_fast_returns_error_on_parse_failure() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        // An unparseable expression
        let result = interpolate_text("{{ > invalid }}", &evaluator, ScanMode::Plain, true, "test");
        assert!(result.is_err());
    }

    #[test]
    fn non_fail_fast_records_warnings() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ > invalid }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("failed to parse"));
        // Original is preserved
        assert!(result.output.contains("{{ > invalid }}"));
    }

    #[test]
    fn unknown_function_is_fatal_even_without_fail_fast() {
        // An unrecognized symbol can never resolve; it must surface as an error
        // rather than leaking its literal `{{ … }}` text downstream, even when
        // fail_fast is off.
        let state = make_state(json!({"spec": "a/b/spec.md"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ unknown_fn(spec) }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        );
        let Err(err) = result else {
            panic!("unknown function must be fatal");
        };
        assert!(err.to_string().contains("Unknown function: unknown_fn"));
    }

    #[test]
    fn no_expressions_returns_input_unchanged() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "no expressions here",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "no expressions here");
        assert_eq!(result.replacements, 0);
        assert!(result.warnings.is_empty());
    }

    /// F14 fast-path: input with single braces but no `{{` skips the scan
    /// pipeline entirely and returns verbatim (byte-identical to the scanning
    /// path, which would find zero expressions and zero literals).
    #[test]
    fn single_brace_input_takes_fast_path_verbatim() {
        let state = make_state(json!({"name": "Alice"}));
        let evaluator = Evaluator::new(&state);
        // Contains `{` and `}` but never `{{` — the JSON-ish body must survive
        // untouched under both scan modes.
        let input = "config { key: value } and a lone } brace";
        for mode in [ScanMode::Plain, ScanMode::Body { include_code_blocks: false }] {
            let result = interpolate_text(input, &evaluator, mode, false, "test").unwrap();
            assert_eq!(result.output, input);
            assert_eq!(result.replacements, 0);
            assert!(result.warnings.is_empty());
        }
    }

    /// F14 fast-path must NOT swallow `{{{ … }}}` literals: `{{{` contains `{{`,
    /// so the guard falls through to the normal literal-conversion path.
    #[test]
    fn triple_brace_literal_still_converted_despite_fast_path() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text("{{{ x }}}", &evaluator, ScanMode::Plain, false, "test")
            .unwrap();
        assert_eq!(result.output, "{{ x }}");
        assert_eq!(result.replacements, 0);
    }

    #[test]
    fn nested_ternary_in_true_branch_via_interpolate_text() {
        let state = make_state(json!({"a": true, "b": true}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? b ? 'inner-true' : 'inner-false' : 'outer-false' }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "inner-true");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn nested_ternary_in_false_branch_via_interpolate_text() {
        let state = make_state(json!({"a": false, "c": true}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? 'outer-true' : c ? 'inner-true' : 'inner-false' }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "inner-true");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn deeply_nested_ternary_via_interpolate_text() {
        let state = make_state(json!({"a": true, "b": true, "c": false}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ a ? b ? c ? 'd' : 'e' : 'f' : 'g' }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "e");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn nested_ternary_with_context_variable() {
        // ctx.today is always truthy in test context (returns "2024-06-15")
        let state = make_state(json!({"flag": false}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ ctx.today ? ctx.today : 'no date' }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "2024-06-15");
        assert_eq!(result.replacements, 1);
    }

    #[test]
    fn rescans_replacement_text_for_nested_interpolation() {
        // A ternary branch that contains an interpolation placeholder
        // should be resolved in a subsequent pass.
        let state = make_state(json!({"pkg": "darkmatter"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ pkg ? 'in a package directory: {{pkg}}' : 'not in a package directory' }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "in a package directory: darkmatter");
        assert_eq!(result.replacements, 2);
    }

    #[test]
    fn rescans_false_branch_for_nested_interpolation() {
        let state = make_state(json!({"pkg": null, "fallback": "none"}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ pkg ? 'has: {{pkg}}' : 'missing: {{fallback}}' }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "missing: none");
        assert_eq!(result.replacements, 2);
    }

    #[test]
    fn context_typo_emits_warning_with_suggestion() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ ctx.tody }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert_eq!(result.output, "");
        assert!(result.warnings.iter().any(|w| {
            w.message.contains("unknown context variable")
                && w.message.contains("today")
        }));
    }

    #[test]
    fn valid_context_variable_emits_no_typo_warning() {
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let result = interpolate_text(
            "{{ ctx.today }}",
            &evaluator,
            ScanMode::Plain,
            false,
            "test",
        )
        .unwrap();
        assert!(!result.output.is_empty());
        assert!(!result.warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    // ── Whole-value frontmatter context-typo coverage ─────────────────────
    //
    // `interpolate_value`'s whole-value path bypasses `interpolate_text`, so
    // the same ctx-typo diagnostic must still fire when the whole value is a
    // single `{{ expr }}`.

    #[test]
    fn whole_value_scalar_typo_emits_warning_with_suggestion() {
        // `ctx.toady` is unknown and resolves to null, taking the whole-value
        // path — it must still warn.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let (value, replacements, warnings) =
            interpolate_value("{{ ctx.toady }}", &evaluator, false, "frontmatter-interpolation")
                .unwrap();
        // Silent-null evaluation is unchanged: still null, still one replacement.
        assert_eq!(value, Value::Null);
        assert_eq!(replacements, 1);
        assert!(warnings.iter().any(|w| {
            w.message.contains("unknown context variable") && w.message.contains("today")
        }));
    }

    #[test]
    fn whole_value_scalar_string_literal_does_not_warn() {
        // A string literal that merely *spells* `ctx.toady` must not warn:
        // the check is AST-based, and a string literal evaluates to a String
        // (so it falls through to the string path, never the scalar one) — but
        // either way no ctx reference exists in the AST.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        let (_value, _replacements, warnings) = interpolate_value(
            r#"{{ "ctx.toady" }}"#,
            &evaluator,
            false,
            "frontmatter-interpolation",
        )
        .unwrap();
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    #[test]
    fn whole_value_scalar_valid_ctx_does_not_warn() {
        // A valid `ctx.*` reference that resolves to a scalar (a numeric ctx
        // value) takes the fast-path but must not warn.
        let state = make_state(json!({}));
        let evaluator = Evaluator::new(&state);
        // ctx.year resolves to a number in the fixed test context.
        let (value, replacements, warnings) =
            interpolate_value("{{ number(ctx.year) }}", &evaluator, false, "frontmatter-interpolation")
                .unwrap();
        assert!(matches!(value, Value::Number(_)));
        assert_eq!(replacements, 1);
        assert!(!warnings.iter().any(|w| w.message.contains("unknown context variable")));
    }

    mod interpolation_literal_tests {
        use super::*;

        #[test]
        fn body_literal_converts_to_literal_text() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{{ name }}}",
                &evaluator,
                ScanMode::Body { include_code_blocks: false },
                false,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ name }}");
            assert_eq!(result.replacements, 0);
            assert!(result.warnings.is_empty());
        }

        #[test]
        fn inline_code_literal_converts() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "`{{{ name }}}`",
                &evaluator,
                ScanMode::Body { include_code_blocks: false },
                false,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "`{{ name }}`");
            assert_eq!(result.replacements, 0);
        }

        #[test]
        fn fenced_code_block_literal_is_untouched() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let input = "```\n{{{ name }}}\n```";
            let result = interpolate_text(input, &evaluator, ScanMode::Body { include_code_blocks: false }, false, "test").unwrap();
            assert_eq!(result.output, input);
            assert_eq!(result.replacements, 0);
        }

        #[test]
        fn tight_and_empty_literal_forms() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            for (input, expected) in [
                ("{{{x}}}", "{{x}}"),
                ("{{{}}}", "{{}}"),
                ("{{{ }}}", "{{ }}"),
            ] {
                let result = interpolate_text(input, &evaluator, ScanMode::Plain, false, "test").unwrap();
                assert_eq!(result.output, expected, "input: {input:?}");
                assert_eq!(result.replacements, 0, "input: {input:?}");
            }
        }

        #[test]
        fn adjacent_expression_and_literal() {
            let state = make_state(json!({"a": "evaluated"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ a }}{{{ b }}}",
                &evaluator,
                ScanMode::Plain,
                false,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "evaluated{{ b }}");
            assert_eq!(result.replacements, 1);
        }

        #[test]
        fn rescan_loop_converts_introduced_literal() {
            let state = make_state(json!({"tmpl": "{{{ y }}}"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{ tmpl }}",
                &evaluator,
                ScanMode::Plain,
                false,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ y }}");
            assert_eq!(result.replacements, 1);
        }

        #[test]
        fn fail_fast_over_only_literals_succeeds() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{{ name }}} {{{ other }}}",
                &evaluator,
                ScanMode::Plain,
                true,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ name }} {{ other }}");
            assert_eq!(result.replacements, 0);
            assert!(result.warnings.is_empty());
        }

        #[test]
        fn whole_value_literal_takes_string_path() {
            let state = make_state(json!({}));
            let evaluator = Evaluator::new(&state);
            let (value, replacements, warnings) =
                interpolate_value("{{{ x }}}", &evaluator, false, "frontmatter-interpolation").unwrap();
            assert_eq!(value, Value::String("{{ x }}".to_string()));
            assert_eq!(replacements, 0);
            assert!(warnings.is_empty());
        }

        #[test]
        fn literal_containing_expression_is_not_evaluated() {
            let state = make_state(json!({"x": "replaced"}));
            let evaluator = Evaluator::new(&state);
            let result = interpolate_text(
                "{{{ {{ x }} }}}",
                &evaluator,
                ScanMode::Plain,
                false,
                "test",
            )
            .unwrap();
            assert_eq!(result.output, "{{ {{ x }} }}");
            assert_eq!(result.replacements, 0);
        }

        #[test]
        fn convert_literals_plain_leaves_expressions_intact() {
            let output = convert_literals("{{ a }}{{{ b }}}", ScanMode::Plain);
            assert_eq!(output, "{{ a }}{{ b }}");
        }
    }
}
