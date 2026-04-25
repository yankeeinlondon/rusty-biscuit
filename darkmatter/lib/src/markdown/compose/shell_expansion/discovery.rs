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
use crate::markdown::compose::frontmatter_shell_expansion::scan_frontmatter;
use crate::markdown::compose::prepare_frontmatter_for_compose;
use crate::markdown::compose::shell_expansion::alias::resolve_alias;
use crate::markdown::compose::shell_expansion::parser::parse_directives;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::types::{ShellCommandEntry, ShellCommandOrigin};
use crate::markdown::compose::state;
use crate::markdown::compose::transclusion;
use crate::markdown::compose::types::SourceRange;
use crate::markdown::types::MarkdownResult;

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

    let discovery_options = options.clone().only(&discovery_ops);

    let (composed, report) = markdown.compose_with(discovery_options)?;

    // Parse ::shell directives from the fully-resolved content.
    // ShellExpansionError converts into MarkdownError via From impl.
    let directives = parse_directives(composed.content())?;

    for directive in directives {
        let (executable, args) = if which::which(&directive.executable).is_ok() {
            (directive.executable.clone(), directive.args.clone())
        } else if let Some(resolved) = resolve_alias(&directive.executable) {
            let mut merged_args = resolved.args;
            merged_args.extend_from_slice(&directive.args);
            (resolved.executable, merged_args)
        } else {
            (directive.executable.clone(), directive.args.clone())
        };

        let normalized = normalize_command(&executable, &args);

        if seen.insert(normalized.clone()) {
            // Look up provenance from the source map
            let (source_file, line) = lookup_provenance(
                directive.span.start,
                directive.origin.line_number(),
                &report.source_map,
                composed.content(),
                &default_source,
            );

            entries.push(ShellCommandEntry {
                raw_command: directive.raw_command,
                executable,
                args,
                normalized,
                source_file,
                origin: ShellCommandOrigin::Body { line },
            });
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

    let (prepared, _) = markdown.compose_with(options.clone().only(&inline_ops))?;
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

    for directive in transclusion::parse_directives(prepared.content())? {
        if directive.kind != transclusion::DirectiveKind::File {
            continue;
        }

        if let Some(expr) = &directive.options.when_expr
            && !transclusion::evaluate_condition(expr, &state, directive.line)?
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

    let refs = transclusion::parse_frontmatter_refs(prepared.frontmatter().as_map())?;
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

        let transclusion::ResolvedTarget::File { path, .. } =
            transclusion::resolve_target(kind, reference, &transclusion_opts, &options.source, 0)?
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
        let _ = interpolate_frontmatter(fm_clone.frontmatter_mut(), options.context(), false);
    }

    let candidates = scan_frontmatter(fm_clone.frontmatter(), pre_interpolation_snapshot.as_ref())?;

    for candidate in candidates {
        let (executable, args) = if which::which(&candidate.executable).is_ok() {
            (candidate.executable.clone(), candidate.args.clone())
        } else if let Some(resolved) = resolve_alias(&candidate.executable) {
            let mut merged_args = resolved.args;
            merged_args.extend_from_slice(&candidate.args);
            (resolved.executable, merged_args)
        } else {
            (candidate.executable.clone(), candidate.args.clone())
        };

        let normalized = normalize_command(&executable, &args);

        if seen.insert(normalized.clone()) {
            entries.push(ShellCommandEntry {
                raw_command: candidate.raw_command,
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
}
