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
// Direct install (single program by name)
// ---------------------------------------------------------------------------

macro_rules! direct_install_category {
    ($name:expr, $resolve_fn:ident, $detector_type:ty) => {{
        let program = $resolve_fn($name)?;
        let detector = <$detector_type>::new();
        if detector.is_installed(program) {
            println!("{} is already installed.", program.display_name());
            return Ok(());
        }
        if !detector.installable(program) {
            return Err(
                format!("{} is not installable on this OS.", program.display_name()).into(),
            );
        }
        println!("Installing {}...", program.display_name());
        detector.install(program)?;
        println!("Successfully installed {}.", program.display_name());
        Ok(())
    }};
}

pub fn direct_install(filter: OutputFilter, name: &str) -> Result<(), Box<dyn Error>> {
    match filter {
        OutputFilter::Editors => {
            direct_install_category!(name, resolve_editor, sniff::programs::InstalledEditors)
        }
        OutputFilter::Utilities => {
            direct_install_category!(name, resolve_utility, sniff::programs::InstalledUtilities)
        }
        OutputFilter::LanguagePackageManagers => {
            direct_install_category!(
                name,
                resolve_lang_pkg_mgr,
                sniff::programs::InstalledLanguagePackageManagers
            )
        }
        OutputFilter::OsPackageManagers => {
            direct_install_category!(
                name,
                resolve_os_pkg_mgr,
                sniff::programs::InstalledOsPackageManagers
            )
        }
        OutputFilter::TtsClients => {
            direct_install_category!(
                name,
                resolve_tts_client,
                sniff::programs::InstalledTtsClients
            )
        }
        OutputFilter::TerminalApps => {
            direct_install_category!(
                name,
                resolve_terminal_app,
                sniff::programs::InstalledTerminalApps
            )
        }
        OutputFilter::HeadlessAudio => {
            direct_install_category!(name, resolve_audio, sniff::programs::InstalledHeadlessAudio)
        }
        OutputFilter::AiClients => {
            direct_install_category!(name, resolve_agent, sniff::programs::InstalledAiClients)
        }
        OutputFilter::Programs => {
            // Search all categories, install first match
            let categories = [
                OutputFilter::Editors,
                OutputFilter::Utilities,
                OutputFilter::LanguagePackageManagers,
                OutputFilter::OsPackageManagers,
                OutputFilter::TtsClients,
                OutputFilter::TerminalApps,
                OutputFilter::HeadlessAudio,
                OutputFilter::AiClients,
            ];

            for category in categories {
                if direct_install(category, name).is_ok() {
                    return Ok(());
                }
            }

            Err(format!(
                "Unknown program '{}'. Use a category subcommand (e.g., sniff editors install {0}) to see valid names.",
                name
            ).into())
        }
        _ => unreachable!("direct_install only called for program filters"),
    }
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
}
