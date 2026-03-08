#[cfg(feature = "sfx-native")]
use rodio::cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
};

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
#[cfg(feature = "sfx-native")]
pub fn get_output_channels() -> Result<Vec<OutputChannel>, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let default_output = host.default_output_device();
    let default_audio_name = default_output
        .as_ref()
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_string()));

    #[cfg(all(target_os = "macos", feature = "sfx-native-macos"))]
    let sfx_device_name = crate::sfx_player::macos::find_system_sound_device()
        .unwrap_or(None)
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_string()))
        .or_else(|| default_audio_name.clone());

    #[cfg(not(all(target_os = "macos", feature = "sfx-native-macos")))]
    let sfx_device_name = default_audio_name.clone();

    let mut channels = Vec::new();
    let mut name_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let devices = host.output_devices()?;

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

            channels.push(OutputChannel {
                name,
                id,
                is_default_audio,
                is_default_sfx,
            });
        }
    }

    Ok(channels)
}

/// Finds an output device by its slugified id or exact name.
#[cfg(feature = "sfx-native")]
pub fn find_device_by_id_or_name(id_or_name: &str) -> Option<rodio::Device> {
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
