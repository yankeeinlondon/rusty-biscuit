//! An isolated host for running the root justfile's kache recipes
//! (`kache-status`, `_ensure-kache`) as real subprocesses against a fake
//! `kache`, a controllable clone probe, and fixture-owned home, Cargo, and
//! config directories — never the developer's kache config or daemon.
//!
//! Layout under one temporary root:
//!
//! - `checkout/` — the recipes' working directory: a copy of the justfile and
//!   the floor file, with `just/` symlinked from the repository and each of
//!   [`RepoInputs::scripts`] symlinked into `scripts/`. It is not a Git
//!   repository, so `kache-host.sh` treats it as the checkout.
//! - `store/` — kache's built-in default store: what the fake
//!   `kache doctor --json` reports when the config file it loads pins no
//!   `local_store` (a pinned one is reported instead).
//! - `home/`, `cargo-home/`, `tmp/` — `HOME`, `CARGO_HOME`, and `TMPDIR`;
//!   `home/.config/kache/config.toml` is the kache user config
//!   (`XDG_CONFIG_HOME` is `home/.config`).
//! - `fake-kache/` — the files the fake `kache` answers from, re-read on
//!   every invocation: `version`, `daemon.json` (the whole
//!   `kache daemon --json` document), `passthrough` (`forward` or
//!   `strip`), `service-config` (the config file the daemon loads when
//!   it starts), and, only after [`KacheHostFixture::set_doctor_store`],
//!   `doctor-store`; each invocation's arguments are appended to
//!   `invocations.log`. `daemon.json` is also the fake daemon's state:
//!   `daemon install|start|restart` change it the way a service manager
//!   would. `release` is the version the fake `cargo binstall` installs.
//!   `commands.log` records, in order, every fake tool that ran — `kache`,
//!   `cargo`, `cargo-binstall`, `codesign`, a clone through `cp`, and each
//!   `python3 scripts/kache-config-merge.py` config write — as one
//!   `<tool> <args>` line ([`KacheHostFixture::commands`]).
//! - `bin/` — the fakes above plus a no-op `sleep` (the daemon waits poll
//!   state that changes synchronously here, so a wait that times out costs
//!   only its reads), a link to the test host's
//!   `just` (recipes call recipes), and a logging shim over the host's
//!   `python3` (the scripts need `tomllib`, which macOS's `/usr/bin/python3`
//!   lacks); the child `PATH` is this directory plus `/usr/bin:/bin`, so no
//!   host-installed kache, cargo, or codesign is reachable.
//! - `other-kache/kache` — only after [`KacheHostFixture::install_other_kache`]:
//!   an executable named kache that is not the one on `PATH`.
//! - `cargo-probe/` — only after [`KacheHostFixture::cargo_check_runs_kache`]:
//!   the scratch crate, its target directory, and the logging `kache` stub
//!   real Cargo is given.
//! - `.cargo/` — only after [`KacheHostFixture::write_parent_config_wrapper`]:
//!   a Cargo config in the checkout's parent directory.
//! - `fake-host/`, `volume/` — only after [`KacheHostFixture::emulate_windows`]:
//!   the state its fake Git Bash tools answer from, and the directory that
//!   stands in for the `B:` drive root. `fake-host/off-device` alone also
//!   after [`KacheHostFixture::place_off_device`] on the native host.
//!
//! Repository files are passed in through [`RepoInputs`] because the CI
//! test-input index only attributes a `repo_root().join("…")` read to the
//! binary that spells it; a literal here would name this helper module,
//! which holds no tests.

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

/// The repository files a [`KacheHostFixture`] reproduces in its checkout.
pub struct RepoInputs {
    pub justfile: PathBuf,
    pub just_dir: PathBuf,
    /// The scripts the kache recipes run, each linked into the checkout's
    /// `scripts/` under its own name. Listed one by one, rather than linking
    /// the directory, so a recipe that starts running another script fails
    /// here, and so the test-input index sees each path in the binary that
    /// spells it (`source-inputs` in this package's manifest).
    pub scripts: Vec<PathBuf>,
    pub floor_file: PathBuf,
}

/// The daemon state the fake `kache daemon --json` reports.
pub struct FakeDaemon {
    pub service_installed: bool,
    pub running: bool,
    /// The running daemon's version; `None` when it is not reachable.
    pub version: Option<String>,
}

/// Worktree-base settings `wt` refuses, one per rule an L1 case pins.
#[derive(Clone, Copy, Debug)]
pub enum InvalidWorktreeBase {
    MissingWtPath,
    WtGitRepository,
    MalformedConfig,
    MissingConfigBaseDir,
}

pub struct KacheHostFixture {
    root: TempDir,
    worktree_base: Option<PathBuf>,
    rustc_wrapper_env: Option<String>,
    cargo_build_rustc_wrapper_env: Option<String>,
    kache_config_env: Option<PathBuf>,
    windows: bool,
}

impl KacheHostFixture {
    /// A host with kache installed at the floor version, its store on the
    /// checkout's device and pinned (with `ignore_env = true`) in the user
    /// config, its daemon installed and running at the same version, `DYLD_*`
    /// passthrough working, no worktree base, no wrapper activated, and a
    /// clone probe that succeeds everywhere — healthy once a wrapper is
    /// activated.
    pub fn new(repo: &RepoInputs) -> Self {
        let root = tempfile::tempdir().expect("fixture root");
        let fixture = Self {
            root,
            worktree_base: None,
            rustc_wrapper_env: None,
            cargo_build_rustc_wrapper_env: None,
            kache_config_env: None,
            windows: false,
        };
        for dir in [
            "checkout/.github",
            "store",
            "home",
            "cargo-home",
            "tmp",
            "bin",
            "fake-kache",
            "home/.config/kache",
        ] {
            fs::create_dir_all(fixture.path(dir)).expect("fixture directory");
        }
        let checkout = fixture.checkout();
        fs::copy(&repo.justfile, checkout.join("justfile")).expect("copy justfile");
        fs::copy(&repo.floor_file, checkout.join(".github/kache-min-version"))
            .expect("copy floor file");
        symlink(&repo.just_dir, checkout.join("just")).expect("link just/");
        fs::create_dir_all(checkout.join("scripts")).expect("checkout scripts/");
        for script in &repo.scripts {
            let name = script.file_name().expect("a script file name");
            symlink(script, checkout.join("scripts").join(name)).expect("link a script");
        }
        symlink(host_tool("just"), fixture.path("bin/just")).expect("link just");

        let floor = fs::read_to_string(&repo.floor_file).expect("floor file");
        let floor = floor.trim();
        fixture.write_executable(
            "fake-kache/kache-bin",
            &format!(
                r#"#!/usr/bin/env bash
state='{state}'
printf '%s\n' "$*" >> "$state/invocations.log"
printf 'kache %s\n' "$*" >> "$state/commands.log"
case "${{1:-}}" in
    --version) echo "kache $(cat "$state/version")" ;;
    # kache's resolution with no KACHE_* store variable: the `local_store` of
    # the config file it loads (a non-empty KACHE_CONFIG selects that file),
    # else the built-in default, the fixture store.
    doctor)
        config="${{KACHE_CONFIG:-${{XDG_CONFIG_HOME:-$HOME/.config}}/kache/config.toml}}"
        store="$(sed -n 's/^local_store = "\(.*\)"$/\1/p' "$config" 2> /dev/null | head -1)"
        store="${{store:-{store}}}"
        [[ -e "$state/doctor-store" ]] && store="$(cat "$state/doctor-store")"
        printf '{{"version":"%s","checks":[{{"label":"Cache dir","pass":true,"detail":"%s"}}]}}\n' "$(cat "$state/version")" "$store" ;;
    daemon)
        case "${{2:-}}" in
            --json) cat "$state/daemon.json" ;;
            install) python3 "$state/daemon.py" "$state/daemon.json" install - ;;
            start | restart)
                if [[ "$2" == restart && -e "$state/fail-daemon-restart" ]]; then
                    echo "fake kache: daemon restart failed" >&2
                    exit 1
                fi
                if [[ "$2" == restart && -e "$state/stale-daemon-restart" ]]; then
                    exit 0
                fi
                python3 "$state/daemon.py" "$state/daemon.json" "$2" "$(cat "$state/version")" ;;
            *) exit 64 ;;
        esac ;;
    # `kache rustc ARGS` runs the `rustc` on PATH — the passthrough probe's
    # stub compiler. dyld strips DYLD_* before a
    # bash script starts, so no bash fake can forward one: `forward` answers
    # with the value scripts/kache-host.sh's probe exports, as a kache that
    # forwards it would; `strip` behaves like a hardened-runtime kache.
    rustc)
        if [[ "${{2:-}}" == DYLD_FALLBACK_LIBRARY_PATH && "$(cat "$state/passthrough")" == forward ]]; then
            echo ".kache-host-dyld-survivor"
            exit 0
        fi
        exec "$@" ;;
    *) echo "fake kache: unsupported invocation: $*" >&2; exit 64 ;;
esac
"#,
                state = fixture.path("fake-kache").display(),
                store = fixture.store().display(),
            ),
        );
        fs::write(fixture.path("fake-kache/daemon.py"), FAKE_DAEMON_PY).expect("write fake daemon");
        fixture.install_fake_tools(&host_tool("python3"));
        fs::copy(
            fixture.path("fake-kache/kache-bin"),
            fixture.path("bin/kache"),
        )
        .expect("install fake kache");
        fixture.set_daemon_service_config(&fixture.kache_config());
        fixture.set_kache_version(floor);
        fixture.set_release(floor);
        fixture.set_daemon(&FakeDaemon {
            service_installed: true,
            running: true,
            version: Some(floor.to_owned()),
        });
        fixture.set_passthrough(true);
        fixture.write_kache_config(&format!(
            "[cache]\nlocal_store = \"{}\"\nignore_env = true\n",
            fixture.store().display()
        ));
        fixture.fail_clones_into(&[]);
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    pub fn checkout(&self) -> PathBuf {
        self.path("checkout")
    }

    pub fn home(&self) -> PathBuf {
        self.path("home")
    }

    pub fn store(&self) -> PathBuf {
        self.path("store")
    }

    pub fn cargo_home(&self) -> PathBuf {
        self.path("cargo-home")
    }

    /// The version `kache --version` and `kache doctor --json` report. The
    /// daemon's version is set separately ([`Self::set_daemon`]).
    pub fn set_kache_version(&self, version: &str) {
        fs::write(self.path("fake-kache/version"), version).expect("write fake version");
    }

    pub fn set_daemon(&self, daemon: &FakeDaemon) {
        let document = serde_json::json!({
            "schema_version": 1,
            "command": "daemon-status",
            "daemon_running": daemon.running,
            "service_installed": daemon.service_installed,
            "socket": self.store().join("daemon.sock"),
            "daemon_version": daemon.version,
            "daemon_config_path": daemon.running.then(|| self.daemon_service_config()),
        });
        fs::write(self.path("fake-kache/daemon.json"), document.to_string())
            .expect("write fake daemon state");
    }

    /// The config file the daemon loads whenever it starts — the one its
    /// service environment selects. [`Self::new`] makes it
    /// [`Self::kache_config`]; the running daemon reports it as
    /// `daemon_config_path`. Takes effect for a daemon running now too.
    pub fn set_daemon_service_config(&self, path: &Path) {
        fs::write(
            self.path("fake-kache/service-config"),
            path.to_str().expect("utf-8 config path"),
        )
        .expect("write fake service config");
        if let Ok(state) = fs::read_to_string(self.path("fake-kache/daemon.json")) {
            let mut document: serde_json::Value =
                serde_json::from_str(&state).expect("daemon state parses");
            if document["daemon_running"] == true {
                document["daemon_config_path"] = path.to_str().into();
                fs::write(self.path("fake-kache/daemon.json"), document.to_string())
                    .expect("write fake daemon state");
            }
        }
    }

    fn daemon_service_config(&self) -> String {
        fs::read_to_string(self.path("fake-kache/service-config")).expect("fake service config")
    }

    /// Makes `kache doctor --json` report `store` whatever the config says —
    /// a kache resolving its store from a source other than the pin.
    pub fn set_doctor_store(&self, store: &Path) {
        fs::write(
            self.path("fake-kache/doctor-store"),
            store.to_str().expect("utf-8 store path"),
        )
        .expect("write fake doctor store");
    }

    /// Exports `KACHE_CONFIG` to the recipe — kache's config-file selector,
    /// which the fake doctor honors by resolving that file's `local_store`.
    pub fn set_kache_config_env(&mut self, path: PathBuf) {
        self.kache_config_env = Some(path);
    }

    /// Whether a `DYLD_*` variable reaches the compiler the fake wraps
    /// (`false` is the hardened-runtime binary the macOS re-sign cures).
    pub fn set_passthrough(&self, forwards: bool) {
        let mode = if forwards { "forward" } else { "strip" };
        fs::write(self.path("fake-kache/passthrough"), mode).expect("write fake passthrough");
    }

    /// The kache user config file, at the path kache and `_ensure-kache`
    /// resolve under the fixture's `XDG_CONFIG_HOME`.
    pub fn kache_config(&self) -> PathBuf {
        self.path("home/.config/kache/config.toml")
    }

    /// Replaces the whole kache user config.
    pub fn write_kache_config(&self, contents: &str) {
        fs::write(self.kache_config(), contents).expect("write kache config");
    }

    /// Every argument list the fake `kache` was invoked with, one per line.
    pub fn kache_invocations(&self) -> String {
        fs::read_to_string(self.path("fake-kache/invocations.log")).unwrap_or_default()
    }

    /// Activates kache host-wide the way `_ensure-kache` does: a
    /// `rustc-wrapper` line in `$CARGO_HOME/config.toml`.
    pub fn activate_wrapper(&self) {
        self.write_config_wrapper("kache");
    }

    /// Writes `$CARGO_HOME/config.toml` with `[build] rustc-wrapper` set to
    /// `value` — `""` is the neutralized state `_ensure-kache` leaves behind.
    pub fn write_config_wrapper(&self, value: &str) {
        fs::write(
            self.cargo_home().join("config.toml"),
            format!("[build]\nrustc-wrapper = \"{value}\"\n"),
        )
        .expect("write Cargo config");
    }

    /// Writes the legacy extensionless `$CARGO_HOME/config`. While it exists
    /// Cargo reads it and ignores `$CARGO_HOME/config.toml`.
    pub fn write_legacy_cargo_config(&self, contents: &str) {
        fs::write(self.cargo_home().join("config"), contents).expect("write legacy Cargo config");
    }

    /// Runs a real `cargo check --offline` of an empty scratch crate against
    /// this host's `$CARGO_HOME`, with a logging pass-through `kache` first
    /// on `PATH`, and reports whether Cargo ran that wrapper. `None` when
    /// cargo cannot be started. Panics when the build fails, so "the wrapper
    /// never ran" cannot be a broken build in disguise.
    pub fn cargo_check_runs_kache(&self) -> Option<bool> {
        let probe = self.path("cargo-probe");
        fs::create_dir_all(probe.join("stub")).expect("probe stub directory");
        fs::create_dir_all(probe.join("project/src")).expect("probe project directory");
        let log = probe.join("kache.log");
        let stub = probe.join("stub/kache");
        fs::write(
            &stub,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec \"$@\"\n",
                log.display()
            ),
        )
        .expect("write pass-through kache");
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
        fs::write(
            probe.join("project/Cargo.toml"),
            "[package]\nname = \"wrapper-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .expect("write probe manifest");
        fs::write(probe.join("project/src/lib.rs"), "").expect("write probe lib");
        let mut path = vec![probe.join("stub")];
        path.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let output = Command::new("cargo")
            .args(["check", "--offline", "--quiet"])
            .current_dir(probe.join("project"))
            .env("PATH", std::env::join_paths(path).expect("probe PATH"))
            .env("CARGO_HOME", self.cargo_home())
            .env("CARGO_TARGET_DIR", probe.join("target"))
            .env_remove("RUSTC_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")
            .env_remove("RUSTC")
            .env_remove("CARGO_BUILD_RUSTC")
            .output()
            .ok()?;
        assert!(
            output.status.success(),
            "the probe build must succeed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Some(fs::read_to_string(&log).is_ok_and(|invocations| !invocations.is_empty()))
    }

    /// Writes `.cargo/config.toml` with `[build] rustc-wrapper` set to `value`
    /// in the checkout's parent directory, which Cargo reads from the checkout
    /// as an ancestor config; returns that file's canonical path (the helper
    /// names it by the physical working directory).
    pub fn write_parent_config_wrapper(&self, value: &str) -> PathBuf {
        let dir = self.path(".cargo");
        fs::create_dir_all(&dir).expect("parent .cargo directory");
        let config = dir.join("config.toml");
        fs::write(&config, format!("[build]\nrustc-wrapper = \"{value}\"\n"))
            .expect("write parent Cargo config");
        fs::canonicalize(config).expect("canonical parent Cargo config")
    }

    /// Writes the checkout's own `.cargo/config.toml` with `[build]
    /// rustc-wrapper` set to `value` — a repository config, which outranks
    /// `$CARGO_HOME`; returns that file's canonical path.
    pub fn write_checkout_config_wrapper(&self, value: &str) -> PathBuf {
        let dir = self.checkout().join(".cargo");
        fs::create_dir_all(&dir).expect("checkout .cargo directory");
        let config = dir.join("config.toml");
        fs::write(&config, format!("[build]\nrustc-wrapper = \"{value}\"\n"))
            .expect("write checkout Cargo config");
        fs::canonicalize(config).expect("canonical checkout Cargo config")
    }

    /// Exports `CARGO_BUILD_RUSTC_WRAPPER` to the recipe, which outranks
    /// every config file but not `RUSTC_WRAPPER`.
    pub fn set_cargo_build_rustc_wrapper_env(&mut self, value: &str) {
        self.cargo_build_rustc_wrapper_env = Some(value.to_owned());
    }

    /// Exports `RUSTC_WRAPPER` to the recipe; `""` is set-but-empty, which
    /// Cargo reads as "no wrapper" over any config file.
    pub fn set_rustc_wrapper_env(&mut self, value: &str) {
        self.rustc_wrapper_env = Some(value.to_owned());
    }

    /// Puts the logging fakes of `cargo`, `cargo-binstall`, `codesign`,
    /// `sleep`, and the `python3` shim on the fixture `PATH`.
    ///
    /// `cargo binstall` installs `fake-kache/kache-bin` at the `release`
    /// version when kache is absent or another version, as a hardened build
    /// (passthrough `strip`), and is a no-op otherwise, like the real one.
    /// `codesign` on the installed kache is the ad hoc re-sign that makes it
    /// forward `DYLD_*`; on any other file it runs the host's codesign (the
    /// passthrough probe signs its stub compiler).
    fn install_fake_tools(&self, host_python: &Path) {
        let state = self.path("fake-kache");
        let state = state.display();
        let bin = self.path("bin");
        let bin = bin.display();
        self.write_executable(
            "bin/cargo",
            &format!(
                r#"#!/usr/bin/env bash
state='{state}'
printf 'cargo %s\n' "$*" >> "$state/commands.log"
case "${{1:-}}" in
    binstall)
        release="$(cat "$state/release")"
        if [[ -x '{bin}/kache' && "$(cat "$state/version")" == "$release" ]]; then
            echo "fake binstall: kache $release is already the latest"
        else
            /bin/cp "$state/kache-bin" '{bin}/kache'
            printf '%s' "$release" > "$state/version"
            printf strip > "$state/passthrough"
            echo "fake binstall: installed kache $release"
        fi ;;
    install) ;;
    *) exit 64 ;;
esac
"#
            ),
        );
        self.write_executable(
            "bin/cargo-binstall",
            &format!(
                "#!/usr/bin/env bash\nprintf 'cargo-binstall %s\\n' \"$*\" >> '{state}/commands.log'\n"
            ),
        );
        self.write_executable(
            "bin/codesign",
            &format!(
                r#"#!/usr/bin/env bash
state='{state}'
printf 'codesign %s\n' "$*" >> "$state/commands.log"
if [[ "${{@: -1}}" == '{bin}/kache' ]]; then
    printf forward > "$state/passthrough"
    exit 0
fi
exec /usr/bin/codesign "$@"
"#
            ),
        );
        self.write_executable("bin/sleep", "#!/usr/bin/env bash\nexit 0\n");
        self.write_executable(
            "bin/python3",
            &format!(
                r#"#!/usr/bin/env bash
case "${{1:-}}" in
    *kache-config-merge.py) printf 'config-merge %s\n' "${{*:2}}" >> '{state}/commands.log' ;;
esac
exec '{python}' "$@"
"#,
                python = host_python.display(),
            ),
        );
    }

    /// The version the fake `cargo binstall` treats as the latest release.
    pub fn set_release(&self, version: &str) {
        fs::write(self.path("fake-kache/release"), version).expect("write fake release");
    }

    /// Removes the kache binary, as on a host that never installed it; the
    /// fake `cargo binstall` puts it back.
    pub fn uninstall_kache(&self) {
        fs::remove_file(self.kache_bin()).expect("remove fake kache");
    }

    /// Where the fake `kache` is installed (and absent after
    /// [`Self::uninstall_kache`]).
    pub fn kache_bin(&self) -> PathBuf {
        self.path("bin/kache")
    }

    /// Installs a second executable named `kache` outside the fixture `PATH`
    /// — a copy of the fake, so it would answer like kache, but not the
    /// binary on `PATH` whose version and passthrough the recipes check —
    /// and returns its path.
    pub fn install_other_kache(&self) -> PathBuf {
        fs::create_dir_all(self.path("other-kache")).expect("other kache directory");
        let other = self.path("other-kache/kache");
        fs::copy(self.path("fake-kache/kache-bin"), &other).expect("install other kache");
        other
    }

    /// The version the fake daemon runs now, as `kache daemon --json` reports.
    pub fn daemon_version(&self) -> Option<String> {
        let state = fs::read_to_string(self.path("fake-kache/daemon.json")).expect("daemon state");
        let document: serde_json::Value =
            serde_json::from_str(&state).expect("daemon state parses");
        document["daemon_version"].as_str().map(str::to_owned)
    }

    /// Makes `kache daemon restart` exit 1 and leave the daemon unchanged.
    pub fn fail_daemon_restart(&self) {
        fs::write(self.path("fake-kache/fail-daemon-restart"), "").expect("flag restart failure");
    }

    /// Makes `kache daemon restart` exit 0 and leave the daemon unchanged —
    /// a service manager that reports the restart done while the old daemon
    /// is still the one answering.
    pub fn keep_daemon_on_restart(&self) {
        fs::write(self.path("fake-kache/stale-daemon-restart"), "").expect("flag stale restart");
    }

    /// Every fake tool invocation, in order, as `<tool> <args>`.
    pub fn commands(&self) -> Vec<String> {
        fs::read_to_string(self.path("fake-kache/commands.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Empties [`Self::commands`] and [`Self::kache_invocations`], so a
    /// second run's log holds only that run.
    pub fn clear_command_logs(&self) {
        for log in ["fake-kache/commands.log", "fake-kache/invocations.log"] {
            let _ = fs::remove_file(self.path(log));
        }
    }

    /// Makes `just install-kache` fail — the first pre-activation step of
    /// `_ensure-kache` on a qualifying host — through a `cargo` and
    /// `cargo-binstall` that exit 1.
    pub fn fail_kache_install(&self) {
        for tool in ["bin/cargo", "bin/cargo-binstall"] {
            self.write_executable(tool, "#!/usr/bin/env bash\nexit 1\n");
        }
    }

    /// Makes `$CARGO_HOME/config.toml` and its directory read-only, so no
    /// write, atomic rename, or backup can land there. Returns `false` when
    /// the process can still write (root), which the caller must treat as a
    /// skip. Permissions are restored on drop so the fixture can be removed.
    pub fn make_cargo_home_read_only(&self) -> bool {
        let config = self.cargo_home().join("config.toml");
        fs::set_permissions(&config, fs::Permissions::from_mode(0o444)).expect("chmod config");
        fs::set_permissions(self.cargo_home(), fs::Permissions::from_mode(0o555))
            .expect("chmod cargo home");
        fs::write(self.cargo_home().join(".write-probe"), "").is_err()
    }

    /// Configures a worktree base (through `WT`) on the checkout's device and
    /// returns it.
    pub fn configure_worktree_base(&mut self) -> PathBuf {
        let base = self.path("worktree-base");
        fs::create_dir_all(&base).expect("worktree base");
        self.worktree_base = Some(base.clone());
        base
    }

    /// Configures one worktree-base setting `wt` refuses
    /// (`worktree/lib/src/config.rs` `resolve_base_dir`) and returns the
    /// reason and path `kache-host.sh` must report for it.
    pub fn configure_invalid_worktree_base(
        &mut self,
        setting: InvalidWorktreeBase,
    ) -> (&'static str, PathBuf) {
        let config = self.path("home/.worktree.json");
        match setting {
            InvalidWorktreeBase::MissingWtPath => {
                let base = self.path("missing-worktree-base");
                self.worktree_base = Some(base.clone());
                ("wt-path-missing", base)
            }
            InvalidWorktreeBase::WtGitRepository => {
                let base = self.path("repository-base");
                fs::create_dir_all(base.join(".git")).expect("git directory");
                self.worktree_base = Some(base.clone());
                ("wt-path-is-a-git-repository", base)
            }
            InvalidWorktreeBase::MalformedConfig => {
                fs::write(&config, "{\"base_dir\": ").expect("write worktree config");
                ("config-malformed", config)
            }
            InvalidWorktreeBase::MissingConfigBaseDir => {
                let base = self.path("missing-config-base");
                self.write_worktree_config(&base);
                ("config-base-dir-missing", base)
            }
        }
    }

    /// Writes `~/.worktree.json` naming `base_dir`, which `wt` reads when
    /// `WT` is empty.
    pub fn write_worktree_config(&self, base_dir: &Path) {
        let contents = serde_json::json!({ "base_dir": base_dir }).to_string();
        fs::write(self.path("home/.worktree.json"), contents).expect("write worktree config");
    }

    /// Sets `WT` to `value` verbatim, without creating anything.
    pub fn set_wt_env(&mut self, value: PathBuf) {
        self.worktree_base = Some(value);
    }

    /// Replaces `cp` so a clone (`cp -c` on macOS, `cp --reflink=always`
    /// elsewhere) whose destination sits directly in a directory with one of
    /// `dir_names` fails, as it does on a filesystem that cannot clone. Every
    /// other clone succeeds as a plain copy, whatever the host filesystem, and
    /// every non-clone `cp` runs `/bin/cp` unchanged.
    pub fn fail_clones_into(&self, dir_names: &[&str]) {
        let failing = dir_names.join("|");
        let case_arm = if failing.is_empty() {
            String::new()
        } else {
            format!("        {failing}) exit 1 ;;\n")
        };
        self.write_executable(
            "bin/cp",
            &format!(
                r#"#!/usr/bin/env bash
args=() clone=0
for arg in "$@"; do
    case "$arg" in
        -c | --reflink=always) clone=1 ;;
        *) args+=("$arg") ;;
    esac
done
if [[ $clone -eq 1 ]]; then
    destination="${{args[${{#args[@]}}-1]}}"
    printf 'clone %s\n' "$destination" >> '{log}'
    case "$(basename "$(dirname "$destination")")" in
{case_arm}    esac
fi
exec /bin/cp "${{args[@]}}"
"#,
                log = self.path("fake-kache/commands.log").display(),
            ),
        );
    }

    /// Turns the fixture into a native-Windows Git Bash host whose checkout is
    /// on drive `B:` (ReFS unless [`Self::set_windows_checkout_fstype`] says
    /// otherwise), by putting fakes of the host tools `scripts/kache-host.sh`
    /// and the recipes ask on `PATH`: `uname -s` answers `MINGW64_NT`,
    /// `powershell.exe` answers `Get-Volume`'s filesystem type, `cygpath`
    /// maps `B:/` to [`Self::windows_volume_root`] and every other path to
    /// itself, and `stat -c %d` answers the host's device id except for the
    /// paths [`Self::place_off_device`] moved to a second drive `C:`. `cargo`
    /// and `cargo-binstall` succeed without doing anything, so `install-kache`
    /// leaves the fake `kache` in place. `APPDATA` is `home/.config`, keeping
    /// [`Self::kache_config`] the config file, and `LOCALAPPDATA` is
    /// [`Self::local_app_data`].
    pub fn emulate_windows(&mut self) {
        self.windows = true;
        fs::create_dir_all(self.path("fake-host")).expect("fake host state");
        fs::create_dir_all(self.windows_volume_root()).expect("B: drive root");
        fs::create_dir_all(self.local_app_data()).expect("LOCALAPPDATA");
        self.set_windows_checkout_fstype("ReFS");
        self.place_off_device(&[]);
        let state = self.path("fake-host");
        let state = state.display();
        let volume = self.windows_volume_root();
        let volume = volume.display();
        self.write_executable(
            "bin/uname",
            r#"#!/usr/bin/env bash
[[ "${1:-}" == -s ]] && { echo MINGW64_NT-10.0-26100; exit 0; }
exec /usr/bin/uname "$@"
"#,
        );
        self.write_executable(
            "bin/powershell.exe",
            &format!(
                r#"#!/usr/bin/env bash
case "$*" in
    *"-DriveLetter B)"*) cat '{state}/fstype' ;;
    *"-DriveLetter "*) echo NTFS ;;
    *) exit 1 ;;
esac
"#
            ),
        );
        let off_device = OFF_DEVICE_SH;
        self.write_executable(
            "bin/cygpath",
            &format!(
                r#"#!/usr/bin/env bash
STATE='{state}'
{off_device}
mode=identity
case "${{1:-}}" in -u | -w | -m) mode="$1"; shift ;; esac
path="$1"
if [[ "$mode" == -w ]]; then
    if off_device "$path"; then echo "C:$path"; else echo "B:$path"; fi
    exit 0
fi
case "$path" in
    B:/ | 'B:\') echo '{volume}/' ;;
    ?:/ | ?:\\) exit 1 ;;
    *) printf '%s\n' "$path" ;;
esac
"#
            ),
        );
        self.write_executable(
            "bin/stat",
            &format!(
                r#"#!/usr/bin/env bash
STATE='{state}'
{off_device}
[[ "${{1:-}}" == -c && "${{2:-}}" == %d && $# -eq 3 ]] || exec /usr/bin/stat "$@"
[[ -e "$3" ]] || exec /usr/bin/stat "$@"
if off_device "$3"; then echo 999999; exit 0; fi
/usr/bin/stat -c %d "$3" 2> /dev/null || /usr/bin/stat -f %d "$3"
"#
            ),
        );
        for tool in ["bin/cargo", "bin/cargo-binstall"] {
            self.write_executable(tool, "#!/usr/bin/env bash\nexit 0\n");
        }
    }

    /// The directory the emulated `B:` drive root resolves to.
    pub fn windows_volume_root(&self) -> PathBuf {
        self.path("volume")
    }

    /// The emulated `%LOCALAPPDATA%`, whose `kache` directory is kache's
    /// default store on Windows.
    pub fn local_app_data(&self) -> PathBuf {
        self.path("home/AppData/Local")
    }

    /// The filesystem type `Get-Volume` reports for the emulated `B:` drive.
    pub fn set_windows_checkout_fstype(&self, fstype: &str) {
        fs::write(self.path("fake-host/fstype"), fstype).expect("write fake fstype");
    }

    /// Moves `paths` (and everything under them) to another device. Under
    /// [`Self::emulate_windows`] that is a second emulated drive `C:`, NTFS
    /// with its own device id — where `%LOCALAPPDATA%` and kache's default
    /// store live on a host whose Dev Drive holds only the checkout. On the
    /// native host a `stat` shim answers a foreign device id for them, as for
    /// a store on another volume.
    pub fn place_off_device(&self, paths: &[&Path]) {
        let lines: String = paths
            .iter()
            .map(|path| format!("{}\n", path.display()))
            .collect();
        fs::create_dir_all(self.path("fake-host")).expect("fake host state");
        fs::write(self.path("fake-host/off-device"), lines).expect("write off-device paths");
        if !self.windows {
            let state = self.path("fake-host");
            self.write_executable(
                "bin/stat",
                &format!(
                    r#"#!/usr/bin/env bash
STATE='{state}'
{OFF_DEVICE_SH}
if [[ ( "${{1:-}}" == -c || "${{1:-}}" == -f ) && "${{2:-}}" == %d && $# -eq 3 && -e "$3" ]] && off_device "$3"; then
    echo 999999
    exit 0
fi
exec /usr/bin/stat "$@"
"#,
                    state = state.display(),
                ),
            );
        }
    }

    /// Runs `just <recipe>` in the checkout with the fixture's environment
    /// only: the parent environment is cleared, so no `RUSTC_WRAPPER` or
    /// `CARGO_BUILD_RUSTC_WRAPPER` (other than the fixture's own setters'),
    /// `KACHE_*`, `WT`, or host `PATH` leaks in.
    pub fn just(&self, recipe: &str) -> Output {
        self.just_command(recipe).output().expect("just must spawn")
    }

    /// The [`Self::just`] command, unspawned — for a test that attaches it to
    /// a PTY.
    pub fn just_command(&self, recipe: &str) -> Command {
        let mut command = self.isolated_command(host_tool("just"));
        command.arg(recipe);
        command
    }

    /// Runs `scripts/kache-host.sh <subcommand>` in the checkout with the
    /// same isolated environment as [`Self::just`].
    pub fn host_script(&self, subcommand: &str) -> Output {
        self.isolated_command(PathBuf::from("bash"))
            .arg("scripts/kache-host.sh")
            .arg(subcommand)
            .output()
            .expect("kache-host.sh must spawn")
    }

    fn isolated_command(&self, program: PathBuf) -> Command {
        let path = format!("{}:/usr/bin:/bin", self.path("bin").display());
        let mut command = Command::new(program);
        command
            .current_dir(self.checkout())
            .env_clear()
            .env("PATH", path)
            .env("HOME", self.path("home"))
            .env("CARGO_HOME", self.cargo_home())
            .env("TMPDIR", self.path("tmp"))
            .env("XDG_CONFIG_HOME", self.path("home/.config"))
            .env("XDG_CACHE_HOME", self.path("home/.cache"))
            .env("GIT_CEILING_DIRECTORIES", self.root.path())
            .env(
                "WT",
                self.worktree_base
                    .as_deref()
                    .map(Path::as_os_str)
                    .unwrap_or_default(),
            );
        if let Some(wrapper) = &self.rustc_wrapper_env {
            command.env("RUSTC_WRAPPER", wrapper);
        }
        if let Some(wrapper) = &self.cargo_build_rustc_wrapper_env {
            command.env("CARGO_BUILD_RUSTC_WRAPPER", wrapper);
        }
        if let Some(config) = &self.kache_config_env {
            command.env("KACHE_CONFIG", config);
        }
        if self.windows {
            command
                .env("APPDATA", self.path("home/.config"))
                .env("LOCALAPPDATA", self.local_app_data());
        }
        command
    }

    fn write_executable(&self, relative: &str, body: &str) {
        let path = self.path(relative);
        fs::write(&path, body).expect("write fake executable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
}

impl Drop for KacheHostFixture {
    fn drop(&mut self) {
        let _ = fs::set_permissions(self.cargo_home(), fs::Permissions::from_mode(0o755));
    }
}

/// `name` from the test's own `PATH`, resolved here because the child's
/// `PATH` deliberately omits the directories it is installed in.
fn host_tool(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("`{name}` must be on PATH to run the kache recipes"))
}

/// `off_device PATH`: whether PATH is, or lies under, a prefix listed in
/// `$STATE/off-device` ([`KacheHostFixture::place_off_device`]).
const OFF_DEVICE_SH: &str = r#"off_device() {
    local prefix
    while IFS= read -r prefix; do
        [[ -n "$prefix" && ( "$1" == "$prefix" || "$1" == "$prefix"/* ) ]] && return 0
    done < "$STATE/off-device"
    return 1
}"#;

/// The fake daemon's state transitions over `daemon.json`:
/// `daemon.py FILE install -` registers the service without starting it;
/// `daemon.py FILE start|restart VERSION` runs the daemon at the binary's
/// VERSION, loading the config named in `service-config` beside FILE, and
/// fails when no service is installed.
const FAKE_DAEMON_PY: &str = r#"import json, os, sys
path, action, version = sys.argv[1:4]
with open(path) as handle:
    state = json.load(handle)
if action == "install":
    state["service_installed"] = True
else:
    if not state.get("service_installed"):
        sys.exit(1)
    state["daemon_running"] = True
    state["daemon_version"] = version
    with open(os.path.join(os.path.dirname(path), "service-config")) as handle:
        state["daemon_config_path"] = handle.read()
with open(path, "w") as handle:
    json.dump(state, handle)
"#;
