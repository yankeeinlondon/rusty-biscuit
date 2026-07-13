---
prompt: |-
	Your task is to research and document macOS capabilities and features for programmatically working with audio on macOS.

	Through this investigation your assumption will be that you are writing an audio program in Rust. This means all code examples in your research should be Rust based.

	In your research you should try to address the following questions:

	1. What are the common features which application developers look for on the macOS platform to achieve their audio goals? What API's, CLI's or other means of access do developers have available to them?
	2. How can you determine the following things on macOS:
      	- is audio currently playing? what program is playing the audio? 
        	- if the audio output does not provide software based volume control, are there any ways to influence the volume?
      	- what is the current audio volume? how can I mute/unmute?
      	- what are the various audio sources for input? What metadata can you get on each of these inputs?
      	- what are the various audio outputs? What metadata can you get on each of these outputs?
      	- how can I direct audio to a particular audio output? what common gotchas do people hit when trying to do this and how can these obstacles be worked around?
      	- how can I perform "audio ducking" between audio streams?
      	- what audio codec's are natively supported on macOS?
 	3. The latest version of macOS is Tahoe and all of the content outside of this section should try to focus exclusively on Tahoe, however, it's very useful to understand some important differences between older versions of macOS and Tahoe:
    	- **Tahoe versus Sequoia**
		- **Tahoe versus Sonoma**
		- **Tahoe versus Ventura**
		- **Tahoe versus Monterey**

    Your output should be in well-formed idiomatic Markdown.  Use of Mermaid code blocks is welcome if that is helpful in illustrating an idea.
last_updated: 2026-02-27
update_policy:
	- Duration(6mo)
model: Gemini 3 Pro
source: rusty-biscuit/sniff
---
# macOS Audio

## 1. The macOS Audio Ecosystem & Rust Access

Application developers working on macOS primarily interact with **Core Audio**. This is a C-based framework that sits at the lowest level of user-space audio processing. Above that sits Audio Toolbox and AVFoundation (which are Objective-C/Swift-based).

For a Rust developer, you have a few ways to access these capabilities:

* **`coreaudio-sys` / `coreaudio-rs`:** These are the standard bindings. `coreaudio-sys` provides raw FFI bindings to the C API, while `coreaudio-rs` offers a more idiomatic, safe Rust wrapper (primarily focused on Audio Units).
* **`cpal` (Cross-Platform Audio Library):** If you eventually want your code to run on Windows or Linux, `cpal` wraps Core Audio under the hood for macOS.
* **`cubeb-coreaudio-rs`:** Originally developed by Mozilla for Firefox, this is a highly robust and battle-tested Rust implementation for interacting with the Core Audio Hardware Abstraction Layer (HAL).

---

## 2. Addressing Your Audio Goals

Here is how you can achieve your specific objectives, along with some candid realities about the macOS platform.

### Is audio playing? What program is playing it?

**The reality check:** It is a common misconception that macOS easily exposes per-application audio routing natively. It does not. Core Audio strictly protects process boundaries for security and privacy.

* **Is a device active?** You can query `kAudioDevicePropertyDeviceIsRunningSomewhere` on an `AudioDeviceID`. This returns a simple `1` (audio is flowing through the device) or `0` (it is idle). It won't tell you if it's just playing silence, though.
* **What program is it?** You cannot determine this natively from user space. To achieve this, you must build an Audio Server Plug-in or a Kernel Extension (which Apple is actively phasing out) to intercept system audio at the HAL level. Commercial apps like Rogue Amoeba's SoundSource use deeply complex, proprietary audio drivers to achieve this.

### Influencing volume on fixed-volume outputs

If an external DAC or monitor does not expose a software volume control via Core Audio, you cannot force it via a standard volume API.
**The workaround:** You have to intercept the audio stream before it hits the hardware. Developers usually instruct users to create an **Aggregate Device** or **Multi-Output Device** via the Audio MIDI Setup utility, or they use an embedded Audio Unit (like a mixer node) in their Rust app to scale the PCM float values (multiplying the samples by a fractional value) before handing the buffer to the OS.

### Current Volume and Muting

You can get and set the master volume and mute state by querying the specific `AudioDeviceID`. Because high-level wrappers often don't cover system hardware toggles, you will likely need to drop into `coreaudio-sys` and use `unsafe` Rust.

```rust
use coreaudio::sys::{
    AudioObjectPropertyAddress, AudioObjectGetPropertyData,
    kAudioDevicePropertyVolumeScalar, kAudioObjectPropertyScopeOutput,
    kAudioObjectPropertyElementMaster, AudioDeviceID
};
use std::mem;

pub unsafe fn get_device_volume(device_id: AudioDeviceID) -> Result<f32, i32> {
    let mut volume: f32 = 0.0;
    let mut size = mem::size_of::<f32>() as u32;
    
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyVolumeScalar,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMaster,
    };
    
    let status = AudioObjectGetPropertyData(
        device_id,
        &address as *const _,
        0,
        std::ptr::null(),
        &mut size,
        &mut volume as *mut _ as *mut _,
    );
    
    if status == 0 { Ok(volume) } else { Err(status) }
}

```

*Note: Swap `kAudioDevicePropertyVolumeScalar` with `kAudioDevicePropertyMute` (and change the type to `u32`) to handle muting.*

### Identifying Audio Inputs, Outputs, and Metadata

To list devices, you first query the system object (`kAudioObjectSystemObject`) for `kAudioHardwarePropertyDevices`. This returns an array of all `AudioDeviceID`s.

To separate inputs from outputs, you must query each device's `kAudioDevicePropertyStreams`.

* If a device has streams in `kAudioDevicePropertyScopeInput`, it's an input source.
* If a device has streams in `kAudioDevicePropertyScopeOutput`, it's an output destination.

**Available Metadata:** Once you have the device ID, you can query properties like `kAudioObjectPropertyName` (human-readable name), `kAudioDevicePropertyDeviceUID` (persistent unique identifier), `kAudioDevicePropertyNominalSampleRate`, and `kAudioDevicePropertyAvailableNominalSampleRates`.

### Directing Audio and Common Gotchas

You can set the default system output by setting `kAudioHardwarePropertyDefaultOutputDevice` on the System Object.

**Common Gotchas:**

1. **Sample Rate Mismatches:** If your Rust application is generating audio at 48kHz, but you route it to a device locked at 44.1kHz, Core Audio will throw an error unless you insert an Audio Converter node into your graph.
2. **Clock Drift:** When routing audio between multiple outputs (Aggregate Devices), the hardware clocks will drift, causing artifacts. You must ensure Core Audio's drift compensation is enabled.
3. **Permissions:** macOS requires the `com.apple.security.device.audio-input` entitlement. If you don't declare this in your app's `Info.plist`, your Rust app will silently fail to capture any microphone data.

### Audio Ducking

macOS does not have a simple "duck this specific app" API. However, if your Rust app manages its own `AVAudioSession` (via Objective-C bindings like `objc2`), you can set the session category to allow ducking background audio. At the lower Core Audio level, you can query `kAudioDevicePropertyDeviceCanDoDucking`, but actual implementation usually requires you to apply a sidechain compressor to your own audio unit graph, measuring the RMS of the incoming vocal stream and scaling down the music stream's volume accordingly.

### Natively Supported Codecs

Through Core Audio's `AudioFormatID`, macOS natively supports:

* Linear PCM (The standard format for Core Audio processing)
* AAC / HE-AAC
* ALAC (Apple Lossless)
* MP3 (Decoding is fully supported; encoding requires specific licensing setups historically, but is widely available now)
* FLAC
* AC-3 / E-AC-3

---

## 3. The macOS Landscape: Tahoe vs. The Past

As you know, Apple jumped the versioning to macOS 26 (Tahoe) to align with iOS 26. When writing hardware-level audio code, OS version differences matter immensely. Here is how Tahoe compares to its predecessors:

| Feature/Behavior | macOS 26 (Tahoe) | Older Versions (Sequoia, Sonoma, Ventura, Monterey) |
| --- | --- | --- |
| **System Architecture** | Tahoe is the absolute final release to support Intel Macs and Rosetta 2. Your Rust targets should heavily prioritize `aarch64-apple-darwin`. | **Sequoia (15) / Sonoma (14):** Maintained broad Intel support. **Ventura (13) / Monterey (12):** Intel was still a primary focus. |
| **Hardware Bugs** | **Current Tahoe Bug:** There is a known behavior where disconnecting certain USB Audio interfaces does not properly detach the kernel driver, leaving ghost devices in the output list. You must handle device invalidation gracefully in Rust. | **Older OS:** USB devices generally detached cleanly upon physical removal. |
| **MIDI Protocols** | Tahoe introduces native **Network MIDI 2.0 sessions via UDP**, supporting Universal MIDI Packets (UMP) and automatic mDNS discovery directly in the Audio MIDI Setup. | **Older OS:** Relied primarily on Legacy RTP MIDI (MIDI 1.0) for network sessions. |
| **AI Audio APIs** | Introduces system-level Live Translation and heavy Apple Intelligence integration into standard audio streams. | **Sequoia (15):** Introduced AI foundations. **Monterey (12):** Introduced Voice Isolation and Wide Spectrum audio processing. |
| **Audio App Routing** | System audio routing remains strictly locked down; relies heavily on the `Liquid Glass` redesigned Control Center for user-facing changes. | **Ventura (13):** Redesigned System Settings introduced minor breaking changes to how third-party apps hooked into audio preference panes. |


## 4. Sound Effects versus Normal Audio

You made an excellent observation. You are absolutely right: macOS natively treats "sound effects" (alerts, notifications, and UI feedback) as a completely separate audio routing pathway from standard media playback.

When you look at the macOS System Settings under the "Sound" panel, you will notice that users can choose a general "Output" device, but they also have a separate dropdown for "Play sound effects through."

Here is a breakdown of how this works under the hood in Core Audio, how it impacts your Rust application, and how you can leverage it.

1. The Core Audio Distinction
In the Apple ecosystem, the OS maintains two distinct default output targets at the hardware level. These are represented by two different property selectors on the System Object:

kAudioHardwarePropertyDefaultOutputDevice: This is the destination for standard audio (music, video playback, DAWs, video calls). If your Rust app is generating general media, it should route its stream here.

kAudioHardwarePropertyDefaultSystemOutputDevice: This is the destination specifically reserved for system alerts and interface sounds.

The Reality Check: By default, macOS points both of these properties to the same AudioDeviceID. However, a user (or an application) can split them. For example, a user might route their Spotify music to their high-end USB DAC (DefaultOutputDevice), but route the "ping" of new messages to their Mac Studio's tiny internal speaker (DefaultSystemOutputDevice) so it doesn't interrupt their music listening experience.

2. Volume and Ducking Differences
Sound effects carry a few unique behaviors that standard audio does not:

Independent Volume Scaling: System sound effects have their own dedicated volume multiplier (the "Alert volume" slider in System Settings). If you route audio to the DefaultSystemOutputDevice, it will be subject to this secondary gain stage, meaning your audio might play much quieter than you expect if the user has turned down their alert volume.

Automatic Ducking: If a user is listening to music and a system sound effect plays, macOS automatically ducks (lowers the volume of) the DefaultOutputDevice for the duration of the alert, then ramps it back up.

3. How to Handle This in Rust
If you are writing a Rust app and you want to play a notification sound (like a chat "ding" or a task completion chime), you have two choices depending on how low-level you want to go.

Option A: The Core Audio Route (Manual)
If you are already managing your own coreaudio-sys output streams, you would simply query the System Device property instead of the Output Device property to find out where to send your audio buffers:

Rust
use coreaudio_sys::{
    kAudioHardwarePropertyDefaultSystemOutputDevice, // <-- The key difference
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal,
    AudioObjectPropertyAddress,
};

// You use this address to find the AudioDeviceID for sound effects
let sfx_device_address = AudioObjectPropertyAddress {
    mSelector: kAudioHardwarePropertyDefaultSystemOutputDevice,
    mScope: kAudioObjectPropertyScopeGlobal,
    mElement: kAudioObjectPropertyElementMain,
};
Option B: The AudioToolbox Route (Easier)
If you just want to play a short sound effect and don't want to build an entire Core Audio graph for it, macOS provides a higher-level C API called System Sound Services (part of AudioToolbox). This API automatically respects the user's sound effects routing, alert volume, and handles the ducking for you.

Tahoe vs. Older Versions
In macOS 26 (Tahoe), this architecture remains structurally identical to Sequoia and Sonoma. However, Tahoe's deeper integration of spatial audio means that if the user's DefaultSystemOutputDevice is a pair of AirPods, system sound effects are now processed with a fixed, subtle spatialization to make them sound like they are coming from the physical location of the Mac screen, even if the main media is being rendered in full dynamic head-tracking spatial audio.


---

Would you like me to draft a complete, runnable Rust function utilizing `coreaudio-sys` that queries and prints out all available audio output devices and their unique IDs?

## Example: Enumerating the output devices

Getting this information out of Core Audio requires navigating its object hierarchy. In Core Audio, everything is an `AudioObject` (the system, the devices, the streams), and you interact with them by asking for specific properties.

To find output devices, we have to start at the top (the System Object), get all devices, and then check each device to see if it has output streams.

Here is a complete, runnable Rust example. Be prepared: because `coreaudio-sys` is a direct wrapper around Apple's C API, there is a lot of `unsafe` code, manual memory sizing, and pointer passing.

### 1. Project Setup (`Cargo.toml`)

You will need the `coreaudio-sys` crate for the Core Audio API, and the `core-foundation` crate to safely handle macOS string types (`CFString`), which is how Apple returns device names and IDs.

```toml
[dependencies]
coreaudio-sys = "0.2"
core-foundation = "0.9"

```

### 2. The Rust Implementation

```rust
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use coreaudio_sys::{
    kAudioDevicePropertyDeviceUID, kAudioDevicePropertyStreams, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMaster, kAudioObjectPropertyElementMain, kAudioObjectPropertyName,
    kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
    AudioDeviceID, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize,
    AudioObjectPropertyAddress, OSStatus,
};
use std::mem;
use std::ptr;

/// Helper to easily construct property addresses
fn property_address(selector: u32, scope: u32, element: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

pub fn list_output_devices() -> Result<(), String> {
    unsafe {
        // 1. Ask the System Object how much memory we need to hold all Device IDs
        let address = property_address(
            kAudioHardwarePropertyDevices,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain, // Master element is deprecated in newer macOS
        );
        
        let mut data_size: u32 = 0;
        let status = AudioObjectGetPropertyDataSize(
            kAudioObjectSystemObject,
            &address,
            0,
            ptr::null(),
            &mut data_size,
        );
        if status != 0 { return Err(format!("Failed to get device list size: {}", status)); }

        // 2. Fetch the actual Device IDs
        let device_count = data_size as usize / mem::size_of::<AudioDeviceID>();
        let mut device_ids: Vec<AudioDeviceID> = vec![0; device_count];
        
        let status = AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &address,
            0,
            ptr::null(),
            &mut data_size,
            device_ids.as_mut_ptr() as *mut _,
        );
        if status != 0 { return Err(format!("Failed to get device IDs: {}", status)); }

        println!("Found {} total audio devices. Filtering for outputs...\n", device_count);

        // 3. Iterate through each device and check for output streams
        for &device_id in &device_ids {
            // Check output streams
            let stream_address = property_address(
                kAudioDevicePropertyStreams,
                kAudioObjectPropertyScopeOutput, // Look specifically at the Output scope
                kAudioObjectPropertyElementMain,
            );
            
            let mut stream_data_size: u32 = 0;
            AudioObjectGetPropertyDataSize(device_id, &stream_address, 0, ptr::null(), &mut stream_data_size);
            
            // If the device has no output streams, skip it (it's likely an input-only device)
            if stream_data_size == 0 {
                continue; 
            }

            // 4. It's an output device! Fetch its Name and UID
            let name = get_device_string_property(device_id, kAudioObjectPropertyName)
                .unwrap_or_else(|| "Unknown Device".to_string());
                
            let uid = get_device_string_property(device_id, kAudioDevicePropertyDeviceUID)
                .unwrap_or_else(|| "Unknown UID".to_string());

            println!("Output Device: {}", name);
            println!("  ID:  {}", device_id);
            println!("  UID: {}\n", uid);
        }
    }
    Ok(())
}

/// Helper function to fetch a CFString property and convert it to a Rust String
unsafe fn get_device_string_property(device_id: AudioDeviceID, selector: u32) -> Option<String> {
    let address = property_address(
        selector,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );

    let mut cf_string_ref: core_foundation::sys::string::CFStringRef = ptr::null();
    let mut data_size: u32 = mem::size_of_val(&cf_string_ref) as u32;

    let status = AudioObjectGetPropertyData(
        device_id,
        &address,
        0,
        ptr::null(),
        &mut data_size,
        &mut cf_string_ref as *mut _ as *mut _,
    );

    if status == 0 && !cf_string_ref.is_null() {
        // Wrap the raw C pointer in a safe Rust Core Foundation type
        let cf_string = CFString::wrap_under_create_rule(cf_string_ref);
        Some(cf_string.to_string())
    } else {
        None
    }
}

fn main() {
    if let Err(e) = list_output_devices() {
        eprintln!("Error: {}", e);
    }
}

```

### Why this approach matters

Notice how we check for `kAudioDevicePropertyStreams` under `kAudioObjectPropertyScopeOutput`. This is the single most reliable way on macOS to distinguish a microphone from a speaker. Relying on naming conventions or other properties often leads to edge-case bugs with virtual audio drivers (like BlackHole or Loopback).

## Example #2: Hardware Listeners

Setting up hardware listeners is where the rubber meets the road in macOS audio development. It's incredibly satisfying when it works, but bridging C-based asynchronous callbacks into safe Rust requires some careful maneuvering.

Here is a reality check about how macOS handles this: you rarely listen to a specific device to see if it was "unplugged." Instead, you listen to **property changes**. When a user unplugs their headphones, macOS automatically reroutes the audio to the built-in speakers. To detect this, the most reliable method is to watch the **System Object** and listen for a change to its `kAudioHardwarePropertyDefaultOutputDevice` property.

Let's look at how to implement this in Rust.

---

### The Property Listener Implementation

To do this, we need to create an `extern "C"` function that Core Audio can call from its own background thread, and then register that function using `AudioObjectAddPropertyListener`.

Here is the complete, runnable code to listen for headphone unplugs (or any default output change):

```rust
use coreaudio_sys::{
    kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject, AudioObjectAddPropertyListener,
    AudioObjectID, AudioObjectPropertyAddress, OSStatus,
};
use std::ffi::c_void;
use std::ptr;
use std::thread;
use std::time::Duration;

/// 1. The C-compatible callback function
/// Core Audio will fire this on a background thread whenever the monitored property changes.
extern "C" fn default_device_changed_callback(
    _object_id: AudioObjectID,
    _number_addresses: u32,
    _addresses: *const AudioObjectPropertyAddress,
    _client_data: *mut c_void,
) -> OSStatus {
    // Note: Because this runs on a high-priority Core Audio thread, 
    // you should avoid doing heavy work or allocating memory here in a production app.
    println!("🔔 Hardware change detected! The default output device just changed.");
    
    0 // Return noErr (0) to indicate success
}

pub fn listen_for_route_changes() -> Result<(), String> {
    unsafe {
        // 2. Define the exact property we want to watch
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain, // Master is deprecated, use Main
        };

        // 3. Register the listener on the System Object
        let status = AudioObjectAddPropertyListener(
            kAudioObjectSystemObject,
            &address,
            Some(default_device_changed_callback),
            ptr::null_mut(), // The 'client_data' pointer (useful for passing Rust state)
        );

        if status != 0 {
            return Err(format!("Failed to add property listener: {}", status));
        }

        println!("🎧 Listening for default device changes. Try plugging/unplugging headphones...");
        println!("(Press Ctrl+C to exit)\n");

        // 4. Keep the main thread alive. 
        // Core Audio handles the background listener thread automatically, 
        // but if our main function exits, the program terminates.
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn main() {
    if let Err(e) = listen_for_route_changes() {
        eprintln!("Error: {}", e);
    }
}

```

---

### Breaking Down the Mechanics

* **The Callback Signature:** The `extern "C"` block ensures the Rust compiler formats the function exactly how the C-based Core Audio framework expects it.
* **The Threading Reality:** When you unplug your headphones, Core Audio invokes `default_device_changed_callback` from one of its internal, high-priority threads. In a real-world application, doing a `println!` inside this callback is dangerous because I/O can block the thread, potentially causing audio glitches.
* **The `client_data` Pointer:** In the `AudioObjectAddPropertyListener` call, we passed `ptr::null_mut()`. This is a void pointer `*mut c_void` that Core Audio will hand back to you inside the callback. It is the key to escaping the C-world and getting back into safe Rust.

To make this production-ready, you cannot just print to the console. You need a way to tell your main Rust application that the device changed so you can tear down your old audio stream and build a new one.

## Example #3: Leveraging Sound Effects versus Normal Audio

Using the `AudioToolbox` framework is a breath of fresh air compared to manually wrestling with Core Audio streams. It abstracts away sample rates, buffer sizes, and hardware routing.

When you use System Sound Services (a specific C API within AudioToolbox), the OS assumes you are playing a UI sound or an alert. It will automatically route the audio to the `kAudioHardwarePropertyDefaultSystemOutputDevice`, apply the user's specific "Alert volume" multiplier, and handle the background ducking of their Spotify or Apple Music stream.

Here is how you can implement this in Rust to play one of the built-in macOS alert sounds (or any short `.wav`/`.aiff` file you provide).

### 1. Project Setup (`Cargo.toml`)

You will need the same dependencies as before. `coreaudio-sys` helpfully includes the bindings for `AudioToolbox` alongside `CoreAudio`.

```toml
[dependencies]
coreaudio-sys = "0.2"
core-foundation = "0.9"

```

### 2. The Rust Implementation

The biggest "gotcha" developers hit with this API is that **playback is asynchronous**. If you tell the OS to play the sound and your Rust function immediately returns (or your CLI app exits), the audio will cut off instantly or never play at all.

```rust
use core_foundation::url::CFURL;
use coreaudio_sys::{
    AudioServicesCreateSystemSoundID, AudioServicesDisposeSystemSoundID,
    AudioServicesPlaySystemSound, SystemSoundID,
};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn play_sound_effect(file_path: &Path) -> Result<(), String> {
    // 1. Create a Core Foundation URL from our standard Rust Path
    let url = CFURL::from_path(file_path, false)
        .ok_or_else(|| "Failed to create CFURL from the provided path.".to_string())?;

    unsafe {
        let mut sound_id: SystemSoundID = 0;

        // 2. Register the audio file with the system to get a SystemSoundID
        // We cast the safe CFURL reference to the raw C pointer expected by the FFI
        let status = AudioServicesCreateSystemSoundID(
            url.as_concrete_TypeRef() as *mut _,
            &mut sound_id,
        );

        if status != 0 {
            return Err(format!("Failed to create SystemSoundID. OSStatus: {}", status));
        }

        println!("🔊 Triggering sound effect via AudioToolbox...");

        // 3. Instruct macOS to play it. 
        // This fires asynchronously on a background system daemon.
        AudioServicesPlaySystemSound(sound_id);

        // 4. The Gotcha: We must keep the thread alive long enough for the sound to finish.
        // For a simple CLI tool, sleeping works. In a GUI app, you'd just let the event loop run.
        thread::sleep(Duration::from_secs(2));

        // 5. Clean up the system memory allocation
        AudioServicesDisposeSystemSoundID(sound_id);
    }

    Ok(())
}

fn main() {
    // macOS ships with several classic alert sounds out of the box.
    // Let's use the classic 'Glass' sound effect.
    let sound_path = Path::new("/System/Library/Sounds/Glass.aiff");
    
    if let Err(e) = play_sound_effect(sound_path) {
        eprintln!("Error: {}", e);
    }
}

```

### The Limitations of System Sound Services

While this API is incredibly convenient, it is intentionally limited. You should only use it for audio under 30 seconds. Furthermore:

* **No volume control:** You cannot programmatically change the volume of this specific sound; it is entirely at the mercy of the user's system-wide alert volume slider.
* **No looping:** You cannot tell the API to loop a sound infinitely.
* **No spatial positioning:** Unless it's the subtle, automatic screen-centric spatialization applied in macOS Tahoe to AirPods, you cannot pan this audio left or right.

If you need volume control, panning, or playback for longer files (like background music in a game), you have to step up to building an **Audio Unit Graph** (using `AUGraph` or `AVAudioEngine`).


