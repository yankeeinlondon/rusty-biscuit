#[cfg(feature = "sfx-native")]
use rodio::cpal::{self, traits::{DeviceTrait, HostTrait}};

/// Information about a native output channel (audio device).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputChannel {
    /// The display name of the audio device
    pub name: String,
    /// Whether this is the default device for general audio playback
    pub is_default_audio: bool,
    /// Whether this is the default device for sound effects
    pub is_default_sfx: bool,
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
    let default_audio_name = default_output.as_ref().and_then(|d| d.description().ok().map(|desc| desc.name().to_string()));

    #[cfg(all(target_os = "macos", feature = "sfx-native-macos"))]
    let sfx_device_name = crate::sfx_player::macos::find_system_sound_device()
        .unwrap_or(None)
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_string()))
        .or_else(|| default_audio_name.clone());

    #[cfg(not(all(target_os = "macos", feature = "sfx-native-macos")))]
    let sfx_device_name = default_audio_name.clone();

    let mut channels = Vec::new();
    let devices = host.output_devices()?;

    for device in devices {
        if let Ok(desc) = device.description() {
            let name = desc.name().to_string();
            let is_default_audio = Some(&name) == default_audio_name.as_ref();
            let is_default_sfx = Some(&name) == sfx_device_name.as_ref();

            if !channels.iter().any(|c: &OutputChannel| c.name == name) {
                channels.push(OutputChannel {
                    name,
                    is_default_audio,
                    is_default_sfx,
                });
            }
        }
    }

    Ok(channels)
}
