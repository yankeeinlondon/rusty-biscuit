use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

struct ZedDmlsFixture {
    root: TempDir,
}

impl ZedDmlsFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        for directory in ["cwd", "home", "config", "cache", "tmp"] {
            std::fs::create_dir(root.path().join(directory)).unwrap();
        }
        // Windows resolves the per-user known folders beneath `USERPROFILE`
        // (`AppData\Local`, `AppData\Roaming`) and verifies they exist, so a
        // bare fixture home makes `dirs::data_local_dir()` return `None` and
        // the CLI report "unable to determine the required per-user
        // directory". `LOCALAPPDATA`/`APPDATA` are not consulted for that.
        if cfg!(windows) {
            for directory in ["AppData/Local", "AppData/Roaming"] {
                std::fs::create_dir_all(root.path().join("home").join(directory)).unwrap();
            }
        }
        Self { root }
    }

    fn path(&self) -> &std::path::Path {
        self.root.path()
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("zed-dmls").unwrap();
        command
            .current_dir(self.path().join("cwd"))
            .env("HOME", self.path().join("home"))
            .env("USERPROFILE", self.path().join("home"))
            .env("APPDATA", self.path().join("config"))
            .env("LOCALAPPDATA", self.path().join("config"))
            .env("XDG_CONFIG_HOME", self.path().join("config"))
            .env("XDG_CACHE_HOME", self.path().join("cache"))
            .env("TMPDIR", self.path().join("tmp"))
            .env("TEMP", self.path().join("tmp"))
            .env("TMP", self.path().join("tmp"))
            .env("PATH", "")
            .env_remove("HOMEDRIVE")
            .env_remove("HOMEPATH");
        for (key, _) in std::env::vars_os() {
            let name = key.to_string_lossy();
            if name.starts_with("GIT_")
                || name.starts_with("DARKMATTER_")
                || name.starts_with("DM_")
                || name.starts_with("MD_")
                || matches!(
                    name.as_ref(),
                    "COLUMNS"
                        | "LINES"
                        | "TERM"
                        | "COLORTERM"
                        | "COLORFGBG"
                        | "CLICOLOR_FORCE"
                        | "FORCE_COLOR"
                        | "NO_COLOR"
                        | "RUST_LOG"
                )
            {
                command.env_remove(key);
            }
        }
        command.env("GIT_CONFIG_NOSYSTEM", "1").env("NO_COLOR", "1");
        command
    }
}

#[test]
fn missing_command_uses_clap_exit_two() {
    let fixture = ZedDmlsFixture::new();
    fixture.command().assert().code(2);
}

#[test]
fn doctor_is_hermetic_with_path_overrides_and_plain_output() {
    let fixture = ZedDmlsFixture::new();
    fixture
        .command()
        .args([
            "doctor",
            "--plain",
            "--staging-dir",
            fixture.path().join("stage").to_str().unwrap(),
            "--zed-data-dir",
            fixture.path().join("zed").to_str().unwrap(),
            "--zed-log",
            fixture.path().join("Zed.log").to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL: dmls binary missing from PATH",
        ))
        .stdout(predicate::str::contains("\u{1b}[").not());
}

#[test]
fn conditional_doctor_is_silent_when_zed_is_absent() {
    let fixture = ZedDmlsFixture::new();
    fixture
        .command()
        .args([
            "doctor",
            "--if-zed-present",
            "--plain",
            "--zed-data-dir",
            fixture.path().join("missing-zed").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn stage_returns_three_when_manual_registration_remains() {
    let fixture = ZedDmlsFixture::new();
    let staging = fixture.path().join("stable/zed-dmls");
    fixture
        .command()
        .args([
            "stage",
            "--plain",
            "--staging-dir",
            staging.to_str().unwrap(),
            "--zed-data-dir",
            fixture.path().join("zed").to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains(staging.display().to_string()))
        .stdout(predicate::str::contains("manual registration required"))
        .stdout(predicate::str::contains("does not exist"));
    assert!(staging.join("extension.toml").exists());
    assert!(staging.join("extension.wasm").exists());
}

#[test]
fn stage_registers_and_returns_zero_when_zed_is_present() {
    let fixture = ZedDmlsFixture::new();
    let staging = fixture.path().join("stable/zed-dmls");
    let zed = fixture.path().join("zed");
    std::fs::create_dir_all(&zed).unwrap();
    fixture
        .command()
        .args([
            "stage",
            "--plain",
            "--staging-dir",
            staging.to_str().unwrap(),
            "--zed-data-dir",
            zed.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "registered it as Zed's `dmls` dev extension",
        ));
    let registration = zed.join("extensions/installed/dmls");
    assert_eq!(
        std::fs::canonicalize(&registration).unwrap(),
        std::fs::canonicalize(&staging).unwrap()
    );
}

#[test]
fn conditional_stage_is_silent_when_zed_is_absent() {
    let fixture = ZedDmlsFixture::new();
    let staging = fixture.path().join("stable/zed-dmls");
    fixture
        .command()
        .args([
            "stage",
            "--if-zed-present",
            "--plain",
            "--staging-dir",
            staging.to_str().unwrap(),
            "--zed-data-dir",
            fixture.path().join("missing-zed").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(!staging.exists());
}
