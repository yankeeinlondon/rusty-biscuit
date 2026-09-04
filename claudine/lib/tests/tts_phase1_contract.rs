use std::{fs, path::PathBuf};

fn fix_artifact(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixes/2026-09-03-tts-not-finishing")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn detached_audio_protocol_decisions_are_ratified_and_versioned() {
    let spec = fix_artifact("spec.md");

    for required in [
        "phase_1_protocols_ratified: true",
        "**Ratified: option 3.**",
        "schema_version",
        "capability_version",
        "state: preparing",
        "PLAYA_SPOOL_WORKER",
        "PLAYA_DELEGATED_PLAY_WORKER",
        "BISCUIT_SPEAKS_PREPARATION_WORKER",
        "exactly ten minutes after",
        "unsupported version",
        "at-most-once",
    ] {
        assert!(
            spec.contains(required),
            "ratified detached-audio contract is missing `{required}`"
        );
    }
}

#[test]
fn regression_matrix_covers_every_phase_one_failure_category() {
    let baseline = fix_artifact("phase-1-baseline.md");

    for required in [
        "free_functions_take_native_route_when_available",
        "probe_audio_metadata_shipped_effect_corpus",
        "truncated_verdict_warns_once_and_returns_ok",
        "mpv_args_upmix_only_mono",
        "dry_run_detached_has_no_side_effects",
        "spool_publication_empty_handoff_cannot_strand",
        "spool_selects_lossless_enqueuer_executable",
        "preparing_timeout_and_failed_advance",
        "unsupported_versions_are_quarantined",
        "background_cache_miss_returns_after_reservation",
        "speak_result_playback_serde",
        "handle_audio_survives_parent_exit",
        "real_native_plays_mono_wav_to_the_end",
        "host-only routing",
        "absent mono flag/report",
        "missing worker protocol",
        "blocking synthesis",
        "process-local task loss",
    ] {
        assert!(
            baseline.contains(required),
            "Phase 1 regression matrix is missing `{required}`"
        );
    }
}

#[test]
fn protocol_fixture_corpus_is_versioned_and_redacts_diagnostic_records() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixes/2026-09-03-tts-not-finishing/fixtures");
    let mut fixture_names = Vec::new();

    for entry in fs::read_dir(&fixture_dir).expect("protocol fixture directory must exist") {
        let path = entry.expect("fixture entry must be readable").path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        fixture_names.push(path.file_name().unwrap().to_string_lossy().into_owned());
        let source = fs::read_to_string(&path).expect("fixture must be readable");
        let value: serde_json::Value =
            serde_json::from_str(&source).expect("fixture must be valid JSON");
        assert!(value.get("schema_version").is_some());
        assert!(value.get("job_id").is_some());
        assert!(value.get("sequence").is_some());

        if path.file_name().unwrap() == "v1-journal-record.json" {
            for private_key in ["text", "args", "credentials", "path", "enqueuer"] {
                assert!(
                    !source.contains(&format!("\"{private_key}\"")),
                    "journal fixture leaked private key `{private_key}`"
                );
            }
        }
    }

    fixture_names.sort();
    assert_eq!(
        fixture_names,
        [
            "v1-delegated-report.json",
            "v1-journal-record.json",
            "v1-preparing-job.json",
            "v1-ready-job.json",
            "v2-unsupported-job.json",
        ]
    );
}
