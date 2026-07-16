//! `DmlsConfig`: the server configuration model and its three sources.
//!
//! Precedence (design "Configuration"): editor `workspace/configuration`
//! overlay > `.dmls.toml` at the workspace root > built-in defaults.
//! Frontmatter never configures the server. All spec keys are parsed now
//! even where their consumers land in later phases, so configs written
//! today stay valid as features arrive.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The repo config file name, discovered at each workspace root. Also serves
/// as an editor root marker (R-7 snippets pair it with `.git`).
pub const CONFIG_FILE_NAME: &str = ".dmls.toml";

/// Errors loading a config file.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The file could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// The file is not valid TOML for the config shape.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying TOML error.
        source: Box<toml::de::Error>,
    },
}

/// Wiki-link completion path style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WikiPathStyle {
    /// Shortest suffix that resolves uniquely (escalating through path
    /// segments).
    #[default]
    Shortest,
    /// Path relative to the containing document.
    Relative,
    /// Path relative to the wiki root.
    RootRelative,
}

/// What wiki heading completion inserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeadingCompletionStyle {
    /// The heading's visible text.
    #[default]
    Text,
    /// The GitHub-style slug.
    Slug,
}

/// Which Darkmatter cleanup variant formatting uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CleanupVariant {
    /// `Markdown::cleanup`.
    #[default]
    Default,
    /// `Markdown::cleanup_compact`.
    Compact,
    /// `Markdown::cleanup_loose`.
    Loose,
}

/// Workspace/library root and discovery globs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    /// Optional root override (relative to the LSP workspace folder).
    pub root: Option<PathBuf>,
    /// Include globs for workspace discovery.
    pub include: Vec<String>,
    /// Exclude globs for workspace discovery.
    pub exclude: Vec<String>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root: None,
            include: vec!["**/*.md".to_string(), "**/*.markdown".to_string()],
            exclude: Vec::new(),
        }
    }
}

/// Wiki-link behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WikiConfig {
    /// Master switch for wiki-link features.
    pub enable: bool,
    /// Narrows or redirects the wiki resolution universe.
    pub wiki_root: Option<PathBuf>,
    /// Completion insertion style.
    pub path_style: WikiPathStyle,
    /// Heading completion insertion style.
    pub heading_completion_style: HeadingCompletionStyle,
}

impl Default for WikiConfig {
    fn default() -> Self {
        Self {
            enable: true,
            wiki_root: None,
            path_style: WikiPathStyle::default(),
            heading_completion_style: HeadingCompletionStyle::default(),
        }
    }
}

/// One baseline schema extension (e.g. Claudine): pure data, no code paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaExtensionConfig {
    /// SimplifiedSchema YAML path (relative paths resolve against the
    /// workspace root).
    pub path: PathBuf,
    /// Activation globs (e.g. `.claude/**`).
    #[serde(default)]
    pub globs: Vec<String>,
}

/// Frontmatter schema behavior.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SchemaConfig {
    /// Strict mode: unknown frontmatter keys become diagnostics.
    pub strict: bool,
    /// Baseline schema extensions by name.
    pub extensions: BTreeMap<String, SchemaExtensionConfig>,
}

/// `style:` frontmatter behavior.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleConfig {
    /// Strict mode: unknown/deprecated style keys become diagnostics.
    pub strict: bool,
}

/// Shell-policy discovery.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    /// Where to discover the shell policy from.
    pub policy_path: Option<PathBuf>,
}

/// Inlay-hint toggles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HintsConfig {
    /// Master inlay-hint switch.
    pub inlay: bool,
}

impl Default for HintsConfig {
    fn default() -> Self {
        Self { inlay: true }
    }
}

/// Code-action category toggles. Categories absent from the map are enabled.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeActionsConfig {
    /// Per-category enable/disable overrides.
    pub categories: BTreeMap<String, bool>,
}

/// Formatting behavior (`textDocument/formatting` → `Markdown::cleanup`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FormattingConfig {
    /// Cleanup variant.
    pub cleanup: CleanupVariant,
    /// Reflow prose to this display-column width.
    pub fixed_width: Option<u16>,
}

/// Document-symbol behavior.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SymbolsConfig {
    /// Include top-level frontmatter keys in the document outline.
    pub frontmatter: bool,
}

/// Semantic-token behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SemanticTokensConfig {
    /// Master switch for semantic-token emission. When `false`, full and range
    /// requests return an empty stream (F1/F2/F4 all suppressed).
    pub enable: bool,
}

impl Default for SemanticTokensConfig {
    fn default() -> Self {
        Self { enable: true }
    }
}

/// Diagnostics scheduling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiagnosticsConfig {
    /// Debounce for immediate-tier diagnostics after the last edit.
    pub debounce_ms: u64,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self { debounce_ms: 200 }
    }
}

/// The full DMLS configuration.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DmlsConfig {
    /// Workspace root override and discovery globs.
    pub workspace: WorkspaceConfig,
    /// Wiki-link behavior.
    pub wiki: WikiConfig,
    /// Frontmatter schema behavior and extensions.
    pub schema: SchemaConfig,
    /// `style:` frontmatter behavior.
    pub style: StyleConfig,
    /// Shell-policy discovery.
    pub shell: ShellConfig,
    /// Inlay-hint toggles.
    pub hints: HintsConfig,
    /// Code-action category toggles.
    pub code_actions: CodeActionsConfig,
    /// Formatting behavior.
    pub formatting: FormattingConfig,
    /// Document-symbol behavior.
    pub symbols: SymbolsConfig,
    /// Semantic-token behavior.
    pub semantic_tokens: SemanticTokensConfig,
    /// Diagnostics scheduling.
    pub diagnostics: DiagnosticsConfig,
}

impl DmlsConfig {
    /// Loads a config file.
    ///
    /// ## Errors
    ///
    /// [`ConfigError::Io`] when the file cannot be read and
    /// [`ConfigError::Parse`] when it is not valid config TOML.
    pub fn load_file(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
    }

    /// Discovers `.dmls.toml` at the workspace roots (first hit wins).
    ///
    /// Unreadable or malformed files are logged and skipped so a broken
    /// config degrades to defaults instead of killing the server.
    pub fn discover(roots: &[PathBuf]) -> (Self, Option<PathBuf>) {
        for root in roots {
            let candidate = root.join(CONFIG_FILE_NAME);
            if !candidate.is_file() {
                continue;
            }
            match Self::load_file(&candidate) {
                Ok(config) => return (config, Some(candidate)),
                Err(error) => {
                    tracing::warn!("ignoring config {}: {error}", candidate.display());
                }
            }
        }
        (Self::default(), None)
    }
}

/// The three config sources and the effective merge.
///
/// The file layer reloads from disk; the overlay layer holds the most recent
/// `workspace/configuration` / `didChangeConfiguration` settings. The
/// effective config is file → overlay deep-merged as JSON, then
/// deserialized; a merge that no longer deserializes keeps the previous
/// effective config (logged), so bad editor settings cannot wedge the
/// server.
#[derive(Debug)]
pub struct ConfigState {
    workspace_roots: Vec<PathBuf>,
    /// Explicit `--config` path; wins over discovery when set.
    explicit_path: Option<PathBuf>,
    file_path: Option<PathBuf>,
    file: DmlsConfig,
    overlay: Option<serde_json::Value>,
    effective: DmlsConfig,
}

impl ConfigState {
    /// Loads config for the given workspace roots, honoring an explicit
    /// `--config` path when provided.
    pub fn load(workspace_roots: Vec<PathBuf>, explicit_path: Option<PathBuf>) -> Self {
        let (file, file_path) = match &explicit_path {
            Some(path) => match DmlsConfig::load_file(path) {
                Ok(config) => (config, Some(path.clone())),
                Err(error) => {
                    tracing::warn!("ignoring --config {}: {error}", path.display());
                    (DmlsConfig::default(), None)
                }
            },
            None => DmlsConfig::discover(&workspace_roots),
        };
        let mut state = Self {
            workspace_roots,
            explicit_path,
            file_path,
            file,
            overlay: None,
            effective: DmlsConfig::default(),
        };
        state.recompute();
        state
    }

    /// The merged, effective configuration.
    pub fn effective(&self) -> &DmlsConfig {
        &self.effective
    }

    /// The config file currently in use, if any.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Applies `workspace/didChangeConfiguration` settings and reloads the
    /// config file in the same pass (editors don't watch `.dmls.toml` for
    /// us in Phase 2).
    ///
    /// Accepts either `{ "dmls": { ... } }` or the bare section object; a
    /// `null` settings value clears the overlay.
    pub fn apply_client_settings(&mut self, settings: serde_json::Value) {
        self.overlay = match settings {
            serde_json::Value::Null => None,
            serde_json::Value::Object(mut map) => match map.remove("dmls") {
                Some(serde_json::Value::Null) | None if map.is_empty() => None,
                Some(section) => Some(section),
                None => Some(serde_json::Value::Object(map)),
            },
            other => {
                tracing::warn!("ignoring non-object configuration settings: {other}");
                self.overlay.take()
            }
        };
        self.reload();
    }

    /// Re-reads the config file and recomputes the effective config.
    pub fn reload(&mut self) {
        let (file, file_path) = match &self.explicit_path {
            Some(path) => match DmlsConfig::load_file(path) {
                Ok(config) => (config, Some(path.clone())),
                Err(error) => {
                    tracing::warn!("ignoring --config {}: {error}", path.display());
                    (DmlsConfig::default(), None)
                }
            },
            None => DmlsConfig::discover(&self.workspace_roots),
        };
        self.file = file;
        self.file_path = file_path;
        self.recompute();
    }

    fn recompute(&mut self) {
        let mut merged = serde_json::to_value(&self.file)
            .expect("DmlsConfig always serializes to a JSON object");
        if let Some(overlay) = &self.overlay {
            merge_json(&mut merged, overlay);
        }
        match serde_json::from_value::<DmlsConfig>(merged) {
            Ok(effective) => self.effective = effective,
            Err(error) => {
                tracing::warn!("configuration overlay produced an invalid config: {error}");
                self.effective = self.file.clone();
            }
        }
    }
}

/// Deep-merges `overlay` into `base`: objects merge key-wise, everything
/// else replaces.
fn merge_json(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(slot) => merge_json(slot, value),
                    None => {
                        base_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (slot, value) => *slot = value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_defaults() {
        let config = DmlsConfig::default();
        assert!(config.wiki.enable);
        assert_eq!(config.wiki.path_style, WikiPathStyle::Shortest);
        assert_eq!(
            config.wiki.heading_completion_style,
            HeadingCompletionStyle::Text
        );
        assert!(!config.schema.strict);
        assert!(!config.style.strict);
        assert!(config.hints.inlay);
        assert!(config.semantic_tokens.enable);
        assert_eq!(config.diagnostics.debounce_ms, 200);
        assert_eq!(config.formatting.cleanup, CleanupVariant::Default);
        assert_eq!(config.formatting.fixed_width, None);
        assert_eq!(
            config.workspace.include,
            vec!["**/*.md".to_string(), "**/*.markdown".to_string()]
        );
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_text = r#"
            [workspace]
            root = "docs"
            include = ["**/*.md"]
            exclude = ["target/**"]

            [wiki]
            enable = true
            wiki_root = "notes"
            path_style = "root-relative"
            heading_completion_style = "slug"

            [schema]
            strict = true

            [schema.extensions.claudine]
            path = "darkmatter/docs/schemas/claudine.yaml"
            globs = [".claude/**", "prompts/**"]

            [style]
            strict = true

            [shell]
            policy_path = ".darkmatter/policy.toml"

            [hints]
            inlay = false

            [semantic_tokens]
            enable = false

            [code_actions.categories]
            "create-missing-file" = false

            [formatting]
            cleanup = "compact"
            fixed_width = 80

            [diagnostics]
            debounce_ms = 150
        "#;
        let config: DmlsConfig = toml::from_str(toml_text).unwrap();
        assert_eq!(config.workspace.root, Some(PathBuf::from("docs")));
        assert_eq!(config.workspace.exclude, vec!["target/**".to_string()]);
        assert_eq!(config.wiki.wiki_root, Some(PathBuf::from("notes")));
        assert_eq!(config.wiki.path_style, WikiPathStyle::RootRelative);
        assert_eq!(
            config.wiki.heading_completion_style,
            HeadingCompletionStyle::Slug
        );
        assert!(config.schema.strict);
        let claudine = &config.schema.extensions["claudine"];
        assert_eq!(
            claudine.path,
            PathBuf::from("darkmatter/docs/schemas/claudine.yaml")
        );
        assert_eq!(claudine.globs.len(), 2);
        assert!(config.style.strict);
        assert_eq!(
            config.shell.policy_path,
            Some(PathBuf::from(".darkmatter/policy.toml"))
        );
        assert!(!config.hints.inlay);
        assert!(!config.semantic_tokens.enable);
        assert_eq!(
            config.code_actions.categories.get("create-missing-file"),
            Some(&false)
        );
        assert_eq!(config.formatting.cleanup, CleanupVariant::Compact);
        assert_eq!(config.formatting.fixed_width, Some(80));
        assert_eq!(config.diagnostics.debounce_ms, 150);
    }

    #[test]
    fn test_partial_toml_keeps_defaults() {
        let config: DmlsConfig = toml::from_str("[wiki]\nenable = false\n").unwrap();
        assert!(!config.wiki.enable);
        assert_eq!(config.diagnostics.debounce_ms, 200);
        assert!(config.hints.inlay);
    }

    #[test]
    fn test_discover_finds_first_root() {
        let temp = tempfile::tempdir().unwrap();
        let root_a = temp.path().join("a");
        let root_b = temp.path().join("b");
        std::fs::create_dir_all(&root_a).unwrap();
        std::fs::create_dir_all(&root_b).unwrap();
        std::fs::write(root_b.join(CONFIG_FILE_NAME), "[wiki]\nenable = false\n").unwrap();
        let (config, path) = DmlsConfig::discover(&[root_a, root_b.clone()]);
        assert!(!config.wiki.enable);
        assert_eq!(path, Some(root_b.join(CONFIG_FILE_NAME)));
    }

    #[test]
    fn test_discover_malformed_degrades_to_defaults() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(CONFIG_FILE_NAME), "not [valid toml").unwrap();
        let (config, path) = DmlsConfig::discover(&[temp.path().to_path_buf()]);
        assert_eq!(config, DmlsConfig::default());
        assert_eq!(path, None);
    }

    #[test]
    fn test_config_state_overlay_wins_over_file() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "[diagnostics]\ndebounce_ms = 150\n[wiki]\nenable = false\n",
        )
        .unwrap();
        let mut state = ConfigState::load(vec![temp.path().to_path_buf()], None);
        assert_eq!(state.effective().diagnostics.debounce_ms, 150);

        state.apply_client_settings(json!({
            "dmls": { "diagnostics": { "debounce_ms": 300 } }
        }));
        // Overlay wins where set; file survives where not.
        assert_eq!(state.effective().diagnostics.debounce_ms, 300);
        assert!(!state.effective().wiki.enable);

        // Clearing the overlay restores the file layer.
        state.apply_client_settings(serde_json::Value::Null);
        assert_eq!(state.effective().diagnostics.debounce_ms, 150);
    }

    #[test]
    fn test_semantic_tokens_defaults_enabled() {
        let config: DmlsConfig = toml::from_str("").unwrap();
        assert!(config.semantic_tokens.enable);
    }

    #[test]
    fn test_semantic_tokens_explicit_false() {
        let config: DmlsConfig =
            toml::from_str("[semantic_tokens]\nenable = false\n").unwrap();
        assert!(!config.semantic_tokens.enable);
    }

    #[test]
    fn test_semantic_tokens_partial_overlay_update() {
        // A `workspace/configuration` overlay flips the master switch off while
        // leaving the rest of the config at its file/default values.
        let mut state = ConfigState::load(Vec::new(), None);
        assert!(state.effective().semantic_tokens.enable);
        state.apply_client_settings(json!({
            "dmls": { "semantic_tokens": { "enable": false } }
        }));
        assert!(!state.effective().semantic_tokens.enable);
        // Unrelated sections are untouched by the partial update.
        assert!(state.effective().wiki.enable);
        // Clearing the overlay restores the default-enabled state.
        state.apply_client_settings(serde_json::Value::Null);
        assert!(state.effective().semantic_tokens.enable);
    }

    #[test]
    fn test_semantic_tokens_malformed_value_keeps_previous() {
        // A non-boolean `enable` fails to deserialize the overlay, so the last
        // good (default-enabled) config survives rather than wedging the server.
        let mut state = ConfigState::load(Vec::new(), None);
        state.apply_client_settings(json!({
            "dmls": { "semantic_tokens": { "enable": "yes-please" } }
        }));
        assert!(state.effective().semantic_tokens.enable);
    }

    #[test]
    fn test_config_state_bare_section_settings() {
        let mut state = ConfigState::load(Vec::new(), None);
        state.apply_client_settings(json!({ "schema": { "strict": true } }));
        assert!(state.effective().schema.strict);
    }

    #[test]
    fn test_config_state_invalid_overlay_keeps_file_layer() {
        let mut state = ConfigState::load(Vec::new(), None);
        state.apply_client_settings(json!({
            "dmls": { "diagnostics": { "debounce_ms": "not-a-number" } }
        }));
        assert_eq!(state.effective().diagnostics.debounce_ms, 200);
    }

    #[test]
    fn test_config_state_reload_picks_up_file_changes() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join(CONFIG_FILE_NAME);
        std::fs::write(&config_path, "[style]\nstrict = false\n").unwrap();
        let mut state = ConfigState::load(vec![temp.path().to_path_buf()], None);
        assert!(!state.effective().style.strict);

        std::fs::write(&config_path, "[style]\nstrict = true\n").unwrap();
        state.reload();
        assert!(state.effective().style.strict);
    }

    #[test]
    fn test_explicit_config_path_wins_over_discovery() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join(CONFIG_FILE_NAME),
            "[hints]\ninlay = true\n",
        )
        .unwrap();
        let explicit = temp.path().join("custom.toml");
        std::fs::write(&explicit, "[hints]\ninlay = false\n").unwrap();
        let state = ConfigState::load(vec![temp.path().to_path_buf()], Some(explicit.clone()));
        assert!(!state.effective().hints.inlay);
        assert_eq!(state.file_path(), Some(explicit.as_path()));
    }
}
