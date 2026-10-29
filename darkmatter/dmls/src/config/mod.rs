//! `DmlsConfig`: the server configuration model and its three sources.
//!
//! Precedence (design "Configuration"): editor `didChangeConfiguration`
//! overlay > `.dmls.toml` at the workspace root > built-in defaults.
//! Frontmatter never configures the server. All spec keys are parsed now
//! even where their consumers land in later phases, so configs written
//! today stay valid as features arrive.
//!
//! `formatting` is read differently from every other section. Each authored
//! source (the file, the editor overlay) is validated and normalized on its
//! own into a [`MarkdownRuleOverrides`], translating the legacy
//! `formatting.cleanup` / `formatting.fixed_width` keys, and the two are then
//! merged leaf by leaf: an overlay leaf with a value wins, and an absent or
//! `null` overlay leaf releases to the file's leaf. Serialized defaults never
//! enter that merge, so they cannot manufacture legacy keys. A source whose
//! `formatting` is invalid contributes no formatting overrides at all. The
//! document's own `style.markdown` and CLI flags rank above the merged
//! workspace overrides; see `darkmatter::markdown::cleanup::rules::resolve`.
//!
//! Findings from a load are kept as [`ConfigDiagnostic`]s on
//! [`ConfigState::diagnostics`], recomputed (never accumulated) on every load,
//! reload, and settings change.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use darkmatter::markdown::cleanup::rules::{
    FIXED_WIDTH_MAX_COLUMNS, FixedWidth, Leaf, ListItemPadding, MarkdownRuleOverrides,
    rules_from_json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// Workspace/library root and discovery globs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    /// Optional root override: a relative path is joined onto each LSP
    /// workspace folder, and an absolute path replaces them.
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
    /// SimplifiedSchema YAML path (relative paths resolve against the first
    /// workspace folder, not `workspace.root`).
    pub path: PathBuf,
    /// Activation globs (e.g. `.claude/**`).
    #[serde(default)]
    pub globs: Vec<String>,
}

/// Frontmatter schema behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SchemaConfig {
    /// Strict mode: unknown frontmatter keys become errors rather than
    /// warnings, and missing required keys are diagnosed.
    pub strict: bool,
    /// Baseline schema extensions by name.
    pub extensions: BTreeMap<String, SchemaExtensionConfig>,
    /// The total, in milliseconds, at or above which a document's schema
    /// validation is logged at `warn` rather than `debug`; zero logs every
    /// one at `warn`. Read only by the log, never by validation.
    pub slow_validation_ms: u64,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            strict: false,
            extensions: BTreeMap::new(),
            slow_validation_ms: 200,
        }
    }
}

/// `style:` frontmatter behavior.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleConfig {
    /// Strict mode: unknown/deprecated style keys become diagnostics.
    pub strict: bool,
}

/// Expression call diagnostics.
///
/// The only setting for call diagnostics: no function has a setting of its
/// own.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpressionsConfig {
    /// Strict mode: a call argument that may be `null` where the parameter
    /// rejects it (category 2) or whose type is unknown (category 4) warns
    /// too. Proven errors and possible conversion failures are shown either
    /// way.
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

/// Formatting behavior (`textDocument/formatting`).
///
/// Never serialized or deserialized implicitly: [`DmlsConfig::load_file`] and
/// [`ConfigState`] read each source's `formatting` table through the shared
/// rule validator, so this holds only validated, normalized overrides.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormattingConfig {
    /// The workspace level of the `style.markdown` rules: the file's
    /// `formatting.markdown` (or its translated legacy keys) with the editor
    /// overlay merged over it leaf by leaf.
    pub markdown: MarkdownRuleOverrides,
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
    /// Expression call diagnostics.
    pub expressions: ExpressionsConfig,
    /// Shell-policy discovery.
    pub shell: ShellConfig,
    /// Inlay-hint toggles.
    pub hints: HintsConfig,
    /// Code-action category toggles.
    pub code_actions: CodeActionsConfig,
    /// Formatting behavior; read separately from the other sections.
    #[serde(skip)]
    pub formatting: FormattingConfig,
    /// Document-symbol behavior.
    pub symbols: SymbolsConfig,
    /// Semantic-token behavior.
    pub semantic_tokens: SemanticTokensConfig,
    /// Diagnostics scheduling.
    pub diagnostics: DiagnosticsConfig,
}

impl DmlsConfig {
    /// Loads a config file, reading its `formatting` table through the shared
    /// rule validator.
    ///
    /// ## Errors
    ///
    /// [`ConfigError::Io`] when the file cannot be read and
    /// [`ConfigError::Parse`] when it is not valid TOML (including duplicate
    /// keys and trailing garbage) or does not fit the config shape. An invalid
    /// `formatting` table is not an error: the file's other settings load,
    /// its formatting overrides are dropped, and the finding is in
    /// [`LoadedConfig::diagnostics`].
    pub fn load_file(path: &Path) -> Result<LoadedConfig, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let parse_error = |source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let table: toml::Table = toml::from_str(&text).map_err(parse_error)?;
        let formatting = table.get("formatting").cloned();
        let mut config: DmlsConfig = toml::Value::Table(table).try_into().map_err(parse_error)?;

        let source = ConfigSource::File(path.to_path_buf());
        let mut diagnostics = Vec::new();
        match formatting
            .as_ref()
            .map(|value| toml_to_json(value, "formatting"))
            .transpose()
        {
            Ok(section) => {
                config.formatting.markdown =
                    normalize_formatting(section.as_ref(), &source, &mut diagnostics);
            }
            Err((key, message)) => diagnostics.push(source.rejected(key, message)),
        }
        Ok(LoadedConfig {
            config,
            diagnostics,
        })
    }

    /// Discovers `.dmls.toml` at the workspace roots (first loadable hit
    /// wins).
    ///
    /// Unreadable or malformed files are skipped so a broken config degrades
    /// to defaults instead of killing the server; each one leaves an error in
    /// the returned diagnostics.
    pub fn discover(roots: &[PathBuf]) -> (LoadedConfig, Option<PathBuf>) {
        let mut skipped = Vec::new();
        for root in roots {
            let candidate = root.join(CONFIG_FILE_NAME);
            if !candidate.is_file() {
                continue;
            }
            match Self::load_file(&candidate) {
                Ok(mut loaded) => {
                    skipped.append(&mut loaded.diagnostics);
                    loaded.diagnostics = skipped;
                    return (loaded, Some(candidate));
                }
                Err(error) => skipped.push(error.into_diagnostic()),
            }
        }
        (
            LoadedConfig {
                config: Self::default(),
                diagnostics: skipped,
            },
            None,
        )
    }
}

/// A loaded config and the findings loading it produced.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedConfig {
    /// The loaded settings; `formatting.markdown` holds this source's
    /// normalized overrides.
    pub config: DmlsConfig,
    /// Migration warnings and rejected settings, in discovery order.
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl ConfigError {
    /// The visible record of a file that contributed nothing.
    fn into_diagnostic(self) -> ConfigDiagnostic {
        let path = match &self {
            ConfigError::Io { path, .. } | ConfigError::Parse { path, .. } => path.clone(),
        };
        ConfigDiagnostic {
            severity: ConfigSeverity::Error,
            source: ConfigSource::File(path),
            key: None,
            message: format!("{self}; none of its settings were applied"),
        }
    }
}

/// Where a configuration finding came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// A `.dmls.toml` (or `--config`) file.
    File(PathBuf),
    /// The editor's `didChangeConfiguration` settings.
    EditorSettings,
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigSource::File(path) => write!(f, "{}", path.display()),
            ConfigSource::EditorSettings => f.write_str("editor settings"),
        }
    }
}

impl ConfigSource {
    /// What this source calls a mapping, for messages.
    fn mapping(&self) -> &'static str {
        match self {
            ConfigSource::File(_) => "a table",
            ConfigSource::EditorSettings => "an object",
        }
    }

    fn rejected(&self, key: impl Into<String>, message: impl fmt::Display) -> ConfigDiagnostic {
        ConfigDiagnostic {
            severity: ConfigSeverity::Error,
            source: self.clone(),
            key: Some(key.into()),
            message: format!("{message}; no formatting settings from this source were applied"),
        }
    }

    fn migrated(&self, key: &str, message: impl fmt::Display) -> ConfigDiagnostic {
        ConfigDiagnostic {
            severity: ConfigSeverity::Warning,
            source: self.clone(),
            key: Some(key.to_string()),
            message: message.to_string(),
        }
    }
}

/// How serious a configuration finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSeverity {
    /// The setting was applied, possibly translated (legacy migration).
    Warning,
    /// Settings were rejected and not applied.
    Error,
}

/// One visible configuration finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    /// Warning (applied) or error (not applied).
    pub severity: ConfigSeverity,
    /// The file or the editor settings.
    pub source: ConfigSource,
    /// The dotted setting path (`formatting.markdown.fixed-width`); `None`
    /// when the whole source is at fault.
    pub key: Option<String>,
    /// What happened, phrased for the author.
    pub message: String,
}

impl fmt::Display for ConfigDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.key {
            Some(key) => write!(f, "{}: `{key}`: {}", self.source, self.message),
            None => write!(f, "{}: {}", self.source, self.message),
        }
    }
}

/// The three config sources and the effective merge.
///
/// The file layer reloads from disk; the overlay layer holds the most recent
/// `didChangeConfiguration` settings. For every section but `formatting`, the
/// effective config is file → overlay deep-merged as JSON, then deserialized;
/// a merge that no longer deserializes falls back to the file layer (the
/// file's settings over defaults), not to the previous effective config, and
/// records an error diagnostic, so bad editor settings cannot wedge the
/// server. An explicit `null` for a section or a typed leaf (for example
/// `expressions` or `expressions.strict`) is such an invalid merge; only a
/// whole-settings `null` clears the overlay.
/// `formatting` is normalized per source and merged leaf by leaf (see the
/// module docs).
#[derive(Debug)]
pub struct ConfigState {
    workspace_roots: Vec<PathBuf>,
    /// Explicit `--config` path; wins over discovery when set.
    explicit_path: Option<PathBuf>,
    file_path: Option<PathBuf>,
    file: DmlsConfig,
    file_diagnostics: Vec<ConfigDiagnostic>,
    overlay: Option<Value>,
    effective: DmlsConfig,
    diagnostics: Vec<ConfigDiagnostic>,
}

impl ConfigState {
    /// Loads config for the given workspace roots, honoring an explicit
    /// `--config` path when provided.
    pub fn load(workspace_roots: Vec<PathBuf>, explicit_path: Option<PathBuf>) -> Self {
        let mut state = Self {
            workspace_roots,
            explicit_path,
            file_path: None,
            file: DmlsConfig::default(),
            file_diagnostics: Vec::new(),
            overlay: None,
            effective: DmlsConfig::default(),
            diagnostics: Vec::new(),
        };
        state.reload();
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

    /// Findings from the most recent load, reload, or settings change: file
    /// findings first, then editor-settings findings. Replaced, never
    /// accumulated, so each authored legacy key appears once per change.
    pub fn diagnostics(&self) -> &[ConfigDiagnostic] {
        &self.diagnostics
    }

    /// Applies `workspace/didChangeConfiguration` settings and reloads the
    /// config file in the same pass, so a client without a reliable file
    /// watcher still picks up `.dmls.toml` edits.
    ///
    /// Accepts either `{ "dmls": { ... } }` or the bare section object; a
    /// `null` settings value clears the overlay. Any other non-object value
    /// keeps the previous overlay and records an error.
    pub fn apply_client_settings(&mut self, settings: Value) {
        let mut rejected = None;
        self.overlay = match settings {
            Value::Null => None,
            Value::Object(mut map) => match map.remove("dmls") {
                Some(Value::Null) | None if map.is_empty() => None,
                Some(section) => Some(section),
                None => Some(Value::Object(map)),
            },
            other => {
                rejected = Some(ConfigDiagnostic {
                    severity: ConfigSeverity::Error,
                    source: ConfigSource::EditorSettings,
                    key: None,
                    message: format!(
                        "settings must be an object, found {}; the previous settings remain",
                        describe(&other)
                    ),
                });
                self.overlay.take()
            }
        };
        self.reload();
        if let Some(rejected) = rejected {
            let at = self.file_diagnostics.len();
            self.diagnostics.insert(at, rejected);
        }
    }

    /// Re-reads the config file and recomputes the effective config.
    pub fn reload(&mut self) {
        let (loaded, file_path) = match &self.explicit_path {
            Some(path) => match DmlsConfig::load_file(path) {
                Ok(loaded) => (loaded, Some(path.clone())),
                Err(error) => (
                    LoadedConfig {
                        config: DmlsConfig::default(),
                        diagnostics: vec![error.into_diagnostic()],
                    },
                    None,
                ),
            },
            None => DmlsConfig::discover(&self.workspace_roots),
        };
        for diagnostic in &loaded.diagnostics {
            tracing::warn!("{diagnostic}");
        }
        self.file = loaded.config;
        self.file_diagnostics = loaded.diagnostics;
        self.file_path = file_path;
        self.recompute();
    }

    fn recompute(&mut self) {
        let mut diagnostics = self.file_diagnostics.clone();
        let mut overlay = self.overlay.clone();
        // `None` = the overlay has no `formatting` key (absent, not null).
        let overlay_formatting = match &mut overlay {
            Some(Value::Object(map)) => map.remove("formatting"),
            _ => None,
        };

        let mut merged =
            serde_json::to_value(&self.file).expect("DmlsConfig always serializes to a JSON object");
        if let Some(overlay) = &overlay {
            merge_json(&mut merged, overlay);
        }
        self.effective = match serde_json::from_value::<DmlsConfig>(merged) {
            Ok(effective) => effective,
            Err(error) => {
                diagnostics.push(ConfigDiagnostic {
                    severity: ConfigSeverity::Error,
                    source: ConfigSource::EditorSettings,
                    key: None,
                    message: format!(
                        "settings produced an invalid config ({error}); the file and default \
                         settings remain"
                    ),
                });
                self.file.clone()
            }
        };

        let first_overlay_finding = diagnostics.len();
        let overlay_rules = normalize_formatting(
            overlay_formatting.as_ref(),
            &ConfigSource::EditorSettings,
            &mut diagnostics,
        );
        self.effective.formatting.markdown =
            layer_rules(&overlay_rules, &self.file.formatting.markdown);
        for diagnostic in &diagnostics[first_overlay_finding..] {
            tracing::warn!("{diagnostic}");
        }
        self.diagnostics = diagnostics;
    }
}

/// Merges two levels leaf by leaf: an `upper` value wins; otherwise a `lower`
/// value; otherwise `upper`'s explicit `null` is kept so it stays visible.
fn layer_rules(upper: &MarkdownRuleOverrides, lower: &MarkdownRuleOverrides) -> MarkdownRuleOverrides {
    fn leaf<T: Clone>(upper: &Leaf<T>, lower: &Leaf<T>) -> Leaf<T> {
        match (upper, lower) {
            (Leaf::Value(_), _) => upper.clone(),
            (_, Leaf::Value(_)) => lower.clone(),
            (Leaf::Null, _) => Leaf::Null,
            (Leaf::Absent, _) => lower.clone(),
        }
    }
    MarkdownRuleOverrides {
        frontmatter: leaf(&upper.frontmatter, &lower.frontmatter),
        table_layout: leaf(&upper.table_layout, &lower.table_layout),
        fixed_width: leaf(&upper.fixed_width, &lower.fixed_width),
        unwrap: leaf(&upper.unwrap, &lower.unwrap),
        indentation: leaf(&upper.indentation, &lower.indentation),
        ordered_lists: leaf(&upper.ordered_lists, &lower.ordered_lists),
        list_padding: leaf(&upper.list_padding, &lower.list_padding),
        list_item_padding: leaf(&upper.list_item_padding, &lower.list_item_padding),
        list_interior_padding: leaf(&upper.list_interior_padding, &lower.list_interior_padding),
        default_language: leaf(&upper.default_language, &lower.default_language),
    }
}

/// The keys `formatting` accepts; the last two are legacy.
const FORMATTING_KEYS: [&str; 3] = ["markdown", "cleanup", "fixed_width"];

/// Validates and normalizes one source's `formatting` value (`None` when the
/// key is absent).
///
/// ## Returns
///
/// The source's overrides, or no overrides when anything in its `formatting`
/// is rejected. Each authored legacy key adds exactly one diagnostic: a
/// migration warning, or an error when its value is unsupported. A source that
/// mixes a legacy key with `markdown` gets one error instead.
fn normalize_formatting(
    section: Option<&Value>,
    source: &ConfigSource,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) -> MarkdownRuleOverrides {
    // `None` means rejected, already reported in `diagnostics`: the source
    // contributes no overrides.
    read_formatting(section, source, diagnostics).unwrap_or_default()
}

fn read_formatting(
    section: Option<&Value>,
    source: &ConfigSource,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) -> Option<MarkdownRuleOverrides> {
    let map = match section {
        None | Some(Value::Null) => return Some(MarkdownRuleOverrides::default()),
        Some(Value::Object(map)) => map,
        Some(other) => {
            diagnostics.push(source.rejected(
                "formatting",
                format!("must be {}, found {}", source.mapping(), describe(other)),
            ));
            return None;
        }
    };

    let mut accepted = true;
    for key in map.keys() {
        if !FORMATTING_KEYS.contains(&key.as_str()) {
            diagnostics.push(source.rejected(
                format!("formatting.{key}"),
                "unknown formatting setting; expected `markdown`",
            ));
            accepted = false;
        }
    }

    // Key presence decides the shape: even `markdown = {}` or a `null`
    // legacy value makes the source mixed.
    let legacy: Vec<&str> = ["cleanup", "fixed_width"]
        .into_iter()
        .filter(|key| map.contains_key(*key))
        .collect();
    if map.contains_key("markdown") && !legacy.is_empty() {
        let names = legacy
            .iter()
            .map(|key| format!("`formatting.{key}`"))
            .collect::<Vec<_>>()
            .join(" and ");
        diagnostics.push(source.rejected(
            "formatting",
            format!(
                "mixes the legacy {names} with `formatting.markdown`; express them as \
                 `formatting.markdown` rules instead"
            ),
        ));
        return None;
    }

    let mut overrides = MarkdownRuleOverrides::default();
    match map.get("markdown") {
        None | Some(Value::Null) => {}
        Some(markdown @ Value::Object(_)) => match rules_from_json(markdown) {
            Ok(rules) => overrides = rules,
            Err(errors) => {
                for error in errors {
                    let key = match error.key {
                        Some(name) => format!("formatting.markdown.{name}"),
                        None => "formatting.markdown".to_string(),
                    };
                    diagnostics.push(source.rejected(key, error.message));
                }
                accepted = false;
            }
        },
        Some(other) => {
            diagnostics.push(source.rejected(
                "formatting.markdown",
                format!("must be {}, found {}", source.mapping(), describe(other)),
            ));
            accepted = false;
        }
    }

    if let Some(cleanup) = map.get("cleanup") {
        let key = "formatting.cleanup";
        let deprecated = "`formatting.cleanup` is deprecated";
        match cleanup {
            Value::String(variant) if variant == "compact" || variant == "loose" => {
                let (padding, spelled) = if variant == "compact" {
                    (ListItemPadding::None, "none")
                } else {
                    (ListItemPadding::One, "one")
                };
                overrides.list_item_padding = Leaf::Value(padding);
                diagnostics.push(source.migrated(
                    key,
                    format!(
                        "{deprecated}; \"{variant}\" was applied as \
                         `formatting.markdown.list-item-padding: {spelled}`"
                    ),
                ));
            }
            Value::String(variant) if variant == "default" => diagnostics.push(source.migrated(
                key,
                format!("{deprecated}; \"default\" sets no override and can be removed"),
            )),
            Value::Null => diagnostics.push(source.migrated(
                key,
                format!("{deprecated}; null sets no override and can be removed"),
            )),
            other => {
                diagnostics.push(source.rejected(
                    key,
                    format!(
                        "{deprecated} and accepted only \"default\", \"compact\", or \"loose\", \
                         found {}",
                        describe(other)
                    ),
                ));
                accepted = false;
            }
        }
    }

    if let Some(width) = map.get("fixed_width") {
        let key = "formatting.fixed_width";
        let deprecated = "`formatting.fixed_width` is deprecated";
        let columns = width
            .as_u64()
            .filter(|columns| (1..=u64::from(FIXED_WIDTH_MAX_COLUMNS)).contains(columns))
            .and_then(|columns| u16::try_from(columns).ok());
        match (width, columns) {
            (_, Some(columns)) => {
                overrides.fixed_width = Leaf::Value(FixedWidth::Columns(columns));
                diagnostics.push(source.migrated(
                    key,
                    format!("{deprecated}; applied as `formatting.markdown.fixed-width: {columns}`"),
                ));
            }
            (Value::Null, None) => diagnostics.push(source.migrated(
                key,
                format!("{deprecated}; null sets no override and can be removed"),
            )),
            (other, None) => {
                diagnostics.push(source.rejected(
                    key,
                    format!(
                        "{deprecated} and accepted only a whole number from 1 to \
                         {FIXED_WIDTH_MAX_COLUMNS}, found {}",
                        describe(other)
                    ),
                ));
                accepted = false;
            }
        }
    }

    accepted.then_some(overrides)
}

/// Converts a TOML value to JSON for the shared rule validator.
///
/// ## Errors
///
/// The dotted key and reason for a value JSON cannot carry faithfully: a
/// date/time, or a non-finite float.
fn toml_to_json(value: &toml::Value, key: &str) -> Result<Value, (String, String)> {
    Ok(match value {
        toml::Value::String(text) => Value::String(text.clone()),
        toml::Value::Integer(number) => Value::from(*number),
        toml::Value::Float(number) => match serde_json::Number::from_f64(*number) {
            Some(number) => Value::Number(number),
            None => return Err((key.to_string(), format!("{number} is not a usable number"))),
        },
        toml::Value::Boolean(flag) => Value::Bool(*flag),
        toml::Value::Datetime(datetime) => {
            return Err((
                key.to_string(),
                format!("a date/time ({datetime}) is not a formatting value"),
            ));
        }
        toml::Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| toml_to_json(item, key))
                .collect::<Result<_, _>>()?,
        ),
        toml::Value::Table(table) => Value::Object(
            table
                .iter()
                .map(|(name, item)| {
                    toml_to_json(item, &format!("{key}.{name}")).map(|item| (name.clone(), item))
                })
                .collect::<Result<_, _>>()?,
        ),
    })
}

/// Describes `value` for a message: scalars by their text, so `"80"` and `80`
/// read differently, collections by their kind.
fn describe(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.to_string(),
        Value::Array(_) => "a list".to_string(),
        Value::Object(_) => "a mapping".to_string(),
    }
}

/// Deep-merges `overlay` into `base`: objects merge key-wise, everything
/// else replaces.
fn merge_json(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
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
        assert!(!config.expressions.strict);
        assert!(config.hints.inlay);
        assert!(config.semantic_tokens.enable);
        assert_eq!(config.diagnostics.debounce_ms, 200);
        assert_eq!(config.formatting.markdown, MarkdownRuleOverrides::default());
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

            [schema.extensions.prompts]
            path = "schemas/prompts.yaml"
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

            [formatting.markdown]
            list-item-padding = "none"
            fixed-width = 80

            [diagnostics]
            debounce_ms = 150
        "#;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(CONFIG_FILE_NAME);
        std::fs::write(&path, toml_text).unwrap();
        let loaded = DmlsConfig::load_file(&path).unwrap();
        assert_eq!(loaded.diagnostics, Vec::new());
        let config = loaded.config;
        assert_eq!(config.workspace.root, Some(PathBuf::from("docs")));
        assert_eq!(config.workspace.exclude, vec!["target/**".to_string()]);
        assert_eq!(config.wiki.wiki_root, Some(PathBuf::from("notes")));
        assert_eq!(config.wiki.path_style, WikiPathStyle::RootRelative);
        assert_eq!(
            config.wiki.heading_completion_style,
            HeadingCompletionStyle::Slug
        );
        assert!(config.schema.strict);
        let prompts = &config.schema.extensions["prompts"];
        assert_eq!(prompts.path, PathBuf::from("schemas/prompts.yaml"));
        assert_eq!(prompts.globs.len(), 2);
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
        assert_eq!(
            config.formatting.markdown,
            MarkdownRuleOverrides {
                list_item_padding: Leaf::Value(ListItemPadding::None),
                fixed_width: Leaf::Value(FixedWidth::Columns(80)),
                ..MarkdownRuleOverrides::default()
            }
        );
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
        let (loaded, path) = DmlsConfig::discover(&[root_a, root_b.clone()]);
        assert!(!loaded.config.wiki.enable);
        assert_eq!(path, Some(root_b.join(CONFIG_FILE_NAME)));
    }

    #[test]
    fn test_discover_malformed_degrades_to_defaults() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(CONFIG_FILE_NAME), "not [valid toml").unwrap();
        let (loaded, path) = DmlsConfig::discover(&[temp.path().to_path_buf()]);
        assert_eq!(loaded.config, DmlsConfig::default());
        assert_eq!(path, None);
        // The skipped file is reported, not only logged.
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(loaded.diagnostics[0].severity, ConfigSeverity::Error);
        assert_eq!(
            loaded.diagnostics[0].source,
            ConfigSource::File(temp.path().join(CONFIG_FILE_NAME))
        );
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
    fn test_semantic_tokens_malformed_value_falls_back_to_file_layer() {
        // A non-boolean `enable` fails to deserialize the merge, so the file
        // layer (here the defaults, enabled) applies rather than wedging the
        // server.
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
        assert_eq!(state.diagnostics().len(), 1);
        assert_eq!(state.diagnostics()[0].source, ConfigSource::EditorSettings);
        assert_eq!(state.diagnostics()[0].severity, ConfigSeverity::Error);
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

    /// `expressions.strict`: the same file/editor layering and serde
    /// validation as every other strict flag.
    mod expressions_strict {
        use super::*;

        /// A state over a workspace whose `.dmls.toml` holds `toml`, or none
        /// for `None`. The directory lives as long as the state is used.
        fn load(toml: Option<&str>) -> (tempfile::TempDir, ConfigState) {
            let temp = tempfile::tempdir().unwrap();
            if let Some(toml) = toml {
                std::fs::write(temp.path().join(CONFIG_FILE_NAME), toml).unwrap();
            }
            let state = ConfigState::load(vec![temp.path().to_path_buf()], None);
            (temp, state)
        }

        fn strict(state: &ConfigState) -> bool {
            state.effective().expressions.strict
        }

        /// `(severity, source is the editor, key)` of every finding.
        fn findings(state: &ConfigState) -> Vec<(ConfigSeverity, bool, Option<String>)> {
            state
                .diagnostics()
                .iter()
                .map(|diagnostic| {
                    (diagnostic.severity, diagnostic.source == ConfigSource::EditorSettings, diagnostic.key.clone())
                })
                .collect()
        }

        #[test]
        fn absent_and_empty_sections_default_to_false() {
            for toml in [None, Some(""), Some("[expressions]\n"), Some("[wiki]\nenable = false\n")] {
                let (_temp, state) = load(toml);
                assert!(!strict(&state), "{toml:?}");
                assert!(state.diagnostics().is_empty(), "{toml:?}: {:?}", state.diagnostics());
            }
            let (_temp, mut state) = load(None);
            for settings in [json!({ "dmls": { "expressions": {} } }), json!({ "dmls": {} }), json!({})] {
                state.apply_client_settings(settings.clone());
                assert!(!strict(&state), "{settings}");
                assert!(state.diagnostics().is_empty(), "{settings}: {:?}", state.diagnostics());
            }
        }

        #[test]
        fn the_file_sets_either_value() {
            for (toml, expected) in [("[expressions]\nstrict = true\n", true), ("[expressions]\nstrict = false\n", false)] {
                let (_temp, state) = load(Some(toml));
                assert_eq!(strict(&state), expected, "{toml}");
                assert!(state.diagnostics().is_empty());
            }
        }

        #[test]
        fn the_editor_overrides_the_file_in_both_directions() {
            let (_temp, mut state) = load(Some("[expressions]\nstrict = false\n"));
            state.apply_client_settings(json!({ "dmls": { "expressions": { "strict": true } } }));
            assert!(strict(&state));
            // The bare section form, as for `schema.strict`.
            state.apply_client_settings(json!({ "expressions": { "strict": true } }));
            assert!(strict(&state));
            // An empty editor table sets nothing, so the file's value stands.
            state.apply_client_settings(json!({ "dmls": { "expressions": {} } }));
            assert!(!strict(&state));

            let (_temp, mut state) = load(Some("[expressions]\nstrict = true\n"));
            state.apply_client_settings(json!({ "dmls": { "expressions": { "strict": false } } }));
            assert!(!strict(&state));
            // A whole-settings `null` clears the overlay.
            state.apply_client_settings(Value::Null);
            assert!(strict(&state));
            assert!(state.diagnostics().is_empty());
        }

        #[test]
        fn invalid_editor_values_are_errors_that_fall_back_to_the_file() {
            for settings in [
                json!({ "dmls": { "expressions": null } }),
                json!({ "dmls": { "expressions": { "strict": null } } }),
                json!({ "dmls": { "expressions": { "strict": "yes" } } }),
                json!({ "dmls": { "expressions": { "strict": 1 } } }),
                json!({ "dmls": { "expressions": 5 } }),
            ] {
                let (_temp, mut state) = load(Some("[expressions]\nstrict = true\n[diagnostics]\ndebounce_ms = 150\n"));
                state.apply_client_settings(settings.clone());
                assert!(strict(&state), "{settings}: the file's value applies");
                assert_eq!(state.effective().diagnostics.debounce_ms, 150, "{settings}");
                assert_eq!(findings(&state), vec![(ConfigSeverity::Error, true, None)], "{settings}");
            }
        }

        #[test]
        fn invalid_file_values_reject_the_file() {
            for toml in [
                "[expressions]\nstrict = \"yes\"\n",
                "[expressions]\nstrict = 1\n",
                "expressions = 5\n",
                // TOML forbids duplicate keys and tables; the loader rejects both.
                "[expressions]\nstrict = true\nstrict = false\n",
                "[expressions]\nstrict = true\n[expressions]\nstrict = false\n",
            ] {
                let (_temp, state) = load(Some(&format!("{toml}[diagnostics]\ndebounce_ms = 150\n")));
                assert!(!strict(&state), "{toml}");
                // None of the file's settings applied.
                assert_eq!(state.effective().diagnostics.debounce_ms, 200, "{toml}");
                assert_eq!(state.file_path(), None, "{toml}");
                assert_eq!(findings(&state), vec![(ConfigSeverity::Error, false, None)], "{toml}");
            }
        }

        /// The fallback after an invalid merge is the file layer, not the
        /// previous effective config: a strict mode the editor turned on is
        /// off again while its settings are invalid, and the diagnostic says
        /// so.
        #[test]
        fn an_invalid_merge_falls_back_to_the_file_layer_not_the_previous_config() {
            let (_temp, mut state) = load(Some("[expressions]\nstrict = false\n"));
            state.apply_client_settings(json!({ "dmls": { "expressions": { "strict": true } } }));
            assert!(strict(&state));

            state.apply_client_settings(json!({ "dmls": { "expressions": { "strict": "yes" } } }));
            assert!(!strict(&state), "the file layer applies, not the previous `true`");
            let [diagnostic] = state.diagnostics() else {
                panic!("one finding: {:?}", state.diagnostics());
            };
            assert_eq!(diagnostic.severity, ConfigSeverity::Error);
            assert_eq!(diagnostic.source, ConfigSource::EditorSettings);
            assert!(
                diagnostic.message.contains("the file and default settings remain"),
                "{diagnostic}"
            );

            // Recovered settings apply again and the finding is gone.
            state.apply_client_settings(json!({ "dmls": { "expressions": { "strict": true } } }));
            assert!(strict(&state));
            assert!(state.diagnostics().is_empty());
        }

        #[test]
        fn a_malformed_file_recovers_on_reload() {
            let (temp, mut state) = load(Some("[expressions]\nstrict = \"yes\"\n"));
            assert!(!strict(&state));
            assert_eq!(state.diagnostics().len(), 1);

            std::fs::write(temp.path().join(CONFIG_FILE_NAME), "[expressions]\nstrict = true\n").unwrap();
            state.reload();
            assert!(strict(&state));
            assert!(state.diagnostics().is_empty());
        }

        #[test]
        fn empty_values_are_wrong_types_not_absent() {
            for toml in ["[expressions]\nstrict = \"\"\n", "[expressions]\nstrict = []\n", "expressions = \"\"\n"] {
                let (_temp, state) = load(Some(&format!("{toml}[diagnostics]\ndebounce_ms = 150\n")));
                assert_eq!(state.effective().diagnostics.debounce_ms, 200, "{toml}");
                assert_eq!(findings(&state), vec![(ConfigSeverity::Error, false, None)], "{toml}");
            }
            for settings in [
                json!({ "dmls": { "expressions": { "strict": "" } } }),
                json!({ "dmls": { "expressions": { "strict": [] } } }),
                json!({ "dmls": { "expressions": "" } }),
            ] {
                let (_temp, mut state) = load(Some("[expressions]\nstrict = true\n"));
                state.apply_client_settings(settings.clone());
                assert!(strict(&state), "{settings}: the file's value applies");
                assert_eq!(findings(&state), vec![(ConfigSeverity::Error, true, None)], "{settings}");
            }
        }

        /// serde reads a sequence as a struct's fields in order, for every
        /// section alike, so a one-element list sets `strict`; this pins that
        /// shared behavior rather than endorsing it.
        #[test]
        fn a_list_section_is_read_positionally_as_every_section_is() {
            let (_temp, mut state) = load(None);
            state.apply_client_settings(json!({ "dmls": { "expressions": [true] } }));
            assert!(strict(&state));
            assert!(state.diagnostics().is_empty());
            state.apply_client_settings(json!({ "dmls": { "expressions": ["yes"] } }));
            assert!(!strict(&state));
            assert_eq!(findings(&state), vec![(ConfigSeverity::Error, true, None)]);
        }

        #[test]
        fn a_syntactically_broken_file_is_rejected_whole() {
            for toml in ["[expressions\nstrict = true\n", "[expressions]\nstrict = true garbage\n"] {
                let (_temp, state) = load(Some(toml));
                assert!(!strict(&state), "{toml}");
                assert_eq!(state.file_path(), None, "{toml}");
                assert_eq!(findings(&state), vec![(ConfigSeverity::Error, false, None)], "{toml}");
            }
        }

        /// Ruling R6: an unknown key in `[expressions]` is ignored, as in
        /// every section except `[formatting]`; a misspelling leaves strict off.
        #[test]
        fn an_unknown_key_is_ignored() {
            let (_temp, state) = load(Some("[expressions]\nstrcit = true\n"));
            assert!(!strict(&state));
            assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
            let (_temp, mut state) = load(None);
            state.apply_client_settings(json!({ "dmls": { "expressions": { "strcit": true } } }));
            assert!(!strict(&state));
            assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
        }

        /// Criterion 42: the section has one setting, and no function has
        /// one of its own.
        #[test]
        fn strict_is_the_only_expressions_setting() {
            assert_eq!(serde_json::to_value(ExpressionsConfig::default()).unwrap(), json!({ "strict": false }));
        }
    }
}
