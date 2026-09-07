use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use playa::detached::{
    DetachedPlayback, JobId, OsValue, PlaybackRouting, SpoolJob, enqueue, run_if_worker,
};
use playa::PlaybackOptions;
use playa::{AudioData, AudioFileFormat, AudioFormat, Codec, Playa};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn isolated_spool(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "playa-phase3-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after Unix epoch")
            .as_nanos()
    ))
}

#[test]
fn detached_protocol_round_trips_native_os_strings_and_playback_options() {
    let native = if cfg!(windows) {
        OsString::from(r"C:\Users\Ken Snyder\speech.wav")
    } else {
        OsString::from("/tmp/Ken Snyder/speech.wav")
    };
    let encoded = OsValue::from(native.clone());
    assert_eq!(encoded.to_os_string().expect("native encoding should decode"), native);

    let job = SpoolJob::PlayFile {
        path: encoded,
        playback: DetachedPlayback {
            options: PlaybackOptions::new()
                .with_speed(1.25)
                .with_volume(0.5)
                .with_channel("USB DAC"),
            routing: PlaybackRouting::ForceHost,
            ducking: None,
        },
        delete_after: false,
    };
    let json = serde_json::to_string(&job).expect("job should serialize");
    let restored: SpoolJob = serde_json::from_str(&json).expect("job should deserialize");
    assert_eq!(restored, job);
    assert!(json.contains("force_host"));
    assert!(json.contains("USB DAC"));
}

#[test]
fn dry_run_detached_has_no_side_effects_for_original_kokoro_shape() {
    let _guard = ENV_LOCK.lock().expect("environment lock should not be poisoned");
    let spool = isolated_spool("dry-run");
    // SAFETY: this integration-test process serializes its environment mutations.
    unsafe {
        std::env::set_var("PLAYA_SPOOL_DIR", &spool);
        std::env::set_var("PLAYA_DRY_RUN", "1");
    }

    assert_eq!(run_if_worker(), None);
    let job = SpoolJob::PlayFile {
        path: OsValue::from(OsString::from("kokoro-24khz-mono-5.97s.wav")),
        playback: DetachedPlayback::default(),
        delete_after: false,
    };
    let id = enqueue(job).expect("dry-run enqueue should succeed");

    // SAFETY: guarded as above.
    unsafe {
        std::env::remove_var("PLAYA_DRY_RUN");
        std::env::remove_var("PLAYA_SPOOL_DIR");
    }

    assert_eq!(id, JobId::dry_run());
    assert!(!spool.exists(), "dry-run must not create the spool root");
}

#[test]
fn delete_after_rejects_non_spool_owned_paths() {
    let _guard = ENV_LOCK.lock().expect("environment lock should not be poisoned");
    let spool = isolated_spool("delete-after");
    // SAFETY: this integration-test process serializes its environment mutations.
    unsafe {
        std::env::set_var("PLAYA_SPOOL_DIR", &spool);
    }
    assert_eq!(run_if_worker(), None);

    let job = SpoolJob::PlayFile {
        path: OsValue::from(OsString::from("/tmp/not-owned-by-playa.wav")),
        playback: DetachedPlayback::default(),
        delete_after: true,
    };
    let error = enqueue(job).expect_err("external files must never be delete-after jobs");

    // SAFETY: guarded as above.
    unsafe {
        std::env::remove_var("PLAYA_SPOOL_DIR");
    }
    assert!(error.to_string().contains("delete_after"));
    assert!(!spool.exists(), "validation should happen before spool creation");
}

#[test]
fn builder_dry_run_skips_missing_source_and_spool_work() {
    let _guard = ENV_LOCK.lock().expect("environment lock should not be poisoned");
    let spool = isolated_spool("builder-dry-run");
    // SAFETY: this integration-test process serializes its environment mutations.
    unsafe { std::env::set_var("PLAYA_SPOOL_DIR", &spool) };

    let id = Playa::from_data(
        AudioData::FilePath(PathBuf::from("missing-24khz-mono-kokoro.wav")),
        AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm)),
    )
    .dry_run()
    .play_detached()
    .expect("builder dry-run should succeed before reading the source");

    // SAFETY: guarded as above.
    unsafe { std::env::remove_var("PLAYA_SPOOL_DIR") };
    assert_eq!(id, JobId::dry_run());
    assert!(!spool.exists());
}

#[test]
fn enqueue_without_registered_worker_seam_fails_before_spool_creation() {
    let _guard = ENV_LOCK.lock().expect("environment lock should not be poisoned");
    let spool = isolated_spool("missing-seam");
    // SAFETY: this integration-test process serializes its environment mutations.
    unsafe { std::env::set_var("PLAYA_SPOOL_DIR", &spool) };
    let job = SpoolJob::PlayFile {
        path: OsValue::from(OsString::from("missing.wav")),
        playback: DetachedPlayback::default(),
        delete_after: false,
    };
    let error = enqueue(job).expect_err("an unregistered host must be rejected");
    // SAFETY: guarded as above.
    unsafe { std::env::remove_var("PLAYA_SPOOL_DIR") };
    assert!(matches!(error, playa::PlaybackError::NoDetachedWorker));
    assert!(!spool.exists());
}
