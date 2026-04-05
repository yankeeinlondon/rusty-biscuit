//! Program inventory with the `Program` tagged union.
//!
//! This module provides the `Program` tagged union spanning all program categories.
//! Each variant wraps a category-specific enum that implements `ProgramMetadata`,
//! so all metadata (display name, description, website, installation methods, etc.)
//! is accessible via `program.info()`.

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::programs::enums::{
    AiCli, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager, TerminalApp, TtsClient,
    Utility,
};
use crate::programs::schema::{ProgramInfo, ProgramMetadata};

/// Unified enum spanning all program categories.
///
/// Each variant wraps a category-specific enum, making the relationship
/// between categories and the unified type structural rather than manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Program {
    Editor(Editor),
    Utility(Utility),
    LanguagePackageManager(LanguagePackageManager),
    OsPackageManager(OsPackageManager),
    TtsClient(TtsClient),
    TerminalApp(TerminalApp),
    HeadlessAudio(HeadlessAudio),
    AiCli(AiCli),
}

// ============================================================================
// ProgramMetadata implementation
// ============================================================================

impl ProgramMetadata for Program {
    fn info(&self) -> &'static ProgramInfo {
        match self {
            Program::Editor(e) => e.info(),
            Program::Utility(u) => u.info(),
            Program::LanguagePackageManager(l) => l.info(),
            Program::OsPackageManager(o) => o.info(),
            Program::TtsClient(t) => t.info(),
            Program::TerminalApp(t) => t.info(),
            Program::HeadlessAudio(h) => h.info(),
            Program::AiCli(a) => a.info(),
        }
    }
}

// ============================================================================
// From conversions
// ============================================================================

impl From<Editor> for Program {
    fn from(e: Editor) -> Self {
        Program::Editor(e)
    }
}

impl From<Utility> for Program {
    fn from(u: Utility) -> Self {
        Program::Utility(u)
    }
}

impl From<LanguagePackageManager> for Program {
    fn from(l: LanguagePackageManager) -> Self {
        Program::LanguagePackageManager(l)
    }
}

impl From<OsPackageManager> for Program {
    fn from(o: OsPackageManager) -> Self {
        Program::OsPackageManager(o)
    }
}

impl From<TtsClient> for Program {
    fn from(t: TtsClient) -> Self {
        Program::TtsClient(t)
    }
}

impl From<TerminalApp> for Program {
    fn from(t: TerminalApp) -> Self {
        Program::TerminalApp(t)
    }
}

impl From<HeadlessAudio> for Program {
    fn from(h: HeadlessAudio) -> Self {
        Program::HeadlessAudio(h)
    }
}

impl From<AiCli> for Program {
    fn from(a: AiCli) -> Self {
        Program::AiCli(a)
    }
}

// ============================================================================
// Display, Serialize, Deserialize
// ============================================================================

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.binary_name())
    }
}

impl Serialize for Program {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.binary_name())
    }
}

impl<'de> Deserialize<'de> for Program {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Program::from_binary_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown program: {}", name)))
    }
}

// ============================================================================
// Helper methods
// ============================================================================

impl Program {
    /// Look up a Program by binary name.
    pub fn from_binary_name(name: &str) -> Option<Self> {
        Editor::iter()
            .find(|e| e.binary_name() == name)
            .map(Program::Editor)
            .or_else(|| {
                Utility::iter()
                    .find(|u| u.binary_name() == name)
                    .map(Program::Utility)
            })
            .or_else(|| {
                LanguagePackageManager::iter()
                    .find(|l| l.binary_name() == name)
                    .map(Program::LanguagePackageManager)
            })
            .or_else(|| {
                OsPackageManager::iter()
                    .find(|o| o.binary_name() == name)
                    .map(Program::OsPackageManager)
            })
            .or_else(|| {
                TtsClient::iter()
                    .find(|t| t.binary_name() == name)
                    .map(Program::TtsClient)
            })
            .or_else(|| {
                TerminalApp::iter()
                    .find(|t| t.binary_name() == name)
                    .map(Program::TerminalApp)
            })
            .or_else(|| {
                HeadlessAudio::iter()
                    .find(|h| h.binary_name() == name)
                    .map(Program::HeadlessAudio)
            })
            .or_else(|| {
                AiCli::iter()
                    .find(|a| a.binary_name() == name)
                    .map(Program::AiCli)
            })
    }

    /// Iterate over all programs across all categories.
    pub fn iter() -> impl Iterator<Item = Program> {
        Editor::iter()
            .map(Program::from)
            .chain(Utility::iter().map(Program::from))
            .chain(LanguagePackageManager::iter().map(Program::from))
            .chain(OsPackageManager::iter().map(Program::from))
            .chain(TtsClient::iter().map(Program::from))
            .chain(TerminalApp::iter().map(Program::from))
            .chain(HeadlessAudio::iter().map(Program::from))
            .chain(AiCli::iter().map(Program::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::enums::*;
    use crate::programs::schema::ProgramMetadata;
    use strum::EnumCount;

    #[test]
    fn test_program_from_all_editors() {
        for editor in Editor::iter() {
            let p = Program::from(editor);
            assert_eq!(p.display_name(), editor.display_name());
        }
    }

    #[test]
    fn test_program_vim_has_metadata() {
        let p = Program::from(Editor::Vim);
        assert_eq!(p.display_name(), "Vim");
        assert!(!p.info().website.is_empty());
    }

    #[test]
    fn test_program_iter_count() {
        let count = Program::iter().count();
        let expected = Editor::COUNT
            + Utility::COUNT
            + LanguagePackageManager::COUNT
            + OsPackageManager::COUNT
            + TtsClient::COUNT
            + TerminalApp::COUNT
            + HeadlessAudio::COUNT
            + AiCli::COUNT;
        assert_eq!(count, expected);
    }

    #[test]
    fn test_all_programs_have_valid_metadata() {
        for program in Program::iter() {
            let info = program.info();
            assert!(
                !info.display_name.is_empty(),
                "{:?} has empty display_name",
                program
            );
            assert!(
                !info.description.is_empty(),
                "{:?} has empty description",
                program
            );
            assert!(!info.website.is_empty(), "{:?} has empty website", program);
        }
    }

    #[test]
    fn test_program_serde_roundtrip() {
        for program in Program::iter() {
            let json = serde_json::to_string(&program).unwrap();
            let decoded: Program = serde_json::from_str(&json).unwrap();
            assert_eq!(program, decoded, "Roundtrip failed for {:?}", program);
        }
    }

    #[test]
    fn test_program_copy_derive() {
        let p = Program::from(Editor::Vim);
        let p2 = p;
        assert_eq!(p, p2);
    }

    #[test]
    fn test_program_display_uses_binary_name() {
        let p = Program::from(Editor::Vim);
        assert_eq!(p.to_string(), "vim");
    }

    #[test]
    fn test_program_from_binary_name() {
        assert_eq!(
            Program::from_binary_name("vim"),
            Some(Program::Editor(Editor::Vim))
        );
        assert_eq!(
            Program::from_binary_name("rg"),
            Some(Program::Utility(Utility::Ripgrep))
        );
        assert_eq!(Program::from_binary_name("nonexistent"), None);
    }
}
