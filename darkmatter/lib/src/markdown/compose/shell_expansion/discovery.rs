//! Shell command discovery across the document graph.
//!
//! Walks transclusions and resolves interpolation to find every `::shell`
//! directive that would be executed during composition. Returns entries
//! without executing or approving anything.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::ComposeOperation;
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::shell_expansion::alias::resolve_alias;
use crate::markdown::compose::shell_expansion::parser::parse_directives;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::types::ShellCommandEntry;
use crate::markdown::types::MarkdownResult;

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
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::FrontmatterTransclusion,
    ]);

    let (composed, _report) = markdown.compose_with(discovery_options)?;

    // Parse ::shell directives from the fully-resolved content.
    // ShellExpansionError converts into MarkdownError via From impl.
    let directives = parse_directives(composed.content())?;

    let source_file = match &options.source {
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
            entries.push(ShellCommandEntry {
                raw_command: directive.raw_command,
                executable,
                args,
                normalized,
                source_file: source_file.clone(),
                line: directive.line,
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
}
