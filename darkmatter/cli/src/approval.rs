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
        let source_desc = match &request.source {
            darkmatter::markdown::transform::TransformSource::File(p) => p.display().to_string(),
            darkmatter::markdown::transform::TransformSource::Url(u) => u.to_string(),
            darkmatter::markdown::transform::TransformSource::Unknown => "<stdin>".to_string(),
        };

        let stderr = io::stderr();
        let stdin = io::stdin();

        loop {
            let mut handle = stderr.lock();
            writeln!(
                handle,
                "\nUnapproved shell command in {}:{}",
                source_desc, request.line
            )
            .map_err(|e| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source: e,
            })?;
            writeln!(handle, "  {}", request.raw_command).map_err(|e| {
                ShellExpansionError::PolicyIo {
                    path: request.whitelist_path.clone(),
                    source: e,
                }
            })?;
            writeln!(handle).map_err(|e| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source: e,
            })?;
            writeln!(handle, "1. allow exact and save").map_err(|e| {
                ShellExpansionError::PolicyIo {
                    path: request.whitelist_path.clone(),
                    source: e,
                }
            })?;
            writeln!(handle, "2. allow command and save").map_err(|e| {
                ShellExpansionError::PolicyIo {
                    path: request.whitelist_path.clone(),
                    source: e,
                }
            })?;
            writeln!(handle, "3. allow once").map_err(|e| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source: e,
            })?;
            writeln!(handle, "4. deny").map_err(|e| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source: e,
            })?;
            writeln!(handle, "5. blacklist and stop").map_err(|e| {
                ShellExpansionError::PolicyIo {
                    path: request.whitelist_path.clone(),
                    source: e,
                }
            })?;
            write!(handle, "> ").map_err(|e| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source: e,
            })?;
            handle.flush().map_err(|e| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source: e,
            })?;
            drop(handle);

            let mut input = String::new();
            stdin
                .lock()
                .read_line(&mut input)
                .map_err(|e| ShellExpansionError::PolicyIo {
                    path: request.whitelist_path.clone(),
                    source: e,
                })?;

            match input.trim() {
                "1" => return Ok(ShellApprovalDecision::AllowExactPersist),
                "2" => return Ok(ShellApprovalDecision::AllowCommandPersist),
                "3" => return Ok(ShellApprovalDecision::AllowOnce),
                "4" => return Ok(ShellApprovalDecision::Deny),
                "5" => return Ok(ShellApprovalDecision::BlacklistPersist),
                _ => {
                    let mut handle = stderr.lock();
                    let _ = writeln!(handle, "Invalid choice. Please enter 1-5.");
                }
            }
        }
    }
}

/// Returns true if interactive prompting is safe.
///
/// Prompting is safe when:
/// 1. stdin is a terminal
/// 2. stderr is a terminal
pub fn can_prompt_interactively() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}
