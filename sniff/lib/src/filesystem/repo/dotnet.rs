//! .NET solution (`*.sln` / `*.slnx`) detection.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;

use crate::Result;
use crate::performance;
use crate::performance::counters;

use super::detection::{
    DetectorOutcome, probe_exists,
};
use super::seed::{PackageSeed, merge_seeds};
use super::standard::{MonorepoStandard, PackageProvenance};

/// Project file extensions a solution entry must carry to count as a package.
///
/// Solution folders share the `Project(...)` syntax but point at a name without
/// a project extension, so filtering by extension drops them.
const PROJECT_EXTENSIONS: [&str; 3] = [".csproj", ".fsproj", ".vbproj"];

pub(super) fn detect_dotnet_solution(root: &Path) -> Result<Option<DetectorOutcome>> {
    let solutions = find_solution_files(root);
    if solutions.is_empty() {
        return Ok(None);
    }

    let mut seeds = Vec::new();
    for solution in &solutions {
        performance::increment_counter(counters::FS_FILE_OPENS, 1);
        let Ok(content) = std::fs::read_to_string(solution) else {
            continue;
        };
        performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
        performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
        let project_paths = if has_slnx_extension(solution) {
            parse_slnx_projects(&content)
        } else {
            parse_sln_projects(&content)
        };
        for project in project_paths {
            let project_dir = match project.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => root.join(parent),
                _ => root.to_path_buf(),
            };
            if !probe_exists(&project_dir) {
                continue;
            }
            seeds.push(PackageSeed::new(
                &project_dir,
                root,
                MonorepoStandard::DotNetSolution,
                PackageProvenance::Explicit,
            ));
        }
    }

    if seeds.is_empty() {
        return Ok(None);
    }


    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::DotNetSolution,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

/// Whether `root` directly contains any `.sln` / `.slnx` solution file.
///
/// Solution file names are arbitrary, so detection scans the root directory for
/// the extension rather than probing a fixed name.
pub(super) fn root_has_solution_file(root: &Path) -> bool {
    !find_solution_files(root).is_empty()
}

/// Collect the `.sln` / `.slnx` files directly under `root` (non-recursive).
fn find_solution_files(root: &Path) -> Vec<PathBuf> {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut solutions: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            name.ends_with(".sln") || name.ends_with(".slnx")
        })
        .collect();
    solutions.sort();
    solutions
}

fn has_slnx_extension(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.ends_with(".slnx"))
}

/// Parse project paths from a classic `.sln` solution.
///
/// Each `Project("{type}") = "Name", "rel\path.csproj", "{guid}"` line carries
/// the project path as its second quoted field. Solution paths use Windows `\`
/// separators, which are normalized to `/`. Only entries ending in a project
/// extension are kept, so solution folders are excluded.
fn parse_sln_projects(content: &str) -> Vec<PathBuf> {
    static PROJECT_RE: OnceLock<Regex> = OnceLock::new();
    let re = PROJECT_RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*Project\("[^"]*"\)\s*=\s*"[^"]*",\s*"([^"]+)""#).unwrap()
    });

    re.captures_iter(content)
        .filter_map(|cap| project_path(cap.get(1)?.as_str()))
        .collect()
}

/// Parse project paths from an XML `.slnx` solution (`<Project Path="..." />`).
fn parse_slnx_projects(content: &str) -> Vec<PathBuf> {
    static PROJECT_RE: OnceLock<Regex> = OnceLock::new();
    let re = PROJECT_RE.get_or_init(|| Regex::new(r#"<Project\s+Path="([^"]+)""#).unwrap());

    re.captures_iter(content)
        .filter_map(|cap| project_path(cap.get(1)?.as_str()))
        .collect()
}

/// Normalize a solution-relative project reference, keeping only real project
/// files and converting Windows separators to `/`.
fn project_path(raw: &str) -> Option<PathBuf> {
    let normalized = raw.trim().replace('\\', "/");
    let is_project = PROJECT_EXTENSIONS
        .iter()
        .any(|ext| normalized.to_ascii_lowercase().ends_with(ext));
    if is_project {
        Some(PathBuf::from(normalized))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sln_extracts_project_paths_and_skips_folders() {
        let content = "\
Microsoft Visual Studio Solution File, Format Version 12.00
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"App\", \"src\\App\\App.csproj\", \"{1}\"
EndProject
Project(\"{2150E333-8FDC-42A3-9474-1A3956D46DE8}\") = \"solution-items\", \"solution-items\", \"{2}\"
EndProject
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Lib\", \"src\\Lib\\Lib.csproj\", \"{3}\"
EndProject
";
        let projects = parse_sln_projects(content);
        assert_eq!(
            projects,
            vec![
                PathBuf::from("src/App/App.csproj"),
                PathBuf::from("src/Lib/Lib.csproj")
            ]
        );
    }

    #[test]
    fn parse_slnx_extracts_project_paths() {
        let content = "<Solution>\n  <Project Path=\"src/App/App.csproj\" />\n\
             <Project Path=\"src/Lib/Lib.fsproj\" />\n</Solution>\n";
        assert_eq!(
            parse_slnx_projects(content),
            vec![
                PathBuf::from("src/App/App.csproj"),
                PathBuf::from("src/Lib/Lib.fsproj")
            ]
        );
    }

    #[test]
    fn parse_sln_empty_when_no_projects() {
        assert!(parse_sln_projects("Microsoft Visual Studio Solution File\n").is_empty());
    }
}
