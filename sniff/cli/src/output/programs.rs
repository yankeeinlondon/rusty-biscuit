//! Programs section output formatting (table and JSON).

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table as TerminalTable, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
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

fn linked_name_cell(name: &str, website: &str, term: &Terminal) -> String {
    if website.is_empty() {
        return name.to_string();
    }

    // Use Prose OSC8 support so links are clickable without burning table width.
    Prose::new(format!(r#"<a href="{website}">{name}</a>"#)).render(term)
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

/// Render programs information as a markdown table.
///
/// ## Returns
///
/// A String containing the formatted table ready for terminal output.
pub fn render_programs_markdown(
    programs: &ProgramsInfo,
    verbose: u8,
    filter: OutputFilter,
) -> String {
    let include_versions = verbose > 1;
    let entries = collect_program_entries(programs, filter, include_versions);
    let term = Terminal::default();

    let mut columns = vec![
        TableColumn::new("Name"),
        TableColumn::new("Installed")
            .with_alignment(Alignment::Center)
            .with_uniform_alignment(true),
    ];

    if verbose > 0 {
        columns.push(TableColumn::new("Binary"));
        columns.push(TableColumn::new("Path"));
    }
    if verbose > 1 {
        columns.push(TableColumn::new("Version"));
    }

    columns.push(TableColumn::new("Description"));

    let mut table = TerminalTable::new()
        .with_columns(columns)
        .prefer_cursor_alignment();

    for entry in &entries {
        let mut cells: Vec<TableCellContent> = vec![
            linked_name_cell(&entry.name, &entry.website, &term).into(),
            (if entry.installed { "✅" } else { "❌" }).into(),
        ];

        if verbose > 0 {
            cells.push(entry.binary_name.clone().into());
            cells.push(entry.path.as_deref().unwrap_or("").into());
        }
        if verbose > 1 {
            cells.push(entry.version.as_deref().unwrap_or("").into());
        }

        cells.push(entry.description.clone().into());
        table.add_row(cells);
    }

    table.display(&term).to_string()
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

fn build_json_entry(
    name: &str,
    binary: &str,
    installed: bool,
    path: Option<std::path::PathBuf>,
    version: Option<String>,
    desc: &str,
    website: &str,
) -> ProgramJsonEntry {
    ProgramJsonEntry {
        name: name.to_string(),
        binary_name: binary.to_string(),
        installed,
        path: path.map(|p| p.display().to_string()),
        version,
        description: desc.to_string(),
        website: website.to_string(),
    }
}

fn json_editors(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::Editor::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|editor| {
            let path_info = programs.editors.path_with_source(editor);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.editors.version(editor).ok()
            } else {
                None
            };
            build_json_entry(
                editor.display_name(),
                editor.binary_name(),
                installed,
                path,
                version,
                editor.description(),
                editor.website(),
            )
        })
        .collect()
}

fn json_utilities(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::Utility::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|util| {
            let path_info = programs.utilities.path_with_source(util);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.utilities.version(util).ok()
            } else {
                None
            };
            build_json_entry(
                util.display_name(),
                util.binary_name(),
                installed,
                path,
                version,
                util.description(),
                util.website(),
            )
        })
        .collect()
}

fn json_language_package_managers(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::LanguagePackageManager::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|pm| {
            let path_info = programs.language_package_managers.path_with_source(pm);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.language_package_managers.version(pm).ok()
            } else {
                None
            };
            build_json_entry(
                pm.display_name(),
                pm.binary_name(),
                installed,
                path,
                version,
                pm.description(),
                pm.website(),
            )
        })
        .collect()
}

fn json_os_package_managers(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::OsPackageManager::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|pm| {
            let path_info = programs.os_package_managers.path_with_source(pm);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.os_package_managers.version(pm).ok()
            } else {
                None
            };
            build_json_entry(
                pm.display_name(),
                pm.binary_name(),
                installed,
                path,
                version,
                pm.description(),
                pm.website(),
            )
        })
        .collect()
}

fn json_tts_clients(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::TtsClient::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|client| {
            let path_info = programs.tts_clients.path_with_source(client);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.tts_clients.version(client).ok()
            } else {
                None
            };
            build_json_entry(
                client.display_name(),
                client.binary_name(),
                installed,
                path,
                version,
                client.description(),
                client.website(),
            )
        })
        .collect()
}

fn json_terminal_apps(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::TerminalApp::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|app| {
            let path_info = programs.terminal_apps.path_with_source(app);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.terminal_apps.version(app).ok()
            } else {
                None
            };
            build_json_entry(
                app.display_name(),
                app.binary_name(),
                installed,
                path,
                version,
                app.description(),
                app.website(),
            )
        })
        .collect()
}

fn json_headless_audio(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::HeadlessAudio::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|player| {
            let path_info = programs.headless_audio.path_with_source(player);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.headless_audio.version(player).ok()
            } else {
                None
            };
            build_json_entry(
                player.display_name(),
                player.binary_name(),
                installed,
                path,
                version,
                player.description(),
                player.website(),
            )
        })
        .collect()
}

fn json_ai_clients(programs: &ProgramsInfo) -> Vec<ProgramJsonEntry> {
    use rayon::prelude::*;
    use sniff::programs::ProgramMetadata;
    use strum::IntoEnumIterator;
    sniff::programs::AiCli::iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|client| {
            let path_info = programs.ai_clients.path_with_source(client);
            let installed = path_info.is_some();
            let path = path_info.as_ref().map(|(p, _)| p.clone());
            let source = path_info.as_ref().map(|(_, source)| *source);
            let version = if version_allowed(true, source) {
                programs.ai_clients.version(client).ok()
            } else {
                None
            };
            build_json_entry(
                client.display_name(),
                client.binary_name(),
                installed,
                path,
                version,
                client.description(),
                client.website(),
            )
        })
        .collect()
}

/// Print programs information as JSON.
/// Build a JSON value for the given program filter.
///
/// Version detection spawns one `--version` subprocess per installed program
/// (50-800 ms each). Sequential probing of all 70+ installed programs takes
/// many seconds, so the work is parallelized two ways via Rayon: across
/// the eight categories (using nested `rayon::join`) and within each category
/// (using `par_iter`). Wall-clock time becomes roughly the slowest single
/// program probe rather than the sum of all probes.
pub fn build_programs_json(
    programs: &ProgramsInfo,
    filter: OutputFilter,
) -> serde_json::Result<serde_json::Value> {
    let entries: Vec<ProgramJsonEntry> = match filter {
        OutputFilter::Editors => json_editors(programs),
        OutputFilter::Utilities => json_utilities(programs),
        OutputFilter::LanguagePackageManagers => json_language_package_managers(programs),
        OutputFilter::OsPackageManagers => json_os_package_managers(programs),
        OutputFilter::TtsClients => json_tts_clients(programs),
        OutputFilter::TerminalApps => json_terminal_apps(programs),
        OutputFilter::HeadlessAudio => json_headless_audio(programs),
        OutputFilter::AiClients => json_ai_clients(programs),
        OutputFilter::Programs => {
            // Run all 8 categories concurrently. Nested rayon::join cooperates
            // with the inner par_iter calls via the shared thread pool.
            let ((editors, utilities), (lang_pms, os_pms)) = rayon::join(
                || {
                    rayon::join(
                        || json_editors(programs),
                        || json_utilities(programs),
                    )
                },
                || {
                    rayon::join(
                        || json_language_package_managers(programs),
                        || json_os_package_managers(programs),
                    )
                },
            );
            let ((tts, terms), (audio, ai)) = rayon::join(
                || {
                    rayon::join(
                        || json_tts_clients(programs),
                        || json_terminal_apps(programs),
                    )
                },
                || {
                    rayon::join(
                        || json_headless_audio(programs),
                        || json_ai_clients(programs),
                    )
                },
            );

            let mut all = Vec::with_capacity(
                editors.len()
                    + utilities.len()
                    + lang_pms.len()
                    + os_pms.len()
                    + tts.len()
                    + terms.len()
                    + audio.len()
                    + ai.len(),
            );
            all.extend(editors);
            all.extend(utilities);
            all.extend(lang_pms);
            all.extend(os_pms);
            all.extend(tts);
            all.extend(terms);
            all.extend(audio);
            all.extend(ai);
            all
        }
        _ => Vec::new(),
    };

    serde_json::to_value(&entries)
}
