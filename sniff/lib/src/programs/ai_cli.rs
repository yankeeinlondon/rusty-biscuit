use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SniffInstallationError;
use crate::os::detect_os_type;
use crate::programs::enums::AiCli;
use crate::programs::find_program::find_programs_with_source_parallel;
use crate::programs::installer::{
    execute_install, execute_versioned_install, method_available, select_best_method,
    InstallOptions,
};
use crate::programs::schema::{ProgramEntry, ProgramError, ProgramMetadata};
use crate::programs::types::{ExecutableSource, ProgramDetector};
use crate::programs::{
    InstalledLanguagePackageManagers, InstalledOsPackageManagers, Program, PROGRAM_LOOKUP,
};

fn ai_cli_details(client: AiCli) -> Option<&'static crate::programs::ProgramDetails> {
    let program = match client {
        AiCli::Claude => Program::Claude,
        AiCli::Opencode => Program::Opencode,
        AiCli::Roo => Program::Roo,
        AiCli::GeminiCli => Program::GeminiCli,
        AiCli::Aider => Program::Aider,
        AiCli::Codex => Program::Codex,
        AiCli::Goose => Program::Goose,
    };

    PROGRAM_LOOKUP.get(&program)
}

/// AI-powered CLI coding tools found on the system.
///
/// Stores path and discovery source for each installed AI CLI tool.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledAiClients {
    claude: Option<(PathBuf, ExecutableSource)>,
    opencode: Option<(PathBuf, ExecutableSource)>,
    roo: Option<(PathBuf, ExecutableSource)>,
    gemini_cli: Option<(PathBuf, ExecutableSource)>,
    aider: Option<(PathBuf, ExecutableSource)>,
    codex: Option<(PathBuf, ExecutableSource)>,
    goose: Option<(PathBuf, ExecutableSource)>,
}

impl InstalledAiClients {
    /// Detect which AI CLI tools are installed on the system.
    pub fn new() -> Self {
        let programs = ["claude", "opencode", "roo", "gemini", "aider", "codex", "goose"];

        let results = find_programs_with_source_parallel(&programs);

        let get = |name: &str| results.get(name).and_then(|r| r.clone());

        Self {
            claude: get("claude"),
            opencode: get("opencode"),
            roo: get("roo"),
            gemini_cli: get("gemini"),
            aider: get("aider"),
            codex: get("codex"),
            goose: get("goose"),
        }
    }

    /// Re-check program availability and update all fields.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    // Boolean accessors for backward compatibility with direct field access patterns.

    /// Returns true if Claude Code is installed.
    pub fn claude(&self) -> bool {
        self.claude.is_some()
    }

    /// Returns true if OpenCode is installed.
    pub fn opencode(&self) -> bool {
        self.opencode.is_some()
    }

    /// Returns true if Roo Code is installed.
    pub fn roo(&self) -> bool {
        self.roo.is_some()
    }

    /// Returns true if Gemini CLI is installed.
    pub fn gemini_cli(&self) -> bool {
        self.gemini_cli.is_some()
    }

    /// Returns true if Aider is installed.
    pub fn aider(&self) -> bool {
        self.aider.is_some()
    }

    /// Returns true if Codex CLI is installed.
    pub fn codex(&self) -> bool {
        self.codex.is_some()
    }

    /// Returns true if Goose is installed.
    pub fn goose(&self) -> bool {
        self.goose.is_some()
    }

    /// Returns the path to the specified AI CLI tool's binary if installed.
    pub fn path(&self, client: AiCli) -> Option<PathBuf> {
        self.path_with_source(client).map(|(p, _)| p)
    }

    /// Returns the path and source of the specified AI CLI tool if installed.
    pub fn path_with_source(&self, client: AiCli) -> Option<(PathBuf, ExecutableSource)> {
        match client {
            AiCli::Claude => self.claude.clone(),
            AiCli::Opencode => self.opencode.clone(),
            AiCli::Roo => self.roo.clone(),
            AiCli::GeminiCli => self.gemini_cli.clone(),
            AiCli::Aider => self.aider.clone(),
            AiCli::Codex => self.codex.clone(),
            AiCli::Goose => self.goose.clone(),
        }
    }

    /// Returns the version of the specified AI CLI tool if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The AI CLI tool is not installed
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    pub fn version(&self, client: AiCli) -> Result<String, ProgramError> {
        if !self.is_installed(client) {
            return Err(ProgramError::NotFound(client.binary_name().to_string()));
        }
        client.version()
    }

    /// Returns the official website URL for the specified AI CLI tool.
    pub fn website(&self, client: AiCli) -> &'static str {
        client.website()
    }

    /// Returns a one-line description of the specified AI CLI tool.
    pub fn description(&self, client: AiCli) -> &'static str {
        client.description()
    }

    /// Checks if the specified AI CLI tool is installed.
    pub fn is_installed(&self, client: AiCli) -> bool {
        match client {
            AiCli::Claude => self.claude.is_some(),
            AiCli::Opencode => self.opencode.is_some(),
            AiCli::Roo => self.roo.is_some(),
            AiCli::GeminiCli => self.gemini_cli.is_some(),
            AiCli::Aider => self.aider.is_some(),
            AiCli::Codex => self.codex.is_some(),
            AiCli::Goose => self.goose.is_some(),
        }
    }

    /// Returns a list of all installed AI CLI tools.
    pub fn installed(&self) -> Vec<AiCli> {
        use strum::IntoEnumIterator;
        AiCli::iter().filter(|c| self.is_installed(*c)).collect()
    }

    /// Mark a client as installed (for testing purposes).
    ///
    /// Creates a fake path entry for the specified client.
    /// This is useful for unit tests that need to mock installed clients.
    pub fn with_client(mut self, client: AiCli) -> Self {
        let cmd_name = match client {
            AiCli::Claude => "claude",
            AiCli::Opencode => "opencode",
            AiCli::Roo => "roo",
            AiCli::GeminiCli => "gemini",
            AiCli::Aider => "aider",
            AiCli::Codex => "codex",
            AiCli::Goose => "goose",
        };
        let fake_path = PathBuf::from(format!("/usr/bin/{}", cmd_name));
        let entry = Some((fake_path, ExecutableSource::Path));
        match client {
            AiCli::Claude => self.claude = entry,
            AiCli::Opencode => self.opencode = entry,
            AiCli::Roo => self.roo = entry,
            AiCli::GeminiCli => self.gemini_cli = entry,
            AiCli::Aider => self.aider = entry,
            AiCli::Codex => self.codex = entry,
            AiCli::Goose => self.goose = entry,
        }
        self
    }
}

impl Serialize for InstalledAiClients {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use strum::IntoEnumIterator;

        let entry = |client: AiCli| -> ProgramEntry {
            let info = client.info();
            match self.path_with_source(client) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            }
        };

        let mut state = serializer.serialize_struct("InstalledAiClients", 7)?;
        for client in AiCli::iter() {
            let field_name = match client {
                AiCli::Claude => "claude",
                AiCli::Opencode => "opencode",
                AiCli::Roo => "roo",
                AiCli::GeminiCli => "gemini_cli",
                AiCli::Aider => "aider",
                AiCli::Codex => "codex",
                AiCli::Goose => "goose",
            };
            state.serialize_field(field_name, &entry(client))?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for InstalledAiClients {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BoolAiClients {
            #[serde(default)]
            claude: bool,
            #[serde(default)]
            opencode: bool,
            #[serde(default)]
            roo: bool,
            #[serde(default)]
            gemini_cli: bool,
            #[serde(default)]
            aider: bool,
            #[serde(default)]
            codex: bool,
            #[serde(default)]
            goose: bool,
        }

        let b = BoolAiClients::deserialize(deserializer)?;

        let to_opt = |installed: bool| {
            if installed {
                Some((PathBuf::new(), ExecutableSource::Path))
            } else {
                None
            }
        };

        Ok(InstalledAiClients {
            claude: to_opt(b.claude),
            opencode: to_opt(b.opencode),
            roo: to_opt(b.roo),
            gemini_cli: to_opt(b.gemini_cli),
            aider: to_opt(b.aider),
            codex: to_opt(b.codex),
            goose: to_opt(b.goose),
        })
    }
}

impl ProgramDetector for InstalledAiClients {
    type Program = AiCli;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: Self::Program) -> bool {
        self.is_installed(program)
    }

    fn path(&self, program: Self::Program) -> Option<PathBuf> {
        InstalledAiClients::path(self, program)
    }

    fn path_with_source(&self, program: Self::Program) -> Option<(PathBuf, ExecutableSource)> {
        InstalledAiClients::path_with_source(self, program)
    }

    fn version(&self, program: Self::Program) -> Result<String, ProgramError> {
        InstalledAiClients::version(self, program)
    }

    fn website(&self, program: Self::Program) -> &'static str {
        InstalledAiClients::website(self, program)
    }

    fn description(&self, program: Self::Program) -> &'static str {
        InstalledAiClients::description(self, program)
    }

    fn installed(&self) -> Vec<Self::Program> {
        InstalledAiClients::installed(self)
    }

    fn installable(&self, program: Self::Program) -> bool {
        let Some(details) = ai_cli_details(program) else {
            return false;
        };

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return false;
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();

        details
            .installation_methods
            .iter()
            .any(|method| method_available(method, &os_pkg_mgrs, &lang_pkg_mgrs))
    }

    fn install(&self, program: Self::Program) -> Result<(), SniffInstallationError> {
        let details = ai_cli_details(program).ok_or_else(|| {
            SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            }
        })?;

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: os_type.to_string(),
            });
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();
        let method = select_best_method(details.installation_methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .ok_or_else(|| SniffInstallationError::MissingPackageManager {
                pkg: program.display_name().to_string(),
                manager: "package manager".to_string(),
            })?;

        let _result = execute_install(method, &InstallOptions::default())?;
        Ok(())
    }

    fn install_version(
        &self,
        program: Self::Program,
        version: &str,
    ) -> Result<(), SniffInstallationError> {
        let details = ai_cli_details(program).ok_or_else(|| {
            SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            }
        })?;

        let os_type = detect_os_type();
        if !details.os_availability.contains(&os_type) {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: os_type.to_string(),
            });
        }

        let os_pkg_mgrs = InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = InstalledLanguagePackageManagers::new();
        let method = select_best_method(details.installation_methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .ok_or_else(|| SniffInstallationError::MissingPackageManager {
                pkg: program.display_name().to_string(),
                manager: "package manager".to_string(),
            })?;

        let _result = execute_versioned_install(method, version, &InstallOptions::default())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_source_returns_none_when_not_installed() {
        let clients = InstalledAiClients::default();
        assert!(clients.path_with_source(AiCli::Claude).is_none());
    }

    #[test]
    fn test_is_installed_returns_false_for_default() {
        let clients = InstalledAiClients::default();
        assert!(!clients.is_installed(AiCli::Claude));
        assert!(!clients.is_installed(AiCli::Aider));
    }

    #[test]
    fn test_serialize_produces_program_entries() {
        let clients = InstalledAiClients::default();
        let json = serde_json::to_string(&clients).unwrap();
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"claude\":{"));
        assert!(json.contains("\"name\":\"Claude Code\""));
    }

    #[test]
    fn test_deserialize_from_boolean_fields() {
        let json = r#"{"claude": true, "aider": false}"#;
        let clients: InstalledAiClients = serde_json::from_str(json).unwrap();
        assert!(clients.is_installed(AiCli::Claude));
        assert!(!clients.is_installed(AiCli::Aider));
    }

    #[test]
    fn test_serialize_to_json() {
        let original = InstalledAiClients::default();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("claude").is_some());
    }

    #[test]
    fn test_with_client_marks_as_installed() {
        let clients = InstalledAiClients::default().with_client(AiCli::Claude);
        assert!(clients.is_installed(AiCli::Claude));
        assert!(!clients.is_installed(AiCli::Aider));
    }

    #[test]
    fn test_installed_returns_empty_for_default() {
        let clients = InstalledAiClients::default();
        assert!(clients.installed().is_empty());
    }

    #[test]
    fn test_installed_returns_marked_clients() {
        let clients = InstalledAiClients::default()
            .with_client(AiCli::Claude)
            .with_client(AiCli::Aider);
        let installed = clients.installed();
        assert_eq!(installed.len(), 2);
        assert!(installed.contains(&AiCli::Claude));
        assert!(installed.contains(&AiCli::Aider));
    }

    #[test]
    fn test_boolean_accessors() {
        let clients = InstalledAiClients::default().with_client(AiCli::Claude);
        assert!(clients.claude());
        assert!(!clients.opencode());
        assert!(!clients.roo());
        assert!(!clients.gemini_cli());
        assert!(!clients.aider());
        assert!(!clients.codex());
        assert!(!clients.goose());
    }

    #[test]
    fn test_version_returns_not_found_for_uninstalled() {
        let clients = InstalledAiClients::default();
        let result = clients.version(AiCli::Claude);
        assert!(result.is_err());
        if let Err(ProgramError::NotFound(name)) = result {
            assert_eq!(name, "claude");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_website_returns_static_str() {
        let clients = InstalledAiClients::default();
        let website = clients.website(AiCli::Claude);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn test_description_returns_static_str() {
        let clients = InstalledAiClients::default();
        let desc = clients.description(AiCli::Claude);
        assert!(!desc.is_empty());
    }
}
