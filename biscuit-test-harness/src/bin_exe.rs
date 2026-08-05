//! Locating a workspace binary that a test spawns.
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
