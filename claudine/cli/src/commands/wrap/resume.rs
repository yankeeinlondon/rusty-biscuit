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

/// Assemble a resume launch's argv: the resume entrypoint's own arguments,
/// then the forwarded provider tail exactly once, then the transport and
/// safety flags Claudine injected into the fresh launch.
///
/// `base_args` is the fresh launch's argv, which already contains
/// `provider_tail`. The allowlist reads only Claudine's injections (the base
/// argv with the tail's one contiguous run removed), so a user-supplied
/// `--json` or `--format` in the tail is neither dropped nor doubled. The tail
/// keeps its authored order and repetitions, and is not filtered per
/// entrypoint: a resume entrypoint that rejects it fails natively.
pub(crate) fn assemble_resume_args(
    entrypoint_args: Vec<String>,
    base_args: &[String],
    provider_tail: &[String],
) -> Vec<String> {
    let injected = without_provider_tail(base_args, provider_tail);
    let carried = carried_injections(&entrypoint_args, &injected);
    let mut args = entrypoint_args;
    args.extend(provider_tail.iter().cloned());
    args.extend(carried);
    args
}

/// `base_args` with the first contiguous run equal to `provider_tail`
/// removed.
///
/// Every launch plan seeds its argv with the tail and later stages only
/// prepend the entrypoint subcommand or append flags, so the tail is one
/// contiguous run. When it is not found that invariant broke; the base argv
/// is returned whole so no Claudine injection is lost.
fn without_provider_tail(base_args: &[String], provider_tail: &[String]) -> Vec<String> {
    if provider_tail.is_empty() {
        return base_args.to_vec();
    }
    match base_args
        .windows(provider_tail.len())
        .position(|window| window == provider_tail)
    {
        Some(start) => base_args[..start]
            .iter()
            .chain(&base_args[start + provider_tail.len()..])
            .cloned()
            .collect(),
        None => {
            tracing::warn!(
                tail_len = provider_tail.len(),
                "provider tail not found as one run in the launch argv; resume reads the whole argv"
            );
            base_args.to_vec()
        }
    }
}

/// The allowlisted flags (and their values) in `injected` that `entrypoint`
/// does not already carry, each at most once.
fn carried_injections(entrypoint: &[String], injected: &[String]) -> Vec<String> {
    let mut carried: Vec<String> = Vec::new();
    let present = |flag: &String, carried: &[String]| {
        entrypoint.iter().chain(carried).any(|arg| arg == flag)
    };
    let mut index = 0;
    while index < injected.len() {
        let flag = &injected[index];
        match flag.as_str() {
            // `--print-logs` (with its `--log-level` below) keeps a resumed
            // OpenCode structured run on the stderr-bridge contract: without
            // it the relaunch loses the progress signal the
            // stalled-generation backstop reads.
            // `--approve`/`--no-approve` is Pi's per-run project-trust
            // decision, which the resumed run must repeat.
            "--json" | "--verbose" | "--print-logs" | "--approve" | "--no-approve"
                if !present(flag, &carried) =>
            {
                carried.push(flag.clone());
            }
            // `--mode` selects Pi's structured interface (RPC or JSON).
            "--output-format" | "--format" | "--output-last-message" | "--log-level" | "--mode" => {
                if index + 1 < injected.len() && !present(flag, &carried) {
                    carried.push(flag.clone());
                    carried.push(injected[index + 1].clone());
                }
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
    carried
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
    fn carries_opencode_structured_stream_flags() {
        let base = args(&["run", "--format", "json", "--print-logs", "--log-level", "INFO"]);
        assert_eq!(
            assemble_resume_args(args(&["run", "--session", "ses_123"]), &base, &[]),
            args(&[
                "run", "--session", "ses_123", "--format", "json", "--print-logs", "--log-level",
                "INFO",
            ])
        );
    }

    /// A resumed Pi run keeps its structured interface and trust decision;
    /// without them Pi reads the piped follow-up as a plain-text print run.
    #[test]
    fn carries_pi_mode_and_trust() {
        let base = args(&["--extension", "./x.ts", "--no-approve", "--mode", "rpc"]);
        assert_eq!(
            assemble_resume_args(args(&["--session-id", "abc"]), &base, &[]),
            args(&["--session-id", "abc", "--no-approve", "--mode", "rpc"])
        );
    }

    #[test]
    fn does_not_duplicate_flags_the_entrypoint_already_carries() {
        let entry = args(&["--json", "--output-format", "stream-json"]);
        let base = args(&["--json", "--output-format", "stream-json"]);
        assert_eq!(assemble_resume_args(entry.clone(), &base, &[]), entry);
    }

    /// The tail follows the entrypoint once, with its authored repetitions;
    /// the allowlist carries only Claudine's own injections, so a user
    /// `--json`/`--format` in the tail is neither dropped nor doubled.
    #[test]
    fn appends_the_tail_once_and_carries_only_injections() {
        let tail = args(&["--add-dir", "a", "--add-dir", "a", "--json", "--format", "text"]);
        let mut base = args(&["exec"]);
        base.extend(tail.iter().cloned());
        base.extend(args(&["--output-last-message", "/tmp/last"]));

        assert_eq!(
            assemble_resume_args(args(&["exec", "resume", "s-1"]), &base, &tail),
            args(&[
                "exec", "resume", "s-1", "--add-dir", "a", "--add-dir", "a", "--json", "--format",
                "text", "--output-last-message", "/tmp/last",
            ])
        );
    }

    /// When Claudine injected the same flag the user also forwarded, the
    /// injection is still carried: both reached the fresh launch.
    #[test]
    fn an_injection_matching_a_tail_flag_is_still_carried() {
        let tail = args(&["--json"]);
        let base = args(&["exec", "--json", "--json"]);
        assert_eq!(
            assemble_resume_args(args(&["exec", "resume", "s-1"]), &base, &tail),
            args(&["exec", "resume", "s-1", "--json", "--json"])
        );
    }

    #[test]
    fn a_tail_missing_from_the_base_reads_the_whole_base() {
        let base = args(&["--json"]);
        let tail = args(&["-c", "x=y"]);
        assert_eq!(
            assemble_resume_args(args(&["resume"]), &base, &tail),
            args(&["resume", "-c", "x=y", "--json"])
        );
    }
}
