//! Programs section output formatting (markdown and JSON).

use darkmatter::markdown::output::terminal::{for_terminal, TerminalOptions};
use darkmatter::markdown::Markdown;
use darkmatter::render::link::Link;
use sniff::programs::{ExecutableSource, ProgramsInfo};

use super::OutputFilter;

#[derive(Debug)]
struct ProgramTableEntry {
    name: String,
    binary_name: String,
    installed: bool,
    path: Option<String>,
    version: Option<String>,
    description: String,
    website: String,
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn name_link(name: &str, website: &str) -> String {
    if website.is_empty() {
        return escape_markdown_cell(name);
    }

    let display = escape_markdown_cell(name);
    Link::new(display, website).to_markdown()
}

fn version_allowed(include_versions: bool, source: Option<ExecutableSource>) -> bool {
    include_versions && matches!(source, Some(ExecutableSource::Path))
}

fn collect_program_entries(
    programs: &ProgramsInfo,
    filter: OutputFilter,
    include_versions: bool,
) -> Vec<ProgramTableEntry> {
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;

    let mut entries = Vec::new();

    match filter {
        OutputFilter::Programs | OutputFilter::Editors => {
            for editor in sniff::programs::Editor::iter() {
                let path_info = programs.editors.path_with_source(editor);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.editors.version(editor).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: editor.display_name().to_string(),
                    binary_name: editor.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: editor.description().to_string(),
                    website: editor.website().to_string(),
                });
            }
            if filter == OutputFilter::Editors {
                return entries;
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::Utilities => {
            for util in sniff::programs::Utility::iter() {
                let path_info = programs.utilities.path_with_source(util);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.utilities.version(util).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: util.display_name().to_string(),
                    binary_name: util.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: util.description().to_string(),
                    website: util.website().to_string(),
                });
            }
            if filter == OutputFilter::Utilities {
                return entries;
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::LanguagePackageManagers => {
            for pm in sniff::programs::LanguagePackageManager::iter() {
                let path_info = programs.language_package_managers.path_with_source(pm);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.language_package_managers.version(pm).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: pm.display_name().to_string(),
                    binary_name: pm.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: pm.description().to_string(),
                    website: pm.website().to_string(),
                });
            }
            if filter == OutputFilter::LanguagePackageManagers {
                return entries;
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::OsPackageManagers => {
            for pm in sniff::programs::OsPackageManager::iter() {
                let path_info = programs.os_package_managers.path_with_source(pm);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.os_package_managers.version(pm).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: pm.display_name().to_string(),
                    binary_name: pm.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: pm.description().to_string(),
                    website: pm.website().to_string(),
                });
            }
            if filter == OutputFilter::OsPackageManagers {
                return entries;
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::TtsClients => {
            for client in sniff::programs::TtsClient::iter() {
                let path_info = programs.tts_clients.path_with_source(client);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.tts_clients.version(client).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: client.display_name().to_string(),
                    binary_name: client.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: client.description().to_string(),
                    website: client.website().to_string(),
                });
            }
            if filter == OutputFilter::TtsClients {
                return entries;
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::TerminalApps => {
            for app in sniff::programs::TerminalApp::iter() {
                let path_info = programs.terminal_apps.path_with_source(app);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.terminal_apps.version(app).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: app.display_name().to_string(),
                    binary_name: app.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: app.description().to_string(),
                    website: app.website().to_string(),
                });
            }
            if filter == OutputFilter::TerminalApps {
                return entries;
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::HeadlessAudio => {
            for player in sniff::programs::HeadlessAudio::iter() {
                let path_info = programs.headless_audio.path_with_source(player);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.headless_audio.version(player).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: player.display_name().to_string(),
                    binary_name: player.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: player.description().to_string(),
                    website: player.website().to_string(),
                });
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::AiClients => {
            for client in sniff::programs::AiCli::iter() {
                let path_info = programs.ai_clients.path_with_source(client);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(include_versions, source) {
                    programs.ai_clients.version(client).ok()
                } else {
                    None
                };
                entries.push(ProgramTableEntry {
                    name: client.display_name().to_string(),
                    binary_name: client.binary_name().to_string(),
                    installed,
                    path,
                    version,
                    description: client.description().to_string(),
                    website: client.website().to_string(),
                });
            }
            if filter == OutputFilter::AiClients {
                return entries;
            }
        }
        _ => {}
    }

    entries
}

pub fn print_programs_markdown(programs: &ProgramsInfo, verbose: u8, filter: OutputFilter) {
    let include_versions = verbose > 1;
    let entries = collect_program_entries(programs, filter, include_versions);
    let mut headers = vec!["Name", "Installed", "Description"];

    if verbose > 0 {
        headers.insert(2, "Binary");
        headers.insert(3, "Path");
    }
    if verbose > 1 {
        headers.insert(4, "Version");
    }

    let mut lines = Vec::new();
    lines.push(format!("| {} |", headers.join(" | ")));
    lines.push(format!(
        "| {} |",
        headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));

    for entry in entries {
        let mut cells = vec![
            name_link(&entry.name, &entry.website),
            if entry.installed {
                "✅".to_string()
            } else {
                "❌".to_string()
            },
        ];

        if verbose > 0 {
            cells.push(escape_markdown_cell(&entry.binary_name));
            cells.push(escape_markdown_cell(entry.path.as_deref().unwrap_or("")));
        }
        if verbose > 1 {
            cells.push(escape_markdown_cell(entry.version.as_deref().unwrap_or("")));
        }

        cells.push(escape_markdown_cell(&entry.description));

        lines.push(format!("| {} |", cells.join(" | ")));
    }

    let markdown = Markdown::from(lines.join("\n"));
    match for_terminal(&markdown, TerminalOptions::default()) {
        Ok(rendered) => print!("{}", rendered),
        Err(_) => println!("{}", markdown.content()),
    }
}

/// Rich program metadata for JSON output.
#[derive(::serde::Serialize)]
struct ProgramJsonEntry {
    name: String,
    binary_name: String,
    installed: bool,
    path: Option<String>,
    version: Option<String>,
    description: String,
    website: String,
}

/// Print programs information as JSON.
///
/// ## Arguments
///
/// * `format` - "simple" for backward-compatible output, "full" for rich metadata
pub fn print_programs_json(
    programs: &ProgramsInfo,
    filter: OutputFilter,
    format: &str,
) -> serde_json::Result<()> {
    use serde_json::json;
    use sniff::programs::ProgramMetadata;

    // Simple format: backward-compatible serialization
    if format != "full" {
        let json_value = match filter {
            OutputFilter::Programs => serde_json::to_value(programs)?,
            OutputFilter::Editors => serde_json::to_value(&programs.editors)?,
            OutputFilter::Utilities => serde_json::to_value(&programs.utilities)?,
            OutputFilter::LanguagePackageManagers => {
                serde_json::to_value(&programs.language_package_managers)?
            }
            OutputFilter::OsPackageManagers => serde_json::to_value(&programs.os_package_managers)?,
            OutputFilter::TtsClients => serde_json::to_value(&programs.tts_clients)?,
            OutputFilter::TerminalApps => serde_json::to_value(&programs.terminal_apps)?,
            OutputFilter::HeadlessAudio => serde_json::to_value(&programs.headless_audio)?,
            OutputFilter::AiClients => serde_json::to_value(&programs.ai_clients)?,
            _ => json!({}),
        };
        println!("{}", serde_json::to_string_pretty(&json_value)?);
        return Ok(());
    }

    // Full format: rich metadata with version and path info
    let build_entry = |name: &str,
                       binary: &str,
                       installed: bool,
                       path: Option<std::path::PathBuf>,
                       version: Option<String>,
                       desc: &str,
                       website: &str|
     -> ProgramJsonEntry {
        ProgramJsonEntry {
            name: name.to_string(),
            binary_name: binary.to_string(),
            installed,
            path: path.map(|p| p.display().to_string()),
            version,
            description: desc.to_string(),
            website: website.to_string(),
        }
    };

    let mut entries: Vec<ProgramJsonEntry> = Vec::new();

    // Collect entries based on filter
    match filter {
        OutputFilter::Programs | OutputFilter::Editors => {
            use strum::IntoEnumIterator;
            for editor in sniff::programs::Editor::iter() {
                let path_info = programs.editors.path_with_source(editor);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.editors.version(editor).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    editor.display_name(),
                    editor.binary_name(),
                    installed,
                    path,
                    version,
                    editor.description(),
                    editor.website(),
                ));
            }
            if filter == OutputFilter::Editors {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::Utilities => {
            use strum::IntoEnumIterator;
            for util in sniff::programs::Utility::iter() {
                let path_info = programs.utilities.path_with_source(util);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.utilities.version(util).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    util.display_name(),
                    util.binary_name(),
                    installed,
                    path,
                    version,
                    util.description(),
                    util.website(),
                ));
            }
            if filter == OutputFilter::Utilities {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::LanguagePackageManagers => {
            use strum::IntoEnumIterator;
            for pm in sniff::programs::LanguagePackageManager::iter() {
                let path_info = programs.language_package_managers.path_with_source(pm);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.language_package_managers.version(pm).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    pm.display_name(),
                    pm.binary_name(),
                    installed,
                    path,
                    version,
                    pm.description(),
                    pm.website(),
                ));
            }
            if filter == OutputFilter::LanguagePackageManagers {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::OsPackageManagers => {
            use strum::IntoEnumIterator;
            for pm in sniff::programs::OsPackageManager::iter() {
                let path_info = programs.os_package_managers.path_with_source(pm);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.os_package_managers.version(pm).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    pm.display_name(),
                    pm.binary_name(),
                    installed,
                    path,
                    version,
                    pm.description(),
                    pm.website(),
                ));
            }
            if filter == OutputFilter::OsPackageManagers {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::TtsClients => {
            use strum::IntoEnumIterator;
            for client in sniff::programs::TtsClient::iter() {
                let path_info = programs.tts_clients.path_with_source(client);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.tts_clients.version(client).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    client.display_name(),
                    client.binary_name(),
                    installed,
                    path,
                    version,
                    client.description(),
                    client.website(),
                ));
            }
            if filter == OutputFilter::TtsClients {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::TerminalApps => {
            use strum::IntoEnumIterator;
            for app in sniff::programs::TerminalApp::iter() {
                let path_info = programs.terminal_apps.path_with_source(app);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.terminal_apps.version(app).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    app.display_name(),
                    app.binary_name(),
                    installed,
                    path,
                    version,
                    app.description(),
                    app.website(),
                ));
            }
            if filter == OutputFilter::TerminalApps {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::HeadlessAudio => {
            use strum::IntoEnumIterator;
            for player in sniff::programs::HeadlessAudio::iter() {
                let path_info = programs.headless_audio.path_with_source(player);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.headless_audio.version(player).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    player.display_name(),
                    player.binary_name(),
                    installed,
                    path,
                    version,
                    player.description(),
                    player.website(),
                ));
            }
            if filter == OutputFilter::HeadlessAudio {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    match filter {
        OutputFilter::Programs | OutputFilter::AiClients => {
            use strum::IntoEnumIterator;
            for client in sniff::programs::AiCli::iter() {
                let path_info = programs.ai_clients.path_with_source(client);
                let installed = path_info.is_some();
                let path = path_info.as_ref().map(|(p, _)| p.clone());
                let source = path_info.as_ref().map(|(_, source)| *source);
                let version = if version_allowed(true, source) {
                    programs.ai_clients.version(client).ok()
                } else {
                    None
                };
                entries.push(build_entry(
                    client.display_name(),
                    client.binary_name(),
                    installed,
                    path,
                    version,
                    client.description(),
                    client.website(),
                ));
            }
            if filter == OutputFilter::AiClients {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
        }
        _ => {}
    }

    // For OutputFilter::Programs, print all collected entries
    if filter == OutputFilter::Programs {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    }

    Ok(())
}
