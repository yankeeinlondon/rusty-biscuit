use sniff_lib::programs::{ProgramDetector, ProgramsInfo};

#[cfg(target_os = "macos")]
use sniff_lib::programs::TerminalApp;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use sniff_lib::programs::Editor;

#[cfg(target_os = "windows")]
use sniff_lib::programs::TtsClient;

#[test]
fn test_installable_false_for_os_specific_programs() {
    let programs = ProgramsInfo::detect();

    #[cfg(target_os = "macos")]
    assert!(
        !programs
            .terminal_apps
            .installable(TerminalApp::WindowsTerminal),
        "Windows Terminal should not be installable on macOS"
    );

    #[cfg(target_os = "linux")]
    assert!(
        !programs.editors.installable(Editor::TextMate),
        "TextMate should not be installable on Linux"
    );

    #[cfg(target_os = "windows")]
    assert!(
        !programs.editors.installable(Editor::TextMate),
        "TextMate should not be installable on Windows"
    );
}

#[cfg(target_os = "windows")]
#[test]
fn test_installable_false_for_builtin_tts() {
    let programs = ProgramsInfo::detect();
    assert!(
        !programs.tts_clients.installable(TtsClient::WindowsSapi),
        "Windows SAPI has no install methods"
    );
}
