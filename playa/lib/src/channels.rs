#[cfg(feature = "sfx-native")]
use std::sync::mpsc;
#[cfg(feature = "sfx-native")]
use std::time::Duration;

#[cfg(feature = "sfx-native")]
use rodio::cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
};

/// Maximum time to wait for any audio device operation (enumeration, probing).
/// If CoreAudio (or the platform audio subsystem) doesn't respond within this
/// window, it's likely hung and we should bail out rather than blocking forever.
#[cfg(feature = "sfx-native")]
const DEVICE_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Outcome categories for bounded audio device lookup.
#[cfg(feature = "sfx-native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceLookupError {
    /// The platform audio subsystem did not respond before the caller deadline.
    TimedOut,
}

/// Information about a native output channel (audio device).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputChannel {
    /// The display name of the audio device
    pub name: String,
    /// The slugified identifier used for the CLI
    pub id: String,
    /// Whether this is the default device for general audio playback
    pub is_default_audio: bool,
    /// Whether this is the default device for sound effects
    pub is_default_sfx: bool,
    /// Supported output sample-rate ranges reported by the audio backend.
    pub sample_rates: Vec<SampleRateRange>,
}

/// A supported sample-rate range for a native output channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRateRange {
    /// Lowest supported sample rate in Hz.
    pub min_hz: u32,
    /// Highest supported sample rate in Hz.
    pub max_hz: u32,
}

#[cfg(feature = "sfx-native")]
pub(crate) fn slugify(s: &str) -> String {
    let mut slug = String::with_capacity(s.len());
    let mut last_was_dash = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            slug.extend(c.to_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// Lists all available native output channels (audio devices).
///
/// Indicates which channel is the default for general audio and which is the
/// default for sound effects. On macOS, these can be different if configured
/// in System Settings.
///
/// This function is guarded by a timeout to prevent hanging when the platform
/// audio subsystem (e.g., CoreAudio on macOS) is unresponsive.
#[cfg(feature = "sfx-native")]
pub fn get_output_channels() -> Result<Vec<OutputChannel>, Box<dyn std::error::Error>> {
    let (tx, rx) =
        mpsc::channel::<Result<Vec<OutputChannel>, Box<dyn std::error::Error + Send + Sync>>>();

    std::thread::spawn(move || {
        let _ = tx.send(get_output_channels_inner());
    });

    match rx.recv_timeout(DEVICE_PROBE_TIMEOUT) {
        Ok(result) => result.map_err(|e| e as Box<dyn std::error::Error>),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "audio device enumeration timed out after {}s — audio subsystem may be unresponsive",
            DEVICE_PROBE_TIMEOUT.as_secs()
        )
        .into()),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("audio device enumeration thread panicked".into())
        }
    }
}

#[cfg(feature = "sfx-native")]
fn get_output_channels_inner()
-> Result<Vec<OutputChannel>, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let default_output = host.default_output_device();
    let default_audio_name = default_output
        .as_ref()
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_string()));

    #[cfg(all(target_os = "macos", feature = "sfx-native-audio"))]
    let sfx_device_name = crate::sfx_player::macos::find_system_sound_device()
        .unwrap_or(None)
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_string()))
        .or_else(|| default_audio_name.clone());

    #[cfg(not(all(target_os = "macos", feature = "sfx-native-audio")))]
    let sfx_device_name = default_audio_name.clone();

    let mut channels = Vec::new();
    let mut name_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let devices = host
        .output_devices()
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;

    for device in devices {
        if let Ok(desc) = device.description() {
            let base_name = desc.name().to_string();
            let count = name_counts.entry(base_name.clone()).or_insert(0);
            *count += 1;

            let name = if *count > 1 {
                format!("{} ({})", base_name, count)
            } else {
                base_name.clone()
            };

            let id = slugify(&name);

            let is_default_audio = Some(&base_name) == default_audio_name.as_ref() && *count == 1;
            let is_default_sfx = Some(&base_name) == sfx_device_name.as_ref() && *count == 1;
            let sample_rates = supported_sample_rates(&device);

            channels.push(OutputChannel {
                name,
                id,
                is_default_audio,
                is_default_sfx,
                sample_rates,
            });
        }
    }

    Ok(channels)
}

#[cfg(feature = "sfx-native")]
fn supported_sample_rates(device: &rodio::Device) -> Vec<SampleRateRange> {
    let Ok(configs) = device.supported_output_configs() else {
        return Vec::new();
    };

    let mut ranges = std::collections::BTreeSet::new();
    for config in configs {
        ranges.insert(SampleRateRange {
            min_hz: config.min_sample_rate(),
            max_hz: config.max_sample_rate(),
        });
    }

    ranges.into_iter().collect()
}

/// Finds an output device by its slugified id or exact name using a caller
/// provided timeout.
#[cfg(feature = "sfx-native")]
pub(crate) fn find_device_by_id_or_name_with_timeout(
    id_or_name: &str,
    timeout: Duration,
) -> Result<Option<rodio::Device>, DeviceLookupError> {
    let name = id_or_name.to_string();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let _ = tx.send(find_device_by_id_or_name_inner(&name));
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
            Err(DeviceLookupError::TimedOut)
        }
    }
}

#[cfg(feature = "sfx-native")]
fn find_device_by_id_or_name_inner(id_or_name: &str) -> Option<rodio::Device> {
    let host = cpal::default_host();
    if let Ok(devices) = host.output_devices() {
        let mut name_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for device in devices {
            if let Ok(desc) = device.description() {
                let base_name = desc.name().to_string();
                let count = name_counts.entry(base_name.clone()).or_insert(0);
                *count += 1;

                let current_name = if *count > 1 {
                    format!("{} ({})", base_name, count)
                } else {
                    base_name
                };

                let current_id = slugify(&current_name);

                if current_id == id_or_name || current_name == id_or_name {
                    return Some(device);
                }
            }
        }
    }
    None
}
