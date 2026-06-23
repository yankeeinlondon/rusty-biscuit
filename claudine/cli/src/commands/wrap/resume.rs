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
            "--json" | "--verbose" if !resume_args.iter().any(|arg| arg == &base_args[index]) => {
                resume_args.push(base_args[index].clone());
            }
            "--output-format" | "--format" | "--output-last-message" => {
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
