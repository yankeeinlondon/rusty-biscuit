# Biscuit Speaks TTS Library

<table>
<tr>
<td><img src="../assets/biscuit-speaks-512.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>biscuit-speaks</h2>
<p>This library provides TTS functionality to Rust programs:</p>

<ul>
  <li>leverages the host's existing capabilities for small yet cross-platform TTS functionality</li>
  <li>optionally add in <i>cloud</i> support for <a href="https://elevenlabs.io/docs/overview/intro">ElevenLabs</a> TTS</li>
  <li>add the <code>piper</code> feature flag to run TTS models locally </li>
</ul>

<p>
  This library is the TTS functionality behind the <code>so-you-say</code> CLI.
</p>
</td>
</tr>
</table>


## Usage Examples

```rust
use biscuit_speaks::{speak_when_able, VoiceConfig, Gender};

// Simple usage with defaults
speak_when_able("Hello, world!", &VoiceConfig::default());

// With custom voice selection
speak_when_able(
    "Custom voice",
    &VoiceConfig::new()
        .with_name("Samantha")
        .of_gender(Gender::Female)
        .with_volume(0.8),
);
```

## Features

- Cross-platform system TTS (macOS, Windows, Linux)
- Voice selection by name or ID
- Gender and language preferences
- Volume control
- Premium/Enhanced voice quality preference on macOS

