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

#[cfg(feature = "playa")]
#[derive(Serialize, Deserialize)]
struct PreparationRecord {
    text: String,
    tts_config: TtsConfig,
}

#[cfg(feature = "playa")]
pub(crate) async fn cached_job(
    provider: TtsProvider,
    text: &str,
    config: &TtsConfig,
) -> Result<Option<playa::detached::SpoolJob>, TtsError> {
    match provider {
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
            HostTtsProvider::KokoroTts
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
) -> Result<DetachedJobId, TtsError> {
    if !PREPARATION_SEAM_INSTALLED.load(Ordering::Acquire) {
        return Err(TtsError::DetachedUnsupported {
            provider: "current executable has not installed the preparation worker seam".into(),
        });
    }
    let private = serde_json::to_value(PreparationRecord {
        text,
        tts_config: config,
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

#[cfg(feature = "playa")]
fn spawn_preparation(record: &Path) -> Result<(), TtsError> {
    let executable = std::env::current_exe()?;
    let mut command = Command::new(executable);
    command
        .env_remove(playa::detached::SPOOL_WORKER_ENV)
        .env_remove(playa::detached::DELEGATED_PLAY_WORKER_ENV)
        .env(PREPARATION_WORKER_ENV, record)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_detached(&mut command);
    command.spawn().map(|_| ()).map_err(|source| TtsError::ProcessSpawnFailed {
        provider: "detached preparation".into(),
        source,
    })
}

#[cfg(all(feature = "playa", unix))]
fn configure_detached(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(all(feature = "playa", windows))]
fn configure_detached(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    command.creation_flags(0x0000_0008 | 0x0000_0200 | 0x0800_0000);
}

#[cfg(feature = "playa")]
async fn run_preparation(path: &Path) -> Result<(), TtsError> {
    let (id, private) = playa::detached::read_preparation_record(path)?;
    let record: PreparationRecord = serde_json::from_value(serde_json::Value::Object(private))
        .map_err(|error| TtsError::ProviderFailed {
            provider: "detached preparation".into(),
            message: error.to_string(),
        })?;
    let providers = get_providers_for_strategy(&record.tts_config.failover_strategy);
    let mut errors = Vec::new();
    for provider in providers {
        match build_job(provider, &record.text, &record.tts_config).await {
            Ok(job) => return playa::detached::publish_ready(&id, job).map_err(Into::into),
            Err(error) => errors.push((provider, error)),
        }
    }
    playa::detached::publish_failed(&id, "all_providers_failed")?;
    Err(TtsError::AllProvidersFailed(AllProvidersFailed { errors }))
}
