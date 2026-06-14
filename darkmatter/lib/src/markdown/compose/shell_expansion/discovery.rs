//! Shell command discovery across the document graph.
//!
//! Walks transclusions and resolves interpolation to find every `::shell`
//! directive that would be executed during composition. Returns entries
//! without executing or approving anything.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOperation;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::EffectiveStateBuilder;
use crate::markdown::compose::frontmatter_interpolation::interpolate_frontmatter;
use crate::markdown::compose::frontmatter_shell_expansion::{
    directive_reachable_pipelines, scan_frontmatter,
};
use crate::markdown::compose::prepare_frontmatter_for_compose;
use crate::markdown::compose::remote_fetch;
use crate::markdown::compose::shell_expansion::alias::resolve_alias;
use crate::markdown::compose::shell_expansion::parser::parse_directives;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::types::{
    ShellCommandEntry, ShellCommandOrigin, ShellDirective,
};
use crate::markdown::compose::state;
use crate::markdown::compose::transclusion;
use crate::markdown::compose::types::SourceRange;
use crate::markdown::types::MarkdownResult;

use super::super::block_pairs;
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

/// Looks up the originating source file for a byte position in composed output.
///
/// Checks the source map for a range containing `byte_pos`. If found, computes
/// the line within the transcluded region by counting newlines. Otherwise falls
/// back to the root document.
fn lookup_provenance(
    byte_pos: usize,
    composed_line: usize,
    source_map: &[SourceRange],
    composed_content: &str,
    default_source: &std::path::Path,
) -> (PathBuf, usize) {
    for range in source_map {
        if byte_pos >= range.byte_start && byte_pos < range.byte_end {
            // Count newlines from the start of the transcluded region to the
            // directive position to get the line number within the source file.
            let region = &composed_content[range.byte_start..byte_pos];
            let relative_line = region.chars().filter(|&c| c == '\n').count();
            return (
                range.source_file.clone(),
                range.source_start_line + relative_line,
            );
        }
    }
    (default_source.to_path_buf(), composed_line)
}

/// Walks the full document graph and returns every `::shell` directive found.
///
/// Runs interpolation and transclusion using the provided `ComposeOptions`
/// state so that template variables and dynamic transclusion paths resolve
/// identically to how they would during `compose_with()`.
///
/// Shell directives remain as text in the composed output because
/// `ShellExpansion` is deliberately not enabled. The function then parses
/// them from the fully-resolved content.
///
/// No approval checks, no whitelist lookups, no execution.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::compose::ComposeOptions;
/// use darkmatter::markdown::compose::shell_expansion::discovery::collect_shell_commands;
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
/// Returns an error if the compose pipeline (interpolation + transclusion)
/// fails, or if `::shell` directive parsing encounters invalid syntax.
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<Vec<ShellCommandEntry>> {
    let default_source = match &options.source {
        ComposeSource::File(p) => p.clone(),
        _ => PathBuf::from("<unknown>"),
    };

    // Build entries, resolving aliases and deduplicating by normalized form.
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    // ── Phase 1: Discover frontmatter shell commands ───────────────
    // Frontmatter commands execute before each document's body is
    // transcluded, so walk markdown transclusions recursively and scan each
    // child's frontmatter without enabling FrontmatterShellExpansion.
    let mut visited_frontmatter = HashSet::new();
    collect_frontmatter_commands_recursive(
        markdown,
        options,
        &mut seen,
        &mut entries,
        &mut visited_frontmatter,
    )?;

    // ── Phase 2: Discover body shell commands ──────────────────────
    // Run compose with only interpolation + transclusion (no shell execution).
    // ::shell directives remain as text in the composed output.
    let discovery_ops: Vec<_> = [
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::FrontmatterTransclusion,
    ]
    .into_iter()
    .filter(|op| options.is_enabled(*op))
    .collect();

    // Discovery is a non-terminal pass: it strips FrontmatterShellExpansion
    // to avoid executing commands, so schema validation here would judge
    // still-literal `$(...)` values as final violations. Skip it — the
    // terminal compose pass validates the resolved frontmatter.
    let mut discovery_options = options.clone().only(&discovery_ops);
    discovery_options.skip_schema_validation = true;

    let (composed, report) = markdown.compose_with(discovery_options)?;

    // Parse ::shell directives from the fully-resolved content.
    // ShellExpansionError converts into MarkdownError via From impl.
    let directives = parse_directives(composed.content(), composed.source_context_for_errors())?;

    for directive in directives {
        // Look up provenance from the source map (shared by every action)
        let (source_file, line) = lookup_provenance(
            directive.span.start,
            directive.origin.line_number(),
            &report.source_map,
            composed.content(),
            &default_source,
        );

        for (raw_action, exe_raw, args_raw) in directive_action_iter(&directive) {
            let (executable, args) = if which::which(&exe_raw).is_ok() {
                (exe_raw.clone(), args_raw.clone())
            } else if let Some(resolved) = resolve_alias(&exe_raw) {
                let mut merged_args = resolved.args;
                merged_args.extend_from_slice(&args_raw);
                (resolved.executable, merged_args)
            } else {
                (exe_raw.clone(), args_raw.clone())
            };

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

    // Discover shell-block commands from the fully-resolved content.
    let block_pairs = block_pairs::scan_block_pairs(composed.content())
        .map_err(|e| crate::markdown::types::MarkdownError::Transform(e.to_string()))?;

    for pair in block_pairs {
        if !matches!(pair.kind, block_pairs::BlockOpenKind::Shell) {
            continue;
        }

        let body_text = &composed.content()[pair.body_span.clone()];
        let commands = split_logical_commands(body_text, pair.start_line + 1)
            .map_err(|e| crate::markdown::types::MarkdownError::Transform(e.to_string()))?;

        for command in commands {
            let (source_file, command_line) = lookup_provenance(
                pair.body_span.start + command.physical_span.start,
                command.start_line,
                &report.source_map,
                composed.content(),
                &default_source,
            );

            let (_, start_line) = lookup_provenance(
                pair.span.start,
                pair.start_line,
                &report.source_map,
                composed.content(),
                &default_source,
            );

            for action in &command.pipeline.actions {
                let exe_raw = action.command.executable.clone();
                let args_raw = action.command.args.clone();
                let raw_command = render_action(&exe_raw, &args_raw);

                let (executable, args) = if which::which(&exe_raw).is_ok() {
                    (exe_raw.clone(), args_raw.clone())
                } else if let Some(resolved) = resolve_alias(&exe_raw) {
                    let mut merged_args = resolved.args;
                    merged_args.extend_from_slice(&args_raw);
                    (resolved.executable, merged_args)
                } else {
                    (exe_raw.clone(), args_raw.clone())
                };

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
                        start_line,
                        command_line,
                    },
                });
            }
        }
    }

    Ok(entries)
}

fn collect_frontmatter_commands_recursive(
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

    scan_one_frontmatter(markdown, options, &source_file, seen, entries)?;

    let inline_ops: Vec<_> = [
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
    ]
    .into_iter()
    .filter(|op| options.is_enabled(*op))
    .collect();

    // Non-terminal pass (see Phase 2 below): skip schema validation so a
    // still-literal `$(...)` value is not judged as a final violation.
    let mut inline_options = options.clone().only(&inline_ops);
    inline_options.skip_schema_validation = true;
    let (prepared, _) = markdown.compose_with(inline_options)?;
    let transclusion_opts = options.transclusion_options();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(
            prepared
                .frontmatter()
                .as_map()
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
        .with_external_state(
            options
                .external_state
                .clone()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        )
        .with_merge_strategy(crate::markdown::MergeStrategy::PreferDocument)
        .with_replace_parent_wins(options.replace_parent_wins)
        .with_context(options.context().clone())
        .with_allow_ctx_override(options.allow_ctx_override)
        .build()?;

    let prepared_ctx = prepared.source_context_for_errors();
    let remote_fetch =
        remote_fetch::RemoteFetchRuntime::with_store(&options.remote_read_config, None);
    let lookup =
        state::ResolvingLookup::new(&state, options.expression_resolution_context(&remote_fetch));
    for directive in transclusion::parse_directives(prepared.content(), prepared_ctx.clone())? {
        if directive.kind != transclusion::DirectiveKind::File {
            continue;
        }

        if let Some(expr) = &directive.options.when_expr
            && !transclusion::evaluate_condition(
                expr,
                &lookup,
                directive.line,
                prepared_ctx.clone(),
            )?
        {
            continue;
        }

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

        collect_frontmatter_commands_recursive(
            &child,
            &options.clone().with_source_file(path),
            seen,
            entries,
            visited,
        )?;
    }

    let refs = transclusion::parse_frontmatter_refs(
        prepared.frontmatter().as_map(),
        prepared_ctx.clone(),
    )?;
    for reference in refs.prologue.iter().chain(refs.epilogue.iter()) {
        if !transclusion::is_url_like(reference) && !transclusion::is_file_like_reference(reference)
        {
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
        collect_frontmatter_commands_recursive(
            &child,
            &options.clone().with_source_file(path),
            seen,
            entries,
            visited,
        )?;
    }

    Ok(())
}

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
        let _ =
            interpolate_frontmatter(fm_clone.frontmatter_mut(), options.context(), false, false);
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
        // contributes a pipeline so both reachable command sets surface in
        // discovery. The legacy `executable`/`args` fields on the candidate
        // are placeholders for the ternary case (see `parse_shell_value`)
        // and cannot be used directly.
        let pipelines =
            directive_reachable_pipelines(&candidate, fm_clone.frontmatter(), options, &scan_ctx)?;

        for pipeline in pipelines {
            // Build a synthetic per-pipeline ShellDirective so we can reuse the
            // existing chain expander uniformly with the non-ternary path.
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
                pipeline: Some(pipeline),
                ctx: scan_ctx.clone(),
            };

            for (raw_action, exe_raw, args_raw) in directive_action_iter(&directive) {
                let (executable, args) = if which::which(&exe_raw).is_ok() {
                    (exe_raw.clone(), args_raw.clone())
                } else if let Some(resolved) = resolve_alias(&exe_raw) {
                    let mut merged_args = resolved.args;
                    merged_args.extend_from_slice(&args_raw);
                    (resolved.executable, merged_args)
                } else {
                    (exe_raw.clone(), args_raw.clone())
                };

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

        // Write child document with a shell directive.
        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "::shell echo child").unwrap();

        // Write root document that transcludes the child and has its own directive.
        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"));
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

    #[test]
    fn excludes_directives_inside_false_page_blocks() {
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

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo always");
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

        // Child document with a shell directive
        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "# Child").unwrap();
        writeln!(child_file, "::shell echo from-child").unwrap();

        // Root document with its own directive and a transclusion
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
        // Canonicalize both paths to handle symlinks like /var -> /private/var on macOS
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
        // Line 2 in child.md (line 1 is "# Child", line 2 is "::shell echo from-child")
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

        assert_eq!(entries.len(), 2, "entries: {:?}", entries);
        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(
            executables.contains(&"sniff"),
            "Missing sniff: {:?}",
            executables
        );
        assert!(
            executables.contains(&"echo"),
            "Missing echo: {:?}",
            executables
        );
    }

    #[test]
    fn discovers_frontmatter_command_when_schema_constrains_the_value() {
        // Regression: a `$schema`-constrained frontmatter value that holds a
        // `$(...)` expression must not abort discovery. Discovery composes
        // with FrontmatterShellExpansion stripped (so the command is not yet
        // executed), but schema validation is irrelevant to command
        // collection and must not treat the still-literal `$(...)` value as a
        // final schema violation — the real compose pass (with shell
        // expansion) validates the resolved value downstream.
        let content = "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\ntier: $(echo small)\n---\n# Doc\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {:?}", entries);
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
    fn skips_frontmatter_shell_commands_in_false_transclusions() {
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

        assert!(entries.is_empty(), "entries: {:?}", entries);
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
            other => panic!("Expected Frontmatter origin, got: {:?}", other),
        }
    }

    #[test]
    fn frontmatter_and_body_commands_deduplicate() {
        // Same command in both frontmatter and body — should only appear once
        let content = "---\nval: \"$(echo hello)\"\n---\n::shell echo hello\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        // Deduplicated by normalized form
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

    /// Body chains report every action so `md compose --shell` mirrors the
    /// approval/execution surface. Regression for review-3.
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

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"));
        assert!(executables.contains(&"pwd"));
        assert!(executables.contains(&"ls"));
    }

    /// Frontmatter `$()` chains also expand into per-action entries.
    #[test]
    fn frontmatter_chain_emits_one_entry_per_action() {
        let content = "---\nfiles: \"$(echo first || pwd)\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(
            executables.contains(&"echo"),
            "missing echo: {executables:?}"
        );
        assert!(executables.contains(&"pwd"), "missing pwd: {executables:?}");
    }

    /// Body chains with redirection still emit a per-action entry, and the
    /// redirected action is preserved as a separate entry.
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
        // Same command in both ::shell and ::shell-block — should only appear once
        let content = "::shell echo hello\n::shell-block\necho hello\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        // Deduplicated by normalized form
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn excludes_shell_block_commands_inside_false_page_blocks() {
        let content = "\
---
include_shell: false
---
::shell echo always
::block when=\"include_shell\"
::shell-block
::shell echo conditional
::end-block
::end-block
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo always");
    }

    /// Review-3 high finding: both branches of a frontmatter ternary must
    /// surface in discovery so the allowlist/approval workflow covers every
    /// reachable command — not just the branch the runtime condition would
    /// select.
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

    /// An empty branch contributes no shell entry — the literal `''`
    /// short-circuits to `""` at runtime and runs no command.
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

    /// A ternary branch that contains a `&&` / `||` chain emits one entry
    /// per chained action, mirroring the per-action expansion already
    /// applied to bare frontmatter pipelines.
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
        assert!(
            executables.contains(&"echo"),
            "missing echo: {executables:?}"
        );
        assert!(executables.contains(&"pwd"), "missing pwd: {executables:?}");
        assert!(executables.contains(&"ls"), "missing ls: {executables:?}");
    }

    /// Interpolated argument values inside a ternary branch must be
    /// resolved against frontmatter state before discovery emits the
    /// entry, so the allowlist sees the final argument shape.
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

    /// Executable-position interpolation inside a ternary branch is still
    /// rejected — discovery refuses to materialize a directive whose
    /// reachable executable name is not statically determinable.
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

        // Child document with a shell block
        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "# Child").unwrap();
        writeln!(child_file, "::shell-block").unwrap();
        writeln!(child_file, "echo from-child").unwrap();
        writeln!(child_file, "::end-block").unwrap();

        // Root document with its own directive and a transclusion
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
        match &child_entry.origin {
            ShellCommandOrigin::ShellBlock {
                start_line,
                command_line,
            } => {
                // In child.md: line 2 is "::shell-block", line 3 is "echo from-child"
                assert_eq!(*start_line, 2);
                assert_eq!(*command_line, 3);
            }
            other => panic!("Expected ShellBlock origin, got: {:?}", other),
        }
    }
}
