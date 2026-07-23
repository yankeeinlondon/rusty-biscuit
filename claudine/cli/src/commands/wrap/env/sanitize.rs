//! Environment sanitization and secret redaction for the wrap pipeline.
//!
//! Strips sensitive process-env keys (honoring `--include` and provider
//! allow-lists), validates `--include` names, and redacts secret-looking CLI
//! arguments before they are serialized into `AGENT_PARAMS`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::OsString;

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

pub(crate) fn sanitize_process_env(
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

    for (key, value) in std::env::vars_os() {
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

        kept.insert(key, value);
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

pub(crate) fn is_sensitive_key(key: &str) -> bool {
    let uppercase = key.to_ascii_uppercase();
    uppercase.contains("API_KEY")
        || uppercase.contains("TOKEN")
        || uppercase.contains("PASSWORD")
        || uppercase.contains("SECRET")
        || uppercase.contains("PRIVATE_KEY")
        || uppercase.contains("CREDENTIAL")
        || uppercase.contains("ACCESS_KEY")
        || uppercase.contains("PASSPHRASE")
        || (uppercase.ends_with("_KEY") && !uppercase.contains("PUBLIC_KEY"))
        || uppercase.ends_with("_AUTH")
        || uppercase.ends_with("_PAT")
        || uppercase.ends_with("_PWD")
        || uppercase.ends_with("_PEM")
}

/// Redact values in CLI args that look like they contain secrets.
///
/// Scans for patterns like `--api-key=sk-...` or `--token sk-...` and
/// replaces the value portion with `****`. Matching is case-insensitive for
/// flag names, and a few common short aliases (`-k`) and token-value shapes
/// (`sk-`, `ghp_`, `xox[bp]-`, `AKIA`) are also redacted.
pub(crate) fn redact_sensitive_args(args: &[String]) -> Vec<String> {
    let sensitive_prefixes: &[&str] = &[
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

    let mut result = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            result.push("****".to_string());
            redact_next = false;
            continue;
        }

        // Check for --flag=value format first, preserving the authored casing
        // of the flag while redacting the value.
        let mut matched = false;
        if let Some(eq_pos) = arg.find('=') {
            let (flag, _rest) = arg.split_at(eq_pos);
            let flag_lower = flag.to_ascii_lowercase();
            for prefix in sensitive_prefixes {
                if flag_lower == *prefix {
                    result.push(format!("{flag}=****"));
                    matched = true;
                    break;
                }
            }
        }
        if matched {
            continue;
        }

        // Check for --flag value format.
        let arg_lower = arg.to_ascii_lowercase();
        for prefix in sensitive_prefixes {
            if arg_lower == *prefix {
                result.push(arg.clone());
                redact_next = true;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        // Value-shape redaction: bare secret-looking tokens.
        if looks_like_secret_token(arg) {
            result.push("****".to_string());
            continue;
        }

        result.push(arg.clone());
    }

    result
}

/// Returns `true` when a bare argument value starts with a known secret prefix.
fn looks_like_secret_token(value: &str) -> bool {
    value.starts_with("sk-")
        || value.starts_with("ghp_")
        || value.starts_with("xoxb-")
        || value.starts_with("xoxp-")
        || value.starts_with("AKIA")
}
