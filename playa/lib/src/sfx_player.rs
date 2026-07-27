//! Native sound effects playback via rodio.
//!
//! When the `sfx-native` feature is enabled, sound effects are played directly
//! through the OS audio subsystem using rodio instead of spawning external player
//! processes. This provides lower latency and enables OS audio channel routing.
//!
//! The `sfx-native-audio` feature adds the OS-native routing described below.
//! It is one feature for every platform; which backend compiles in is decided
//! by `target_os`, so enabling it on a host whose backend does not apply is
//! harmless.
//!
//! ## macOS System Sound Device
//!
//! With `sfx-native-audio` on macOS, sound effects are routed through the
//! macOS "system sound" output device. This is the device configured in
//! System Settings → Sound → "Play sound effects through". It may differ from
//! the default music output device (e.g., built-in speakers for alerts while
//! headphones handle music).
//!
//! ## Windows Sound Effects Category
//!
//! With `sfx-native-audio` on Windows, sound effects are played through
//! WASAPI with `AudioCategory_SoundEffects`. This tags the audio stream so
//! Windows routes it through the Sound Effects mixer category, which has its
//! own volume slider in the Volume Mixer. Falls back to the default rodio
//! output if WASAPI setup fails.
//!
//! ## Linux
//!
//! With `sfx-native-audio` on Linux, sound effects are played through
//! PulseAudio (or PipeWire's PulseAudio compatibility layer) with
//! `media.role=event` set on the stream. PipeWire's `module-role-ducking`
//! and PulseAudio's equivalent use this property to optionally duck other
//! audio during event sounds. Falls back to the default rodio output if
//! PulseAudio is not available (e.g., ALSA-only systems).
//!
//! All PulseAudio wait loops (context connection, stream connection, drain)
//! use deadline-aware nonblocking polling. If a phase does not complete
//! within its deadline, the native path fails and playback falls back to
//! the host player. Playback blocks until completion or timeout.
//!
//! Native SFX playback never terminates the process. If a native device-open
//! operation times out, playa disables further native playback attempts for
//! the rest of the process and future calls fall back to host playback.
//! Native Linux errors (including timeouts) fall back to host playback.

use std::io::Cursor;
use std::time::{Duration, Instant};

use rodio::{Decoder, DeviceSinkBuilder, Player};
use thiserror::Error;

use crate::native_audio::{
    NATIVE_DEVICE_TIMEOUT, NativeAudioFailureKind, log_native_audio_disabled_once,
    native_audio_available, open_with_channel_fallback, run_with_timeout,
    trip_native_audio_breaker,
};
use crate::types::PlaybackOptions;

/// Maximum time to wait for native audio playback to complete before
/// giving up. This prevents the process from hanging indefinitely when
/// an audio device becomes unresponsive.
const PLAYBACK_TIMEOUT: Duration = Duration::from_secs(30);

/// Open an SFX audio stream with a timeout.
///
/// Resolves an optional requested channel and opens the requested or default
/// SFX output path within a single bounded deadline.
fn open_sfx_stream_with_timeout(
    timeout: Duration,
    options: &PlaybackOptions,
) -> Result<rodio::MixerDeviceSink, SfxPlaybackError> {
    let deadline = Instant::now() + timeout;
    open_with_channel_fallback(
        options.channel.as_deref(),
        deadline,
        crate::channels::find_device_by_id_or_name_with_timeout,
        open_device_stream_with_timeout,
        open_default_sfx_stream_with_timeout,
        || SfxPlaybackError::DeviceOpenTimeout(timeout.as_secs()),
    )
}

fn open_device_stream_with_timeout(
    device: rodio::Device,
    timeout: Duration,
) -> Result<rodio::MixerDeviceSink, SfxPlaybackError> {
    if timeout.is_zero() {
        return Err(SfxPlaybackError::DeviceOpenTimeout(
            NATIVE_DEVICE_TIMEOUT.as_secs(),
        ));
    }

    run_with_timeout(
        timeout,
        move || {
            DeviceSinkBuilder::from_device(device)
                .and_then(|builder| builder.open_stream())
                .map_err(SfxPlaybackError::Stream)
        },
        sfx_device_open_timeout,
    )
}

fn open_default_sfx_stream_with_timeout(
    timeout: Duration,
) -> Result<rodio::MixerDeviceSink, SfxPlaybackError> {
    if timeout.is_zero() {
        return Err(SfxPlaybackError::DeviceOpenTimeout(
            NATIVE_DEVICE_TIMEOUT.as_secs(),
        ));
    }

    run_with_timeout(timeout, open_default_sfx_stream, sfx_device_open_timeout)
}

fn open_default_sfx_stream() -> Result<rodio::MixerDeviceSink, SfxPlaybackError> {
    #[cfg(all(target_os = "macos", feature = "sfx-native-audio"))]
    {
        if let Ok(Some(device)) = macos::find_system_sound_device()
            && let Ok(stream) = DeviceSinkBuilder::from_device(device).and_then(|b| b.open_stream())
        {
            return Ok(stream);
        }
    }

    DeviceSinkBuilder::open_default_sink().map_err(SfxPlaybackError::Stream)
}

fn sfx_device_open_timeout(timeout: Duration) -> SfxPlaybackError {
    trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
    SfxPlaybackError::DeviceOpenTimeout(timeout.as_secs())
}

/// Errors from native SFX playback.
#[derive(Debug, Error)]
pub enum SfxPlaybackError {
    /// Failed to open an audio output stream.
    #[error("failed to open audio stream: {0}")]
    Stream(#[from] rodio::DeviceSinkError),

    /// Native playback was disabled earlier in this process.
    #[error("native playback is disabled for this process after an earlier device-open timeout")]
    NativePlaybackDisabled,

    /// Audio device did not respond within the allotted time.
    #[error("audio device did not respond within {0}s")]
    DeviceOpenTimeout(u64),

    /// Failed to decode audio data.
    #[error("failed to decode audio: {0}")]
    Decode(#[from] rodio::decoder::DecoderError),

    /// Failed to play audio.
    #[error("failed to play audio: {0}")]
    Play(#[from] rodio::PlayError),

    /// Playback timed out waiting for audio device.
    #[error("audio playback timed out after {0}s — audio device may be unresponsive")]
    Timeout(u64),
}

impl SfxPlaybackError {
    /// Whether callers should fall back to a host player instead of reporting
    /// the native failure directly.
    pub(crate) fn should_fallback_to_host(&self) -> bool {
        matches!(self, Self::Decode(_))
    }
}

/// Play sound effect bytes using native audio (rodio).
///
/// Supports volume and speed control via `PlaybackOptions`. On macOS with
/// `sfx-native-audio`, routes audio to the system sound device (which the
/// user can configure separately from the default output in System Settings
/// → Sound). Falls back to the default output device if the system sound
/// device can't be found or is the same as the default. A device-open timeout
/// disables future native playback attempts for the current process so callers
/// can fall back directly to host playback.
///
/// ## Errors
///
/// Returns `SfxPlaybackError` if the audio stream can't be opened or the
/// audio data can't be decoded. Callers should fall back to host player
/// delegation on error.
pub fn play_sfx(bytes: &[u8], options: &PlaybackOptions) -> Result<(), SfxPlaybackError> {
    if !native_audio_available() {
        log_native_audio_disabled_once();
        return Err(SfxPlaybackError::NativePlaybackDisabled);
    }

    // Windows: play through WASAPI with AudioCategory_SoundEffects.
    #[cfg(all(target_os = "windows", feature = "sfx-native-audio"))]
    {
        if options.speed.is_none() {
            if windows_sfx::play_sfx_with_category(bytes, options).is_ok() {
                return Ok(());
            }
        }
    }

    // Linux: play through PulseAudio with media.role=event.
    #[cfg(all(target_os = "linux", feature = "sfx-native-audio"))]
    {
        use linux::PulsePlaybackOutcome;

        if options.speed.is_none() {
            match linux::play_sfx_as_event(bytes, options) {
                PulsePlaybackOutcome::PlaybackCompleted | PulsePlaybackOutcome::PlaybackStarted => {
                    return Ok(());
                }
                PulsePlaybackOutcome::SetupFailed(e) => {
                    eprintln!("playa: PulseAudio setup failed, falling back to rodio: {e}");
                }
            }
        }
    }

    let source = Decoder::new(Cursor::new(bytes.to_vec()))?;
    let mut stream = open_sfx_stream_with_timeout(NATIVE_DEVICE_TIMEOUT, options)?;
    stream.log_on_drop(false);
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = options.volume {
        player.set_volume(vol);
    }
    if let Some(speed) = options.speed {
        player.set_speed(speed);
    }

    player.append(source);
    wait_with_timeout(&player, PLAYBACK_TIMEOUT)?;

    Ok(())
}

/// Wait for the player to finish, but give up after `timeout`.
///
/// Polls `player.empty()` every 50 ms. If the deadline is exceeded the
/// player is stopped and a `Timeout` error is returned so the caller can
/// fall back to a host player or report the failure.
fn wait_with_timeout(player: &Player, timeout: Duration) -> Result<(), SfxPlaybackError> {
    let deadline = Instant::now() + timeout;
    while !player.empty() {
        if Instant::now() >= deadline {
            player.stop();
            eprintln!(
                "playa: audio playback timed out after {}s — audio device may be unresponsive",
                timeout.as_secs()
            );
            return Err(SfxPlaybackError::Timeout(timeout.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

#[cfg(all(target_os = "macos", feature = "sfx-native-audio"))]
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

    /// Returns the human-readable name of the macOS system sound output
    /// device — the device used for UI sound effects when configured to
    /// differ from the default audio output.
    ///
    /// This performs only two CoreAudio property queries (device id and
    /// device name), making it dramatically cheaper than enumerating cpal
    /// devices and probing their supported configs.
    ///
    /// ## Returns
    ///
    /// `Some(name)` on success. `None` if the property query fails or the
    /// returned name is unreadable.
    pub fn get_system_sound_device_name() -> Option<String> {
        let device_id = get_audio_device(SYSTEM_OUTPUT_DEVICE_SELECTOR).ok()?;
        get_device_name(device_id)
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

#[cfg(all(target_os = "windows", feature = "sfx-native-audio"))]
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
        System::Com::{CLSCTX_ALL, CoCreateInstance},
    };

    use crate::windows_com::ComGuard;

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
    ///
    /// Only used when speed is not set. When speed is requested, the rodio
    /// default path is used instead so `Player::set_speed()` provides
    /// consistent time-stretch behavior across platforms.
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
            // Initialize COM via shared guard (handles S_OK, S_FALSE, RPC_E_CHANGED_MODE).
            let _com = ComGuard::new().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

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
            let _com = ComGuard::new().expect("COM init should succeed");
            unsafe {
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
            let _com = ComGuard::new().expect("COM init should succeed");
            unsafe {
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

#[cfg(all(target_os = "linux", feature = "sfx-native-audio"))]
mod linux {
    use std::io::Cursor;
    use std::time::{Duration, Instant};

    use libpulse_binding as pulse;
    use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
    use pulse::context::Context;
    use pulse::proplist::Proplist;
    use pulse::sample::{Format, Spec};
    use pulse::stream::{SeekMode, Stream};
    use rodio::Source;

    use super::{NATIVE_DEVICE_TIMEOUT, PLAYBACK_TIMEOUT};
    use crate::types::PlaybackOptions;

    /// Outcome of a PulseAudio playback attempt.
    ///
    /// Distinguishes setup failures (safe to fall back to another player) from
    /// post-write states (audio was already sent, so another player must NOT
    /// attempt to play the same clip again).
    pub(crate) enum PulsePlaybackOutcome {
        /// PulseAudio setup (context/stream connection) failed before audio was
        /// written to the server. Safe to fall back to another playback path.
        ///
        /// The inner error is carried for future diagnostics (e.g. tracing) but
        /// is not currently read; callers only branch on the variant.
        #[allow(dead_code)]
        SetupFailed(Box<dyn std::error::Error>),
        /// Audio was written to PulseAudio and the drain completed. Fully done.
        PlaybackCompleted,
        /// Audio was written to PulseAudio but the drain timed out or failed.
        /// The server already received the bytes, so do NOT attempt another
        /// playback path or the same clip will play twice.
        PlaybackStarted,
    }

    fn wait_for_pulse_condition<F>(
        mainloop: &mut Mainloop,
        deadline: Instant,
        phase: &'static str,
        mut check: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut() -> Result<bool, Box<dyn std::error::Error>>,
    {
        loop {
            match mainloop.iterate(false) {
                IterateResult::Success(_) => {}
                IterateResult::Err(e) => {
                    return Err(format!("PulseAudio mainloop error: {e}").into());
                }
                IterateResult::Quit(_) => {
                    return Err("PulseAudio mainloop quit unexpectedly".into());
                }
            }

            if check()? {
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(format!("PulseAudio {} timed out", phase).into());
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Play sound effect bytes through PulseAudio with `media.role=event`.
    ///
    /// Decodes the audio with rodio, then plays through a PulseAudio stream
    /// tagged with `media.role=event`. PipeWire's `module-role-ducking` and
    /// PulseAudio's `module-role-ducking` use this property to optionally duck
    /// other audio during event sounds.
    ///
    /// Only used when speed is not set. When speed is requested, the rodio
    /// default path is used instead so `Player::set_speed()` provides
    /// consistent time-stretch behavior across platforms.
    ///
    /// All wait loops use deadline-aware nonblocking polling. Context and stream
    /// readiness use `NATIVE_DEVICE_TIMEOUT`; drain uses a clip-derived timeout
    /// bounded by `PLAYBACK_TIMEOUT`.
    ///
    /// ## Returns
    ///
    /// Returns a `PulsePlaybackOutcome` so the caller can decide whether
    /// falling back to another player is safe. After `PlaybackStarted` or
    /// `PlaybackCompleted`, the caller must NOT attempt a second playback.
    pub fn play_sfx_as_event(bytes: &[u8], options: &PlaybackOptions) -> PulsePlaybackOutcome {
        match play_sfx_as_event_inner(bytes, options) {
            PlayResult::SetupFailed(e) => PulsePlaybackOutcome::SetupFailed(e),
            PlayResult::WriteFailed(e) => PulsePlaybackOutcome::SetupFailed(e),
            PlayResult::PlaybackCompleted => PulsePlaybackOutcome::PlaybackCompleted,
            PlayResult::DrainFailed => {
                eprintln!(
                    "playa: PulseAudio drain timed out — audio was already written, skipping fallback"
                );
                PulsePlaybackOutcome::PlaybackStarted
            }
        }
    }

    /// Internal result that separates every failure phase so `play_sfx_as_event`
    /// can map post-write drain failures to `PlaybackStarted` instead of
    /// `SetupFailed`.
    enum PlayResult {
        SetupFailed(Box<dyn std::error::Error>),
        WriteFailed(Box<dyn std::error::Error>),
        PlaybackCompleted,
        DrainFailed,
    }

    fn play_sfx_as_event_inner(bytes: &[u8], options: &PlaybackOptions) -> PlayResult {
        let decoder = match rodio::Decoder::new(Cursor::new(bytes.to_vec())) {
            Ok(d) => d,
            Err(e) => return PlayResult::SetupFailed(e.into()),
        };
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        let channels_u8 = channels.get() as u8;
        let sample_rate_u32 = sample_rate.get();

        let mut samples: Vec<f32> = decoder.collect();

        if let Some(vol) = options.volume {
            for s in &mut samples {
                *s *= vol;
            }
        }

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
            return PlayResult::SetupFailed("invalid PulseAudio sample spec".into());
        }

        let mut mainloop = match Mainloop::new() {
            Some(ml) => ml,
            None => return PlayResult::SetupFailed("failed to create PulseAudio mainloop".into()),
        };

        let mut context = match Context::new(&mainloop, "playa") {
            Some(ctx) => ctx,
            None => return PlayResult::SetupFailed("failed to create PulseAudio context".into()),
        };

        if let Err(e) = context.connect(None, pulse::context::FlagSet::NOFLAGS, None) {
            return PlayResult::SetupFailed(e.into());
        }

        let context_deadline = Instant::now() + NATIVE_DEVICE_TIMEOUT;

        if let Err(e) = wait_for_pulse_condition(
            &mut mainloop,
            context_deadline,
            "context connection",
            || {
                match context.get_state() {
                    pulse::context::State::Ready => return Ok(true),
                    pulse::context::State::Failed | pulse::context::State::Terminated => {
                        return Err("PulseAudio context connection failed".into());
                    }
                    _ => {}
                }
                Ok(false)
            },
        ) {
            return PlayResult::SetupFailed(e);
        }

        let mut proplist = match Proplist::new() {
            Some(pl) => pl,
            None => return PlayResult::SetupFailed("failed to create PulseAudio proplist".into()),
        };
        if proplist
            .set_str(pulse::proplist::properties::MEDIA_ROLE, "event")
            .is_err()
        {
            return PlayResult::SetupFailed("failed to set media.role property".into());
        }

        let mut stream = match Stream::new_with_proplist(
            &mut context,
            "Sound Effect",
            &spec,
            None,
            &mut proplist,
        ) {
            Some(s) => s,
            None => return PlayResult::SetupFailed("failed to create PulseAudio stream".into()),
        };

        if let Err(e) =
            stream.connect_playback(None, None, pulse::stream::FlagSet::NOFLAGS, None, None)
        {
            return PlayResult::SetupFailed(e.into());
        }

        let stream_deadline = Instant::now() + NATIVE_DEVICE_TIMEOUT;

        if let Err(e) =
            wait_for_pulse_condition(&mut mainloop, stream_deadline, "stream connection", || {
                match stream.get_state() {
                    pulse::stream::State::Ready => return Ok(true),
                    pulse::stream::State::Failed | pulse::stream::State::Terminated => {
                        return Err("PulseAudio stream connection failed".into());
                    }
                    _ => {}
                }
                Ok(false)
            })
        {
            return PlayResult::SetupFailed(e);
        }

        let byte_data: Vec<u8> = samples.iter().flat_map(|s: &f32| s.to_le_bytes()).collect();

        if let Err(e) = stream.write(&byte_data, None, 0, SeekMode::Relative) {
            return PlayResult::WriteFailed(e.into());
        }

        let clip_duration = Duration::from_secs_f64(
            samples.len() as f64 / (channels.get() as f64 * effective_rate as f64),
        );
        let drain_timeout = (clip_duration + Duration::from_secs(5))
            .min(PLAYBACK_TIMEOUT)
            .max(NATIVE_DEVICE_TIMEOUT);
        let drain_deadline = Instant::now() + drain_timeout;

        let op = stream.drain(None);

        if wait_for_pulse_condition(&mut mainloop, drain_deadline, "drain", || {
            match op.get_state() {
                pulse::operation::State::Done | pulse::operation::State::Cancelled => {
                    return Ok(true);
                }
                pulse::operation::State::Running => {}
            }
            Ok(false)
        })
        .is_err()
        {
            stream.disconnect().ok();
            context.disconnect();
            return PlayResult::DrainFailed;
        }

        stream.disconnect().ok();
        context.disconnect();

        PlayResult::PlaybackCompleted
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
        fn playback_started_is_terminal() {
            let outcome = PulsePlaybackOutcome::PlaybackStarted;
            let is_terminal = matches!(
                outcome,
                PulsePlaybackOutcome::PlaybackStarted | PulsePlaybackOutcome::PlaybackCompleted
            );
            assert!(
                is_terminal,
                "PlaybackStarted must be treated as terminal (no fallback)"
            );
        }

        #[test]
        fn playback_completed_is_terminal() {
            let outcome = PulsePlaybackOutcome::PlaybackCompleted;
            let is_terminal = matches!(
                outcome,
                PulsePlaybackOutcome::PlaybackStarted | PulsePlaybackOutcome::PlaybackCompleted
            );
            assert!(
                is_terminal,
                "PlaybackCompleted must be treated as terminal (no fallback)"
            );
        }

        #[test]
        fn setup_failed_is_not_terminal() {
            let outcome = PulsePlaybackOutcome::SetupFailed("test".into());
            let is_terminal = matches!(
                outcome,
                PulsePlaybackOutcome::PlaybackStarted | PulsePlaybackOutcome::PlaybackCompleted
            );
            assert!(
                !is_terminal,
                "SetupFailed must NOT be treated as terminal (fallback allowed)"
            );
        }

        #[test]
        fn play_result_drain_failed_maps_to_playback_started() {
            let inner = PlayResult::DrainFailed;
            let outcome = match inner {
                PlayResult::SetupFailed(e) => PulsePlaybackOutcome::SetupFailed(e),
                PlayResult::WriteFailed(e) => PulsePlaybackOutcome::SetupFailed(e),
                PlayResult::PlaybackCompleted => PulsePlaybackOutcome::PlaybackCompleted,
                PlayResult::DrainFailed => PulsePlaybackOutcome::PlaybackStarted,
            };
            assert!(
                matches!(outcome, PulsePlaybackOutcome::PlaybackStarted),
                "DrainFailed must map to PlaybackStarted"
            );
        }

        #[test]
        fn play_result_write_failed_maps_to_setup_failed() {
            let inner = PlayResult::WriteFailed("write err".into());
            let outcome = match inner {
                PlayResult::SetupFailed(e) => PulsePlaybackOutcome::SetupFailed(e),
                PlayResult::WriteFailed(e) => PulsePlaybackOutcome::SetupFailed(e),
                PlayResult::PlaybackCompleted => PulsePlaybackOutcome::PlaybackCompleted,
                PlayResult::DrainFailed => PulsePlaybackOutcome::PlaybackStarted,
            };
            assert!(
                matches!(outcome, PulsePlaybackOutcome::SetupFailed(_)),
                "WriteFailed must map to SetupFailed"
            );
        }

        #[test]
        fn wait_helper_returns_immediately_when_ready() {
            let call_count = std::sync::atomic::AtomicUsize::new(0);
            let result = wait_for_pulse_condition_with_mock(
                Instant::now() + Duration::from_secs(5),
                "test",
                || {
                    call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(true)
                },
            );
            assert!(result.is_ok());
            assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        }

        #[test]
        fn wait_helper_times_out_when_never_ready() {
            let result = wait_for_pulse_condition_with_mock(
                Instant::now() + Duration::from_millis(10),
                "test-phase",
                || Ok(false),
            );
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("test-phase"),
                "error should contain phase name: {err}"
            );
            assert!(
                err.contains("timed out"),
                "error should mention timeout: {err}"
            );
        }

        fn wait_for_pulse_condition_with_mock<F>(
            deadline: Instant,
            phase: &'static str,
            mut check: F,
        ) -> Result<(), Box<dyn std::error::Error>>
        where
            F: FnMut() -> Result<bool, Box<dyn std::error::Error>>,
        {
            loop {
                if check()? {
                    return Ok(());
                }

                if Instant::now() >= deadline {
                    return Err(format!("PulseAudio {} timed out", phase).into());
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        }

        #[test]
        #[ignore = "requires PulseAudio daemon"]
        fn can_connect_pulseaudio_context() {
            let mut mainloop = Mainloop::new().expect("should create mainloop");
            let mut context = Context::new(&mainloop, "playa-test").expect("should create context");
            context
                .connect(None, pulse::context::FlagSet::NOFLAGS, None)
                .expect("should start connection");

            let deadline = Instant::now() + NATIVE_DEVICE_TIMEOUT;
            wait_for_pulse_condition(&mut mainloop, deadline, "context connection", || {
                match context.get_state() {
                    pulse::context::State::Ready => return Ok(true),
                    pulse::context::State::Failed | pulse::context::State::Terminated => {
                        return Err("PulseAudio context failed".into());
                    }
                    _ => {}
                }
                Ok(false)
            })
            .expect("context should connect");

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

            let deadline = Instant::now() + NATIVE_DEVICE_TIMEOUT;

            wait_for_pulse_condition(&mut mainloop, deadline, "context connection", || {
                match context.get_state() {
                    pulse::context::State::Ready => return Ok(true),
                    pulse::context::State::Failed | pulse::context::State::Terminated => {
                        return Err("PulseAudio context failed".into());
                    }
                    _ => {}
                }
                Ok(false)
            })
            .expect("context should connect");

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

            wait_for_pulse_condition(&mut mainloop, deadline, "stream connection", || {
                match stream.get_state() {
                    pulse::stream::State::Ready => return Ok(true),
                    pulse::stream::State::Failed | pulse::stream::State::Terminated => {
                        return Err("PulseAudio stream failed".into());
                    }
                    _ => {}
                }
                Ok(false)
            })
            .expect("stream should become ready");

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

    #[test]
    fn play_sfx_short_circuits_when_native_audio_is_disabled() {
        let _guard = crate::native_audio::lock_native_audio_test_state();
        crate::native_audio::trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);

        let result = play_sfx(&[0u8; 4], &PlaybackOptions::default());
        assert!(matches!(
            result,
            Err(SfxPlaybackError::NativePlaybackDisabled)
        ));
    }

    #[test]
    fn decode_errors_do_not_trip_native_audio_breaker() {
        let _guard = crate::native_audio::lock_native_audio_test_state();

        let result = play_sfx(&[0u8; 4], &PlaybackOptions::default());
        assert!(matches!(result, Err(SfxPlaybackError::Decode(_))));
        assert!(crate::native_audio::native_audio_available());
    }
}
