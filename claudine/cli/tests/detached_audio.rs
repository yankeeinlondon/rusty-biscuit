use std::fs::{self, OpenOptions};

use fs4::fs_std::FileExt as _;

mod common;
use common::{TestWorkspace, write};

#[test]
fn handle_human_in_the_loop_leaves_durable_doorbell_job_after_exit() {
    let workspace = TestWorkspace::named("claudine-handle-detached-audio");
    let home = workspace.path().join("home");
    let spool = workspace.path().join("spool");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir(&spool).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&spool, fs::Permissions::from_mode(0o700)).unwrap();
    }

    let config = serde_json::json!({
        "preferred_agent": "claude",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false },
        "actions": {
            "human_in_the_loop": [{
                "type": "sound_effect",
                "effect": "doorbell-2"
            }]
        }
    });
    write(
        &home.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let worker = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(spool.join("worker.lock"))
        .unwrap();
    worker.lock_exclusive().unwrap();

    let payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "AskUserQuestion",
        "session_id": "detached-doorbell"
    })
    .to_string();
    let output = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .current_dir(workspace.path())
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("APPDATA", &home)
        .env("LOCALAPPDATA", &home)
        .env("PLAYA_SPOOL_DIR", &spool)
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("NO_COLOR", "1")
        .env_remove("PLAYA_DRY_RUN")
        .args(["handle", "human_in_the_loop", "--provider", "claude"])
        .write_stdin(payload)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "handle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let pending = fs::read_dir(&spool)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().ends_with(".pending.json"))
        .expect("doorbell job should remain durable after claudine exits");
    let envelope: serde_json::Value =
        serde_json::from_slice(&fs::read(pending.path()).unwrap()).unwrap();
    assert_eq!(envelope["payload"]["state"], "ready");
    assert_eq!(envelope["payload"]["kind"], "play_file");
    assert_eq!(envelope["sequence"], 1);
}
