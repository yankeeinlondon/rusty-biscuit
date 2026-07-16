//! Frontmatter shell expansion parsing and validation.
//!
//! Scans top-level frontmatter string values for `$(cmd)` shell expressions,
//! parses them, validates the executable-token interpolation rule, and returns
//! structured directives ready for execution.
//!
//! ## `$()` Token Resolution (the §2 ladder)
//!
//! A `$( … )` value is a shell expansion, but a token in **executed position**
//! (a non-ternary directive body, or a ternary branch) resolves by a precedence
//! ladder rather than always being a shell command:
//!
//! 1. quoted literal → string,
//! 2. numeric literal → number,
//! 3. `true` / `false` → boolean (never a command or a property),
//! 4. `name(...)` (trailing parens) → a **safe** expression function — it spawns
//!    no process and is excluded from shell approval discovery,
//! 5. path-bearing token (`/usr/bin/x`, `./x`) → executable, never a property,
//! 6. bare name found on `PATH` → executable (a shell command), otherwise a
//!    frontmatter property, and `null` when that property is absent.
//!
//! A `$()` is valid only if at least one **executed-position** token is a real
//! shell command (for a ternary, at least one branch; for a non-ternary, the
//! directive itself). The condition of a ternary is always expression content
//! and never counts. A `$()` that resolves to no shell command — e.g.
//! `"$( file_exists('x') ? 'a' : 'b' )"` — is a user error and is rejected with a
//! diagnostic suggesting `{{ … }}` interpolation instead.

use super::expression::{doc_namespace, evaluate, is_truthy, parse, parse_condition, scalar_string};
use super::frontmatter_interpolation::FrontmatterSeedState;
use super::interpolation::{Evaluator, ScanMode, interpolate_text};
use super::shell_expansion::store::resolve_policy_paths;
use super::shell_expansion::tokenize::{parse_pipeline, tokenize};
use super::shell_expansion::types::{
    ChainOperator, ErrorHandling, PipelineRuntime, ShellCommandOrigin, ShellDirective,
    ShellExpansionError, ShellPipeline, ShellPolicyPaths, frontmatter_key_line,
};
use super::shell_expansion::{
    PreparedShellDirective, ShellRuntimeSnapshot, execute_prepared_directive, prepare_directive,
};
use super::{ComposeOptions, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::span::{SourceSpan, Spanned};
use crate::markdown::types::MarkdownResult;
use biscuit_terminal::errors::SourceContext;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

/// A recognized trailing suffix on a frontmatter `$(...)` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterShellSuffix {
    /// `::timeout:N` — a per-directive timeout in whole seconds.
    Timeout(u64),
    /// `::no-cache` — bypass the per-compose command cache.
    NoCache,
}

/// One chain action of a spanned frontmatter `$(...)` pipeline (the segments
/// between top-level `&&` / `||` operators).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterShellAction {
    /// Byte span of the action's text (trimmed).
    pub span: SourceSpan,
    /// Byte spans of the action's whitespace-delimited tokens (quote/paren
    /// aware). The first token is the executable position.
    pub tokens: Vec<SourceSpan>,
}

/// A spanned frontmatter `$(...)` command pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterShellPipeline {
    /// Byte span of the whole inner command (excluding delimiters/suffixes).
    pub span: SourceSpan,
    /// One entry per chain action, in source order.
    pub actions: Vec<FrontmatterShellAction>,
}

/// A spanned frontmatter `$(...)` ternary `COND ? THEN : ELSE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterShellTernary {
    /// Byte span of the condition expression (trimmed).
    pub condition_span: SourceSpan,
    /// Byte span of the then-branch (trimmed).
    pub then_span: SourceSpan,
    /// Byte span of the else-branch (trimmed).
    pub else_span: SourceSpan,
}

/// The parsed body of a spanned frontmatter `$(...)` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterShellBody {
    /// A bare command pipeline.
    Pipeline(FrontmatterShellPipeline),
    /// A conditional `COND ? THEN : ELSE`.
    Ternary(FrontmatterShellTernary),
}

/// A read-only, span-carrying mirror of a frontmatter `$(...)` shell value.
///
/// Produced by [`parse_frontmatter_shell_value_spanned`]. Every span is a byte
/// offset into the original value string. This is a *description*, not a runnable
/// directive: it exposes no execution surface, so a language server can offer
/// hover, folding, and command-token navigation over a `$(...)` value without any
/// risk of running it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterShellValue {
    /// Byte span of the whole `$(...)` value including delimiters and suffixes.
    pub span: SourceSpan,
    /// Byte span of the opening `$(`.
    pub open_span: SourceSpan,
    /// Byte span of the closing `)`.
    pub close_span: SourceSpan,
    /// Byte span of the inner command text between `$(` and `)`.
    pub inner_span: SourceSpan,
    /// Recognized trailing suffixes with their spans, in source order.
    pub suffixes: Vec<Spanned<FrontmatterShellSuffix>>,
    /// The parsed body — a pipeline or a ternary.
    pub body: FrontmatterShellBody,
}

/// Parses a frontmatter string value into a read-only, span-carrying
/// [`FrontmatterShellValue`], or `None` when the value is not a whole-value
/// `$( … )` shell expression.
///
/// The whole-value `$( … )` rule is defined on the trimmed value (so a padded
/// `"  $(echo ok)  "` parses identically), and all reported spans are byte
/// offsets into the *original* `value` — leading whitespace is accounted for.
/// The parse is lenient and never executes: a value that opens `$(` but never
/// closes it, or carries an unrecognized suffix, still yields the spans that
/// could be recovered (unrecognized trailing content simply ends suffix
/// scanning). It shares the executor's [`unquoted_closing_paren_offset`],
/// [`split_top_level_ternary`], and [`split_at_chain_operators`] helpers so its
/// view of structure never drifts from the run-time path.
pub fn parse_frontmatter_shell_value_spanned(value: &str) -> Option<FrontmatterShellValue> {
    let base = value.len() - value.trim_start().len();
    let trimmed = value.trim();

    let rest = trimmed.strip_prefix("$(")?;
    let close_rel = unquoted_closing_paren_offset(rest)?;

    // Offsets are computed relative to `trimmed`, then shifted by `base` to land
    // in the original `value`.
    let inner_start = base + 2;
    let inner_end = inner_start + close_rel;
    let close_pos = inner_end; // the `)` byte
    let open_span = base..base + 2;
    let close_span = close_pos..close_pos + 1;
    let inner_span = inner_start..inner_end;
    let inner = &rest[..close_rel];

    let after_close = &rest[close_rel + 1..];
    let suffix_base = close_pos + 1;
    let suffixes = scan_suffix_spans(after_close, suffix_base);

    let body = match split_top_level_ternary(inner) {
        Some((cond, then_b, else_b)) => FrontmatterShellBody::Ternary(FrontmatterShellTernary {
            condition_span: trimmed_subspan(inner, inner_start, cond),
            then_span: trimmed_subspan(inner, inner_start, then_b),
            else_span: trimmed_subspan(inner, inner_start, else_b),
        }),
        None => {
            let actions = split_at_chain_operators(inner)
                .into_iter()
                .map(|segment| {
                    let seg_offset = inner_start + subslice_offset(inner, segment);
                    let span = trimmed_subspan(inner, inner_start, segment);
                    let tokens = shell_token_spans(segment)
                        .into_iter()
                        .map(|r| seg_offset + r.start..seg_offset + r.end)
                        .collect();
                    FrontmatterShellAction { span, tokens }
                })
                .collect();
            FrontmatterShellBody::Pipeline(FrontmatterShellPipeline {
                span: inner_span.clone(),
                actions,
            })
        }
    };

    Some(FrontmatterShellValue {
        span: base..base + trimmed.len(),
        open_span,
        close_span,
        inner_span,
        suffixes,
        body,
    })
}

/// Byte offset of `child` within `parent`; `child` must be a subslice of
/// `parent` (as produced by the shared split helpers).
fn subslice_offset(parent: &str, child: &str) -> usize {
    child.as_ptr() as usize - parent.as_ptr() as usize
}

/// Span of `child` (trimmed) within `parent`, shifted to absolute coordinates
/// by adding `parent_base`. `child` must be a subslice of `parent`.
fn trimmed_subspan(parent: &str, parent_base: usize, child: &str) -> SourceSpan {
    let offset = subslice_offset(parent, child);
    let leading = child.len() - child.trim_start().len();
    let trimmed = child.trim();
    let start = parent_base + offset + leading;
    start..start + trimmed.len()
}

/// Scans the text after the closing `)` for `::timeout:N` / `::no-cache`
/// suffixes, returning each recognized suffix with its span (base-shifted).
///
/// Lenient: scanning stops at the first unrecognized token rather than erroring,
/// because this is a passive analyzer.
fn scan_suffix_spans(after_close: &str, base: usize) -> Vec<Spanned<FrontmatterShellSuffix>> {
    let mut out = Vec::new();
    let mut suffix = after_close;
    let mut offset = base;

    while !suffix.is_empty() {
        if let Some(rest) = suffix.strip_prefix("::no-cache") {
            let len = suffix.len() - rest.len();
            out.push(Spanned::new(FrontmatterShellSuffix::NoCache, offset..offset + len));
            offset += len;
            suffix = rest;
        } else if let Some(rest) = suffix.strip_prefix("::timeout:") {
            let digits_end = rest.find("::").unwrap_or(rest.len());
            let digits = &rest[..digits_end];
            match digits.parse::<u64>() {
                Ok(value) => {
                    let len = suffix.len() - rest.len() + digits_end;
                    out.push(Spanned::new(
                        FrontmatterShellSuffix::Timeout(value),
                        offset..offset + len,
                    ));
                    offset += len;
                    suffix = &rest[digits_end..];
                }
                Err(_) => break,
            }
        } else {
            break;
        }
    }

    out
}

/// Byte spans of the whitespace-delimited tokens in a shell action, respecting
/// single/double quotes and backslash escapes. Offsets are relative to `s`.
fn shell_token_spans(s: &str) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut start: Option<usize> = None;

    for (idx, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if !in_single => {
                start.get_or_insert(idx);
                escaped = true;
            }
            '\'' if !in_double => {
                start.get_or_insert(idx);
                in_single = !in_single;
            }
            '"' if !in_single => {
                start.get_or_insert(idx);
                in_double = !in_double;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if let Some(st) = start.take() {
                    spans.push(st..idx);
                }
            }
            _ => {
                start.get_or_insert(idx);
            }
        }
    }
    if let Some(st) = start.take() {
        spans.push(st..s.len());
    }
    spans
}

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
    /// A §2-ladder value branch: a quoted/numeric/boolean literal, a `name(...)`
    /// expression function, a `doc.*` reference, or a lone bare name that is not
    /// on `PATH` (a frontmatter property, or `null` when absent). It resolves to
    /// a value via the expression engine at execute time and contributes **no**
    /// shell command to the allowlist-reachable set, so it is excluded from
    /// approval discovery.
    Value {
        /// Pre-interpolation branch text, evaluated as an expression at execute
        /// time.
        source: String,
    },
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
            Self::Value { source } => Some(source.as_str()),
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
    /// When `true` (from the `::no-cache` suffix), the directive bypasses the
    /// per-compose command cache and executes fresh at every occurrence.
    pub no_cache: bool,
    #[allow(dead_code)] // Inspected by parser tests; production uses `directive_reachable_pipelines`.
    pub pipeline: Option<ShellPipeline>,
    pub ast: FrontmatterShellAst,
    /// 1-indexed file line of the frontmatter key when it can be resolved from
    /// the source context; `None` otherwise.
    pub line: Option<usize>,
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

/// Parses the suffix tail that follows the closing `)` of a `$(...)` shell
/// expression.
///
/// Two suffixes are recognized — `::timeout:N` and the `::no-cache` cache
/// opt-out — in any order. Each may appear at most once; anything else (any
/// non-suffix trailing content) is a hard parse error. This grammar is shared
/// between [`parse_shell_value`] and the leak-guard shape detector so the
/// supported suffix forms live in exactly one place.
///
/// ## Errors
///
/// Returns a key-tagged [`ShellExpansionError::ParseDirective`] for a duplicate
/// suffix, an invalid/zero timeout value, or any non-suffix trailing content.
fn parse_shell_suffixes(
    after_close: &str,
    key: &str,
    ctx: &SourceContext,
) -> Result<(Option<std::time::Duration>, bool), ShellExpansionError> {
    let mut timeout_override = None;
    let mut no_cache = false;
    let mut suffix = after_close;
    while !suffix.is_empty() {
        if let Some(rest) = suffix.strip_prefix("::no-cache") {
            if no_cache {
                return Err(frontmatter_parse_error(
                    key,
                    ctx,
                    "Duplicate ::no-cache suffix in frontmatter shell expression",
                ));
            }
            no_cache = true;
            suffix = rest;
        } else if let Some(rest) = suffix.strip_prefix("::timeout:") {
            if timeout_override.is_some() {
                return Err(frontmatter_parse_error(
                    key,
                    ctx,
                    "Duplicate ::timeout suffix in frontmatter shell expression",
                ));
            }
            // The timeout digits run up to the next `::` suffix or end of string.
            let digits_end = rest.find("::").unwrap_or(rest.len());
            let (digits, tail) = rest.split_at(digits_end);
            let timeout_val: u64 = digits.parse().map_err(|_| {
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
            timeout_override = Some(std::time::Duration::from_secs(timeout_val));
            suffix = tail;
        } else {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                "Unexpected trailing content after frontmatter shell expression",
            ));
        }
    }
    Ok((timeout_override, no_cache))
}

/// Parses a single frontmatter string value for shell expression.
///
/// Returns `Some` if the value matches `$(cmd)` or `$(cmd)::timeout:N`.
///
/// ## Rules
///
/// - The value and the `original_value` snapshot are trimmed first; the
///   whole-value `$( … )` rule is defined on that trimmed boundary, so
///   `"  $(echo ok)  "` parses identically to `"$(echo ok)"`
/// - Trimmed value must start with `$(` (a mixed literal whose trimmed form
///   starts with other text returns `Ok(None)` and is left to other handling)
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
    // The whole-value `$( … )` rule is defined on the trimmed value, so a padded
    // value (`"  $(echo ok)  "`) parses and expands identically to its trimmed
    // form. The pre-interpolation snapshot is trimmed to the same boundary so
    // ternary structure and executable-interpolation checks see the same body.
    // A mixed literal whose trimmed form does not open `$(` still returns
    // `Ok(None)` below and stays outside this strict rule.
    let value = value.trim();
    let original_value = original_value.map(str::trim);

    // Must start with $(
    if !value.starts_with("$(") {
        return Ok(None);
    }

    let rest = &value[2..];
    let close_pos = find_unquoted_closing_paren(rest, key, ctx)?;

    let inner_command = &rest[..close_pos];
    let after_close = &rest[close_pos + 1..];

    let (timeout_override, no_cache) = parse_shell_suffixes(after_close, key, ctx)?;

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

        // §2 validity rule: at least one branch must be a real shell command.
        // When both branches are value-producing (literals, `name(...)`
        // functions, or property references), the whole `$()` is expression
        // content with no command — reject it with a `{{ }}` suggestion.
        if !matches!(then_branch, Branch::Pipeline { .. })
            && !matches!(else_branch, Branch::Pipeline { .. })
        {
            return Err(no_command_diagnostic(key, ctx, validation_inner));
        }

        return Ok(Some(FrontmatterShellDirective {
            key: key.to_string(),
            raw_command: inner_command.to_string(),
            executable: String::new(),
            args: Vec::new(),
            timeout_override,
            no_cache,
            pipeline: None,
            ast: FrontmatterShellAst::Ternary {
                condition_source,
                then_branch,
                else_branch,
            },
            line: None,
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

    // §2 validity rule for a non-ternary directive: the body must be a real
    // shell command in executed position. A body that resolves to a value
    // (a quoted/numeric/boolean literal, a `name(...)` function, a `doc.*`
    // reference, or a bare frontmatter property) is expression content, not a
    // shell pipeline — reject it with a `{{ }}` suggestion.
    if !matches!(classify_executed_body(validation_inner), BodyClass::Command) {
        return Err(no_command_diagnostic(key, ctx, validation_inner));
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
        no_cache,
        pipeline: Some(pipeline.clone()),
        ast: FrontmatterShellAst::Pipeline(pipeline),
        line: None,
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

    if original_trim.is_empty() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary {} must be a command pipeline, an expression value, or an empty string literal",
                position.name()
            ),
        ));
    }

    // Classify the branch per the §2 ladder. Value branches (literals,
    // `name(...)` functions, `doc.*`, or bare properties) resolve through the
    // expression engine and carry no shell executable, so the
    // executable-interpolation guard applies only to command branches.
    match classify_executed_body(original_trim) {
        BodyClass::Empty => Ok(Branch::Empty),
        BodyClass::Value => Ok(Branch::Value {
            source: original.to_string(),
        }),
        BodyClass::Command => {
            validate_branch_no_executable_interpolation(original, position, key, ctx)?;
            Ok(Branch::Pipeline {
                original_text: original.to_string(),
            })
        }
    }
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

/// §2 token-resolution classification of a `$()` executed-position body — a
/// ternary branch, or the body of a non-ternary directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyClass {
    /// An empty-string literal (`''` / `""`) — produces `""`.
    Empty,
    /// Resolves to a value through the expression engine (literal, `name(...)`
    /// function, `doc.*`, or a bare frontmatter property / `null`). Not a shell
    /// command.
    Value,
    /// A shell command pipeline (path-bearing executable, a bare name found on
    /// `PATH`, or any multi-token / chained pipeline).
    Command,
}

/// Classifies a `$()` executed-position body per the §2 token-resolution
/// ladder.
///
/// Only a *lone* bare name is ambiguous between command and property: it is a
/// command when found on `PATH` and a frontmatter property otherwise. Anything
/// with arguments, pipes, or chain operators is a command. `true`/`false` are
/// always booleans, path-bearing tokens are always executables, and a token
/// with trailing parentheses is always a (safe) expression function.
fn classify_executed_body(body: &str) -> BodyClass {
    let t = body.trim();
    if t.is_empty() || is_empty_string_literal(t) {
        return BodyClass::Empty;
    }
    // 1–3: quoted / numeric / boolean literals.
    if is_quoted_literal(t) || t.parse::<f64>().is_ok() || t == "true" || t == "false" {
        return BodyClass::Value;
    }
    // 4: `name(...)` expression function — no shell executable contains parens.
    if is_expression_function_call(t) {
        return BodyClass::Value;
    }
    // 5–6: a lone bare name / path. Multi-token bodies are always commands.
    if is_single_token(t) {
        // An interpolated executable is still a command attempt; defer to the
        // executable-interpolation guard to reject it rather than treating the
        // `{{ … }}` marker as a property name.
        if t.contains("{{") {
            return BodyClass::Command;
        }
        // Path-bearing tokens are always executables, never properties.
        if t.contains('/') {
            return BodyClass::Command;
        }
        // `doc.*` is always an explicit frontmatter property reference.
        if doc_namespace::is_doc_namespace(t) {
            return BodyClass::Value;
        }
        if which::which(t).is_ok() {
            BodyClass::Command
        } else {
            BodyClass::Value
        }
    } else {
        BodyClass::Command
    }
}

/// Returns `true` when `t` is a single complete quoted string literal — it
/// opens with `'` or `"`, closes with the same quote as its final character,
/// and contains no top-level whitespace.
fn is_quoted_literal(t: &str) -> bool {
    let bytes = t.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    let quote = bytes[0];
    (quote == b'\'' || quote == b'"') && bytes[bytes.len() - 1] == quote && is_single_token(t)
}

/// Returns `true` when `t` is an expression function call: a (dotted)
/// identifier immediately followed by a parenthesized argument list spanning to
/// the end of the body. Because no shell executable name may contain `(` or
/// `)`, this is an unambiguous syntactic test rather than a heuristic.
fn is_expression_function_call(t: &str) -> bool {
    let Some(open) = t.find('(') else {
        return false;
    };
    if open == 0 || !t.ends_with(')') {
        return false;
    }
    let name = &t[..open];
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Returns `true` when `body` is a single shell token — it contains no
/// whitespace outside single quotes, double quotes, parentheses, or backslash
/// escapes.
fn is_single_token(body: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut paren_depth: u32 = 0;

    for ch in body.chars() {
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
            c if c.is_whitespace() && !in_single && !in_double && paren_depth == 0 => {
                return false;
            }
            _ => {}
        }
    }
    true
}

/// Builds the "no shell command" diagnostic for a `$()` whose executed-position
/// content is entirely expression-engine material, steering the author toward
/// `{{ … }}` interpolation.
fn no_command_diagnostic(key: &str, ctx: &SourceContext, inner: &str) -> ShellExpansionError {
    let trimmed = inner.trim();
    frontmatter_parse_error(
        key,
        ctx,
        format!(
            "Frontmatter shell expression `$( {trimmed} )` contains no shell command in executed \
             position — it is entirely darkmatter expression content (literals, `name(...)` \
             functions, or property references), not a shell pipeline. Did you mean to use \
             `{{{{ {trimmed} }}}}` interpolation instead of `$( … )`?"
        ),
    )
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
/// Keys present in `exclude_keys` are skipped entirely (DM1 passthrough).
pub(crate) fn scan_frontmatter(
    frontmatter: &Frontmatter,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
    ctx: &SourceContext,
    exclude_keys: &std::collections::HashSet<String>,
) -> Result<Vec<FrontmatterShellDirective>, ShellExpansionError> {
    let mut directives = Vec::new();
    let fm = frontmatter.as_map();

    for (key, value) in fm.iter() {
        if exclude_keys.contains(key) {
            continue;
        }
        // Only process top-level string values
        if let Value::String(s) = value {
            let original = pre_interpolation_snapshot
                .and_then(|map| map.get(key))
                .map(|s| s.as_str());

            if let Some(mut directive) = parse_shell_value(s, key, original, ctx)? {
                directive.line = frontmatter_key_line(ctx, key);
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
    let candidates = scan_frontmatter(frontmatter, pre_interpolation_snapshot, ctx, &options.exclude_keys)?;

    if candidates.is_empty() {
        // No directives to execute, but a scan-skipped whole-value `$(...)`
        // (e.g. one behind leading whitespace) could still be sitting in
        // frontmatter. Enabled shell expansion must never leave a raw
        // expansion-form value behind, so guard even on the no-op path.
        validate_no_whole_value_shell_leak(frontmatter, ctx, &options.exclude_keys)?;
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

    // One policy snapshot opens the stage and is shared by every directive in
    // it, rather than each directive cloning the rule sets for itself.
    let snapshot = runtime.shell.snapshot();

    // Snapshot frontmatter values for ternary condition lookups. Built lazily
    // so non-ternary inputs don't pay the cloning cost. The real run carries a
    // resolution context so a ternary condition and its selected branch can use
    // the read-side functions (`file_exists`, `frontmatter`, …) and `doc.*`.
    // Frontmatter is local-only (Decision B): this context never attaches a
    // remote-fetch runtime, so a remote URL argument here fails loudly.
    let resolution_context = options.frontmatter_resolution_context();
    let mut seed_state: Option<FrontmatterSeedState> = None;

    // Either "execute this prepared pipeline" or "use this resolved value".
    // PreparedShellDirective is hefty (~432 bytes), so box the variant to
    // keep the enum compact. `Value` carries an empty-string branch or a §2
    // value branch already evaluated through the expression engine.
    enum Pending {
        Execute(Box<PreparedShellDirective>),
        Value(String),
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
                    &snapshot,
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
                        seed_state = Some(
                            FrontmatterSeedState::new(map, options.context().clone())
                                .with_resolution_context(Some(resolution_context.clone())),
                        );
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
                    &snapshot,
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
                    &snapshot,
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
                    PreparedBranch::Value(value) => Pending::Value(value),
                    PreparedBranch::Command(prepared) => Pending::Execute(prepared),
                }
            }
        };

        pending.push((index, candidate.key.clone(), pending_item));
    }

    // Run executable preparations in parallel; short-circuit branches contribute
    // an empty string and skip the shell runtime entirely. The cache lives behind
    // the runtime's shared mutex, so a shared borrow suffices here.
    let shell_runtime = &runtime.shell;
    let mut executions: Vec<_> = pending
        .into_par_iter()
        .map(|(index, key, item)| {
            let result = match item {
                Pending::Execute(prepared) => {
                    execute_prepared_directive(&prepared, options, shell_runtime)
                        .map(|res| (res.stdout, res.warnings))
                }
                Pending::Value(value) => Ok((value, Vec::new())),
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

    // Post-expansion leak guard: after replacements, no top-level value may
    // still be a whole-value `$(...)` candidate (e.g. command output that
    // reproduced `$( … )`). Such a value is leaked executable state, not text.
    validate_no_whole_value_shell_leak(frontmatter, ctx, &options.exclude_keys)?;

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
            // Preflight only: this seed state stays context-free. Discovery
            // enumerates the reachable pipelines for approval without performing
            // any expression-driven branch selection, so read-side functions
            // (which would need a `ResolutionContext`) are intentionally absent
            // here. The real run in `execute_frontmatter_shell_expansion`
            // supplies the context.
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

    let value = evaluate(&parsed, state).map_err(|error| {
        frontmatter_parse_error(
            key,
            ctx,
            format!("Frontmatter shell ternary condition must be a boolean expression: {error}"),
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
#[allow(clippy::too_many_arguments)]
fn prepare_branch_pipeline(
    pipeline: ShellPipeline,
    raw_command: String,
    candidate: &FrontmatterShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    snapshot: &ShellRuntimeSnapshot,
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
            line: candidate.line,
        },
        error_handling: ErrorHandling::default(),
        timeout_override: candidate.timeout_override,
        no_cache: candidate.no_cache,
        pipeline: Some(pipeline),
        ctx: ctx.clone(),
    };

    prepare_directive(&directive, options, policy_paths, snapshot, &mut runtime.shell)
}

/// A ternary branch resolved for execution: either a value produced without a
/// shell invocation, or an allowlist-checked command pipeline.
enum PreparedBranch {
    /// A resolved value — an empty-string branch or a §2 value branch evaluated
    /// through the expression engine. Runs no shell command.
    Value(String),
    /// A prepared, allowlist-checked command pipeline.
    Command(Box<PreparedShellDirective>),
}

/// Prepares one side of a ternary.
///
/// `Branch::Empty` short-circuits to `""` and contributes no command. A
/// `Branch::Value` is evaluated through the expression engine (literals,
/// `name(...)` functions, `doc.*`, or a bare frontmatter property / `null`)
/// and likewise contributes no command, so it is excluded from approval
/// discovery. `Branch::Pipeline` is interpolated against the seed state,
/// tokenized into a pipeline, and dispatched through [`prepare_branch_pipeline`]
/// so its commands are validated against the user's allowlist before the
/// ternary condition is evaluated. Per-branch interpolation keeps the branch
/// boundary anchored to the original ternary AST.
#[allow(clippy::too_many_arguments)]
fn prepare_optional_branch(
    branch: &Branch,
    candidate: &FrontmatterShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    snapshot: &ShellRuntimeSnapshot,
    runtime: &mut PipelineRuntime,
    ctx: &SourceContext,
    state: &FrontmatterSeedState,
    position: BranchPosition,
) -> Result<PreparedBranch, ShellExpansionError> {
    match branch {
        Branch::Empty => Ok(PreparedBranch::Value(String::new())),
        Branch::Value { source } => Ok(PreparedBranch::Value(evaluate_value_branch(
            source,
            state,
            position,
            &candidate.key,
            ctx,
        )?)),
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
                snapshot,
                runtime,
                ctx,
            )?;
            Ok(PreparedBranch::Command(Box::new(prepared)))
        }
    }
}

/// Evaluates a §2 value branch (a literal, `name(...)` function, `doc.*`, or a
/// bare frontmatter property) against the seed state and returns its scalar
/// string form.
///
/// The branch source is first interpolated (so an embedded `{{ … }}` resolves),
/// then parsed and evaluated as a template expression. The seed state carries
/// the [`ResolutionContext`](super::expression::ResolutionContext) when one is
/// available, so a read-side function such as `file_exists(...)` resolves; an
/// absent bare property evaluates to `null`, which renders as an empty string.
fn evaluate_value_branch(
    source: &str,
    state: &FrontmatterSeedState,
    position: BranchPosition,
    key: &str,
    ctx: &SourceContext,
) -> Result<String, ShellExpansionError> {
    let evaluator = Evaluator::new(state);
    let rewrite = interpolate_text(
        source,
        &evaluator,
        ScanMode::Plain,
        true,
        "frontmatter-shell-ternary-value",
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

    let expression_text = rewrite.output.trim();
    let parsed = parse(expression_text).map_err(|err| {
        frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary {} failed to parse as an expression: {}",
                position.name(),
                err.message
            ),
        )
    })?;

    let value = evaluate(&parsed, state).map_err(|error| {
        frontmatter_parse_error(
            key,
            ctx,
            format!(
                "Frontmatter shell ternary {} evaluation failed: {error}",
                position.name()
            ),
        )
    })?;

    Ok(scalar_string(&value))
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

/// Post-expansion leak guard for top-level frontmatter string values.
///
/// After enabled frontmatter shell expansion has run, no top-level value may
/// still be a whole-value `$(...)` shell-expansion candidate. A surviving
/// candidate means executable state leaked as raw syntax — e.g. a command whose
/// own output reproduced `$( … )`, or a `$(...)` value behind leading
/// whitespace that the strict-start scan skipped. Frontmatter values that are
/// exactly an expansion form are executable state, not text, and must resolve
/// or fail loudly rather than reach the composed document verbatim.
///
/// Candidate recognition is delegated to [`is_whole_value_shell_candidate`],
/// which reuses [`parse_shell_value`] so the `$( … )` grammar and supported
/// suffix rules (`::timeout` / `::no-cache`) are defined in exactly one place.
/// Mixed literals (`literal $(echo ok)`) and values with trailing content after
/// the closing paren are not whole-value candidates and pass through unchanged.
///
/// ## Errors
///
/// Returns a key-tagged [`ShellExpansionError::ParseDirective`] for the first
/// surviving whole-value candidate, carrying the frontmatter key, the offending
/// expression text, and source location when [`SourceContext`] can locate the
/// key.
fn validate_no_whole_value_shell_leak(
    frontmatter: &Frontmatter,
    ctx: &SourceContext,
    exclude_keys: &std::collections::HashSet<String>,
) -> Result<(), ShellExpansionError> {
    for (key, value) in frontmatter.as_map().iter() {
        if exclude_keys.contains(key) {
            continue;
        }
        if let Value::String(s) = value {
            match is_whole_value_shell_candidate(s, key, ctx) {
                WholeValueShellShape::NotCandidate => {}
                WholeValueShellShape::CleanDirective => {
                    let trimmed = s.trim();
                    return Err(frontmatter_parse_error(
                        key,
                        ctx,
                        format!(
                            "Frontmatter shell expression `{trimmed}` survived shell expansion as a raw \
                             whole-value `$( … )` candidate. A frontmatter value that is exactly a `$( … )` \
                             expansion is executable state, not text; it must resolve during shell \
                             expansion, so a surviving candidate (e.g. command output that reproduced \
                             `$( … )`) is rejected rather than leaked into the composed frontmatter."
                        ),
                    ));
                }
                // A whole-value shape that fails the shared parser (malformed,
                // unclosed, or no-command) is executable state that cannot be
                // expanded — fatal, not lenient. Propagate the underlying parse
                // error so the original diagnostic (e.g. missing `)`, no-command
                // `{{ }}` suggestion) reaches the caller.
                WholeValueShellShape::ShapeButError(err) => return Err(err),
            }
        }
    }
    Ok(())
}

/// Classification of a trimmed frontmatter string value with respect to the
/// whole-value `$( … )` shell-expansion shape.
enum WholeValueShellShape {
    /// Not a whole-value `$( … )` value: ordinary text, a mixed literal with a
    /// non-`$(` prefix, or a `$( … )` followed by non-suffix trailing content
    /// (`$(echo ok) trailing`). Left to existing behavior — the guard skips it.
    NotCandidate,
    /// A whole-value `$( … )` that cleanly parses to a directive. A surviving
    /// instance is a fatal leak (executable state reaching the composed
    /// document verbatim).
    CleanDirective,
    /// A whole-value `$( … )` *shape* (unclosed, or a clean close followed only
    /// by valid suffixes) that nonetheless fails the shared parser. Carries the
    /// underlying parse error to propagate as fatal.
    ShapeButError(ShellExpansionError),
}

/// Classifies `value`, after trimming, with respect to the whole-value `$( … )`
/// shell-expansion shape.
///
/// A value that does not open `$(` is ordinary text ([`NotCandidate`]). When it
/// does open `$(`, the closing paren and any suffix tail decide the shape:
///
/// - **Unclosed** (no unquoted closing `)`): a whole-value shape whose body
///   cannot parse — returns [`ShapeButError`] carrying the missing-paren error.
/// - **Clean close followed only by valid suffixes** (`::no-cache` /
///   `::timeout:N`, any order, each at most once): a whole-value shape. If the
///   directive then parses, [`CleanDirective`]; otherwise [`ShapeButError`]
///   carrying the body's parse error (e.g. the no-command diagnostic).
/// - **Clean close followed by non-suffix content** (`$(echo ok) trailing`): a
///   trailing-literal mixed value — [`NotCandidate`], left lenient.
///
/// Close detection reuses [`find_unquoted_closing_paren`] and suffix detection
/// reuses [`parse_shell_suffixes`], so the `$( … )` grammar lives in one place.
///
/// [`NotCandidate`]: WholeValueShellShape::NotCandidate
/// [`CleanDirective`]: WholeValueShellShape::CleanDirective
/// [`ShapeButError`]: WholeValueShellShape::ShapeButError
fn is_whole_value_shell_candidate(
    value: &str,
    key: &str,
    ctx: &SourceContext,
) -> WholeValueShellShape {
    let trimmed = value.trim();
    if !trimmed.starts_with("$(") {
        return WholeValueShellShape::NotCandidate;
    }

    let rest = &trimmed[2..];
    let close_pos = match find_unquoted_closing_paren(rest, key, ctx) {
        Ok(pos) => pos,
        // Unclosed whole-value shape: not expandable, must be fatal.
        Err(err) => return WholeValueShellShape::ShapeButError(err),
    };

    let after_close = &rest[close_pos + 1..];
    // Non-suffix trailing content (`$(echo ok) trailing`) is a trailing-literal
    // mixed value, not a whole-value shape — stay lenient. Valid suffixes keep
    // the whole-value shape, so a body parse error below still escalates.
    if parse_shell_suffixes(after_close, key, ctx).is_err() {
        return WholeValueShellShape::NotCandidate;
    }

    match parse_shell_value(trimmed, key, None, ctx) {
        Ok(Some(_)) => WholeValueShellShape::CleanDirective,
        // The body of a whole-value shape failed to parse (e.g. a no-command
        // body). Propagate that diagnostic as fatal.
        Err(err) => WholeValueShellShape::ShapeButError(err),
        // `Ok(None)` is unreachable for a `$(`-opening value (the parser returns
        // `Ok(None)` only when the value does not start with `$(`), but treat it
        // defensively as not-a-candidate.
        Ok(None) => WholeValueShellShape::NotCandidate,
    }
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
            line: frontmatter_key_line(ctx, key),
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
    unquoted_closing_paren_offset(rest).ok_or_else(|| {
        frontmatter_parse_error(
            key,
            ctx,
            "Missing closing ')' in frontmatter shell expression",
        )
    })
}

/// Returns the byte offset of the top-level, unquoted `)` that closes a
/// `$(...)` body opened at the start of `rest`, or `None` when the body is
/// never closed.
///
/// Shared by [`find_unquoted_closing_paren`] (which adds error context) and the
/// read-only span parser [`parse_frontmatter_shell_value_spanned`], so both
/// agree on what "top-level" means: outside single/double quotes and outside any
/// nested `( … )` pair.
fn unquoted_closing_paren_offset(rest: &str) -> Option<usize> {
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
            ')' if !in_single && !in_double => return Some(idx),
            _ => {}
        }
    }

    None
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
#[path = "frontmatter_shell_expansion/tests/spanned_value_tests.rs"]
mod spanned_value_tests;

#[cfg(test)]
#[path = "frontmatter_shell_expansion/tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "frontmatter_shell_expansion/tests/execution_tests.rs"]
mod execution_tests;
