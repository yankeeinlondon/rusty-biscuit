---
name: audio-programming
description: Expert knowledge for programmatic audio across macOS, Linux, Windows, iOS, and Android, with Rust and TypeScript examples. Use when choosing platform audio backends, implementing playback or capture, or troubleshooting cross-platform audio behavior.
---

# Audio Programming by Operating System

This skill will provide detailed knowledge useful for programmatic use of the audio subsystems which modern OS's provide. Use the links provided throughout this skill document to get more details on the areas which are most relevant.

## Desktop Operating Systems

To get details on how to handle on audio on a particular desktop operating system, choose from

### macOS

The following links provide details on how to work with audio on the **macOS** operating system:

- [The macOS Audio Ecosystem & Rust Access](./macOS.md#1-the-macos-audio-ecosystem-rust-access)
- [Addressing Your Audio Goals](./macOS.md#2-addressing-your-audio-goals)
- [The macOS Landscape: Tahoe vs. The Past](./macOS.md#3-the-macos-landscape-tahoe-vs-the-past)
- [Sound Effects versus Normal Audio](./macOS.md#4-sound-effects-versus-normal-audio)
- [Example: Enumerating the output devices](./macOS.md#example-enumerating-the-output-devices)
- [Example #2: Hardware Listeners](./macOS.md#example-2-hardware-listeners)
- [Example #3: Leveraging Sound Effects versus Normal Audio](./macOS.md#example-3-leveraging-sound-effects-versus-normal-audio)

### Windows

The following links provide details on how to work with audio on the **Windows** operating system:

- [Introduction](./windows.md#introduction)
- [Windows Audio Architecture Overview](./windows.md#windows-audio-architecture-overview)
- [Common Audio APIs and Access Methods](./windows.md#common-audio-apis-and-access-methods)
- [Sound Effects on Windows](./windows.md#sound-effects-on-windows)
- [Audio Detection and Monitoring](./windows.md#audio-detection-and-monitoring)
- [Volume Control and Muting](./windows.md#volume-control-and-muting)
- [Audio Device Enumeration](./windows.md#audio-device-enumeration)
- [Audio Routing to Specific Outputs](./windows.md#audio-routing-to-specific-outputs)
- [Audio Ducking (Stream Attenuation)](./windows.md#audio-ducking-stream-attenuation)
- [Native Audio Codec Support](./windows.md#native-audio-codec-support)
- [Windows 10 vs Windows 11 Audio Differences](./windows.md#windows-10-vs-windows-11-audio-differences)
- [Rust Crate Recommendations](./windows.md#rust-crate-recommendations)
- [Conclusion](./windows.md#conclusion)
- [Additional Resources](./windows.md#additional-resources)

### Linux

The following links provide details on how to work with audio on the **Linux** operating system:

- [A Comprehensive Guide for Rust Programmers](./linux.md#a-comprehensive-guide-for-rust-programmers)
- [Linux Audio Architecture Overview](./linux.md#1-linux-audio-architecture-overview)
- [Common Features and APIs for Developers](./linux.md#2-common-features-and-apis-for-developers)
- [Sound Effects on Linux](./linux.md#3-sound-effects-on-linux)
- [Audio Detection and Monitoring](./linux.md#4-audio-detection-and-monitoring)
- [Volume Control and Muting](./linux.md#5-volume-control-and-muting)
- [Audio Input/Output Management](./linux.md#6-audio-inputoutput-management)
- [Audio Routing and Stream Direction](./linux.md#7-audio-routing-and-stream-direction)
- [Audio Ducking](./linux.md#8-audio-ducking)
- [Audio Codec Support](./linux.md#9-audio-codec-support)
- [Historical Evolution and Distribution Variations](./linux.md#10-historical-evolution-and-distribution-variations)
- [Rust Audio Libraries and Examples](./linux.md#11-rust-audio-libraries-and-examples)
- [Summary](./linux.md#summary)

## Mobile Operating Systems

### IOS

Apple's mobile platform **IOS**:

- [Introduction](./IOS.md#introduction)
- [IOS Audio Frameworks Overview](./IOS.md#ios-audio-frameworks-overview)
- [Sound Effects on iOS](./IOS.md#sound-effects-on-ios)
- [Audio Detection and Monitoring](./IOS.md#audio-detection-and-monitoring)
- [Volume Control and Muting](./IOS.md#volume-control-and-muting)
- [Audio Input Sources](./IOS.md#audio-input-sources)
- [Audio Output Routing](./IOS.md#audio-output-routing)
- [Audio Ducking](./IOS.md#audio-ducking)
- [Native Audio Codecs](./IOS.md#native-audio-codecs)
- [Evolution of iOS Audio Support](./IOS.md#evolution-of-ios-audio-support)
- [Rust Libraries for iOS Audio Development](./IOS.md#rust-libraries-for-ios-audio-development)
- [Practical Rust Code Examples](./IOS.md#practical-rust-code-examples)
- [Common Gotchas and Solutions](./IOS.md#common-gotchas-and-solutions)
- [References and Resources](./IOS.md#references-and-resources)
- [Conclusion](./IOS.md#conclusion)

### Android

Google's mobile platform **Android**:

- [Overview of Android Audio APIs](./Android.md#1-overview-of-android-audio-apis)
- [Sound Effects: SoundPool vs MediaPlayer](./Android.md#2-sound-effects-soundpool-vs-mediaplayer)
- [Detecting and Controlling Audio State](./Android.md#3-detecting-and-controlling-audio-state)
- [Audio Inputs and Sources](./Android.md#4-audio-inputs-and-sources)
- [Audio Outputs and Device Routing](./Android.md#5-audio-outputs-and-device-routing)
- [Audio Ducking and Focus Management](./Android.md#6-audio-ducking-and-focus-management)
- [Supported Audio Codecs](./Android.md#7-supported-audio-codecs)
- [Evolution of Android Audio Support](./Android.md#8-evolution-of-android-audio-support)
- [Rust-Based Audio Development on Android](./Android.md#9-rust-based-audio-development-on-android)
- [Summary](./Android.md#summary)

## Software Libraries

### Rust Crates

This section will dig into the `crates` that most Rust developers will consider when developing an application that needs audio support.

- [Introduction](./crates.md#introduction)
- [Cpal (Cross-Platform Audio Library)](./crates.md#1-cpal-cross-platform-audio-library)
- [Rodio](./crates.md#2-rodio)
- [Kira](./crates.md#3-kira)
- [Symphonia](./crates.md#4-symphonia)
- [Fundsp](./crates.md#5-fundsp)
- [Dasp (Digital Audio Signal Processing)](./crates.md#6-dasp-digital-audio-signal-processing)
- [Rubato](./crates.md#7-rubato)
- [Hound](./crates.md#8-hound)
- [Nih-plug](./crates.md#9-nih-plug)
- [Tinyaudio](./crates.md#10-tinyaudio)
- [Coreaudio-rs](./crates.md#11-coreaudio-rs)
- [Oboe](./crates.md#12-oboe)
- [Vst-rs](./crates.md#13-vst-rs)
- [Summary Comparison Table](./crates.md#summary-comparison-table)
- [Choosing the Right Crate](./crates.md#choosing-the-right-crate)
- [Conclusion](./crates.md#conclusion)

### Typescript Libraries

This section will dig into the **npm** libraries that most Typescript developers will consider when developing an application that needs audio support.

- [Introduction](./typescript-libraries.md#introduction)
- [The Web Audio API Foundation](./typescript-libraries.md#the-web-audio-api-foundation)
- [Web Browser Audio Libraries](./typescript-libraries.md#web-browser-audio-libraries)
- [React Native / Mobile Audio Libraries](./typescript-libraries.md#react-native-mobile-audio-libraries)
- [Node.js Desktop Audio Libraries](./typescript-libraries.md#nodejs-desktop-audio-libraries)
- [Platform Support Summary Table](./typescript-libraries.md#platform-support-summary-table)
- [Choosing the Right Library](./typescript-libraries.md#choosing-the-right-library)
- [Common Gotchas Across All Libraries](./typescript-libraries.md#common-gotchas-across-all-libraries)
- [Conclusion](./typescript-libraries.md#conclusion)
