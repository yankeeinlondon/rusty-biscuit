use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use serde::Deserialize;

use crate::adapter::{
    AdapterCacheStatus, AdapterConfig, AdapterError, AdapterMetadata, AdapterResult,
    ExternalDiagnosticAdapter, external_diagnostic, map_category,
};
use crate::shared::{
    CodeRange, DiagnosticCategory, DiagnosticConfidence, DiagnosticSeverity, ProgrammingLanguage,
};

/// Adapter for the Oxlint JavaScript/TypeScript linter.
///
/// Discovers `oxlint` via explicit config, project-local `node_modules`,
/// or `PATH`. Parses JSON output into normalized Tree Hugger diagnostics.
#[derive(Debug, Clone)]
pub struct OxlintAdapter {
    cached_version: Option<String>,
    cached_path: Option<PathBuf>,
}

impl Default for OxlintAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OxlintAdapter {
    /// Creates a new Oxlint adapter.
    pub fn new() -> Self {
        Self {
            cached_version: None,
            cached_path: None,
        }
    }

    /// Discovers the oxlint binary path.
    ///
    /// Priority:
    /// 1. Explicit `tool_path` in config
    /// 2. Project-local `node_modules/.bin/oxlint`
    /// 3. `oxlint` on `PATH`
    fn discover_tool_path(&self, project_root: &Path, config: &AdapterConfig) -> Option<PathBuf> {
        // 1. Explicit path
        if let Some(path) = &config.tool_path
            && path.exists()
        {
            return Some(path.clone());
        }

        // 2. Project-local node_modules
        let local_oxlint = project_root
            .join("node_modules")
            .join(".bin")
            .join("oxlint");
        if local_oxlint.exists() {
            return Some(local_oxlint);
        }

        // 3. PATH
        which_oxlint().map(PathBuf::from)
    }

    /// Returns the tool version by running `oxlint --version`.
    fn get_version(&mut self, tool_path: &Path) -> Option<String> {
        if let Some(v) = &self.cached_version {
            return Some(v.clone());
        }

        let output = Command::new(tool_path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .ok()?;

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if version.is_empty() {
            return None;
        }

        self.cached_version = Some(version.clone());
        Some(version)
    }

    /// Discovers Oxlint configuration files.
    fn discover_config_files(&self, project_root: &Path, config: &AdapterConfig) -> Vec<PathBuf> {
        let mut configs = Vec::new();

        if let Some(path) = &config.config_path {
            configs.push(path.clone());
        } else {
            let candidates = [".oxlintrc.json", ".oxlintrc", "oxlint.json"];
            for candidate in &candidates {
                let path = project_root.join(candidate);
                if path.exists() {
                    configs.push(path);
                }
            }
        }

        configs
    }

    /// Runs oxlint on the given files and returns parsed JSON output.
    fn run_oxlint(
        &self,
        tool_path: &Path,
        files: &[PathBuf],
        project_root: &Path,
        config: &AdapterConfig,
    ) -> Result<OxlintJsonOutput, AdapterError> {
        let mut cmd = Command::new(tool_path);
        cmd.arg("--format").arg("json").current_dir(project_root);

        // Add config if available
        let config_files = self.discover_config_files(project_root, config);
        if let Some(config_path) = config_files.first() {
            cmd.arg("--config").arg(config_path);
        }

        // Add extra args
        for arg in &config.extra_args {
            cmd.arg(arg);
        }

        // Add files
        for file in files {
            cmd.arg(file);
        }

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| AdapterError::ExecutionFailed {
                tool: "oxlint".to_string(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Oxlint may exit with non-zero when diagnostics are found
        // We still want to parse the output
        let json: OxlintJsonOutput =
            serde_json::from_str(&stdout).map_err(|e| AdapterError::ParseFailed {
                message: format!("Failed to parse oxlint JSON: {e}"),
            })?;

        Ok(json)
    }
}

impl ExternalDiagnosticAdapter for OxlintAdapter {
    fn name(&self) -> &str {
        "oxlint"
    }

    fn version(&self) -> Option<String> {
        self.cached_version.clone()
    }

    fn is_available(&self) -> bool {
        if self.cached_path.is_some() {
            return true;
        }
        which_oxlint().is_some()
    }

    fn supported_languages(&self) -> Vec<ProgrammingLanguage> {
        vec![
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::TypeScript,
        ]
    }

    fn run(
        &self,
        files: &[PathBuf],
        project_root: &Path,
        language: ProgrammingLanguage,
        config: &AdapterConfig,
    ) -> Result<AdapterResult, AdapterError> {
        let start = Instant::now();

        if !matches!(
            language,
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript
        ) {
            return Err(AdapterError::UnsupportedLanguage {
                language,
                adapter: self.name().to_string(),
            });
        }

        let tool_path = self.discover_tool_path(project_root, config);

        if tool_path.is_none() {
            if config.strict {
                return Err(AdapterError::ToolNotFound {
                    tool: "oxlint".to_string(),
                });
            }

            return Ok(AdapterResult {
                diagnostics: Vec::new(),
                metadata: AdapterMetadata {
                    tool_name: "oxlint".to_string(),
                    version: None,
                    config_files: Vec::new(),
                    working_directory: project_root.to_path_buf(),
                    exit_status: None,
                    elapsed_time_ms: start.elapsed().as_millis() as u64,
                    cache_status: AdapterCacheStatus::NotUsed,
                    tool_available: false,
                    fixes_available: false,
                },
                success: false,
                error_message: Some("oxlint not found".to_string()),
            });
        }

        let tool_path = tool_path.unwrap();
        let mut adapter = self.clone();
        let version = adapter.get_version(&tool_path);
        let config_files = adapter.discover_config_files(project_root, config);

        let oxlint_output = self.run_oxlint(&tool_path, files, project_root, config)?;

        let diagnostics = normalize_oxlint_diagnostics(&oxlint_output, files, language);

        let fixes_available = oxlint_output
            .messages
            .iter()
            .any(|msg| msg.fix.is_some() || msg.suggestions.is_some());

        Ok(AdapterResult {
            diagnostics,
            metadata: AdapterMetadata {
                tool_name: "oxlint".to_string(),
                version,
                config_files,
                working_directory: project_root.to_path_buf(),
                exit_status: oxlint_output.exit_code.map(|c| c as i32),
                elapsed_time_ms: start.elapsed().as_millis() as u64,
                cache_status: AdapterCacheStatus::NotUsed,
                tool_available: true,
                fixes_available,
            },
            success: true,
            error_message: None,
        })
    }
}

/// Tries to find `oxlint` on PATH.
fn which_oxlint() -> Option<String> {
    if let Ok(path) = std::env::var("PATH") {
        let path_sep = if cfg!(windows) { ";" } else { ":" };
        for dir in path.split(path_sep) {
            let candidate = PathBuf::from(dir).join("oxlint");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
            // Windows executable
            #[cfg(windows)]
            {
                let candidate_exe = PathBuf::from(dir).join("oxlint.exe");
                if candidate_exe.exists() {
                    return Some(candidate_exe.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

/// Oxlint JSON output structure.
#[derive(Debug, Clone, Deserialize)]
struct OxlintJsonOutput {
    #[serde(default)]
    messages: Vec<OxlintMessage>,
    #[serde(default)]
    exit_code: Option<u8>,
}

/// A single diagnostic message from Oxlint.
#[derive(Debug, Clone, Deserialize)]
struct OxlintMessage {
    message: String,
    #[serde(default)]
    severity: Option<u8>,
    #[serde(default)]
    rule_id: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
    #[serde(default)]
    end_line: Option<u32>,
    #[serde(default)]
    end_column: Option<u32>,
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    fix: Option<OxlintFix>,
    #[serde(default)]
    suggestions: Option<Vec<OxlintSuggestion>>,
}

/// Fix information from Oxlint.
#[derive(Debug, Clone, Deserialize)]
struct OxlintFix {
    #[serde(default)]
    _range: Option<Vec<u32>>,
    #[serde(default)]
    _text: Option<String>,
}

/// Suggestion information from Oxlint.
#[derive(Debug, Clone, Deserialize)]
struct OxlintSuggestion {
    #[serde(default)]
    _desc: Option<String>,
    #[serde(default)]
    _fix: Option<OxlintFix>,
}

/// Normalizes Oxlint JSON output into Tree Hugger diagnostics.
fn normalize_oxlint_diagnostics(
    output: &OxlintJsonOutput,
    input_files: &[PathBuf],
    _language: ProgrammingLanguage,
) -> Vec<crate::shared::Diagnostic> {
    let mut diagnostics = Vec::new();

    for message in &output.messages {
        let severity = match message.severity {
            Some(2) => DiagnosticSeverity::Error,
            Some(1) => DiagnosticSeverity::Warning,
            _ => DiagnosticSeverity::Info,
        };

        let rule_id = message
            .rule_id
            .clone()
            .unwrap_or_else(|| "oxlint".to_string());

        let category = message
            .category
            .as_deref()
            .map(map_category)
            .unwrap_or(DiagnosticCategory::Suspicious);

        let confidence = DiagnosticConfidence::High;

        // Determine which file this diagnostic belongs to
        let file_path = message
            .file_path
            .as_ref()
            .and_then(|fp| input_files.iter().find(|f| f.ends_with(fp)).cloned())
            .or_else(|| input_files.first().cloned());

        if file_path.is_none() {
            continue;
        }

        let range = CodeRange {
            start_line: message.line.unwrap_or(1) as usize,
            start_column: message.column.unwrap_or(1) as usize,
            end_line: message.end_line.unwrap_or(message.line.unwrap_or(1)) as usize,
            end_column: message.end_column.unwrap_or(message.column.unwrap_or(1)) as usize,
            start_byte: 0,
            end_byte: 0,
        };

        diagnostics.push(external_diagnostic(
            message.message.clone(),
            range,
            severity,
            rule_id,
            category,
            confidence,
            "oxlint".to_string(),
        ));
    }

    diagnostics
}
