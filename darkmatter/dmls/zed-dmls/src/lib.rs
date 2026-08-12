//! Thin Zed extension that launches the native `dmls` binary.
//!
//! Contains no language logic: it resolves the binary (PATH → settings override
//! → GitHub release download, cached by version) and returns the command Zed
//! runs. All Markdown/Darkmatter intelligence lives in `dmls`.

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

/// GitHub `owner/repo` that publishes `dmls` release archives.
///
/// Update this to the repository that ships the per-platform assets described
/// in the README before publishing the extension.
const REPO: &str = "ken/darkmatter-dmls";

struct DmlsExtension {
    /// Cached path to a downloaded binary for the current version, so an upgrade
    /// downloads once rather than on every server start.
    cached_binary_path: Option<String>,
}

impl DmlsExtension {
    fn binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // 1. PATH.
        if let Some(path) = worktree.which("dmls") {
            return Ok(path);
        }

        // 2. Settings override (`binary.path`).
        if let Ok(settings) = LspSettings::for_worktree("dmls", worktree) {
            if let Some(binary) = settings.binary {
                if let Some(path) = binary.path {
                    return Ok(path);
                }
            }
        }

        // 3. GitHub release download, cached by version.
        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).map(|m| m.is_file()).unwrap_or(false) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = asset_name(&release.version, platform, arch);
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no release asset named `{asset_name}`"))?;

        let version_dir = format!("dmls-{}", release.version);
        let binary_path = format!("{version_dir}/{}", binary_file_name(platform));

        if !std::fs::metadata(&binary_path).map(|m| m.is_file()).unwrap_or(false) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let file_kind = match platform {
                zed::Os::Windows => zed::DownloadedFileType::Zip,
                _ => zed::DownloadedFileType::GzipTar,
            };
            zed::download_file(&asset.download_url, &version_dir, file_kind)
                .map_err(|e| format!("failed to download `{asset_name}`: {e}"))?;
            zed::make_file_executable(&binary_path)?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

/// Release asset name for a platform, matching the `just dist` recipe naming.
fn asset_name(version: &str, platform: zed::Os, arch: zed::Architecture) -> String {
    match (platform, arch) {
        (zed::Os::Mac, _) => format!("dmls-{version}-macos-universal.tar.gz"),
        (zed::Os::Linux, zed::Architecture::Aarch64) => {
            format!("dmls-{version}-linux-aarch64.tar.gz")
        }
        (zed::Os::Linux, _) => format!("dmls-{version}-linux-x86_64.tar.gz"),
        (zed::Os::Windows, _) => format!("dmls-{version}-windows-x86_64.zip"),
    }
}

fn binary_file_name(platform: zed::Os) -> &'static str {
    match platform {
        zed::Os::Windows => "dmls.exe",
        _ => "dmls",
    }
}

impl zed::Extension for DmlsExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let path = self.binary_path(language_server_id, worktree)?;
        let args = LspSettings::for_worktree("dmls", worktree)
            .ok()
            .and_then(|settings| settings.binary)
            .and_then(|binary| binary.arguments)
            .unwrap_or_default();

        Ok(zed::Command {
            command: path,
            args,
            env: Default::default(),
        })
    }
}

zed::register_extension!(DmlsExtension);
