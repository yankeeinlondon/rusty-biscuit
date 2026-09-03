use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::os::{OsType, detect_os_with_request};
use sniff::programs::ExecutableIndex;
use sniff::request::OsRequest;
use thiserror::Error;
use wait_timeout::ChildExt;

const EXPECTED_DMLS_VERSION: &str = env!("DMLS_PACKAGE_VERSION");

const EXPECTED_EXTENSION_ID: &str = "dmls";
const LOG_TAIL_BYTES: u64 = 64 * 1024;
static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum CliError {
    #[error("host OS discovery failed: {0}")]
    HostDiscovery(String),
    #[error("unable to determine the required per-user directory")]
    UserDirectory,
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("the source is not a DMLS Zed extension: {0}")]
    InvalidSource(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPaths {
    pub staging_dir: PathBuf,
    pub zed_data_dir: PathBuf,
    pub zed_log: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PathOverrides {
    pub staging_dir: Option<PathBuf>,
    pub zed_data_dir: Option<PathBuf>,
    pub zed_log: Option<PathBuf>,
}

impl PathOverrides {
    pub fn none() -> Self {
        Self {
            staging_dir: None,
            zed_data_dir: None,
            zed_log: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryRoots {
    pub data: PathBuf,
    pub local_data: PathBuf,
    pub home: PathBuf,
}

pub fn default_paths(
    os: OsType,
    roots: &DirectoryRoots,
    overrides: &PathOverrides,
) -> Result<UserPaths, CliError> {
    let (staging_root, zed_root, zed_name) = match os {
        OsType::MacOS => (&roots.data, &roots.data, "Zed"),
        OsType::Linux => (&roots.data, &roots.local_data, "zed"),
        OsType::Windows => (&roots.local_data, &roots.local_data, "Zed"),
        other => {
            return Err(CliError::HostDiscovery(format!(
                "unsupported operating system: {other}"
            )));
        }
    };
    let staging_dir = overrides
        .staging_dir
        .clone()
        .unwrap_or_else(|| staging_root.join("dmls").join("zed-dmls"));
    let zed_data_dir = overrides
        .zed_data_dir
        .clone()
        .unwrap_or_else(|| zed_root.join(zed_name));
    let zed_log = overrides.zed_log.clone().unwrap_or_else(|| match os {
        OsType::MacOS => roots.home.join("Library").join("Logs").join("Zed").join("Zed.log"),
        OsType::Linux | OsType::Windows => zed_data_dir.join("logs").join("Zed.log"),
        _ => unreachable!("unsupported operating systems returned above"),
    });
    Ok(UserPaths {
        staging_dir,
        zed_data_dir,
        zed_log,
    })
}

pub struct HostDiscovery {
    pub os: OsType,
    pub roots: DirectoryRoots,
    executables: ExecutableIndex,
}

impl HostDiscovery {
    pub fn capture(overrides: &PathOverrides) -> Result<Self, CliError> {
        let os = detect_os_with_request(&OsRequest::summary())
            .map_err(|error| CliError::HostDiscovery(error.to_string()))?
            .os_type;
        let roots = discover_directory_roots(
            os,
            overrides,
            dirs::data_dir,
            dirs::data_local_dir,
            dirs::home_dir,
            || std::env::var("FLATPAK_XDG_DATA_HOME"),
        )?;
        Ok(Self {
            os,
            roots,
            executables: ExecutableIndex::build_eager_path_only(),
        })
    }
}

fn discover_directory_roots(
    os: OsType,
    overrides: &PathOverrides,
    data_dir: impl FnOnce() -> Option<PathBuf>,
    data_local_dir: impl FnOnce() -> Option<PathBuf>,
    home_dir: impl FnOnce() -> Option<PathBuf>,
    flatpak_xdg_data_home: impl FnOnce() -> Result<String, std::env::VarError>,
) -> Result<DirectoryRoots, CliError> {
    let data_required = match os {
        OsType::MacOS => overrides.staging_dir.is_none() || overrides.zed_data_dir.is_none(),
        OsType::Linux => overrides.staging_dir.is_none(),
        _ => false,
    };
    let local_data_required = match os {
        OsType::Linux => overrides.zed_data_dir.is_none() || overrides.zed_log.is_none(),
        OsType::Windows => {
            overrides.staging_dir.is_none()
                || overrides.zed_data_dir.is_none()
                || overrides.zed_log.is_none()
        }
        _ => false,
    };
    let home_required = os == OsType::MacOS && overrides.zed_log.is_none();

    Ok(DirectoryRoots {
        data: if data_required {
            data_dir().ok_or(CliError::UserDirectory)?
        } else {
            PathBuf::new()
        },
        local_data: if local_data_required {
            if os == OsType::Linux {
                match flatpak_xdg_data_home() {
                    Ok(path) => PathBuf::from(path),
                    Err(_) => data_local_dir().ok_or(CliError::UserDirectory)?,
                }
            } else {
                data_local_dir().ok_or(CliError::UserDirectory)?
            }
        } else {
            PathBuf::new()
        },
        home: if home_required {
            home_dir().ok_or(CliError::UserDirectory)?
        } else {
            PathBuf::new()
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Registered,
    ManualRegistrationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReport {
    pub staging_dir: PathBuf,
    pub status: StageStatus,
}

pub fn checked_in_extension_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CLI package has a dmls parent")
        .join("zed-dmls")
}

pub fn stage_extension(source: &Path, paths: &UserPaths) -> Result<StageReport, CliError> {
    stage_extension_inner(source, paths, false)
}

fn stage_extension_inner(
    source: &Path,
    paths: &UserPaths,
    fail_before_activation: bool,
) -> Result<StageReport, CliError> {
    validate_manifest(&source.join("extension.toml"))?;
    let parent = paths.staging_dir.parent().ok_or_else(|| CliError::InvalidSource(
        "staging directory must have a parent".to_string(),
    ))?;
    fs::create_dir_all(parent).map_err(|source| io_error("create staging parent", source))?;

    let token = format!(
        "{}-{}",
        std::process::id(),
        STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let leaf = paths
        .staging_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("zed-dmls");
    let temporary = parent.join(format!(".{leaf}.stage-{token}"));
    let backup = parent.join(format!(".{leaf}.backup-{token}"));

    let result = (|| {
        fs::create_dir(&temporary).map_err(|source| io_error("create temporary stage", source))?;
        for file in ["extension.toml", "Cargo.toml", "Cargo.lock", "README.md"] {
            copy_regular_file(&source.join(file), &temporary.join(file))?;
        }
        copy_directory(&source.join("src"), &temporary.join("src"))?;

        let had_previous = paths.staging_dir.exists();
        if had_previous {
            fs::rename(&paths.staging_dir, &backup)
                .map_err(|source| io_error("move previous stage to backup", source))?;
        }

        let activation = if fail_before_activation {
            Err(std::io::Error::other("injected activation failure"))
        } else {
            fs::rename(&temporary, &paths.staging_dir)
        };
        if let Err(source) = activation {
            if had_previous {
                fs::rename(&backup, &paths.staging_dir)
                    .map_err(|restore| io_error("restore previous stage after swap failure", restore))?;
            }
            return Err(io_error("activate staged extension", source));
        }
        if had_previous {
            fs::remove_dir_all(&backup)
                .map_err(|source| io_error("remove staging backup", source))?;
        }
        Ok(())
    })();

    if temporary.exists() {
        let _ = fs::remove_dir_all(&temporary);
    }
    if result.is_err() && backup.exists() && !paths.staging_dir.exists() {
        let _ = fs::rename(&backup, &paths.staging_dir);
    }
    result?;

    let registration = paths.zed_data_dir.join("extensions").join("installed").join("dmls");
    let status = match (fs::canonicalize(&registration), fs::canonicalize(&paths.staging_dir)) {
        (Ok(registered), Ok(staged)) if registered == staged => StageStatus::Registered,
        _ => StageStatus::ManualRegistrationRequired,
    };
    Ok(StageReport {
        staging_dir: paths.staging_dir.clone(),
        status,
    })
}

fn copy_regular_file(source: &Path, destination: &Path) -> Result<(), CliError> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| io_error(format!("read {}", source.display()), error))?;
    if !metadata.file_type().is_file() {
        return Err(CliError::InvalidSource(format!(
            "{} is not a regular file",
            source.display()
        )));
    }
    fs::copy(source, destination)
        .map_err(|error| io_error(format!("copy {}", source.display()), error))?;
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), CliError> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| io_error(format!("read {}", source.display()), error))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(CliError::InvalidSource(format!(
            "{} is not a regular directory",
            source.display()
        )));
    }
    fs::create_dir(destination)
        .map_err(|error| io_error(format!("create {}", destination.display()), error))?;
    for entry in fs::read_dir(source)
        .map_err(|error| io_error(format!("read {}", source.display()), error))?
    {
        let entry = entry.map_err(|error| io_error("read source directory entry", error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| io_error(format!("inspect {}", entry.path().display()), error))?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            copy_regular_file(&entry.path(), &target)?;
        } else {
            return Err(CliError::InvalidSource(format!(
                "{} contains a link or unsupported entry",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn validate_manifest(path: &Path) -> Result<(), CliError> {
    let source = fs::read_to_string(path)
        .map_err(|error| CliError::InvalidSource(format!("{}: {error}", path.display())))?;
    let manifest: toml::Value = toml::from_str(&source)
        .map_err(|error| CliError::InvalidSource(format!("{}: {error}", path.display())))?;
    if manifest.get("id").and_then(toml::Value::as_str) != Some(EXPECTED_EXTENSION_ID) {
        return Err(CliError::InvalidSource(format!(
            "{} does not declare id = \"dmls\"",
            path.display()
        )));
    }
    Ok(())
}

fn io_error(context: impl Into<String>, source: std::io::Error) -> CliError {
    CliError::Io {
        context: context.into(),
        source,
    }
}

pub trait BinaryProbe {
    fn dmls_path(&self) -> Option<PathBuf>;
    fn dmls_version(&self, path: &Path) -> Result<String, String>;
}

impl BinaryProbe for HostDiscovery {
    fn dmls_path(&self) -> Option<PathBuf> {
        self.executables.find("dmls")
    }

    fn dmls_version(&self, path: &Path) -> Result<String, String> {
        let mut child = Command::new(path)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;
        let status = match child
            .wait_timeout(Duration::from_secs(3))
            .map_err(|error| error.to_string())?
        {
            Some(status) => status,
            None => {
                child.kill().map_err(|error| error.to_string())?;
                child.wait().map_err(|error| error.to_string())?;
                return Err("timed out after 3 seconds".to_string());
            }
        };
        let mut stdout = String::new();
        if let Some(mut pipe) = child.stdout.take() {
            pipe.read_to_string(&mut stdout)
                .map_err(|error| error.to_string())?;
        }
        if status.success() {
            Ok(stdout.trim().to_string())
        } else {
            Err(format!("exited with {status}"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportLevel {
    Success,
    Warning,
    Failure,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportLine {
    pub level: ReportLevel,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub healthy: bool,
    pub lines: Vec<ReportLine>,
}

pub fn doctor(paths: &UserPaths, probe: &impl BinaryProbe) -> DoctorReport {
    let mut report = DoctorReport {
        healthy: true,
        lines: Vec::new(),
    };
    check_binary(probe, &mut report);
    let registration_healthy = check_registration(paths, &mut report);
    check_log(paths, registration_healthy, &mut report);
    report
}

pub fn should_run_doctor(paths: &UserPaths) -> bool {
    paths.zed_data_dir.exists()
        || fs::symlink_metadata(
            paths
                .zed_data_dir
                .join("extensions")
                .join("installed")
                .join("dmls"),
        )
        .is_ok()
}

fn check_binary(probe: &impl BinaryProbe, report: &mut DoctorReport) {
    let Some(path) = probe.dmls_path() else {
        failure(report, "dmls binary missing from PATH; run `just install-dmls`".to_string());
        return;
    };
    match probe.dmls_version(&path) {
        Ok(output) if output.trim() == format!("dmls {EXPECTED_DMLS_VERSION}") => {
            report.lines.push(ReportLine {
                level: ReportLevel::Success,
                text: format!(
                    "dmls {} is version-compatible at {}; freshness is unverified because no installation receipt is available",
                    EXPECTED_DMLS_VERSION,
                    path.display()
                ),
            });
        }
        Ok(output) => failure(
            report,
            format!(
                "dmls version-incompatible at {}: expected {}, got `{output}`; run `just install-dmls`",
                path.display(),
                EXPECTED_DMLS_VERSION
            ),
        ),
        Err(error) => failure(
            report,
            format!("could not query dmls at {}: {error}", path.display()),
        ),
    }
}

fn check_registration(paths: &UserPaths, report: &mut DoctorReport) -> bool {
    let registration = paths.zed_data_dir.join("extensions").join("installed").join("dmls");
    if fs::symlink_metadata(&registration).is_err() {
        let detail = if paths.zed_data_dir.exists() {
            "dev extension not installed"
        } else {
            "Zed data directory is absent; dev extension not installed"
        };
        failure(
            report,
            format!(
                "{detail} at `{}`; run `just install-zed` and select the printed stable folder",
                paths.zed_data_dir.display()
            ),
        );
        return false;
    }
    check_registration_target(report, || {
        fs::canonicalize(&registration).map_err(|_| {
            fs::read_link(&registration)
                .map(|target| {
                    if target.is_absolute() {
                        target
                    } else {
                        registration.parent().unwrap_or(Path::new("")).join(target)
                    }
                })
                .unwrap_or_else(|_| registration.clone())
        })
    })
}

fn check_registration_target(
    report: &mut DoctorReport,
    resolve: impl FnOnce() -> Result<PathBuf, PathBuf>,
) -> bool {
    let resolved = match resolve() {
        Ok(path) => path,
        Err(target) => {
            failure(
                report,
                format!(
                    "dev extension points at `{}`, which no longer exists (worktree removed?); run `just install-zed`",
                    target.display()
                ),
            );
            return false;
        }
    };
    let manifest = resolved.join("extension.toml");
    if let Err(error) = validate_manifest(&manifest) {
        failure(
            report,
            format!(
                "selected target `{}` is not the DMLS Zed extension: {error}; run `just install-zed`",
                resolved.display()
            ),
        );
        return false;
    }
    report.lines.push(ReportLine {
        level: ReportLevel::Success,
        text: format!("DMLS dev extension resolves to `{}`", resolved.display()),
    });
    true
}

fn check_log(paths: &UserPaths, registration_healthy: bool, report: &mut DoctorReport) {
    let Ok(tail) = read_log_tail(&paths.zed_log) else {
        report.lines.push(ReportLine {
            level: ReportLevel::Info,
            text: format!("no readable Zed log at `{}`", paths.zed_log.display()),
        });
        return;
    };
    let Some(id) = newest_manifest_error_id(&tail) else {
        return;
    };
    let meaning = if id == EXPECTED_EXTENSION_ID {
        "a prior DMLS registration was broken"
    } else {
        "the wrong folder was selected during a dev-extension install"
    };
    let text = format!(
        "Zed log reports `No extension manifest found for extension {id}`: {meaning}"
    );
    if registration_healthy {
        report.lines.push(ReportLine {
            level: ReportLevel::Warning,
            text: format!("historical context: {text}; the current registration is healthy"),
        });
    } else {
        failure(report, text);
    }
}

fn read_log_tail(path: &Path) -> Result<String, std::io::Error> {
    let file = fs::File::open(path)?;
    let length = file.metadata()?.len();
    let mut limited = file;
    if length > LOG_TAIL_BYTES {
        use std::io::{Seek, SeekFrom};
        limited.seek(SeekFrom::Start(length - LOG_TAIL_BYTES))?;
    }
    let mut tail = Vec::new();
    limited.read_to_end(&mut tail)?;
    Ok(String::from_utf8_lossy(&tail).into_owned())
}

fn newest_manifest_error_id(tail: &str) -> Option<String> {
    const PREFIX: &str = "No extension manifest found for extension ";
    tail.lines().rev().find_map(|line| {
        let rest = line.split_once(PREFIX)?.1.trim();
        let id = rest
            .trim_matches(|character: char| character == '`' || character == '"')
            .split(|character: char| character.is_whitespace() || character == '`' || character == '"')
            .next()?;
        (!id.is_empty()).then(|| id.to_string())
    })
}

fn failure(report: &mut DoctorReport, text: String) {
    report.healthy = false;
    report.lines.push(ReportLine {
        level: ReportLevel::Failure,
        text,
    });
}

pub fn render_lines(lines: &[ReportLine], plain: bool) -> String {
    let terminal = Terminal::new();
    lines
        .iter()
        .map(|line| {
            let prefix = match line.level {
                ReportLevel::Success => "OK",
                ReportLevel::Warning => "WARN",
                ReportLevel::Failure => "FAIL",
                ReportLevel::Info => "INFO",
            };
            let text = format!("{prefix}: {}", line.text);
            let markup = if plain {
                Prose::escape_text(&text)
            } else {
                let tag = match line.level {
                    ReportLevel::Success => "green",
                    ReportLevel::Warning => "yellow",
                    ReportLevel::Failure => "red",
                    ReportLevel::Info => "dim",
                };
                format!("<{tag}>{}</{tag}>", Prose::escape_text(&text))
            };
            Prose::new(markup).render(&terminal)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct FakeProbe {
        path: Option<PathBuf>,
        version: Result<String, String>,
    }

    impl BinaryProbe for FakeProbe {
        fn dmls_path(&self) -> Option<PathBuf> {
            self.path.clone()
        }

        fn dmls_version(&self, _path: &Path) -> Result<String, String> {
            self.version.clone()
        }
    }

    fn roots(root: &Path) -> DirectoryRoots {
        DirectoryRoots {
            data: root.join("data"),
            local_data: root.join("local"),
            home: root.join("home"),
        }
    }

    fn paths(root: &Path) -> UserPaths {
        UserPaths {
            staging_dir: root.join("stable").join("dmls").join("zed-dmls"),
            zed_data_dir: root.join("zed"),
            zed_log: root.join("Zed.log"),
        }
    }

    fn source(root: &Path) -> PathBuf {
        let source = root.join("source");
        fs::create_dir_all(source.join("src")).unwrap();
        fs::write(source.join("extension.toml"), "id = \"dmls\"\n").unwrap();
        fs::write(source.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::write(source.join("Cargo.lock"), "# lock\n").unwrap();
        fs::write(source.join("README.md"), "docs\n").unwrap();
        fs::write(source.join("src/lib.rs"), "old\n").unwrap();
        source
    }

    #[test]
    fn platform_paths_use_native_data_roots_and_overrides() {
        let temp = TempDir::new().unwrap();
        let roots = roots(temp.path());
        let none = PathOverrides::none();
        let mac = default_paths(OsType::MacOS, &roots, &none).unwrap();
        assert_eq!(mac.staging_dir, roots.data.join("dmls/zed-dmls"));
        assert_eq!(mac.zed_data_dir, roots.data.join("Zed"));
        assert_eq!(mac.zed_log, roots.home.join("Library/Logs/Zed/Zed.log"));
        let linux = default_paths(OsType::Linux, &roots, &none).unwrap();
        assert_eq!(linux.zed_data_dir, roots.local_data.join("zed"));
        let windows = default_paths(OsType::Windows, &roots, &none).unwrap();
        assert_eq!(windows.staging_dir, roots.local_data.join("dmls/zed-dmls"));
        assert_eq!(windows.zed_data_dir, roots.local_data.join("Zed"));

        let custom = temp.path().join("custom");
        let overridden = default_paths(
            OsType::Linux,
            &roots,
            &PathOverrides {
                staging_dir: Some(custom.join("stage")),
                zed_data_dir: Some(custom.join("data")),
                zed_log: Some(custom.join("log")),
            },
        )
        .unwrap();
        assert_eq!(overridden.staging_dir, custom.join("stage"));
        assert_eq!(overridden.zed_data_dir, custom.join("data"));
        assert_eq!(overridden.zed_log, custom.join("log"));
    }

    #[test]
    fn linux_directory_discovery_uses_ordinary_xdg_roots() {
        let temp = TempDir::new().unwrap();
        let xdg_data = temp.path().join("xdg-data");
        let xdg_local_data = temp.path().join("xdg-local-data");
        let roots = discover_directory_roots(
            OsType::Linux,
            &PathOverrides::none(),
            || Some(xdg_data.clone()),
            || Some(xdg_local_data.clone()),
            || unreachable!("Linux discovery does not require the home directory"),
            || Err(std::env::VarError::NotPresent),
        )
        .unwrap();
        let paths = default_paths(OsType::Linux, &roots, &PathOverrides::none()).unwrap();

        assert_eq!(paths.staging_dir, xdg_data.join("dmls/zed-dmls"));
        assert_eq!(paths.zed_data_dir, xdg_local_data.join("zed"));
        assert_eq!(paths.zed_log, xdg_local_data.join("zed/logs/Zed.log"));
    }

    #[test]
    fn linux_directory_discovery_prefers_flatpak_xdg_data_home_for_zed() {
        let temp = TempDir::new().unwrap();
        let xdg_data = temp.path().join("xdg-data");
        let flatpak_data = temp.path().join("flatpak-data");
        let roots = discover_directory_roots(
            OsType::Linux,
            &PathOverrides::none(),
            || Some(xdg_data.clone()),
            || unreachable!("Flatpak discovery must take precedence over ordinary XDG data"),
            || unreachable!("Linux discovery does not require the home directory"),
            || Ok(flatpak_data.to_string_lossy().into_owned()),
        )
        .unwrap();
        let paths = default_paths(OsType::Linux, &roots, &PathOverrides::none()).unwrap();

        assert_eq!(paths.staging_dir, xdg_data.join("dmls/zed-dmls"));
        assert_eq!(paths.zed_data_dir, flatpak_data.join("zed"));
        assert_eq!(paths.zed_log, flatpak_data.join("zed/logs/Zed.log"));
    }

    #[test]
    fn staging_is_allowlisted_repeatable_and_removes_stale_files() {
        let temp = TempDir::new().unwrap();
        let source = source(temp.path());
        fs::write(source.join("extension.wasm"), "excluded").unwrap();
        fs::create_dir(source.join("target")).unwrap();
        fs::write(source.join("target/output"), "excluded").unwrap();
        fs::write(source.join("unrecognized"), "excluded").unwrap();
        let paths = paths(temp.path());
        let first = stage_extension(&source, &paths).unwrap();
        assert_eq!(first.status, StageStatus::ManualRegistrationRequired);
        assert!(!paths.staging_dir.join("extension.wasm").exists());
        assert!(!paths.staging_dir.join("target").exists());
        assert!(!paths.staging_dir.join("unrecognized").exists());

        fs::write(paths.staging_dir.join("stale"), "remove me").unwrap();
        fs::write(source.join("src/lib.rs"), "new\n").unwrap();
        stage_extension(&source, &paths).unwrap();
        assert_eq!(fs::read_to_string(paths.staging_dir.join("src/lib.rs")).unwrap(), "new\n");
        assert!(!paths.staging_dir.join("stale").exists());
        let siblings = fs::read_dir(paths.staging_dir.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(siblings, vec!["zed-dmls"]);
    }

    #[cfg(unix)]
    #[test]
    fn linked_registration_survives_source_worktree_removal_on_unix() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let source_worktree = source(temp.path());
        let paths = paths(temp.path());
        let registration = paths.zed_data_dir.join("extensions/installed/dmls");
        fs::create_dir_all(registration.parent().unwrap()).unwrap();

        stage_extension(&source_worktree, &paths).unwrap();
        symlink(&paths.staging_dir, &registration).unwrap();
        let report = stage_extension(&source_worktree, &paths).unwrap();
        assert_eq!(report.status, StageStatus::Registered);

        fs::remove_dir_all(&source_worktree).unwrap();

        assert!(
            fs::symlink_metadata(&registration)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::canonicalize(&registration).unwrap(),
            fs::canonicalize(&paths.staging_dir).unwrap()
        );
        let report = doctor(
            &paths,
            &FakeProbe {
                path: Some(PathBuf::from("/bin/dmls")),
                version: Ok(format!("dmls {EXPECTED_DMLS_VERSION}")),
            },
        );
        assert!(report.healthy);
        let expected_resolution = format!(
            "DMLS dev extension resolves to `{}`",
            fs::canonicalize(&paths.staging_dir).unwrap().display()
        );
        assert!(report.lines.iter().any(|line| line.text == expected_resolution));
    }

    #[cfg(windows)]
    #[test]
    fn junction_registration_survives_source_worktree_removal_on_windows() {
        let temp = TempDir::new().unwrap();
        let source_worktree = source(temp.path());
        let paths = paths(temp.path());
        let registration = paths.zed_data_dir.join("extensions/installed/dmls");
        fs::create_dir_all(registration.parent().unwrap()).unwrap();

        stage_extension(&source_worktree, &paths).unwrap();
        let output = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&registration)
            .arg(&paths.staging_dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "mklink /J failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = stage_extension(&source_worktree, &paths).unwrap();
        assert_eq!(report.status, StageStatus::Registered);

        fs::remove_dir_all(&source_worktree).unwrap();

        assert_eq!(
            fs::canonicalize(&registration).unwrap(),
            fs::canonicalize(&paths.staging_dir).unwrap()
        );
        let report = doctor(
            &paths,
            &FakeProbe {
                path: Some(PathBuf::from("dmls.exe")),
                version: Ok(format!("dmls {EXPECTED_DMLS_VERSION}")),
            },
        );
        assert!(report.healthy);
        let expected_resolution = format!(
            "DMLS dev extension resolves to `{}`",
            fs::canonicalize(&paths.staging_dir).unwrap().display()
        );
        assert!(report.lines.iter().any(|line| line.text == expected_resolution));
    }

    #[test]
    fn invalid_manifest_does_not_mutate_existing_stage() {
        let temp = TempDir::new().unwrap();
        let source = source(temp.path());
        let paths = paths(temp.path());
        fs::create_dir_all(&paths.staging_dir).unwrap();
        fs::write(paths.staging_dir.join("sentinel"), "preserved").unwrap();
        fs::write(source.join("extension.toml"), "id = \"wrong\"\n").unwrap();
        assert!(stage_extension(&source, &paths).is_err());
        assert_eq!(fs::read_to_string(paths.staging_dir.join("sentinel")).unwrap(), "preserved");
    }

    #[test]
    fn activation_failure_rolls_back_and_cleans_siblings() {
        let temp = TempDir::new().unwrap();
        let source = source(temp.path());
        let paths = paths(temp.path());
        fs::create_dir_all(&paths.staging_dir).unwrap();
        fs::write(paths.staging_dir.join("sentinel"), "preserved").unwrap();
        assert!(stage_extension_inner(&source, &paths, true).is_err());
        assert_eq!(fs::read_to_string(paths.staging_dir.join("sentinel")).unwrap(), "preserved");
        let siblings = fs::read_dir(paths.staging_dir.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(siblings, vec!["zed-dmls"]);
    }

    #[test]
    fn doctor_reports_missing_binary_and_registration() {
        let temp = TempDir::new().unwrap();
        let report = doctor(
            &paths(temp.path()),
            &FakeProbe { path: None, version: Ok(String::new()) },
        );
        assert!(!report.healthy);
        let output = render_lines(&report.lines, true);
        assert!(output.contains("dmls binary missing from PATH"));
        assert!(output.contains("Zed data directory is absent; dev extension not installed"));
        assert!(output.contains("run `just install-zed`"));
    }

    #[test]
    fn conditional_doctor_only_runs_when_zed_state_exists() {
        let temp = TempDir::new().unwrap();
        let paths = paths(temp.path());
        assert!(!should_run_doctor(&paths));
        fs::create_dir_all(&paths.zed_data_dir).unwrap();
        assert!(should_run_doctor(&paths));
    }

    #[test]
    fn doctor_reports_dangling_registration_target() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("removed-worktree/zed-dmls");
        let mut report = DoctorReport {
            healthy: true,
            lines: Vec::new(),
        };
        assert!(!check_registration_target(&mut report, || Err(missing.clone())));
        let output = render_lines(&report.lines, true);
        assert!(output.contains(&missing.display().to_string()));
        assert!(output.contains("worktree removed?"));
    }

    #[test]
    fn doctor_distinguishes_missing_manifest_wrong_id_and_wrong_folder_log() {
        let temp = TempDir::new().unwrap();
        let paths = paths(temp.path());
        let registration = paths.zed_data_dir.join("extensions/installed/dmls");
        fs::create_dir_all(&registration).unwrap();
        fs::write(
            &paths.zed_log,
            "Failed to install dev extension: No extension manifest found for extension vscode-dmls\n",
        )
        .unwrap();
        let probe = FakeProbe {
            path: Some(PathBuf::from("dmls")),
            version: Ok(format!("dmls {EXPECTED_DMLS_VERSION}")),
        };
        let missing = doctor(&paths, &probe);
        let output = render_lines(&missing.lines, true);
        assert!(output.contains("is not the DMLS Zed extension"));
        assert!(output.contains("wrong folder was selected"));

        fs::write(registration.join("extension.toml"), "id = \"vscode-dmls\"\n").unwrap();
        let wrong = doctor(&paths, &probe);
        assert!(render_lines(&wrong.lines, true).contains("does not declare id = \"dmls\""));
    }

    #[test]
    fn healthy_registration_keeps_old_log_error_historical() {
        let temp = TempDir::new().unwrap();
        let paths = paths(temp.path());
        let registration = paths.zed_data_dir.join("extensions/installed/dmls");
        fs::create_dir_all(&registration).unwrap();
        fs::write(registration.join("extension.toml"), "id = \"dmls\"\n").unwrap();
        fs::write(
            &paths.zed_log,
            "No extension manifest found for extension dmls\n",
        )
        .unwrap();
        let report = doctor(
            &paths,
            &FakeProbe {
                path: Some(PathBuf::from("/bin/dmls")),
                version: Ok(format!("dmls {EXPECTED_DMLS_VERSION}")),
            },
        );
        assert!(report.healthy);
        let output = render_lines(&report.lines, true);
        assert!(output.contains("version-compatible"));
        assert!(output.contains("freshness is unverified"));
        assert!(output.contains("historical context"));
    }

    #[test]
    fn incompatible_binary_is_a_failure() {
        let temp = TempDir::new().unwrap();
        let paths = paths(temp.path());
        let registration = paths.zed_data_dir.join("extensions/installed/dmls");
        fs::create_dir_all(&registration).unwrap();
        fs::write(registration.join("extension.toml"), "id = \"dmls\"\n").unwrap();
        let report = doctor(
            &paths,
            &FakeProbe {
                path: Some(PathBuf::from("dmls")),
                version: Ok("dmls 99.0.0".to_string()),
            },
        );
        assert!(!report.healthy);
        assert!(render_lines(&report.lines, true).contains("version-incompatible"));
    }

    #[test]
    fn binary_version_output_must_match_exactly() {
        let temp = TempDir::new().unwrap();
        let paths = paths(temp.path());
        let registration = paths.zed_data_dir.join("extensions/installed/dmls");
        fs::create_dir_all(&registration).unwrap();
        fs::write(registration.join("extension.toml"), "id = \"dmls\"\n").unwrap();
        let report = doctor(
            &paths,
            &FakeProbe {
                path: Some(PathBuf::from("dmls")),
                version: Ok(format!("unexpected wrapper output dmls {EXPECTED_DMLS_VERSION}")),
            },
        );
        assert!(!report.healthy);
        assert!(render_lines(&report.lines, true).contains("version-incompatible"));
    }

    #[test]
    fn compiled_dmls_version_matches_dmls_package_metadata() {
        let manifest: toml::Value = toml::from_str(include_str!("../../Cargo.toml")).unwrap();
        let package_version = manifest["package"]["version"].as_str().unwrap();

        assert_eq!(EXPECTED_DMLS_VERSION, package_version);
    }

    #[test]
    fn bounded_log_tail_tolerates_a_split_utf8_codepoint() {
        let temp = TempDir::new().unwrap();
        let log = temp.path().join("Zed.log");
        let mut contents = vec![b'x'; LOG_TAIL_BYTES as usize - 1];
        contents.extend_from_slice("é\nNo extension manifest found for extension dmls\n".as_bytes());
        fs::write(&log, contents).unwrap();

        let tail = read_log_tail(&log).unwrap();

        assert_eq!(newest_manifest_error_id(&tail).as_deref(), Some("dmls"));
    }
}
