//! Rush layout classification and comparison (ruling R3).
//!
//! `rush.json` selects the package manager and so the lockfile. Only the
//! ordinary single pnpm workspace layout is compared: pnpm with
//! `useWorkspaces`, no subspaces, and no installation variants. Its importer
//! keys are relative to `common/temp`, not to the lockfile's directory. Every
//! other layout is `unverifiable` + `unsupported_layout`.
//!
//! `rush.json` is already cached by the Rush detector, so the manager costs
//! no read. `pnpm-config.json` and `subspaces.json` are read only when the
//! request corroborates, and each read counts as a manifest parse.

use std::path::Path;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use tracing::debug;

use super::super::detection::{ManifestStore, probe_exists, probe_is_dir, read_counted_config};
use super::super::jsonc;
use super::super::npm::{RushJson, RushManager};
use super::super::seed::PackageSeed;
use super::super::standard::MonorepoLayer;
use super::membership::normalize_member;
use super::sources::Format;
use super::{LockfileObservation, LockfilePresence, LockfileReason, ParsedLockfile, classify};

/// The directory pnpm importer keys resolve against, relative to the root.
const PNPM_IMPORTER_BASE: [&str; 2] = ["common", "temp"];

/// The lockfile each Rush package manager writes, relative to the root.
fn manager_lockfile(manager: RushManager) -> &'static str {
    match manager {
        RushManager::Pnpm => "common/config/rush/pnpm-lock.yaml",
        RushManager::Npm => "common/config/rush/npm-shrinkwrap.json",
        RushManager::Yarn => "common/config/rush/yarn.lock",
    }
}

/// Observe a Rush layer's lockfile.
///
/// A metadata failure on the manager's lockfile is `unreadable`. An npm or
/// Yarn manager, or a `rush.json` naming no single manager, is never compared.
/// With corroboration declined, a present pnpm lockfile is `not_requested` and
/// an absent one `absent`, because the configuration that could make it an
/// unsupported layout is not read.
pub(super) fn observe(
    layer: &MonorepoLayer,
    owned: Option<&[PackageSeed]>,
    wants: bool,
    store: &ManifestStore,
) -> LockfileObservation {
    let root = layer.root.as_path();
    let config = store.rush_json(&root.join("rush.json")).ok().flatten();
    let Some(config) = config else {
        return LockfileObservation::unverifiable(Vec::new(), LockfileReason::UnsupportedLayout);
    };
    let Some(manager) = config.manager() else {
        debug!(root = %root.display(), "rush.json names no single package manager");
        return LockfileObservation::unverifiable(Vec::new(), LockfileReason::UnsupportedLayout);
    };

    let lockfile = manager_lockfile(manager);
    let path = root.join(lockfile);
    let present = match store.lockfile_presence(&path) {
        LockfilePresence::Failed(kind) => {
            debug!(path = %path.display(), ?kind, "lockfile metadata probe failed");
            return LockfileObservation::unreadable(Vec::new(), LockfileReason::MetadataFailed);
        }
        presence => presence.is_present(),
    };
    let paths = if present {
        vec![lockfile.to_owned()]
    } else {
        Vec::new()
    };

    if manager != RushManager::Pnpm {
        return if present && !wants {
            LockfileObservation::not_requested(paths)
        } else {
            LockfileObservation::unverifiable(paths, LockfileReason::UnsupportedLayout)
        };
    }
    if !wants {
        return if present {
            LockfileObservation::not_requested(paths)
        } else {
            LockfileObservation::absent()
        };
    }
    if let Some(why) = unsupported_pnpm_layout(root, &config) {
        debug!(root = %root.display(), why, "unsupported Rush layout");
        return LockfileObservation::unverifiable(paths, LockfileReason::UnsupportedLayout);
    }
    if !present {
        return LockfileObservation::absent();
    }

    let parsed = match store.lockfile(&path, Format::Pnpm) {
        Ok(parsed) => parsed,
        Err(failure) => {
            debug!(path = %path.display(), ?failure, "lockfile could not be corroborated");
            return failure.observation(paths);
        }
    };
    let parsed = without_synthetic_project(&parsed);
    classify(&parsed, &PNPM_IMPORTER_BASE, layer, owned, store, paths)
}

/// Drop the `.` importer: it is Rush's synthetic `common/temp` project, not
/// the repository root, and would otherwise read as a member `common/temp`.
fn without_synthetic_project(parsed: &ParsedLockfile) -> ParsedLockfile {
    match parsed {
        ParsedLockfile::Members(keys) => ParsedLockfile::Members(
            keys.iter()
                .filter(|key| normalize_member(key, &[]).is_ok_and(|key| !key.is_empty()))
                .cloned()
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Why the pnpm install is not the ordinary single-workspace layout, if it
/// is not. Configuration that cannot be read or parsed cannot be classified,
/// so it counts as unsupported.
fn unsupported_pnpm_layout(root: &Path, config: &RushJson) -> Option<&'static str> {
    if config.declares_variants() {
        return Some("rush.json declares installation variants");
    }
    if probe_is_dir(&root.join("common/config/rush/variants")) {
        return Some("an installation variants directory exists");
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Subspaces {
        subspaces_enabled: Option<bool>,
    }
    match read_config::<Subspaces>(&root.join("common/config/rush/subspaces.json")) {
        Config::Absent => {}
        Config::Failed => return Some("subspaces.json cannot be read"),
        Config::Parsed(subspaces) => {
            if subspaces.subspaces_enabled == Some(true) {
                return Some("subspaces are enabled");
            }
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PnpmConfig {
        use_workspaces: Option<bool>,
    }
    // Rush's default is the legacy install, which writes no workspace
    // importers.
    let use_workspaces =
        match read_config::<PnpmConfig>(&root.join("common/config/rush/pnpm-config.json")) {
            Config::Absent => config.legacy_use_workspaces(),
            Config::Failed => return Some("pnpm-config.json cannot be read"),
            Config::Parsed(pnpm) => pnpm.use_workspaces,
        };
    (use_workspaces != Some(true)).then_some("pnpm workspaces are not enabled")
}

/// One optional Rush configuration file.
enum Config<T> {
    Absent,
    Parsed(T),
    Failed,
}

fn read_config<T: DeserializeOwned>(path: &Path) -> Config<T> {
    if !probe_exists(path) {
        return Config::Absent;
    }
    let parsed = read_counted_config(path)
        .map_err(|error| error.to_string())
        .and_then(|content| jsonc::from_str::<T>(&content));
    match parsed {
        Ok(parsed) => Config::Parsed(parsed),
        Err(message) => {
            debug!(path = %path.display(), %message, "Rush configuration is unreadable");
            Config::Failed
        }
    }
}
