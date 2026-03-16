//! Prompt-file resolution, Darkmatter composition, and env derivation.
//!
//! This module encapsulates the `--prompt-file` / `-p` wrapper flag logic:
//! path resolution with `@`, `./`, bare filename, and absolute path semantics;
//! Darkmatter-based composition; and frontmatter-to-environment-variable
//! conversion.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use color_eyre::eyre::{Result, bail, eyre};
use darkmatter::markdown::{Markdown, transform::TransformOptions};

use super::profile::WrapperProfile;

// ---------------------------------------------------------------------------
// Protected env var names
// ---------------------------------------------------------------------------

/// Environment variable names that prompt-file frontmatter must not override.
const PROTECTED_ENV_NAMES: &[&str] = &[
    "HOME",
    "PATH",
    "SHELL",
    "USER",
    "AGENT",
    "YOLO",
    "INTERACTIVE",
    "AGENT_PARAMS",
    "CLAUDINE_SESSION_ID",
    "PACKAGE",
    "PACKAGE_AREA",
    "REPO_ROOT",
    "NON_INTERACTIVE",
];

// ---------------------------------------------------------------------------
// Resolution types
// ---------------------------------------------------------------------------

/// Context needed for prompt-file path resolution.
#[derive(Debug)]
pub(crate) struct PromptResolutionContext {
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub package_root: Option<PathBuf>,
    pub interactive: bool,
}

/// A successfully resolved prompt-file path.
#[derive(Debug)]
pub(crate) struct ResolvedPromptFile {
    pub original: String,
    pub resolved_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Composition types
// ---------------------------------------------------------------------------

/// Result of composing a prompt file through the Darkmatter pipeline.
#[derive(Debug)]
pub(crate) struct ComposedPrompt {
    pub resolved_path: PathBuf,
    pub body: String,
    pub env_overrides: Vec<(String, String)>,
    pub env_names: Vec<String>,
}

/// Dry-run metadata for prompt-file display.
#[derive(Debug)]
pub(crate) struct PromptFileDryRunInfo {
    pub original: String,
    pub resolved_path: PathBuf,
    pub delivery_method: String,
    pub env_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Resolve a `--prompt-file` value to an absolute path on disk.
pub(crate) fn resolve_prompt_file(
    input: &str,
    ctx: &PromptResolutionContext,
) -> Result<ResolvedPromptFile> {
    // Extension validation
    let lower = input.to_ascii_lowercase();
    if !lower.ends_with(".md") && !lower.ends_with(".markdown") {
        return Err(eyre!(
            "prompt file must be a Markdown file (.md or .markdown), got: {input}"
        ));
    }

    let path = if let Some(rest) = input.strip_prefix('@') {
        // @ prefix: resolve against repo root
        let repo_root = ctx
            .repo_root
            .as_ref()
            .ok_or_else(|| eyre!("cannot use '@' prefix: no git repository detected"))?;
        repo_root.join(rest)
    } else if let Some(rest) = input.strip_prefix("./") {
        // ./ prefix: resolve against package root
        let package_root = ctx
            .package_root
            .as_ref()
            .ok_or_else(|| eyre!("cannot use './' prefix: no package root detected"))?;
        package_root.join(rest)
    } else if Path::new(input).is_absolute() {
        PathBuf::from(input)
    } else if input.contains('/') || input.contains(std::path::MAIN_SEPARATOR) {
        // Relative path with directory separators: resolve against cwd
        ctx.cwd.join(input)
    } else {
        // Bare filename: search in order
        resolve_bare_filename(input, ctx)?
    };

    if !path.exists() {
        return Err(eyre!("prompt file not found: {}", path.display()));
    }
    if !path.is_file() {
        return Err(eyre!("prompt file is not a regular file: {}", path.display()));
    }

    Ok(ResolvedPromptFile {
        original: input.to_string(),
        resolved_path: path,
    })
}

/// Search for a bare filename in cwd, package root, repo root, then repo-wide.
fn resolve_bare_filename(filename: &str, ctx: &PromptResolutionContext) -> Result<PathBuf> {
    // 1. cwd
    let cwd_path = ctx.cwd.join(filename);
    if cwd_path.exists() {
        return Ok(cwd_path);
    }

    // 2. package root
    if let Some(ref package_root) = ctx.package_root {
        let pkg_path = package_root.join(filename);
        if pkg_path.exists() {
            return Ok(pkg_path);
        }
    }

    // 3. repo root
    if let Some(ref repo_root) = ctx.repo_root {
        let repo_path = repo_root.join(filename);
        if repo_path.exists() {
            return Ok(repo_path);
        }

        // 4. Repo-wide leaf-name search
        let matches = search_repo_for_filename(repo_root, filename)?;
        return match matches.len() {
            0 => Err(eyre!(
                "prompt file '{filename}' not found in cwd, package root, repo root, \
                 or anywhere in the repository"
            )),
            1 => {
                let found = &matches[0];
                if ctx.interactive {
                    let display = found.strip_prefix(repo_root).unwrap_or(found);
                    let confirm = inquire::Confirm::new(&format!(
                        "Found prompt file at '{}'. Use it?",
                        display.display()
                    ))
                    .with_default(true)
                    .prompt()
                    .map_err(|e| eyre!("prompt cancelled: {e}"))?;
                    if confirm {
                        Ok(found.clone())
                    } else {
                        Err(eyre!("prompt file selection cancelled"))
                    }
                } else {
                    let display = found.strip_prefix(repo_root).unwrap_or(found);
                    Err(eyre!(
                        "prompt file '{filename}' not found in cwd, package root, or repo root. \
                         A match was found at '{}'; use an explicit path or run interactively.",
                        display.display()
                    ))
                }
            }
            _ => {
                if ctx.interactive {
                    let options: Vec<String> = matches
                        .iter()
                        .map(|p| {
                            p.strip_prefix(repo_root)
                                .unwrap_or(p)
                                .display()
                                .to_string()
                        })
                        .collect();
                    let selection = inquire::Select::new(
                        &format!("Multiple matches for '{filename}'. Choose one:"),
                        options.clone(),
                    )
                    .prompt()
                    .map_err(|e| eyre!("prompt cancelled: {e}"))?;
                    let index = options.iter().position(|o| o == &selection).unwrap_or(0);
                    Ok(matches[index].clone())
                } else {
                    let candidates: Vec<String> = matches
                        .iter()
                        .map(|p| {
                            p.strip_prefix(repo_root)
                                .unwrap_or(p)
                                .display()
                                .to_string()
                        })
                        .collect();
                    Err(eyre!(
                        "prompt file '{filename}' matched multiple files in the repository:\n  {}\n\
                         Use an explicit path or run interactively to choose.",
                        candidates.join("\n  ")
                    ))
                }
            }
        };
    }

    Err(eyre!(
        "prompt file '{filename}' not found in current directory \
         (no package root or repo root available for broader search)"
    ))
}

/// Walk the repo for exact leaf-name matches with .md or .markdown extension.
fn search_repo_for_filename(repo_root: &Path, filename: &str) -> Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    walk_dir_recursive(repo_root, filename, &mut matches)?;
    matches.sort();
    Ok(matches)
}

fn walk_dir_recursive(dir: &Path, filename: &str, matches: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| eyre!("failed to read '{}': {e}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip .git and other hidden directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name.starts_with('.') || name == "node_modules" || name == "target")
            {
                continue;
            }
            walk_dir_recursive(&path, filename, matches)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name == filename
        {
            matches.push(path);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Darkmatter composition
// ---------------------------------------------------------------------------

/// Load and compose a prompt file through the Darkmatter transform pipeline.
pub(crate) fn compose_prompt_file(resolved: &ResolvedPromptFile) -> Result<ComposedPrompt> {
    let md = Markdown::try_from(resolved.resolved_path.as_path()).map_err(|e| {
        eyre!(
            "failed to load prompt file '{}': {e}",
            resolved.resolved_path.display()
        )
    })?;

    let options = TransformOptions::new().with_source_file(&resolved.resolved_path);

    let (transformed, _report) = md.transform_with(options).map_err(|e| {
        eyre!(
            "Darkmatter compose failed for '{}': {e}",
            resolved.resolved_path.display()
        )
    })?;

    let body = transformed.content().to_string();
    let fm_map = transformed.frontmatter().as_map();

    // If the body is empty but frontmatter has a `prompt` key, the user likely
    // wants --frontmatter-prompt (--fp) instead of --prompt-file.
    if body.trim().is_empty() && fm_map.contains_key("prompt") {
        bail!(
            "prompt file '{}' has an empty body but contains a frontmatter `prompt` key; \
             use <blue>--frontmatter-prompt</blue> (or <blue>--fp</blue>) instead of \
             <blue>--prompt-file</blue> for this file",
            resolved.resolved_path.display()
        );
    }

    let env_overrides = frontmatter_to_env(fm_map)?;
    let env_names: Vec<String> = env_overrides.iter().map(|(k, _)| k.clone()).collect();

    Ok(ComposedPrompt {
        resolved_path: resolved.resolved_path.clone(),
        body,
        env_overrides,
        env_names,
    })
}

// ---------------------------------------------------------------------------
// Frontmatter to environment variables
// ---------------------------------------------------------------------------

/// Convert a frontmatter map to a vector of `(ENV_NAME, value)` pairs.
pub(crate) fn frontmatter_to_env(
    map: &IndexMap<String, serde_json::Value>,
) -> Result<Vec<(String, String)>> {
    let protected: HashSet<&str> = PROTECTED_ENV_NAMES.iter().copied().collect();
    let mut seen: HashMap<String, String> = HashMap::new(); // normalized -> original key
    let mut result = Vec::new();

    for (key, value) in map {
        let normalized = normalize_env_name(key);

        // Protected env rejection
        if protected.contains(normalized.as_str()) {
            return Err(eyre!(
                "frontmatter key '{key}' normalizes to protected env var '{normalized}'; \
                 prompt files cannot override this variable"
            ));
        }

        // Collision detection
        if let Some(existing_key) = seen.get(&normalized) {
            return Err(eyre!(
                "frontmatter keys '{existing_key}' and '{key}' both normalize to env var \
                 '{normalized}'; rename one to avoid the collision"
            ));
        }
        seen.insert(normalized.clone(), key.clone());

        let serialized = serialize_env_value(value);
        result.push((normalized, serialized));
    }

    // Sort for deterministic output
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

/// Normalize a frontmatter key to a valid environment variable name.
///
/// 1. Uppercase
/// 2. Replace non-`[A-Z0-9]` with `_`
/// 3. Collapse repeated `_`
/// 4. Trim leading and trailing `_`
/// 5. Prefix `PROMPT_FILE_` if result starts with a digit or is empty
pub(crate) fn normalize_env_name(key: &str) -> String {
    let upper = key.to_ascii_uppercase();
    let replaced: String = upper
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    // Collapse repeated underscores
    let mut collapsed = String::with_capacity(replaced.len());
    let mut prev_underscore = false;
    for c in replaced.chars() {
        if c == '_' {
            if !prev_underscore {
                collapsed.push(c);
            }
            prev_underscore = true;
        } else {
            collapsed.push(c);
            prev_underscore = false;
        }
    }

    // Trim leading and trailing underscores
    let trimmed = collapsed.trim_matches('_');

    if trimmed.is_empty() || trimmed.starts_with(|c: char| c.is_ascii_digit()) {
        format!("PROMPT_FILE_{trimmed}")
    } else {
        trimmed.to_string()
    }
}

/// Serialize a JSON value to a string suitable for an env var.
pub(crate) fn serialize_env_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_json::Value::Null => String::new(),
        // Arrays and objects → compact JSON
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// Prompt-source conflict detection
// ---------------------------------------------------------------------------

/// Check if the passthrough args already contain a provider-native prompt.
pub(crate) fn detect_existing_prompt_source(
    profile: &dyn WrapperProfile,
    child_args: &[String],
    provider: claudine::events::Provider,
) -> Result<()> {
    use claudine::events::Provider;

    let has_native_prompt = match provider {
        Provider::Gemini => {
            has_flag(child_args, "--prompt")
                || has_flag(child_args, "-p")
                || super::find_prompt_location(provider, child_args).is_some()
        }
        Provider::QwenCode => {
            has_flag(child_args, "--prompt")
                || has_flag(child_args, "-p")
        }
        Provider::Codex | Provider::OpenCode => {
            super::find_prompt_location(provider, child_args).is_some()
        }
        Provider::Goose => {
            has_flag(child_args, "-t")
                || super::find_prompt_location(provider, child_args).is_some()
        }
        _ => false,
    };

    if has_native_prompt {
        return Err(eyre!(
            "--prompt-file cannot be combined with a native prompt for {}; \
             remove the inline prompt or the --prompt-file flag",
            profile.provider()
        ));
    }

    Ok(())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter()
        .any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde_json::json;

    // -- normalize_env_name tests --

    #[test]
    fn normalize_simple_key() {
        assert_eq!(normalize_env_name("model"), "MODEL");
    }

    #[test]
    fn normalize_hyphenated_key() {
        assert_eq!(normalize_env_name("agent-type"), "AGENT_TYPE");
    }

    #[test]
    fn normalize_dotted_key() {
        assert_eq!(normalize_env_name("target.env"), "TARGET_ENV");
    }

    #[test]
    fn normalize_digit_prefix() {
        assert_eq!(normalize_env_name("2026_plan"), "PROMPT_FILE_2026_PLAN");
    }

    #[test]
    fn normalize_empty_key() {
        assert_eq!(normalize_env_name(""), "PROMPT_FILE_");
    }

    #[test]
    fn normalize_all_special_chars() {
        assert_eq!(normalize_env_name("---"), "PROMPT_FILE_");
    }

    // -- serialize_env_value tests --

    #[test]
    fn serialize_string_value() {
        assert_eq!(serialize_env_value(&json!("hello")), "hello");
    }

    #[test]
    fn serialize_number_value() {
        assert_eq!(serialize_env_value(&json!(42)), "42");
    }

    #[test]
    fn serialize_float_value() {
        assert_eq!(serialize_env_value(&json!(3.14)), "3.14");
    }

    #[test]
    fn serialize_bool_true() {
        assert_eq!(serialize_env_value(&json!(true)), "true");
    }

    #[test]
    fn serialize_bool_false() {
        assert_eq!(serialize_env_value(&json!(false)), "false");
    }

    #[test]
    fn serialize_null() {
        assert_eq!(serialize_env_value(&json!(null)), "");
    }

    #[test]
    fn serialize_array() {
        let val = json!(["a", "b"]);
        assert_eq!(serialize_env_value(&val), r#"["a","b"]"#);
    }

    #[test]
    fn serialize_object() {
        let val = json!({"key": "value"});
        assert_eq!(serialize_env_value(&val), r#"{"key":"value"}"#);
    }

    // -- frontmatter_to_env tests --

    #[test]
    fn frontmatter_to_env_basic() {
        let mut map = IndexMap::new();
        map.insert("model".to_string(), json!("gpt-4"));
        map.insert("temperature".to_string(), json!(0.7));

        let result = frontmatter_to_env(&map).unwrap();
        assert!(result.contains(&("MODEL".to_string(), "gpt-4".to_string())));
        assert!(result.contains(&("TEMPERATURE".to_string(), "0.7".to_string())));
    }

    #[test]
    fn frontmatter_collision_detected() {
        let mut map = IndexMap::new();
        map.insert("foo-bar".to_string(), json!("a"));
        map.insert("foo_bar".to_string(), json!("b"));

        let err = frontmatter_to_env(&map).unwrap_err();
        assert!(err.to_string().contains("both normalize to"));
        assert!(err.to_string().contains("FOO_BAR"));
    }

    #[test]
    fn frontmatter_protected_env_rejected() {
        let mut map = IndexMap::new();
        map.insert("path".to_string(), json!("/usr/bin"));

        let err = frontmatter_to_env(&map).unwrap_err();
        assert!(err.to_string().contains("protected env var"));
        assert!(err.to_string().contains("PATH"));
    }

    #[test]
    fn frontmatter_protected_home_rejected() {
        let mut map = IndexMap::new();
        map.insert("home".to_string(), json!("/tmp"));

        let err = frontmatter_to_env(&map).unwrap_err();
        assert!(err.to_string().contains("protected"));
    }

    // -- resolve_prompt_file tests --

    #[test]
    fn rejects_non_markdown_extension() {
        let ctx = PromptResolutionContext {
            cwd: PathBuf::from("/tmp"),
            repo_root: None,
            package_root: None,
            interactive: false,
        };
        let err = resolve_prompt_file("notes.txt", &ctx).unwrap_err();
        assert!(err.to_string().contains("Markdown file"));
    }

    #[test]
    fn at_prefix_errors_without_repo_root() {
        let ctx = PromptResolutionContext {
            cwd: PathBuf::from("/tmp"),
            repo_root: None,
            package_root: None,
            interactive: false,
        };
        let err = resolve_prompt_file("@prompts/test.md", &ctx).unwrap_err();
        assert!(err.to_string().contains("no git repository"));
    }

    #[test]
    fn dot_slash_prefix_errors_without_package_root() {
        let ctx = PromptResolutionContext {
            cwd: PathBuf::from("/tmp"),
            repo_root: None,
            package_root: None,
            interactive: false,
        };
        let err = resolve_prompt_file("./prompts/test.md", &ctx).unwrap_err();
        assert!(err.to_string().contains("no package root"));
    }

    #[test]
    fn at_prefix_resolves_against_repo_root() {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        let file = prompts_dir.join("test.md");
        std::fs::write(&file, "# Test").unwrap();

        let ctx = PromptResolutionContext {
            cwd: dir.path().to_path_buf(),
            repo_root: Some(dir.path().to_path_buf()),
            package_root: None,
            interactive: false,
        };
        let resolved = resolve_prompt_file("@prompts/test.md", &ctx).unwrap();
        assert_eq!(resolved.resolved_path, file);
    }

    #[test]
    fn dot_slash_resolves_against_package_root() {
        let dir = tempfile::tempdir().unwrap();
        let pkg_dir = dir.path().join("pkg");
        let prompts_dir = pkg_dir.join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        let file = prompts_dir.join("test.md");
        std::fs::write(&file, "# Test").unwrap();

        let ctx = PromptResolutionContext {
            cwd: dir.path().to_path_buf(),
            repo_root: None,
            package_root: Some(pkg_dir),
            interactive: false,
        };
        let resolved = resolve_prompt_file("./prompts/test.md", &ctx).unwrap();
        assert_eq!(resolved.resolved_path, file);
    }

    #[test]
    fn absolute_path_used_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.md");
        std::fs::write(&file, "# Test").unwrap();

        let ctx = PromptResolutionContext {
            cwd: PathBuf::from("/tmp"),
            repo_root: None,
            package_root: None,
            interactive: false,
        };
        let resolved = resolve_prompt_file(file.to_str().unwrap(), &ctx).unwrap();
        assert_eq!(resolved.resolved_path, file);
    }

    #[test]
    fn relative_path_with_separator_resolves_against_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("test.md");
        std::fs::write(&file, "# Test").unwrap();

        let ctx = PromptResolutionContext {
            cwd: dir.path().to_path_buf(),
            repo_root: None,
            package_root: None,
            interactive: false,
        };
        let resolved = resolve_prompt_file("sub/test.md", &ctx).unwrap();
        assert_eq!(resolved.resolved_path, file);
    }

    #[test]
    fn bare_filename_finds_in_cwd_first() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("review.md");
        std::fs::write(&file, "# Review").unwrap();

        let ctx = PromptResolutionContext {
            cwd: dir.path().to_path_buf(),
            repo_root: Some(dir.path().to_path_buf()),
            package_root: None,
            interactive: false,
        };
        let resolved = resolve_prompt_file("review.md", &ctx).unwrap();
        assert_eq!(resolved.resolved_path, file);
    }

    #[test]
    fn bare_filename_falls_through_to_package_root() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        let file = pkg.join("review.md");
        std::fs::write(&file, "# Review").unwrap();
        // cwd has no such file
        let cwd = dir.path().join("other");
        std::fs::create_dir_all(&cwd).unwrap();

        let ctx = PromptResolutionContext {
            cwd,
            repo_root: Some(dir.path().to_path_buf()),
            package_root: Some(pkg),
            interactive: false,
        };
        let resolved = resolve_prompt_file("review.md", &ctx).unwrap();
        assert_eq!(resolved.resolved_path, file);
    }

    #[test]
    fn bare_filename_repo_wide_single_match_non_interactive_errors_with_suggestion() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("deep").join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let file = nested.join("review.md");
        std::fs::write(&file, "# Review").unwrap();
        let cwd = dir.path().join("empty");
        std::fs::create_dir_all(&cwd).unwrap();

        let ctx = PromptResolutionContext {
            cwd,
            repo_root: Some(dir.path().to_path_buf()),
            package_root: None,
            interactive: false,
        };
        let err = resolve_prompt_file("review.md", &ctx).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("deep/nested/review.md") || msg.contains("deep"));
    }

    #[test]
    fn bare_filename_repo_wide_multi_match_non_interactive_errors_with_candidates() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("review.md"), "# A").unwrap();
        std::fs::write(b.join("review.md"), "# B").unwrap();
        let cwd = dir.path().join("empty");
        std::fs::create_dir_all(&cwd).unwrap();

        let ctx = PromptResolutionContext {
            cwd,
            repo_root: Some(dir.path().to_path_buf()),
            package_root: None,
            interactive: false,
        };
        let err = resolve_prompt_file("review.md", &ctx).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("multiple files"));
    }
}
