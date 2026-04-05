//! Serialization roundtrip tests for ProgramsInfo and CategoryDetector.

use sniff::programs::{
    AiCli, CategoryDetector, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager,
    ProgramsInfo, TerminalApp, TtsClient, Utility,
};

#[test]
fn programs_info_serialization_roundtrip() {
    let info = ProgramsInfo::default();
    let json = serde_json::to_string(&info).unwrap();

    // Verify deserialization succeeds and produces valid JSON structure
    let decoded: ProgramsInfo = serde_json::from_str(&json).unwrap();

    // Re-serialize and verify the JSON is stable
    let json2 = serde_json::to_string(&decoded).unwrap();
    assert_eq!(json, json2, "Serialization is not stable");
}

#[test]
fn all_categories_serialization_roundtrip() {
    macro_rules! roundtrip {
        ($t:ty) => {{
            let detector = CategoryDetector::<$t>::default();
            let json = serde_json::to_string(&detector).unwrap();
            let decoded: CategoryDetector<$t> = serde_json::from_str(&json).unwrap();
            assert_eq!(
                detector, decoded,
                "Roundtrip failed for {}",
                std::any::type_name::<$t>()
            );
        }};
    }
    roundtrip!(Editor);
    roundtrip!(Utility);
    roundtrip!(LanguagePackageManager);
    roundtrip!(OsPackageManager);
    roundtrip!(TtsClient);
    roundtrip!(TerminalApp);
    roundtrip!(HeadlessAudio);
    roundtrip!(AiCli);
}

#[test]
fn deserialize_from_legacy_boolean_format() {
    let json = r#"{"vim": true, "neovim": true, "emacs": false}"#;
    let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
    assert!(detector.is_installed(Editor::Vim));
    assert!(detector.is_installed(Editor::Neovim));
    assert!(!detector.is_installed(Editor::Emacs));
}

#[test]
fn deserialize_from_rich_entry_format() {
    let json = r#"{"vim": {"installed": true, "name": "Vim", "description": "test", "website": "test"}}"#;
    let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
    assert!(detector.is_installed(Editor::Vim));
}
