//! Condition-blind shell-command collection across the document graph.
//!
//! Walks frontmatter, body `::shell`/`::shell-block` directives, and the
//! transclusion graph **without** evaluating any `when=`/page-block condition,
//! so the resulting approval set is a superset of anything reachable under any
//! document state. This is the *approval* half of pre-flight; condition-aware
//! *execution* stays in the inline shell-expansion stage.
//!
//! The walk is a single recursive pass per document: each visited file resolves
//! its own inline state (frontmatter interpolation, text replacement, body
//! interpolation) — never page blocks, never transclusion, never shell
//! execution — then contributes its commands and recurses into every
//! referenced child. There is no second discovery compose that runs the full
//! transclusion merge.

use std::collections::HashSet;
use std::ops::Range;
use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOperation;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::context::effective_state as state;
use crate::markdown::compose::frontmatter_interpolation::interpolate_frontmatter;
use crate::markdown::compose::frontmatter_shell_expansion::{
    directive_reachable_pipelines, parse_shell_value, scan_frontmatter,
};
use crate::markdown::compose::prepare_frontmatter_for_compose;
use crate::markdown::compose::shell_expansion::alias::resolve_alias;
use crate::markdown::compose::shell_expansion::parser::parse_directives;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::types::{
    ShellCommandEntry, ShellCommandOrigin, ShellDirective, ShellExpansionError,
};
use crate::markdown::compose::transclusion;
use crate::markdown::types::MarkdownResult;

use super::super::block_pairs;
use super::super::parse_utils::strip_blockquote_prefix;
use super::super::shell_blocks::body::split_logical_commands;

/// Renders a single action back to its raw command form for display in
/// discovery output (without any preceding chain operator).
fn render_action(executable: &str, args: &[String]) -> String {
    if args.is_empty() {
        return executable.to_string();
    }
    let mut out = String::from(executable);
    for arg in args {
        out.push(' ');
        if arg.contains(' ') || arg.contains('"') || arg.contains('\'') {
            out.push('"');
            out.push_str(&arg.replace('"', "\\\""));
            out.push('"');
        } else {
            out.push_str(arg);
        }
    }
    out
}

/// Yields one tuple per executable action in a directive: the per-action raw
/// command rendering, the executable, and its args. For non-pipeline
/// directives this yields a single entry.
fn directive_action_iter(directive: &ShellDirective) -> Vec<(String, String, Vec<String>)> {
    if let Some(ref pipeline) = directive.pipeline
        && pipeline.actions.len() > 1
    {
        return pipeline
            .actions
            .iter()
            .map(|a| {
                let exe = a.command.executable.clone();
                let args = a.command.args.clone();
                let raw = render_action(&exe, &args);
                (raw, exe, args)
            })
            .collect();
    }
    vec![(
        directive.raw_command.clone(),
        directive.executable.clone(),
        directive.args.clone(),
    )]
}

/// Resolves an executable through alias expansion, mirroring the runtime path.
fn resolve_executable(exe_raw: &str, args_raw: &[String]) -> (String, Vec<String>) {
    if which::which(exe_raw).is_ok() {
        (exe_raw.to_string(), args_raw.to_vec())
    } else if let Some(resolved) = resolve_alias(exe_raw) {
        let mut merged_args = resolved.args;
        merged_args.extend_from_slice(args_raw);
        (resolved.executable, merged_args)
    } else {
        (exe_raw.to_string(), args_raw.to_vec())
    }
}

/// Walks the full document graph and returns every shell command that *could*
/// run under any document state.
///
/// Collection is **condition-blind**: commands inside `when=`-false page blocks
/// and false-condition transclusions are still collected, so the approval set
/// covers every branch. Interpolation and text replacement resolve normally so
/// commands have their final shape; page blocks, transclusion, and shell
/// execution never run. Entries are deduped by normalized command string.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::compose::ComposeOptions;
/// use darkmatter::markdown::compose::preflight::collect_shell_commands;
///
/// let md: Markdown = "# Test\n::shell echo hello\n".into();
/// let options = ComposeOptions::new();
/// let entries = collect_shell_commands(&md, &options).unwrap();
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].executable, "echo");
/// ```
///
/// ## Errors
///
/// - Propagates interpolation/transclusion-resolution failures.
/// - Propagates `::shell` directive parse errors.
/// - Returns [`ShellExpansionError::DynamicCommandShape`] when a body command
///   embeds a frontmatter value still pending frontmatter-shell expansion.
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<Vec<ShellCommandEntry>> {
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut visited = HashSet::new();
    collect_recursive(markdown, options, &mut seen, &mut entries, &mut visited)?;
    Ok(entries)
}

/// Collects every command from one document and recurses into its referenced
/// children, condition-blind.
fn collect_recursive(
    markdown: &Markdown,
    options: &ComposeOptions,
    seen: &mut HashSet<String>,
    entries: &mut Vec<ShellCommandEntry>,
    visited: &mut HashSet<PathBuf>,
) -> MarkdownResult<()> {
    let source_file = match &options.source {
        ComposeSource::File(p) => p.clone(),
        _ => PathBuf::from("<unknown>"),
    };

    if let ComposeSource::File(path) = &options.source {
        let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        if !visited.insert(key) {
            return Ok(());
        }
    }

    // ── Frontmatter commands ───────────────────────────────────────
    scan_one_frontmatter(markdown, options, &source_file, seen, entries)?;

    // ── Resolve this document's inline state ───────────────────────
    // Interpolation + text replacement only: never page blocks (so
    // `when=`-false regions survive), never transclusion (we walk children
    // ourselves), never shell execution.
    let inline_ops: Vec<_> = [
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::Interpolation,
    ]
    .into_iter()
    .filter(|op| options.is_enabled(*op))
    .collect();

    // Discovery is a non-terminal pass: it strips FrontmatterShellExpansion to
    // avoid executing commands, so schema validation here would judge
    // still-literal `$(...)` values as final violations. Skip it — the terminal
    // compose pass validates the resolved frontmatter.
    let mut inline_options = options.clone().only(&inline_ops);
    inline_options.skip_schema_validation = true;
    let (prepared, _) = markdown.compose_with(inline_options)?;
    let prepared_ctx = prepared.source_context_for_errors();

    // ── Dynamic command shape (chicken-and-egg) ────────────────────
    // A body command whose text embeds a frontmatter value still pending
    // frontmatter-shell expansion cannot be approved condition-blind: its
    // approved shape (with `$(...)`) differs from its executed shape. Reject it
    // here rather than letting it surface as a late `NotPreApproved`.
    let pending = pending_shell_literals(&prepared, &prepared_ctx);
    detect_dynamic_command_shape(prepared.content(), &pending, &prepared_ctx)?;

    // ── Body `::shell` directives ──────────────────────────────────
    let directives = parse_directives(prepared.content(), prepared_ctx.clone())?;
    for directive in directives {
        let line = directive.origin.line_number();
        for (raw_action, exe_raw, args_raw) in directive_action_iter(&directive) {
            let (executable, args) = resolve_executable(&exe_raw, &args_raw);
            let normalized = normalize_command(&executable, &args);
            if seen.insert(normalized.clone()) {
                entries.push(ShellCommandEntry {
                    raw_command: raw_action,
                    executable,
                    args,
                    normalized,
                    source_file: source_file.clone(),
                    origin: ShellCommandOrigin::Body { line },
                });
            }
        }
    }

    // ── Body `::shell-block` commands ──────────────────────────────
    let pairs = block_pairs::scan_block_pairs(prepared.content())
        .map_err(|e| crate::markdown::types::MarkdownError::Transform(e.to_string()))?;
    for pair in pairs {
        if !matches!(pair.kind, block_pairs::BlockOpenKind::Shell) {
            continue;
        }
        let body_text = &prepared.content()[pair.body_span.clone()];
        let commands = split_logical_commands(body_text, pair.start_line + 1)
            .map_err(|e| crate::markdown::types::MarkdownError::Transform(e.to_string()))?;
        for command in commands {
            for action in &command.pipeline.actions {
                let exe_raw = action.command.executable.clone();
                let args_raw = action.command.args.clone();
                let raw_command = render_action(&exe_raw, &args_raw);
                let (executable, args) = resolve_executable(&exe_raw, &args_raw);
                let normalized = normalize_command(&executable, &args);
                if !seen.insert(normalized.clone()) {
                    continue;
                }
                entries.push(ShellCommandEntry {
                    raw_command,
                    executable,
                    args,
                    normalized,
                    source_file: source_file.clone(),
                    origin: ShellCommandOrigin::ShellBlock {
                        start_line: pair.start_line,
                        command_line: command.start_line,
                    },
                });
            }
        }
    }

    // ── Recurse into referenced children (condition-blind) ─────────
    let transclusion_opts = options.transclusion_options();

    for directive in transclusion::parse_directives(prepared.content(), prepared_ctx.clone())? {
        if directive.kind != transclusion::DirectiveKind::File {
            continue;
        }

        // No `when=` evaluation: a false-condition transclusion still
        // contributes its commands to the approval set.
        let target = transclusion::normalize_reference_token(&directive.raw_target);
        let transclusion::ResolvedTarget::File { path, .. } = transclusion::resolve_target(
            directive.kind,
            &target,
            &transclusion_opts,
            &options.source,
            directive.line,
            prepared_ctx.clone(),
        )?
        else {
            continue;
        };

        let mut child = Markdown::try_from(path.as_path())?;
        if directive.options.set_object.is_some() || !directive.options.set_properties.is_empty() {
            let base_indexmap = std::mem::take(child.frontmatter_mut().as_map_mut());
            let base_map: serde_json::Map<String, serde_json::Value> =
                base_indexmap.into_iter().collect();
            let overlaid = state::apply_set_overrides(
                &base_map,
                directive.options.set_object.as_ref(),
                &directive.options.set_properties,
            );
            *child.frontmatter_mut().as_map_mut() = overlaid.into_iter().collect();
        }

        collect_recursive(
            &child,
            &options.clone().with_source_file(path),
            seen,
            entries,
            visited,
        )?;
    }

    let refs =
        transclusion::parse_frontmatter_refs(prepared.frontmatter().as_map(), prepared_ctx.clone())?;
    for reference in refs.prologue.iter().chain(refs.epilogue.iter()) {
        if !transclusion::is_url_like(reference) && !transclusion::is_file_like_reference(reference) {
            continue;
        }

        let kind = if transclusion::is_url_like(reference) {
            transclusion::DirectiveKind::Url
        } else {
            transclusion::DirectiveKind::File
        };

        let transclusion::ResolvedTarget::File { path, .. } = transclusion::resolve_target(
            kind,
            reference,
            &transclusion_opts,
            &options.source,
            0,
            prepared_ctx.clone(),
        )?
        else {
            continue;
        };

        let child = Markdown::try_from(path.as_path())?;
        collect_recursive(
            &child,
            &options.clone().with_source_file(path),
            seen,
            entries,
            visited,
        )?;
    }

    Ok(())
}

/// Collects the frontmatter `$(...)` commands for a single document.
fn scan_one_frontmatter(
    markdown: &Markdown,
    options: &ComposeOptions,
    source_file: &std::path::Path,
    seen: &mut HashSet<String>,
    entries: &mut Vec<ShellCommandEntry>,
) -> MarkdownResult<()> {
    let mut fm_clone = markdown.clone();
    let pre_interpolation_snapshot = prepare_frontmatter_for_compose(&mut fm_clone, options, true);
    if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
        // Defer templated keys that reference a shell-pending (`$(...)`) value.
        // Without this, a key like `review: "{{ dir + '/x' }}"` resolves against
        // `dir`'s still-literal `$(...)` text and becomes `$(...)/x`, which then
        // trips `scan_frontmatter`'s "trailing content" guard. Deferral keeps it
        // as template text so only the real `$(...)` directive (`dir`) is scanned.
        //
        // Preflight only: shell-command discovery enumerates the reachable
        // pipelines for the approval workflow; it never performs expression
        // selection, so it stays context-free (no `ResolutionContext`). The real
        // run supplies the context.
        let _ = interpolate_frontmatter(
            fm_clone.frontmatter_mut(),
            options.context(),
            false,
            true,
            None,
        );
    }

    let scan_ctx = fm_clone.source_context_for_errors();
    let candidates = scan_frontmatter(
        fm_clone.frontmatter(),
        pre_interpolation_snapshot.as_ref(),
        &scan_ctx,
    )?;

    for candidate in candidates {
        // Walk every pipeline this directive could run — for a plain pipeline
        // there is exactly one; for a ternary, every non-empty branch
        // contributes a pipeline so both reachable command sets surface. The
        // legacy `executable`/`args` fields on the candidate are placeholders
        // for the ternary case and cannot be used directly.
        let pipelines = directive_reachable_pipelines(
            &candidate,
            fm_clone.frontmatter(),
            options,
            &scan_ctx,
        )?;

        for pipeline in pipelines {
            let executable = pipeline
                .actions
                .first()
                .map(|a| a.command.executable.clone())
                .unwrap_or_default();
            let args = pipeline
                .actions
                .first()
                .map(|a| a.command.args.clone())
                .unwrap_or_default();
            let raw_command = pipeline.display_string();

            let directive = ShellDirective {
                raw_command,
                executable,
                args,
                span: 0..0,
                indent: String::new(),
                origin: ShellCommandOrigin::Frontmatter {
                    key: candidate.key.clone(),
                },
                error_handling: Default::default(),
                timeout_override: candidate.timeout_override,
                // Collection only enumerates commands for approval; it never
                // executes, so the cache flag is irrelevant here.
                no_cache: false,
                pipeline: Some(pipeline),
                ctx: scan_ctx.clone(),
            };

            for (raw_action, exe_raw, args_raw) in directive_action_iter(&directive) {
                let (executable, args) = resolve_executable(&exe_raw, &args_raw);
                let normalized = normalize_command(&executable, &args);
                if seen.insert(normalized.clone()) {
                    entries.push(ShellCommandEntry {
                        raw_command: raw_action,
                        executable,
                        args,
                        normalized,
                        source_file: source_file.to_path_buf(),
                        origin: ShellCommandOrigin::Frontmatter {
                            key: candidate.key.clone(),
                        },
                    });
                }
            }
        }
    }

    Ok(())
}

/// Returns the `(key, "$(...)")` frontmatter values still pending
/// frontmatter-shell expansion after pass-1 interpolation.
///
/// These are the only values whose literal `$(...)` text can leak into a body
/// command via `{{ doc.key }}` interpolation, which is the dynamic-shape case.
fn pending_shell_literals(
    prepared: &Markdown,
    ctx: &biscuit_terminal::errors::SourceContext,
) -> Vec<(String, String)> {
    let mut pending = Vec::new();
    for (key, value) in prepared.frontmatter().as_map().iter() {
        if let serde_json::Value::String(s) = value
            && s.starts_with("$(")
            && matches!(parse_shell_value(s, key, None, ctx), Ok(Some(_)))
        {
            pending.push((key.clone(), s.clone()));
        }
    }
    pending
}

/// Fails with [`ShellExpansionError::DynamicCommandShape`] when a pending
/// frontmatter shell value has been interpolated into a body `::shell` directive
/// or `::shell-block` command.
fn detect_dynamic_command_shape(
    content: &str,
    pending: &[(String, String)],
    ctx: &biscuit_terminal::errors::SourceContext,
) -> MarkdownResult<()> {
    if pending.is_empty() {
        return Ok(());
    }

    let shell_block_spans: Vec<Range<usize>> = block_pairs::scan_block_pairs(content)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| matches!(p.kind, block_pairs::BlockOpenKind::Shell))
        .map(|p| p.body_span)
        .collect();

    for (key, literal) in pending {
        let mut from = 0usize;
        while let Some(rel) = content[from..].find(literal.as_str()) {
            let at = from + rel;
            from = at + literal.len();

            let line_start = content[..at].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line_end = content[at..]
                .find('\n')
                .map(|p| at + p)
                .unwrap_or(content.len());
            let line = strip_blockquote_prefix(content[line_start..line_end].trim());
            let in_shell_directive =
                line == "::shell" || line.starts_with("::shell ") || line.starts_with("::shell\t");
            let in_shell_block = shell_block_spans
                .iter()
                .any(|span| at >= span.start && at < span.end);

            if in_shell_directive || in_shell_block {
                let line_number = content[..at].matches('\n').count() + 1;
                return Err(ShellExpansionError::DynamicCommandShape {
                    ctx: Box::new(ctx.clone()),
                    command: line.to_string(),
                    key: key.clone(),
                    origin: ShellCommandOrigin::Body { line: line_number },
                }
                .into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::compose::ComposeOptions;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn discovers_shell_directives_in_simple_document() {
        let content = "# Test\n::shell echo hello\n::shell ls -la\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].executable, "echo");
        assert_eq!(entries[0].args, vec!["hello"]);
        assert_eq!(entries[1].executable, "ls");
        assert_eq!(entries[1].args, vec!["-la"]);
    }

    #[test]
    fn discovers_directives_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "::shell echo child").unwrap();

        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        let raw_commands: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw_commands.contains(&"echo root"));
        assert!(raw_commands.contains(&"echo child"));
    }

    #[test]
    fn deduplicates_by_normalized_form() {
        let content = "::shell echo hello\n::shell echo hello\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].normalized, "echo hello");
    }

    #[test]
    fn defers_keys_referencing_shell_pending_values() {
        let content = "---\ndir: \"$(dirname '{{spec}}')\"\nreview: \"{{ dir + '/review-' + iteration + '.md' }}\"\n---\nbody\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new().with_set_overrides(serde_json::json!({
            "spec": "features/rough-edges/spec.md",
            "iteration": "1",
        }));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].executable, "dirname");
        assert_eq!(entries[0].args, vec!["features/rough-edges/spec.md"]);
    }

    #[test]
    fn resolves_interpolated_variables_before_scanning() {
        let content = "---\ncmd_arg: world\n---\n::shell echo {{cmd_arg}}\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo world");
    }

    #[test]
    fn ignores_directives_in_code_blocks() {
        let content = "::shell echo outside\n\n```bash\n::shell echo inside\n```\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo outside");
    }

    #[test]
    fn empty_document_returns_empty_vec() {
        let md: Markdown = "# Just a heading\n\nSome text.\n".into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert!(entries.is_empty());
    }

    /// T1: approval is condition-blind — a command in a `when=`-false page block
    /// is still collected.
    #[test]
    fn includes_directives_inside_false_page_blocks() {
        let content = "\
---
include_shell: false
---
::shell echo always
::block when=\"include_shell\"
::shell echo conditional
::end-block
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo always"), "raw: {raw:?}");
        assert!(raw.contains(&"echo conditional"), "raw: {raw:?}");
    }

    /// T1: a command inside a `when=`-false page block nested in a shell block
    /// is still collected.
    #[test]
    fn includes_shell_block_commands_inside_false_page_blocks() {
        let content = "\
---
include_shell: false
---
::shell echo always
::block when=\"include_shell\"
::shell-block
echo conditional
::end-block
::end-block
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo always"), "raw: {raw:?}");
        assert!(raw.contains(&"echo conditional"), "raw: {raw:?}");
    }

    /// T1: a command in a false-condition transclusion is still collected.
    #[test]
    fn includes_frontmatter_shell_commands_in_false_transclusions() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(&child_path, "---\ncmd: \"$(echo hidden)\"\n---\n# Child\n").unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\ninclude_child: false\n---\n::file ./child.md when=\"include_child\"\n",
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].raw_command, "echo hidden");
    }

    /// T1: a body command in a false-condition transclusion is still collected.
    #[test]
    fn includes_body_shell_commands_in_false_transclusions() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(&child_path, "# Child\n::shell echo hidden-body\n").unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\ninclude_child: false\n---\n::file ./child.md when=\"include_child\"\n",
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].raw_command, "echo hidden-body");
    }

    #[test]
    fn discovers_directives_introduced_by_text_replacement() {
        let content = "\
---
replace:
  PLACEHOLDER: \"echo replaced\"
---
::shell PLACEHOLDER
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo replaced");
    }

    #[test]
    fn transclusion_provenance_attributes_to_child_file() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "# Child").unwrap();
        writeln!(child_file, "::shell echo from-child").unwrap();

        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo from-root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);

        let root_entry = entries
            .iter()
            .find(|e| e.raw_command == "echo from-root")
            .unwrap();
        assert_eq!(
            root_entry.source_file.canonicalize().unwrap(),
            root_path.canonicalize().unwrap()
        );

        let child_entry = entries
            .iter()
            .find(|e| e.raw_command == "echo from-child")
            .unwrap();
        assert_eq!(
            child_entry.source_file.canonicalize().unwrap(),
            child_path.canonicalize().unwrap()
        );
        assert_eq!(
            child_entry.origin,
            ShellCommandOrigin::Body { line: 2 },
            "line should be 2 in the child file"
        );
    }

    #[test]
    fn discovers_frontmatter_shell_commands() {
        let content = "---\nfiles: \"$(sniff repo dirty-files)\"\n---\n# Doc\n::shell echo body\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2, "entries: {entries:?}");
        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"sniff"), "Missing sniff: {executables:?}");
        assert!(executables.contains(&"echo"), "Missing echo: {executables:?}");
    }

    #[test]
    fn discovers_frontmatter_command_when_schema_constrains_the_value() {
        let content = "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\ntier: $(echo small)\n---\n# Doc\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].executable, "echo");
        assert_eq!(entries[0].raw_command, "echo small");
    }

    #[test]
    fn discovers_frontmatter_shell_commands_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(
            &child_path,
            "---\nchild_cmd: \"$(echo child-frontmatter)\"\n---\n# Child\n",
        )
        .unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(&root_path, "::file ./child.md\n").unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo child-frontmatter");
        assert_eq!(
            entries[0].source_file.canonicalize().unwrap(),
            child_path.canonicalize().unwrap()
        );
        assert_eq!(
            entries[0].origin,
            ShellCommandOrigin::Frontmatter {
                key: "child_cmd".to_string()
            }
        );
    }

    #[test]
    fn frontmatter_shell_commands_use_frontmatter_origin() {
        let content = "---\nfiles: \"$(echo fm-cmd)\"\n---\n# Doc\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        match &entries[0].origin {
            ShellCommandOrigin::Frontmatter { key } => assert_eq!(key, "files"),
            other => panic!("Expected Frontmatter origin, got: {other:?}"),
        }
    }

    #[test]
    fn frontmatter_and_body_commands_deduplicate() {
        let content = "---\nval: \"$(echo hello)\"\n---\n::shell echo hello\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn frontmatter_interpolation_resolves_before_discovery() {
        let content = "---\nname: world\ncmd: \"$(echo {{name}})\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo world");
    }

    #[test]
    fn discovery_uses_external_state_before_frontmatter_scan() {
        let content = "---\ncmd: \"{{tool}}\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new()
            .with_external_state(serde_json::json!({"tool": "$(echo external)"}));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo external");
    }

    #[test]
    fn discovery_uses_set_overrides_before_frontmatter_scan() {
        let content = "---\ncmd: \"$(echo before)\"\n---\n";
        let md: Markdown = content.into();
        let options =
            ComposeOptions::new().with_set_overrides(serde_json::json!({"cmd": "$(echo after)"}));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo after");
    }

    #[test]
    fn discovery_rejects_interpolated_frontmatter_executable() {
        let content = "---\ncmd_name: echo\ncmd: \"$({{cmd_name}} hello)\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        assert!(
            err.to_string()
                .contains("Frontmatter shell executable may not come from interpolation")
        );
    }

    /// T10: a body command embedding a shell-pending frontmatter value is a hard
    /// pre-flight error, not a late `NotPreApproved`.
    #[test]
    fn dynamic_command_shape_in_body_directive_is_rejected() {
        let content = "---\nbranch: $(git branch --show-current)\n---\n::shell git log {{ doc.branch }} -1\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("depends on frontmatter key 'branch'"),
            "unexpected error: {msg}"
        );
        assert!(
            !msg.contains("not pre-approved"),
            "must not surface as NotPreApproved: {msg}"
        );
    }

    /// A pending frontmatter value referenced only as prose content is fine;
    /// dynamic-shape detection is scoped to shell directives.
    #[test]
    fn dynamic_value_in_prose_is_not_rejected() {
        let content = "---\nbranch: $(git branch --show-current)\n---\nThe branch is {{ doc.branch }}.\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        // Only the frontmatter command itself is collected.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].executable, "git");
    }

    #[test]
    fn body_chain_emits_one_entry_per_action() {
        let content = "::shell echo ok && pwd || ls\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo ok"), "missing echo ok: {raw:?}");
        assert!(raw.contains(&"pwd"), "missing pwd: {raw:?}");
        assert!(raw.contains(&"ls"), "missing ls: {raw:?}");
    }

    #[test]
    fn frontmatter_chain_emits_one_entry_per_action() {
        let content = "---\nfiles: \"$(echo first || pwd)\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"), "missing echo: {executables:?}");
        assert!(executables.contains(&"pwd"), "missing pwd: {executables:?}");
    }

    #[test]
    fn body_chain_with_redirection_emits_per_action_entries() {
        let content = "::shell echo silent > /dev/null && pwd\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"));
        assert!(executables.contains(&"pwd"));
    }

    #[test]
    fn discovers_shell_block_commands() {
        let content = "::shell-block\necho hello\necho world\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].raw_command, "echo hello");
        assert_eq!(entries[1].raw_command, "echo world");
        assert_eq!(
            entries[0].origin,
            ShellCommandOrigin::ShellBlock {
                start_line: 1,
                command_line: 2,
            }
        );
    }

    #[test]
    fn shell_block_chain_emits_one_entry_per_action() {
        let content = "::shell-block\necho ok && pwd || ls\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo ok"), "missing echo ok: {raw:?}");
        assert!(raw.contains(&"pwd"), "missing pwd: {raw:?}");
        assert!(raw.contains(&"ls"), "missing ls: {raw:?}");

        let blocks: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.origin, ShellCommandOrigin::ShellBlock { .. }))
            .collect();
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn discovers_mixed_shell_and_shell_block_commands() {
        let content = "::shell echo standalone\n::shell-block\necho block\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        let standalones: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.origin, ShellCommandOrigin::Body { .. }))
            .collect();
        let blocks: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.origin, ShellCommandOrigin::ShellBlock { .. }))
            .collect();
        assert_eq!(standalones.len(), 1);
        assert_eq!(blocks.len(), 1);
        assert_eq!(standalones[0].raw_command, "echo standalone");
        assert_eq!(blocks[0].raw_command, "echo block");
    }

    #[test]
    fn shell_block_commands_deduplicate_with_shell_directives() {
        let content = "::shell echo hello\n::shell-block\necho hello\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
    }

    /// T1: both branches of a frontmatter ternary surface, condition-blind.
    #[test]
    fn frontmatter_ternary_emits_both_branch_commands() {
        let content = "\
---
flag: true
out: \"$(flag ? echo yes : basename README.md)\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo yes"), "missing echo yes: {raw:?}");
        assert!(
            raw.contains(&"basename README.md"),
            "missing basename README.md: {raw:?}"
        );
    }

    #[test]
    fn frontmatter_ternary_empty_branch_emits_nothing() {
        let content = "\
---
flag: true
out: \"$(flag ? echo only : '')\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo only");
    }

    #[test]
    fn frontmatter_ternary_branch_chain_emits_one_entry_per_action() {
        let content = "\
---
flag: true
out: \"$(flag ? echo a && pwd : ls)\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"), "missing echo: {executables:?}");
        assert!(executables.contains(&"pwd"), "missing pwd: {executables:?}");
        assert!(executables.contains(&"ls"), "missing ls: {executables:?}");
    }

    #[test]
    fn frontmatter_ternary_branch_arguments_are_interpolated() {
        let content = "\
---
flag: true
name: README.md
out: \"$(flag ? basename {{name}} : '')\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].executable, "basename");
        assert_eq!(entries[0].args, vec!["README.md"]);
    }

    #[test]
    fn frontmatter_ternary_branch_rejects_interpolated_executable() {
        let content = "\
---
flag: true
cmd_name: echo
out: \"$(flag ? {{cmd_name}} hi : '')\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        assert!(
            err.to_string().contains("may not come from interpolation"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn discovers_shell_block_commands_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "# Child").unwrap();
        writeln!(child_file, "::shell-block").unwrap();
        writeln!(child_file, "echo from-child").unwrap();
        writeln!(child_file, "::end-block").unwrap();

        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo from-root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);

        let child_entry = entries
            .iter()
            .find(|e| e.raw_command == "echo from-child")
            .unwrap();
        assert_eq!(
            child_entry.source_file.canonicalize().unwrap(),
            child_path.canonicalize().unwrap()
        );
        match &child_entry.origin {
            ShellCommandOrigin::ShellBlock {
                start_line,
                command_line,
            } => {
                assert_eq!(*start_line, 2);
                assert_eq!(*command_line, 3);
            }
            other => panic!("Expected ShellBlock origin, got: {other:?}"),
        }
    }
}
