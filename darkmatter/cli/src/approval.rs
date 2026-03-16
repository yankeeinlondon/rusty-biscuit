//! Interactive shell command approval handler for the CLI.

use darkmatter::markdown::transform::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use std::io::{self, BufRead, IsTerminal, Write};

/// CLI approval handler that prompts the user interactively on stderr.
pub struct CliShellApprovalHandler;

impl ShellApprovalHandler for CliShellApprovalHandler {
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

fn approve_with_io<R: BufRead, W: Write>(
    request: &ShellApprovalRequest,
    input: &mut R,
    output: &mut W,
) -> Result<ShellApprovalDecision, ShellExpansionError> {
    let source_desc = match &request.source {
        darkmatter::markdown::transform::TransformSource::File(p) => p.display().to_string(),
        darkmatter::markdown::transform::TransformSource::Url(u) => u.to_string(),
        darkmatter::markdown::transform::TransformSource::Unknown => "<stdin>".to_string(),
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
            _ => writeln!(output, "Invalid choice. Please enter 1-5.").map_err(|source| {
                ShellExpansionError::PolicyIo {
                    path: request.whitelist_path.clone(),
                    source,
                }
            })?,
        }
    }
}

fn write_prompt<W: Write>(
    output: &mut W,
    request: &ShellApprovalRequest,
    source_desc: &str,
) -> Result<(), ShellExpansionError> {
    writeln!(
        output,
        "\nUnapproved shell command in {}:{}",
        source_desc, request.line
    )
    .map_err(|source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    })?;
    writeln!(output, "  {}", request.raw_command).map_err(|source| {
        ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        }
    })?;
    writeln!(output).map_err(|source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    })?;
    writeln!(output, "1. allow exact and save").map_err(|source| {
        ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        }
    })?;
    writeln!(output, "2. allow command and save").map_err(|source| {
        ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        }
    })?;
    writeln!(output, "3. allow once").map_err(|source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    })?;
    writeln!(output, "4. deny").map_err(|source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    })?;
    writeln!(output, "5. blacklist and stop").map_err(|source| {
        ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        }
    })?;
    write!(output, "> ").map_err(|source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    })?;
    output.flush().map_err(|source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    })
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
    use darkmatter::markdown::transform::TransformSource;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn request() -> ShellApprovalRequest {
        ShellApprovalRequest {
            source: TransformSource::File(PathBuf::from("/tmp/doc.md")),
            line: 12,
            raw_command: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
            normalized_exact: "echo hello".to_string(),
            whitelist_path: PathBuf::from("/tmp/.darkmatter-shell-whitelist"),
            blacklist_path: PathBuf::from("/tmp/.darkmatter-shell-blacklist"),
        }
    }

    #[test]
    fn prompt_accepts_valid_choice() {
        let request = request();
        let mut input = Cursor::new(b"2\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&request, &mut input, &mut output).unwrap();

        assert_eq!(decision, ShellApprovalDecision::AllowCommandPersist);
        let prompt = String::from_utf8(output).unwrap();
        assert!(prompt.contains("Unapproved shell command in /tmp/doc.md:12"));
        assert!(prompt.contains("echo hello"));
    }

    #[test]
    fn prompt_retries_after_invalid_choice() {
        let request = request();
        let mut input = Cursor::new(b"9\n3\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&request, &mut input, &mut output).unwrap();

        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
        let prompt = String::from_utf8(output).unwrap();
        assert!(prompt.contains("Invalid choice. Please enter 1-5."));
        assert_eq!(prompt.matches("1. allow exact and save").count(), 2);
    }
}
