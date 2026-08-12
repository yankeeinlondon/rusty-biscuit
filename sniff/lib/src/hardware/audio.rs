//! Audio device detection module.
//!
//! Detects audio hardware devices (inputs, outputs) using platform-specific
//! APIs:
//! - **macOS**: CoreAudio for full device enumeration (channels, sample rates)
//! - **Linux**: Parses `/proc/asound/cards` for ALSA card names
//! - **Windows**: Queries `Win32_SoundDevice` via a PowerShell CIM probe
//!   (`Get-CimInstance`); the deprecated `wmic` tool is no longer used

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

    merge_split_io_devices(devices)
}

/// Strip a trailing `:\d+` from a UID, returning the prefix that remains.
///
/// CoreAudio exposes some devices (notably LG UltraFine displays) as two
/// separate `AudioObjectID`s with the same display name but distinct UIDs
/// that differ only in a trailing scope tag (`:1` for input, `:2` for output).
/// This function returns the shared prefix so such pairs can be matched.
#[cfg(any(target_os = "macos", test))]
fn uid_pair_key(uid: &str) -> &str {
    let Some(colon_pos) = uid.rfind(':') else {
        return uid;
    };
    let tail = &uid[colon_pos + 1..];
    if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
        &uid[..colon_pos]
    } else {
        uid
    }
}

/// Merge pairs of input-only and output-only devices that share a name and a
/// UID prefix (see [`uid_pair_key`]) into a single [`AudioDirection::InputOutput`]
/// entry. Devices that don't match a pair pass through unchanged.
#[cfg(any(target_os = "macos", test))]
fn merge_split_io_devices(devices: Vec<AudioDeviceInfo>) -> Vec<AudioDeviceInfo> {
    use std::collections::HashMap;

    // Index devices by (name, uid_pair_key) and collect slot indices per key.
    let mut buckets: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (idx, dev) in devices.iter().enumerate() {
        let key = (dev.name.clone(), uid_pair_key(&dev.uid).to_string());
        buckets.entry(key).or_default().push(idx);
    }

    let mut drop_mask = vec![false; devices.len()];
    let mut merged: Vec<(usize, AudioDeviceInfo)> = Vec::new();

    for (_key, indices) in buckets {
        if indices.len() != 2 {
            continue;
        }
        let (a, b) = (indices[0], indices[1]);
        let da = &devices[a];
        let db = &devices[b];

        let (input_dev, output_dev) = match (da.direction, db.direction) {
            (AudioDirection::Input, AudioDirection::Output) => (da, db),
            (AudioDirection::Output, AudioDirection::Input) => (db, da),
            _ => continue,
        };

        // Union available sample rates (sorted, deduped).
        let mut rates: Vec<f64> = input_dev
            .available_sample_rates
            .iter()
            .chain(output_dev.available_sample_rates.iter())
            .copied()
            .collect();
        rates.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        rates.dedup_by(|x, y| (*x - *y).abs() < 0.01);

        // Prefer the output device's current sample rate, else the input's.
        let sample_rate = if output_dev.sample_rate > 0.0 {
            output_dev.sample_rate
        } else {
            input_dev.sample_rate
        };

        // Anchor the merged entry at the earlier slot for stable ordering.
        let anchor = a.min(b);
        let merged_uid = uid_pair_key(&input_dev.uid).to_string();

        merged.push((
            anchor,
            AudioDeviceInfo {
                name: input_dev.name.clone(),
                uid: merged_uid,
                kind: input_dev.kind,
                direction: AudioDirection::InputOutput,
                is_default_input: input_dev.is_default_input,
                is_default_output: output_dev.is_default_output,
                sample_rate,
                available_sample_rates: rates,
                input_channels: input_dev.input_channels,
                output_channels: output_dev.output_channels,
            },
        ));
        drop_mask[a] = true;
        drop_mask[b] = true;
    }

    let mut result: Vec<(usize, AudioDeviceInfo)> = devices
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !drop_mask[*i])
        .collect();
    result.extend(merged);
    result.sort_by_key(|(i, _)| *i);
    result.into_iter().map(|(_, d)| d).collect()
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

    build_alsa_devices(cards, &directions)
}

/// Merges parsed ALSA cards with their PCM direction flags into audio devices.
///
/// A card absent from `directions` falls back to output-only. This is the path
/// taken when `/proc/asound/pcm` is missing or unreadable: the directions map is
/// empty, so every card defaults to playback, matching ALSA's convention that a
/// bare card exposes output.
#[cfg(target_os = "linux")]
fn build_alsa_devices(
    cards: Vec<AlsaCard>,
    directions: &std::collections::HashMap<u32, AlsaCardDirections>,
) -> Vec<AudioDeviceInfo> {
    let mut devices = Vec::with_capacity(cards.len());
    for card in cards {
        let dirs = directions
            .get(&card.id)
            .copied()
            .unwrap_or(AlsaCardDirections {
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

/// Classifies a Windows sound-device name into an [`AudioDeviceKind`].
///
/// Channel counts and sample rates are not available from `Win32_SoundDevice`,
/// so the kind is inferred from the device name alone.
#[cfg(any(target_os = "windows", test))]
fn classify_windows_audio_kind(name: &str) -> AudioDeviceKind {
    let upper = name.to_uppercase();
    if upper.contains("HDMI") {
        AudioDeviceKind::Hdmi
    } else if upper.contains("USB") {
        AudioDeviceKind::Usb
    } else if upper.contains("BLUETOOTH") {
        AudioDeviceKind::Bluetooth
    } else if upper.contains("VIRTUAL") {
        AudioDeviceKind::Virtual
    } else {
        AudioDeviceKind::Unknown
    }
}

/// Splits one CSV record into fields, honoring double-quoted fields with
/// embedded commas and `""`-escaped quotes — the format `ConvertTo-Csv` emits.
#[cfg(any(target_os = "windows", test))]
fn parse_csv_record(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => fields.push(std::mem::take(&mut field)),
            _ => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// Parses CSV output from
/// `Get-CimInstance Win32_SoundDevice | Select-Object Name,Status | ConvertTo-Csv`
/// into `(name, status)` pairs.
///
/// The `"Name","Status"` header row and any row without a non-empty `Name`
/// field are skipped, so blank, malformed, or name-less rows contribute no
/// devices. `Status` is captured for callers that want it but may be empty when
/// PowerShell reports a null status.
#[cfg(any(target_os = "windows", test))]
fn parse_cim_sound_devices(contents: &str) -> Vec<(String, String)> {
    let mut devices = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let fields = parse_csv_record(trimmed);
        // Skip the `"Name","Status"` header emitted by ConvertTo-Csv.
        if fields.first().is_some_and(|f| f == "Name") {
            continue;
        }
        let Some(name) = fields.first().map(|f| f.trim()).filter(|f| !f.is_empty()) else {
            continue;
        };
        let status = fields
            .get(1)
            .map(|f| f.trim().to_string())
            .unwrap_or_default();
        devices.push((name.to_string(), status));
    }
    devices
}

/// Runs a PowerShell command, returning stdout on success or `None` on missing
/// PowerShell, non-zero exit, or timeout.
///
/// The probe is best-effort: any failure reports no devices rather than
/// hanging.
#[cfg(any(target_os = "windows", test))]
fn powershell_audio_args(command: &str) -> [&str; 4] {
    ["-NoProfile", "-NonInteractive", "-Command", command]
}

#[cfg(target_os = "windows")]
fn run_powershell_with_timeout(command: &str, timeout: std::time::Duration) -> Option<String> {
    let output = crate::process::run_with_timeout(
        "powershell",
        &powershell_audio_args(command),
        timeout,
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(output.stdout_lossy().into_owned())
}

/// Detects audio devices on Windows via a PowerShell CIM probe.
///
/// Queries `Win32_SoundDevice` with `Get-CimInstance` for device names and
/// status. Device kind is inferred from the name (USB, Bluetooth, HDMI).
/// Channel counts and sample rates are not available from this source. Missing
/// PowerShell, a failed query, or a parse failure reports no devices.
#[cfg(target_os = "windows")]
pub fn detect_audio_devices() -> Vec<AudioDeviceInfo> {
    const COMMAND: &str = "Get-CimInstance Win32_SoundDevice | Select-Object Name,Status | ConvertTo-Csv -NoTypeInformation";

    let Some(output) =
        run_powershell_with_timeout(COMMAND, crate::process::timeouts::WINDOWS_AUDIO)
    else {
        return Vec::new();
    };

    parse_cim_sound_devices(&output)
        .into_iter()
        .enumerate()
        .map(|(idx, (name, _status))| AudioDeviceInfo {
            kind: classify_windows_audio_kind(&name),
            name,
            uid: format!("win-audio-{idx}"),
            direction: AudioDirection::Output,
            ..Default::default()
        })
        .collect()
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
    fn alsa_missing_pcm_falls_back_to_output_only() {
        // An empty directions map stands in for a missing or unreadable
        // /proc/asound/pcm. Every detected card must then default to
        // output-only rather than disappearing.
        let cards = parse_alsa_cards(" 0 [PCH            ]: HDA-Intel - HDA Intel PCH\n");
        let directions = std::collections::HashMap::new();
        let devices = build_alsa_devices(cards, &directions);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].direction, AudioDirection::Output);
    }

    #[test]
    fn uid_pair_key_strips_trailing_numeric_segment() {
        assert_eq!(
            uid_pair_key("AppleUSBAudioEngine:LG:USB Audio:20141000:1"),
            "AppleUSBAudioEngine:LG:USB Audio:20141000"
        );
        assert_eq!(
            uid_pair_key("AppleUSBAudioEngine:LG:USB Audio:20141000:2"),
            "AppleUSBAudioEngine:LG:USB Audio:20141000"
        );
    }

    #[test]
    fn uid_pair_key_preserves_non_numeric_tail() {
        assert_eq!(uid_pair_key("BuiltInSpeakerDevice"), "BuiltInSpeakerDevice");
        assert_eq!(uid_pair_key("foo:bar"), "foo:bar");
        assert_eq!(uid_pair_key(""), "");
    }

    #[test]
    fn merge_pairs_input_only_and_output_only_with_shared_prefix() {
        let devices = vec![
            AudioDeviceInfo {
                name: "LG UltraFine Display Audio".into(),
                uid: "AppleUSBAudioEngine:LG:20141000:1".into(),
                kind: AudioDeviceKind::Usb,
                direction: AudioDirection::Input,
                input_channels: 1,
                output_channels: 0,
                sample_rate: 48000.0,
                available_sample_rates: vec![48000.0],
                ..Default::default()
            },
            AudioDeviceInfo {
                name: "LG UltraFine Display Audio".into(),
                uid: "AppleUSBAudioEngine:LG:20141000:2".into(),
                kind: AudioDeviceKind::Usb,
                direction: AudioDirection::Output,
                input_channels: 0,
                output_channels: 2,
                sample_rate: 48000.0,
                available_sample_rates: vec![44100.0, 48000.0],
                is_default_output: true,
                ..Default::default()
            },
        ];
        let merged = merge_split_io_devices(devices);
        assert_eq!(merged.len(), 1);
        let d = &merged[0];
        assert_eq!(d.name, "LG UltraFine Display Audio");
        assert_eq!(d.uid, "AppleUSBAudioEngine:LG:20141000");
        assert_eq!(d.direction, AudioDirection::InputOutput);
        assert_eq!(d.input_channels, 1);
        assert_eq!(d.output_channels, 2);
        assert_eq!(d.available_sample_rates, vec![44100.0, 48000.0]);
        assert!(d.is_default_output);
    }

    #[test]
    fn merge_pairs_two_physical_devices_independently() {
        let mk = |uid: &str, dir: AudioDirection, ic, oc| AudioDeviceInfo {
            name: "LG UltraFine Display Audio".into(),
            uid: uid.into(),
            kind: AudioDeviceKind::Usb,
            direction: dir,
            input_channels: ic,
            output_channels: oc,
            ..Default::default()
        };
        let devices = vec![
            mk("Engine:20141000:1", AudioDirection::Input, 1, 0),
            mk("Engine:20141000:2", AudioDirection::Output, 0, 2),
            mk("Engine:21141000:1", AudioDirection::Input, 1, 0),
            mk("Engine:21141000:2", AudioDirection::Output, 0, 2),
        ];
        let merged = merge_split_io_devices(devices);
        assert_eq!(merged.len(), 2);
        assert!(
            merged
                .iter()
                .all(|d| d.direction == AudioDirection::InputOutput)
        );
        let keys: Vec<&str> = merged.iter().map(|d| d.uid.as_str()).collect();
        assert!(keys.contains(&"Engine:20141000"));
        assert!(keys.contains(&"Engine:21141000"));
    }

    #[test]
    fn merge_leaves_unpaired_devices_untouched() {
        let devices = vec![
            AudioDeviceInfo {
                name: "MacBook Pro Speakers".into(),
                uid: "BuiltInSpeaker".into(),
                direction: AudioDirection::Output,
                output_channels: 2,
                ..Default::default()
            },
            AudioDeviceInfo {
                name: "MacBook Pro Microphone".into(),
                uid: "BuiltInMic".into(),
                direction: AudioDirection::Input,
                input_channels: 1,
                ..Default::default()
            },
        ];
        let merged = merge_split_io_devices(devices.clone());
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, devices[0].name);
        assert_eq!(merged[1].name, devices[1].name);
    }

    #[test]
    fn merge_does_not_collapse_two_input_only_same_name() {
        let mk = |uid: &str| AudioDeviceInfo {
            name: "Clone".into(),
            uid: uid.into(),
            direction: AudioDirection::Input,
            input_channels: 1,
            ..Default::default()
        };
        // Same pair_key but both are input-only — not a valid merge.
        let merged = merge_split_io_devices(vec![mk("prefix:1"), mk("prefix:2")]);
        // pair_keys differ here ("prefix" after stripping both), so they'd group,
        // but directions are both Input — merge is skipped.
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|d| d.direction == AudioDirection::Input));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn classify_alsa_kind_matches_expected_categories() {
        assert_eq!(
            classify_alsa_kind("HDA Intel PCH"),
            AudioDeviceKind::BuiltIn
        );
        assert_eq!(classify_alsa_kind("HDA Intel HDMI"), AudioDeviceKind::Hdmi);
        assert_eq!(
            classify_alsa_kind("Blue Yeti USB Microphone"),
            AudioDeviceKind::Usb
        );
        assert_eq!(
            classify_alsa_kind("Sony WH-1000XM5 (Bluetooth)"),
            AudioDeviceKind::Bluetooth
        );
    }

    #[test]
    fn cim_parses_healthy_devices() {
        let sample = "\"Name\",\"Status\"\n\
\"Realtek High Definition Audio\",\"OK\"\n\
\"NVIDIA High Definition Audio\",\"OK\"\n";
        let devices = parse_cim_sound_devices(sample);
        assert_eq!(devices.len(), 2);
        assert_eq!(
            devices[0],
            ("Realtek High Definition Audio".to_string(), "OK".to_string())
        );
        assert_eq!(devices[1].0, "NVIDIA High Definition Audio");
        assert_eq!(devices[1].1, "OK");
    }

    #[test]
    fn cim_handles_missing_status() {
        // PowerShell renders a null Status as an empty quoted field; a row with
        // no Status column at all must still yield the device with empty status.
        let sample = "\"Name\",\"Status\"\n\
\"USB Audio Device\",\"\"\n\
\"Bare Name Without Status Column\"\n";
        let devices = parse_cim_sound_devices(sample);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0], ("USB Audio Device".to_string(), String::new()));
        assert_eq!(
            devices[1],
            ("Bare Name Without Status Column".to_string(), String::new())
        );
    }

    #[test]
    fn cim_skips_malformed_and_empty_rows() {
        // A blank line, a quoted-empty name, and a whitespace-only name all
        // contribute no device.
        let sample = "\"Name\",\"Status\"\n\
\n\
\"\",\"OK\"\n\
\"   \",\"OK\"\n\
\"Speakers (High Definition Audio)\",\"OK\"\n";
        let devices = parse_cim_sound_devices(sample);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].0, "Speakers (High Definition Audio)");
    }

    #[test]
    fn cim_preserves_commas_inside_quoted_names() {
        let sample = "\"Name\",\"Status\"\n\"Speakers, Rear\",\"OK\"\n";
        let devices = parse_cim_sound_devices(sample);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].0, "Speakers, Rear");
    }

    #[test]
    fn classify_windows_audio_kind_matches_categories() {
        assert_eq!(
            classify_windows_audio_kind("Intel HDMI Audio"),
            AudioDeviceKind::Hdmi
        );
        assert_eq!(
            classify_windows_audio_kind("Generic USB Audio"),
            AudioDeviceKind::Usb
        );
        assert_eq!(
            classify_windows_audio_kind("Bluetooth Hands-Free"),
            AudioDeviceKind::Bluetooth
        );
        assert_eq!(
            classify_windows_audio_kind("VB-Audio Virtual Cable"),
            AudioDeviceKind::Virtual
        );
        assert_eq!(
            classify_windows_audio_kind("Realtek High Definition Audio"),
            AudioDeviceKind::Unknown
        );
    }

    #[test]
    fn windows_audio_probe_constructs_noninteractive_powershell_command() {
        assert_eq!(
            powershell_audio_args("Get-CimInstance Win32_SoundDevice"),
            [
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_SoundDevice"
            ]
        );
        assert_eq!(
            crate::process::timeouts::WINDOWS_AUDIO,
            std::time::Duration::from_secs(5)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_audio_probe_drains_large_stdout_and_stderr() {
        let output = run_powershell_with_timeout(
            "[Console]::Out.Write(('o' * 1048576 -join '')); [Console]::Error.Write(('e' * 1048576 -join ''))",
            std::time::Duration::from_secs(30),
        )
        .expect("verbose PowerShell probe should complete");

        assert_eq!(output.len(), 1_048_576);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_audio_probe_honors_injected_deadline() {
        let started = std::time::Instant::now();
        let output = run_powershell_with_timeout(
            "Start-Sleep -Seconds 30",
            std::time::Duration::from_millis(100),
        );

        assert!(output.is_none());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
}
