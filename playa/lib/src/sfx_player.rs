//! Native sound effects playback via rodio.
//!
//! When the `sfx-native` feature is enabled, sound effects are played directly
//! through the OS audio subsystem using rodio instead of spawning external player
//! processes. This provides lower latency and enables OS audio channel routing.
//!
//! ## macOS System Sound Device
//!
//! With the `sfx-native-macos` feature, sound effects are routed through the
//! macOS "system sound" output device. This is the device configured in
//! System Settings → Sound → "Play sound effects through". It may differ from
//! the default music output device (e.g., built-in speakers for alerts while
//! headphones handle music).
//!
//! ## Windows Sound Effects Category
//!
//! With the `sfx-native-windows` feature, sound effects are played through
//! WASAPI with `AudioCategory_SoundEffects`. This tags the audio stream so
//! Windows routes it through the Sound Effects mixer category, which has its
//! own volume slider in the Volume Mixer. Falls back to the default rodio
//! output if WASAPI setup fails.
//!
//! ## Linux
//!
//! With the `sfx-native-linux` feature, sound effects are played through
//! PulseAudio (or PipeWire's PulseAudio compatibility layer) with
//! `media.role=event` set on the stream. PipeWire's `module-role-ducking`
//! and PulseAudio's equivalent use this property to optionally duck other
//! audio during event sounds. Falls back to the default rodio output if
//! PulseAudio is not available (e.g., ALSA-only systems).

use std::io::Cursor;

use rodio::{Decoder, DeviceSinkBuilder, Player};
use thiserror::Error;

use crate::types::PlaybackOptions;

/// Errors from native SFX playback.
#[derive(Debug, Error)]
pub enum SfxPlaybackError {
    /// Failed to open an audio output stream.
    #[error("failed to open audio stream: {0}")]
    Stream(#[from] rodio::DeviceSinkError),

    /// Failed to decode audio data.
    #[error("failed to decode audio: {0}")]
    Decode(#[from] rodio::decoder::DecoderError),

    /// Failed to play audio.
    #[error("failed to play audio: {0}")]
    Play(#[from] rodio::PlayError),
}

/// Play sound effect bytes using native audio (rodio).
///
/// Supports volume and speed control via `PlaybackOptions`. On macOS with
/// `sfx-native-macos`, routes audio to the system sound device (which the
/// user can configure separately from the default output in System Settings
/// → Sound). Falls back to the default output device if the system sound
/// device can't be found or is the same as the default.
///
/// ## Errors
///
/// Returns `SfxPlaybackError` if the audio stream can't be opened or the
/// audio data can't be decoded. Callers should fall back to host player
/// delegation on error.
pub fn play_sfx(bytes: &[u8], options: &PlaybackOptions) -> Result<(), SfxPlaybackError> {
    // Windows: play through WASAPI with AudioCategory_SoundEffects.
    #[cfg(all(target_os = "windows", feature = "sfx-native-windows"))]
    {
        if windows_sfx::play_sfx_with_category(bytes, options).is_ok() {
            return Ok(());
        }
        // Fall through to default rodio path on error.
    }

    // Linux: play through PulseAudio with media.role=event.
    #[cfg(all(target_os = "linux", feature = "sfx-native-linux"))]
    {
        if linux::play_sfx_as_event(bytes, options).is_ok() {
            return Ok(());
        }
        // Fall through to default rodio path on error.
    }

    let mut stream = open_sfx_stream()?;
    stream.log_on_drop(false);
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = options.volume {
        player.set_volume(vol);
    }
    if let Some(speed) = options.speed {
        player.set_speed(speed);
    }

    let source = Decoder::new(Cursor::new(bytes.to_vec()))?;
    player.append(source);
    player.sleep_until_end();

    Ok(())
}

/// Open an audio output stream targeting the OS sound effects channel.
///
/// On macOS with `sfx-native-macos`, attempts to route to the system sound
/// device. Falls back to the default output device on all other platforms
/// or if device lookup fails.
pub(crate) fn open_sfx_stream() -> Result<rodio::MixerDeviceSink, rodio::DeviceSinkError> {
    #[cfg(all(target_os = "macos", feature = "sfx-native-macos"))]
    {
        if let Ok(Some(device)) = macos::find_system_sound_device()
            && let Ok(stream) = DeviceSinkBuilder::from_device(device).and_then(|b| b.open_stream())
        {
            return Ok(stream);
        }
    }

    DeviceSinkBuilder::open_default_sink()
}

// ============================================================================
// macOS: System sound device routing
// ============================================================================

#[cfg(all(target_os = "macos", feature = "sfx-native-macos"))]
pub(crate) mod macos {
    use std::ffi::CStr;
    use std::mem;

    use coreaudio_sys::{
        AudioObjectGetPropertyData, AudioObjectID, AudioObjectPropertyAddress,
        kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyElementMain,
        kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
    };
    use rodio::DeviceTrait;

    // CoreAudio property selectors not always exported by coreaudio-sys.
    // kAudioHardwarePropertyDefaultSystemOutputDevice = 'sOut'
    const SYSTEM_OUTPUT_DEVICE_SELECTOR: u32 =
        ((b's' as u32) << 24) | ((b'O' as u32) << 16) | ((b'u' as u32) << 8) | (b't' as u32);

    // kAudioObjectPropertyName = 'lnam' (returns CFStringRef)
    const DEVICE_NAME_SELECTOR: u32 =
        ((b'l' as u32) << 24) | ((b'n' as u32) << 16) | ((b'a' as u32) << 8) | (b'm' as u32);

    // kAudioDevicePropertyDeviceUID = 'uid ' (returns CFStringRef)
    // Unique persistent identifier for a CoreAudio device.
    const DEVICE_UID_SELECTOR: u32 =
        ((b'u' as u32) << 24) | ((b'i' as u32) << 16) | ((b'd' as u32) << 8) | (b' ' as u32);

    // CoreFoundation FFI for CFStringRef → String conversion.
    // Using inline declarations to avoid a `core-foundation-sys` dependency.
    type CFStringRef = *const std::ffi::c_void;
    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    unsafe extern "C" {
        fn CFStringGetLength(the_string: CFStringRef) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut i8,
            buffer_size: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(cf: *const std::ffi::c_void);
    }

    /// Find the macOS system sound output device as a rodio `Device`.
    ///
    /// Returns `Ok(None)` if the system sound device is the same as the default
    /// output device (the common case), meaning callers should just use the
    /// default rodio output.
    ///
    /// Returns `Ok(Some(device))` when the system sound device differs and a
    /// matching cpal device was found.
    ///
    /// Uses CoreAudio device UID for disambiguation when multiple sub-devices
    /// share the same name (e.g., two "LG UltraFine Display Audio" entries).
    /// Determines which same-name occurrence to pick by checking UIDs of all
    /// CoreAudio devices with the target name.
    pub fn find_system_sound_device() -> Result<Option<rodio::Device>, Box<dyn std::error::Error>> {
        let system_device_id = get_audio_device(SYSTEM_OUTPUT_DEVICE_SELECTOR)?;
        let default_device_id = get_audio_device(kAudioHardwarePropertyDefaultOutputDevice)?;

        // Common case: same device, no special routing needed.
        if system_device_id == default_device_id {
            return Ok(None);
        }

        // Devices differ — find the correct cpal device by UID-based index.
        let system_name =
            get_device_name(system_device_id).ok_or("failed to get system sound device name")?;
        let system_uid =
            get_device_uid(system_device_id).ok_or("failed to get system sound device UID")?;

        // Enumerate all CoreAudio devices and find which output-capable
        // device with the target name has the matching UID. Track its
        // position among same-name output devices.
        let all_ids = get_all_device_ids()?;
        let mut same_name_index: Option<usize> = None;
        let mut counter = 0usize;

        for &ca_id in &all_ids {
            if !has_output_streams(ca_id) {
                continue;
            }
            if get_device_name(ca_id).as_deref() != Some(&system_name) {
                continue;
            }
            if get_device_uid(ca_id).as_deref() == Some(&system_uid) {
                same_name_index = Some(counter);
                break;
            }
            counter += 1;
        }

        let target_index = match same_name_index {
            Some(idx) => idx,
            None => return Ok(None),
        };

        // Now pick the nth cpal device with the matching name.
        let host = rodio::cpal::default_host();
        let devices = rodio::cpal::traits::HostTrait::output_devices(&host)?;
        let mut name_counter = 0usize;

        for device in devices {
            if let Ok(desc) = device.description()
                && desc.name() == system_name
            {
                if name_counter == target_index {
                    return Ok(Some(device));
                }
                name_counter += 1;
            }
        }

        // Device not found via cpal - fall back to default.
        Ok(None)
    }

    /// Query a hardware-level audio device property that returns an AudioObjectID.
    fn get_audio_device(selector: u32) -> Result<AudioObjectID, String> {
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
                &mut device_id as *mut _ as *mut _,
            );

            if status != 0 {
                return Err(format!("CoreAudio error: OSStatus {status}"));
            }
            if device_id == 0 {
                return Err("no audio device found".to_string());
            }

            Ok(device_id)
        }
    }

    /// Get all CoreAudio device IDs.
    fn get_all_device_ids() -> Result<Vec<AudioObjectID>, String> {
        unsafe {
            let address = AudioObjectPropertyAddress {
                mSelector: coreaudio_sys::kAudioHardwarePropertyDevices,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            // Query size using AudioObjectGetPropertyDataSize.
            let mut data_size: u32 = 0;
            let status = coreaudio_sys::AudioObjectGetPropertyDataSize(
                kAudioObjectSystemObject,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
            );
            if status != 0 {
                return Err(format!(
                    "CoreAudio error querying device list size: OSStatus {status}"
                ));
            }

            let count = data_size as usize / mem::size_of::<AudioObjectID>();
            let mut device_ids = vec![0u32; count];

            let status = AudioObjectGetPropertyData(
                kAudioObjectSystemObject,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                device_ids.as_mut_ptr() as *mut _,
            );
            if status != 0 {
                return Err(format!(
                    "CoreAudio error fetching device list: OSStatus {status}"
                ));
            }

            Ok(device_ids)
        }
    }

    /// Check whether a CoreAudio device has output streams.
    fn has_output_streams(device_id: AudioObjectID) -> bool {
        unsafe {
            let address = AudioObjectPropertyAddress {
                mSelector: coreaudio_sys::kAudioDevicePropertyStreams,
                mScope: coreaudio_sys::kAudioObjectPropertyScopeOutput,
                mElement: kAudioObjectPropertyElementMain,
            };
            let mut data_size: u32 = 0;
            let status = coreaudio_sys::AudioObjectGetPropertyDataSize(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
            );
            status == 0 && data_size > 0
        }
    }

    /// Get the unique persistent UID of a CoreAudio device.
    fn get_device_uid(device_id: AudioObjectID) -> Option<String> {
        get_device_cfstring_property(device_id, DEVICE_UID_SELECTOR)
    }

    /// Get the human-readable name of a CoreAudio device.
    fn get_device_name(device_id: AudioObjectID) -> Option<String> {
        get_device_cfstring_property(device_id, DEVICE_NAME_SELECTOR)
    }

    /// Query a CFStringRef property from a CoreAudio device.
    fn get_device_cfstring_property(device_id: AudioObjectID, selector: u32) -> Option<String> {
        unsafe {
            let mut value: CFStringRef = std::ptr::null();
            let mut data_size = mem::size_of::<CFStringRef>() as u32;

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
                &mut value as *mut _ as *mut _,
            );

            if status != 0 || value.is_null() {
                return None;
            }

            let result = cfstring_to_string(value);
            CFRelease(value);
            result
        }
    }

    /// Convert a CoreFoundation CFStringRef to a Rust String.
    unsafe fn cfstring_to_string(cf_string: CFStringRef) -> Option<String> {
        let length = unsafe { CFStringGetLength(cf_string) };
        let max_size =
            unsafe { CFStringGetMaximumSizeForEncoding(length, K_CF_STRING_ENCODING_UTF8) } + 1;
        let mut buf = vec![0i8; max_size as usize];

        if unsafe {
            CFStringGetCString(
                cf_string,
                buf.as_mut_ptr(),
                max_size,
                K_CF_STRING_ENCODING_UTF8,
            )
        } != 0
        {
            let c_str = unsafe { CStr::from_ptr(buf.as_ptr()) };
            Some(c_str.to_string_lossy().into_owned())
        } else {
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn system_output_device_selector_is_correct() {
            // 'sOut' = 0x734F7574
            assert_eq!(SYSTEM_OUTPUT_DEVICE_SELECTOR, 0x734F_7574);
        }

        #[test]
        fn device_name_selector_is_correct() {
            // 'lnam' = 0x6C6E616D
            assert_eq!(DEVICE_NAME_SELECTOR, 0x6C6E_616D);
        }

        #[test]
        fn device_uid_selector_is_correct() {
            // 'uid ' = 0x75696420
            assert_eq!(DEVICE_UID_SELECTOR, 0x7569_6420);
        }

        #[test]
        #[ignore = "requires real audio device"]
        fn can_query_system_sound_device() {
            let result = get_audio_device(SYSTEM_OUTPUT_DEVICE_SELECTOR);
            assert!(result.is_ok(), "should find system sound device");
            let device_id = result.unwrap();
            assert_ne!(device_id, 0);
        }

        #[test]
        #[ignore = "requires real audio device"]
        fn can_get_device_name() {
            let device_id = get_audio_device(SYSTEM_OUTPUT_DEVICE_SELECTOR).unwrap();
            let name = get_device_name(device_id);
            assert!(name.is_some(), "should get device name");
            let name = name.unwrap();
            assert!(!name.is_empty(), "device name should not be empty");
        }

        #[test]
        #[ignore = "requires real audio device"]
        fn can_get_device_uid() {
            let device_id = get_audio_device(SYSTEM_OUTPUT_DEVICE_SELECTOR).unwrap();
            let uid = get_device_uid(device_id);
            assert!(uid.is_some(), "should get device UID");
            let uid = uid.unwrap();
            assert!(!uid.is_empty(), "device UID should not be empty");
        }

        #[test]
        #[ignore = "requires real audio device"]
        fn find_system_sound_device_returns_result() {
            // Should succeed regardless of whether devices differ.
            let result = find_system_sound_device();
            assert!(result.is_ok());
        }
    }
}

// ============================================================================
// Windows: WASAPI sound effects category
// ============================================================================

#[cfg(all(target_os = "windows", feature = "sfx-native-windows"))]
mod windows_sfx {
    use std::io::Cursor;

    use rodio::Source;
    use windows::Win32::{
        Media::{
            Audio::{
                AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, AudioCategory_SoundEffects,
                AudioClientProperties, IAudioClient2, IAudioRenderClient, IMMDeviceEnumerator,
                MMDeviceEnumerator, WAVEFORMATEX, eMultimedia, eRender,
            },
            Multimedia::WAVE_FORMAT_IEEE_FLOAT,
        },
        System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx},
    };

    use crate::types::PlaybackOptions;

    /// Play sound effect bytes through WASAPI with `AudioCategory_SoundEffects`.
    ///
    /// Decodes the audio, then plays it through a WASAPI shared-mode stream
    /// tagged with the Sound Effects audio category. Windows shows this stream
    /// separately in the Volume Mixer, letting users control SFX volume
    /// independently of other audio.
    ///
    /// Uses `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` so WASAPI handles sample rate
    /// and channel conversion from the decoded format to the device format.
    /// Speed is implemented by adjusting the declared sample rate (higher rate =
    /// faster playback with proportional pitch shift).
    pub fn play_sfx_with_category(
        bytes: &[u8],
        options: &PlaybackOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Decode audio using rodio's decoder.
        let decoder = rodio::Decoder::new(Cursor::new(bytes.to_vec()))?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        // Extract primitive values from NonZero types.
        let channels_u16 = channels.get();
        let sample_rate_u32 = sample_rate.get();

        // Collect all samples as f32 (fine for short SFX clips).
        let mut samples: Vec<f32> = decoder.collect();

        // Apply volume.
        if let Some(vol) = options.volume {
            for s in &mut samples {
                *s *= vol;
            }
        }

        // Speed: adjust the declared sample rate. WASAPI's auto-conversion
        // resamples from our declared rate to the device rate. Declaring a
        // higher rate makes WASAPI treat the samples as "faster", producing
        // the standard speed-with-pitch-shift effect.
        let effective_rate = if let Some(speed) = options.speed {
            (sample_rate_u32 as f32 * speed) as u32
        } else {
            sample_rate_u32
        };

        unsafe {
            // Initialize COM (ignore error if already initialized on this thread).
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            // Get default audio render endpoint.
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            // Activate IAudioClient2 (supports SetClientProperties).
            let client: IAudioClient2 = device.Activate(CLSCTX_ALL, None)?;

            // Tag this stream as Sound Effects.
            let props = AudioClientProperties {
                cbSize: std::mem::size_of::<AudioClientProperties>() as u32,
                bIsOffload: false.into(),
                eCategory: AudioCategory_SoundEffects,
                Options: Default::default(),
            };
            client.SetClientProperties(&props)?;

            // Describe our decoded audio format (float32 PCM).
            let bytes_per_sample = 4u16; // f32
            let block_align = channels_u16 * bytes_per_sample;
            let format = WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_IEEE_FLOAT as u16,
                nChannels: channels_u16,
                nSamplesPerSec: effective_rate,
                nAvgBytesPerSec: effective_rate * block_align as u32,
                nBlockAlign: block_align,
                wBitsPerSample: 32,
                cbSize: 0,
            };

            // Initialize shared-mode stream with automatic format conversion.
            // WASAPI converts from our declared format to the device's native format.
            let buffer_duration = 10_000_000i64; // 1 second in 100ns REFERENCE_TIME units
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                buffer_duration,
                0, // periodicity (must be 0 for shared mode)
                &format,
                None,
            )?;

            let buffer_frames = client.GetBufferSize()?;
            let render: IAudioRenderClient = client.GetService()?;

            client.Start()?;

            // Render loop: write decoded samples to the WASAPI buffer.
            let ch = channels_u16 as usize;
            let total_frames = samples.len() / ch;
            let mut frame_offset = 0usize;

            while frame_offset < total_frames {
                let padding = client.GetCurrentPadding()?;
                let available = buffer_frames - padding;

                if available == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }

                let frames_to_write = available.min((total_frames - frame_offset) as u32);
                let buf_ptr = render.GetBuffer(frames_to_write)?;

                let dst = std::slice::from_raw_parts_mut(
                    buf_ptr as *mut f32,
                    frames_to_write as usize * ch,
                );
                let src_start = frame_offset * ch;
                let src_end = (src_start + dst.len()).min(samples.len());
                let copy_len = src_end - src_start;
                dst[..copy_len].copy_from_slice(&samples[src_start..src_end]);
                // Zero any trailing samples (end of audio).
                if copy_len < dst.len() {
                    dst[copy_len..].fill(0.0);
                }

                render.ReleaseBuffer(frames_to_write, 0)?;
                frame_offset += frames_to_write as usize;
            }

            // Drain: wait for the remaining buffered audio to play.
            let drain_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let padding = client.GetCurrentPadding()?;
                if padding == 0 || std::time::Instant::now() > drain_deadline {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            client.Stop()?;
        }

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "requires Windows audio device"]
        fn can_get_default_audio_endpoint() {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                        .expect("should create device enumerator");
                let device = enumerator
                    .GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .expect("should find default audio endpoint");
                let _client: IAudioClient2 = device
                    .Activate(CLSCTX_ALL, None)
                    .expect("should activate audio client");
            }
        }

        #[test]
        #[ignore = "requires Windows audio device"]
        fn can_set_sound_effects_category() {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
                let device = enumerator
                    .GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .unwrap();
                let client: IAudioClient2 = device.Activate(CLSCTX_ALL, None).unwrap();

                let props = AudioClientProperties {
                    cbSize: std::mem::size_of::<AudioClientProperties>() as u32,
                    bIsOffload: false.into(),
                    eCategory: AudioCategory_SoundEffects,
                    Options: Default::default(),
                };
                client
                    .SetClientProperties(&props)
                    .expect("should set SoundEffects category");
            }
        }
    }
}

// ============================================================================
// Linux: PulseAudio media.role=event tagging
// ============================================================================

#[cfg(all(target_os = "linux", feature = "sfx-native-linux"))]
mod linux {
    use std::io::Cursor;

    use libpulse_binding as pulse;
    use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
    use pulse::context::Context;
    use pulse::proplist::Proplist;
    use pulse::sample::{Format, Spec};
    use pulse::stream::{SeekMode, Stream};
    use rodio::Source;

    use crate::types::PlaybackOptions;

    /// Play sound effect bytes through PulseAudio with `media.role=event`.
    ///
    /// Decodes the audio with rodio, then plays through a PulseAudio stream
    /// tagged with `media.role=event`. PipeWire's `module-role-ducking` and
    /// PulseAudio's `module-role-ducking` use this property to optionally duck
    /// other audio during event sounds.
    ///
    /// Speed is implemented by adjusting the declared sample rate (higher rate =
    /// faster playback with proportional pitch shift).
    pub fn play_sfx_as_event(
        bytes: &[u8],
        options: &PlaybackOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Decode audio using rodio's decoder.
        let decoder = rodio::Decoder::new(Cursor::new(bytes.to_vec()))?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        // Extract primitive values from NonZero types.
        let channels_u8 = channels.get() as u8;
        let sample_rate_u32 = sample_rate.get();

        // Collect all samples as f32 (fine for short SFX clips).
        let mut samples: Vec<f32> = decoder.collect();

        // Apply volume.
        if let Some(vol) = options.volume {
            for s in &mut samples {
                *s *= vol;
            }
        }

        // Speed: adjust the declared sample rate. PulseAudio resamples from our
        // declared rate to the device rate, producing the speed-with-pitch effect.
        let effective_rate = if let Some(speed) = options.speed {
            (sample_rate_u32 as f32 * speed) as u32
        } else {
            sample_rate_u32
        };

        let spec = Spec {
            format: Format::F32le,
            channels: channels_u8,
            rate: effective_rate,
        };

        if !spec.is_valid() {
            return Err("invalid PulseAudio sample spec".into());
        }

        // Create PulseAudio mainloop (single-threaded, we iterate manually).
        let mut mainloop = Mainloop::new().ok_or("failed to create PulseAudio mainloop")?;

        // Create and connect context.
        let mut context =
            Context::new(&mainloop, "playa").ok_or("failed to create PulseAudio context")?;

        context.connect(None, pulse::context::FlagSet::NOFLAGS, None)?;

        // Wait for context to become ready.
        loop {
            iterate_or_fail(&mut mainloop)?;
            match context.get_state() {
                pulse::context::State::Ready => break,
                pulse::context::State::Failed | pulse::context::State::Terminated => {
                    return Err("PulseAudio context connection failed".into());
                }
                _ => {}
            }
        }

        // Create stream proplist with media.role=event.
        let mut proplist = Proplist::new().ok_or("failed to create PulseAudio proplist")?;
        proplist
            .set_str(pulse::proplist::properties::MEDIA_ROLE, "event")
            .map_err(|()| "failed to set media.role property")?;

        let mut stream =
            Stream::new_with_proplist(&mut context, "Sound Effect", &spec, None, &mut proplist)
                .ok_or("failed to create PulseAudio stream")?;

        // Connect stream for playback.
        stream.connect_playback(
            None, // default device
            None, // default buffer attributes
            pulse::stream::FlagSet::NOFLAGS,
            None, // no volume override
            None, // no sync stream
        )?;

        // Wait for stream to become ready.
        loop {
            iterate_or_fail(&mut mainloop)?;
            match stream.get_state() {
                pulse::stream::State::Ready => break,
                pulse::stream::State::Failed | pulse::stream::State::Terminated => {
                    return Err("PulseAudio stream connection failed".into());
                }
                _ => {}
            }
        }

        // Convert f32 samples to little-endian byte representation.
        let byte_data: Vec<u8> = samples.iter().flat_map(|s: &f32| s.to_le_bytes()).collect();

        // Write all audio data. PulseAudio buffers internally.
        stream.write(&byte_data, None, 0, SeekMode::Relative)?;

        // Drain: wait for all buffered audio to finish playing.
        let op = stream.drain(None);
        loop {
            iterate_or_fail(&mut mainloop)?;
            match op.get_state() {
                pulse::operation::State::Done | pulse::operation::State::Cancelled => break,
                pulse::operation::State::Running => {}
            }
        }

        // Clean up (Drop impls handle the rest).
        stream.disconnect().ok();
        context.disconnect();

        Ok(())
    }

    /// Iterate the PulseAudio mainloop once, blocking until an event arrives.
    fn iterate_or_fail(mainloop: &mut Mainloop) -> Result<(), Box<dyn std::error::Error>> {
        match mainloop.iterate(true) {
            IterateResult::Success(_) => Ok(()),
            IterateResult::Err(e) => Err(format!("PulseAudio mainloop error: {e}").into()),
            IterateResult::Quit(_) => Err("PulseAudio mainloop quit unexpectedly".into()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn valid_sample_spec() {
            let spec = Spec {
                format: Format::F32le,
                channels: 2,
                rate: 44100,
            };
            assert!(spec.is_valid());
        }

        #[test]
        fn invalid_sample_spec_zero_rate() {
            let spec = Spec {
                format: Format::F32le,
                channels: 2,
                rate: 0,
            };
            assert!(!spec.is_valid());
        }

        #[test]
        fn invalid_sample_spec_zero_channels() {
            let spec = Spec {
                format: Format::F32le,
                channels: 0,
                rate: 44100,
            };
            assert!(!spec.is_valid());
        }

        #[test]
        fn proplist_can_set_media_role() {
            let mut proplist = Proplist::new().expect("should create proplist");
            let result = proplist.set_str(pulse::proplist::properties::MEDIA_ROLE, "event");
            assert!(result.is_ok());
        }

        #[test]
        #[ignore = "requires PulseAudio daemon"]
        fn can_connect_pulseaudio_context() {
            let mut mainloop = Mainloop::new().expect("should create mainloop");
            let mut context = Context::new(&mainloop, "playa-test").expect("should create context");
            context
                .connect(None, pulse::context::FlagSet::NOFLAGS, None)
                .expect("should start connection");

            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(5);
            loop {
                if start.elapsed() > timeout {
                    panic!("timed out waiting for PulseAudio context");
                }
                iterate_or_fail(&mut mainloop).expect("mainloop iterate failed");
                match context.get_state() {
                    pulse::context::State::Ready => break,
                    pulse::context::State::Failed | pulse::context::State::Terminated => {
                        panic!("PulseAudio context failed");
                    }
                    _ => {}
                }
            }

            context.disconnect();
        }

        #[test]
        #[ignore = "requires PulseAudio daemon"]
        fn can_create_event_stream() {
            let mut mainloop = Mainloop::new().expect("should create mainloop");
            let mut context = Context::new(&mainloop, "playa-test").expect("should create context");
            context
                .connect(None, pulse::context::FlagSet::NOFLAGS, None)
                .expect("should start connection");

            // Wait for context ready.
            loop {
                iterate_or_fail(&mut mainloop).expect("mainloop iterate failed");
                match context.get_state() {
                    pulse::context::State::Ready => break,
                    pulse::context::State::Failed | pulse::context::State::Terminated => {
                        panic!("PulseAudio context failed");
                    }
                    _ => {}
                }
            }

            // Create stream with media.role=event.
            let spec = Spec {
                format: Format::F32le,
                channels: 1,
                rate: 44100,
            };
            let mut proplist = Proplist::new().expect("should create proplist");
            proplist
                .set_str(pulse::proplist::properties::MEDIA_ROLE, "event")
                .expect("should set media.role");

            let mut stream =
                Stream::new_with_proplist(&mut context, "test-sfx", &spec, None, &mut proplist)
                    .expect("should create stream");

            stream
                .connect_playback(None, None, pulse::stream::FlagSet::NOFLAGS, None, None)
                .expect("should connect for playback");

            // Wait for stream ready.
            loop {
                iterate_or_fail(&mut mainloop).expect("mainloop iterate failed");
                match stream.get_state() {
                    pulse::stream::State::Ready => break,
                    pulse::stream::State::Failed | pulse::stream::State::Terminated => {
                        panic!("PulseAudio stream failed");
                    }
                    _ => {}
                }
            }

            stream.disconnect().ok();
            context.disconnect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sfx_playback_error_display() {
        // Verify error types implement Display correctly.
        let err = SfxPlaybackError::Decode(rodio::decoder::DecoderError::UnrecognizedFormat);
        let msg = err.to_string();
        assert!(msg.contains("decode"), "error message: {msg}");
    }
}
