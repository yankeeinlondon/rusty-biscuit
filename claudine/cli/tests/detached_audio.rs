use std::fs;

mod audio_spool {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../test-support/locked_audio_spool.rs"));
}
use audio_spool::LockedAudioSpool;

mod common;
use common::{CliProcessFixture, write};

#[test]
fn handle_human_in_the_loop_leaves_durable_doorbell_job_after_exit() {
    let fixture = CliProcessFixture::named("claudine-handle-detached-audio");
    let home = fixture.home();
    let spool = fixture.workspace_path().join("spool");


    let config = serde_json::json!({
        "preferred_agent": "claude",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false },
        "actions": {
            "human_in_the_loop": [{
                "type": "sound_effect",
                "effect": "doorbell-2",
                "volume": 0.0
            }]
        }
    });
    write(
        &home.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    let _audio_spool = LockedAudioSpool::new(&spool);

    let payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "AskUserQuestion",
        "session_id": "detached-doorbell"
    })
    .to_string();
    let output = fixture
        .command()
        .env("PLAYA_SPOOL_DIR", &spool)
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
    assert_eq!(envelope["payload"]["playback"]["volume"], 0.0);
    _audio_spool.clear_pending().unwrap();
}
