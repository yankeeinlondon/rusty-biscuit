//! Frontmatter shell expansion parsing and validation.
//!
//! Scans top-level frontmatter string values for `$(cmd)` shell expressions,
//! parses them, validates the executable-token interpolation rule, and returns
//! structured directives ready for execution.

use super::expression::{evaluate, is_truthy, parse_condition};
use super::frontmatter_interpolation::FrontmatterSeedState;
use super::interpolation::{Evaluator, ScanMode, interpolate_text};
use super::shell_expansion::store::resolve_policy_paths;
use super::shell_expansion::tokenize::{parse_pipeline, tokenize};
use super::shell_expansion::types::{
    ChainOperator, ErrorHandling, PipelineRuntime, ShellCommandOrigin, ShellDirective,
    ShellExpansionError, ShellPipeline, ShellPolicyPaths,
};
use super::shell_expansion::{PreparedShellDirective, execute_prepared_directive, prepare_directive};
use super::types::{ComposeOptions, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::types::MarkdownResult;
use biscuit_terminal::errors::SourceContext;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

/// Parsed shape of a `$(...)` frontmatter shell expression.
///
/// The shell-directive parser produces one of two variants:
///
/// - [`FrontmatterShellAst::Pipeline`] — a plain command pipeline (today's path).
/// - [`FrontmatterShellAst::Ternary`] — a conditional `COND ? THEN : ELSE`
///   whose branches each evaluate to either an empty literal or a pipeline.
///
/// The condition is captured as the original, pre-interpolation text so that
/// it can be evaluated by the template expression engine at execution time;
/// it never reaches the shell. The branch AST exists so that validation and
/// allowlist enforcement can walk every reachable command before any branch
/// is selected.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are consumed by per-branch execution in phase 3.
pub(crate) enum FrontmatterShellAst {
    /// A bare command pipeline.
    Pipeline(ShellPipeline),
    /// A ternary `COND ? THEN : ELSE` with independently parsed branches.
    Ternary {
        /// Original (pre-interpolation) text of the condition expression.
        condition_source: String,
        then_branch: Branch,
        else_branch: Branch,
    },
}

/// One side of a ternary in a frontmatter shell expression.
///
/// `Empty` corresponds to a literal empty string (`''` / `""`) and contributes
/// no command; selecting it short-circuits the directive to `""`. `Pipeline`
/// carries the pre-interpolation branch text from the original ternary AST.
/// At execution time the text is interpolated against the frontmatter seed
/// state and then tokenized into a pipeline. Anchoring the branch boundary
/// to the original — instead of re-splitting the resolved whole `$(...)`
/// body — prevents condition interpolation from introducing `?` or `:`
/// that would retroactively shift branch boundaries and bleed executable
/// text into a branch.
#[derive(Debug, Clone)]
pub(crate) enum Branch {
    /// Literal empty string branch — produces no shell invocation.
    Empty,
    /// Command pipeline branch — interpolated and tokenized at execute time.
    Pipeline {
        /// Pre-interpolation branch text, exactly as sliced from the original
        /// `$(...)` body.
        original_text: String,
    },
}

/// Which side of a ternary a parse error or validation diagnostic refers to.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Variants are consumed in phase 3 / phase 4 branch wiring.
pub(crate) enum BranchPosition {
    Then,
    Else,
}

impl BranchPosition {
    fn name(self) -> &'static str {
        match self {
            Self::Then => "then-branch",
            Self::Else => "else-branch",
        }
    }
}

impl Branch {
    /// Returns the static (pre-interpolation) source text for this branch.
    ///
    /// `Branch::Empty` yields nothing; `Branch::Pipeline` yields the single
    /// original branch slice. Used by diagnostics and tests; allowlist
    /// enforcement walks the interpolated pipeline at execute time.
    #[allow(dead_code)]
    pub(crate) fn original_text(&self) -> Option<&str> {
        match self {
            Self::Empty => None,
            Self::Pipeline { original_text } => Some(original_text.as_str()),
        }
    }
}

/// A parsed frontmatter shell directive ready for execution.
///
/// Holds the parsed [`FrontmatterShellAst`] alongside execution-time metadata.
/// The legacy `executable`, `args`, and `pipeline` fields are retained for
/// in-crate inspection by parser tests; production code reads the AST
/// directly via `directive_reachable_pipelines` and `ast`. For a `Ternary`
/// AST the legacy fields are placeholders (empty / `None`).
#[derive(Debug, Clone)]
pub(crate) struct FrontmatterShellDirective {
    pub key: String,
    pub raw_command: String,
    #[allow(dead_code)] // Inspected by parser tests; production reads `ast`.
    pub executable: String,
    #[allow(dead_code)] // Inspected by parser tests; production reads `ast`.
    pub args: Vec<String>,
    pub timeout_override: Option<std::time::Duration>,
    #[allow(dead_code)] // Inspected by parser tests; production uses `directive_reachable_pipelines`.
    pub pipeline: Option<ShellPipeline>,
    pub ast: FrontmatterShellAst,
}

/// Result of frontmatter shell expansion.
pub(crate) struct FrontmatterShellExpansionReport {
    /// Number of frontmatter values rewritten.
    pub replacements: usize,
    /// Number of shell approvals consumed.
    pub approvals_used: usize,
    /// Warnings emitted.
    pub warnings: Vec<ComposeWarning>,
}

/// Parses a single frontmatter string value for shell expression.
///
/// Returns `Some` if the value matches `$(cmd)` or `$(cmd)::timeout:N`.
///
/// ## Rules
///
/// - Value must start with `$(`
/// - Must have a closing `)` — either at the end, or followed by `::timeout:N`
/// - If `::timeout:N` suffix present: extract it, validate N > 0
/// - Extract the inner command string between `$(` and `)`
/// - Tokenize with the existing tokenizer
/// - If `original_value` is provided (pre-interpolation snapshot), check the
///   executable-token interpolation rule:
///   - Find the executable portion in the ORIGINAL string (first token between
///     `$(` and first whitespace)
///   - If that portion contains `{{` and `}}`, reject (return None)
///   - Also check after pipe/chain operators in the original for executable-position
///     interpolation
/// - Return an error for any parse/validation failure once the value matches
///   the `$(` shell-expression shape
pub(crate) fn parse_shell_value(
    value: &str,
    key: &str,
    original_value: Option<&str>,
    ctx: &SourceContext,
) -> Result<Option<FrontmatterShellDirective>, ShellExpansionError> {
    // Must start with $(
    if !value.starts_with("$(") {
        return Ok(None);
    }

    let rest = &value[2..];
    let close_pos = find_unquoted_closing_paren(rest, key, ctx)?;

    let inner_command = &rest[..close_pos];
    let after_close = &rest[close_pos + 1..];

    // Check for ::timeout:N suffix
    let timeout_override = if let Some(timeout_str) = after_close.strip_prefix("::timeout:") {
        let timeout_val: u64 = timeout_str.parse().map_err(|_| {
            frontmatter_parse_error(
                key,
                ctx,
                "Invalid ::timeout value in frontmatter shell expression; expected a positive integer number of seconds",
            )
        })?;
        if timeout_val == 0 {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                "Frontmatter shell timeout must be greater than zero",
            ));
        }
        Some(std::time::Duration::from_secs(timeout_val))
    } else if !after_close.is_empty() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            "Unexpected trailing content after frontmatter shell expression",
        ));
    } else {
        None
    };

    // The original (pre-interpolation) inner text drives ternary structure
    // detection and per-branch executable-interpolation validation. If no
    // original snapshot is supplied, the resolved value IS the original.
    let original_inner = extract_original_inner(original_value, key, ctx)?;
    let validation_inner = original_inner.unwrap_or(inner_command);

    // Ternary path? Structure is determined exclusively from the original so
    // that interpolation in the condition cannot retroactively change branch
    // boundaries. Per-branch resolved text is produced at execution time by
    // interpolating each original branch slice independently.
    if let Some((orig_cond, orig_then, orig_else)) = split_top_level_ternary(validation_inner) {
        // Reject nested ternaries (a v1 non-goal): a top-level `?` followed
        // by a later top-level `:` inside a branch indicates a second ternary.
        // A bare colon without a preceding `?` is allowed — it is valid
        // pipeline content (URLs, times, key:value arguments).
        reject_nested_ternary(orig_then, BranchPosition::Then, key, ctx)?;
        reject_nested_ternary(orig_else, BranchPosition::Else, key, ctx)?;

        let then_branch = parse_branch_from_original(orig_then, BranchPosition::Then, key, ctx)?;
        let else_branch = parse_branch_from_original(orig_else, BranchPosition::Else, key, ctx)?;

        let condition_source = orig_cond.trim().to_string();
        if condition_source.is_empty() {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                "Frontmatter shell ternary condition must not be empty",
            ));
        }

        return Ok(Some(FrontmatterShellDirective {
            key: key.to_string(),
            raw_command: inner_command.to_string(),
            executable: String::new(),
            args: Vec::new(),
            timeout_override,
            pipeline: None,
            ast: FrontmatterShellAst::Ternary {
                condition_source,
                then_branch,
                else_branch,
            },
        }));
    }

    // A top-level `?` without a matching `:` is an unbalanced ternary, not a
    // silent fall-through to pipeline parsing.
    if find_top_level_separator(validation_inner, '?').is_some() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            "Frontmatter shell ternary missing ':' separator after '?'",
        ));
    }

    // Check executable-token interpolation rule if we have the original
    if let Some(original) = original_value {
        validate_no_executable_interpolation(original, key, ctx)?;
    }

    // Tokenize the inner command
    let tokens =
        tokenize(inner_command, ctx).map_err(|error| remap_parse_error(error, key, ctx))?;

    // Parse into a pipeline
    let pipeline =
        parse_pipeline(&tokens, ctx).map_err(|error| remap_parse_error(error, key, ctx))?;

    // Shape preservation: an interpolation in argument position must not
    // synthesize new pipeline actions or change the executable token. We
    // already reject `{{...}}` in executable position via
    // `validate_no_executable_interpolation`; this catches arg-position
    // values that contain `&&` / `||` (or quote-escaping payloads) that
    // would extend the pipeline at execute time.
    if original_inner.is_some() {
        let original_pipeline = parse_static_pipeline_shape(validation_inner, ctx)
            .map_err(|error| remap_parse_error(error, key, ctx))?;
        validate_pipeline_shape_preserved(&original_pipeline, &pipeline, None, key, ctx)?;
    }

    let executable = pipeline.actions[0].command.executable.clone();
    let args = pipeline.actions[0].command.args.clone();

    Ok(Some(FrontmatterShellDirective {
        key: key.to_string(),
        raw_command: inner_command.to_string(),
        executable,
        args,
        timeout_override,
        pipeline: Some(pipeline.clone()),
        ast: FrontmatterShellAst::Pipeline(pipeline),
    }))
}

/// Extracts the inner `$(...)` body from the pre-interpolation snapshot.
///
/// Returns `Ok(None)` when no snapshot was supplied or when the snapshot does
/// not itself match the `$(...)` shape (e.g. the resolved form invented `$(`).
/// Returns `Err` if the snapshot opens `$(` but never closes it.
fn extract_original_inner<'a>(
    original_value: Option<&'a str>,
    key: &str,
    ctx: &SourceContext,
) -> Result<Option<&'a str>, ShellExpansionError> {
    let Some(original) = original_value else {
        return Ok(None);
    };
    if !original.starts_with("$(") {
        return Ok(None);
    }
    let rest = &original[2..];
    let close_pos = find_unquoted_closing_paren(rest, key, ctx)?;
    Ok(Some(&rest[..close_pos]))
}

/// Splits the inner text of a `$(...)` body at the first top-level
/// whitespace-padded `?` / `:` separator pair, returning
/// `(condition, then, else)` slices.
///
/// "Top-level" means: not inside single quotes, double quotes, or a balanced
/// `(` / `)` pair, and not preceded by a backslash escape. A `?` or `:`
/// counts as a separator only when surrounded by whitespace (or sitting at
/// the slice boundary). This keeps URL-style arguments like
/// `http://example.com`, `cmd:arg` key/value pairs, and glob fragments such
/// as `file?.txt` from being misclassified as ternary punctuation. Returns
/// `None` when no such pair exists.
fn split_top_level_ternary(inner: &str) -> Option<(&str, &str, &str)> {
    let q_pos = find_top_level_separator(inner, '?')?;
    let after_q = &inner[q_pos + '?'.len_utf8()..];
    let c_rel = find_top_level_separator(after_q, ':')?;
    let c_pos = q_pos + '?'.len_utf8() + c_rel;
    Some((
        &inner[..q_pos],
        &inner[q_pos + '?'.len_utf8()..c_pos],
        &inner[c_pos + ':'.len_utf8()..],
    ))
}

/// Returns the byte offset of the first top-level whitespace-padded
/// occurrence of `target`.
///
/// "Top-level" respects single quotes, double quotes, parentheses, and
/// backslash escapes. "Whitespace-padded" means the byte immediately
/// before `target` is whitespace (or `target` sits at the start of the
/// slice) and the byte immediately after is whitespace (or end-of-slice).
/// This is the rule that distinguishes ternary separators from the same
/// characters appearing inside argument text such as `http://`, `cmd:arg`,
/// or `file?.txt`.
fn find_top_level_separator(input: &str, target: char) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut paren_depth: u32 = 0;
    let bytes = input.as_bytes();

    for (idx, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' if !in_single && !in_double => paren_depth += 1,
            ')' if !in_single && !in_double && paren_depth > 0 => paren_depth -= 1,
            c if c == target && !in_single && !in_double && paren_depth == 0 => {
                let pre_ok = idx == 0
                    || matches!(
                        bytes.get(idx.saturating_sub(1)),
                        Some(b' ') | Some(b'\t')
                    );
                let after_idx = idx + c.len_utf8();
                let post_ok = after_idx == bytes.len()
                    || matches!(bytes.get(after_idx), Some(b' ') | Some(b'\t'));
                if pre_ok && post_ok {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

/// Parses one side of a ternary into a [`Branch`] from the original
/// (pre-interpolation) source slice.
///
/// The resolved branch text is produced at execution time by interpolating
/// `original_text` against the frontmatter seed state; deferring tokenization
/// keeps the branch boundary pinned to the original ternary AST. A bare
/// empty-string literal — `''` or `""`, with optional surrounding whitespace
/// — produces [`Branch::Empty`] and contributes no command.
fn parse_branch_from_original(
    original: &str,
    position: BranchPosition,
    key: &str,
    ctx: &SourceContext,
) -> Result<Branch, ShellExpansionError> {
    let original_trim = original.trim();

    if is_empty_string_literal(original_trim) {
        return Ok(Branch::Empty);
    }

    if original_trim.is_empty() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary {} must be a command pipeline or an empty string literal",
                position.name()
            ),
        ));
    }

    validate_branch_no_executable_interpolation(original, position, key, ctx)?;

    Ok(Branch::Pipeline {
        original_text: original.to_string(),
    })
}

/// Rejects nested ternaries inside a branch slice.
///
/// A nested ternary manifests as an additional separator-style `?`
/// (whitespace-padded, top-level, unquoted) inside one of the branches.
/// Bare colons are not a signal — they appear legitimately in URLs
/// (`http://`), times (`12:30`), and `key:value` arguments — so only
/// `?` is checked. Glob `?` and other in-word question marks are ignored
/// by the separator rule and pass through to the shell tokenizer.
fn reject_nested_ternary(
    branch: &str,
    position: BranchPosition,
    key: &str,
    ctx: &SourceContext,
) -> Result<(), ShellExpansionError> {
    if find_top_level_separator(branch, '?').is_some() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary {} contains an additional separator-style '?'; nested ternaries are not supported",
                position.name()
            ),
        ));
    }
    Ok(())
}

/// Recognizes `''` or `""` as a bare empty-string literal branch form.
fn is_empty_string_literal(s: &str) -> bool {
    s == "''" || s == "\"\""
}

/// Per-branch executable-interpolation check.
///
/// Walks the original (pre-interpolation) branch text, splits it at
/// top-level `&&` / `||` chain operators, and rejects any segment whose
/// first token contains `{{` and `}}`. Mirrors the rule enforced by
/// [`validate_no_executable_interpolation`] but operates on a branch slice
/// and embeds the branch name into the error message.
fn validate_branch_no_executable_interpolation(
    branch: &str,
    position: BranchPosition,
    key: &str,
    ctx: &SourceContext,
) -> Result<(), ShellExpansionError> {
    for segment in split_at_chain_operators(branch) {
        let executable_portion = first_token_portion(segment);
        if executable_portion.contains("{{") && executable_portion.contains("}}") {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                format!(
                    "Frontmatter shell executable may not come from interpolation in {} of ternary",
                    position.name()
                ),
            ));
        }
    }
    Ok(())
}

fn remap_branch_parse_error(
    error: ShellExpansionError,
    position: BranchPosition,
    key: &str,
    ctx: &SourceContext,
) -> ShellExpansionError {
    match error {
        ShellExpansionError::ParseDirective { message, .. } => frontmatter_parse_error(
            key,
            ctx,
            format!("In {} of ternary: {message}", position.name()),
        ),
        other => other,
    }
}

/// Parses the static shape of a pre-interpolation pipeline by first masking
/// every `{{...}}` placeholder with a shell-inert sentinel token, then
/// running the shared tokenizer and pipeline parser.
///
/// The returned [`ShellPipeline`] captures the author's intended action
/// count, chain operators, and executable tokens. Arg counts will not
/// match the post-interpolation pipeline (an interpolated value can
/// legally expand to additional words) — only the structural fields are
/// suitable for comparison.
fn parse_static_pipeline_shape(
    original: &str,
    ctx: &SourceContext,
) -> Result<ShellPipeline, ShellExpansionError> {
    let masked = mask_interpolations(original);
    let tokens = tokenize(&masked, ctx)?;
    parse_pipeline(&tokens, ctx)
}

/// Replaces every `{{...}}` interpolation marker with a sentinel word that
/// contains no shell metacharacters, so the original text can be fed to
/// the shell tokenizer without risk of the placeholder body confusing
/// quote tracking or chain-operator detection.
///
/// Unterminated `{{` with no matching `}}` is left untouched; the
/// downstream tokenizer will surface a diagnostic against the original
/// shape if that matters.
fn mask_interpolations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    let mut counter: usize = 0;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        match after_open.find("}}") {
            Some(close) => {
                out.push_str(&format!("__DM_INTERP_{counter}__"));
                counter += 1;
                rest = &after_open[close + 2..];
            }
            None => {
                out.push_str(&rest[open..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Asserts that interpolation has not changed the static shape of a
/// pipeline — its action count, chain operators, or executable tokens.
///
/// Argument counts intentionally are not compared: an interpolated value
/// can legally tokenize into multiple words (e.g. `echo {{x}}` with
/// `x = "a b"` becomes `echo a b`). The structural fields are the ones
/// that gate the static-allowlist invariant: every executable the
/// directive could run must be statically determinable at parse time.
///
/// `position` is `Some(...)` when validating a ternary branch and `None`
/// for a bare pipeline; the diagnostic text adapts accordingly.
fn validate_pipeline_shape_preserved(
    original: &ShellPipeline,
    resolved: &ShellPipeline,
    position: Option<BranchPosition>,
    key: &str,
    ctx: &SourceContext,
) -> Result<(), ShellExpansionError> {
    let where_text = match position {
        Some(p) => format!(" in {} of ternary", p.name()),
        None => String::new(),
    };

    if original.actions.len() != resolved.actions.len() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell pipeline{where_text} changed from {} action(s) to {} after interpolation; \
                 interpolated argument values may not introduce new chain operators ('&&' / '||') or break out of quoting",
                original.actions.len(),
                resolved.actions.len(),
            ),
        ));
    }

    for (idx, (orig_action, new_action)) in original
        .actions
        .iter()
        .zip(resolved.actions.iter())
        .enumerate()
    {
        if orig_action.operator != new_action.operator {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                format!(
                    "Frontmatter shell pipeline{where_text} chain operator at action {} changed from {} to {} after interpolation",
                    idx,
                    operator_label(orig_action.operator),
                    operator_label(new_action.operator),
                ),
            ));
        }
        if orig_action.command.executable != new_action.command.executable {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                format!(
                    "Frontmatter shell pipeline{where_text} executable at action {} changed from `{}` to `{}` after interpolation; \
                     interpolation may not change the executable",
                    idx,
                    orig_action.command.executable,
                    new_action.command.executable,
                ),
            ));
        }
    }

    Ok(())
}

fn operator_label(op: ChainOperator) -> &'static str {
    match op {
        ChainOperator::None => "(first)",
        ChainOperator::And => "'&&'",
        ChainOperator::Or => "'||'",
    }
}

/// Validates that no executable token in the pipeline comes from interpolation.
///
/// Checks the ORIGINAL (pre-interpolation) string to ensure `{{ }}` doesn't
/// appear in any executable position — that is, the first non-whitespace token
/// after `$(`, plus the first non-whitespace token after every top-level `&&`
/// or `||` chain operator.
fn validate_no_executable_interpolation(
    original: &str,
    key: &str,
    ctx: &SourceContext,
) -> Result<(), ShellExpansionError> {
    // Must start with $(
    if !original.starts_with("$(") {
        return Ok(());
    }

    let rest = &original[2..];
    let close_pos = find_unquoted_closing_paren(rest, key, ctx)?;

    let inner = &rest[..close_pos];

    for segment in split_at_chain_operators(inner) {
        let executable_portion = first_token_portion(segment);
        if executable_portion.contains("{{") && executable_portion.contains("}}") {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                "Frontmatter shell executable may not come from interpolation",
            ));
        }
    }

    Ok(())
}

/// Splits an inner `$(...)` body at top-level `&&` and `||` operators,
/// respecting single quotes, double quotes, and backslash escaping.
///
/// Each returned segment corresponds to one chain action; the executable
/// position of that action is the first non-whitespace token of the segment.
fn split_at_chain_operators(input: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let bytes = input.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }

        let ch = bytes[i];
        match ch {
            b'\\' if !in_single => {
                escaped = true;
                i += 1;
            }
            b'\'' if !in_double => {
                in_single = !in_single;
                i += 1;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                i += 1;
            }
            b'&' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'&' => {
                segments.push(&input[start..i]);
                i += 2;
                start = i;
            }
            b'|' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                segments.push(&input[start..i]);
                i += 2;
                start = i;
            }
            _ => {
                i += 1;
            }
        }
    }

    segments.push(&input[start..]);
    segments
}

/// Scans all top-level string-valued frontmatter entries and returns candidates.
///
/// Only examines top-level keys — nested objects and arrays are ignored.
pub(crate) fn scan_frontmatter(
    frontmatter: &Frontmatter,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
    ctx: &SourceContext,
) -> Result<Vec<FrontmatterShellDirective>, ShellExpansionError> {
    let mut directives = Vec::new();
    let fm = frontmatter.as_map();

    for (key, value) in fm.iter() {
        // Only process top-level string values
        if let Value::String(s) = value {
            let original = pre_interpolation_snapshot
                .and_then(|map| map.get(key))
                .map(|s| s.as_str());

            if let Some(directive) = parse_shell_value(s, key, original, ctx)? {
                directives.push(directive);
            }
        }
    }

    Ok(directives)
}

/// Executes all frontmatter shell expansion directives and rewrites
/// frontmatter values in place.
///
/// Scans top-level string-valued frontmatter entries for `$(...)` expressions,
/// validates them, executes approved commands through the shared shell runtime,
/// trims output, and rewrites the frontmatter values.
///
/// ## Errors
///
/// Returns an error if:
/// - Policy file I/O fails
/// - Command execution fails (blacklisted, denied, execution error, etc.)
pub(crate) fn execute_frontmatter_shell_expansion(
    frontmatter: &mut Frontmatter,
    options: &ComposeOptions,
    runtime: &mut PipelineRuntime,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
    ctx: &SourceContext,
) -> MarkdownResult<FrontmatterShellExpansionReport> {
    let candidates = scan_frontmatter(frontmatter, pre_interpolation_snapshot, ctx)?;

    if candidates.is_empty() {
        return Ok(FrontmatterShellExpansionReport {
            replacements: 0,
            approvals_used: 0,
            warnings: vec![],
        });
    }

    // Resolve policy paths once for all directives
    let shell_opts = options.shell_options();
    let policy_paths = resolve_policy_paths(&shell_opts, &options.source)?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    // Snapshot frontmatter values for ternary condition lookups. Built lazily
    // so non-ternary inputs don't pay the cloning cost.
    let mut seed_state: Option<FrontmatterSeedState> = None;

    // Either "execute this prepared pipeline" or "short-circuit to empty".
    // PreparedShellDirective is hefty (~432 bytes), so box the variant to
    // keep the enum compact.
    enum Pending {
        Execute(Box<PreparedShellDirective>),
        Empty,
    }

    let mut pending: Vec<(usize, String, Pending)> = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.iter().enumerate() {
        let pending_item = match &candidate.ast {
            FrontmatterShellAst::Pipeline(pipeline) => {
                let prepared = prepare_branch_pipeline(
                    pipeline.clone(),
                    candidate.raw_command.clone(),
                    candidate,
                    options,
                    &policy_paths,
                    runtime,
                    ctx,
                )?;
                Pending::Execute(Box::new(prepared))
            }
            FrontmatterShellAst::Ternary {
                condition_source,
                then_branch,
                else_branch,
            } => {
                // The seed state drives both per-branch interpolation and
                // condition evaluation; build it once on first ternary.
                let state = match seed_state.as_ref() {
                    Some(s) => s,
                    None => {
                        let map: HashMap<String, Value> = frontmatter
                            .as_map()
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        seed_state =
                            Some(FrontmatterSeedState::new(map, options.context().clone()));
                        seed_state.as_ref().unwrap()
                    }
                };

                // Every command that the directive could execute must be
                // allowlisted, so prepare BOTH branches before evaluating
                // the condition. An unselected branch with an un-approved
                // command still fails the entire directive.
                let then_prepared = prepare_optional_branch(
                    then_branch,
                    candidate,
                    options,
                    &policy_paths,
                    runtime,
                    ctx,
                    state,
                    BranchPosition::Then,
                )?;
                let else_prepared = prepare_optional_branch(
                    else_branch,
                    candidate,
                    options,
                    &policy_paths,
                    runtime,
                    ctx,
                    state,
                    BranchPosition::Else,
                )?;

                let pick_then = evaluate_ternary_condition(
                    condition_source,
                    state,
                    &candidate.key,
                    ctx,
                )?;

                let selected = if pick_then { then_prepared } else { else_prepared };
                match selected {
                    None => Pending::Empty,
                    Some(prepared) => Pending::Execute(prepared),
                }
            }
        };

        pending.push((index, candidate.key.clone(), pending_item));
    }

    // Run executable preparations in parallel; short-circuit branches contribute
    // an empty string and skip the shell runtime entirely.
    let mut executions: Vec<_> = pending
        .into_par_iter()
        .map(|(index, key, item)| {
            let result = match item {
                Pending::Execute(prepared) => execute_prepared_directive(&prepared, options)
                    .map(|res| (res.stdout, res.warnings)),
                Pending::Empty => Ok((String::new(), Vec::new())),
            };
            (index, key, result)
        })
        .collect();
    executions.sort_by_key(|(index, _, _)| *index);

    // Rewrite frontmatter values
    let fm_mut = frontmatter.as_map_mut();
    let mut warnings = Vec::new();
    for (_, key, execution) in executions {
        let (stdout, exec_warnings) = execution?;
        warnings.extend(exec_warnings);
        fm_mut.insert(key, Value::String(stdout.trim().to_string()));
    }

    let approvals_used = runtime.shell.take_recent_approval_count();

    Ok(FrontmatterShellExpansionReport {
        replacements: candidates.len(),
        approvals_used,
        warnings,
    })
}

/// Returns every shell pipeline that this directive could execute at runtime.
///
/// For a [`FrontmatterShellAst::Pipeline`] directive: returns the single
/// captured pipeline. For a [`FrontmatterShellAst::Ternary`] directive:
/// interpolates each non-[`Branch::Empty`] branch against a freshly built
/// [`FrontmatterSeedState`] (using `frontmatter`'s current values and
/// `options.context()`), tokenizes, and parses each into a [`ShellPipeline`].
/// [`Branch::Empty`] contributes no pipeline.
///
/// Used by preflight shell-command discovery so the allowlist/approval
/// workflow surfaces commands from *both* ternary branches — not just the
/// one that runtime condition evaluation would select.
///
/// ## Errors
///
/// Returns an error if a branch's text fails interpolation, tokenization,
/// or pipeline parsing.
pub(crate) fn directive_reachable_pipelines(
    directive: &FrontmatterShellDirective,
    frontmatter: &Frontmatter,
    options: &ComposeOptions,
    ctx: &SourceContext,
) -> Result<Vec<ShellPipeline>, ShellExpansionError> {
    match &directive.ast {
        FrontmatterShellAst::Pipeline(pipeline) => Ok(vec![pipeline.clone()]),
        FrontmatterShellAst::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            let map: HashMap<String, Value> = frontmatter
                .as_map()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let state = FrontmatterSeedState::new(map, options.context().clone());

            let mut pipelines = Vec::new();
            for (branch, position) in [
                (then_branch, BranchPosition::Then),
                (else_branch, BranchPosition::Else),
            ] {
                if let Branch::Pipeline { original_text } = branch {
                    let resolved = interpolate_branch_text(
                        original_text,
                        &state,
                        position,
                        &directive.key,
                        ctx,
                    )?;
                    let tokens = tokenize(&resolved, ctx).map_err(|error| {
                        remap_branch_parse_error(error, position, &directive.key, ctx)
                    })?;
                    let pipeline = parse_pipeline(&tokens, ctx).map_err(|error| {
                        remap_branch_parse_error(error, position, &directive.key, ctx)
                    })?;
                    pipelines.push(pipeline);
                }
            }
            Ok(pipelines)
        }
    }
}

/// Evaluates a ternary's condition and returns `true` when the then-branch
/// should be selected.
///
/// The condition source is the original pre-interpolation slice. It is
/// first run through the shared interpolation engine against the seed
/// state, then the resulting text is parsed as a *condition-mode*
/// template expression (the same grammar used by `::block when="..."`
/// and `::file when="..."`) and evaluated. The two-step shape matters
/// when a frontmatter value itself was rendered from a template — e.g.
/// `has_spec: "{{ false }}"` resolves to the string `"false"`, which a
/// direct variable lookup would treat as a truthy non-empty string.
/// Parsing the interpolated text as an expression recovers the literal
/// boolean (`false`) before the truthy check runs. Condition-mode
/// parsing additionally enables infix `&&` / `||` / `!` and comparisons
/// in the condition, matching the spec's "single boolean expression"
/// contract. Parse, interpolation, and evaluation failures surface as
/// [`ShellExpansionError::ParseDirective`] tagged with the frontmatter key.
fn evaluate_ternary_condition(
    condition_source: &str,
    state: &FrontmatterSeedState,
    key: &str,
    ctx: &SourceContext,
) -> Result<bool, ShellExpansionError> {
    let evaluator = Evaluator::new(state);
    let rewrite = interpolate_text(
        condition_source,
        &evaluator,
        ScanMode::Plain,
        true,
        "frontmatter-shell-ternary-condition",
    )
    .map_err(|err| {
        frontmatter_parse_error(
            key,
            ctx,
            format!("Frontmatter shell ternary condition interpolation failed: {err}"),
        )
    })?;

    let expression_text = rewrite.output.trim().to_string();
    if expression_text.is_empty() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            "Frontmatter shell ternary condition resolved to empty text",
        ));
    }

    let parsed = parse_condition(&expression_text).map_err(|err| {
        frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary condition failed to parse: {}",
                err.message
            ),
        )
    })?;

    let value = evaluate(&parsed, state).map_err(|message| {
        frontmatter_parse_error(
            key,
            ctx,
            format!("Frontmatter shell ternary condition must be a boolean expression: {message}"),
        )
    })?;

    Ok(is_truthy(&value))
}

/// Prepares a single branch pipeline through the shared shell allowlist path.
///
/// Builds a synthetic [`ShellDirective`] for the branch's pipeline and runs
/// it through [`prepare_directive`] so the executable-token allowlist policy
/// is enforced. The pipeline's [`ShellPipeline::display_string`] is used as
/// `raw_command` so approval prompts identify the specific branch being
/// authorized rather than the surrounding ternary text.
fn prepare_branch_pipeline(
    pipeline: ShellPipeline,
    raw_command: String,
    candidate: &FrontmatterShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    runtime: &mut PipelineRuntime,
    ctx: &SourceContext,
) -> Result<PreparedShellDirective, ShellExpansionError> {
    let executable = pipeline.actions[0].command.executable.clone();
    let args = pipeline.actions[0].command.args.clone();

    let directive = ShellDirective {
        raw_command,
        executable,
        args,
        span: 0..0,
        indent: String::new(),
        origin: ShellCommandOrigin::Frontmatter {
            key: candidate.key.clone(),
        },
        error_handling: ErrorHandling::default(),
        timeout_override: candidate.timeout_override,
        pipeline: Some(pipeline),
        ctx: ctx.clone(),
    };

    prepare_directive(&directive, options, policy_paths, &mut runtime.shell)
}

/// Prepares one side of a ternary, returning `None` for an empty-string branch.
///
/// `Branch::Empty` short-circuits to no shell invocation and contributes no
/// command to the allowlist-reachable set, so it is skipped entirely.
/// `Branch::Pipeline` is interpolated against the seed state, tokenized into
/// a pipeline, and dispatched through [`prepare_branch_pipeline`] so its
/// commands are validated against the user's allowlist before the ternary
/// condition is evaluated. Per-branch interpolation keeps the branch
/// boundary anchored to the original ternary AST.
#[allow(clippy::too_many_arguments)]
fn prepare_optional_branch(
    branch: &Branch,
    candidate: &FrontmatterShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    runtime: &mut PipelineRuntime,
    ctx: &SourceContext,
    state: &FrontmatterSeedState,
    position: BranchPosition,
) -> Result<Option<Box<PreparedShellDirective>>, ShellExpansionError> {
    match branch {
        Branch::Empty => Ok(None),
        Branch::Pipeline { original_text } => {
            // Anchor the static command-set to the ORIGINAL branch text. Any
            // expansion that introduces a new action or changes an executable
            // would let an interpolation value smuggle past the allowlist.
            let original_pipeline = parse_static_pipeline_shape(original_text, ctx)
                .map_err(|error| remap_branch_parse_error(error, position, &candidate.key, ctx))?;

            let resolved = interpolate_branch_text(original_text, state, position, &candidate.key, ctx)?;
            let tokens = tokenize(&resolved, ctx)
                .map_err(|error| remap_branch_parse_error(error, position, &candidate.key, ctx))?;
            let pipeline = parse_pipeline(&tokens, ctx)
                .map_err(|error| remap_branch_parse_error(error, position, &candidate.key, ctx))?;

            validate_pipeline_shape_preserved(
                &original_pipeline,
                &pipeline,
                Some(position),
                &candidate.key,
                ctx,
            )?;

            let raw_command = pipeline.display_string();
            let prepared = prepare_branch_pipeline(
                pipeline,
                raw_command,
                candidate,
                options,
                policy_paths,
                runtime,
                ctx,
            )?;
            Ok(Some(Box::new(prepared)))
        }
    }
}

/// Interpolates a single branch's pre-interpolation text against the
/// frontmatter seed state, producing the final command text.
///
/// Interpolation runs in [`ScanMode::Plain`] so the entire branch is
/// scanned. Failures are surfaced as branch-tagged
/// [`ShellExpansionError::ParseDirective`] errors.
fn interpolate_branch_text(
    original_text: &str,
    state: &FrontmatterSeedState,
    position: BranchPosition,
    key: &str,
    ctx: &SourceContext,
) -> Result<String, ShellExpansionError> {
    let evaluator = Evaluator::new(state);
    let rewrite = interpolate_text(
        original_text,
        &evaluator,
        ScanMode::Plain,
        true,
        "frontmatter-shell-ternary-branch",
    )
    .map_err(|err| {
        frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary {} interpolation failed: {err}",
                position.name()
            ),
        )
    })?;
    Ok(rewrite.output)
}

fn frontmatter_parse_error(
    key: &str,
    ctx: &SourceContext,
    message: impl Into<String>,
) -> ShellExpansionError {
    ShellExpansionError::ParseDirective {
        ctx: Box::new(ctx.clone()),
        origin: ShellCommandOrigin::Frontmatter {
            key: key.to_string(),
        },
        message: message.into(),
    }
}

fn remap_parse_error(
    error: ShellExpansionError,
    key: &str,
    ctx: &SourceContext,
) -> ShellExpansionError {
    match error {
        ShellExpansionError::ParseDirective { message, .. } => {
            frontmatter_parse_error(key, ctx, message)
        }
        other => other,
    }
}

fn find_unquoted_closing_paren(
    rest: &str,
    key: &str,
    ctx: &SourceContext,
) -> Result<usize, ShellExpansionError> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    // Tracking nested parens lets parenthesized sub-expressions inside the
    // directive body — including the spec's parenthesized condition form
    // `(flag ? a : b)` — coexist with the surrounding `$(...)` without the
    // inner `)` prematurely closing the directive. The ternary splitter
    // already respects paren depth; this mirrors that view of "top-level".
    let mut paren_depth: u32 = 0;

    for (idx, ch) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' if !in_single && !in_double => paren_depth += 1,
            ')' if !in_single && !in_double && paren_depth > 0 => paren_depth -= 1,
            ')' if !in_single && !in_double => return Ok(idx),
            _ => {}
        }
    }

    Err(frontmatter_parse_error(
        key,
        ctx,
        "Missing closing ')' in frontmatter shell expression",
    ))
}

fn first_token_portion(input: &str) -> &str {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return trimmed;
    }

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (idx, ch) in trimmed.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => return &trimmed[..idx],
            _ => {}
        }
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_ctx() -> SourceContext {
        SourceContext::new(
            std::path::PathBuf::from("/test"),
            std::path::PathBuf::from("test"),
            String::new(),
        )
    }

    fn parse_shell_value(
        value: &str,
        key: &str,
        original_value: Option<&str>,
    ) -> Result<Option<FrontmatterShellDirective>, ShellExpansionError> {
        super::parse_shell_value(value, key, original_value, &test_ctx())
    }

    #[allow(dead_code)]
    fn scan_frontmatter(
        frontmatter: &Frontmatter,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<Vec<FrontmatterShellDirective>, ShellExpansionError> {
        super::scan_frontmatter(frontmatter, pre_interpolation_snapshot, &test_ctx())
    }

    #[allow(dead_code)]
    fn execute_frontmatter_shell_expansion(
        frontmatter: &mut Frontmatter,
        options: &ComposeOptions,
        runtime: &mut PipelineRuntime,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> MarkdownResult<FrontmatterShellExpansionReport> {
        super::execute_frontmatter_shell_expansion(
            frontmatter,
            options,
            runtime,
            pre_interpolation_snapshot,
            &test_ctx(),
        )
    }

    #[test]
    fn detects_simple_shell_expression() {
        let result = parse_shell_value("$(echo hello)", "key", None);
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
        assert_eq!(directive.raw_command, "echo hello");
        assert!(directive.timeout_override.is_none());
    }

    #[test]
    fn detects_expression_with_timeout() {
        let result = parse_shell_value("$(pwd)::timeout:3", "key", None);
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "pwd");
        assert_eq!(directive.args.len(), 0);
        assert_eq!(
            directive.timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn ignores_non_shell_string() {
        let result = parse_shell_value("plain text", "key", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn ignores_partial_match_no_closing() {
        let result = parse_shell_value("$(echo hello", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn ignores_embedded_expression() {
        let result = parse_shell_value("prefix $(cmd) suffix", "key", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn rejects_zero_timeout() {
        let result = parse_shell_value("$(echo)::timeout:0", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_integer_timeout() {
        let result = parse_shell_value("$(echo)::timeout:abc", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_interpolated_executable() {
        let original = "$({{cmd}} arg)";
        let resolved = "$(ls arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_interpolated_executable_after_or_operator() {
        let original = "$(false || {{cmd}} arg)";
        let resolved = "$(false || echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn rejects_interpolated_executable_after_and_operator() {
        let original = "$(true && {{cmd}} arg)";
        let resolved = "$(true && echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn rejects_interpolated_executable_in_third_chain_segment() {
        let original = "$(true && false || {{cmd}} arg)";
        let resolved = "$(true && false || echo arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_err());
    }

    #[test]
    fn accepts_interpolated_argument_after_chain_operator() {
        let original = "$(false || echo {{file}})";
        let resolved = "$(false || echo README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
    }

    #[test]
    fn ignores_chain_operators_inside_quotes() {
        // `||` inside single quotes is literal, not a chain operator,
        // so the interpolation here is in argument position, not executable.
        let original = "$(echo 'a || {{cmd}}')";
        let resolved = "$(echo 'a || hello')";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
    }

    #[test]
    fn accepts_interpolated_argument() {
        let original = "$(dirname {{file}})";
        let resolved = "$(dirname README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "dirname");
        assert_eq!(directive.args, vec!["README.md"]);
    }

    #[test]
    fn accepts_no_interpolation_at_all() {
        let original = "$(echo hello)";
        let result = parse_shell_value(original, "key", Some(original));
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
    }

    #[test]
    fn accepts_closing_paren_inside_quoted_argument() {
        let result = parse_shell_value("$(printf ')')", "key", None).unwrap();
        let directive = result.unwrap();
        assert_eq!(directive.executable, "printf");
        assert_eq!(directive.args, vec![")"]);
    }

    #[test]
    fn scan_finds_shell_in_top_level_strings() {
        let mut fm = Frontmatter::new();
        fm.insert("cmd1", json!("$(echo hello)")).unwrap();
        fm.insert("plain", json!("not a shell command")).unwrap();
        fm.insert("cmd2", json!("$(pwd)")).unwrap();
        fm.insert("number", json!(42)).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 2);
        assert!(directives.iter().any(|d| d.key == "cmd1"));
        assert!(directives.iter().any(|d| d.key == "cmd2"));
    }

    #[test]
    fn scan_skips_nested_objects() {
        let mut fm = Frontmatter::new();
        fm.insert("outer", json!({"inner": "$(echo nested)"}))
            .unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_skips_arrays() {
        let mut fm = Frontmatter::new();
        fm.insert("arr", json!(["$(echo one)", "$(echo two)"]))
            .unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_errors_on_malformed_shell_expression() {
        let mut fm = Frontmatter::new();
        fm.insert("bad", json!("$(echo hi)::timeout:0")).unwrap();

        let err = scan_frontmatter(&fm, None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { origin, .. } => {
                assert_eq!(
                    origin,
                    ShellCommandOrigin::Frontmatter {
                        key: "bad".to_string()
                    }
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    fn unwrap_ternary(
        directive: &FrontmatterShellDirective,
    ) -> (&str, &Branch, &Branch) {
        match &directive.ast {
            FrontmatterShellAst::Ternary {
                condition_source,
                then_branch,
                else_branch,
            } => (condition_source.as_str(), then_branch, else_branch),
            FrontmatterShellAst::Pipeline(_) => {
                panic!("expected Ternary AST, got Pipeline")
            }
        }
    }

    #[test]
    fn split_top_level_ternary_basic() {
        let (cond, then_s, else_s) =
            super::split_top_level_ternary("{{has_spec}} ? basename '{{spec}}' : ''")
                .expect("ternary should split");
        assert_eq!(cond.trim(), "{{has_spec}}");
        assert_eq!(then_s.trim(), "basename '{{spec}}'");
        assert_eq!(else_s.trim(), "''");
    }

    #[test]
    fn split_top_level_ternary_quotes_protect_punctuation() {
        // `?` and `:` inside single/double quotes are not top-level.
        assert!(super::split_top_level_ternary("echo 'is it?'").is_none());
        assert!(super::split_top_level_ternary("echo \"a : b\"").is_none());
    }

    #[test]
    fn split_top_level_ternary_parens_protect_punctuation() {
        // Parenthesized sub-expression in the condition is masked.
        let (cond, then_s, else_s) =
            super::split_top_level_ternary("(a ? b) ? then_cmd : else_cmd")
                .expect("outer ternary should split");
        assert_eq!(cond.trim(), "(a ? b)");
        assert_eq!(then_s.trim(), "then_cmd");
        assert_eq!(else_s.trim(), "else_cmd");
    }

    #[test]
    fn split_top_level_ternary_question_without_colon_returns_none() {
        assert!(super::split_top_level_ternary("a ? b").is_none());
    }

    #[test]
    fn parses_basic_ternary() {
        let original = "$({{has_spec}} ? basename '{{spec}}' : '')";
        let resolved = "$(true ? basename '/tmp/spec.md' : '')";
        let directive = parse_shell_value(resolved, "spec_file", Some(original))
            .expect("parse should succeed")
            .expect("directive should be returned");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "{{has_spec}}");
        // Branches now carry the ORIGINAL slice; resolved text is produced
        // by per-branch interpolation at execute time.
        match then_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "basename '{{spec}}'");
            }
            Branch::Empty => panic!("expected then-branch pipeline"),
        }
        assert!(matches!(else_b, Branch::Empty));
        assert!(directive.pipeline.is_none());
    }

    #[test]
    fn parses_ternary_with_both_pipeline_branches() {
        let inner = "$(cond ? echo yes : echo no)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "cond");
        match (then_b, else_b) {
            (
                Branch::Pipeline { original_text: then_text },
                Branch::Pipeline { original_text: else_text },
            ) => {
                assert_eq!(then_text.trim(), "echo yes");
                assert_eq!(else_text.trim(), "echo no");
            }
            _ => panic!("expected both branches to be pipelines"),
        }
    }

    #[test]
    fn parses_ternary_with_double_quoted_empty_branch() {
        let inner = r#"$(cond ? echo yes : "")"#;
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_cond, _then_b, else_b) = unwrap_ternary(&directive);
        assert!(matches!(else_b, Branch::Empty));
    }

    #[test]
    fn ternary_question_without_colon_errors() {
        let inner = "$({{cond}} ? echo yes)";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("missing ':'"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_rejects_interpolated_executable_in_then_branch() {
        let original = "$(cond ? {{cmd}} arg : '')";
        let resolved = "$(cond ? echo arg : '')";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains("then-branch"),
                    "expected branch label in message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_rejects_interpolated_executable_in_else_branch() {
        let original = "$(cond ? echo yes : {{cmd}} arg)";
        let resolved = "$(cond ? echo yes : echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains("else-branch"),
                    "expected branch label in message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_allows_interpolated_argument_in_branch() {
        let original = "$(cond ? basename {{file}} : '')";
        let resolved = "$(cond ? basename README.md : '')";
        let directive = parse_shell_value(resolved, "key", Some(original))
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, _) = unwrap_ternary(&directive);
        match then_b {
            Branch::Pipeline { original_text } => {
                // Branch retains the original {{file}} placeholder; expansion
                // happens at execute time via per-branch interpolation.
                assert_eq!(original_text.trim(), "basename {{file}}");
            }
            Branch::Empty => panic!("expected pipeline branch"),
        }
    }

    #[test]
    fn ternary_empty_condition_errors() {
        let inner = "$( ? echo yes : '')";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("condition"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_branch_command_pipeline_with_chain_operators() {
        let inner = "$(cond ? echo a && echo b : echo c)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, _) = unwrap_ternary(&directive);
        match then_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo a && echo b");
            }
            Branch::Empty => panic!("expected pipeline branch"),
        }
    }

    #[test]
    fn ternary_quote_protected_punctuation_is_not_split() {
        // The `?` is inside single quotes — the directive is a plain pipeline.
        let inner = "$(echo 'is it?')";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        match directive.ast {
            FrontmatterShellAst::Pipeline(_) => {}
            FrontmatterShellAst::Ternary { .. } => {
                panic!("expected Pipeline AST, got Ternary")
            }
        }
    }

    #[test]
    fn ternary_rejects_nested_in_then_branch() {
        // Review finding 3: a second top-level `?` (here turning the then-
        // branch into another ternary `b ? echo two : echo three`) must be
        // refused at parse time rather than silently tokenized.
        let inner = "$(a ? b ? echo two : echo three : echo c)";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("nested ternaries are not supported")
                        && message.contains("additional separator-style '?'")
                        && message.contains("then-branch"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_rejects_nested_in_else_branch() {
        // Review finding 3: a second top-level `?` in the else-branch is
        // also rejected.
        let inner = "$(a ? echo one : b ? echo two : echo three)";
        let err = parse_shell_value(inner, "key", None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("nested ternaries are not supported")
                        && message.contains("additional separator-style '?'")
                        && message.contains("else-branch"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn ternary_accepts_bare_colon_in_else_branch_argument() {
        // A bare top-level `:` in a branch is valid pipeline content —
        // URLs and key:value arguments commonly contain it — so it must
        // not be rejected as a nested ternary. After the outer split, the
        // else-branch text `echo two : echo three` parses as a single
        // command with three arguments.
        let inner = "$(a ? echo one : echo two : echo three)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, _, else_b) = unwrap_ternary(&directive);
        match else_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo two : echo three");
            }
            Branch::Empty => panic!("expected pipeline else-branch"),
        }
    }

    #[test]
    fn ternary_accepts_url_with_colon_in_then_branch() {
        // Review finding 2: `:` inside a URL argument is normal pipeline
        // content and must round-trip through both branches.
        let inner = "$(flag ? echo http://example.com : echo none)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, then_b, _) = unwrap_ternary(&directive);
        match then_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo http://example.com");
            }
            Branch::Empty => panic!("expected pipeline then-branch"),
        }
    }

    #[test]
    fn ternary_accepts_url_with_colon_in_else_branch() {
        let inner = "$(flag ? echo none : echo http://example.com)";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        let (_, _, else_b) = unwrap_ternary(&directive);
        match else_b {
            Branch::Pipeline { original_text } => {
                assert_eq!(original_text.trim(), "echo http://example.com");
            }
            Branch::Empty => panic!("expected pipeline else-branch"),
        }
    }

    #[test]
    fn ternary_branch_boundaries_pin_to_original_snapshot() {
        // Review finding 1: when an interpolated condition introduces
        // top-level `?` / `:` punctuation, the resolved inner naively
        // re-split would shift branch boundaries and let condition text
        // bleed into the then-branch executable. Anchoring the AST to the
        // original snapshot keeps the then-branch text statically equal to
        // what the author wrote.
        let original = "$({{cond}} ? basename README.md : '')";
        // Suppose `cond` was rendered to the literal text `true ? date : false`
        // by an earlier interpolation pass.
        let resolved = "$(true ? date : false ? basename README.md : '')";
        let directive = parse_shell_value(resolved, "key", Some(original))
            .expect("parse should succeed")
            .expect("directive should be returned");
        let (cond, then_b, else_b) = unwrap_ternary(&directive);
        assert_eq!(cond, "{{cond}}");
        match then_b {
            Branch::Pipeline { original_text } => {
                // Critically, the then-branch text is the ORIGINAL slice
                // (`basename README.md`), NOT the resolved slice (`date`)
                // that a naive resplit would have produced.
                assert_eq!(original_text.trim(), "basename README.md");
            }
            Branch::Empty => panic!("expected pipeline then-branch"),
        }
        assert!(matches!(else_b, Branch::Empty));
    }

    #[test]
    fn ternary_separator_requires_whitespace_padding() {
        // Review-3 medium finding: the separator detection contract requires
        // whitespace padding on both sides of `?` and `:`. An unpadded `?`
        // like `flag? echo yes : ''` is parsed as part of the executable
        // token (`flag?`) — not as a ternary separator. This lock-in test
        // documents that the implementation matches the spec's
        // whitespace-padded rule rather than a raw "any top-level `?`" rule.
        let inner = "$(flag? echo yes : '')";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        match directive.ast {
            FrontmatterShellAst::Pipeline(ref pipeline) => {
                assert_eq!(pipeline.actions[0].command.executable, "flag?");
            }
            FrontmatterShellAst::Ternary { .. } => {
                panic!("expected Pipeline AST, got Ternary — separator must require whitespace padding")
            }
        }
    }

    #[test]
    fn ternary_with_timeout_preserves_timeout() {
        let inner = "$(cond ? echo yes : '')::timeout:5";
        let directive = parse_shell_value(inner, "key", None)
            .unwrap()
            .expect("directive should parse");
        assert!(matches!(directive.ast, FrontmatterShellAst::Ternary { .. }));
        assert_eq!(
            directive.timeout_override,
            Some(std::time::Duration::from_secs(5))
        );
    }
}

#[cfg(test)]
mod execution_tests {
    use super::*;
    use crate::markdown::compose::cache::CacheAccessMode;
    use crate::markdown::compose::shell_expansion::types::{
        PipelineRuntime, ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
        ShellExpansionError, ShellExpansionOptions,
    };
    use crate::markdown::compose::types::ComposeOptions;
    use crate::markdown::frontmatter::Frontmatter;
    use serde_json::json;
    use serial_test::serial;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    fn fm_from_json(data: serde_json::Value) -> Frontmatter {
        let map: crate::markdown::types::FrontmatterMap = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => Default::default(),
        };
        Frontmatter::from_map(map)
    }

    fn test_ctx() -> SourceContext {
        SourceContext::new(PathBuf::from("/test"), PathBuf::from("test"), String::new())
    }

    fn execute_frontmatter_shell_expansion(
        frontmatter: &mut Frontmatter,
        options: &ComposeOptions,
        runtime: &mut PipelineRuntime,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> crate::markdown::types::MarkdownResult<FrontmatterShellExpansionReport> {
        super::execute_frontmatter_shell_expansion(
            frontmatter,
            options,
            runtime,
            pre_interpolation_snapshot,
            &test_ctx(),
        )
    }

    struct MockApproval;
    impl ShellApprovalHandler for MockApproval {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    fn make_runtime() -> PipelineRuntime {
        PipelineRuntime::new(16, CacheAccessMode::Off, None)
    }

    fn find_python() -> Option<PathBuf> {
        ["python3", "python"]
            .into_iter()
            .find_map(|candidate| which::which(candidate).ok())
            .filter(|path| !path.as_os_str().is_empty())
    }

    #[test]
    fn execute_replaces_frontmatter_value_with_output() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "greeting": "$(echo hello world)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("greeting"), Some(&json!("hello world")));
    }

    #[test]
    fn execute_trims_output_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(echo '  padded  ')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("padded")));
    }

    #[test]
    fn execute_skips_non_shell_values() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42,
            "cmd": "$(echo result)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("title"), Some(&json!("Hello")));
        assert_eq!(fm.as_map().get("count"), Some(&json!(42)));
        assert_eq!(fm.as_map().get("cmd"), Some(&json!("result")));
    }

    #[test]
    fn execute_no_candidates_returns_empty_report() {
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42
        }));
        let options = ComposeOptions::new();
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 0);
        assert_eq!(report.approvals_used, 0);
    }

    #[test]
    fn execute_frontmatter_uses_stdout_only() {
        let Some(python) = find_python() else {
            return;
        };

        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": format!(
                "$({} -c \"import sys; sys.stdout.write('out'); sys.stderr.write('warn')\")",
                python.display()
            )
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("out")));
    }

    #[test]
    fn execute_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(sleep 1)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_millis(100),
            timeout_behavior:
                super::super::shell_expansion::types::ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("val"), Some(&json!("")));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    fn execute_pipeline_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(sleep 1 && echo after)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_millis(100),
            timeout_behavior:
                super::super::shell_expansion::types::ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("val"), Some(&json!("after")));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    #[serial]
    fn execute_frontmatter_commands_concurrently() {
        let Some(python) = find_python() else {
            return;
        };

        // Each command records a wall-clock start timestamp, then sleeps briefly.
        // Concurrent execution => both commands start within startup jitter of
        // each other. Serial execution => the second start is delayed by the
        // first command's full sleep. Comparing start offsets (rather than total
        // wall-clock time) keeps the test fast and immune to interpreter startup
        // variance.
        let temp_dir = TempDir::new().unwrap();
        let stamp_cmd = || {
            format!(
                "$({} -c \"import time; print(time.time(), end=''); time.sleep(0.3)\")",
                python.display()
            )
        };
        let mut fm = fm_from_json(json!({
            "one": stamp_cmd(),
            "two": stamp_cmd()
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_secs(2),
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();

        assert_eq!(report.replacements, 2);
        let start_time = |key: &str| -> f64 {
            fm.as_map()
                .get(key)
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .parse()
                .unwrap_or_else(|_| panic!("expected numeric start timestamp for `{key}`"))
        };
        let start_skew = (start_time("one") - start_time("two")).abs();
        assert!(
            start_skew < 0.2,
            "expected commands to start concurrently (skew < 0.2s), got {start_skew}s \
             — commands appear to have run serially"
        );
    }

    #[test]
    fn ternary_truthy_condition_runs_then_branch() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_falsy_condition_runs_else_branch() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("no")));
    }

    #[test]
    fn ternary_empty_branch_short_circuits_to_empty_string() {
        // Phase 4: the unselected then-branch must still be allowlisted, so
        // pre-approve `echo yes` and pin policy_root to an isolated dir to
        // avoid picking up the repo whitelist. Selecting the empty branch then
        // short-circuits to "" without executing any shell command.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo yes".to_string());

        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo yes : '')"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(report.approvals_used, 0);
        assert_eq!(fm.as_map().get("out"), Some(&json!("")));
    }

    #[test]
    fn ternary_then_branch_must_be_allowlisted_even_when_else_selected() {
        // Phase 4.3: an unallowlisted command in the then-branch fails the
        // entire directive even when the else-branch (Empty) is selected at
        // runtime.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? not_allowed_cmd : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let err = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
        {
            Ok(_) => panic!("expected directive to fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("approval"),
            "expected approval-required error for unselected branch, got: {msg}"
        );
    }

    #[test]
    fn ternary_else_branch_must_be_allowlisted_even_when_then_selected() {
        // Phase 4.3: an unallowlisted command in the else-branch fails the
        // entire directive even when the then-branch is selected at runtime.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? '' : not_allowed_cmd)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let err = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
        {
            Ok(_) => panic!("expected directive to fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("approval"),
            "expected approval-required error for unselected branch, got: {msg}"
        );
    }

    #[test]
    fn ternary_partial_preapproval_fails_with_unselected_branch_command() {
        // Phase 4.3: pre-approving only the then-branch's command is not
        // enough — the else-branch's command must also be in the reachable
        // pre-approved set even though it would not have been selected.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo yes".to_string());
        // `echo no` intentionally omitted.

        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let err = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
        {
            Ok(_) => panic!("expected directive to fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("not pre-approved") && msg.contains("echo no"),
            "expected pre-approval rejection naming `echo no`, got: {msg}"
        );
    }

    #[test]
    fn ternary_both_branches_allowlisted_succeeds() {
        // Phase 4.2: when every reachable command is allowlisted, the
        // directive succeeds and the selected branch's output is used.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo yes".to_string());
        approved.insert("echo no".to_string());

        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_with_brace_wrapped_condition_resolves_via_seed_state() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_value": true,
            "out": "$({{has_value}} ? echo present : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("present")));
    }

    #[test]
    fn ternary_with_stringified_false_condition_selects_else_branch() {
        // Review finding 2: when a frontmatter value is itself rendered from
        // a template (`has_spec: "{{ false }}"`), the post-interpolation
        // value is the string `"false"`. The condition `{{has_spec}}` must
        // be recognized as boolean-false — not as a truthy non-empty string
        // — and select the else-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": "false",
            "spec_file": "$({{has_spec}} ? echo present : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("spec_file"), Some(&json!("")));
    }

    #[test]
    fn ternary_with_stringified_true_condition_selects_then_branch() {
        // Symmetric to the false case: a stringified `"true"` condition
        // selects the then-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": "true",
            "spec_file": "$({{has_spec}} ? echo present : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("spec_file"), Some(&json!("present")));
    }

    #[test]
    fn ternary_condition_interpolation_cannot_bleed_into_then_branch_executable() {
        // Review finding 1: a condition variable whose value contains
        // top-level `?` / `:` punctuation must not shift the then-branch
        // boundary or let condition text become a branch executable. With
        // the original snapshot anchoring the branch slices, the
        // then-branch runs the static `basename README.md` (or doesn't run
        // at all, depending on how the rewritten condition evaluates), but
        // it never tokenizes anything from the condition text.
        let temp_dir = TempDir::new().unwrap();
        // `cond` is itself the literal string `"true ? date : false"`. Once
        // it lands in the directive value, a naive split-of-resolved would
        // pick `date` as the then-branch executable.
        let mut fm = fm_from_json(json!({
            "cond": "true ? date : false",
            "out": "$(true ? date : false ? basename README.md : '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert(
            "out".to_string(),
            "$({{cond}} ? basename README.md : '')".to_string(),
        );

        let mut approved = std::collections::HashSet::new();
        approved.insert("basename README.md".to_string());

        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
                .unwrap();
        assert_eq!(report.replacements, 1);
        // Result is either `README` (basename ran from the then-branch) or
        // `""` (else-branch selected); either way, `date` from the
        // condition text must not be the executable that ran.
        let out = fm
            .as_map()
            .get("out")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        assert!(
            out == "README" || out.is_empty(),
            "expected branch text to remain anchored to the original snapshot; got {out:?}"
        );
    }

    #[test]
    fn ternary_branch_arg_interpolation_cannot_inject_new_action() {
        // Review 2 finding 1: an interpolation in argument position must
        // not be able to introduce `&& date` (or any other chain
        // continuation) after the executable-interpolation check has
        // already accepted the branch. Even though the then-branch's
        // static executable is `basename`, a malicious `spec` value with
        // an embedded `&& date` would extend the pipeline to two
        // additional actions. The shape-preservation guard must catch the
        // action-count drift and reject the directive.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": true,
            // Crafted value: closes the outer single quote, chains `date`,
            // then reopens a quote so the surrounding directive still
            // tokenizes cleanly.
            "spec": "README.md' && date && echo '",
            "out": "$({{has_spec}} ? basename '{{spec}}' : '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert(
            "out".to_string(),
            "$({{has_spec}} ? basename '{{spec}}' : '')".to_string(),
        );

        let mut approved = std::collections::HashSet::new();
        approved.insert("basename".to_string());
        approved.insert("date".to_string());
        approved.insert("echo".to_string());

        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let err =
            match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
            {
                Ok(_) => panic!("expected directive to fail with a shape-preservation error"),
                Err(err) => err,
            };
        let msg = err.to_string();
        assert!(
            msg.contains("changed from 1 action") || msg.contains("introduce new chain operators"),
            "expected pipeline-shape error mentioning action count, got: {msg}"
        );
    }

    #[test]
    fn ternary_branch_quoted_interpolation_breakout_is_rejected() {
        // Review 2 finding 1: a `{{spec}}` interpolation inside double
        // quotes must not be able to introduce a closing `"` followed by
        // additional commands. Depending on how the resolved text lands,
        // the directive is rejected either by the shape guard (action
        // count drift) or by the tokenizer (unterminated quote). Both
        // refuse to execute the smuggled command — that is what matters.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": true,
            "spec": "README.md\" && evil",
            "out": "$({{has_spec}} ? basename \"{{spec}}\" : '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert(
            "out".to_string(),
            "$({{has_spec}} ? basename \"{{spec}}\" : '')".to_string(),
        );

        let mut approved = std::collections::HashSet::new();
        approved.insert("basename".to_string());
        approved.insert("evil".to_string());

        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let err =
            match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
            {
                Ok(_) => panic!("expected quote-breakout to fail"),
                Err(err) => err,
            };
        let msg = err.to_string();
        assert!(
            msg.contains("changed from 1 action")
                || msg.contains("introduce new chain operators")
                || msg.contains("Unterminated double quote")
                || msg.contains("Unterminated single quote"),
            "expected pipeline-shape or tokenizer rejection, got: {msg}"
        );
    }

    #[test]
    fn non_ternary_arg_interpolation_cannot_inject_new_action() {
        // The same static-command-invariant applies to bare (non-ternary)
        // pipelines: an interpolated argument value may not introduce
        // additional `&&` / `||` actions even when the executable token is
        // statically `basename`. In a real compose run, frontmatter
        // interpolation has already rewritten `{{spec}}` before shell
        // expansion runs — so `out`'s value here is the RESOLVED text and
        // the snapshot carries the ORIGINAL with the placeholder intact.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "spec": "README.md' && date && echo '",
            "out": "$(basename 'README.md' && date && echo '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert("out".to_string(), "$(basename '{{spec}}')".to_string());

        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let err =
            match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
            {
                Ok(_) => panic!("expected non-ternary directive to fail"),
                Err(err) => err,
            };
        let msg = err.to_string();
        assert!(
            msg.contains("changed from 1 action") || msg.contains("introduce new chain operators"),
            "expected pipeline-shape error mentioning action count, got: {msg}"
        );
    }

    #[test]
    fn ternary_condition_supports_infix_and() {
        // Review-3 high finding: condition-mode infix `&&` must lower to
        // `and(a, b)` and select the then-branch when both operands are
        // truthy. The non-condition `parse` entrypoint refuses bare `&&`
        // outside `{{ }}` interpolation; only `parse_condition` accepts it.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "a": true,
            "b": true,
            "out": "$(a && b ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_condition_infix_and_selects_else_when_one_falsy() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "a": true,
            "b": false,
            "out": "$(a && b ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("out"), Some(&json!("no")));
        assert_eq!(report.replacements, 1);
    }

    #[test]
    fn ternary_condition_supports_infix_or() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "a": false,
            "b": true,
            "out": "$(a || b ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_condition_supports_negation() {
        // Condition-mode parses `!flag` as `not(flag)`.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(!flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_condition_supports_comparison() {
        // The spec's "single boolean expression" contract includes
        // comparisons. `count == 3` must evaluate to true and select the
        // then-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "count": 3,
            "out": "$(count == 3 ? echo equal : echo unequal)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("equal")));
    }

    #[test]
    fn ternary_condition_supports_nested_expression_ternary() {
        // Condition-mode supports `?:` inside the condition expression
        // itself. Here `(flag ? true : false)` evaluates to `true`, which
        // selects the outer then-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$((flag ? true : false) ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_branch_url_with_colon_argument_executes() {
        // Review 2 finding 2: a bare `:` inside a branch argument (here
        // part of a URL) must not be misclassified as a nested ternary.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo http://example.com".to_string());
        approved.insert("echo none".to_string());

        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo http://example.com : echo none)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("http://example.com")));
    }
}
