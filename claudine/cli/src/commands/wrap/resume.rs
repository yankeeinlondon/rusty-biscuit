use color_eyre::eyre::Result;

use super::profile::WrapperProfile;

pub(crate) fn normalize_resume_args(
    profile: &dyn WrapperProfile,
    mut args: Vec<String>,
) -> Vec<String> {
    if args.first().is_some_and(|arg| arg == profile.binary()) {
        args.remove(0);
    }
    args
}

pub(crate) fn append_resume_passthrough_args(resume_args: &mut Vec<String>, base_args: &[String]) {
    let mut index = 0;
    while index < base_args.len() {
        match base_args[index].as_str() {
            // `--print-logs` (with its `--log-level` below) keeps a resumed
            // OpenCode structured run on the stderr-bridge contract: without
            // it the relaunch loses the progress signal the
            // stalled-generation backstop reads.
            "--json" | "--verbose" | "--print-logs"
                if !resume_args.iter().any(|arg| arg == &base_args[index]) =>
            {
                resume_args.push(base_args[index].clone());
            }
            "--output-format" | "--format" | "--output-last-message" | "--log-level" => {
                if index + 1 < base_args.len()
                    && !resume_args.iter().any(|arg| arg == &base_args[index])
                {
                    resume_args.push(base_args[index].clone());
                    resume_args.push(base_args[index + 1].clone());
                }
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
}

/// Validates that a lifecycle `Resume` control can proceed for the given
/// provider and session.
///
/// This is the CLI-side resume gate that replaced the removed handler DSL's
/// resume validation; it returns an eyre error instead of a typed harness error.
pub(crate) fn check_resume_support(
    provider_name: &str,
    supports_resume: bool,
    session_id: Option<&str>,
) -> Result<()> {
    if !supports_resume {
        return Err(color_eyre::eyre::eyre!(
            "provider \"{provider_name}\" does not support session resume"
        ));
    }
    if session_id.is_none() {
        return Err(color_eyre::eyre::eyre!(
            "cannot resume: no session ID available from previous attempt"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn passthrough_carries_opencode_structured_stream_flags() {
        let mut resume_args = args(&["run", "--session", "ses_123"]);
        let base = args(&[
            "run",
            "--format",
            "json",
            "--print-logs",
            "--log-level",
            "INFO",
        ]);
        append_resume_passthrough_args(&mut resume_args, &base);
        assert_eq!(
            resume_args,
            args(&[
                "run",
                "--session",
                "ses_123",
                "--format",
                "json",
                "--print-logs",
                "--log-level",
                "INFO",
            ])
        );
    }

    #[test]
    fn passthrough_does_not_duplicate_flags_already_present() {
        let mut resume_args = args(&["--json", "--output-format", "stream-json"]);
        let base = args(&["--json", "--output-format", "stream-json"]);
        append_resume_passthrough_args(&mut resume_args, &base);
        assert_eq!(
            resume_args,
            args(&["--json", "--output-format", "stream-json"])
        );
    }
}
