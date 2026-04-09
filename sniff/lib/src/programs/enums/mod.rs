//! Program enums with strum derives for metadata and iteration.
//!
//! This module defines enums for each program category with full metadata
//! lookup support via the `ProgramMetadata` trait. Each category also carries
//! installation metadata (OS availability, repository URL, installation
//! methods) previously stored in `inventory.rs`.

pub mod categories;
pub mod metadata;

pub use categories::{
    AiCli, CategoryEnum, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager,
    TerminalApp, TtsClient, Utility,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::schema::ProgramMetadata;
    use strum::{EnumCount, IntoEnumIterator};

    #[test]
    fn test_editor_count_matches_info() {
        assert_eq!(Editor::COUNT, metadata::EDITOR_INFO.len());
    }

    #[test]
    fn test_ai_cli_count_matches_info() {
        assert_eq!(AiCli::COUNT, metadata::AI_CLI_INFO.len());
    }

    #[test]
    fn test_utility_count_matches_info() {
        assert_eq!(Utility::COUNT, metadata::UTILITY_INFO.len());
    }

    #[test]
    fn test_lang_pkg_mgr_count_matches_info() {
        assert_eq!(
            LanguagePackageManager::COUNT,
            metadata::LANG_PKG_MGR_INFO.len()
        );
    }

    #[test]
    fn test_os_pkg_mgr_count_matches_info() {
        assert_eq!(OsPackageManager::COUNT, metadata::OS_PKG_MGR_INFO.len());
    }

    #[test]
    fn test_tts_client_count_matches_info() {
        assert_eq!(TtsClient::COUNT, metadata::TTS_CLIENT_INFO.len());
    }

    #[test]
    fn test_terminal_app_count_matches_info() {
        assert_eq!(TerminalApp::COUNT, metadata::TERMINAL_APP_INFO.len());
    }

    #[test]
    fn test_headless_audio_count_matches_info() {
        assert_eq!(HeadlessAudio::COUNT, metadata::HEADLESS_AUDIO_INFO.len());
    }

    #[test]
    fn test_editor_metadata_access() {
        let vim = Editor::Vim;
        assert_eq!(vim.binary_name(), "vim");
        assert_eq!(vim.display_name(), "Vim");
        assert!(vim.website().starts_with("https://"));
    }

    #[test]
    fn test_enum_iteration() {
        let count = Editor::iter().count();
        assert_eq!(count, Editor::COUNT);
    }

    #[test]
    fn test_category_enum_trait_on_editor() {
        assert_eq!(Editor::category_name(), "editors");
        assert_eq!(Editor::Vi.variant_index(), 0);
        assert_eq!(Editor::Kate.variant_index(), Editor::COUNT - 1);

        let mut seen = std::collections::HashSet::new();
        for editor in Editor::iter() {
            let idx = editor.variant_index();
            assert!(
                idx < Editor::COUNT,
                "{:?} index {} >= COUNT {}",
                editor,
                idx,
                Editor::COUNT
            );
            assert!(seen.insert(idx), "{:?} has duplicate index {}", editor, idx);
        }
    }

    #[test]
    fn test_all_category_enums_cover_all_programs() {
        use crate::programs::inventory::Program;
        use strum::EnumCount;

        let category_total = Editor::COUNT
            + Utility::COUNT
            + LanguagePackageManager::COUNT
            + OsPackageManager::COUNT
            + TtsClient::COUNT
            + TerminalApp::COUNT
            + HeadlessAudio::COUNT
            + AiCli::COUNT;

        let program_total = Program::iter().count();
        assert_eq!(
            category_total, program_total,
            "Category enum total ({}) != Program::iter() count ({})",
            category_total, program_total
        );
    }

    #[test]
    fn test_program_mapping_is_bijective() {
        use crate::programs::inventory::Program;
        use std::collections::HashSet;

        let mut seen = HashSet::new();

        macro_rules! check_category {
            ($enum_type:ty) => {
                for variant in <$enum_type>::iter() {
                    let program = Program::from(variant);
                    let key = format!("{:?}", program);
                    assert!(
                        seen.insert(key.clone()),
                        "Duplicate Program mapping from {:?} -> {}",
                        variant,
                        key
                    );
                }
            };
        }

        check_category!(Editor);
        check_category!(Utility);
        check_category!(LanguagePackageManager);
        check_category!(OsPackageManager);
        check_category!(TtsClient);
        check_category!(TerminalApp);
        check_category!(HeadlessAudio);
        check_category!(AiCli);
    }

    #[test]
    fn test_category_variant_indices_are_contiguous() {
        macro_rules! check_indices {
            ($enum_type:ty) => {{
                let mut indices: Vec<usize> =
                    <$enum_type>::iter().map(|v| v.variant_index()).collect();
                indices.sort();
                let expected: Vec<usize> = (0..<$enum_type>::COUNT).collect();
                assert_eq!(
                    indices,
                    expected,
                    "{} variant indices are not contiguous",
                    std::any::type_name::<$enum_type>()
                );
            }};
        }
        check_indices!(Editor);
        check_indices!(Utility);
        check_indices!(LanguagePackageManager);
        check_indices!(OsPackageManager);
        check_indices!(TtsClient);
        check_indices!(TerminalApp);
        check_indices!(HeadlessAudio);
        check_indices!(AiCli);
    }
}
