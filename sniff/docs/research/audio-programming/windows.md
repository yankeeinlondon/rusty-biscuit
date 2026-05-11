---
prompt: |-
    Your task is to research and document Windows capabilities and features for programmatically working with audio on Windows.

    Through this investigation your assumption will be that you are writing an audio program in Rust. This means all code examples in your research should be Rust based.

    In your research you should try to address the following questions:

    1. What are the common features which application developers look for on the Windows platform to achieve their audio goals? What API's, CLI's or other means of access do developers have available to them?
    2. Are "Sound Effects" treated differently than normal audio on Windows? Can you play OS sound effects only or can you provide your own? What format's can these sound effects be? Explain how to work with sound effects and give an example of how you could play a sound effect on Windows.
    3. How can you determine the following things on Windows:
          - is audio currently playing? what program is playing the audio? 
            - if the audio output does not provide software based volume control, are there any ways to influence the volume?
          - what is the current audio volume? how can I mute/unmute?
          - what are the various audio sources for input? What metadata can you get on each of these inputs?
          - what are the various audio outputs? What metadata can you get on each of these outputs?
          - how can I direct audio to a particular audio output? what common gotchas do people hit when trying to do this and how can these obstacles be worked around?
          - how can I perform "audio ducking" between audio streams?
          - what audio codec's are natively supported on Windows?
    4. The latest version of Windows is Windows 11 and all of the content outside of this section should try to focus exclusively on Windows 11, however, it's very useful to understand some important differences between Windows 10 and Windows 11


    Your output should be in well-formed idiomatic Markdown.  Use of Mermaid code blocks is welcome if that is helpful in illustrating an idea.

last_updated: 2026-02-27
update_policy:
    - Duration(6mo)
model: Gemini 3 Pro
source: rusty-biscuit/sniff
---

# Windows Audio Programming: A Comprehensive Guide for Rust Developers


## Introduction

Windows provides a rich set of audio APIs that enable developers to build sophisticated audio applications. This guide focuses on programmatic audio manipulation on Windows 11 from a Rust developer's perspective, covering everything from basic playback to advanced audio session management. The Windows audio stack has evolved significantly over the years, and understanding its architecture is crucial for building robust audio applications that work seamlessly across different Windows versions and hardware configurations.

For Rust developers, the Windows audio ecosystem offers several abstraction layers. At the lowest level, you can directly interface with Windows COM APIs using the `windows` crate. At a higher level, cross-platform libraries like `cpal` and `rodio` provide easier access while maintaining flexibility. Understanding both approaches allows you to choose the right tool for your specific use case, whether you need simple audio playback or complex real-time audio processing with low latency requirements.

---

## Windows Audio Architecture Overview

Windows audio architecture has evolved through several major iterations, each building upon its predecessors while introducing new capabilities. Understanding this evolution helps developers make informed decisions about which APIs to use and how to handle compatibility across different Windows versions. The current architecture, known as Core Audio, was introduced with Windows Vista and represents a complete redesign of the Windows audio subsystem, offering significantly improved latency, reliability, and flexibility compared to earlier approaches.

### Historical Context: Audio API Evolution

The Windows audio stack has progressed through several distinct generations, each addressing the limitations of its predecessor while introducing new capabilities that reflect evolving audio requirements and hardware capabilities:

**Legacy Audio APIs (Pre-Vista):**

- **WaveOut/WaveIn API**: The original Windows multimedia audio API, introduced in Windows 3.1, providing basic PCM audio playback and recording capabilities. While still functional, this API has significant limitations including high latency, lack of per-application volume control, and no support for advanced audio formats. Applications using this API cannot take advantage of modern Windows audio features like spatial sound or audio ducking.

- **DirectSound**: Introduced as part of DirectX, DirectSound provided lower latency audio playback with hardware acceleration support. It was popular for game development but has been deprecated in favor of WASAPI. DirectSound offered features like 3D audio positioning and effects processing, but its reliance on hardware abstraction layers made it inconsistent across different audio hardware. Modern applications should avoid DirectSound for new development.

- **Windows Multimedia (winmm)**: A broader multimedia API that included the `PlaySound` function for simple audio playback. This API remains relevant for playing system sounds and simple audio clips, though it lacks the sophistication needed for professional audio applications. The winmm API is still maintained for backward compatibility and can be useful for simple notification sounds or basic audio feedback.

**Modern Audio Architecture (Vista and Later):**

- **Core Audio APIs**: Introduced in Windows Vista, Core Audio represents a complete redesign of the Windows audio engine. This architecture provides low-latency audio processing, per-application volume control, and improved reliability. Core Audio is actually composed of four distinct API sets that work together: MMDevice API for device enumeration, WASAPI for audio streaming, DeviceTopology API for hardware control, and EndpointVolume API for volume management. Each component addresses specific aspects of audio functionality, allowing developers to use only the features they need.

### Core Audio Components

The Core Audio architecture consists of several interrelated components that together provide comprehensive audio functionality:

**1. MMDevice API (Multimedia Device API)**

The MMDevice API serves as the foundation for device enumeration and management in the Core Audio architecture. It provides the `IMMDeviceEnumerator` interface for discovering audio endpoints and the `IMMDevice` interface representing individual audio devices. The MMDevice API abstracts the complexity of audio hardware, presenting a consistent interface regardless of whether the device is a built-in sound card, USB audio interface, Bluetooth headset, or virtual audio device. This API also provides access to device properties including friendly names, device state, and audio format capabilities.

**2. WASAPI (Windows Audio Session API)**

WASAPI is the primary interface for audio streaming in modern Windows applications. It offers two operational modes: shared mode and exclusive mode. In shared mode, applications share the audio endpoint with other applications through the Windows audio engine, which handles mixing, format conversion, and effects processing. Shared mode is suitable for most applications and provides good compatibility with different audio hardware. Exclusive mode bypasses the audio engine entirely, allowing direct communication with the audio hardware for the lowest possible latency. This mode is essential for professional audio applications like DAWs and real-time audio processing but requires careful management of audio buffer sizes and formats.

**3. DeviceTopology API**

The DeviceTopology API provides access to the internal topology of audio hardware, allowing applications to control hardware-level features like input multiplexers, volume controls, and mute switches. This API is particularly useful for applications that need fine-grained control over audio hardware, such as recording software that needs to select specific input sources or adjust preamp gain. Not all audio hardware exposes full topology information, so applications should gracefully handle cases where topology access is limited.

**4. EndpointVolume API**

The EndpointVolume API provides programmatic control over the volume level and mute state of audio endpoint devices. This API supports both software volume control (where the audio engine applies volume scaling) and hardware volume control (where supported by the audio hardware). Applications can query whether hardware volume control is available and respond appropriately, implementing software volume control as a fallback when necessary. The API also provides notification callbacks for volume changes, allowing applications to update their UI to reflect system volume changes.

---

## Common Audio APIs and Access Methods

### Primary APIs for Rust Developers

Windows offers multiple access methods for audio programming, ranging from high-level abstractions to low-level hardware control. Rust developers can access these APIs through several crates that provide varying levels of abstraction and control over the audio subsystem.

#### 1. WASAPI (Windows Audio Session API)

WASAPI is the native Windows audio API that provides the most direct access to the audio subsystem while maintaining good compatibility with modern Windows versions. It offers excellent control over audio streaming and is the foundation upon which higher-level libraries are built. WASAPI provides two primary modes of operation: shared mode for general-purpose audio applications and exclusive mode for low-latency professional audio work.

**Rust Access via `wasapi` crate:**

```rust
use wasapi::{DeviceCollection, DeviceState, Direction, StreamMode};

fn enumerate_audio_devices() -> Result<(), Box<dyn std::error::Error>> {
    // Get collection of active output devices
    let collection = DeviceCollection::new(Direction::Render)?;
    
    println!("=== Output Audio Devices ===");
    for device in collection.iter() {
        if device.get_state()? == DeviceState::Active {
            let friendly_name = device.get_friendly_name()?;
            let id = device.get_id()?;
            println!("Device: {}", friendly_name);
            println!("  ID: {}", id);
            
            // Get default audio format
            if let Ok(format) = device.get_default_format(Direction::Render) {
                println!("  Sample Rate: {} Hz", format.sample_rate);
                println!("  Channels: {}", format.channels);
                println!("  Bits per sample: {}", format.bits_per_sample);
            }
            println!();
        }
    }
    Ok(())
}
```

**Rust Access via `windows` crate (Low-level COM):**

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

fn initialize_audio_client() -> Result<()> {
    unsafe {
        // Initialize COM apartment
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        // Create device enumerator
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        // Get default audio endpoint
        let device = enumerator.GetDefaultAudioEndpoint(
            eRender, 
            eConsole
        )?;
        
        // Activate audio client
        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        
        // Get mix format (preferred format for shared mode)
        let format = audio_client.GetMixFormat()?;
        
        Ok(())
    }
}
```

#### 2. Cross-Platform Audio Library: `cpal`

The `cpal` crate provides a cross-platform abstraction for audio input and output, making it ideal for applications that need to work on Windows, macOS, and Linux. While it abstracts away platform-specific details, it still provides enough control for most audio applications and handles the complexity of WASAPI internally when running on Windows.

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn list_audio_devices() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    
    // List input devices
    println!("=== Input Devices ===");
    for device in host.input_devices()? {
        let name = device.name()?;
        let default_config = device.default_input_config()?;
        println!("Input: {}", name);
        println!("  Sample Rate: {:?}", default_config.sample_rate());
        println!("  Channels: {}", default_config.channels());
        println!();
    }
    
    // List output devices
    println!("=== Output Devices ===");
    for device in host.output_devices()? {
        let name = device.name()?;
        let default_config = device.default_output_config()?;
        println!("Output: {}", name);
        println!("  Sample Rate: {:?}", default_config.sample_rate());
        println!("  Channels: {}", default_config.channels());
        println!();
    }
    
    Ok(())
}

fn play_sine_wave() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config()?;
    
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    
    // Create output stream
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let value = (2.0 * std::f32::consts::PI * 440.0 * 0.0).sin() * 0.1;
                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;
    
    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    Ok(())
}
```

#### 3. High-Level Audio Playback: `rodio`

For applications that primarily need to play audio files without complex real-time processing, `rodio` provides a convenient high-level interface. Built on top of `cpal`, `rodio` handles audio file decoding, volume control, and playback management automatically.

```rust
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

fn play_audio_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get output stream and handle
    let (_stream, stream_handle) = OutputStream::try_default()?;
    
    // Create sink for playback control
    let sink = Sink::try_new(&stream_handle)?;
    
    // Load audio file
    let file = BufReader::new(File::open(path)?);
    let source = Decoder::new(file)?;
    
    // Add source to sink and play
    sink.append(source);
    sink.sleep_until_end();
    
    Ok(())
}
```

### Command-Line Tools for Audio Management

Windows provides several command-line interfaces that can be useful for scripting and automation of audio settings:

| Tool              | Description                                      | Example Usage                                  |
| ----------------- | ------------------------------------------------ | ---------------------------------------------- |
| `SoundVolumeView` | Third-party tool for comprehensive audio control | `SoundVolumeView.exe /SetVolume "Speakers" 50` |
| `PowerShell`      | Access audio settings via COM objects            | See volume control example below               |
| ` NirCmd`         | Multi-purpose command-line tool                  | `nircmd.exe setsysvolume 32768`                |

---

## Sound Effects on Windows

### Understanding Sound Effects vs. Normal Audio

Windows distinguishes between "system sounds" (also called sound effects or event sounds) and regular application audio. System sounds are predefined audio cues that Windows plays in response to specific events, such as device connections, notifications, errors, or user actions. These sounds are integrated with the Windows registry and can be customized by users through the Sound Control Panel. Unlike regular audio playback, system sounds are managed by the operating system and respect user preferences for notification sounds, including the ability to disable them entirely.

### System Sound Categories

Windows organizes system sounds into categories that correspond to different types of events and system actions:

**1. Windows System Events**

- Device Connect/Disconnect (`WindowsDefault.wav` variants)
- Windows Logon/Logoff sounds
- Windows Startup sound
- Critical Stop and Error sounds
- Default Beep

**2. Application Notifications**

- New Mail Notification
- Calendar Reminder
- Instant Message Notification

**3. User Interface Feedback**

- Menu Command sounds
- Open/Close Program sounds
- Minimize/Maximize sounds
- Notification sounds

### Playing System Sounds in Rust

The `PlaySound` function from `winmm.dll` provides the ability to play system sounds by their registry name. This function looks up the sound configuration in the registry and plays the appropriate audio file based on user preferences. Rust developers can access this functionality through the `windows` crate.

```rust
use windows::{
    core::PCWSTR,
    Win32::Media::Audio::PlaySoundW,
    Win32::Media::Audio::SND_ASYNC,
    Win32::Media::Audio::SND_ALIAS,
    Win32::Media::Audio::SND_NODEFAULT,
};

/// Play a Windows system sound by its alias name
/// Common aliases include:
/// - "SystemDefault" - Default beep
/// - "SystemAsterisk" - Asterisk (informational)
/// - "SystemExclamation" - Exclamation (warning)
/// - "SystemExit" - Windows exit sound
/// - "SystemHand" - Critical stop
/// - "SystemQuestion" - Question
/// - "SystemStart" - Windows startup
/// - "DeviceConnect" - Device connected
/// - "DeviceDisconnect" - Device disconnected
/// - "Notification.Default" - Default notification
fn play_system_sound(alias: &str) -> Result<(), Box<dyn std::error::Error>> {
    let wide_alias: Vec<u16> = alias.encode_utf16().chain(std::iter::once(0)).collect();
    
    unsafe {
        let result = PlaySoundW(
            PCWSTR(wide_alias.as_ptr()),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        );
        
        if !result.as_bool() {
            return Err("Failed to play system sound".into());
        }
    }
    
    Ok(())
}

// Example usage
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Play device connect sound
    play_system_sound("DeviceConnect")?;
    
    // Play notification sound
    play_system_sound("Notification.Default")?;
    
    // Wait for sound to play
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    Ok(())
}
```

### Playing Custom Sound Effects

For custom sound effects (WAV files), you can use `PlaySound` with the `SND_FILENAME` flag or use higher-level libraries like `rodio` for more format support:

```rust
use windows::{
    core::PCWSTR,
    Win32::Media::Audio::{PlaySoundW, SND_FILENAME, SND_ASYNC},
};

/// Play a WAV file as a sound effect
fn play_wav_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    
    unsafe {
        let result = PlaySoundW(
            PCWSTR(wide_path.as_ptr()),
            None,
            SND_FILENAME | SND_ASYNC,
        );
        
        if !result.as_bool() {
            return Err("Failed to play WAV file".into());
        }
    }
    
    Ok(())
}
```

### Supported Sound Effect Formats

**System Sounds (via PlaySound):**

- WAV (.wav) - Primary format for system sounds
- Format: PCM, typically 16-bit, 44.1kHz or 48kHz

**Custom Sound Effects (via higher-level APIs):**

- WAV (.wav) - Uncompressed PCM audio
- MP3 (.mp3) - Compressed audio (via Media Foundation)
- AAC/M4A (.m4a, .aac) - Advanced Audio Coding
- FLAC (.flac) - Free Lossless Audio Codec
- OGG Vorbis (.ogg) - Open source compressed format

---

## Audio Detection and Monitoring

### Determining if Audio is Currently Playing

Windows provides the Audio Session API to detect active audio streams and identify which applications are producing audio. This functionality is essential for building audio mixers, monitoring tools, and applications that need to coordinate with other audio sources.

#### Using IAudioSessionEnumerator

The `IAudioSessionEnumerator` interface allows you to enumerate all active audio sessions and retrieve information about each session, including the process ID of the application that owns the session.

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

struct AudioSessionInfo {
    process_id: u32,
    session_id: String,
    is_system_sound: bool,
    state: AudioSessionState,
}

#[derive(Debug, Clone, Copy)]
enum AudioSessionState {
    Inactive,
    Active,
    Expired,
}

fn get_active_audio_sessions() -> Result<Vec<AudioSessionInfo>, Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        // Get audio session manager
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        
        // Get session enumerator
        let session_enum = manager.GetSessionEnumerator()?;
        
        let session_count = session_enum.GetCount()?;
        let mut sessions = Vec::new();
        
        for i in 0..session_count {
            if let Ok(control) = session_enum.GetSession(i) {
                // Get IAudioSessionControl2 for extended info
                let control2: IAudioSessionControl2 = control.cast()?;
                
                let process_id = control2.GetProcessId()?;
                let is_system = control2.IsSystemSoundsSession()?.as_bool();
                
                // Get session state
                let state = match control.GetState()? {
                    AudioSessionStateActive => AudioSessionState::Active,
                    AudioSessionStateInactive => AudioSessionState::Inactive,
                    AudioSessionStateExpired => AudioSessionState::Expired,
                    _ => AudioSessionState::Inactive,
                };
                
                // Get session identifier
                let session_id = control2.GetSessionIdentifier()?.to_string();
                
                sessions.push(AudioSessionInfo {
                    process_id,
                    session_id,
                    is_system_sound: is_system,
                    state,
                });
            }
        }
        
        Ok(sessions)
    }
}

fn print_playing_processes() -> Result<(), Box<dyn std::error::Error>> {
    let sessions = get_active_audio_sessions()?;
    
    println!("=== Active Audio Sessions ===");
    for session in sessions {
        if session.state == AudioSessionState::Active {
            println!("Process ID: {}", session.process_id);
            println!("  Session ID: {}", session.session_id);
            println!("  Is System Sound: {}", session.is_system_sound);
            
            // Optionally get process name from PID
            if let Ok(process) = get_process_name(session.process_id) {
                println!("  Process Name: {}", process);
            }
            println!();
        }
    }
    
    Ok(())
}

fn get_process_name(pid: u32) -> Result<String, Box<dyn std::error::Error>> {
    use windows::Win32::System::Diagnostics::ToolHelp::*;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Foundation::HANDLE;
    
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        
        if Process32FirstW(snapshot, &mut entry).as_bool() {
            loop {
                if entry.th32ProcessID == pid {
                    let name = String::from_utf16_lossy(
                        &entry.szExeFile[..entry.szExeFile.iter()
                            .position(|&c| c == 0)
                            .unwrap_or(entry.szExeFile.len())]
                    );
                    CloseHandle(snapshot)?;
                    return Ok(name);
                }
                if !Process32NextW(snapshot, &mut entry).as_bool() {
                    break;
                }
            }
        }
        
        CloseHandle(snapshot)?;
        Err("Process not found".into())
    }
}
```

### Listening for Audio Session Events

For real-time monitoring of audio activity, you can register for audio session notifications:

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

// Define callback interface for session events
#[implement(IAudioSessionEvents)]
struct SessionEventHandler;

impl IAudioSessionEvents_Impl for SessionEventHandler_Impl {
    fn OnDisplayNameChanged(&self, _new_display_name: &PCWSTR, _event_context: *const GUID) -> Result<()> {
        println!("Display name changed");
        Ok(())
    }
    
    fn OnIconPathChanged(&self, _new_icon_path: &PCWSTR, _event_context: *const GUID) -> Result<()> {
        println!("Icon path changed");
        Ok(())
    }
    
    fn OnSimpleVolumeChanged(&self, new_volume: f32, new_mute: BOOL, _event_context: *const GUID) -> Result<()> {
        println!("Volume: {:.2}, Muted: {}", new_volume, new_mute.as_bool());
        Ok(())
    }
    
    fn OnChannelVolumeChanged(&self, _channel_count: u32, _new_channel_volume: *const f32, _changed_channel: u32, _event_context: *const GUID) -> Result<()> {
        println!("Channel volume changed");
        Ok(())
    }
    
    fn OnGroupingParamChanged(&self, _new_grouping_param: *const GUID, _event_context: *const GUID) -> Result<()> {
        println!("Grouping parameter changed");
        Ok(())
    }
    
    fn OnStateChanged(&self, new_state: AudioSessionState) -> Result<()> {
        match new_state {
            AudioSessionStateActive => println!("Session became ACTIVE"),
            AudioSessionStateInactive => println!("Session became INACTIVE"),
            AudioSessionStateExpired => println!("Session EXPIRED"),
            _ => {}
        }
        Ok(())
    }
    
    fn OnDisconnected(&self, _disconnect_reason: AudioSessionDisconnectReason) -> Result<()> {
        println!("Session disconnected");
        Ok(())
    }
}
```

---

## Volume Control and Muting

### Understanding Volume Control on Windows

Windows implements a hierarchical volume control system with multiple levels: system volume, endpoint volume, and application (per-session) volume. Each level can be controlled independently, providing flexibility for both users and applications. Understanding this hierarchy is essential for implementing proper volume control in your applications.

### Hardware vs. Software Volume Control

Some audio devices provide hardware volume control, where volume changes are applied directly by the audio hardware rather than software. The `IAudioEndpointVolume` interface can detect whether hardware volume is available and use it when possible. When hardware volume control is unavailable or insufficient, Windows applies software volume scaling in the audio engine.

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

/// Get current master volume level (0.0 to 1.0)
fn get_master_volume() -> Result<f32, Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        
        let volume = endpoint_volume.GetMasterVolumeLevelScalar()?;
        
        Ok(volume)
    }
}

/// Set master volume level (0.0 to 1.0)
fn set_master_volume(level: f32) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        
        endpoint_volume.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
        
        Ok(())
    }
}

/// Get mute state
fn is_muted() -> Result<bool, Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        
        let muted = endpoint_volume.GetMute()?.as_bool();
        
        Ok(muted)
    }
}

/// Mute or unmute audio
fn set_mute(mute: bool) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        
        endpoint_volume.SetMute(mute, std::ptr::null())?;
        
        Ok(())
    }
}

/// Check if hardware volume control is available
fn has_hardware_volume_control() -> Result<bool, Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        
        // Query hardware support
        let hardware_support = endpoint_volume.QueryHardwareSupport()?;
        
        // Check for volume and mute support
        let has_volume = (hardware_support & ENDPOINT_HARDWARE_SUPPORT_VOLUME) != 0;
        let has_mute = (hardware_support & ENDPOINT_HARDWARE_SUPPORT_MUTE) != 0;
        
        Ok(has_volume && has_mute)
    }
}
```

### Per-Application Volume Control

Windows allows controlling the volume of individual applications through the Audio Session API:

```rust
/// Set volume for a specific application session
fn set_application_volume(session_id: &str, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        
        let session_enum = manager.GetSessionEnumerator()?;
        let count = session_enum.GetCount()?;
        
        for i in 0..count {
            if let Ok(control) = session_enum.GetSession(i) {
                let control2: IAudioSessionControl2 = control.cast()?;
                let current_id = control2.GetSessionIdentifier()?.to_string();
                
                if current_id.contains(session_id) {
                    let simple_volume: ISimpleAudioVolume = control.cast()?;
                    simple_volume.SetMasterVolume(volume, std::ptr::null())?;
                    return Ok(());
                }
            }
        }
        
        Err("Session not found".into())
    }
}
```

### Handling Devices Without Software Volume Control

When audio output devices don't provide software-based volume control (such as some professional audio interfaces or HDMI outputs), you have several options to influence the volume:

1. **Hardware Volume Control**: Check if the device supports hardware volume using `QueryHardwareSupport()` and use `SetMasterVolumeLevel()` instead of `SetMasterVolumeLevelScalar()` for dB-based control.

2. **Application-Level Volume Scaling**: Apply volume scaling to your audio data before sending it to the audio API:

```rust
fn apply_volume_to_buffer(buffer: &mut [f32], volume: f32) {
    for sample in buffer.iter_mut() {
        *sample *= volume;
    }
}
```

1. **Per-Session Volume**: Even if the endpoint doesn't support volume control, Windows always supports per-session (application) volume control.

---

## Audio Device Enumeration

### Enumerating Audio Input Devices

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

#[derive(Debug)]
struct AudioDeviceInfo {
    id: String,
    friendly_name: String,
    state: DeviceState,
    is_default: bool,
}

#[derive(Debug)]
enum DeviceState {
    Active,
    Disabled,
    NotPresent,
    Unplugged,
}

fn enumerate_input_devices() -> Result<Vec<AudioDeviceInfo>, Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        // Get default input device for comparison
        let default_device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole);
        let default_id = if let Ok(ref dev) = default_device {
            Some(dev.GetId()?.to_string())
        } else {
            None
        };
        
        // Enumerate all input devices
        let collection = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;
        
        let mut devices = Vec::new();
        
        for i in 0..count {
            let device = collection.Item(i)?;
            let id = device.GetId()?.to_string();
            
            // Get friendly name from property store
            let prop_store = device.OpenPropertyStore(STGM_READ)?;
            let friendly_name = prop_store.GetValue(&PKEY_Device_FriendlyName)?
                .to_string();
            
            let state = match device.GetState()? {
                DEVICE_STATE_ACTIVE => DeviceState::Active,
                DEVICE_STATE_DISABLED => DeviceState::Disabled,
                DEVICE_STATE_NOTPRESENT => DeviceState::NotPresent,
                DEVICE_STATE_UNPLUGGED => DeviceState::Unplugged,
                _ => DeviceState::Disabled,
            };
            
            let is_default = default_id.as_ref()
                .map(|d| d == &id)
                .unwrap_or(false);
            
            devices.push(AudioDeviceInfo {
                id,
                friendly_name,
                state,
                is_default,
            });
        }
        
        Ok(devices)
    }
}
```

### Enumerating Audio Output Devices

```rust
fn enumerate_output_devices() -> Result<Vec<AudioDeviceInfo>, Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        // Get default output device
        let default_device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole);
        let default_id = if let Ok(ref dev) = default_device {
            Some(dev.GetId()?.to_string())
        } else {
            None
        };
        
        // Enumerate all output devices
        let collection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;
        
        let mut devices = Vec::new();
        
        for i in 0..count {
            let device = collection.Item(i)?;
            let id = device.GetId()?.to_string();
            
            let prop_store = device.OpenPropertyStore(STGM_READ)?;
            let friendly_name = prop_store.GetValue(&PKEY_Device_FriendlyName)?
                .to_string();
            
            // Get additional metadata
            let device_interface = prop_store.GetValue(&PKEY_Device_InterfaceFriendlyName)?
                .to_string();
            let device_desc = prop_store.GetValue(&PKEY_Device_DeviceDesc)?
                .to_string();
            
            let state = match device.GetState()? {
                DEVICE_STATE_ACTIVE => DeviceState::Active,
                _ => DeviceState::Disabled,
            };
            
            let is_default = default_id.as_ref()
                .map(|d| d == &id)
                .unwrap_or(false);
            
            println!("Device: {}", friendly_name);
            println!("  Interface: {}", device_interface);
            println!("  Description: {}", device_desc);
            println!("  Default: {}", is_default);
            println!();
            
            devices.push(AudioDeviceInfo {
                id,
                friendly_name,
                state,
                is_default,
            });
        }
        
        Ok(devices)
    }
}
```

### Available Device Metadata Properties

Windows provides extensive metadata about audio devices through the property store. Here are the most commonly used properties:

| Property Key                          | Description                                       |
| ------------------------------------- | ------------------------------------------------- |
| `PKEY_Device_FriendlyName`            | User-friendly device name                         |
| `PKEY_Device_DeviceDesc`              | Device description                                |
| `PKEY_Device_InterfaceFriendlyName`   | Audio interface name                              |
| `PKEY_Device_Manufacturer`            | Device manufacturer                               |
| `PKEY_Device_DriverVersion`           | Driver version                                    |
| `PKEY_AudioEndpoint_FormFactor`       | Physical form factor (speakers, headphones, etc.) |
| `PKEY_AudioEndpoint_PhysicalSpeakers` | Physical speaker configuration                    |
| `PKEY_AudioEndpoint_GUID`             | Device GUID                                       |

---

## Audio Routing to Specific Outputs

### Directing Audio to a Particular Device

WASAPI allows you to direct audio to specific output devices by activating an audio client on the desired device rather than the default device. This approach provides precise control over audio routing but requires careful management of audio streams and device lifecycles.

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

/// Play audio on a specific output device
fn play_on_device(device_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
        
        let enumerator: IMMDeviceEnumerator = 
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        
        // Get the specific device by ID
        let device = enumerator.GetDevice(PCWSTR(
            device_id.encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
                .as_ptr()
        ))?;
        
        // Activate audio client on this specific device
        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        
        // Initialize and use the audio client
        // ... rest of audio initialization code
        
        Ok(())
    }
}

/// Create audio stream for specific device using cpal
fn create_stream_for_device(device_name: &str) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    
    // Find device by name
    let device = host.output_devices()?
        .find(|d| d.name().map(|n| n.contains(device_name)).unwrap_or(false))
        .ok_or("Device not found")?;
    
    let config = device.default_output_config()?;
    
    let stream = device.build_output_stream(
        &config.into(),
        |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // Fill buffer with audio data
            for sample in data.iter_mut() {
                *sample = 0.0; // Silence for example
            }
        },
        |err| eprintln!("Stream error: {}", err),
        None,
    )?;
    
    Ok(stream)
}
```

### Common Gotchas and Solutions

**1. Device Disconnection Handling**

Audio devices can be disconnected at any time (Bluetooth devices, USB devices). Always handle device disconnection gracefully:

```rust
// Register for device notification
use windows::Win32::Media::Audio::IMMNotificationClient;

#[implement(IMMNotificationClient)]
struct DeviceNotificationHandler;

impl IMMNotificationClient_Impl for DeviceNotificationHandler_Impl {
    fn OnDeviceStateChanged(&self, device_id: &PCWSTR, new_state: u32) -> Result<()> {
        let id = device_id.to_string();
        match new_state {
            DEVICE_STATE_ACTIVE => println!("Device {} connected", id),
            DEVICE_STATE_UNPLUGGED => println!("Device {} disconnected", id),
            DEVICE_STATE_DISABLED => println!("Device {} disabled", id),
            _ => {}
        }
        Ok(())
    }
    
    fn OnDefaultDeviceChanged(&self, role: EDataFlow, flow: ERole, device_id: &PCWSTR) -> Result<()> {
        println!("Default device changed: {}", device_id.to_string());
        Ok(())
    }
    
    fn OnDeviceAdded(&self, _pwstr_device_id: &PCWSTR) -> Result<()> {
        Ok(())
    }
    
    fn OnDeviceRemoved(&self, _pwstr_device_id: &PCWSTR) -> Result<()> {
        Ok(())
    }
    
    fn OnPropertyValueChanged(&self, _pwstr_device_id: &PCWSTR, _key: const PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
```

**2. Format Compatibility**

Different devices support different audio formats. Always query the device's supported formats or use the mix format:

```rust
fn get_device_format(device: &IMMDevice) -> Result<WaveFormatEx, Box<dyn std::error::Error>> {
    unsafe {
        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        
        // Get the mix format - this is guaranteed to work in shared mode
        let format_ptr = audio_client.GetMixFormat()?;
        let format = *format_ptr;
        
        Ok(format)
    }
}
```

**3. Exclusive Mode Conflicts**

Using exclusive mode prevents other applications from using the audio device. Always provide a fallback to shared mode:

```rust
fn initialize_audio_with_fallback(device: &IMMDevice) -> Result<IAudioClient, Box<dyn std::error::Error>> {
    unsafe {
        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        
        // Try exclusive mode first
        let format = audio_client.GetMixFormat()?;
        
        let exclusive_result = audio_client.Initialize(
            AUDCLNT_SHAREMODE_EXCLUSIVE,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            10000000, // 1 second buffer in 100ns units
            10000000,
            &*format,
            std::ptr::null(),
        );
        
        if exclusive_result.is_err() {
            println!("Exclusive mode failed, falling back to shared mode");
            audio_client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                0,
                0,
                &*format,
                std::ptr::null(),
            )?;
        }
        
        Ok(audio_client)
    }
}
```

**4. Stream Routing Issues**

When the default device changes, existing audio streams may not automatically redirect. Handle the `OnSessionDisconnected` event:

```rust
// In your IAudioSessionEvents implementation:
fn OnDisconnected(&self, disconnect_reason: AudioSessionDisconnectReason) -> Result<()> {
    match disconnect_reason {
        AudioSessionDisconnectReasonDeviceRemoval => {
            // Device was removed, need to reconnect to new device
            println!("Device removed, reconnecting...");
        }
        AudioSessionDisconnectReasonServerShutdown => {
            // Audio server shutdown
            println!("Audio server shutdown");
        }
        AudioSessionDisconnectReasonFormatChanged => {
            // Format changed, need to reinitialize
            println!("Format changed");
        }
        _ => {}
    }
    Ok(())
}
```

---

## Audio Ducking (Stream Attenuation)

### Understanding Audio Ducking on Windows

Audio ducking, also known as stream attenuation, is a Windows feature that automatically reduces the volume of non-communication audio streams when a communication stream (such as a VoIP call) becomes active. This behavior helps users hear important communications without manually adjusting volume levels for other applications.

### Ducking Behavior Categories

Windows defines three ducking options that users can configure through the Sound Control Panel (Communications tab):

1. **No ducking**: Communication sounds have no effect on other audio streams
2. **Reduce volume by 80%**: Non-communication audio is reduced to 20% of original volume
3. **Reduce volume by 50%**: Non-communication audio is reduced to 50% of original volume
4. **Mute all other sounds**: Non-communication audio is completely muted

### Implementing Audio Ducking in Your Application

You can opt your application into or out of the ducking behavior, and even implement custom ducking:

```rust
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
};

/// Set ducking behavior for an audio session
fn set_ducking_preference(
    audio_client: &IAudioClient, 
    opt_out: bool
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // Get audio session control
        let session_control: IAudioSessionControl2 = 
            audio_client.GetService()?.cast()?;
        
        if opt_out {
            // Opt out of ducking - your app won't be ducked
            // and won't cause ducking of other apps
            session_control.SetDuckingPreference(true)?;
        } else {
            // Default behavior - participate in ducking
            session_control.SetDuckingPreference(false)?;
        }
        
        Ok(())
    }
}
```

### Detecting Ducking Events

To respond to ducking events, implement the `IAudioSessionEvents` interface:

```rust
// Ducking is reflected in volume changes
impl IAudioSessionEvents_Impl for SessionEventHandler_Impl {
    fn OnSimpleVolumeChanged(
        &self, 
        new_volume: f32, 
        new_mute: BOOL, 
        _event_context: *const GUID
    ) -> Result<()> {
        // Volume changed - could be due to ducking
        if new_volume < 1.0 {
            println!("Volume reduced to {:.2}% (possibly ducked)", new_volume * 100.0);
        }
        Ok(())
    }
    
    // ... other trait methods
}
```

### Triggering Audio Ducking (Communication Apps)

If your application is a communication app (VoIP, video conferencing), you should classify your audio streams appropriately to trigger ducking:

```rust
/// Configure stream as a communication stream
fn configure_communication_stream(
    audio_client: &IAudioClient
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // Set the audio category to communications
        // This is done through stream properties
        use windows::Win32::Media::Audio::AUDCLNT_STREAMOPTIONS[];
        
        // In Windows 10+, you can set stream options
        let audio_client2: IAudioClient2 = audio_client.cast()?;
        
        audio_client2.SetClientProperties(&AUDIO_CLIENT_PROPERTIES {
            cbSize: std::mem::size_of::<AUDIO_CLIENT_PROPERTIES>() as u32,
            bIsOffload: false,
            eCategory: AudioCategory_Communications,
            Options: AUDCLNT_STREAMOPTIONS_NONE,
        })?;
        
        Ok(())
    }
}
```

---

## Native Audio Codec Support

### Built-in Codec Support in Windows 11

Windows 11 includes native support for a wide range of audio codecs through the Media Foundation framework. This means applications can play these formats without requiring additional codec packs or third-party software.

| Format               | Extension   | Codec Name                       | Notes                                                 |
| -------------------- | ----------- | -------------------------------- | ----------------------------------------------------- |
| **Uncompressed PCM** | .wav        | PCM                              | Standard WAV format, guaranteed support               |
| **MP3**              | .mp3        | MPEG-1 Audio Layer III           | Ubiquitous compressed format                          |
| **AAC**              | .m4a, .aac  | Advanced Audio Coding            | Default iTunes format, excellent quality/bitrate      |
| **FLAC**             | .flac       | Free Lossless Audio Codec        | Native support added in Windows 10                    |
| **ALAC**             | .m4a        | Apple Lossless Audio Codec       | Apple's lossless format                               |
| **WMA**              | .wma        | Windows Media Audio              | Microsoft's proprietary codec                         |
| **WMA Pro**          | .wma        | Windows Media Audio Professional | Higher quality WMA variant                            |
| **WMA Lossless**     | .wma        | Windows Media Audio Lossless     | Lossless WMA variant                                  |
| **OGG Vorbis**       | .ogg        | Vorbis                           | Open source codec (may require additional components) |
| **Opus**             | .opus, .ogg | Opus                             | Modern codec for voice and music                      |

### Windows 11 Exclusive: Bluetooth LE Audio and LC3

Windows 11 (version 22H2 and later) introduces support for **Bluetooth LE Audio** with the **LC3 codec**:

| Feature                 | Description                                                  |
| ----------------------- | ------------------------------------------------------------ |
| **LC3 Codec**           | Low Complexity Communication Codec - more efficient than SBC |
| **Lower Latency**       | Reduced audio delay compared to classic Bluetooth            |
| **Better Battery Life** | LE Audio is more power efficient                             |
| **Higher Quality**      | Better audio quality at lower bitrates                       |
| **Multi-stream**        | Support for multiple simultaneous audio streams              |
| **Broadcast Audio**     | Ability to broadcast to multiple receivers                   |

### Using Media Foundation for Codec Support

For format conversion and codec access, use Media Foundation:

```rust
use windows::{
    core::*,
    Win32::Media::MediaFoundation::*,
};

/// Initialize Media Foundation and list supported formats
fn initialize_media_foundation() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // Initialize Media Foundation
        MFStartup(MF_VERSION, 0)?;
        
        // Media Foundation is now ready for use
        // You can use SourceReader for decoding various formats
        
        Ok(())
    }
}

/// Decode audio file using Media Foundation
fn decode_audio_file(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    unsafe {
        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        
        // Create source reader
        let reader = MFCreateSourceReaderFromURL(PCWSTR(wide_path.as_ptr()), None)?;
        
        // Configure for PCM output
        let mut media_type = None;
        MFCreateMediaType(&mut media_type)?;
        let media_type = media_type.unwrap();
        
        media_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        media_type.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_Float)?;
        
        reader.SetCurrentMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM, None, &media_type)?;
        
        // Read samples and convert to PCM
        let mut samples = Vec::new();
        
        loop {
            let mut sample: Option<IMFSample> = None;
            let mut flags = 0u32;
            
            reader.ReadSample(
                MF_SOURCE_READER_FIRST_AUDIO_STREAM,
                0,
                None,
                Some(&mut flags),
                None,
                Some(&mut sample),
            )?;
            
            if flags & MF_SOURCE_READERF_ENDOFSTREAM != 0 {
                break;
            }
            
            if let Some(sample) = sample {
                // Convert sample to audio data
                let buffer = sample.ConvertToContiguousBuffer()?;
                let mut data_ptr: *mut u8 = std::ptr::null_mut();
                let mut length = 0u32;
                
                buffer.Lock(&mut data_ptr, None, Some(&mut length))?;
                
                // Process audio data (convert from bytes to f32 samples)
                let sample_count = length as usize / std::mem::size_of::<f32>();
                let audio_data = std::slice::from_raw_parts(data_ptr as *const f32, sample_count);
                samples.extend_from_slice(audio_data);
                
                buffer.Unlock()?;
            }
        }
        
        Ok(samples)
    }
}
```

---

## Windows 10 vs Windows 11 Audio Differences

### Key Architectural Changes

Windows 11 introduced several significant changes to the audio subsystem that developers should be aware of:

### 1. Bluetooth LE Audio Support

| Feature            | Windows 10    | Windows 11              |
| ------------------ | ------------- | ----------------------- |
| Bluetooth LE Audio | Not supported | Native support (v22H2+) |
| LC3 Codec          | Not supported | Native support          |
| Multi-stream Audio | Limited       | Full support            |
| Audio Broadcast    | Not supported | Supported               |

### 2. Enhanced Audio Processing

**Windows 11 Improvements:**

- **Acoustic Echo Cancellation API**: New system-level API for voice applications
- **Voice Clarity**: Enhanced speech processing for clearer communications
- **Improved Audio Engine**: Lower latency and better CPU efficiency (up to 18% improvement reported)
- **Auto HDR for Audio**: Intelligent dynamic range optimization

### 3. User Interface Changes

Windows 11 significantly redesigned the audio settings interface:

| Aspect             | Windows 10        | Windows 11                |
| ------------------ | ----------------- | ------------------------- |
| Volume Mixer       | Legacy mixer      | Modern flyout design      |
| Device Selection   | Control Panel     | Settings app integration  |
| Spatial Sound      | Control Panel     | Quick Settings accessible |
| Audio Enhancements | Device Properties | Modern settings interface |

### 4. API Differences

```rust
// Windows 11 specific features can be detected at runtime
fn check_windows_version() -> Result<(u32, u32), Box<dyn std::error::Error>> {
    use windows::Win32::System::SystemInformation::*;
    
    unsafe {
        let mut version_info: OSVERSIONINFOEXW = std::mem::zeroed();
        version_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
        
        // Note: GetVersionEx is deprecated but still works for basic detection
        // For production, use RtlGetVersion or version helpers
        
        Ok((version_info.dwMajorVersion, version_info.dwMinorVersion))
    }
}
```

### 5. Default Audio Enhancements

Windows 11 enables more audio enhancements by default compared to Windows 10:

- **Loudness Equalization**: Often enabled by default on new installations
- **Bass Boost**: May be enabled depending on device
- **Virtual Surround**: Enabled on compatible devices

### Migration Considerations

When developing applications for both Windows 10 and 11:

```rust
// Use feature detection rather than version detection
fn check_le_audio_support() -> bool {
    // Check for LE Audio support through device capabilities
    // rather than Windows version
    #[cfg(windows)]
    {
        // Try to enumerate LE Audio devices
        // If successful, LE Audio is supported
    }
    false
}
```

---

## Rust Crate Recommendations

### Summary of Recommended Crates

| Crate       | Purpose                  | Level        | Windows Support   |
| ----------- | ------------------------ | ------------ | ----------------- |
| `cpal`      | Cross-platform audio I/O | Mid-level    | WASAPI            |
| `rodio`     | Audio playback           | High-level   | Via cpal          |
| `wasapi`    | Native Windows audio     | Low-level    | Native            |
| `windows`   | Windows API bindings     | Lowest-level | Native            |
| `tinyaudio` | Simple audio output      | High-level   | Cross-platform    |
| `hound`     | WAV file I/O             | File-level   | Platform-agnostic |

### Cargo.toml Example

```toml
[dependencies]
# Cross-platform audio
cpal = "0.15"
rodio = "0.19"

# Windows-specific (conditional)
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Media_Audio",
    "Win32_System_Com",
    "Win32_Media_MediaFoundation",
    "Win32_System_Diagnostics_ToolHelp",
]}
wasapi = "0.14"

# Audio file handling
hound = "3.5"
```

---

## Conclusion

Windows provides a comprehensive audio API ecosystem that Rust developers can leverage through various crates and direct COM interfaces. The key takeaways for audio programming on Windows include:

1. **Choose the Right Abstraction**: Use `rodio` for simple playback, `cpal` for cross-platform control, or `wasapi`/`windows` for full Windows-specific features.

2. **Handle Device Changes**: Audio devices can appear and disappear at any time. Implement proper notification handlers to provide a robust user experience.

3. **Understand the Volume Hierarchy**: Windows has system, endpoint, and application-level volume controls. Choose the appropriate level for your needs.

4. **Respect System Sounds**: System sounds have a special place in Windows audio. Use them appropriately for notifications and user feedback.

5. **Prepare for Windows 11 Features**: Bluetooth LE Audio and improved audio processing are coming. Design your applications to detect and use these features when available.

6. **Test Across Configurations**: Audio hardware varies widely. Test your application with different devices, sample rates, and channel configurations.

The Windows audio stack is powerful and flexible, and with Rust's safety guarantees and the excellent crate ecosystem, building robust audio applications for Windows has never been more accessible.

---

## Additional Resources

- [Microsoft Docs - Core Audio APIs](https://learn.microsoft.com/en-us/windows/win32/coreaudio/core-audio-apis-in-windows-vista)
- [RustAudio Organization on GitHub](https://github.com/RustAudio)
- [cpal Documentation](https://docs.rs/cpal)
- [wasapi Documentation](https://docs.rs/wasapi)
- [Windows Rust API Documentation](https://microsoft.github.io/windows-docs-rs/)
- [Windows 11 LE Audio Support](https://support.microsoft.com/en-us/windows/check-if-a-windows-11-device-supports-bluetooth-low-energy-audio)
