//! Audio device detection module.
//!
//! Detects audio hardware devices (inputs, outputs) using platform-specific
//! APIs:
//! - **macOS**: CoreAudio for full device enumeration (channels, sample rates)
//! - **Linux**: Parses `/proc/asound/cards` for ALSA card names
//! - **Windows**: Queries `Win32_SoundDevice` via `wmic`

use serde::{Deserialize, Serialize};

/// Audio device connection type.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioDeviceKind {
    /// Built-in speaker or microphone
    BuiltIn,
    /// USB audio device
    Usb,
    /// Bluetooth audio device
    Bluetooth,
    /// Virtual/software audio device
    Virtual,
    /// HDMI audio output
    Hdmi,
    /// Thunderbolt audio device
    Thunderbolt,
    /// Unknown or undetected connection type
    #[default]
    Unknown,
}

impl std::fmt::Display for AudioDeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioDeviceKind::BuiltIn => write!(f, "Built-in"),
            AudioDeviceKind::Usb => write!(f, "USB"),
            AudioDeviceKind::Bluetooth => write!(f, "Bluetooth"),
            AudioDeviceKind::Virtual => write!(f, "Virtual"),
            AudioDeviceKind::Hdmi => write!(f, "HDMI"),
            AudioDeviceKind::Thunderbolt => write!(f, "Thunderbolt"),
            AudioDeviceKind::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Audio device direction (input, output, or both).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioDirection {
    /// Input-only device (microphone)
    Input,
    /// Output-only device (speakers, headphones)
    #[default]
    Output,
    /// Bidirectional device (e.g., USB audio interface)
    InputOutput,
}

impl std::fmt::Display for AudioDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioDirection::Input => write!(f, "Input"),
            AudioDirection::Output => write!(f, "Output"),
            AudioDirection::InputOutput => write!(f, "Input/Output"),
        }
    }
}

/// Information about a detected audio device.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// Device display name (e.g., "MacBook Pro Speakers")
    pub name: String,
    /// Unique device identifier
    pub uid: String,
    /// Connection type (built-in, USB, Bluetooth, etc.)
    pub kind: AudioDeviceKind,
    /// Device direction (input, output, or both)
    pub direction: AudioDirection,
    /// Whether this is the system default input device
    pub is_default_input: bool,
    /// Whether this is the system default output device
    pub is_default_output: bool,
    /// Current nominal sample rate in Hz
    pub sample_rate: f64,
    /// Available sample rates in Hz
    pub available_sample_rates: Vec<f64>,
    /// Number of input channels
    pub input_channels: u32,
    /// Number of output channels
    pub output_channels: u32,
}

/// Detects all audio devices on macOS using CoreAudio.
///
/// Enumerates all audio hardware devices and returns their properties
/// including name, UID, connection type, channel counts, sample rates,
/// and default device status.
///
/// ## Examples
///
/// ```no_run
/// use sniff::hardware::detect_audio_devices;
///
/// let devices = detect_audio_devices();
/// for dev in &devices {
///     println!("{} ({}, {})", dev.name, dev.kind, dev.direction);
///     if dev.is_default_output {
///         println!("  ** Default Output **");
///     }
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn detect_audio_devices() -> Vec<AudioDeviceInfo> {
    use std::mem;

    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use coreaudio_sys::{
        AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
        AudioObjectPropertyAddress, AudioValueRange,
        kAudioDevicePropertyAvailableNominalSampleRates, kAudioDevicePropertyDeviceNameCFString,
        kAudioDevicePropertyDeviceUID, kAudioDevicePropertyNominalSampleRate,
        kAudioDevicePropertyStreamConfiguration, kAudioDevicePropertyTransportType,
        kAudioDeviceTransportTypeBluetooth, kAudioDeviceTransportTypeBluetoothLE,
        kAudioDeviceTransportTypeBuiltIn, kAudioDeviceTransportTypeHDMI,
        kAudioDeviceTransportTypeThunderbolt, kAudioDeviceTransportTypeUSB,
        kAudioDeviceTransportTypeVirtual, kAudioHardwarePropertyDefaultInputDevice,
        kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
        kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyScopeInput, kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
    };

    // --- Helper closures ---

    let get_device_ids = || -> Vec<AudioObjectID> {
        unsafe {
            let address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDevices,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut data_size: u32 = 0;
            let status = AudioObjectGetPropertyDataSize(
                kAudioObjectSystemObject,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
            );
            if status != 0 || data_size == 0 {
                return Vec::new();
            }

            let device_count = data_size as usize / mem::size_of::<AudioObjectID>();
            let mut device_ids = vec![0u32; device_count];

            let status = AudioObjectGetPropertyData(
                kAudioObjectSystemObject,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                device_ids.as_mut_ptr().cast(),
            );
            if status != 0 {
                return Vec::new();
            }

            device_ids
        }
    };

    let get_default_device = |selector: u32| -> AudioObjectID {
        unsafe {
            let mut device_id: AudioObjectID = 0;
            let mut data_size = mem::size_of::<AudioObjectID>() as u32;

            let address = AudioObjectPropertyAddress {
                mSelector: selector,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectGetPropertyData(
                kAudioObjectSystemObject,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                (&raw mut device_id).cast(),
            );

            if status != 0 { 0 } else { device_id }
        }
    };

    let get_cfstring_property = |device_id: AudioObjectID, selector: u32| -> Option<String> {
        unsafe {
            let mut cf_ref: coreaudio_sys::CFStringRef = std::ptr::null();
            let mut data_size = mem::size_of::<coreaudio_sys::CFStringRef>() as u32;

            let address = AudioObjectPropertyAddress {
                mSelector: selector,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectGetPropertyData(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                (&raw mut cf_ref).cast(),
            );

            if status != 0 || cf_ref.is_null() {
                return None;
            }

            // Cast coreaudio_sys::CFStringRef to core_foundation::CFStringRef
            let cf_string =
                CFString::wrap_under_create_rule(cf_ref as core_foundation::string::CFStringRef);
            Some(cf_string.to_string())
        }
    };

    let get_transport_type = |device_id: AudioObjectID| -> AudioDeviceKind {
        unsafe {
            let mut transport: u32 = 0;
            let mut data_size = mem::size_of::<u32>() as u32;

            let address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyTransportType,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectGetPropertyData(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                (&raw mut transport).cast(),
            );

            if status != 0 {
                return AudioDeviceKind::Unknown;
            }

            match transport {
                x if x == kAudioDeviceTransportTypeBuiltIn => AudioDeviceKind::BuiltIn,
                x if x == kAudioDeviceTransportTypeUSB => AudioDeviceKind::Usb,
                x if x == kAudioDeviceTransportTypeBluetooth
                    || x == kAudioDeviceTransportTypeBluetoothLE =>
                {
                    AudioDeviceKind::Bluetooth
                }
                x if x == kAudioDeviceTransportTypeVirtual => AudioDeviceKind::Virtual,
                x if x == kAudioDeviceTransportTypeHDMI => AudioDeviceKind::Hdmi,
                x if x == kAudioDeviceTransportTypeThunderbolt => AudioDeviceKind::Thunderbolt,
                _ => AudioDeviceKind::Unknown,
            }
        }
    };

    let get_channel_count = |device_id: AudioObjectID, scope: u32| -> u32 {
        unsafe {
            let address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyStreamConfiguration,
                mScope: scope,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut data_size: u32 = 0;
            let status = AudioObjectGetPropertyDataSize(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
            );
            if status != 0 || data_size == 0 {
                return 0;
            }

            // Allocate properly aligned buffer for AudioBufferList
            // AudioBufferList requires 8-byte alignment (on 64-bit macOS) for its
            // nested AudioBuffer struct which contains a pointer (mData).
            let layout = std::alloc::Layout::from_size_align(data_size as usize, 8)
                .expect("AudioBufferList layout size should be valid");
            let buf = std::alloc::alloc(layout);
            if buf.is_null() {
                return 0;
            }
            let status = AudioObjectGetPropertyData(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                buf.cast(),
            );
            if status != 0 {
                std::alloc::dealloc(buf, layout);
                return 0;
            }

            // Cast to AudioBufferList and read mNumberBuffers, then iterate
            // buffers using proper struct layout (respects alignment/padding).
            let buffer_list = &*(buf as *const coreaudio_sys::AudioBufferList);
            let num_buffers = buffer_list.mNumberBuffers as usize;
            let mut total_channels: u32 = 0;

            // mBuffers is a flexible array member declared as [AudioBuffer; 1].
            // Access additional elements via pointer arithmetic.
            let buffers_ptr = buffer_list.mBuffers.as_ptr();
            for i in 0..num_buffers {
                let audio_buf = &*buffers_ptr.add(i);
                total_channels += audio_buf.mNumberChannels;
            }

            std::alloc::dealloc(buf, layout);
            total_channels
        }
    };

    let get_sample_rate = |device_id: AudioObjectID| -> f64 {
        unsafe {
            let mut rate: f64 = 0.0;
            let mut data_size = mem::size_of::<f64>() as u32;

            let address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyNominalSampleRate,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectGetPropertyData(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                (&raw mut rate).cast(),
            );

            if status != 0 { 0.0 } else { rate }
        }
    };

    let get_available_sample_rates = |device_id: AudioObjectID| -> Vec<f64> {
        unsafe {
            let address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyAvailableNominalSampleRates,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut data_size: u32 = 0;
            let status = AudioObjectGetPropertyDataSize(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
            );
            if status != 0 || data_size == 0 {
                return Vec::new();
            }

            let count = data_size as usize / mem::size_of::<AudioValueRange>();
            let mut ranges = vec![
                AudioValueRange {
                    mMinimum: 0.0,
                    mMaximum: 0.0,
                };
                count
            ];

            let status = AudioObjectGetPropertyData(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                ranges.as_mut_ptr().cast(),
            );
            if status != 0 {
                return Vec::new();
            }

            // Each range is min..max. For discrete rates, min == max.
            // Collect unique rates.
            let mut rates: Vec<f64> = ranges
                .iter()
                .flat_map(|r| {
                    if (r.mMinimum - r.mMaximum).abs() < 0.01 {
                        vec![r.mMinimum]
                    } else {
                        vec![r.mMinimum, r.mMaximum]
                    }
                })
                .collect();

            rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            rates.dedup_by(|a, b| (*a - *b).abs() < 0.01);
            rates
        }
    };

    // --- Main detection logic ---

    let device_ids = get_device_ids();
    if device_ids.is_empty() {
        return Vec::new();
    }

    let default_input = get_default_device(kAudioHardwarePropertyDefaultInputDevice);
    let default_output = get_default_device(kAudioHardwarePropertyDefaultOutputDevice);

    let mut devices = Vec::with_capacity(device_ids.len());

    for &device_id in &device_ids {
        let name = match get_cfstring_property(device_id, kAudioDevicePropertyDeviceNameCFString) {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let uid =
            get_cfstring_property(device_id, kAudioDevicePropertyDeviceUID).unwrap_or_default();

        let input_channels = get_channel_count(device_id, kAudioObjectPropertyScopeInput);
        let output_channels = get_channel_count(device_id, kAudioObjectPropertyScopeOutput);

        // Skip devices with no channels at all
        if input_channels == 0 && output_channels == 0 {
            continue;
        }

        let direction = match (input_channels > 0, output_channels > 0) {
            (true, true) => AudioDirection::InputOutput,
            (true, false) => AudioDirection::Input,
            _ => AudioDirection::Output,
        };

        devices.push(AudioDeviceInfo {
            name,
            uid,
            kind: get_transport_type(device_id),
            direction,
            is_default_input: device_id == default_input,
            is_default_output: device_id == default_output,
            sample_rate: get_sample_rate(device_id),
            available_sample_rates: get_available_sample_rates(device_id),
            input_channels,
            output_channels,
        });
    }

    devices
}

/// Parsed ALSA card header from `/proc/asound/cards`.
#[cfg(target_os = "linux")]
struct AlsaCard {
    id: u32,
    name: String,
}

/// Parses `/proc/asound/cards` contents into `(card_id, long_name)` entries.
#[cfg(target_os = "linux")]
fn parse_alsa_cards(contents: &str) -> Vec<AlsaCard> {
    let mut cards = Vec::new();
    for line in contents.lines() {
        // Card lines start with a number: " 0 [PCH  ]: HDA-Intel - HDA Intel PCH"
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.as_bytes()[0].is_ascii_digit() {
            continue;
        }

        let Some(id_str) = trimmed.split_whitespace().next() else {
            continue;
        };
        let Ok(id) = id_str.parse::<u32>() else {
            continue;
        };

        let name = trimmed.split(" - ").nth(1).unwrap_or(trimmed).to_string();
        cards.push(AlsaCard { id, name });
    }
    cards
}

/// Parsed direction flags for an ALSA card from `/proc/asound/pcm`.
#[cfg(target_os = "linux")]
#[derive(Default, Clone, Copy)]
struct AlsaCardDirections {
    has_playback: bool,
    has_capture: bool,
}

/// Parses `/proc/asound/pcm` contents into per-card direction flags.
///
/// Each line looks like:
/// ```text
/// 00-00: ALC257 Analog : ALC257 Analog : playback 1 : capture 1
/// 01-03: HDMI 0 : HDA Intel HDMI : playback 1
/// ```
#[cfg(target_os = "linux")]
fn parse_alsa_pcm(contents: &str) -> std::collections::HashMap<u32, AlsaCardDirections> {
    use std::collections::HashMap;

    let mut map: HashMap<u32, AlsaCardDirections> = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // First token: "CC-DD:" where CC is the card id.
        let Some(head) = trimmed.split(':').next() else {
            continue;
        };
        let Some(card_str) = head.split('-').next() else {
            continue;
        };
        let Ok(card_id) = card_str.trim().parse::<u32>() else {
            continue;
        };

        let entry = map.entry(card_id).or_default();
        for field in trimmed.split(':').map(str::trim) {
            if field.starts_with("playback ") {
                entry.has_playback = true;
            } else if field.starts_with("capture ") {
                entry.has_capture = true;
            }
        }
    }
    map
}

/// Classifies an ALSA card name into an [`AudioDeviceKind`].
#[cfg(target_os = "linux")]
fn classify_alsa_kind(name: &str) -> AudioDeviceKind {
    let upper = name.to_uppercase();
    if upper.contains("HDMI") {
        AudioDeviceKind::Hdmi
    } else if upper.contains("USB") {
        AudioDeviceKind::Usb
    } else if upper.contains("BLUETOOTH") || upper.contains("BT") {
        AudioDeviceKind::Bluetooth
    } else {
        AudioDeviceKind::BuiltIn
    }
}

/// Detects audio devices on Linux by parsing ALSA procfs.
///
/// Reads `/proc/asound/cards` to enumerate sound cards with their names and
/// `/proc/asound/pcm` to determine which cards expose playback (output) or
/// capture (input) streams. If `/proc/asound/pcm` is unavailable, all cards
/// default to output-only.
///
/// The device kind is inferred from the card name (HDMI, USB, Bluetooth).
/// Channel counts, sample rates, and system default status are not available
/// from this source.
#[cfg(target_os = "linux")]
pub fn detect_audio_devices() -> Vec<AudioDeviceInfo> {
    let cards_raw = match std::fs::read_to_string("/proc/asound/cards") {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let cards = parse_alsa_cards(&cards_raw);
    let directions = std::fs::read_to_string("/proc/asound/pcm")
        .map(|s| parse_alsa_pcm(&s))
        .unwrap_or_default();

    let mut devices = Vec::with_capacity(cards.len());
    for card in cards {
        let dirs = directions.get(&card.id).copied().unwrap_or(AlsaCardDirections {
            has_playback: true,
            has_capture: false,
        });

        let direction = match (dirs.has_capture, dirs.has_playback) {
            (true, true) => AudioDirection::InputOutput,
            (true, false) => AudioDirection::Input,
            (false, true) => AudioDirection::Output,
            (false, false) => continue,
        };

        devices.push(AudioDeviceInfo {
            name: card.name.clone(),
            uid: format!("card{}", card.id),
            kind: classify_alsa_kind(&card.name),
            direction,
            ..Default::default()
        });
    }

    devices
}

/// Detects audio devices on Windows via `wmic`.
///
/// Queries `Win32_SoundDevice` for device names and status. Device kind is
/// inferred from the name (USB, Bluetooth, HDMI). Channel counts and sample
/// rates are not available from this source.
#[cfg(target_os = "windows")]
pub fn detect_audio_devices() -> Vec<AudioDeviceInfo> {
    let output = std::process::Command::new("wmic")
        .args(["path", "Win32_SoundDevice", "get", "Name,Status"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut devices = Vec::new();
    for (idx, line) in output.lines().skip(1).enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // wmic output: "Name      Status" — status is the last word
        let name = line
            .rsplit_once(char::is_whitespace)
            .map(|(n, _)| n.trim())
            .unwrap_or(line)
            .to_string();

        let name_upper = name.to_uppercase();
        let kind = if name_upper.contains("HDMI") {
            AudioDeviceKind::Hdmi
        } else if name_upper.contains("USB") {
            AudioDeviceKind::Usb
        } else if name_upper.contains("BLUETOOTH") {
            AudioDeviceKind::Bluetooth
        } else if name_upper.contains("VIRTUAL") {
            AudioDeviceKind::Virtual
        } else {
            AudioDeviceKind::Unknown
        };

        devices.push(AudioDeviceInfo {
            name,
            uid: format!("win-audio-{idx}"),
            kind,
            direction: AudioDirection::Output,
            ..Default::default()
        });
    }

    devices
}

/// Stub implementation for platforms without audio detection.
///
/// Returns an empty vector.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn detect_audio_devices() -> Vec<AudioDeviceInfo> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_device_kind_display() {
        assert_eq!(AudioDeviceKind::BuiltIn.to_string(), "Built-in");
        assert_eq!(AudioDeviceKind::Usb.to_string(), "USB");
        assert_eq!(AudioDeviceKind::Bluetooth.to_string(), "Bluetooth");
        assert_eq!(AudioDeviceKind::Virtual.to_string(), "Virtual");
        assert_eq!(AudioDeviceKind::Hdmi.to_string(), "HDMI");
        assert_eq!(AudioDeviceKind::Thunderbolt.to_string(), "Thunderbolt");
        assert_eq!(AudioDeviceKind::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn audio_device_kind_default() {
        let default: AudioDeviceKind = Default::default();
        assert_eq!(default, AudioDeviceKind::Unknown);
    }

    #[test]
    fn audio_direction_display() {
        assert_eq!(AudioDirection::Input.to_string(), "Input");
        assert_eq!(AudioDirection::Output.to_string(), "Output");
        assert_eq!(AudioDirection::InputOutput.to_string(), "Input/Output");
    }

    #[test]
    fn audio_direction_default() {
        let default: AudioDirection = Default::default();
        assert_eq!(default, AudioDirection::Output);
    }

    #[test]
    fn audio_device_info_default() {
        let default: AudioDeviceInfo = Default::default();
        assert!(default.name.is_empty());
        assert!(default.uid.is_empty());
        assert_eq!(default.kind, AudioDeviceKind::Unknown);
        assert_eq!(default.direction, AudioDirection::Output);
        assert!(!default.is_default_input);
        assert!(!default.is_default_output);
        assert_eq!(default.sample_rate, 0.0);
        assert!(default.available_sample_rates.is_empty());
        assert_eq!(default.input_channels, 0);
        assert_eq!(default.output_channels, 0);
    }

    #[test]
    fn audio_device_info_serialization_roundtrip() {
        let device = AudioDeviceInfo {
            name: "MacBook Pro Speakers".to_string(),
            uid: "BuiltInSpeakerDevice".to_string(),
            kind: AudioDeviceKind::BuiltIn,
            direction: AudioDirection::Output,
            is_default_input: false,
            is_default_output: true,
            sample_rate: 48000.0,
            available_sample_rates: vec![44100.0, 48000.0, 96000.0],
            input_channels: 0,
            output_channels: 2,
        };

        let json = serde_json::to_string(&device).unwrap();
        let deserialized: AudioDeviceInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "MacBook Pro Speakers");
        assert_eq!(deserialized.uid, "BuiltInSpeakerDevice");
        assert_eq!(deserialized.kind, AudioDeviceKind::BuiltIn);
        assert_eq!(deserialized.direction, AudioDirection::Output);
        assert!(!deserialized.is_default_input);
        assert!(deserialized.is_default_output);
        assert_eq!(deserialized.sample_rate, 48000.0);
        assert_eq!(
            deserialized.available_sample_rates,
            vec![44100.0, 48000.0, 96000.0]
        );
        assert_eq!(deserialized.input_channels, 0);
        assert_eq!(deserialized.output_channels, 2);
    }

    #[test]
    fn detect_audio_devices_returns_vec() {
        let devices = detect_audio_devices();
        let _ = devices.len();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detect_audio_devices_on_macos() {
        // CoreAudio may report 0 channels in sandboxed test environments,
        // causing devices to be filtered out. When devices are found, verify
        // they have valid names and at least one has default status.
        let devices = detect_audio_devices();
        if devices.is_empty() {
            // Sandboxed test runner — skip assertions
            return;
        }

        for dev in &devices {
            assert!(!dev.name.is_empty(), "Device name should not be empty");
        }

        let has_default_output = devices.iter().any(|d| d.is_default_output);
        assert!(
            has_default_output,
            "Expected a default output device on macOS"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn alsa_cards_parses_card_entries() {
        let sample = "\
 0 [PCH            ]: HDA-Intel - HDA Intel PCH
                      HDA Intel PCH at 0xf7d34000 irq 34
 1 [NVidia         ]: HDA-Intel - HDA NVidia
                      HDA NVidia at 0xf7d80000 irq 17
";
        let cards = parse_alsa_cards(sample);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, 0);
        assert_eq!(cards[0].name, "HDA Intel PCH");
        assert_eq!(cards[1].id, 1);
        assert_eq!(cards[1].name, "HDA NVidia");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn alsa_pcm_detects_playback_and_capture() {
        let sample = "\
00-00: ALC257 Analog : ALC257 Analog : playback 1 : capture 1
00-02: ALC257 Alt Analog : ALC257 Alt Analog : capture 1
01-03: HDMI 0 : HDA Intel HDMI : playback 1
01-07: HDMI 1 : HDA Intel HDMI : playback 1
";
        let map = parse_alsa_pcm(sample);
        let card0 = map.get(&0).copied().unwrap_or_default();
        assert!(card0.has_playback);
        assert!(card0.has_capture);

        let card1 = map.get(&1).copied().unwrap_or_default();
        assert!(card1.has_playback);
        assert!(!card1.has_capture);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn alsa_pcm_capture_only_card() {
        let sample = "02-00: USB Mic : USB Audio : capture 1\n";
        let map = parse_alsa_pcm(sample);
        let card = map.get(&2).copied().unwrap_or_default();
        assert!(!card.has_playback);
        assert!(card.has_capture);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn classify_alsa_kind_matches_expected_categories() {
        assert_eq!(classify_alsa_kind("HDA Intel PCH"), AudioDeviceKind::BuiltIn);
        assert_eq!(
            classify_alsa_kind("HDA Intel HDMI"),
            AudioDeviceKind::Hdmi
        );
        assert_eq!(
            classify_alsa_kind("Blue Yeti USB Microphone"),
            AudioDeviceKind::Usb
        );
        assert_eq!(
            classify_alsa_kind("Sony WH-1000XM5 (Bluetooth)"),
            AudioDeviceKind::Bluetooth
        );
    }
}
