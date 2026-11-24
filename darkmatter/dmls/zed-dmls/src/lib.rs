//! Thin Zed extension that launches the native `dmls` binary.
//!
//! Contains no language logic: it resolves the binary (settings override → PATH
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
        // 1. Settings override (`binary.path`). An explicit choice outranks PATH,
        //    and it is the only step that works for single-file worktrees: Zed
        //    loads no shell environment for those, so `which` sees a bare PATH.
        if let Ok(settings) = LspSettings::for_worktree("dmls", worktree) {
            if let Some(binary) = settings.binary {
                if let Some(path) = binary.path {
                    // User settings follow the client to remote projects, so a
                    // home-relative path is the only spelling that is right on
                    // every host. Zed spawns the command without a shell, so
                    // the expansion happens here, against the worktree host's
                    // environment rather than the client's.
                    let env = worktree.shell_env();
                    let home = env
                        .iter()
                        .find(|(key, _)| key == "HOME")
                        .or_else(|| env.iter().find(|(key, _)| key == "USERPROFILE"))
                        .map(|(_, value)| value.as_str());
                    return Ok(expand_home(&path, home));
                }
            }
        }

        // 2. PATH.
        if let Some(path) = worktree.which("dmls") {
            return Ok(path);
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
        )
        .map_err(unresolved_binary_error)?;

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

/// Message for the terminal failure: no local binary, and no published release
/// to fall back on.
///
/// The release lookup is the last resort, so its transport error (typically a
/// bare `404` while `REPO` names an unpublished repository) is the text Zed
/// shows the user. Lead with the two steps that actually resolve the binary.
fn unresolved_binary_error(release_error: String) -> String {
    format!(
        "could not resolve the `dmls` binary. It is not on this worktree's PATH \
         (single-file buffers carry no shell environment) and \
         `lsp.dmls.binary.path` is not set in Zed's settings. Install it with \
         `just install-dmls`, or set `lsp.dmls.binary.path` to its absolute \
         path. Downloading a release from `{REPO}` also failed: {release_error}"
    )
}

/// Expands a leading `~`, `$HOME`, or `${HOME}` in a configured binary path.
///
/// Only a prefix followed by the end of the string or a path separator is
/// expanded (`~user` and `$HOMEDIR` are left alone). With no known home the
/// path is returned unchanged, so the spawn error names what was configured.
fn expand_home(path: &str, home: Option<&str>) -> String {
    let Some(home) = home.filter(|home| !home.is_empty()) else {
        return path.to_string();
    };
    for prefix in ["${HOME}", "$HOME", "~"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            if rest.is_empty() || rest.starts_with('/') || rest.starts_with('\\') {
                return format!("{}{rest}", home.trim_end_matches(['/', '\\']));
            }
        }
    }
    path.to_string()
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

#[cfg(test)]
mod tests {
    use super::expand_home;

    #[test]
    fn expands_each_home_spelling() {
        for path in ["~/.cargo/bin/dmls", "$HOME/.cargo/bin/dmls", "${HOME}/.cargo/bin/dmls"] {
            assert_eq!(expand_home(path, Some("/home/build")), "/home/build/.cargo/bin/dmls");
        }
        assert_eq!(expand_home("~", Some("/home/build/")), "/home/build");
    }

    #[test]
    fn leaves_other_paths_unchanged() {
        let home = Some("/home/build");
        assert_eq!(expand_home("/usr/bin/dmls", home), "/usr/bin/dmls");
        assert_eq!(expand_home("~other/dmls", home), "~other/dmls");
        assert_eq!(expand_home("$HOMEDIR/dmls", home), "$HOMEDIR/dmls");
        assert_eq!(expand_home("~/dmls", None), "~/dmls");
    }
}
