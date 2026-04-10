//! Install handler for program categories.
//!
//! Supports both direct install (by name) and interactive install (MultiSelect picker).

use std::error::Error;
use std::fmt;

use inquire::MultiSelect;
use sniff::programs::{ProgramDetector, ProgramMetadata};
use strum::IntoEnumIterator;

use crate::output::OutputFilter;

// ---------------------------------------------------------------------------
// Custom error type for install resolution
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ResolveError(String);

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ResolveError {}

// ---------------------------------------------------------------------------
// Name resolution — macro-generated per category
// ---------------------------------------------------------------------------

macro_rules! resolve_program {
    ($fn_name:ident, $enum_type:ty, $category:expr) => {
        fn $fn_name(name: &str) -> Result<$enum_type, ResolveError> {
            // 1. Try strum EnumString parse (snake_case)
            if let Ok(p) = name.parse::<$enum_type>() {
                return Ok(p);
            }

            let lower = name.to_lowercase();

            // 2. Try binary_name match (case-insensitive)
            for p in <$enum_type>::iter() {
                if p.binary_name().to_lowercase() == lower {
                    return Ok(p);
                }
            }

            // 3. Try display_name match (case-insensitive)
            for p in <$enum_type>::iter() {
                if p.display_name().to_lowercase() == lower {
                    return Ok(p);
                }
            }

            // Build valid names list for error message
            let valid: Vec<String> = <$enum_type>::iter()
                .map(|p| {
                    let binary = p.binary_name();
                    let display = p.display_name();
                    if binary == display {
                        binary.to_string()
                    } else {
                        format!("{} (binary: {})", display, binary)
                    }
                })
                .collect();

            Err(ResolveError(format!(
                "Unknown {} '{}'. Valid names:\n  {}",
                $category,
                name,
                valid.join("\n  ")
            )))
        }
    };
}

resolve_program!(resolve_editor, sniff::programs::Editor, "editor");
resolve_program!(resolve_utility, sniff::programs::Utility, "utility");
resolve_program!(
    resolve_lang_pkg_mgr,
    sniff::programs::LanguagePackageManager,
    "language package manager"
);
resolve_program!(
    resolve_os_pkg_mgr,
    sniff::programs::OsPackageManager,
    "OS package manager"
);
resolve_program!(resolve_tts_client, sniff::programs::TtsClient, "TTS client");
resolve_program!(
    resolve_terminal_app,
    sniff::programs::TerminalApp,
    "terminal app"
);
resolve_program!(
    resolve_audio,
    sniff::programs::HeadlessAudio,
    "audio player"
);
resolve_program!(resolve_agent, sniff::programs::AiCli, "AI agent");

// ---------------------------------------------------------------------------
// Cross-category resolved program type and resolver
// ---------------------------------------------------------------------------

/// A program identified by the name the user typed.
#[derive(Debug, Clone, Copy)]
pub enum ResolvedProgram {
    Editor(sniff::programs::Editor),
    Utility(sniff::programs::Utility),
    LanguagePackageManager(sniff::programs::LanguagePackageManager),
    OsPackageManager(sniff::programs::OsPackageManager),
    TtsClient(sniff::programs::TtsClient),
    TerminalApp(sniff::programs::TerminalApp),
    HeadlessAudio(sniff::programs::HeadlessAudio),
    AiCli(sniff::programs::AiCli),
}

/// Resolve a program name scoped to a specific output filter category.
///
/// For `OutputFilter::Programs`, falls back to the cross-category
/// [`resolve_program`]. For all other program filters, restricts resolution to
/// the matching category so that error messages name the category correctly
/// (e.g., "Unknown editor" rather than "Unknown program").
pub fn resolve_program_in_category(
    name: &str,
    filter: crate::output::OutputFilter,
) -> Result<ResolvedProgram, ResolveError> {
    use crate::output::OutputFilter;
    match filter {
        OutputFilter::Editors => resolve_editor(name).map(ResolvedProgram::Editor),
        OutputFilter::Utilities => resolve_utility(name).map(ResolvedProgram::Utility),
        OutputFilter::LanguagePackageManagers => {
            resolve_lang_pkg_mgr(name).map(ResolvedProgram::LanguagePackageManager)
        }
        OutputFilter::OsPackageManagers => {
            resolve_os_pkg_mgr(name).map(ResolvedProgram::OsPackageManager)
        }
        OutputFilter::TtsClients => resolve_tts_client(name).map(ResolvedProgram::TtsClient),
        OutputFilter::TerminalApps => resolve_terminal_app(name).map(ResolvedProgram::TerminalApp),
        OutputFilter::HeadlessAudio => resolve_audio(name).map(ResolvedProgram::HeadlessAudio),
        OutputFilter::AiClients => resolve_agent(name).map(ResolvedProgram::AiCli),
        // For Programs or any other filter, fall back to cross-category search.
        _ => resolve_program(name),
    }
}

/// Resolve a free-form program name to a specific category enum variant.
///
/// Tries each category in a deterministic order. Returns an error listing the
/// categories searched if no match is found.
pub fn resolve_program(name: &str) -> Result<ResolvedProgram, ResolveError> {
    if let Ok(p) = resolve_editor(name) {
        return Ok(ResolvedProgram::Editor(p));
    }
    if let Ok(p) = resolve_utility(name) {
        return Ok(ResolvedProgram::Utility(p));
    }
    if let Ok(p) = resolve_lang_pkg_mgr(name) {
        return Ok(ResolvedProgram::LanguagePackageManager(p));
    }
    if let Ok(p) = resolve_os_pkg_mgr(name) {
        return Ok(ResolvedProgram::OsPackageManager(p));
    }
    if let Ok(p) = resolve_tts_client(name) {
        return Ok(ResolvedProgram::TtsClient(p));
    }
    if let Ok(p) = resolve_terminal_app(name) {
        return Ok(ResolvedProgram::TerminalApp(p));
    }
    if let Ok(p) = resolve_audio(name) {
        return Ok(ResolvedProgram::HeadlessAudio(p));
    }
    if let Ok(p) = resolve_agent(name) {
        return Ok(ResolvedProgram::AiCli(p));
    }
    Err(ResolveError(format!(
        "Unknown program '{}'. Searched categories: editors, utilities, language package managers, OS package managers, TTS clients, terminal apps, headless audio, AI agents",
        name
    )))
}

// ---------------------------------------------------------------------------
// Interactive install (MultiSelect picker)
// ---------------------------------------------------------------------------

macro_rules! interactive_install_category {
    ($fn_name:ident, $enum_type:ty, $detector_type:ty, $prompt:expr) => {
        fn $fn_name() -> Result<(), Box<dyn Error>> {
            let detector = <$detector_type>::new();
            let all: Vec<$enum_type> = <$enum_type>::iter().collect();

            let installed: Vec<_> = all.iter().filter(|p| detector.is_installed(**p)).collect();
            let not_installed: Vec<_> =
                all.iter().filter(|p| !detector.is_installed(**p)).collect();

            if !installed.is_empty() {
                let names: Vec<_> = installed.iter().map(|p| p.display_name()).collect();
                println!("\x1b[2mAlready installed: {}\x1b[0m\n", names.join(", "));
            }

            if not_installed.is_empty() {
                println!("All programs in this category are already installed!");
                return Ok(());
            }

            let options: Vec<String> = not_installed
                .iter()
                .map(|p| {
                    let binary = p.binary_name();
                    let display = p.display_name();
                    if binary == display {
                        binary.to_string()
                    } else {
                        format!("{} ({})", display, binary)
                    }
                })
                .collect();

            let selected = match MultiSelect::new($prompt, options.clone())
                .with_help_message("Space to toggle, Enter to confirm, Esc to skip")
                .prompt()
            {
                Ok(sel) => sel,
                Err(inquire::InquireError::OperationCanceled)
                | Err(inquire::InquireError::OperationInterrupted) => {
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            if selected.is_empty() {
                return Ok(());
            }

            for label in &selected {
                let idx = options.iter().position(|o| o == label).unwrap();
                let program = not_installed[idx];
                if !detector.installable(*program) {
                    println!(
                        "  Skipping {} (not installable on this OS)",
                        program.display_name()
                    );
                    continue;
                }
                println!("Installing {}...", program.display_name());
                match detector.install(*program) {
                    Ok(()) => println!("  Successfully installed."),
                    Err(e) => eprintln!("  Failed: {}", e),
                }
            }
            Ok(())
        }
    };
}

interactive_install_category!(
    interactive_install_editors,
    sniff::programs::Editor,
    sniff::programs::InstalledEditors,
    "Select editors to install:"
);
interactive_install_category!(
    interactive_install_utilities,
    sniff::programs::Utility,
    sniff::programs::InstalledUtilities,
    "Select utilities to install:"
);
interactive_install_category!(
    interactive_install_lang_pkg_mgrs,
    sniff::programs::LanguagePackageManager,
    sniff::programs::InstalledLanguagePackageManagers,
    "Select language package managers to install:"
);
interactive_install_category!(
    interactive_install_os_pkg_mgrs,
    sniff::programs::OsPackageManager,
    sniff::programs::InstalledOsPackageManagers,
    "Select OS package managers to install:"
);
interactive_install_category!(
    interactive_install_tts_clients,
    sniff::programs::TtsClient,
    sniff::programs::InstalledTtsClients,
    "Select TTS clients to install:"
);
interactive_install_category!(
    interactive_install_terminal_apps,
    sniff::programs::TerminalApp,
    sniff::programs::InstalledTerminalApps,
    "Select terminal apps to install:"
);
interactive_install_category!(
    interactive_install_audio,
    sniff::programs::HeadlessAudio,
    sniff::programs::InstalledHeadlessAudio,
    "Select audio players to install:"
);
interactive_install_category!(
    interactive_install_agents,
    sniff::programs::AiCli,
    sniff::programs::InstalledAiClients,
    "Select AI agents to install:"
);

/// Dispatch interactive install to the correct category.
pub fn interactive_install(filter: OutputFilter) -> Result<(), Box<dyn Error>> {
    match filter {
        OutputFilter::Editors => interactive_install_editors(),
        OutputFilter::Utilities => interactive_install_utilities(),
        OutputFilter::LanguagePackageManagers => interactive_install_lang_pkg_mgrs(),
        OutputFilter::OsPackageManagers => interactive_install_os_pkg_mgrs(),
        OutputFilter::TtsClients => interactive_install_tts_clients(),
        OutputFilter::TerminalApps => interactive_install_terminal_apps(),
        OutputFilter::HeadlessAudio => interactive_install_audio(),
        OutputFilter::AiClients => interactive_install_agents(),
        OutputFilter::Programs => interactive_install_all(),
        _ => unreachable!("interactive_install only called for program filters"),
    }
}

/// Walk through all categories sequentially with prompts.
fn interactive_install_all() -> Result<(), Box<dyn Error>> {
    println!("=== Editors ===");
    interactive_install_editors()?;
    println!("\n=== Utilities ===");
    interactive_install_utilities()?;
    println!("\n=== Language Package Managers ===");
    interactive_install_lang_pkg_mgrs()?;
    println!("\n=== OS Package Managers ===");
    interactive_install_os_pkg_mgrs()?;
    println!("\n=== TTS Clients ===");
    interactive_install_tts_clients()?;
    println!("\n=== Terminal Apps ===");
    interactive_install_terminal_apps()?;
    println!("\n=== Audio Players ===");
    interactive_install_audio()?;
    println!("\n=== AI Agents ===");
    interactive_install_agents()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_editor_by_snake_case() {
        let editor = resolve_editor("neovim").unwrap();
        assert_eq!(editor, sniff::programs::Editor::Neovim);
    }

    #[test]
    fn resolve_editor_by_binary_name() {
        let editor = resolve_editor("nvim").unwrap();
        assert_eq!(editor, sniff::programs::Editor::Neovim);
    }

    #[test]
    fn resolve_editor_by_display_name() {
        let editor = resolve_editor("Neovim").unwrap();
        assert_eq!(editor, sniff::programs::Editor::Neovim);
    }

    #[test]
    fn resolve_editor_case_insensitive_binary() {
        let editor = resolve_editor("NVIM").unwrap();
        assert_eq!(editor, sniff::programs::Editor::Neovim);
    }

    #[test]
    fn resolve_editor_invalid_name() {
        let err = resolve_editor("nonexistent").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown editor"));
        assert!(msg.contains("nonexistent"));
        assert!(msg.contains("Valid names:"));
        assert!(msg.contains("Vim"));
    }

    #[test]
    fn resolve_utility_by_binary_name() {
        let util = resolve_utility("rg").unwrap();
        assert_eq!(util, sniff::programs::Utility::Ripgrep);
    }

    #[test]
    fn resolve_agent_by_snake_case() {
        let agent = resolve_agent("claude").unwrap();
        assert_eq!(agent, sniff::programs::AiCli::Claude);
    }

    #[test]
    fn resolve_program_editor_by_binary() {
        let resolved = resolve_program("vim").unwrap();
        assert!(matches!(
            resolved,
            ResolvedProgram::Editor(sniff::programs::Editor::Vim)
        ));
    }

    #[test]
    fn resolve_program_utility_alternate() {
        let resolved = resolve_program("rg").unwrap();
        assert!(matches!(
            resolved,
            ResolvedProgram::Utility(sniff::programs::Utility::Ripgrep)
        ));
    }

    #[test]
    fn resolve_program_unknown_name_errors() {
        let err = resolve_program("definitely-not-a-real-program-xyz").unwrap_err();
        assert!(err.to_string().contains("Unknown program"));
    }
}
