use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::player::AudioPlayer;

/// Header metadata used to verify that playback reached the end of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProbedAudioMetadata {
    /// Duration declared by the audio container.
    #[serde(with = "duration_millis")]
    pub duration: Duration,
    /// Number of interleaved audio channels.
    pub channels: u16,
}

/// Route that completed a playback attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackRoute {
    /// In-process rodio/symphonia playback.
    Native,
    /// An explicitly selected host player subprocess.
    Host(AudioPlayer),
    /// Playback was intentionally skipped.
    #[default]
    DryRun,
}

/// Comparison of observed playback time with the source duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackVerdict {
    /// Playback lasted long enough to be considered complete.
    Complete,
    /// Playback ended substantially before the adjusted source duration.
    Truncated {
        /// Difference between adjusted source duration and elapsed time.
        #[serde(with = "duration_millis")]
        missing: Duration,
    },
    /// The source had no trustworthy duration metadata.
    #[default]
    Unverified,
}

impl PlaybackVerdict {
    /// Compare elapsed time with source duration at the speed the route applied.
    pub fn for_timing(
        expected: Option<Duration>,
        elapsed: Duration,
        effective_speed: f32,
    ) -> Self {
        let Some(expected) = expected else {
            return Self::Unverified;
        };
        if !effective_speed.is_finite() || effective_speed <= 0.0 {
            return Self::Unverified;
        }

        let adjusted = expected.div_f64(f64::from(effective_speed));
        // Startup overhead only lengthens wall time. Requiring a 10% shortfall
        // plus 250 ms therefore avoids false truncation at process/device startup.
        let threshold = adjusted.mul_f64(0.9).saturating_sub(Duration::from_millis(250));
        if elapsed < threshold {
            Self::Truncated {
                missing: adjusted.saturating_sub(elapsed),
            }
        } else {
            Self::Complete
        }
    }
}

/// Observable result of one successful or intentionally skipped playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlaybackReport {
    /// Playback route used for the attempt.
    #[serde(default)]
    pub route: PlaybackRoute,
    /// Source duration, when it was available without another network fetch.
    #[serde(default, rename = "expected_millis", with = "optional_duration_millis")]
    pub expected: Option<Duration>,
    /// Wall-clock time spent in the selected playback route.
    #[serde(default, rename = "elapsed_millis", with = "duration_millis")]
    pub elapsed: Duration,
    /// Completion assessment based on expected and elapsed duration.
    #[serde(default)]
    pub verdict: PlaybackVerdict,
}

impl PlaybackReport {
    pub(crate) fn completed(
        route: PlaybackRoute,
        metadata: Option<ProbedAudioMetadata>,
        elapsed: Duration,
        effective_speed: f32,
    ) -> Self {
        let expected = metadata.map(|value| value.duration);
        Self {
            route,
            expected,
            elapsed,
            verdict: PlaybackVerdict::for_timing(expected, elapsed, effective_speed),
        }
    }

    pub(crate) fn warn_if_truncated(self) -> Self {
        if matches!(self.verdict, PlaybackVerdict::Truncated { .. }) {
            tracing::warn!(
                route = ?self.route,
                expected_seconds = self.expected.map(|value| value.as_secs_f64()),
                elapsed_seconds = self.elapsed.as_secs_f64(),
                "audio playback ended before the expected duration"
            );
        }
        self
    }
}

mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(value.as_millis().min(u128::from(u64::MAX)) as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Duration::from_millis)
    }
}

mod optional_duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<u64>::deserialize(deserializer)
            .map(|value| value.map(Duration::from_millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tracing_test::traced_test]
    #[test]
    fn truncated_report_emits_exactly_one_warning() {
        PlaybackReport {
            route: PlaybackRoute::Native,
            expected: Some(Duration::from_secs(2)),
            elapsed: Duration::from_millis(500),
            verdict: PlaybackVerdict::Truncated {
                missing: Duration::from_millis(1_500),
            },
        }
        .warn_if_truncated();

        logs_assert(|lines: &[&str]| {
            let warnings = lines
                .iter()
                .filter(|line| line.contains("audio playback ended before the expected duration"))
                .count();
            if warnings == 1 {
                Ok(())
            } else {
                Err(format!("expected one truncation warning, found {warnings}: {lines:?}"))
            }
        });
    }

    #[tracing_test::traced_test]
    #[test]
    fn truncated_host_report_emits_one_warning_and_complete_emits_none() {
        let base = PlaybackReport {
            route: PlaybackRoute::Host(AudioPlayer::Mpv),
            expected: Some(Duration::from_secs(2)),
            elapsed: Duration::from_millis(500),
            verdict: PlaybackVerdict::Truncated {
                missing: Duration::from_millis(1_500),
            },
        };
        base.warn_if_truncated();
        PlaybackReport {
            verdict: PlaybackVerdict::Complete,
            ..base
        }
        .warn_if_truncated();

        logs_assert(|lines: &[&str]| {
            let warnings = lines
                .iter()
                .filter(|line| line.contains("audio playback ended before the expected duration"))
                .count();
            if warnings == 1 {
                Ok(())
            } else {
                Err(format!("expected one host warning, found {warnings}: {lines:?}"))
            }
        });
    }
}
