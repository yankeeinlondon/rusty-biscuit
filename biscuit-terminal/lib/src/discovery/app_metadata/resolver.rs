//! Host-aware config-file resolution over the seed metadata.
//!
//! The public entry points are methods on [`TerminalApp`]. Resolution answers
//! "what config is *in use* on this host": an env override wins, else the first
//! existing candidate for the current OS target, else `None`. This is
//! existence-checked by design — contrast the back-compat
//! `get_terminal_config_path` (in `config_paths`), which returns the *default*
//! path without checking the filesystem.

use std::path::{Path, PathBuf};

use super::expand::ExpansionCtx;
use super::os_target::{ConfigOsTarget, current_config_os_target};
use super::seed::metadata_for;
use super::types::{
    ConfigCandidateKind, ConfigEnvKind, ConfigFormat, ConfigMetadata, ConfigSource,
    ResolvedConfig, TerminalAppMetadata,
};
use crate::discovery::detection::TerminalApp;

impl TerminalApp {
    /// This app's static metadata, or `None` for uncovered variants.
    pub fn metadata(&self) -> Option<&'static TerminalAppMetadata> {
        metadata_for(self)
    }

    /// This app's config-location + content metadata, or `None`.
    pub fn config_metadata(&self) -> Option<&'static ConfigMetadata> {
        self.metadata().map(|meta| &meta.config)
    }

    /// The config file this host is actually using, or `None`.
    ///
    /// A thin projection of [`get_config_file_resolved`](Self::get_config_file_resolved).
    pub fn get_config_file(&self) -> Option<PathBuf> {
        self.get_config_file_resolved().map(|resolved| resolved.path)
    }

    /// As [`get_config_file`](Self::get_config_file), but also reports why the
    /// path resolved (which env override, or which candidate index matched).
    pub fn get_config_file_resolved(&self) -> Option<ResolvedConfig> {
        let meta = self.config_metadata()?;
        let ctx = ExpansionCtx::from_process(current_config_os_target());
        resolve_config(meta, &ctx)
    }

    /// The default config-file candidate paths for this app on the current OS
    /// target, expanded but not existence-checked.
    pub fn config_candidate_paths(&self) -> Vec<PathBuf> {
        default_config_paths(self)
    }
}

/// Resolve the in-use config file for `meta` against the expansion context.
///
/// Env overrides are tried first (in priority order), then the OS target's
/// candidate list; override values are literal host paths, while candidate
/// templates receive token expansion. The first path that exists wins. Returns
/// `None` if nothing on disk matches.
pub(crate) fn resolve_config(
    meta: &'static ConfigMetadata,
    ctx: &ExpansionCtx,
) -> Option<ResolvedConfig> {
    for env in meta.location_env {
        let Some(value) = ctx.env_value(env.var) else {
            continue;
        };
        let base = PathBuf::from(value);
        let path = match env.kind {
            ConfigEnvKind::File => base,
            ConfigEnvKind::Dir => match config_path_under_env(meta, ctx.os_target, env.var) {
                Some(relative_path) => base.join(relative_path),
                None => continue,
            },
        };
        if config_path_matches(&path, ConfigCandidateKind::File, meta.format) {
            return Some(ResolvedConfig {
                path,
                format: meta.format,
                source: ConfigSource::EnvVar(env.var),
            });
        }
    }

    for (idx, candidate) in meta.locations.for_target(ctx.os_target).iter().enumerate() {
        for path in ctx.expand_candidate(candidate.template) {
            let format = candidate.format.unwrap_or(meta.format);
            if config_path_matches(&path, candidate.kind, format) {
                return Some(ResolvedConfig {
                    path,
                    format,
                    source: ConfigSource::Candidate(idx),
                });
            }
        }
    }

    None
}

fn config_path_matches(path: &Path, kind: ConfigCandidateKind, format: ConfigFormat) -> bool {
    match (kind, format) {
        (_, ConfigFormat::None) => path.exists(),
        (ConfigCandidateKind::File, _) => path.is_file(),
        (ConfigCandidateKind::Directory, _) => path.is_dir(),
    }
}

/// The *default* candidate paths for `app` on the current OS target, expanded
/// but **not** existence-checked.
///
/// This backs the legacy `get_terminal_config_paths` wrapper: it preserves the
/// old "return the default path(s), file may or may not exist" contract, unlike
/// [`TerminalApp::get_config_file`] which resolves what is actually in use.
pub(crate) fn default_config_paths(app: &TerminalApp) -> Vec<PathBuf> {
    let Some(meta) = app.config_metadata() else {
        return Vec::new();
    };
    let ctx = ExpansionCtx::from_process(current_config_os_target());
    meta.locations
        .for_target(ctx.os_target)
        .iter()
        .flat_map(|candidate| ctx.expand_candidate(candidate.template))
        .collect()
}

/// The canonical config filename for joining onto a `Dir`-kind env override.
///
/// Derived from the last path segment of the first candidate for `target` (e.g.
/// `kitty.conf` from `$XDG_CONFIG_HOME/kitty/kitty.conf`). `None` when the target
/// has no candidates.
fn config_path_under_env(
    meta: &ConfigMetadata,
    target: ConfigOsTarget,
    env_var: &str,
) -> Option<PathBuf> {
    let first = meta.locations.for_target(target).first()?;
    let token = format!("${env_var}/");
    if let Some(relative) = first.template.strip_prefix(&token) {
        return Some(PathBuf::from(relative));
    }
    canonical_filename(meta, target).map(PathBuf::from)
}

fn canonical_filename(meta: &ConfigMetadata, target: ConfigOsTarget) -> Option<&'static str> {
    let first = meta.locations.for_target(target).first()?;
    first.template.rsplit(['/', '\\']).next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::expand::EnvSource;
    use std::collections::HashMap;
    use std::fs;

    fn ctx(os_target: ConfigOsTarget, pairs: &[(&str, &str)]) -> ExpansionCtx {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        ExpansionCtx {
            os_target,
            env: EnvSource::Fixture(map),
        }
    }

    #[test]
    fn env_override_wins_over_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();

        // Both an override target and a candidate exist; the override must win.
        let override_dir = home.join("custom-kitty");
        fs::create_dir_all(&override_dir).unwrap();
        fs::write(override_dir.join("kitty.conf"), "font_size 12").unwrap();

        let xdg = home.join(".config");
        fs::create_dir_all(xdg.join("kitty")).unwrap();
        fs::write(xdg.join("kitty").join("kitty.conf"), "font_size 10").unwrap();

        let ctx = ctx(
            ConfigOsTarget::Linux,
            &[
                ("HOME", home.to_str().unwrap()),
                ("KITTY_CONFIG_DIRECTORY", override_dir.to_str().unwrap()),
            ],
        );
        let meta = &super::super::seed::metadata_for(&TerminalApp::Kitty)
            .unwrap()
            .config;
        let resolved = resolve_config(meta, &ctx).unwrap();

        assert_eq!(resolved.path, override_dir.join("kitty.conf"));
        assert_eq!(
            resolved.source,
            ConfigSource::EnvVar("KITTY_CONFIG_DIRECTORY")
        );
    }

    #[test]
    fn first_existing_candidate_wins() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let xdg = home.join(".config");
        fs::create_dir_all(xdg.join("kitty")).unwrap();
        let conf = xdg.join("kitty").join("kitty.conf");
        fs::write(&conf, "font_size 10").unwrap();

        let ctx = ctx(ConfigOsTarget::Linux, &[("HOME", home.to_str().unwrap())]);
        let meta = &super::super::seed::metadata_for(&TerminalApp::Kitty)
            .unwrap()
            .config;
        let resolved = resolve_config(meta, &ctx).unwrap();

        assert_eq!(resolved.path, conf);
        assert_eq!(resolved.source, ConfigSource::Candidate(0));
    }

    #[test]
    fn parseable_candidate_directory_does_not_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let xdg = home.join(".config");
        fs::create_dir_all(xdg.join("kitty").join("kitty.conf")).unwrap();

        let ctx = ctx(ConfigOsTarget::Linux, &[("HOME", home.to_str().unwrap())]);
        let meta = &super::super::seed::metadata_for(&TerminalApp::Kitty)
            .unwrap()
            .config;

        assert!(resolve_config(meta, &ctx).is_none());
    }

    #[test]
    fn warp_directory_candidate_resolves_as_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let warp_dir = dir.path().join(".warp");
        fs::create_dir_all(&warp_dir).unwrap();

        let ctx = ctx(
            ConfigOsTarget::Linux,
            &[("HOME", dir.path().to_str().unwrap())],
        );
        let meta = &super::super::seed::metadata_for(&TerminalApp::Warp)
            .unwrap()
            .config;
        let resolved = resolve_config(meta, &ctx).unwrap();

        assert_eq!(resolved.path, warp_dir);
        assert_eq!(resolved.format, ConfigFormat::Yaml);
        assert_eq!(resolved.source, ConfigSource::Candidate(0));
    }

    #[test]
    fn xdg_config_home_override_uses_candidate_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let config_home = dir.path().join("xdg");
        let alacritty_dir = config_home.join("alacritty");
        fs::create_dir_all(&alacritty_dir).unwrap();
        let conf = alacritty_dir.join("alacritty.toml");
        fs::write(&conf, "[font]\nsize = 12\n").unwrap();

        let stray = config_home.join("alacritty.toml");
        fs::write(&stray, "[font]\nsize = 10\n").unwrap();

        let ctx = ctx(
            ConfigOsTarget::Linux,
            &[("XDG_CONFIG_HOME", config_home.to_str().unwrap())],
        );
        let meta = &super::super::seed::metadata_for(&TerminalApp::Alacritty)
            .unwrap()
            .config;
        let resolved = resolve_config(meta, &ctx).unwrap();

        assert_eq!(resolved.path, conf);
        assert_eq!(resolved.source, ConfigSource::EnvVar("XDG_CONFIG_HOME"));
    }

    #[test]
    fn no_file_resolves_to_none() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ctx(
            ConfigOsTarget::Linux,
            &[("HOME", dir.path().to_str().unwrap())],
        );
        let meta = &super::super::seed::metadata_for(&TerminalApp::Kitty)
            .unwrap()
            .config;
        assert!(resolve_config(meta, &ctx).is_none());
    }

    #[test]
    fn wsl_win_home_candidate_resolves_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let win_home = dir.path();
        let settings = win_home
            .join("AppData")
            .join("Local")
            .join("Packages")
            .join("Microsoft.WindowsTerminal_abc")
            .join("LocalState");
        fs::create_dir_all(&settings).unwrap();
        fs::write(settings.join("settings.json"), "{}").unwrap();

        let ctx = ctx(
            ConfigOsTarget::Wsl2,
            &[("WIN_HOME", win_home.to_str().unwrap())],
        );
        let meta = &super::super::seed::metadata_for(&TerminalApp::WindowsTerminal)
            .unwrap()
            .config;
        let resolved = resolve_config(meta, &ctx).unwrap();

        assert_eq!(resolved.path, settings.join("settings.json"));
        assert_eq!(resolved.source, ConfigSource::Candidate(0));
    }
}
