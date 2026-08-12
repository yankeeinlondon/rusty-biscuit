//! Portable path-template expansion for config-file candidates.
//!
//! Templates carry portable tokens (`~`, `$HOME`, `$XDG_CONFIG_HOME`, `$APPDATA`,
//! `$LOCALAPPDATA`, `$USER`, `$WIN_HOME`) plus an optional single `*` wildcard
//! segment for the Windows Terminal package glob. Expansion is driven by an
//! [`ExpansionCtx`], which pairs the target OS with an [`EnvSource`]. The env
//! source is injectable so the later resolver can be unit-tested against a
//! fabricated Windows / WSL environment without running on such a host.
//!
//! The expansion machinery is consumed by the config resolver ([`super::resolver`]).

use std::collections::HashMap;
use std::path::PathBuf;

use super::os_target::ConfigOsTarget;

/// Where environment-variable lookups come from during expansion.
///
/// [`EnvSource::Process`] reads the live process environment; [`EnvSource::Fixture`]
/// reads a fabricated map (tests). Empty values are treated as unset in both.
pub(crate) enum EnvSource {
    /// Read variables from the live process environment.
    Process,
    /// Read variables from a fabricated map (constructed only in tests).
    #[cfg_attr(not(test), allow(dead_code))]
    Fixture(HashMap<String, String>),
}

impl EnvSource {
    /// Look up a variable, treating empty values as unset.
    fn get(&self, key: &str) -> Option<String> {
        let raw = match self {
            EnvSource::Process => std::env::var(key).ok(),
            EnvSource::Fixture(map) => map.get(key).cloned(),
        };
        raw.filter(|value| !value.is_empty())
    }
}

/// The inputs a template needs to expand: the target OS and an env source.
pub(crate) struct ExpansionCtx {
    pub os_target: ConfigOsTarget,
    pub env: EnvSource,
}

impl ExpansionCtx {
    /// An expansion context bound to the live process environment.
    pub(crate) fn from_process(os_target: ConfigOsTarget) -> Self {
        Self {
            os_target,
            env: EnvSource::Process,
        }
    }

    /// Look up an environment variable through this context's env source.
    pub(crate) fn env_value(&self, key: &str) -> Option<String> {
        self.env.get(key)
    }

    /// The user's home directory: `$HOME`, else `$USERPROFILE`, else `$HOMEDRIVE$HOMEPATH`.
    fn home(&self) -> Option<PathBuf> {
        if let Some(home) = self.env.get("HOME") {
            return Some(PathBuf::from(home));
        }
        if let Some(profile) = self.env.get("USERPROFILE") {
            return Some(PathBuf::from(profile));
        }
        match (self.env.get("HOMEDRIVE"), self.env.get("HOMEPATH")) {
            (Some(drive), Some(path)) => Some(PathBuf::from(format!("{drive}{path}"))),
            _ => None,
        }
    }

    /// The Windows user profile reachable from inside WSL.
    ///
    /// Prefers an explicit `$WIN_HOME` override; otherwise best-effort
    /// `/mnt/c/Users/$USER`. `None` (candidate dropped) when neither resolves.
    fn win_home(&self) -> Option<PathBuf> {
        if let Some(win_home) = self.env.get("WIN_HOME") {
            return Some(PathBuf::from(win_home));
        }
        let user = self.env.get("USER")?;
        Some(PathBuf::from(format!("/mnt/c/Users/{user}")))
    }

    /// Expand a single `$TOKEN` name to a path fragment, or `None` to drop the candidate.
    fn token(&self, name: &str) -> Option<PathBuf> {
        match name {
            "HOME" => self.home(),
            // XDG has a fallback to ~/.config, mirroring config_paths::config_dir.
            "XDG_CONFIG_HOME" => self
                .env
                .get("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| self.home().map(|home| home.join(".config"))),
            "APPDATA" => self.env.get("APPDATA").map(PathBuf::from),
            "LOCALAPPDATA" => self.env.get("LOCALAPPDATA").map(PathBuf::from),
            "USER" => self.env.get("USER").map(PathBuf::from),
            "WIN_HOME" => self.win_home(),
            // Unknown token → drop the candidate.
            _ => None,
        }
    }

    /// Expand one path component, or `None` if a referenced token is unresolvable.
    fn component(&self, comp: &str, is_first: bool) -> Option<PathBuf> {
        if is_first && comp == "~" {
            return self.home();
        }
        if let Some(token) = comp.strip_prefix('$') {
            return self.token(token);
        }
        Some(PathBuf::from(comp))
    }

    /// Expand a token-bearing template into a single path, or `None` if any token drops it.
    fn expand_path(&self, template: &str) -> Option<PathBuf> {
        let mut result = PathBuf::new();
        for (idx, comp) in split_template(template).enumerate() {
            if comp.is_empty() {
                // A leading empty component means the template began with a
                // separator (a Unix-absolute path like `/mnt/c/...`).
                if idx == 0 {
                    result.push("/");
                }
                continue;
            }
            result.push(self.component(comp, idx == 0)?);
        }
        Some(result)
    }

    /// Expand a template containing a single `*` segment against the filesystem.
    ///
    /// Returns every matching path in sorted (deterministic) order — first match
    /// wins downstream. An empty vector means the base directory is missing or no
    /// entry matched.
    fn expand_glob(&self, template: &str) -> Vec<PathBuf> {
        let comps: Vec<&str> = split_template(template).collect();
        let Some(glob_idx) = comps.iter().position(|comp| comp.contains('*')) else {
            return self.expand_path(template).into_iter().collect();
        };

        // Expand the fixed prefix (everything before the wildcard) into a base dir.
        let mut base = PathBuf::new();
        for (idx, comp) in comps[..glob_idx].iter().enumerate() {
            if comp.is_empty() {
                if idx == 0 {
                    base.push("/");
                }
                continue;
            }
            let Some(piece) = self.component(comp, idx == 0) else {
                return Vec::new();
            };
            base.push(piece);
        }

        let pattern = comps[glob_idx];
        let suffix = &comps[glob_idx + 1..];

        let Ok(entries) = std::fs::read_dir(&base) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| wildcard_matches(pattern, name))
            .collect();
        names.sort();

        names
            .into_iter()
            .map(|name| {
                let mut path = base.join(name);
                for seg in suffix {
                    path.push(seg);
                }
                path
            })
            .collect()
    }

    /// Expand a candidate template into zero or more concrete paths.
    ///
    /// A non-glob template yields at most one path (dropped if a token is
    /// unresolvable); a glob template yields every filesystem match in sorted order.
    pub(crate) fn expand_candidate(&self, template: &str) -> Vec<PathBuf> {
        if template.contains('*') {
            self.expand_glob(template)
        } else {
            self.expand_path(template).into_iter().collect()
        }
    }
}

/// Split a template on both `/` and `\` so Windows-style templates expand portably.
fn split_template(template: &str) -> impl Iterator<Item = &str> {
    template.split(['/', '\\'])
}

/// Match a name against a pattern containing at most one `*` wildcard.
fn wildcard_matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => {
            name.len() >= prefix.len() + suffix.len()
                && name.starts_with(prefix)
                && name.ends_with(suffix)
        }
        None => pattern == name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(pairs: &[(&str, &str)]) -> ExpansionCtx {
        let map = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        ExpansionCtx {
            os_target: ConfigOsTarget::Linux,
            env: EnvSource::Fixture(map),
        }
    }

    #[test]
    fn tilde_expands_to_home() {
        let ctx = ctx(&[("HOME", "/home/ken")]);
        assert_eq!(
            ctx.expand_candidate("~/.wezterm.lua"),
            vec![PathBuf::from("/home/ken/.wezterm.lua")]
        );
    }

    #[test]
    fn xdg_config_home_falls_back_to_dot_config() {
        let ctx = ctx(&[("HOME", "/home/ken")]);
        assert_eq!(
            ctx.expand_candidate("$XDG_CONFIG_HOME/kitty/kitty.conf"),
            vec![PathBuf::from("/home/ken/.config/kitty/kitty.conf")]
        );
    }

    #[test]
    fn xdg_config_home_honors_explicit_value() {
        let ctx = ctx(&[("HOME", "/home/ken"), ("XDG_CONFIG_HOME", "/xdg")]);
        assert_eq!(
            ctx.expand_candidate("$XDG_CONFIG_HOME/kitty/kitty.conf"),
            vec![PathBuf::from("/xdg/kitty/kitty.conf")]
        );
    }

    #[test]
    fn unset_appdata_drops_candidate() {
        let ctx = ctx(&[("HOME", "/home/ken")]);
        assert!(
            ctx.expand_candidate("$APPDATA/alacritty/alacritty.toml")
                .is_empty(),
            "an unresolved token must drop the candidate"
        );
    }

    #[test]
    fn empty_value_counts_as_unset() {
        let ctx = ctx(&[("HOME", "/home/ken"), ("APPDATA", "")]);
        assert!(
            ctx.expand_candidate("$APPDATA/alacritty/alacritty.toml")
                .is_empty()
        );
    }

    #[test]
    fn unknown_token_drops_candidate() {
        let ctx = ctx(&[("HOME", "/home/ken")]);
        assert!(ctx.expand_candidate("$NOPE/foo").is_empty());
    }

    #[test]
    fn user_token_expands_mid_path() {
        let ctx = ctx(&[("USER", "ken")]);
        assert_eq!(
            ctx.expand_candidate("/mnt/c/Users/$USER/AppData"),
            vec![PathBuf::from("/mnt/c/Users/ken/AppData")]
        );
    }

    #[test]
    fn win_home_defaults_to_mnt_c_users() {
        let ctx = ctx(&[("USER", "ken")]);
        assert_eq!(
            ctx.expand_candidate("$WIN_HOME/AppData/Local/settings.json"),
            vec![PathBuf::from("/mnt/c/Users/ken/AppData/Local/settings.json")]
        );
    }

    #[test]
    fn win_home_honors_explicit_override() {
        let ctx = ctx(&[("WIN_HOME", "/mnt/d/Profiles/ken")]);
        assert_eq!(
            ctx.expand_candidate("$WIN_HOME/x"),
            vec![PathBuf::from("/mnt/d/Profiles/ken/x")]
        );
    }

    #[test]
    fn win_home_drops_when_unresolvable() {
        let ctx = ctx(&[]);
        assert!(ctx.expand_candidate("$WIN_HOME/x").is_empty());
    }

    #[test]
    fn backslash_templates_split_portably() {
        let ctx = ctx(&[("APPDATA", "/roaming")]);
        assert_eq!(
            ctx.expand_candidate("$APPDATA\\alacritty\\alacritty.toml"),
            vec![PathBuf::from("/roaming/alacritty/alacritty.toml")]
        );
    }

    #[test]
    fn wildcard_matcher_respects_prefix_and_suffix() {
        assert!(wildcard_matches(
            "Microsoft.WindowsTerminal_*",
            "Microsoft.WindowsTerminal_8wekyb3d8bbwe"
        ));
        assert!(!wildcard_matches(
            "Microsoft.WindowsTerminal_*",
            "Microsoft.WindowsTerminalPreview_x"
        ));
        assert!(wildcard_matches("a*z", "az"));
        assert!(!wildcard_matches("a*z", "a"));
    }

    #[test]
    fn windows_terminal_glob_expands_sorted_with_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let packages = dir.path().join("Packages");
        // Create out of order to prove the result is sorted, not fs-order.
        for pkg in [
            "Microsoft.WindowsTerminal_bbb",
            "Microsoft.WindowsTerminal_aaa",
        ] {
            std::fs::create_dir_all(packages.join(pkg).join("LocalState")).unwrap();
        }
        // A non-matching sibling must be ignored.
        std::fs::create_dir_all(packages.join("Microsoft.Other_x").join("LocalState")).unwrap();

        let ctx = ctx(&[("LOCALAPPDATA", dir.path().to_str().unwrap())]);
        let resolved = ctx.expand_candidate(
            "$LOCALAPPDATA\\Packages\\Microsoft.WindowsTerminal_*\\LocalState\\settings.json",
        );

        assert_eq!(
            resolved,
            vec![
                packages
                    .join("Microsoft.WindowsTerminal_aaa")
                    .join("LocalState")
                    .join("settings.json"),
                packages
                    .join("Microsoft.WindowsTerminal_bbb")
                    .join("LocalState")
                    .join("settings.json"),
            ]
        );
    }

    #[test]
    fn glob_returns_empty_when_base_missing() {
        let ctx = ctx(&[("LOCALAPPDATA", "/definitely/not/here")]);
        assert!(
            ctx.expand_candidate("$LOCALAPPDATA\\Packages\\Microsoft.WindowsTerminal_*\\settings.json")
                .is_empty()
        );
    }
}
