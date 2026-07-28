//! Interactive shell command approval handler for the CLI.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use std::io::{self, BufRead, IsTerminal, Write};

/// CLI approval handler that prompts the user interactively on stderr.
pub struct CliShellApprovalHandler;

impl ShellApprovalHandler for CliShellApprovalHandler {
    #[allow(clippy::result_large_err)]
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        let stderr = io::stderr();
        let stdin = io::stdin();
        let mut stderr = stderr.lock();
        let mut stdin = stdin.lock();

        approve_with_io(&request, &mut stdin, &mut stderr)
    }
}

#[allow(clippy::result_large_err)]
fn approve_with_io<R: BufRead, W: Write>(
    request: &ShellApprovalRequest,
    input: &mut R,
    output: &mut W,
) -> Result<ShellApprovalDecision, ShellExpansionError> {
    let source_desc = match &request.source {
        darkmatter::markdown::compose::ComposeSource::File(p) => p.display().to_string(),
        darkmatter::markdown::compose::ComposeSource::Url(u) => u.to_string(),
        darkmatter::markdown::compose::ComposeSource::Unknown => "<stdin>".to_string(),
    };

    loop {
        write_prompt(output, request, &source_desc)?;

        let mut choice = String::new();
        input
            .read_line(&mut choice)
            .map_err(|source| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source,
            })?;

        match choice.trim() {
            "1" => return Ok(ShellApprovalDecision::AllowExactPersist),
            "2" => return Ok(ShellApprovalDecision::AllowCommandPersist),
            "3" => return Ok(ShellApprovalDecision::AllowOnce),
            "4" => return Ok(ShellApprovalDecision::Deny),
            "5" => return Ok(ShellApprovalDecision::BlacklistPersist),
            _ => {
                let err_msg = Prose::new("<red>Invalid choice. Please enter 1-5.</red>")
                    .render_optimistic(None);
                writeln!(output, "  {err_msg}").map_err(|source| {
                    ShellExpansionError::PolicyIo {
                        path: request.whitelist_path.clone(),
                        source,
                    }
                })?;
            }
        }
    }
}

#[allow(clippy::result_large_err)]
fn write_prompt<W: Write>(
    output: &mut W,
    request: &ShellApprovalRequest,
    source_desc: &str,
) -> Result<(), ShellExpansionError> {
    let p = &request.whitelist_path;
    let w = |output: &mut W, args: std::fmt::Arguments| -> Result<(), ShellExpansionError> {
        output
            .write_fmt(args)
            .map_err(|source| ShellExpansionError::PolicyIo {
                path: p.to_path_buf(),
                source,
            })
    };

    let header = Prose::new("\u{26a0}  <bold><yellow>Shell Approval Required</yellow></bold>")
        .render_optimistic(None);
    let source_label = Prose::new("<dim>Source:</dim>").render_optimistic(None);
    let source_value = Prose::new(format!("<bold>{source_desc}:{}</bold>", request.origin))
        .render_optimistic(None);
    let cmd_label = Prose::new("<dim>Command:</dim>").render_optimistic(None);
    let cmd_value = Prose::new(format!(
        "<bold><cyan>{}</cyan></bold>",
        escape_prose(&request.raw_command)
    ))
    .render_optimistic(None);

    w(output, format_args!("\n  {header}\n"))?;
    w(output, format_args!("  {source_label}  {source_value}\n"))?;

    w(output, format_args!("  {cmd_label} {cmd_value}\n\n"))?;

    // Options with color-coded numbers and dim descriptions
    let opt1_num = Prose::new("<green>1</green>").render_optimistic(None);
    let opt1_desc = Prose::new(format!(
        "<dim>(persists \"{}\" to whitelist)</dim>",
        escape_prose(&request.raw_command)
    ))
    .render_optimistic(None);
    let opt2_num = Prose::new("<green>2</green>").render_optimistic(None);
    let opt2_desc = if request.chain_executables.is_empty() {
        Prose::new(format!(
            "<dim>(persists \"{}\" with any args to whitelist)</dim>",
            escape_prose(&request.executable)
        ))
        .render_optimistic(None)
    } else {
        let executables_list = request
            .chain_executables
            .iter()
            .map(|exe| format!("\"{}\"", escape_prose(exe)))
            .collect::<Vec<_>>()
            .join(", ");
        Prose::new(format!(
            "<dim>(persists {executables_list} with any args to whitelist)</dim>"
        ))
        .render_optimistic(None)
    };
    let opt3_num = Prose::new("<cyan>3</cyan>").render_optimistic(None);
    let opt3_desc = Prose::new("<dim>(this session only)</dim>").render_optimistic(None);
    let opt4_num = Prose::new("<yellow>4</yellow>").render_optimistic(None);
    let opt5_num = Prose::new("<red>5</red>").render_optimistic(None);
    let opt5_desc = Prose::new("<dim>(persists to blacklist)</dim>").render_optimistic(None);

    let sep = Prose::new("<dim>\u{2502}</dim>").render_optimistic(None);

    w(
        output,
        format_args!("  {opt1_num} {sep} Allow exact and save    {opt1_desc}\n"),
    )?;
    w(
        output,
        format_args!("  {opt2_num} {sep} Allow command and save  {opt2_desc}\n"),
    )?;
    w(
        output,
        format_args!("  {opt3_num} {sep} Allow once              {opt3_desc}\n"),
    )?;
    w(output, format_args!("  {opt4_num} {sep} Deny\n"))?;
    w(
        output,
        format_args!("  {opt5_num} {sep} Blacklist and stop      {opt5_desc}\n\n"),
    )?;

    let prompt_arrow = Prose::new("<bold>></bold>").render_optimistic(None);
    w(output, format_args!("  {prompt_arrow} "))?;

    output
        .flush()
        .map_err(|source| ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        })
}

/// Escapes angle brackets in user-provided text to prevent Prose tag interpretation.
fn escape_prose(text: &str) -> String {
    text.replace('<', "\\<").replace('>', "\\>")
}

/// Returns true if interactive prompting is safe.
///
/// Prompting is safe when:
/// 1. stdin is a terminal
/// 2. stderr is a terminal
pub fn can_prompt_interactively() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::ComposeSource;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn request() -> ShellApprovalRequest {
        use darkmatter::markdown::compose::ShellCommandOrigin;
        ShellApprovalRequest {
            source: ComposeSource::File(PathBuf::from("/tmp/doc.md")),
            origin: ShellCommandOrigin::Body { line: 12 },
            raw_command: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
            normalized_exact: "echo hello".to_string(),
            whitelist_path: PathBuf::from("/tmp/.darkmatter-shell-whitelist"),
            blacklist_path: PathBuf::from("/tmp/.darkmatter-shell-blacklist"),
            chain_executables: Vec::new(),
        }
    }

    /// Strips ANSI escape sequences for assertion clarity.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for inner in chars.by_ref() {
                    if inner == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn prompt_accepts_valid_choice() {
        let request = request();
        let mut input = Cursor::new(b"2\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&request, &mut input, &mut output).unwrap();

        assert_eq!(decision, ShellApprovalDecision::AllowCommandPersist);
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("Shell Approval Required"));
        assert!(prompt.contains("/tmp/doc.md:line 12"));
        assert!(prompt.contains("echo hello"));
    }

    #[test]
    fn prompt_retries_after_invalid_choice() {
        let request = request();
        let mut input = Cursor::new(b"9\n3\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&request, &mut input, &mut output).unwrap();

        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("Invalid choice. Please enter 1-5."));
        // Prompt should appear twice (once for invalid, once for retry)
        assert_eq!(prompt.matches("Shell Approval Required").count(), 2);
    }

    #[test]
    fn all_five_choices_produce_correct_decisions() {
        let cases = [
            ("1\n", ShellApprovalDecision::AllowExactPersist),
            ("2\n", ShellApprovalDecision::AllowCommandPersist),
            ("3\n", ShellApprovalDecision::AllowOnce),
            ("4\n", ShellApprovalDecision::Deny),
            ("5\n", ShellApprovalDecision::BlacklistPersist),
        ];

        for (input_str, expected) in cases {
            let request = request();
            let mut input = Cursor::new(input_str.as_bytes().to_vec());
            let mut output = Vec::new();

            let decision = approve_with_io(&request, &mut input, &mut output).unwrap();
            assert_eq!(
                decision,
                expected,
                "Input '{}' should produce {:?}",
                input_str.trim(),
                expected
            );
        }
    }

    #[test]
    fn prompt_shows_source_and_command() {
        let request = request();
        let mut input = Cursor::new(b"1\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());

        assert!(prompt.contains("Source:"));
        assert!(prompt.contains("Command:"));
        assert!(prompt.contains("Allow exact and save"));
        assert!(prompt.contains("Allow command and save"));
        assert!(prompt.contains("Allow once"));
        assert!(prompt.contains("Deny"));
        assert!(prompt.contains("Blacklist and stop"));
    }

    #[test]
    fn prompt_lists_chain_executables_for_chain_request() {
        let mut request = request();
        request.chain_executables = vec!["echo".to_string(), "pwd".to_string()];
        request.raw_command = "echo hello && pwd".to_string();
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());

        // Both executables should appear in the option-2 description.
        assert!(
            prompt.contains("\"echo\""),
            "expected echo in prompt: {prompt}"
        );
        assert!(
            prompt.contains("\"pwd\""),
            "expected pwd in prompt: {prompt}"
        );
    }

    #[test]
    fn prompt_shows_url_source() {
        let mut request = request();
        request.source = ComposeSource::Url("https://example.com/doc.md".parse().unwrap());
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("https://example.com/doc.md"));
    }

    #[test]
    fn prompt_shows_stdin_source() {
        let mut request = request();
        request.source = ComposeSource::Unknown;
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("<stdin>"));
    }
}
