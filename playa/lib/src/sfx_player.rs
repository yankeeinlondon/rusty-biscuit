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

use std::io::Cursor;

use rodio::{Decoder, DeviceSinkBuilder, Player};
use thiserror::Error;

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
/// On macOS with `sfx-native-macos`, routes audio to the system sound device
/// (which the user can configure separately from the default output in System
/// Settings → Sound). Falls back to the default output device if the system
/// sound device can't be found or is the same as the default.
///
/// ## Errors
///
/// Returns `SfxPlaybackError` if the audio stream can't be opened or the
/// audio data can't be decoded. Callers should fall back to host player
/// delegation on error.
pub fn play_sfx(bytes: &[u8], volume: Option<f32>) -> Result<(), SfxPlaybackError> {
    let stream = open_sfx_stream()?;
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = volume {
        player.set_volume(vol);
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
fn open_sfx_stream() -> Result<rodio::MixerDeviceSink, rodio::DeviceSinkError> {
    #[cfg(all(target_os = "macos", feature = "sfx-native-macos"))]
    {
        if let Ok(Some(device)) = macos::find_system_sound_device()
            && let Ok(stream) =
                DeviceSinkBuilder::from_device(device).and_then(|b| b.open_stream())
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
mod macos {
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
    pub fn find_system_sound_device() -> Result<Option<rodio::Device>, Box<dyn std::error::Error>> {
        let system_device_id = get_audio_device(SYSTEM_OUTPUT_DEVICE_SELECTOR)?;
        let default_device_id = get_audio_device(kAudioHardwarePropertyDefaultOutputDevice)?;

        // Common case: same device, no special routing needed.
        if system_device_id == default_device_id {
            return Ok(None);
        }

        // Devices differ - find the system sound device by name.
        let system_name =
            get_device_name(system_device_id).ok_or("failed to get system sound device name")?;

        let host = rodio::cpal::default_host();
        let devices = rodio::cpal::traits::HostTrait::output_devices(&host)?;

        for device in devices {
            if let Ok(desc) = device.description()
                && desc.name() == system_name
            {
                return Ok(Some(device));
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

    /// Get the human-readable name of a CoreAudio device.
    fn get_device_name(device_id: AudioObjectID) -> Option<String> {
        unsafe {
            let mut name: CFStringRef = std::ptr::null();
            let mut data_size = mem::size_of::<CFStringRef>() as u32;

            let address = AudioObjectPropertyAddress {
                mSelector: DEVICE_NAME_SELECTOR,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectGetPropertyData(
                device_id,
                &address,
                0,
                std::ptr::null(),
                &mut data_size,
                &mut name as *mut _ as *mut _,
            );

            if status != 0 || name.is_null() {
                return None;
            }

            let result = cfstring_to_string(name);
            CFRelease(name);
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
        fn find_system_sound_device_returns_result() {
            // Should succeed regardless of whether devices differ.
            let result = find_system_sound_device();
            assert!(result.is_ok());
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
