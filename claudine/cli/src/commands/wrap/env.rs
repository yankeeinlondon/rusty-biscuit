use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, bail};
use sniff::filesystem::git::detect_git;
use sniff::filesystem::repo::{Package, detect_repo};

use super::profile::WrapperProfile;
use super::repo_home;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct EnvPlan {
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) removed: Vec<String>,
    pub(crate) included: Vec<String>,
    pub(crate) added: Vec<(String, String)>,
    pub(crate) package_context: Option<PackageContext>,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) warnings: Vec<String>,
    pub(crate) shadow_home_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageContext {
    pub(crate) package_area: String,
    pub(crate) package: Option<String>,
    pub(crate) candidates: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_child_env(
    profile: &dyn WrapperProfile,
    provider: claudine::events::Provider,
    include: &[String],
    yolo: bool,
    interactive: bool,
    agent_params: &[String],
    cwd: &Path,
    env_overrides: &[(String, String)],
    repo: bool,
    force_shadow_home: bool,
    repo_root_hint: Option<&Path>,
) -> Result<EnvPlan> {
    let include_set = validate_include_names(include)?;
    let auto_include: HashSet<String> = profile
        .allowed_env_keys()
        .iter()
        .map(|k| (*k).to_string())
        .collect();
    let (mut env, removed, included, mut warnings) =
        sanitize_process_env(&include_set, &auto_include);
    let mut added = BTreeMap::new();

    let redacted_params = redact_sensitive_args(agent_params);
    let encoded_agent_params = serde_json::to_string(&redacted_params)?;
    set_added_env(
        &mut env,
        &mut added,
        "AGENT",
        profile.agent_env().to_string(),
    );
    set_added_env(
        &mut env,
        &mut added,
        "YOLO",
        if yolo {
            "true".to_string()
        } else {
            "false".to_string()
        },
    );
    set_added_env(
        &mut env,
        &mut added,
        "INTERACTIVE",
        if interactive {
            "true".to_string()
        } else {
            "false".to_string()
        },
    );
    set_added_env(&mut env, &mut added, "AGENT_PARAMS", encoded_agent_params);
    set_added_env(
        &mut env,
        &mut added,
        "CLAUDINE_SESSION_ID",
        uuid::Uuid::new_v4().to_string(),
    );

    for (key, value) in env_overrides {
        set_added_env(&mut env, &mut added, key, value.clone());
    }

    let mut shadow_home_path = None;

    // When a repo_root_hint is provided (composition mode), use it for
    // shadow-HOME and repo_root instead of re-deriving from the caller's CWD.
    // This ensures cross-repo composition picks up the source repo's context.
    let effective_root_for_home = repo_root_hint.unwrap_or(cwd);

    let needs_shadow_home =
        force_shadow_home || repo_home::needs_shadow_home(provider, effective_root_for_home, repo);

    // Use a shadow HOME when repo-only isolation is requested, or when Codex
    // needs repo-local prompt overlay because custom prompts are user-scoped.
    if needs_shadow_home {
        match repo_home::build_repo_home_env(provider, effective_root_for_home, repo) {
            Ok((shadow_env, shadow_path)) => {
                for (key, value) in shadow_env {
                    env.insert(key, value);
                }
                shadow_home_path = shadow_path;
            }
            Err(e) => {
                warnings.push(format!("failed to create shadow HOME: {}", e));
                // Fall back to original behavior
                set_added_env(&mut env, &mut added, "HOME", "/dev/null".to_string());
            }
        }
    }

    // Package context (PACKAGE_AREA, PACKAGE) is always derived from the
    // caller's CWD so the agent knows which package the user was working in.
    // The repo_root, however, uses the hint when available so the child
    // process starts in the correct repository.
    let mut package_context = None;
    let mut repo_root = repo_root_hint.map(Path::to_path_buf);
    match resolve_monorepo_package_context(cwd) {
        Ok(repo_ctx) => {
            if repo_root.is_none() {
                repo_root = repo_ctx.repo_root;
            }
            if let Some(package_ctx) = repo_ctx.package_context {
                set_added_env(
                    &mut env,
                    &mut added,
                    "PACKAGE_AREA",
                    package_ctx.package_area.clone(),
                );
                if let Some(ref package) = package_ctx.package {
                    set_added_env(&mut env, &mut added, "PACKAGE", package.clone());
                }
                package_context = Some(package_ctx);
            }
            warnings.extend(repo_ctx.warnings);
        }
        Err(error) => warnings.push(format!(
            "failed to resolve monorepo package metadata for '{}': {}",
            cwd.display(),
            error
        )),
    }

    Ok(EnvPlan {
        env,
        removed,
        included,
        added: added.into_iter().collect(),
        package_context,
        repo_root,
        warnings,
        shadow_home_path,
    })
}

fn set_added_env(
    env: &mut HashMap<OsString, OsString>,
    added: &mut BTreeMap<String, String>,
    key: &str,
    value: String,
) {
    env.insert(OsString::from(key), OsString::from(value.clone()));
    added.insert(key.to_string(), value);
}

fn validate_include_names(include: &[String]) -> Result<HashSet<String>> {
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

fn sanitize_process_env(
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
            if include_set.contains(&key_display) || auto_include.contains(&key_display) {
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

fn is_sensitive_key(key: &str) -> bool {
    let uppercase = key.to_ascii_uppercase();
    uppercase.contains("API_KEY")
        || uppercase.contains("TOKEN")
        || uppercase.contains("PASSWORD")
        || uppercase.contains("SECRET")
        || uppercase.contains("PRIVATE_KEY")
        || uppercase.contains("CREDENTIAL")
        || uppercase.contains("ACCESS_KEY")
        || uppercase.contains("PASSPHRASE")
}

/// Redact values in CLI args that look like they contain secrets.
///
/// Scans for patterns like `--api-key=sk-...` or `--token sk-...` and
/// replaces the value portion with `****`.
fn redact_sensitive_args(args: &[String]) -> Vec<String> {
    let sensitive_prefixes: &[&str] = &[
        "--api-key",
        "--token",
        "--secret",
        "--password",
        "--credential",
        "--access-key",
        "--private-key",
        "--passphrase",
    ];

    let mut result = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            result.push("****".to_string());
            redact_next = false;
            continue;
        }

        // Check for --flag=value format
        let mut matched = false;
        for prefix in sensitive_prefixes {
            if let Some(rest) = arg.strip_prefix(prefix) {
                if rest.starts_with('=') {
                    result.push(format!("{prefix}=****"));
                    matched = true;
                    break;
                }
                if rest.is_empty() {
                    // Next arg is the value
                    result.push(arg.clone());
                    redact_next = true;
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            result.push(arg.clone());
        }
    }

    result
}

struct RepoContext {
    package_context: Option<PackageContext>,
    repo_root: Option<PathBuf>,
    warnings: Vec<String>,
}

fn resolve_monorepo_package_context(cwd: &Path) -> Result<RepoContext> {
    let git_root = detect_git(cwd, false, 1)?.map(|info| info.repo_root);
    let repo_probe_root = git_root.clone().unwrap_or_else(|| cwd.to_path_buf());
    let Some(repo) = detect_repo(&repo_probe_root)? else {
        return Ok(RepoContext {
            package_context: None,
            repo_root: git_root,
            warnings: Vec::new(),
        });
    };

    if !repo.is_monorepo {
        return Ok(RepoContext {
            package_context: None,
            repo_root: git_root,
            warnings: Vec::new(),
        });
    }

    let Some(packages) = repo.packages else {
        return Ok(RepoContext {
            package_context: None,
            repo_root: git_root,
            warnings: vec![format!(
                "monorepo detected at '{}' but no packages were reported",
                repo.root.display()
            )],
        });
    };

    if let Some(package_ctx) = select_package_for_cwd(cwd, &packages) {
        return Ok(RepoContext {
            package_context: Some(package_ctx),
            repo_root: git_root,
            warnings: Vec::new(),
        });
    }

    if let Some(package_area) = select_package_area_for_cwd(cwd, &repo.root, &packages) {
        let candidates = package_candidates_for_area(&package_area, &packages);
        return Ok(RepoContext {
            package_context: Some(PackageContext {
                package_area,
                package: None,
                candidates,
            }),
            repo_root: git_root,
            warnings: Vec::new(),
        });
    }

    Ok(RepoContext {
        package_context: None,
        repo_root: git_root,
        warnings: vec![format!(
            "monorepo detected at '{}' but no package area matched cwd '{}'",
            repo.root.display(),
            cwd.display()
        )],
    })
}

fn select_package_for_cwd(cwd: &Path, packages: &[Package]) -> Option<PackageContext> {
    let cwd_normalized = canonical_or_self(cwd);

    packages
        .iter()
        .filter_map(|package| {
            let package_path = canonical_or_self(&package.path);
            if cwd_normalized.starts_with(&package_path) {
                Some((package_path.components().count(), package))
            } else {
                None
            }
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, package)| PackageContext {
            package_area: package.package_area.clone(),
            package: Some(package.name.clone()),
            candidates: vec![package.name.clone()],
        })
}

fn select_package_area_for_cwd(
    cwd: &Path,
    repo_root: &Path,
    packages: &[Package],
) -> Option<String> {
    let cwd_normalized = canonical_or_self(cwd);
    let repo_root_normalized = canonical_or_self(repo_root);

    packages
        .iter()
        .map(|package| {
            let area_root = if package.package_area == "root" {
                repo_root_normalized.clone()
            } else {
                repo_root_normalized.join(&package.package_area)
            };
            (area_root, package.package_area.clone())
        })
        .filter(|(area_root, _)| cwd_normalized.starts_with(area_root))
        .max_by_key(|(area_root, _)| area_root.components().count())
        .map(|(_, area)| area)
}

fn package_candidates_for_area(package_area: &str, packages: &[Package]) -> Vec<String> {
    let mut candidates: Vec<String> = packages
        .iter()
        .filter(|package| package.package_area == package_area)
        .map(|package| package.name.clone())
        .collect();
    candidates.sort();
    candidates.dedup();
    candidates
}

fn canonical_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::wrap::profile::profile_for_provider;
    use sniff::filesystem::repo::Package;

    #[test]
    fn sanitization_removes_sensitive_names_unless_included() {
        let include_set = HashSet::from(["OPENAI_API_KEY".to_string()]);
        let env = vec![
            ("OPENAI_API_KEY".to_string(), "keep".to_string()),
            ("SVC_TOKEN".to_string(), "remove".to_string()),
            ("DB_PASSWORD".to_string(), "remove".to_string()),
            ("APP_SECRET".to_string(), "remove".to_string()),
            ("NORMAL_VAR".to_string(), "ok".to_string()),
        ];

        let (kept, removed) = sanitize_env_for_test(&env, &include_set);
        let kept_names: HashSet<_> = kept.into_iter().map(|(name, _)| name).collect();

        assert!(kept_names.contains("OPENAI_API_KEY"));
        assert!(kept_names.contains("NORMAL_VAR"));
        assert_eq!(
            removed,
            vec![
                "APP_SECRET".to_string(),
                "DB_PASSWORD".to_string(),
                "SVC_TOKEN".to_string()
            ]
        );
    }

    #[test]
    fn sanitization_catches_new_sensitive_patterns() {
        let include_set = HashSet::new();
        let env = vec![
            ("SSH_PRIVATE_KEY".to_string(), "secret".to_string()),
            ("AWS_ACCESS_KEY_ID".to_string(), "secret".to_string()),
            ("DB_CREDENTIAL".to_string(), "secret".to_string()),
            ("KEY_PASSPHRASE".to_string(), "secret".to_string()),
            ("NORMAL_VAR".to_string(), "ok".to_string()),
        ];

        let (kept, removed) = sanitize_env_for_test(&env, &include_set);
        let kept_names: HashSet<_> = kept.into_iter().map(|(name, _)| name).collect();

        assert!(!kept_names.contains("SSH_PRIVATE_KEY"));
        assert!(!kept_names.contains("AWS_ACCESS_KEY_ID"));
        assert!(!kept_names.contains("DB_CREDENTIAL"));
        assert!(!kept_names.contains("KEY_PASSPHRASE"));
        assert!(kept_names.contains("NORMAL_VAR"));
        assert_eq!(removed.len(), 4);
    }

    #[test]
    fn include_names_must_be_valid_env_identifiers() {
        let includes = vec!["VALID_NAME".to_string(), "9INVALID".to_string()];
        let error = validate_include_names(&includes).unwrap_err();
        assert!(error.to_string().contains("invalid --include env name"));
    }

    #[test]
    fn redact_sensitive_args_hides_secret_values() {
        let args = vec![
            "--api-key=sk-12345".to_string(),
            "--token".to_string(),
            "bearer-abc".to_string(),
            "--model".to_string(),
            "gpt-4o".to_string(),
            "--password=hunter2".to_string(),
        ];

        let redacted = redact_sensitive_args(&args);
        assert_eq!(
            redacted,
            vec![
                "--api-key=****",
                "--token",
                "****",
                "--model",
                "gpt-4o",
                "--password=****",
            ]
        );
    }

    #[test]
    fn redact_sensitive_args_preserves_non_secret_args() {
        let args = vec![
            "--json".to_string(),
            "summarize".to_string(),
            "--model".to_string(),
            "gpt-4o".to_string(),
        ];

        let redacted = redact_sensitive_args(&args);
        assert_eq!(redacted, args);
    }

    #[test]
    fn package_selection_prefers_longest_matching_prefix() {
        let cwd = Path::new("/repo/apps/browser/my-app/src");
        let packages = vec![
            Package {
                path: PathBuf::from("/repo/apps/browser"),
                relative: "apps/browser".to_string(),
                package_area: "apps".to_string(),
                name: "browser-root".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
            Package {
                path: PathBuf::from("/repo/apps/browser/my-app"),
                relative: "apps/browser/my-app".to_string(),
                package_area: "apps/browser".to_string(),
                name: "my-app".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
        ];

        let selected = select_package_for_cwd(cwd, &packages).unwrap();
        assert_eq!(selected.package, Some("my-app".to_string()));
        assert_eq!(selected.package_area, "apps/browser");
    }

    #[test]
    fn package_area_selection_supports_area_root_without_package_match() {
        let cwd = Path::new("/repo/claudine");
        let repo_root = Path::new("/repo");
        let packages = vec![
            Package {
                path: PathBuf::from("/repo/claudine/lib"),
                relative: "claudine/lib".to_string(),
                package_area: "claudine".to_string(),
                name: "claudine".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
            Package {
                path: PathBuf::from("/repo/claudine/cli"),
                relative: "claudine/cli".to_string(),
                package_area: "claudine".to_string(),
                name: "claudine-cli".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
        ];

        assert!(select_package_for_cwd(cwd, &packages).is_none());
        let area = select_package_area_for_cwd(cwd, repo_root, &packages);
        assert_eq!(area, Some("claudine".to_string()));
        assert_eq!(
            package_candidates_for_area("claudine", &packages),
            vec!["claudine".to_string(), "claudine-cli".to_string()]
        );
    }

    #[test]
    fn build_child_env_uses_interactive_without_claudine_duplicate() {
        let profile = profile_for_provider(claudine::events::Provider::Claude).unwrap();
        let cwd = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::events::Provider::Claude,
            &[],
            false,
            true,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            None,
        )
        .unwrap();

        let added: std::collections::HashMap<_, _> = plan.added.into_iter().collect();
        assert_eq!(added.get("INTERACTIVE").map(String::as_str), Some("true"));
        assert!(!added.contains_key("CLAUDINE_INTERACTIVE"));
    }

    fn sanitize_env_for_test(
        env: &[(String, String)],
        include_set: &HashSet<String>,
    ) -> (Vec<(String, String)>, Vec<String>) {
        let auto_include = HashSet::new();
        sanitize_env_for_test_with_auto(env, include_set, &auto_include)
    }

    fn sanitize_env_for_test_with_auto(
        env: &[(String, String)],
        include_set: &HashSet<String>,
        auto_include: &HashSet<String>,
    ) -> (Vec<(String, String)>, Vec<String>) {
        let mut kept = Vec::new();
        let mut removed = BTreeSet::new();

        for (key, value) in env {
            if is_sensitive_key(key) && !include_set.contains(key) && !auto_include.contains(key) {
                removed.insert(key.clone());
            } else {
                kept.push((key.clone(), value.clone()));
            }
        }

        (kept, removed.into_iter().collect())
    }
}
