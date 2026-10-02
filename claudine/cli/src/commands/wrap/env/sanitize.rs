//! Environment sanitization and secret redaction for the wrap pipeline.
//!
//! Strips sensitive process-env keys (honoring `--include` and provider
//! allow-lists), validates `--include` names, and redacts secret-looking CLI
//! arguments before they are displayed, traced, or serialized into
//! `AGENT_PARAMS`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::OsString;

use claudine::invocation_context::EnvBaseline;
pub(crate) use claudine::secrets::is_sensitive_key_name as is_sensitive_key;
use claudine::secrets::{MASK, find_argument_secret_spans, has_credential_prefix, mask_argument_token};
use color_eyre::eyre::{Result, bail};

pub(crate) fn validate_include_names(include: &[String]) -> Result<HashSet<String>> {
    let mut unique = HashSet::new();
    for name in include {
        if !is_valid_env_name(name) {
            bail!("invalid --include env name '{}'", name);
        }
        unique.insert(name.clone());
    }
    Ok(unique)
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Filter one launch environment snapshot into the child's inherited half.
///
/// The snapshot is supplied rather than read from the process because a wrapper
/// stage may have mutated its own environment after launch; the child must
/// inherit what Claudine was started with, not what Claudine has since become.
pub(crate) fn sanitize_process_env(
    baseline: &EnvBaseline,
    include_set: &HashSet<String>,
    auto_include: &HashSet<String>,
) -> (
    HashMap<OsString, OsString>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
) {
    let mut kept = HashMap::new();
    let mut removed = BTreeSet::new();
    let mut included = BTreeSet::new();
    let mut present_keys = HashSet::new();

    for (key, value) in baseline.iter() {
        let key_display = key.to_string_lossy().to_string();
        present_keys.insert(key_display.clone());

        if is_sensitive_key(&key_display) {
            if admits_sensitive_key(&key_display, include_set, auto_include) {
                included.insert(key_display);
            } else {
                removed.insert(key_display);
                continue;
            }
        }

        kept.insert(key.to_os_string(), value.to_os_string());
    }

    // Only warn about missing keys for explicit --include, not auto-included.
    let mut warnings = Vec::new();
    for include in include_set {
        if !present_keys.contains(include) {
            warnings.push(format!(
                "--include '{}' was requested but is not set in the current environment",
                include
            ));
        }
    }

    (
        kept,
        removed.into_iter().collect(),
        included.into_iter().collect(),
        warnings,
    )
}

/// Whether one sensitive key survives sanitation under a given allow-list.
///
/// `auto_include` is a provider profile's `allowed_env_keys()`; `include_set` is
/// explicit `--include` intent, which admits a key under every provider. Shared
/// with the launch-plan replay so a rebuilt provider reaches the same verdict
/// [`sanitize_process_env`] would have reached had that provider opened the
/// invocation.
pub(crate) fn admits_sensitive_key(
    key: &str,
    include_set: &HashSet<String>,
    auto_include: &HashSet<String>,
) -> bool {
    include_set.contains(key) || auto_include.contains(key)
}

/// Every sensitive key in the wrapper's own process environment, unsanitized.
///
/// Invocation-neutral by construction: no provider allow-list has been consulted
/// yet. This is the snapshot a per-attempt rebuild re-sanitizes for its own
/// profile, which is the only way to *readmit* a credential the opening provider
/// stripped — the sanitized base child environment no longer records it.
pub(crate) fn ambient_sensitive_env() -> HashMap<OsString, OsString> {
    std::env::vars_os()
        .filter(|(key, _)| is_sensitive_key(&key.to_string_lossy()))
        .collect()
}

/// Redact values in CLI args that look like they contain secrets.
///
/// Every display or metadata surface of an argument vector goes through this
/// policy; the child itself always receives the original tokens.
///
/// - A sensitive flag (`--api-key`, `--token`, `-k`, …; case-insensitive)
///   keeps its name and masks its value, attached (`--token=****`) or in the
///   next token.
/// - A short switch with a credential-shaped value attached keeps the switch
///   (`-csk-…` becomes `-c****`).
/// - Every other token keeps only what the shared argument recognizer
///   ([`claudine::secrets::mask_argument_token`]) does not mask, so a bare
///   credential, an embedded assignment (`api_key=sk-…`), and a long attached
///   value (`--config=sk-…`) are masked too.
pub(crate) fn redact_sensitive_args(args: &[String]) -> Vec<String> {
    redact_args(args).into_iter().map(|arg| arg.shown).collect()
}

/// The original values [`redact_sensitive_args`] masks in `args`, so text that
/// echoes one of them without its flag can be masked too.
pub(crate) fn sensitive_arg_values(args: &[String]) -> Vec<String> {
    redact_args(args)
        .into_iter()
        .flat_map(|arg| arg.secrets)
        .collect()
}

/// One token as displays show it, and the original secret text it hides.
struct RedactedArg {
    shown: String,
    secrets: Vec<String>,
}

impl RedactedArg {
    fn kept(arg: &str) -> Self {
        Self {
            shown: arg.to_string(),
            secrets: Vec::new(),
        }
    }

    /// `arg` with everything from byte `at` masked.
    fn masked_from(arg: &str, at: usize) -> Self {
        Self {
            shown: format!("{}{MASK}", &arg[..at]),
            secrets: vec![arg[at..].to_string()],
        }
    }
}

const SENSITIVE_FLAGS: &[&str] = &[
    "--api-key",
    "--apikey",
    "--token",
    "--secret",
    "--password",
    "--credential",
    "--access-key",
    "--accesskey",
    "--private-key",
    "--privatekey",
    "--passphrase",
    "--bearer",
    "-k",
];

fn is_sensitive_flag(flag: &str) -> bool {
    let flag = flag.to_ascii_lowercase();
    SENSITIVE_FLAGS.contains(&flag.as_str())
}

fn redact_args(args: &[String]) -> Vec<RedactedArg> {
    let mut result = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            redact_next = false;
            result.push(RedactedArg::masked_from(arg, 0));
            continue;
        }
        if let Some((flag, _)) = arg.split_once('=')
            && is_sensitive_flag(flag)
        {
            result.push(RedactedArg::masked_from(arg, flag.len() + 1));
            continue;
        }
        if is_sensitive_flag(arg) {
            redact_next = true;
            result.push(RedactedArg::kept(arg));
            continue;
        }
        // Checked before the shared recognizer: `-csk-…` has no word boundary
        // before `sk-`, so the catalog shapes never see the attached value.
        if let Some(attached) = short_attached_value(arg)
            && has_credential_prefix(attached)
        {
            result.push(RedactedArg::masked_from(arg, 2));
            continue;
        }
        let spans = find_argument_secret_spans(arg);
        if spans.is_empty() {
            result.push(RedactedArg::kept(arg));
        } else {
            result.push(RedactedArg {
                shown: mask_argument_token(arg).into_owned(),
                secrets: spans.into_iter().map(|span| arg[span].to_string()).collect(),
            });
        }
    }

    result
}

/// The text attached to a single-dash short switch (`secret` in `-csecret`),
/// or `None` for anything else.
fn short_attached_value(arg: &str) -> Option<&str> {
    let rest = arg.strip_prefix('-')?;
    if rest.starts_with('-') {
        return None;
    }
    let mut chars = rest.chars();
    let switch = chars.next()?;
    let attached = chars.as_str();
    (switch.is_ascii_alphanumeric() && !attached.is_empty()).then_some(attached)
}
