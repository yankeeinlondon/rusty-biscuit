//! Contracts for the shared kache host-decision script
//! (`scripts/kache-host.sh`, fixes/2026-09-23-ensuring-kache-support) and the
//! kache version floor.
//!
//! The script is the one probe both `just init` (qualification verdict) and
//! `just kache-status` (report) call, so its machine-readable line format is
//! a real interface: `_ensure-kache` branches on the `kache-host: verdict=`
//! line. These tests pin that format, the exit-code agreement, the
//! env-passthrough probe's ability to discriminate a wrapper that strips
//! `DYLD_*` from one that forwards it, and the shape of the floor file the
//! justfile compares with `sort -V` and the maintenance audit reads verbatim.

use std::{
    fs,
    path::PathBuf,
    process::Command,
};

fn repo_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

/// The floor file must stay a bare `X.Y.Z` that `sort -V` and the audit's
/// whitespace-stripping read both handle, and must not fall back below the
/// 0.23.0 line the 2026-09-23 rulings raised it to (the precedence stack and
/// the daemon spike were measured on the 0.23 line).
#[test]
fn kache_floor_version_file_is_wellformed() {
    let path = repo_root().join(".github/kache-min-version");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let value = raw.trim();
    let parts: Vec<&str> = value.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "floor must be X.Y.Z so `sort -V` comparisons hold, got {value:?}"
    );
    for part in &parts {
        assert!(
            part.chars().all(|c| c.is_ascii_digit()) && !part.is_empty(),
            "floor components must be numeric, got {value:?}"
        );
    }
    let major: u32 = parts[0].parse().unwrap();
    let minor: u32 = parts[1].parse().unwrap();
    let patch: u32 = parts[2].parse().unwrap();
    assert!(
        (major, minor, patch) >= (0, 23, 0),
        "floor {value} is below the 0.23.0 line the 2026-09-23 spec ratified"
    );
}

#[cfg(unix)]
fn bash() -> Command {
    let mut command = Command::new("bash");
    command.current_dir(repo_root());
    command
}

#[cfg(unix)]
fn run_host_script(args: &[&str]) -> (String, Option<i32>) {
    let output = bash()
        .arg("scripts/kache-host.sh")
        .args(args)
        .env("WT", "")
        .output()
        .expect("running scripts/kache-host.sh must not fail to spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let code = output.status.code();
    (stdout, code)
}

/// The script must stay parseable bash under `set -euo pipefail` — every
/// recipe and later phase builds on it.
#[cfg(unix)]
#[test]
fn kache_host_script_parses() {
    let status = bash()
        .arg("-n")
        .arg("scripts/kache-host.sh")
        .status()
        .expect("bash -n must spawn");
    assert!(status.success(), "bash -n reported a syntax error");
}

/// `qualify` speaks the stable line format `_ensure-kache` parses: the first
/// stdout line is the verdict, the exit code agrees with it, and a qualifying
/// verdict names the candidate store a non-qualifying one cannot lack a
/// reason. The verdict itself is a property of the host's filesystem (this
/// suite runs on cloning and non-cloning filesystems alike), so the contract
/// is the format and the agreement, never the specific verdict.
#[cfg(unix)]
#[test]
fn qualify_prints_a_stable_verdict_line_that_matches_its_exit_code() {
    let (stdout, code) = run_host_script(&["qualify"]);
    let first = stdout.lines().next().unwrap_or_default().to_string();
    if let Some(rest) = first.strip_prefix("kache-host: verdict=qualify") {
        assert!(
            rest.is_empty() || rest.starts_with(" candidate="),
            "a qualify verdict names its candidate (or defers with `-`): {first}"
        );
        assert_eq!(code, Some(0), "a qualify verdict exits 0");
        // The report block's verdict line is built from it. A no-qualify
        // verdict carries its deciding fact in the reason instead (spec §6:
        // one line naming the reason), so only qualify promises this line.
        assert!(
            stdout.contains("kache-host: devices checkout="),
            "a qualify verdict carries the devices line:\n{stdout}"
        );
    } else if let Some(rest) = first.strip_prefix("kache-host: verdict=no-qualify") {
        assert!(
            rest.starts_with(" reason="),
            "a no-qualify verdict names its reason: {first}"
        );
        assert_eq!(code, Some(1), "a no-qualify verdict exits 1");
    } else {
        panic!("first line is not a stable verdict line: {first:?}\nstdout:\n{stdout}");
    }
}

/// The env-passthrough probe must fail against a wrapper that strips
/// `DYLD_*`. The fake wrapper is a plain `exec "$@"` bash script: bash
/// expunges `DYLD_*` from its own environment at startup (measured
/// 2026-09-23), which is the same observable behavior the hardened kache
/// has — the compiler it launches never receives the variable — while
/// non-`DYLD` variables pass through, so the probe's control line still
/// separates "stripped" from "never ran".
#[cfg(target_os = "macos")]
#[test]
fn probe_passthrough_fails_when_the_wrapper_strips_dyld() {
    let dir = std::env::temp_dir().join(format!("kache-host-contract.{}.strip", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let wrapper = dir.join("kache");
    fs::write(
        &wrapper,
        "#!/usr/bin/env bash\nexec \"$@\"\n",
    )
    .expect("write fake wrapper");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).expect("chmod");

    let output = bash()
        .arg("scripts/kache-host.sh")
        .arg("probe-passthrough")
        .env("KACHE_HOST_BIN", &wrapper)
        .output()
        .expect("probe must spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(output.status.code(), Some(1), "a stripping wrapper fails the probe:\n{stdout}");
    assert!(
        stdout.contains("kache-host: passthrough=fail"),
        "the failure is reported on the stable line:\n{stdout}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The passing direction uses an ad hoc re-signed copy of /usr/bin/env as the
/// wrapper — the exact re-sign the probe exists to gate — so the variable
/// survives into the stub compiler and the probe reports pass. (A copy of a
/// platform binary that keeps its Apple signature is killed by AMFI outside
/// the system volume, so re-signing is not optional for any copied-env fake.)
#[cfg(target_os = "macos")]
#[test]
fn probe_passthrough_passes_when_the_wrapper_forwards_dyld() {
    let dir = std::env::temp_dir().join(format!("kache-host-contract.{}.fwd", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let wrapper = dir.join("kache");
    fs::copy("/usr/bin/env", &wrapper).expect("copy /usr/bin/env");
    let signed = Command::new("codesign")
        .args(["--force", "-s", "-"])
        .arg(&wrapper)
        .output()
        .expect("codesign must spawn");
    assert!(
        signed.status.success(),
        "codesign re-sign failed: {}",
        String::from_utf8_lossy(&signed.stderr)
    );

    let output = bash()
        .arg("scripts/kache-host.sh")
        .arg("probe-passthrough")
        .env("KACHE_HOST_BIN", &wrapper)
        .output()
        .expect("probe must spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(output.status.code(), Some(0), "a forwarding wrapper passes the probe:\n{stdout}");
    assert!(
        stdout.contains("kache-host: passthrough=pass"),
        "the pass is reported on the stable line:\n{stdout}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Off macOS the `DYLD_*` question does not exist and the probe says so on
/// its stable line rather than failing.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn probe_passthrough_is_not_applicable_outside_macos() {
    let (stdout, code) = run_host_script(&["probe-passthrough"]);
    assert_eq!(code, Some(0));
    assert!(
        stdout.contains("kache-host: passthrough=n/a reason=not-macos"),
        "the n/a line is stable:\n{stdout}"
    );
}

/// `report` requires kache; with kache off the PATH it must name that on the
/// error line and exit 2 rather than guessing a store.
#[cfg(unix)]
#[test]
fn report_names_kache_absence() {
    let output = bash()
        .arg("scripts/kache-host.sh")
        .arg("report")
        .env("WT", "")
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("report must spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stdout.contains("kache-host: error=kache-absent"),
        "absence is named on the stable line:\n{stdout}"
    );
}

/// Unknown subcommands are usage errors, never silent success.
#[cfg(unix)]
#[test]
fn unknown_subcommand_is_a_usage_error() {
    let output = bash()
        .arg("scripts/kache-host.sh")
        .arg("definitely-not-a-subcommand")
        .output()
        .expect("script must spawn");
    assert_eq!(output.status.code(), Some(2));
}

/// A fresh home has no user cache directory yet (`~/.cache` on Linux,
/// `~/Library/Caches` on macOS). The cascade must still consider the user
/// cache dir, deciding its device from the nearest existing ancestor. The
/// earlier script only looked at the immediate parent, so a fresh home fell
/// through to the root-owned mount point and reported
/// `no-user-writable-store-location-on-the-checkout-volume` on a filesystem
/// that clones (found on the WSL guest, 2026-09-23). A non-qualifying
/// verdict must also leave the fresh home as it found it: the cascade may
/// create the candidate to probe it, and must remove what it created.
#[cfg(unix)]
#[test]
fn qualify_places_the_candidate_in_a_fresh_home_and_cleans_up_on_no() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let home = scratch.path().join("home");
    let checkout = scratch.path().join("checkout");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&checkout).expect("checkout");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/kache-host.sh"))
        .arg("qualify")
        .current_dir(&checkout)
        .env("HOME", &home)
        .env_remove("XDG_CACHE_HOME")
        .env("WT", "")
        .env("GIT_CEILING_DIRECTORIES", scratch.path())
        .output()
        .expect("qualify must spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let first = stdout.lines().next().unwrap_or_default();

    assert!(
        !first.contains("no-user-writable-store-location"),
        "the user cache dir under a fresh home is a valid placement:\n{stdout}"
    );
    if let Some(candidate) = first.strip_prefix("kache-host: verdict=qualify candidate=") {
        assert_eq!(output.status.code(), Some(0));
        let candidate = PathBuf::from(candidate);
        assert!(
            candidate.starts_with(&home) && candidate.is_dir(),
            "the candidate is the user cache dir under the fresh home:\n{stdout}"
        );
    } else {
        assert!(
            first.starts_with("kache-host: verdict=no-qualify reason=clone-unsupported")
                || first.starts_with("kache-host: verdict=no-qualify reason=store-device-"),
            "a fresh home can only fail on the clone probe or the device check:\n{stdout}"
        );
        assert_eq!(output.status.code(), Some(1));
        let leftovers: Vec<_> = fs::read_dir(&home)
            .expect("home")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert!(
            leftovers.is_empty(),
            "a no-qualify verdict leaves the fresh home untouched, found {leftovers:?}:\n{stdout}"
        );
    }
}
