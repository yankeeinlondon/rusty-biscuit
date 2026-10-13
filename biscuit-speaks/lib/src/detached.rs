//! Detached synthesis preparation and durable Playa handoff.

use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "playa")]
use std::path::Path;
#[cfg(feature = "playa")]
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

#[cfg(feature = "playa")]
use crate::detection::get_providers_for_strategy;
#[cfg(feature = "playa")]
use crate::errors::{AllProvidersFailed, TtsError};
#[cfg(feature = "playa")]
use crate::providers::cloud::ElevenLabsProvider;
#[cfg(feature = "playa")]
use crate::providers::host::{
    ESpeakProvider, EchogardenProvider, GttsProvider, KokoroTtsProvider, SapiProvider, SayProvider,
};
#[cfg(feature = "playa")]
use crate::prepared::ResolvedSynthesis;
#[cfg(feature = "playa")]
use crate::traits::TtsExecutor;
#[cfg(feature = "playa")]
use crate::types::{CloudTtsProvider, HostTtsProvider, TtsConfig, TtsProvider};

/// Preparation-helper marker containing a private Playa preparation record path.
pub const PREPARATION_WORKER_ENV: &str = "BISCUIT_SPEAKS_PREPARATION_WORKER";

static PREPARATION_SEAM_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Stable biscuit-speaks projection of a detached Playa job identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DetachedJobId(String);

impl DetachedJobId {
    /// Return the stable textual identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "playa")]
impl From<playa::detached::JobId> for DetachedJobId {
    fn from(value: playa::detached::JobId) -> Self {
        Self(value.as_str().to_string())
    }
}

/// Install Playa and biscuit-speaks worker modes before normal CLI parsing.
pub async fn run_if_worker() -> Option<i32> {
    #[cfg(feature = "playa")]
    if let Some(code) = playa::detached::run_if_worker() {
        return Some(code);
    }

    #[cfg(feature = "playa")]
    if let Some(record) = std::env::var_os(PREPARATION_WORKER_ENV) {
        let result = run_preparation(Path::new(&record)).await;
        return Some(result.map_or_else(
            |error| {
                tracing::error!(%error, "detached speech preparation failed");
                1
            },
            |_| 0,
        ));
    }

    PREPARATION_SEAM_INSTALLED.store(true, Ordering::Release);
    None
}

/// What the preparation helper needs to finish one reserved slot.
///
/// A record written before `prepared` existed has no such key and decodes as
/// unprepared; a present key must hold complete settings.
#[cfg(feature = "playa")]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparationRecord {
    text: String,
    tts_config: TtsConfig,
    /// The settings a prepared `Speak` retained; each names its provider.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    prepared: Option<ResolvedSynthesis>,
}

/// Decode a key that is present, so an explicit `null` is an error rather
/// than the same thing as an absent key.
#[cfg(feature = "playa")]
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(feature = "playa")]
pub(crate) async fn cached_job(
    provider: TtsProvider,
    text: &str,
    config: &TtsConfig,
) -> Result<Option<playa::detached::SpoolJob>, TtsError> {
    match provider {
        TtsProvider::Host(HostTtsProvider::Say) => {
            SayProvider.cached_detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::KokoroTts) => {
            KokoroTtsProvider::new().cached_detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::EchoGarden) => {
            EchogardenProvider::new().cached_detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::Gtts) => {
            GttsProvider::new().cached_detached_job(text, config).await
        }
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => {
            ElevenLabsProvider::new()?.cached_detached_job(text, config).await
        }
        _ => Ok(None),
    }
}

#[cfg(feature = "playa")]
pub(crate) fn requires_preparation(provider: TtsProvider) -> bool {
    matches!(
        provider,
        TtsProvider::Host(
            HostTtsProvider::Say
                | HostTtsProvider::KokoroTts
                | HostTtsProvider::EchoGarden
                | HostTtsProvider::Gtts
        ) | TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
    )
}

#[cfg(feature = "playa")]
pub(crate) async fn build_job(
    provider: TtsProvider,
    text: &str,
    config: &TtsConfig,
) -> Result<playa::detached::SpoolJob, TtsError> {
    match provider {
        TtsProvider::Host(HostTtsProvider::KokoroTts) => {
            KokoroTtsProvider::new().detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::EchoGarden) => {
            EchogardenProvider::new().detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::Gtts) => {
            GttsProvider::new().detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::ESpeak) => {
            ESpeakProvider::new().detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::Say) => {
            SayProvider.detached_job(text, config).await
        }
        TtsProvider::Host(HostTtsProvider::Sapi) => {
            SapiProvider::new().detached_job(text, config).await
        }
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => {
            ElevenLabsProvider::new()?.detached_job(text, config).await
        }
        unsupported => Err(TtsError::DetachedUnsupported {
            provider: format!("{unsupported:?}"),
        }),
    }
}

#[cfg(feature = "playa")]
pub(crate) fn reserve_preparation(
    text: String,
    config: TtsConfig,
    prepared: Option<ResolvedSynthesis>,
) -> Result<DetachedJobId, TtsError> {
    if !PREPARATION_SEAM_INSTALLED.load(Ordering::Acquire) {
        return Err(TtsError::DetachedUnsupported {
            provider: "current executable has not installed the preparation worker seam".into(),
        });
    }
    let private = serde_json::to_value(PreparationRecord {
        text,
        tts_config: config,
        prepared,
    })
    .map_err(|error| TtsError::ProviderFailed {
        provider: "detached preparation".into(),
        message: error.to_string(),
    })?;
    let id = playa::detached::reserve(playa::detached::PreparingPayload::new(private))?;
    let record = playa::detached::preparation_record_path(&id)?;
    if let Err(error) = spawn_preparation(&record) {
        let _ = playa::detached::publish_failed(&id, "preparation_spawn_failed");
        return Err(error);
    }
    Ok(id.into())
}

/// Records the helper would have been started for, while a unit test
/// captures them; the test binary cannot stand in for the helper.
#[cfg(all(test, feature = "playa"))]
static CAPTURED_HELPER_RECORDS: std::sync::Mutex<Option<Vec<std::path::PathBuf>>> =
    std::sync::Mutex::new(None);

#[cfg(feature = "playa")]
fn spawn_preparation(record: &Path) -> Result<(), TtsError> {
    #[cfg(test)]
    if let Some(records) = CAPTURED_HELPER_RECORDS.lock().unwrap().as_mut() {
        records.push(record.to_path_buf());
        return Ok(());
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new(executable);
    command
        .env_remove(playa::detached::SPOOL_WORKER_ENV)
        .env_remove(playa::detached::DELEGATED_PLAY_WORKER_ENV)
        .env(PREPARATION_WORKER_ENV, record)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sniff::process::configure_detached_child(&mut command);
    command.spawn().map(|_| ()).map_err(|source| TtsError::ProcessSpawnFailed {
        provider: "detached preparation".into(),
        source,
    })
}

#[cfg(feature = "playa")]
async fn run_preparation(path: &Path) -> Result<(), TtsError> {
    let (id, private) = playa::detached::read_preparation_record(path)?;
    let record = match decode_record(private) {
        Ok(record) => record,
        Err(error) => {
            // Fail the slot now rather than leave it holding the queue until
            // its preparation deadline.
            playa::detached::publish_failed(&id, "invalid_preparation_record")?;
            return Err(error);
        }
    };
    match prepare_job(&record).await {
        Ok(job) => playa::detached::publish_ready(&id, job).map_err(Into::into),
        Err(error) => {
            playa::detached::publish_failed(&id, "all_providers_failed")?;
            Err(error)
        }
    }
}

#[cfg(feature = "playa")]
fn decode_record(
    private: serde_json::Map<String, serde_json::Value>,
) -> Result<PreparationRecord, TtsError> {
    serde_json::from_value(serde_json::Value::Object(private)).map_err(|error| {
        TtsError::ProviderFailed {
            provider: "detached preparation".into(),
            message: error.to_string(),
        }
    })
}

/// The ready job for a record: the retained settings first, then the
/// strategy's other providers with the original configuration.
#[cfg(feature = "playa")]
async fn prepare_job(record: &PreparationRecord) -> Result<playa::detached::SpoolJob, TtsError> {
    let mut errors = Vec::new();
    if let Some(settings) = &record.prepared {
        match settings.job(&record.text, &record.tts_config).await {
            Ok(job) => return Ok(job),
            Err(error) => errors.push((settings.provider(), error)),
        }
    }
    let prepared = record.prepared.as_ref().map(ResolvedSynthesis::provider);
    for provider in get_providers_for_strategy(&record.tts_config.failover_strategy) {
        if Some(provider) == prepared {
            continue;
        }
        match build_job(provider, &record.text, &record.tts_config).await {
            Ok(job) => return Ok(job),
            Err(error) => errors.push((provider, error)),
        }
    }
    Err(TtsError::AllProvidersFailed(AllProvidersFailed { errors }))
}

#[cfg(all(test, feature = "playa"))]
mod tests {
    use super::*;
    use crate::types::{Gender, TtsFailoverStrategy, VolumeLevel};

    fn record(prepared: Option<serde_json::Value>) -> serde_json::Map<String, serde_json::Value> {
        let mut private = serde_json::to_value(PreparationRecord {
            text: "This is a test message: record".into(),
            tts_config: TtsConfig::default(),
            prepared: None,
        })
        .unwrap();
        if let Some(prepared) = prepared {
            private["prepared"] = prepared;
        }
        let serde_json::Value::Object(map) = private else {
            unreachable!()
        };
        map
    }

    #[test]
    fn prepared_settings_round_trip_and_an_old_record_decodes_as_unprepared() {
        let original = PreparationRecord {
            text: "This is a test message: record".into(),
            tts_config: TtsConfig::default().with_gender(Gender::Male),
            prepared: Some(ResolvedSynthesis::Kokoro {
                voice: "am_adam".into(),
            }),
        };
        let first = serde_json::to_value(&original).unwrap();
        assert_eq!(first["prepared"], serde_json::json!({"kokoro": {"voice": "am_adam"}}));
        let serde_json::Value::Object(map) = first.clone() else {
            unreachable!()
        };
        let decoded = decode_record(map).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(serde_json::to_value(&decoded).unwrap(), first);

        // Written before retained settings existed, or by an unprepared object.
        let unprepared = decode_record(record(None)).unwrap();
        assert_eq!(unprepared.prepared, None);
        assert!(!serde_json::to_value(&unprepared).unwrap().as_object().unwrap().contains_key("prepared"));
    }

    /// Input Robustness Matrix for the record's one new load-bearing field.
    /// The control row decodes; every edited row is rejected, and
    /// `run_preparation` fails the slot for a rejected record.
    #[test]
    fn every_malformed_prepared_shape_is_rejected() {
        let control = serde_json::json!({"kokoro": {"voice": "am_adam"}});
        assert!(decode_record(record(Some(control))).unwrap().prepared.is_some());

        for (row, prepared) in [
            ("explicit null", serde_json::json!(null)),
            ("wrong type, whole field", serde_json::json!("kokoro")),
            ("wrong type, number", serde_json::json!(7)),
            ("empty object", serde_json::json!({})),
            ("empty array", serde_json::json!([])),
            ("array of the pair", serde_json::json!(["kokoro", {"voice": "am_adam"}])),
            ("wrong type, one field", serde_json::json!({"kokoro": {"voice": 7}})),
            ("null field", serde_json::json!({"kokoro": {"voice": null}})),
            ("missing field", serde_json::json!({"say": {"voice": "Alex"}})),
            ("wrong type, every field", serde_json::json!({"say": {"voice": 1, "rate_wpm": "fast"}})),
            ("unknown field", serde_json::json!({"kokoro": {"voice": "am_adam", "speed": "fast"}})),
            ("unknown provider", serde_json::json!({"piper": {"voice": "x"}})),
            ("two providers", serde_json::json!({"kokoro": {"voice": "a"}, "gtts": {"language": "en"}})),
        ] {
            assert!(decode_record(record(Some(prepared))).is_err(), "{row} must be rejected");
        }

        let mut unknown = record(None);
        unknown.insert("retained".into(), serde_json::json!({"kokoro": {"voice": "am_adam"}}));
        assert!(decode_record(unknown).is_err(), "a misspelled key is not read as absent");
    }

    // ------------------------------------------------------------------
    // Helper handoff, through recording fakes
    // ------------------------------------------------------------------

    #[cfg(unix)]
    mod handoff {
        use std::os::unix::fs::PermissionsExt;
        use std::path::PathBuf;

        use super::*;
        use crate::speak::Speak;

        /// Fake `kokoro-tts` and `echogarden` on an isolated `PATH`, with the
        /// audio cache, capability cache, and a locked spool inside the fixture.
        struct Fixture {
            root: tempfile::TempDir,
            spool: Option<test_toolkit::LockedAudioSpool>,
            _env: Vec<test_toolkit::EnvGuard>,
        }

        impl Fixture {
            fn new() -> Self {
                let root = tempfile::tempdir().unwrap();
                for dir in ["bin", "tmp", "records", "spool/files", "spool/requests"] {
                    std::fs::create_dir_all(root.path().join(dir)).unwrap();
                }
                let tmp = root.path().join("tmp");
                let env = vec![
                    test_toolkit::EnvGuard::set_safe("PATH", root.path().join("bin")),
                    test_toolkit::EnvGuard::set_safe("TMPDIR", &tmp),
                    test_toolkit::EnvGuard::set_safe("TMP", &tmp),
                    test_toolkit::EnvGuard::set_safe("TEMP", &tmp),
                    test_toolkit::EnvGuard::set_safe(
                        "BISCUIT_SPEAKS_CACHE",
                        root.path().join("biscuit-speaks-cache.json"),
                    ),
                    test_toolkit::EnvGuard::set_safe("PLAYA_SPOOL_DIR", root.path().join("spool")),
                    test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN"),
                    test_toolkit::EnvGuard::remove_safe("TTS_PROVIDER"),
                    test_toolkit::EnvGuard::remove_safe(PREPARATION_WORKER_ENV),
                ];
                let spool = Some(test_toolkit::LockedAudioSpool::new(&root.path().join("spool")));
                *CAPTURED_HELPER_RECORDS.lock().unwrap() = Some(Vec::new());
                let fixture = Self { root, spool, _env: env };
                fixture.install("kokoro-tts", true);
                fixture.install("echogarden", true);
                fixture
            }

            /// Install `program`, which records its arguments and writes audio
            /// to its output argument, or fails after a partial write.
            fn install(&self, program: &str, succeeds: bool) {
                let locate = match program {
                    "kokoro-tts" => r#"output=; case "$1" in -*) ;; *) output="$2" ;; esac"#,
                    _ => r#"output=; [ "$1" = speak ] && output="$3""#,
                };
                let write = if succeeds {
                    "printf 'fake audio' > \"$output\""
                } else {
                    "printf partial > \"$output\"; exit 1"
                };
                let script = format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{records}/{program}-'$$\n{locate}\n\
                     if [ -n \"$output\" ]; then {write}; fi\nexit 0\n",
                    records = self.root.path().join("records").display(),
                );
                let path = self.root.path().join("bin").join(program);
                std::fs::write(&path, script).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }

            /// The arguments of every synthesis `program` ran.
            fn syntheses(&self, program: &str) -> Vec<Vec<String>> {
                std::fs::read_dir(self.root.path().join("records"))
                    .unwrap()
                    .map(|entry| entry.unwrap())
                    .filter(|entry| {
                        entry.file_name().to_string_lossy().starts_with(&format!("{program}-"))
                    })
                    .map(|entry| {
                        std::fs::read_to_string(entry.path())
                            .unwrap()
                            .lines()
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .filter(|args| args.first().is_some_and(|arg| arg != "list-voices" && !arg.starts_with('-')))
                    .collect()
            }

            fn cache_entries(&self) -> Vec<PathBuf> {
                std::fs::read_dir(self.root.path().join("tmp"))
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .collect()
            }

            /// The helper records captured since the last call.
            fn helper_records(&self) -> Vec<PathBuf> {
                std::mem::take(CAPTURED_HELPER_RECORDS.lock().unwrap().as_mut().unwrap())
            }

            fn states(&self) -> Vec<&'static str> {
                playa::detached::snapshot()
                    .unwrap()
                    .pending
                    .into_iter()
                    .map(|job| job.state)
                    .collect()
            }

            fn clear(&self) {
                self.spool.as_ref().unwrap().clear_pending().unwrap();
            }
        }

        impl Drop for Fixture {
            fn drop(&mut self) {
                *CAPTURED_HELPER_RECORDS.lock().unwrap() = None;
                drop(self.spool.take());
            }
        }

        fn has_args(args: &[String], expected: &[&str]) -> bool {
            args.windows(expected.len()).any(|window| window == expected)
        }

        fn silent(strategy: TtsFailoverStrategy) -> TtsConfig {
            TtsConfig::new()
                .with_volume(VolumeLevel::Explicit(0.0))
                .with_failover(strategy)
        }

        fn kokoro_only() -> TtsFailoverStrategy {
            TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(HostTtsProvider::KokoroTts))
        }

        /// A prepared object whose file vanished reserves a slot without
        /// synthesizing, hands its settings to the helper in the record, and
        /// the helper synthesizes them again and publishes a ready job.
        #[tokio::test]
        #[serial_test::serial]
        async fn a_prepared_object_whose_file_vanished_hands_its_settings_to_the_helper() {
            let fixture = Fixture::new();
            assert_eq!(run_if_worker().await, None);
            let text = "This is a test message: handoff";
            let config = silent(TtsFailoverStrategy::FirstAvailable).with_gender(Gender::Male);
            let prepared = Speak::new(text).with_config(config).prepare().await.unwrap();
            assert!(prepared.is_prepared());
            let cache = fixture.cache_entries();
            assert_eq!(cache.len(), 1);
            std::fs::remove_file(&cache[0]).unwrap();

            prepared.clone().play_detached().await.unwrap();
            assert_eq!(fixture.states(), ["preparing"]);
            assert_eq!(fixture.syntheses("kokoro-tts").len(), 1, "the requester does not synthesize");
            let records = fixture.helper_records();
            assert_eq!(records.len(), 1);
            let (_, private) = playa::detached::read_preparation_record(&records[0]).unwrap();
            assert_eq!(private["prepared"], serde_json::json!({"kokoro": {"voice": "am_adam"}}));

            run_preparation(&records[0]).await.unwrap();
            assert_eq!(fixture.states(), ["ready"]);
            let syntheses = fixture.syntheses("kokoro-tts");
            assert_eq!(syntheses.len(), 2);
            assert!(syntheses.iter().all(|args| has_args(args, &["--voice", "am_adam"])), "{syntheses:?}");
            assert_eq!(fixture.cache_entries(), cache, "the helper republished the entry");
            assert!(fixture.syntheses("echogarden").is_empty());

            // The file is back, so the prepared object now publishes it directly.
            prepared.play_detached().await.unwrap();
            assert_eq!(fixture.states(), ["ready", "ready"]);
            assert!(fixture.helper_records().is_empty());
            assert_eq!(fixture.syntheses("kokoro-tts").len(), 2);
            fixture.clear();
        }

        /// The helper uses the retained settings rather than resolving the
        /// configuration again, and after their failure tries the strategy's
        /// other providers with the original configuration.
        #[tokio::test]
        #[serial_test::serial]
        async fn the_helper_prefers_retained_settings_and_falls_back_with_the_original_configuration() {
            let fixture = Fixture::new();
            assert_eq!(run_if_worker().await, None);
            let text = "This is a test message: retained in the helper";
            let retained = ResolvedSynthesis::Kokoro {
                voice: "bf_emma".into(),
            };

            reserve_preparation(text.into(), silent(kokoro_only()), Some(retained.clone())).unwrap();
            let record = fixture.helper_records().remove(0);
            run_preparation(&record).await.unwrap();
            let syntheses = fixture.syntheses("kokoro-tts");
            assert_eq!(syntheses.len(), 1);
            assert!(has_args(&syntheses[0], &["--voice", "bf_emma"]), "not the configured af_heart: {syntheses:?}");
            assert_eq!(fixture.states(), ["ready"]);
            fixture.clear();
            for entry in fixture.cache_entries() {
                std::fs::remove_file(entry).unwrap();
            }

            fixture.install("kokoro-tts", false);
            let failover = silent(TtsFailoverStrategy::FirstAvailable);
            assert_eq!(
                get_providers_for_strategy(&failover.failover_strategy)[..2],
                [
                    TtsProvider::Host(HostTtsProvider::KokoroTts),
                    TtsProvider::Host(HostTtsProvider::EchoGarden)
                ]
            );
            reserve_preparation(text.into(), failover, Some(retained.clone())).unwrap();
            let record = fixture.helper_records().remove(0);
            run_preparation(&record).await.unwrap();
            assert_eq!(fixture.states(), ["ready"]);
            let kokoro = fixture.syntheses("kokoro-tts");
            assert_eq!(kokoro.len(), 2, "the failed provider is not retried with the configuration");
            assert!(kokoro.iter().all(|args| !args.contains(&"af_heart".to_string())), "{kokoro:?}");
            let echogarden = fixture.syntheses("echogarden");
            assert_eq!(echogarden.len(), 1);
            assert!(echogarden[0].contains(&"--voice=Heart".to_string()), "{echogarden:?}");
            assert!(!echogarden[0].iter().any(|arg| arg.contains("bf_emma")), "{echogarden:?}");
            let published = fixture.cache_entries();
            assert_eq!(published.len(), 1, "no partial Kokoro entry: {published:?}");
            assert_eq!(std::fs::read(&published[0]).unwrap(), b"fake audio");
            fixture.clear();

            // A specific provider never switches: the slot fails instead.
            reserve_preparation(text.into(), silent(kokoro_only()), Some(retained)).unwrap();
            let record = fixture.helper_records().remove(0);
            let error = run_preparation(&record).await.unwrap_err();
            match error {
                TtsError::AllProvidersFailed(failed) => {
                    assert_eq!(failed.errors.len(), 1);
                    assert_eq!(failed.errors[0].0, TtsProvider::Host(HostTtsProvider::KokoroTts));
                }
                other => panic!("{other:?}"),
            }
            assert_eq!(fixture.states(), ["failed"], "the scheduler can advance past the slot");
            assert_eq!(fixture.syntheses("echogarden").len(), 1);
            fixture.clear();
        }

        /// A record that does not decode fails its slot at once instead of
        /// holding the queue until the preparation deadline.
        #[tokio::test]
        #[serial_test::serial]
        async fn an_undecodable_record_fails_its_slot() {
            let fixture = Fixture::new();
            assert_eq!(run_if_worker().await, None);
            let private = serde_json::Value::Object(super::record(Some(serde_json::json!(null))));
            let id = playa::detached::reserve(playa::detached::PreparingPayload::new(private)).unwrap();
            assert_eq!(fixture.states(), ["preparing"]);
            let path = playa::detached::preparation_record_path(&id).unwrap();
            assert!(run_preparation(&path).await.is_err());
            assert_eq!(fixture.states(), ["failed"]);
            assert!(fixture.syntheses("kokoro-tts").is_empty());
            fixture.clear();
        }
    }
}
