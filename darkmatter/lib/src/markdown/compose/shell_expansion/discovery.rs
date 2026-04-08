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
use crate::markdown::compose::shell_expansion::alias::resolve_alias;
use crate::markdown::compose::shell_expansion::parser::parse_directives;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::types::{ShellCommandEntry, ShellCommandOrigin};
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
    // Run compose with only interpolation + transclusion (no shell execution).
    // ::shell directives remain as text in the composed output.
    let discovery_options = options.clone().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::FrontmatterTransclusion,
    ]);

    let (composed, report) = markdown.compose_with(discovery_options)?;

    // Parse ::shell directives from the fully-resolved content.
    // ShellExpansionError converts into MarkdownError via From impl.
    let directives = parse_directives(composed.content())?;

    let default_source = match &options.source {
        ComposeSource::File(p) => p.clone(),
        _ => PathBuf::from("<unknown>"),
    };

    // Build entries, resolving aliases and deduplicating by normalized form.
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

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
        assert_eq!(child_entry.origin, ShellCommandOrigin::Body { line: 2 }, "line should be 2 in the child file");
    }
}
