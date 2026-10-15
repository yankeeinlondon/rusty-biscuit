//! Parser for `::shell` directives in markdown content.

use super::tokenize::{ShellToken, parse_pipeline, tokenize};
use super::types::{ErrorHandling, ShellCommandOrigin, ShellDirective, ShellExpansionError};
use crate::markdown::compose::body_origin::DataRanges;
use crate::markdown::compose::parse_utils::{
    authored_directive, directive_prefix_len, find_code_regions, is_in_code_region,
    structural_view,
};
use biscuit_terminal::errors::SourceContext;

/// Parses all `::shell` directives from markdown content.
///
/// ## Returns
///
/// A vector of parsed directives with their byte ranges and line numbers.
///
/// ## Errors
///
/// Returns an error if any directive has invalid syntax.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::errors::SourceContext;
/// use darkmatter::markdown::compose::shell_expansion::parser::parse_directives;
/// use std::path::PathBuf;
///
/// let content = "# Test\n::shell echo hello\nSome text\n";
/// let ctx = SourceContext::new(PathBuf::from("/t"), PathBuf::from("t"), content);
/// let directives = parse_directives(content, ctx, 0).unwrap();
/// assert_eq!(directives.len(), 1);
/// assert_eq!(directives[0].executable, "echo");
/// ```
pub fn parse_directives(
    content: &str,
    ctx: SourceContext,
    line_offset: usize,
) -> Result<Vec<ShellDirective>, ShellExpansionError> {
    parse_directives_in(content, None, ctx, line_offset)
}

/// [`parse_directives`] over a body whose `data` bytes were inserted by an
/// earlier stage.
///
/// A directive counts only when its line start, prefix, and `::shell` keyword
/// are authored; code regions are found in the masked view. Data may supply a
/// command's arguments but never its shape: a directive whose executable,
/// chain operators, or redirections come from data fails.
pub(crate) fn parse_directives_in(
    content: &str,
    data: Option<&DataRanges>,
    ctx: SourceContext,
    line_offset: usize,
) -> Result<Vec<ShellDirective>, ShellExpansionError> {
    scan_directive_lines(content, data, ctx, line_offset)
        .into_iter()
        .map(|line| line.parsed)
        .collect()
}

/// One `::shell` directive line, found by its authored line start, prefix,
/// and keyword, with its command parsed from the line's current text.
#[derive(Debug)]
pub(crate) struct DirectiveLine {
    /// The whole line, terminator included.
    pub span: std::ops::Range<usize>,
    /// The `::shell` keyword. Always authored, so it projects back to the
    /// authored body.
    pub keyword: std::ops::Range<usize>,
    pub indent: String,
    /// The line's current text parsed as a resolved-byte directive, or why it
    /// could not be. A placeholder value that holds quotes, operators, or
    /// command syntax fails here; a template reading of the authored line may
    /// still run it.
    pub parsed: Result<ShellDirective, ShellExpansionError>,
}

/// [`parse_directives_in`], keeping each line's parse failure with the line
/// instead of failing the whole body, so a caller can still run a directive
/// whose authored text is a valid template.
pub(crate) fn scan_directive_lines(
    content: &str,
    data: Option<&DataRanges>,
    ctx: SourceContext,
    line_offset: usize,
) -> Vec<DirectiveLine> {
    let structural = structural_view(content, data);
    let code_regions = find_code_regions(&structural);
    let mut directives = Vec::new();

    let mut byte_offset = 0;

    for (line_num, line_slice) in (1..).zip(content.split_inclusive('\n')) {
        let file_line = line_num + line_offset;
        let line_start = byte_offset;
        let line = line_slice
            .strip_suffix('\n')
            .and_then(|line| line.strip_suffix('\r').or(Some(line)))
            .unwrap_or(line_slice);
        let line_with_newline_end = byte_offset + line_slice.len();

        // Check if this line is inside a code region
        if !is_in_code_region(line_start, &code_regions) {
            // Detect the directive after any block-quote markers and indentation
            // whitespace. The captured prefix (e.g. `> > ` or `    `) is replayed
            // verbatim on every output line so the splice stays nested in its
            // container — a block quote, a list item, or the document root.
            let prefix_len = directive_prefix_len(line);
            let after = line[prefix_len..].trim_end();
            let keyword_end = line_start + prefix_len + "::shell".len();
            if let Some(command_text) = after.strip_prefix("::shell ")
                && authored_directive(data, line_start, keyword_end)
            {
                let indent = line[..prefix_len].to_string();
                let parsed = parse_directive_command(
                    command_text,
                    &structural,
                    data,
                    keyword_end,
                    file_line,
                    &ctx,
                )
                .map(|(pipeline, error_handling, timeout_override, no_cache)| {
                    let raw_command = pipeline.display_string();
                    let executable = pipeline.actions[0].command.executable.clone();
                    let args = pipeline.actions[0].command.args.clone();
                    ShellDirective {
                        raw_command,
                        executable,
                        args,
                        span: line_start..line_with_newline_end,
                        indent: indent.clone(),
                        origin: ShellCommandOrigin::Body { line: file_line },
                        error_handling,
                        timeout_override,
                        no_cache,
                        pipeline: Some(pipeline),
                        ctx: ctx.clone(),
                    }
                });
                directives.push(DirectiveLine {
                    span: line_start..line_with_newline_end,
                    keyword: line_start + prefix_len..keyword_end,
                    indent,
                    parsed,
                });
            }
        }

        byte_offset = line_with_newline_end;
    }

    directives
}

/// The parsed parts of one directive's command text: its pipeline, error
/// handling, timeout override, and cache opt-out.
type ParsedDirectiveCommand = (
    super::types::ShellPipeline,
    ErrorHandling,
    Option<std::time::Duration>,
    bool,
);

/// Parses the command text of a `::shell` line whose keyword ends at
/// `keyword_end`, failing when data inserted into it changed its shape.
fn parse_directive_command(
    command_text: &str,
    structural: &str,
    data: Option<&DataRanges>,
    keyword_end: usize,
    file_line: usize,
    ctx: &SourceContext,
) -> Result<ParsedDirectiveCommand, ShellExpansionError> {
    // Parse the command
    let tokens = tokenize(command_text, ctx).map_err(|e| {
        ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: file_line },
            message: match e {
                ShellExpansionError::ParseDirective { message, .. } => message,
                _ => e.to_string(),
            },
        }
    })?;

    if tokens.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: file_line },
            message: "Empty command".to_string(),
        });
    }

    // Separate error handling options, timeout, and the cache opt-out
    // from shell tokens
    let (error_handling, shell_tokens, timeout_override, no_cache) =
        extract_options_from_tokens(&tokens, file_line, ctx)?;

    if shell_tokens.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: file_line },
            message: "No command after error handling options".to_string(),
        });
    }

    // Parse the pipeline from the shell tokens
    let pipeline = parse_pipeline(&shell_tokens, ctx).map_err(|e| {
        ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: file_line },
            message: match e {
                ShellExpansionError::ParseDirective { message, .. } => message,
                _ => e.to_string(),
            },
        }
    })?;

    if let Some(data) = data {
        let command_start = keyword_end + 1;
        let command_range = command_start..command_start + command_text.len();
        if data.intersects(&command_range) {
            let masked = &structural[command_range];
            if let Some(change) = data_changed_shape(&pipeline, masked, true, ctx) {
                return Err(ShellExpansionError::ParseDirective {
                    ctx: Box::new(ctx.clone()),
                    origin: ShellCommandOrigin::Body { line: file_line },
                    message: format!(
                        "inserted data {change}; interpolation may supply a \
                         command's arguments, never its executable or structure"
                    ),
                });
            }
        }
    }

    Ok((pipeline, error_handling, timeout_override, no_cache))
}

/// The authored command text of every `::shell` directive in `content`, with
/// its 1-based file line, in document order.
///
/// Uses the same line, prefix, and code-region rules as
/// [`parse_directives_in`] but parses nothing, so a placeholder's own spaces,
/// quotes, and operators survive for [`authored_template`].
pub(crate) fn authored_directive_commands(content: &str, line_offset: usize) -> Vec<(String, usize)> {
    let structural = structural_view(content, None);
    let code_regions = find_code_regions(&structural);
    let mut commands = Vec::new();
    let mut byte_offset = 0;
    for (line_num, line_slice) in (1..).zip(content.split_inclusive('\n')) {
        let line_start = byte_offset;
        byte_offset += line_slice.len();
        if is_in_code_region(line_start, &code_regions) {
            continue;
        }
        let line = line_slice
            .strip_suffix('\n')
            .and_then(|line| line.strip_suffix('\r').or(Some(line)))
            .unwrap_or(line_slice);
        let after = line[directive_prefix_len(line)..].trim_end();
        if let Some(command_text) = after.strip_prefix("::shell ") {
            commands.push((command_text.to_string(), line_num + line_offset));
        }
    }
    commands
}

/// The authored command text of the `::shell` directive whose line holds
/// byte `offset` of `content`, with its 1-based file line.
///
/// Applies the same prefix rule as [`authored_directive_commands`]; the
/// caller has already established that the line is a directive outside code.
pub(crate) fn authored_directive_at(content: &str, offset: usize, line_offset: usize) -> Option<(String, usize)> {
    let start = content.get(..offset)?.rfind('\n').map_or(0, |index| index + 1);
    let end = content[offset..].find('\n').map_or(content.len(), |index| offset + index);
    let line = &content[start..end];
    let line = line.strip_suffix('\r').unwrap_or(line);
    let command = line[directive_prefix_len(line)..].trim_end().strip_prefix("::shell ")?;
    let file_line = content[..start].matches('\n').count() + 1 + line_offset;
    Some((command.to_string(), file_line))
}

/// Parses an authored `::shell` command as a template and removes the
/// directive's own options (`--when-error`, `--no-cache`, `::timeout:<N>`, …),
/// which are not command arguments, exactly as [`parse_directives_in`]
/// removes them from the token stream.
pub(crate) fn authored_template(
    command_text: &str,
) -> Result<super::template::CommandTemplate, super::template::TemplateGrammarError> {
    authored_template_with_options(command_text).map(|(template, _)| template)
}

/// [`authored_template`], also returning the removed option words in order.
/// An option word or option argument that holds a placeholder is `None`: it
/// has no authored value.
fn authored_template_with_options(
    command_text: &str,
) -> Result<(super::template::CommandTemplate, Vec<Option<String>>), super::template::TemplateGrammarError> {
    use super::template::{ArgPart, TemplateArg, TemplateGrammarError};

    let literal_of = |arg: &TemplateArg| match arg.parts.as_slice() {
        [ArgPart::Literal(text)] => Some(text.clone()),
        _ => None,
    };
    let mut template = super::template::parse_authored(command_text)?;
    let mut options = Vec::new();
    for segment in &mut template.segments {
        let mut words: Vec<TemplateArg> = std::iter::once(TemplateArg::normalized(vec![ArgPart::Literal(
            std::mem::take(&mut segment.program),
        )]))
        .chain(std::mem::take(&mut segment.args))
        .collect();
        let mut kept = Vec::with_capacity(words.len());
        let mut index = 0;
        while index < words.len() {
            let literal = literal_of(&words[index]);
            let consumed = match literal.as_deref() {
                Some(option) if ERROR_HANDLING_OPTIONS.contains(&option) => 1 + option_arg_count(option),
                Some("--no-cache") => 1,
                Some(word) if word.starts_with("::timeout:") => 1,
                _ => 0,
            };
            if consumed == 0 {
                kept.push(std::mem::replace(&mut words[index], TemplateArg { parts: Vec::new() }));
                index += 1;
                continue;
            }
            for word in words.iter().skip(index).take(consumed) {
                options.push(literal_of(word));
            }
            index += consumed;
        }
        let mut kept = kept.into_iter();
        let program = kept.next().ok_or(TemplateGrammarError::Empty)?;
        segment.program = match program.parts.as_slice() {
            [ArgPart::Literal(text)] if !text.is_empty() => text.clone(),
            [ArgPart::Literal(_)] => return Err(TemplateGrammarError::Empty),
            _ => return Err(TemplateGrammarError::ProgramWordHole),
        };
        segment.args = kept.collect();
    }
    Ok((template, options))
}

/// A `::shell` directive's own options, read from its authored text: error
/// handling, the timeout override, and the cache opt-out.
pub(crate) struct AuthoredOptions {
    pub error_handling: ErrorHandling,
    pub timeout_override: Option<std::time::Duration>,
    pub no_cache: bool,
}

/// The options of the authored `::shell` command `command_text`, for a
/// directive whose current text no longer parses because a placeholder value
/// holds command syntax. `None` when an option or its argument holds a
/// placeholder, so it has no authored value.
///
/// ## Errors
///
/// The same option errors [`parse_directives_in`] reports.
pub(crate) fn authored_directive_options(
    command_text: &str,
    file_line: usize,
    ctx: &SourceContext,
) -> Result<Option<AuthoredOptions>, ShellExpansionError> {
    let Ok((_, words)) = authored_template_with_options(command_text) else {
        return Ok(None);
    };
    let Some(words) = words.into_iter().collect::<Option<Vec<String>>>() else {
        return Ok(None);
    };
    let tokens: Vec<ShellToken> = words.into_iter().map(ShellToken::Word).collect();
    let (error_handling, _, timeout_override, no_cache) = extract_options_from_tokens(&tokens, file_line, ctx)?;
    Ok(Some(AuthoredOptions {
        error_handling,
        timeout_override,
        no_cache,
    }))
}

/// How inserted data changed a command's shape, if it did.
///
/// `masked` is the command text with every data byte masked (see
/// [`DataRanges::masked`]); masking keeps whitespace, so it tokenizes into the
/// same words as the real command unless data contributed quoting, an
/// operator, a redirection, or the executable itself.
///
/// `directive_options` strips the leading `::shell` error-handling options, as
/// the directive parser does; a `::shell-block` command has none.
pub(crate) fn data_changed_shape(
    real: &super::types::ShellPipeline,
    masked: &str,
    directive_options: bool,
    ctx: &SourceContext,
) -> Option<String> {
    let masked = tokenize(masked, ctx)
        .and_then(|tokens| {
            let shell_tokens = if directive_options {
                extract_options_from_tokens(&tokens, 0, ctx)?.1
            } else {
                tokens
            };
            parse_pipeline(&shell_tokens, ctx)
        })
        .ok();
    let Some(masked) = masked else {
        return Some("changes how the command is quoted or parsed".to_string());
    };
    if masked.actions.len() != real.actions.len() {
        return Some(format!(
            "changes the command from {} action(s) to {}",
            masked.actions.len(),
            real.actions.len()
        ));
    }
    for (masked, real) in masked.actions.iter().zip(&real.actions) {
        if masked.command.executable != real.command.executable {
            return Some(format!("supplies the executable `{}`", real.command.executable));
        }
        if masked.operator != real.operator || masked.command.redirection != real.command.redirection {
            return Some("adds a chain operator or redirection".to_string());
        }
    }
    None
}

/// Known directive option names for error handling.
const ERROR_HANDLING_OPTIONS: &[&str] = &[
    "--when-error",
    "--when-exit-code",
    "--except-exit-code",
    "--stderr-contains",
    "--stderr-lacks",
    "--enrich-error",
    "--enrich-error-on",
];

/// Returns the number of argument tokens consumed by a given option.
fn option_arg_count(option: &str) -> usize {
    match option {
        "--when-error" | "--enrich-error" => 1,
        "--when-exit-code" | "--except-exit-code" | "--enrich-error-on" => 2,
        "--stderr-contains" | "--stderr-lacks" => 2,
        _ => 0,
    }
}

/// Extracts error handling options, the timeout suffix, and the `--no-cache`
/// cache opt-out from a mixed token list, returning the remaining shell tokens.
fn extract_options_from_tokens(
    tokens: &[ShellToken],
    line: usize,
    ctx: &SourceContext,
) -> Result<(ErrorHandling, Vec<ShellToken>, Option<std::time::Duration>, bool), ShellExpansionError>
{
    let mut handling = ErrorHandling::default();
    let mut shell_tokens = Vec::new();
    let mut i = 0;
    let mut timeout_override = None;
    let mut no_cache = false;

    while i < tokens.len() {
        match &tokens[i] {
            ShellToken::Word(w) => {
                if ERROR_HANDLING_OPTIONS.contains(&w.as_str()) {
                    let option = w.as_str();
                    let argc = option_arg_count(option);

                    // Collect enough Word tokens after this one
                    let mut opt_args = Vec::new();
                    let mut j = i + 1;
                    while j < tokens.len() && opt_args.len() < argc {
                        if let ShellToken::Word(a) = &tokens[j] {
                            opt_args.push(a.clone());
                        } else {
                            // Non-word token interrupts option arguments
                            break;
                        }
                        j += 1;
                    }

                    if opt_args.len() < argc {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: format!("{option} requires {argc} argument(s)"),
                        });
                    }

                    match option {
                        "--when-error" => {
                            handling.when_error = Some(opt_args[0].clone());
                        }
                        "--when-exit-code" => {
                            let code = parse_exit_code(&opt_args[0], option, line, ctx)?;
                            handling.when_exit_code.push((code, opt_args[1].clone()));
                        }
                        "--except-exit-code" => {
                            let code = parse_exit_code(&opt_args[0], option, line, ctx)?;
                            handling.except_exit_code.push((code, opt_args[1].clone()));
                        }
                        "--stderr-contains" => {
                            handling
                                .stderr_contains
                                .push((opt_args[0].clone(), opt_args[1].clone()));
                        }
                        "--stderr-lacks" => {
                            handling
                                .stderr_lacks
                                .push((opt_args[0].clone(), opt_args[1].clone()));
                        }
                        "--enrich-error" => {
                            handling.enrich_error = Some(opt_args[0].clone());
                        }
                        "--enrich-error-on" => {
                            let code = parse_exit_code(&opt_args[0], option, line, ctx)?;
                            handling.enrich_error_on.push((code, opt_args[1].clone()));
                        }
                        _ => unreachable!(),
                    }

                    i = j;
                } else if w == "--no-cache" {
                    no_cache = true;
                    i += 1;
                } else if let Some(value_str) = w.strip_prefix("::timeout:") {
                    let seconds: u64 = value_str.parse().map_err(|_| {
                        ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: format!(
                                "::timeout requires a positive integer of seconds, got '{value_str}'"
                            ),
                        }
                    })?;
                    if seconds == 0 {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: "::timeout value must be greater than zero".to_string(),
                        });
                    }
                    // ::timeout must be the last token
                    if i != tokens.len() - 1 {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: "::timeout:<N> must be the last token on the ::shell line"
                                .to_string(),
                        });
                    }
                    timeout_override = Some(std::time::Duration::from_secs(seconds));
                    i += 1;
                } else {
                    shell_tokens.push(tokens[i].clone());
                    i += 1;
                }
            }
            // Non-word tokens (operators, redirections) pass through as shell tokens
            _ => {
                shell_tokens.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    Ok((handling, shell_tokens, timeout_override, no_cache))
}

/// Parses an exit code string into an i32.
fn parse_exit_code(
    raw: &str,
    option_name: &str,
    line: usize,
    ctx: &SourceContext,
) -> Result<i32, ShellExpansionError> {
    raw.parse::<i32>()
        .map_err(|_| ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line },
            message: format!("{option_name} requires an integer exit code, got '{raw}'"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn dummy_ctx(content: &str) -> SourceContext {
        SourceContext::new(
            PathBuf::from("/test.md"),
            PathBuf::from("test.md"),
            Arc::from(content),
        )
    }

    #[test]
    fn parse_simple_directive() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(directives[0].origin, ShellCommandOrigin::Body { line: 1 });
        assert_eq!(directives[0].raw_command, "echo hello");
    }

    #[test]
    fn parse_multiple_directives() {
        let content = "::shell ls -la\nSome text\n::shell pwd\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].executable, "ls");
        assert_eq!(directives[0].args, vec!["-la"]);
        assert_eq!(directives[0].origin, ShellCommandOrigin::Body { line: 1 });
        assert_eq!(directives[1].executable, "pwd");
        assert_eq!(directives[1].args.len(), 0);
        assert_eq!(directives[1].origin, ShellCommandOrigin::Body { line: 3 });
    }

    #[test]
    fn parse_directive_with_quotes() {
        let content = "::shell echo \"hello world\"\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello world"]);
    }

    #[test]
    fn parse_ignores_directives_in_code_blocks() {
        let content = r#"
# Test
::shell echo outside

```bash
::shell echo inside_fenced
```

And `::shell echo inline` should also be ignored.
"#;
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["outside"]);
    }

    #[test]
    fn parse_directive_with_leading_whitespace() {
        let content = "   ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
    }

    #[test]
    fn parse_directive_captures_space_indent() {
        let content = "    ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].indent, "    ");
    }

    #[test]
    fn parse_directive_captures_tab_indent() {
        let content = "\t::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].indent, "\t");
    }

    #[test]
    fn parse_directive_at_column_one_has_empty_indent() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].indent, "");
    }

    #[test]
    fn parse_directive_in_blockquote_captures_marker_indent() {
        let content = "> ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(directives[0].indent, "> ");
    }

    #[test]
    fn parse_directive_in_nested_blockquote_captures_marker_indent() {
        let content = "> > ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].indent, "> > ");
    }

    #[test]
    fn parse_directive_mid_line_after_blockquote_is_not_a_directive() {
        // `::shell` must be at the directive position (only markers/whitespace
        // before it); a `::shell` that follows prose stays literal text.
        let content = "> some text ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_directive_span_includes_newline() {
        let content = "::shell echo hello\nNext line\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 19); // Includes newline
        assert_eq!(&content[directives[0].span.clone()], "::shell echo hello\n");
    }

    #[test]
    fn parse_directive_span_includes_crlf() {
        let content = "::shell echo hello\r\nNext line\r\n::shell pwd\r\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 20); // Includes \r\n
        assert_eq!(
            &content[directives[0].span.clone()],
            "::shell echo hello\r\n"
        );
        assert_eq!(directives[1].origin, ShellCommandOrigin::Body { line: 3 });
        assert_eq!(&content[directives[1].span.clone()], "::shell pwd\r\n");
    }

    #[test]
    fn parse_directive_without_trailing_newline() {
        let content = "::shell echo hello";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, content.len());
    }

    #[test]
    fn parse_directive_span_includes_crlf_newline() {
        let content = "::shell echo hello\r\nNext line\r\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 20);
        assert_eq!(
            &content[directives[0].span.clone()],
            "::shell echo hello\r\n"
        );
    }

    #[test]
    fn parse_empty_content() {
        let content = "";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_no_directives() {
        let content = "# Heading\nSome text\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_directive_with_invalid_syntax_fails() {
        let content = "::shell echo | grep\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ShellExpansionError::ParseDirective {
                origin, message, ..
            } => {
                assert_eq!(origin, ShellCommandOrigin::Body { line: 1 });
                assert!(message.contains("pipes"));
            }
            _ => panic!("Expected ParseDirective error, got {:?}", err),
        }
    }

    #[test]
    fn parse_directive_line_numbers_are_correct() {
        let content = "Line 1\nLine 2\n::shell echo a\nLine 4\n::shell echo b\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].origin, ShellCommandOrigin::Body { line: 3 });
        assert_eq!(directives[1].origin, ShellCommandOrigin::Body { line: 5 });
    }

    #[test]
    fn parse_directive_ignores_incomplete_prefix() {
        let content = "::shel echo hello\n::shell echo world\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["world"]);
    }

    #[test]
    fn parse_when_error_option() {
        let content = "::shell --when-error \"fallback\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("fallback".to_string())
        );
        assert_eq!(directives[0].raw_command, "echo hello");
    }

    #[test]
    fn parse_when_exit_code_option() {
        let content = "::shell --when-exit-code 1 \"not found\" grep pattern file.txt\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "grep");
        assert_eq!(directives[0].args, vec!["pattern", "file.txt"]);
        assert_eq!(
            directives[0].error_handling.when_exit_code,
            vec![(1, "not found".to_string())]
        );
    }

    #[test]
    fn parse_except_exit_code_option() {
        let content = "::shell --except-exit-code 0 \"error occurred\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.except_exit_code,
            vec![(0, "error occurred".to_string())]
        );
    }

    #[test]
    fn parse_stderr_contains_option() {
        let content = "::shell --stderr-contains \"warning\" \"warnings found\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.stderr_contains,
            vec![("warning".to_string(), "warnings found".to_string())]
        );
    }

    #[test]
    fn parse_stderr_lacks_option() {
        let content = "::shell --stderr-lacks \"fatal\" \"non-fatal error\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.stderr_lacks,
            vec![("fatal".to_string(), "non-fatal error".to_string())]
        );
    }

    #[test]
    fn parse_enrich_error_option() {
        let content = "::shell --enrich-error \"Check if git is installed\" git log\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.enrich_error,
            Some("Check if git is installed".to_string())
        );
    }

    #[test]
    fn parse_enrich_error_on_option() {
        let content = "::shell --enrich-error-on 127 \"Command not installed\" git log\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.enrich_error_on,
            vec![(127, "Command not installed".to_string())]
        );
    }

    #[test]
    fn parse_multiple_error_handling_options() {
        let content =
            "::shell --when-exit-code 1 \"not found\" --when-error \"failed\" grep pattern\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "grep");
        assert_eq!(
            directives[0].error_handling.when_exit_code,
            vec![(1, "not found".to_string())]
        );
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("failed".to_string())
        );
    }

    #[test]
    fn parse_no_command_after_options_fails() {
        let content = "::shell --when-error \"fallback\"\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No command after error handling options")
        );
    }

    #[test]
    fn parse_missing_option_argument_fails() {
        let content = "::shell echo hello --when-error\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires"));
    }

    #[test]
    fn parse_invalid_exit_code_fails() {
        let content = "::shell --when-exit-code abc \"text\" echo hello\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires an integer exit code")
        );
    }

    #[test]
    fn parse_no_options_leaves_empty_error_handling() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert!(directives[0].error_handling.is_empty());
    }

    #[test]
    fn parse_options_at_end_of_command() {
        let content = "::shell sniff repo staged-packages --when-error \"no packages staged\"\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "sniff");
        assert_eq!(directives[0].args, vec!["repo", "staged-packages"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("no packages staged".to_string())
        );
        assert_eq!(directives[0].raw_command, "sniff repo staged-packages");
    }

    #[test]
    fn parse_options_in_middle_of_command() {
        let content = "::shell sniff --when-error \"fallback\" repo staged-packages\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "sniff");
        assert_eq!(directives[0].args, vec!["repo", "staged-packages"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("fallback".to_string())
        );
    }

    #[test]
    fn parse_multiple_options_at_end() {
        let content = "::shell grep pattern file.txt --when-exit-code 1 \"not found\" --enrich-error \"check path\"\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "grep");
        assert_eq!(directives[0].args, vec!["pattern", "file.txt"]);
        assert_eq!(
            directives[0].error_handling.when_exit_code,
            vec![(1, "not found".to_string())]
        );
        assert_eq!(
            directives[0].error_handling.enrich_error,
            Some("check path".to_string())
        );
    }

    #[test]
    fn parse_timeout_suffix_at_end() {
        let content = "::shell echo hello ::timeout:5\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(
            directives[0].timeout_override,
            Some(std::time::Duration::from_secs(5))
        );
    }

    #[test]
    fn parse_timeout_suffix_after_error_handling() {
        let content = "::shell echo hello --when-error empty ::timeout:3\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("empty".to_string())
        );
        assert_eq!(
            directives[0].timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn parse_no_timeout_suffix_leaves_none() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert!(directives[0].timeout_override.is_none());
    }

    #[test]
    fn parse_no_cache_flag() {
        let content = "::shell --no-cache uuidgen\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "uuidgen");
        assert!(directives[0].no_cache);
    }

    #[test]
    fn parse_no_cache_defaults_false() {
        let content = "::shell uuidgen\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert!(!directives[0].no_cache);
    }

    #[test]
    fn parse_no_cache_alongside_error_handling_and_timeout() {
        let content = "::shell --no-cache --when-error empty uuidgen ::timeout:3\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "uuidgen");
        assert!(directives[0].no_cache);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("empty".to_string())
        );
        assert_eq!(
            directives[0].timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn parse_timeout_suffix_before_error_handling_is_rejected() {
        let content = "::shell echo hello ::timeout:5 --when-error empty\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("::timeout") && err.contains("last"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_timeout_zero_is_rejected() {
        let content = "::shell echo hello ::timeout:0\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("greater than zero"), "got: {err}");
    }

    #[test]
    fn parse_timeout_non_integer_is_rejected() {
        let content = "::shell echo hello ::timeout:abc\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("integer"), "got: {err}");
    }

    #[test]
    fn parse_trailing_and_operator_is_rejected() {
        let content = "::shell echo ok &&\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Missing command after chain operator"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_trailing_or_operator_is_rejected() {
        let content = "::shell echo ok ||\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Missing command after chain operator"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_directive_raw_command_includes_redirection() {
        let content = "::shell ls > /dev/null\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].raw_command, "ls > /dev/null");
    }

    #[test]
    fn parse_directive_raw_command_includes_redirection_in_chain() {
        let content = "::shell echo hi > /dev/null && echo bye 2>&1\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].raw_command,
            "echo hi > /dev/null && echo bye 2>&1"
        );
    }
}
