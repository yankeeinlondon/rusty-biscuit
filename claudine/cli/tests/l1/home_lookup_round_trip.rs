//! L1 contract: Claudine's user configuration is loaded from and saved to the
//! home the child process was launched with, through the one shared reader
//! (`biscuit_file::home_dir`): `HOME` on POSIX, `USERPROFILE` on native
//! Windows.
//!
//! Every launch drives the real `claudine` binary. `config set favorite-agent`
//! loads the user config (creating it headlessly when missing), changes it,
//! and saves it; a second launch proves the saved file is the one it reloads.
//! Home variables are set only on the child, never on this process.

use std::fs;
use std::path::Path;
use std::process::Output;

use crate::common;
use common::CliProcessFixture;

const CONFIG: &str = ".claudine/config.json";

fn set_favorite(command: &mut assert_cmd::Command, agent: &str) -> String {
    let output: Output = command
        .args(["config", "set", "favorite-agent", agent])
        .output()
        .expect("claudine spawns");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "claudine failed: {text}");
    text
}

fn favorite_in(config: &Path) -> String {
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(config).expect("config was saved")).expect("config is JSON");
    saved["preferred_agent"].as_str().unwrap_or_default().to_string()
}

#[test]
fn config_is_saved_under_the_child_home_and_reloaded_from_it() {
    let fixture = CliProcessFixture::named("home-round-trip");
    let config = fixture.home().join(CONFIG);
    assert!(!config.exists(), "precondition: no user config yet");

    let first = set_favorite(&mut fixture.command(), "codex");
    assert!(first.contains("Set favorite agent"), "{first}");
    assert!(
        first.contains(&biscuit_file::to_portable_string(&config)),
        "the reported path is the fixture config: {first}"
    );
    assert_eq!(favorite_in(&config), "codex");

    // A second launch reads the file the first one wrote.
    let second = set_favorite(&mut fixture.command(), "codex");
    assert!(second.contains("already set to Codex"), "{second}");

    // And a change saved there is what the next launch reloads.
    set_favorite(&mut fixture.command(), "gemini");
    assert_eq!(favorite_in(&config), "gemini");
    let fourth = set_favorite(&mut fixture.command(), "gemini");
    assert!(fourth.contains("already set to Gemini"), "{fourth}");
}

/// `HOME` and `USERPROFILE` disagree. POSIX follows `HOME`; native Windows
/// follows `USERPROFILE`, so setting only `HOME` there does not relocate home.
#[test]
fn conflicting_home_variables_resolve_to_the_platform_variable() {
    let fixture = CliProcessFixture::named("home-conflict");
    let posix_home = fixture.workspace_path().join("posix-home");
    let windows_home = fixture.workspace_path().join("windows-home");
    for home in [&posix_home, &windows_home] {
        fs::create_dir_all(home).unwrap();
    }
    let (chosen, ignored) = if cfg!(windows) {
        (&windows_home, &posix_home)
    } else {
        (&posix_home, &windows_home)
    };

    for _ in 0..2 {
        let mut command = fixture.command();
        command.env("HOME", &posix_home).env("USERPROFILE", &windows_home);
        set_favorite(&mut command, "codex");
    }

    assert_eq!(favorite_in(&chosen.join(CONFIG)), "codex");
    assert!(!ignored.join(".claudine").exists(), "the other variable's home is untouched");
    assert!(!fixture.home().join(".claudine").exists(), "the fixture default home is untouched");
}

/// A relative home is no home: it is neither used as a directory nor replaced
/// by a second lookup of the real profile. Claudine's no-home fallback for its
/// config path is a literal `~` relative to the working directory (existing
/// behavior, pinned here so a change to it is deliberate).
#[test]
fn a_relative_home_is_no_home_and_does_not_reach_the_real_profile() {
    let fixture = CliProcessFixture::named("home-relative");
    fs::create_dir_all(fixture.cwd().join("relhome")).unwrap();

    let mut command = fixture.command();
    command.env("HOME", "relhome").env("USERPROFILE", "relhome");
    let output = set_favorite(&mut command, "codex");

    assert!(!fixture.cwd().join("relhome").join(".claudine").exists(), "{output}");
    assert!(!fixture.home().join(".claudine").exists(), "{output}");
    assert_eq!(favorite_in(&fixture.cwd().join("~").join(CONFIG)), "codex", "{output}");
}
