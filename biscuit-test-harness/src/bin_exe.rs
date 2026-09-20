//! Locating a workspace binary a test spawns, and the checkout it reads
//! fixtures from.
//!
//! `env!("CARGO_BIN_EXE_<name>")` is a compile-time constant holding an
//! absolute path inside the *build* host's target directory. That is wrong for
//! any run that did not build the binary itself: the `wsl2-ubuntu` CI leg
//! executes a `cargo nextest archive` built on `ubuntu-latest`, and nextest
//! extracts the archived binaries into a temp directory unrelated to the
//! build's `target/`, so the baked path fails with `NotFound`. Such a test is
//! green on every machine that builds it and red only in the guest.
//!
//! [`bin_exe!`](macro@crate::bin_exe) asks the environment instead, and falls
//! back to the compile-time value.
//!
//! `env!("CARGO_MANIFEST_DIR")` is the same trap for a *fixture*: it names the
//! build host's checkout, while nextest's `--workspace-remap` rewrites only the
//! run-time variable. [`manifest_dir!`](macro@crate::manifest_dir) is its
//! counterpart.

use std::ffi::OsString;
use std::path::PathBuf;

/// Absolute path to the workspace binary `$name`, resolved at run time.
///
/// `$name` is the binary's target name as Cargo spells it (hyphens and all).
/// The macro must expand in the crate whose Cargo target declares the
/// dependency on that binary, since that is where Cargo sets the compile-time
/// fallback.
///
/// ```ignore
/// let output = Command::new(bin_exe!("so-you-say")).arg("--help").output()?;
/// ```
#[macro_export]
macro_rules! bin_exe {
    ($name:literal) => {
        $crate::bin_exe::resolve($name, env!(concat!("CARGO_BIN_EXE_", $name)))
    };
}

/// Implementation of [`bin_exe!`](macro@crate::bin_exe); call the macro instead.
pub fn resolve(name: &str, compiled: &str) -> PathBuf {
    resolve_with(name, compiled, |var| std::env::var_os(var))
}

/// Resolution order: nextest's run-time republication of the binary's location
/// first, then Cargo's own variable, then the compile-time path.
///
/// Nextest exports `NEXTEST_BIN_EXE_<name>` with hyphens replaced by
/// underscores (shells and debuggers drop hyphenated names), and since 0.9.130
/// also `CARGO_BIN_EXE_<name>` verbatim; `cargo test` on Rust 1.94+ sets the
/// latter too. Preferring them over `compiled` is what makes an archived run
/// work, and costs nothing elsewhere — when the runner built the binary, all
/// three name the same file.
fn resolve_with(
    name: &str,
    compiled: &str,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> PathBuf {
    let candidates = [
        format!("NEXTEST_BIN_EXE_{}", name.replace('-', "_")),
        format!("CARGO_BIN_EXE_{name}"),
    ];

    candidates
        .iter()
        .filter_map(|var| lookup(var))
        .find(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from(compiled), PathBuf::from)
}

/// The directory of the calling crate's `Cargo.toml`, resolved at run time.
///
/// The same hazard as [`bin_exe!`](macro@crate::bin_exe), one level up:
/// `env!("CARGO_MANIFEST_DIR")` is the *build* host's checkout path, so a test
/// that reads a repository fixture through it looks for the producer's
/// directory when it runs from an archive somewhere else. Nextest's
/// `--workspace-remap` rewrites the run-time variable to the consumer's
/// checkout; this macro prefers it and keeps the compile-time value as the
/// fallback for a run that built its own binaries.
///
/// ```ignore
/// let fixture = manifest_dir!().join("tests/fixtures/sample.md");
/// ```
#[macro_export]
macro_rules! manifest_dir {
    () => {
        $crate::bin_exe::manifest_dir(env!("CARGO_MANIFEST_DIR"))
    };
}

/// Implementation of [`manifest_dir!`](macro@crate::manifest_dir); call the
/// macro instead.
pub fn manifest_dir(compiled: &str) -> PathBuf {
    manifest_dir_with(compiled, |var| std::env::var_os(var))
}

fn manifest_dir_with(compiled: &str, lookup: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    lookup("CARGO_MANIFEST_DIR")
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from(compiled), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(
        pairs: &'a [(&'static str, &'static str)],
    ) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |var| {
            pairs
                .iter()
                .find(|(key, _)| *key == var)
                .map(|(_, value)| OsString::from(*value))
        }
    }

    #[test]
    fn nextest_variable_mangles_hyphens_to_underscores() {
        let path = resolve_with(
            "so-you-say",
            "/build-host/target/debug/so-you-say",
            env_of(&[("NEXTEST_BIN_EXE_so_you_say", "/extracted/so-you-say")]),
        );

        assert_eq!(path, PathBuf::from("/extracted/so-you-say"));
    }

    #[test]
    fn cargo_variable_keeps_hyphens() {
        let path = resolve_with(
            "so-you-say",
            "/build-host/target/debug/so-you-say",
            env_of(&[("CARGO_BIN_EXE_so-you-say", "/extracted/so-you-say")]),
        );

        assert_eq!(path, PathBuf::from("/extracted/so-you-say"));
    }

    #[test]
    fn nextest_variable_wins_over_cargo() {
        let path = resolve_with(
            "md",
            "/build-host/target/debug/md",
            env_of(&[
                ("NEXTEST_BIN_EXE_md", "/extracted/md"),
                ("CARGO_BIN_EXE_md", "/stale/md"),
            ]),
        );

        assert_eq!(path, PathBuf::from("/extracted/md"));
    }

    #[test]
    fn a_remapped_workspace_moves_the_manifest_directory() {
        // What `--workspace-remap` does: the run-time variable names the
        // CONSUMER's checkout, and a fixture read through it is found there.
        let path = manifest_dir_with(
            "/build-host/rusty-biscuit/darkmatter/cli",
            env_of(&[("CARGO_MANIFEST_DIR", "/guest/checkout/darkmatter/cli")]),
        );

        assert_eq!(path, PathBuf::from("/guest/checkout/darkmatter/cli"));
    }

    #[test]
    fn the_compile_time_manifest_directory_is_the_fallback() {
        let compiled = "/build-host/rusty-biscuit/darkmatter/cli";
        assert_eq!(
            manifest_dir_with(compiled, env_of(&[])),
            PathBuf::from(compiled)
        );
        // An empty value is how a shell spells "unset"; it must not resolve to
        // the filesystem root.
        assert_eq!(
            manifest_dir_with(compiled, env_of(&[("CARGO_MANIFEST_DIR", "")])),
            PathBuf::from(compiled)
        );
    }

    #[test]
    fn compile_time_path_is_the_fallback() {
        let path = resolve_with("md", "/build-host/target/debug/md", env_of(&[]));

        assert_eq!(path, PathBuf::from("/build-host/target/debug/md"));
    }

    /// An exported-but-empty variable must not win: it would spawn `""`.
    #[test]
    fn empty_variable_is_ignored() {
        let path = resolve_with(
            "md",
            "/build-host/target/debug/md",
            env_of(&[("NEXTEST_BIN_EXE_md", ""), ("CARGO_BIN_EXE_md", "/extracted/md")]),
        );

        assert_eq!(path, PathBuf::from("/extracted/md"));
    }
}
