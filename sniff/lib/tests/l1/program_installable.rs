use sniff::programs::{CategoryDetector, ProgramDetector};

#[cfg(target_os = "macos")]
use sniff::programs::TerminalApp;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use sniff::programs::Editor;

#[cfg(target_os = "windows")]
use sniff::programs::TtsClient;

#[test]
fn test_installable_false_for_os_specific_programs() {
    #[cfg(target_os = "macos")]
    assert!(
        !CategoryDetector::<TerminalApp>::default().installable(TerminalApp::WindowsTerminal),
        "Windows Terminal should not be installable on macOS"
    );

    #[cfg(target_os = "linux")]
    assert!(
        !CategoryDetector::<Editor>::default().installable(Editor::TextMate),
        "TextMate should not be installable on Linux"
    );

    #[cfg(target_os = "windows")]
    assert!(
        !CategoryDetector::<Editor>::default().installable(Editor::TextMate),
        "TextMate should not be installable on Windows"
    );
}

#[cfg(target_os = "windows")]
#[test]
fn test_installable_false_for_builtin_tts() {
    assert!(
        !CategoryDetector::<TtsClient>::default().installable(TtsClient::WindowsSapi),
        "Windows SAPI has no install methods"
    );
}
