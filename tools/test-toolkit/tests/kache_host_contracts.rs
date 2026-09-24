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
};
#[cfg(unix)]
use std::process::Command;

mod common;

#[cfg(unix)]
use common::kache::{InvalidWorktreeBase, KacheHostFixture, RepoInputs};

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

/// A Linux (or, through [`Self::macos`], macOS) host, emulated with fakes on
/// `PATH`, whose checkout volume is mounted at `<scratch>/vol`: `df -P` names
/// that directory as every path's mount point, `stat` (`-c %d` or `-f %d`)
/// puts every path under [`Self::place_off_device`]'s prefixes — by default
/// `<scratch>/home`, which holds the user cache dir — on a second device, and
/// a clone (`cp --reflink=always`, `cp -c`) is a plain copy. The directories
/// below `vol` are the test's to lay out and chmod.
#[cfg(unix)]
struct CheckoutVolumeHost {
    scratch: tempfile::TempDir,
}

#[cfg(unix)]
impl CheckoutVolumeHost {
    fn new() -> Self {
        Self::emulating("Linux")
    }

    fn macos() -> Self {
        Self::emulating("Darwin")
    }

    fn emulating(kernel: &str) -> Self {
        let scratch = tempfile::tempdir().expect("tempdir");
        let host = Self { scratch };
        for dir in ["bin", "home", "vol"] {
            fs::create_dir_all(host.path(dir)).expect("scratch directory");
        }
        host.place_off_device(&["home"]);
        let off_device = host.path("off-device");
        let volume = host.path("vol");
        host.write_tool(
            "uname",
            &format!(
                "[[ \"${{1:-}}\" == -s ]] && {{ echo {kernel}; exit 0; }}\nexec /usr/bin/uname \"$@\"\n"
            ),
        );
        host.write_tool(
            "df",
            &format!(
                "echo 'Filesystem 1024-blocks Used Available Capacity Mounted on'\necho 'fake 1 1 1 1% {}'\n",
                volume.display()
            ),
        );
        host.write_tool(
            "stat",
            &format!(
                r#"[[ ( "${{1:-}}" == -c || "${{1:-}}" == -f ) && "${{2:-}}" == %d && $# -eq 3 ]] || exec /usr/bin/stat "$@"
while IFS= read -r prefix; do
    [[ -n "$prefix" && ( "$3" == "$prefix" || "$3" == "$prefix"/* ) ]] && {{ echo 999999; exit 0; }}
done < '{off_device}'
/usr/bin/stat -c %d "$3" 2> /dev/null || /usr/bin/stat -f %d "$3"
"#,
                off_device = off_device.display()
            ),
        );
        host.write_tool(
            "cp",
            r#"args=()
for arg in "$@"; do [[ "$arg" == --reflink=always || "$arg" == -c ]] || args+=("$arg"); done
exec /bin/cp "${args[@]}"
"#,
        );
        host
    }

    /// Moves the scratch-relative `prefixes` (and everything under them) to
    /// the second device, replacing the previous set.
    fn place_off_device(&self, prefixes: &[&str]) {
        let lines: String = prefixes
            .iter()
            .map(|prefix| format!("{}\n", self.path(prefix).display()))
            .collect();
        fs::write(self.path("off-device"), lines).expect("write off-device prefixes");
    }

    /// Creates `relative` as an empty directory with `mode`, standing in for
    /// a store directory an earlier run or the user left behind.
    fn pre_existing_dir(&self, relative: &str, mode: u32) {
        fs::create_dir_all(self.path(relative)).expect("pre-existing directory");
        self.chmod(relative, mode);
    }

    /// `relative` under the scratch root, symlinks resolved so it matches
    /// the `pwd` the script sees (macOS temp dirs sit behind `/var`).
    fn path(&self, relative: &str) -> PathBuf {
        fs::canonicalize(self.scratch.path())
            .expect("canonical scratch")
            .join(relative)
    }

    fn write_tool(&self, name: &str, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = self.path("bin").join(name);
        fs::write(&path, format!("#!/usr/bin/env bash\n{body}")).expect("write fake tool");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod fake tool");
    }

    fn chmod(&self, relative: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(self.path(relative), fs::Permissions::from_mode(mode))
            .expect("chmod fixture directory");
    }

    /// Whether a 0555 directory keeps this process out; it does not for root
    /// (some CI containers), which makes the layouts below meaningless.
    fn permissions_bind(&self) -> bool {
        let probe = self.path("read-only-probe");
        fs::create_dir_all(&probe).expect("probe directory");
        self.chmod("read-only-probe", 0o555);
        let binds = fs::write(probe.join("file"), "").is_err();
        self.chmod("read-only-probe", 0o755);
        binds
    }

    /// Runs `qualify` with the checkout `relative` as the working directory
    /// and returns stdout and the exit code.
    fn qualify(&self, relative: &str) -> (String, Option<i32>) {
        let output = Command::new("bash")
            .arg(repo_root().join("scripts/kache-host.sh"))
            .arg("qualify")
            .current_dir(self.path(relative))
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.path("bin").display()),
            )
            .env("HOME", self.path("home"))
            .env("XDG_CACHE_HOME", self.path("home/.cache"))
            .env("WT", "")
            .env("GIT_CEILING_DIRECTORIES", self.path(""))
            .output()
            .expect("qualify must spawn");
        (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            output.status.code(),
        )
    }
}

/// Review-1 finding #5: with the user cache dir off the checkout's device
/// and a root-owned mount point (`/data`), the cascade must still find the
/// user-owned directory the checkout lives under (`/data/src`) rather than
/// report that nothing on the volume is writable — the store goes to the
/// highest such ancestor, never into the checkout.
#[cfg(unix)]
#[test]
fn qualify_places_the_store_in_a_user_owned_ancestor_below_a_read_only_mount_point() {
    let host = CheckoutVolumeHost::new();
    if !host.permissions_bind() {
        eprintln!("skipped: running with privileges that ignore directory permissions");
        return;
    }
    fs::create_dir_all(host.path("vol/src/team/repo")).expect("checkout");
    host.chmod("vol", 0o555);

    let (stdout, code) = host.qualify("vol/src/team/repo");
    host.chmod("vol", 0o755);

    let expected = host.path("vol/src/kache");
    assert_eq!(
        stdout.lines().next(),
        Some(
            format!(
                "kache-host: verdict=qualify candidate={}",
                expected.display()
            )
            .as_str()
        ),
        "the highest user-owned ancestor below the mount point holds the store:\n{stdout}"
    );
    assert_eq!(code, Some(0), "{stdout}");
    assert!(expected.is_dir(), "the candidate store exists");
    assert!(
        !host.path("vol/kache").exists(),
        "the mount point is never written"
    );
    assert!(
        !host.path("vol/src/team/repo/kache").exists(),
        "the store never lands in the checkout"
    );
}

/// The negative of the above: no ancestor between the checkout and the
/// read-only mount point is writable, so the cascade reports the existing
/// reason and creates nothing.
#[cfg(unix)]
#[test]
fn qualify_reports_no_writable_location_when_no_ancestor_is_user_writable() {
    let host = CheckoutVolumeHost::new();
    if !host.permissions_bind() {
        eprintln!("skipped: running with privileges that ignore directory permissions");
        return;
    }
    fs::create_dir_all(host.path("vol/src/repo")).expect("checkout");
    host.chmod("vol/src", 0o555);
    host.chmod("vol", 0o555);

    let (stdout, code) = host.qualify("vol/src/repo");
    host.chmod("vol", 0o755);
    host.chmod("vol/src", 0o755);

    assert_eq!(
        stdout.lines().next(),
        Some(
            "kache-host: verdict=no-qualify reason=no-user-writable-store-location-on-the-checkout-volume"
        ),
        "{stdout}"
    );
    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        !host.path("vol/src/repo/kache").exists(),
        "the checkout is never a placement"
    );
}

/// A writable ancestor inside a Git working tree (the checkout nested in
/// another repository) is not a placement: the store would show up in that
/// repository's status and be swept by its cleans.
#[cfg(unix)]
#[test]
fn qualify_never_places_the_store_inside_an_enclosing_git_working_tree() {
    let host = CheckoutVolumeHost::new();
    if !host.permissions_bind() {
        eprintln!("skipped: running with privileges that ignore directory permissions");
        return;
    }
    fs::create_dir_all(host.path("vol/outer/.git")).expect("enclosing working tree");
    fs::create_dir_all(host.path("vol/outer/repo")).expect("checkout");
    host.chmod("vol", 0o555);

    let (stdout, code) = host.qualify("vol/outer/repo");
    host.chmod("vol", 0o755);

    assert!(
        stdout.starts_with(
            "kache-host: verdict=no-qualify reason=no-user-writable-store-location-on-the-checkout-volume"
        ),
        "{stdout}"
    );
    assert_eq!(code, Some(1), "{stdout}");
    assert!(!host.path("vol/outer/kache").exists());
}

/// Review-6 finding: an existing `<mount>/kache` this user cannot write ended
/// the macOS cascade — `mkdir -p` succeeds on an existing directory, so the
/// script chose it, the clone probe failed (`clone-unsupported`), and the
/// cleanup removed the directory it had never created. The cascade must skip
/// it untouched and take the user cache dir on the checkout's device.
#[cfg(unix)]
#[test]
fn qualify_skips_an_unwritable_mount_point_store_on_macos_and_keeps_it() {
    let host = CheckoutVolumeHost::macos();
    if !host.permissions_bind() {
        eprintln!("skipped: running with privileges that ignore directory permissions");
        return;
    }
    host.place_off_device(&[]);
    host.pre_existing_dir("vol/kache", 0o500);
    fs::create_dir_all(host.path("vol/src/repo")).expect("checkout");

    let (stdout, code) = host.qualify("vol/src/repo");
    host.chmod("vol/kache", 0o755);

    let expected = host.path("home/Library/Caches/kache");
    assert_eq!(
        stdout.lines().next(),
        Some(format!("kache-host: verdict=qualify candidate={}", expected.display()).as_str()),
        "the user cache dir on the checkout's device is the next placement:\n{stdout}"
    );
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        host.path("vol/kache").is_dir(),
        "the pre-existing mount-point store survives:\n{stdout}"
    );
}

/// The same defect in the cascade every OS shares: with the user cache dir
/// off the checkout's device, an existing unwritable `<mount>/kache` must be
/// skipped for the user-owned ancestor, and left in place.
#[cfg(unix)]
#[test]
fn qualify_skips_an_unwritable_shared_mount_point_store_and_keeps_it() {
    let host = CheckoutVolumeHost::new();
    if !host.permissions_bind() {
        eprintln!("skipped: running with privileges that ignore directory permissions");
        return;
    }
    host.pre_existing_dir("vol/kache", 0o500);
    fs::create_dir_all(host.path("vol/src/repo")).expect("checkout");

    let (stdout, code) = host.qualify("vol/src/repo");
    host.chmod("vol/kache", 0o755);

    let expected = host.path("vol/src/kache");
    assert_eq!(
        stdout.lines().next(),
        Some(format!("kache-host: verdict=qualify candidate={}", expected.display()).as_str()),
        "the user-owned ancestor is the next placement:\n{stdout}"
    );
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        host.path("vol/kache").is_dir(),
        "the pre-existing mount-point store survives:\n{stdout}"
    );
}

/// A writable existing `<mount>/kache` that is on another device (a mount or
/// symlink onto a second volume) cannot serve clones either: the cascade must
/// move on rather than stop at `store-device-…`.
#[cfg(unix)]
#[test]
fn qualify_skips_an_off_device_mount_point_store_on_macos() {
    let host = CheckoutVolumeHost::macos();
    host.place_off_device(&["vol/kache"]);
    host.pre_existing_dir("vol/kache", 0o755);
    fs::create_dir_all(host.path("vol/src/repo")).expect("checkout");

    let (stdout, code) = host.qualify("vol/src/repo");

    let expected = host.path("home/Library/Caches/kache");
    assert_eq!(
        stdout.lines().next(),
        Some(format!("kache-host: verdict=qualify candidate={}", expected.display()).as_str()),
        "an off-device store directory is not a placement:\n{stdout}"
    );
    assert_eq!(code, Some(0), "{stdout}");
    assert!(host.path("vol/kache").is_dir(), "{stdout}");
}

/// The negative: every placement already exists and is unwritable — the
/// macOS mount point (tried first and again in the shared cascade), the user
/// cache dir on the checkout's device, and the user-owned ancestor's
/// `kache/`. The verdict names the reason, and nothing that existed is
/// removed.
#[cfg(unix)]
#[test]
fn qualify_reports_no_writable_location_and_keeps_every_unusable_store_directory() {
    let host = CheckoutVolumeHost::macos();
    if !host.permissions_bind() {
        eprintln!("skipped: running with privileges that ignore directory permissions");
        return;
    }
    host.place_off_device(&[]);
    let unusable = ["vol/kache", "home/Library/Caches/kache", "vol/src/kache"];
    for relative in unusable {
        host.pre_existing_dir(relative, 0o500);
    }
    fs::create_dir_all(host.path("vol/src/repo")).expect("checkout");

    let (stdout, code) = host.qualify("vol/src/repo");
    for relative in unusable {
        host.chmod(relative, 0o755);
    }

    assert_eq!(
        stdout.lines().next(),
        Some(
            "kache-host: verdict=no-qualify reason=no-user-writable-store-location-on-the-checkout-volume"
        ),
        "{stdout}"
    );
    assert_eq!(code, Some(1), "{stdout}");
    for relative in unusable {
        assert!(
            host.path(relative).is_dir(),
            "the pre-existing {relative} survives a no-qualify verdict:\n{stdout}"
        );
    }
}

/// Runs `kache-host.sh wrapper` from `cwd` with `cargo_home` as
/// `$CARGO_HOME` and neither wrapper variable exported, so only config
/// files decide.
#[cfg(unix)]
fn wrapper_from(cwd: &std::path::Path, cargo_home: &std::path::Path) -> (String, Option<i32>) {
    let output = Command::new("bash")
        .arg(repo_root().join("scripts/kache-host.sh"))
        .arg("wrapper")
        .current_dir(cwd)
        .env("CARGO_HOME", cargo_home)
        .env_remove("RUSTC_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .output()
        .expect("running scripts/kache-host.sh wrapper must not fail to spawn");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        output.status.code(),
    )
}

#[cfg(unix)]
fn write_wrapper_config(dir: &std::path::Path, value: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("config directory");
    let config = dir.join("config.toml");
    fs::write(&config, format!("[build]\nrustc-wrapper = \"{value}\"\n")).expect("write config");
    fs::canonicalize(config).expect("canonical config path")
}

/// Cargo merges `.cargo/config.toml` from the working directory up through
/// every ancestor, nearer first, and reads `$CARGO_HOME` last. The helper
/// used to stop at the working directory's own `.cargo/`, so an ancestor's
/// wrapper — the one Cargo actually runs here — went unreported.
#[cfg(unix)]
#[test]
fn wrapper_reads_ancestor_configs_nearer_first_before_cargo_home() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(scratch.path()).expect("canonical scratch");
    let farther = write_wrapper_config(&root.join("a/.cargo"), "sccache");
    let nearer = write_wrapper_config(&root.join("a/b/.cargo"), "kache");
    let cargo_home = root.join("cargo-home");
    write_wrapper_config(&cargo_home, "other-wrapper");
    let cwd = root.join("a/b/c");
    fs::create_dir_all(&cwd).expect("working directory");

    let (stdout, code) = wrapper_from(&cwd, &cargo_home);

    assert_eq!(code, Some(0), "{stdout}");
    let sources: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("kache-host: wrapper-source "))
        .collect();
    assert_eq!(
        sources,
        [
            format!(
                "kache-host: wrapper-source scope=ancestor kind=kache source={} value=kache",
                nearer.display()
            ),
            format!(
                "kache-host: wrapper-source scope=ancestor kind=other source={} value=sccache",
                farther.display()
            ),
            format!(
                "kache-host: wrapper-source scope=host kind=other source={}/config.toml value=other-wrapper",
                cargo_home.display()
            ),
        ],
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "kache-host: wrapper=kache source={} value=kache",
            nearer.display()
        )),
        "the nearest ancestor wins:\n{stdout}"
    );
}

/// A `$CARGO_HOME` that is itself an ancestor's `.cargo/` (a checkout under
/// `$HOME` with the default `~/.cargo`) is read once, at its place in the
/// walk, and reported as the host file under the `$CARGO_HOME` spelling
/// `_ensure-kache` matches when it neutralizes its own activation.
#[cfg(unix)]
#[test]
fn wrapper_reports_a_cargo_home_among_the_ancestors_once_as_the_host_file() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(scratch.path()).expect("canonical scratch");
    let cargo_home = root.join("home/.cargo");
    write_wrapper_config(&cargo_home, "kache");
    let above_home = write_wrapper_config(&root.join(".cargo"), "sccache");
    let cwd = root.join("home/checkout");
    fs::create_dir_all(&cwd).expect("working directory");

    let (stdout, code) = wrapper_from(&cwd, &cargo_home);

    assert_eq!(code, Some(0), "{stdout}");
    let sources: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("kache-host: wrapper-source "))
        .collect();
    assert_eq!(
        sources,
        [
            format!(
                "kache-host: wrapper-source scope=host kind=kache source={}/config.toml value=kache",
                cargo_home.display()
            ),
            format!(
                "kache-host: wrapper-source scope=ancestor kind=other source={} value=sccache",
                above_home.display()
            ),
        ],
        "{stdout}"
    );
}

/// The ground truth for the two tests above: real Cargo, in a scratch project
/// whose only wrapper setting is a parent directory's `.cargo/config.toml`,
/// runs that wrapper — and the helper names the same file. The stub fails,
/// so nothing is compiled; its log proves Cargo reached it.
#[cfg(unix)]
#[test]
fn wrapper_names_the_parent_config_wrapper_that_real_cargo_invokes() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(scratch.path()).expect("canonical scratch");
    let log = root.join("stub.log");
    let stub = root.join("stub/kache");
    fs::create_dir_all(root.join("stub")).expect("stub directory");
    fs::write(
        &stub,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\necho 'stub kache refuses to compile' >&2\nexit 97\n",
            log.display()
        ),
    )
    .expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    let parent_config =
        write_wrapper_config(&root.join("parent/.cargo"), &stub.display().to_string());
    let project = root.join("parent/project");
    fs::create_dir_all(project.join("src")).expect("project directory");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"wrapper-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("write manifest");
    fs::write(project.join("src/lib.rs"), "").expect("write lib");
    let cargo_home = root.join("cargo-home");
    fs::create_dir_all(&cargo_home).expect("cargo home");

    let cargo = Command::new("cargo")
        .args(["check", "--offline", "--quiet"])
        .current_dir(&project)
        .env("CARGO_HOME", &cargo_home)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env_remove("RUSTC_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")
        .env_remove("RUSTC")
        .env_remove("CARGO_BUILD_RUSTC")
        .output()
        .expect("cargo must spawn");
    let cargo_stderr = String::from_utf8_lossy(&cargo.stderr);
    let invocations = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !invocations.is_empty(),
        "Cargo ran the parent directory's wrapper (exit {:?}):\n{cargo_stderr}",
        cargo.status.code()
    );
    assert!(
        !cargo.status.success(),
        "the failing stub fails the build:\n{cargo_stderr}"
    );

    let (stdout, code) = wrapper_from(&project, &cargo_home);

    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "kache-host: wrapper=kache source={} value={}",
            parent_config.display(),
            stub.display()
        )),
        "the helper names the wrapper Cargo ran:\n{stdout}"
    );
}

/// Cargo reads one config file per directory: the legacy `config` whenever
/// it exists, ignoring a `config.toml` beside it. The helper used to read
/// both and report a wrapper from the ignored `config.toml` as the winner.
/// Real Cargo is the ground truth for both directories it applies to —
/// `$CARGO_HOME` and a project `.cargo/` — through a logging pass-through
/// stub: in the host case the build succeeds without running it, in the
/// project case Cargo runs it.
#[cfg(unix)]
#[test]
fn wrapper_reads_only_the_legacy_config_where_both_files_exist() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(scratch.path()).expect("canonical scratch");
    let log = root.join("stub.log");
    let stub = root.join("stub/kache");
    fs::create_dir_all(root.join("stub")).expect("stub directory");
    fs::write(
        &stub,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec \"$@\"\n",
            log.display()
        ),
    )
    .expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    let project = root.join("project");
    fs::create_dir_all(project.join("src")).expect("project directory");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"wrapper-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("write manifest");
    fs::write(project.join("src/lib.rs"), "").expect("write lib");
    let cargo_home = root.join("cargo-home");
    fs::create_dir_all(&cargo_home).expect("cargo home");
    let cargo_runs_stub = || {
        let _ = fs::remove_file(&log);
        let cargo = Command::new("cargo")
            .args(["check", "--offline", "--quiet"])
            .current_dir(&project)
            .env("CARGO_HOME", &cargo_home)
            .env("CARGO_TARGET_DIR", root.join("target"))
            .env_remove("RUSTC_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")
            .env_remove("RUSTC")
            .env_remove("CARGO_BUILD_RUSTC")
            .output()
            .expect("cargo must spawn");
        let stderr = String::from_utf8_lossy(&cargo.stderr).into_owned();
        assert!(cargo.status.success(), "the probe build succeeds:\n{stderr}");
        (log.exists(), stderr)
    };

    // $CARGO_HOME: the legacy file sets no wrapper; config.toml names the stub.
    fs::write(cargo_home.join("config"), "[term]\ncolor = \"never\"\n").expect("legacy config");
    write_wrapper_config(&cargo_home, &stub.display().to_string());

    let (ran, stderr) = cargo_runs_stub();
    assert!(!ran, "Cargo ignores the config.toml beside a legacy config:\n{stderr}");
    assert!(stderr.contains("Using"), "Cargo warns which file it uses:\n{stderr}");
    let (stdout, code) = wrapper_from(&project, &cargo_home);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.contains("kache-host: wrapper=none source=- value="),
        "the ignored file's wrapper is not reported:\n{stdout}"
    );
    assert!(!stdout.contains("wrapper-source"), "{stdout}");

    // A project `.cargo/`: the legacy file names the stub; config.toml a
    // path that does not exist.
    fs::remove_file(cargo_home.join("config.toml")).expect("clear host config.toml");
    fs::create_dir_all(project.join(".cargo")).expect("project .cargo");
    fs::write(
        project.join(".cargo/config"),
        format!("[build]\nrustc-wrapper = \"{}\"\n", stub.display()),
    )
    .expect("project legacy config");
    write_wrapper_config(&project.join(".cargo"), "/definitely/missing/kache");

    let (ran, stderr) = cargo_runs_stub();
    assert!(ran, "Cargo runs the legacy project config's wrapper:\n{stderr}");
    let (stdout, code) = wrapper_from(&project, &cargo_home);
    assert_eq!(code, Some(0), "{stdout}");
    let sources: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("kache-host: wrapper-source "))
        .collect();
    assert_eq!(
        sources,
        [format!(
            "kache-host: wrapper-source scope=repo kind=kache source=.cargo/config value={}",
            stub.display()
        )],
        "only the file Cargo reads contributes:\n{stdout}"
    );
}

/// `config -> config.toml` is the layout Cargo's own deprecation help
/// suggests; the host file is then named the way Cargo reads it, which is
/// the spelling `_ensure-kache` matches when it undoes its activation.
#[cfg(unix)]
#[test]
fn wrapper_names_a_legacy_config_symlink_as_the_host_file() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(scratch.path()).expect("canonical scratch");
    let cargo_home = root.join("cargo-home");
    write_wrapper_config(&cargo_home, "kache");
    std::os::unix::fs::symlink("config.toml", cargo_home.join("config")).expect("symlink");
    let cwd = root.join("checkout");
    fs::create_dir_all(&cwd).expect("working directory");

    let (stdout, code) = wrapper_from(&cwd, &cargo_home);

    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "kache-host: wrapper=kache source={}/config value=kache",
            cargo_home.display()
        )),
        "{stdout}"
    );
}

/// A `config` directory fails Cargo's read ("Is a directory"), so the helper
/// must not skip past it to a `config.toml` Cargo never reaches.
#[cfg(unix)]
#[test]
fn wrapper_reports_a_legacy_config_directory_as_unreadable() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(scratch.path()).expect("canonical scratch");
    let cargo_home = root.join("cargo-home");
    write_wrapper_config(&cargo_home, "kache");
    fs::create_dir_all(cargo_home.join("config")).expect("config directory");
    let cwd = root.join("checkout");
    fs::create_dir_all(&cwd).expect("working directory");

    let (stdout, code) = wrapper_from(&cwd, &cargo_home);

    assert_eq!(code, Some(2), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "kache-host: error=unparseable-cargo-config path={}/config",
            cargo_home.display()
        )),
        "{stdout}"
    );
}

#[cfg(unix)]
fn repo_inputs() -> RepoInputs {
    let root = repo_root();
    RepoInputs {
        justfile: root.join("justfile"),
        just_dir: root.join("just"),
        scripts: vec![
            root.join("scripts/cargo-path.sh"),
            root.join("scripts/kache-config-merge.py"),
            root.join("scripts/kache-host.sh"),
        ],
        floor_file: root.join(".github/kache-min-version"),
    }
}

/// `report` on an isolated host (fake kache, clones that succeed) whose
/// worktree-base setting `setup` arranges.
#[cfg(unix)]
fn report_with(setup: impl FnOnce(&mut KacheHostFixture)) -> (Option<i32>, String) {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    setup(&mut fixture);
    let output = fixture.host_script("report");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// A setting `wt` refuses is reported as `invalid` with its rule and path —
/// never as `unconfigured` (which the script used to print for all four) or
/// `covered` (which it printed for a `WT` naming a Git repository) — and
/// gets no clone check.
#[cfg(unix)]
fn assert_report_names_invalid_base(setting: InvalidWorktreeBase) {
    let mut expected = None;
    let (code, stdout) = report_with(|fixture| {
        expected = Some(fixture.configure_invalid_worktree_base(setting));
    });
    let (reason, path) = expected.expect("configured");

    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.contains(&format!(
            "kache-host: base=invalid reason={reason} path={}\n",
            path.display()
        )),
        "{setting:?} is invalid with its reason and path:\n{stdout}"
    );
    assert!(
        stdout.contains("kache-host: clone base=- reason=invalid\n"),
        "an invalid base gets no clone check:\n{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn report_names_a_missing_wt_path_as_an_invalid_base() {
    assert_report_names_invalid_base(InvalidWorktreeBase::MissingWtPath);
}

#[cfg(unix)]
#[test]
fn report_names_a_wt_git_repository_as_an_invalid_base() {
    assert_report_names_invalid_base(InvalidWorktreeBase::WtGitRepository);
}

#[cfg(unix)]
#[test]
fn report_names_a_malformed_worktree_config_as_an_invalid_base() {
    assert_report_names_invalid_base(InvalidWorktreeBase::MalformedConfig);
}

#[cfg(unix)]
#[test]
fn report_names_a_missing_config_base_dir_as_an_invalid_base() {
    assert_report_names_invalid_base(InvalidWorktreeBase::MissingConfigBaseDir);
}

/// The rest of `wt`'s resolver contract (`resolve_base_dir`): a non-empty
/// `WT` wins without `~/.worktree.json` being read, an empty one defers to
/// it, and the config parses as serde_json parses `{ base_dir: String }` —
/// a missing, non-string, or repeated `base_dir` is malformed, the
/// one-element array form is not. A path that exists but is not a directory
/// is one kache cannot clone into, so it is invalid here though `wt`
/// accepts it.
#[cfg(unix)]
#[test]
fn report_resolves_the_worktree_base_by_the_wt_contract() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let base = scratch.path().join("base");
    fs::create_dir_all(&base).expect("base");
    let repository = scratch.path().join("repository");
    fs::create_dir_all(repository.join(".git")).expect("repository");
    let file = scratch.path().join("file");
    fs::write(&file, "").expect("file");
    let quoted = |path: &std::path::Path| serde_json::to_string(path).expect("JSON path");

    let cases: Vec<(&str, Option<PathBuf>, Option<String>, String)> = vec![
        (
            "WT wins over a malformed config",
            Some(base.clone()),
            Some("{".to_owned()),
            format!("base=covered path={}", base.display()),
        ),
        (
            "an empty WT defers to the config",
            None,
            Some(format!("{{\"base_dir\": {}, \"extra\": 1}}", quoted(&base))),
            format!("base=covered path={}", base.display()),
        ),
        (
            "the one-element array form",
            None,
            Some(format!("[{}]", quoted(&base))),
            format!("base=covered path={}", base.display()),
        ),
        (
            "a config base_dir naming a Git repository",
            None,
            Some(format!("{{\"base_dir\": {}}}", quoted(&repository))),
            format!(
                "base=invalid reason=config-base-dir-is-a-git-repository path={}",
                repository.display()
            ),
        ),
        (
            "no base_dir key",
            None,
            Some("{}".to_owned()),
            "base=invalid reason=config-malformed path=".to_owned(),
        ),
        (
            "a null base_dir",
            None,
            Some("{\"base_dir\": null}".to_owned()),
            "base=invalid reason=config-malformed path=".to_owned(),
        ),
        (
            "a repeated base_dir",
            None,
            Some(format!(
                "{{\"base_dir\": {0}, \"base_dir\": {0}}}",
                quoted(&base)
            )),
            "base=invalid reason=config-malformed path=".to_owned(),
        ),
        (
            "a WT naming a file",
            Some(file.clone()),
            None,
            format!("base=invalid reason=wt-not-a-directory path={}", file.display()),
        ),
        ("nothing configured", None, None, "base=unconfigured path=-".to_owned()),
    ];
    for (label, wt, config, expected) in cases {
        let (code, stdout) = report_with(|fixture| {
            if let Some(wt) = wt {
                fixture.set_wt_env(wt);
            }
            if let Some(config) = config {
                fs::write(fixture.home().join(".worktree.json"), config).expect("config");
            }
        });
        assert_eq!(code, Some(0), "{label}:\n{stdout}");
        assert!(
            stdout.contains(&format!("kache-host: {expected}")),
            "{label}: expected `{expected}`:\n{stdout}"
        );
    }
}
