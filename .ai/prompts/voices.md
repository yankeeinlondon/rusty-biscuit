# Voices and Gender

We recently refactored the `biscuit-speaks` library and while it was a step in the right direction is has caused some regressions:

- we can no longer list the voices which are "available" to a host
- there is some sort of variance when we specify gender but no voices sound male (just different)
- `echogarden` is detected as a "provider" but does not work (at least not via the `so-you-say` CLI)
- A LOT of the other providers we were "supposed" to support were NOT implemented at all!

## Solution Approach

Some of the symbols discussed here already have a good code starting point but because you find the symbol already in code does NOT mean it is complete. Always try to evaluate if an existing symbol can be made better.

### Voice Inventory By Provider

To solve this we will start with identifying the available voices the host has, not in general, but _per_ provider. When we identify the available voices we will also try to discern:

- the voice's gender
- the language(s) supported for the voice
- the quality level of the voice (`VoiceQuality` enum)
- a host's providers, and those provider's capabilities will be cached


## Technical Solution

### `TtsExecutor` Trait

we have a `TtsExecutor` trait which defines the _interface_ which we must implement for each "provider" which we hope to support.

- Currently the only requirement for a provider is that implement the `speak()` function but going forward this will not be enough
- We will add the following:
    - `is_ready()`
          - We can quickly detect whether a given host has the TTS providers _executable program_ (for local providers) or the appropriate API Key (for cloud providers) and for some providers this is all that is required to be able to use the service
          - However, other providers, require one or more of the following:
              - has an internet connection (any cloud solution)
              - has the appropriate API Keys
              - has an ENV variable to point to a valid TTS model (for local abstracted solutions like `sherpa-onnx` or `echogarden`)
              - has an ENV variable to point to valid Voice samples
              - has a complimentary audio player which can render it's audio output
                  - some TTS solutions will send audio directly to OS's audio channel, others require
    - `list_voices()`
        - each provider -- local or cloud -- must be able to detect what voices are available.
        - voices must be defined to a gender; this includes TTS solutions which don't always explicitly express this in their metadata. To resolve gender-non-specific voices we will use the `gender_guesser` crate which uses a simple lookup based approach to assigning gender.
        - voices must also be matched to 1:M languages
        - voices must also be assigned a "quality" setting (`VoiceQuality`)
    - `info()`
        - will provide a markdown string of information about the provider
        - CLI's like `so-you-say` will use the Darkmatter library to provide rich descriptions of the various providers to the console
    - `available_voices()`
        - provides a list of voices which the TTS solution _can_ download and use but which are NOT currently on the host machine
        - we should only ever list voices which are of Good or Excellent voice quality
            - for instance the `say` command for a user probably has a subset of voices which "are" available
    - `async download_voice<T: DownloadableVoice>(voice: T) -> void`
        - for TTS providers where we can offer a simple library call to download a set of voices we will provide an enumeration with the available voices for download
        - this enum must be a member of the `DownloadableVoice` trait
        - for TTS providers which do not provides an enumerated set of voices for download this function will use the shared `NoDownloadableVoice` enumeration (_which is a member of `DownloadableVoice`) which has no valid options to choose from


### Host Capabilities Cache

In order to avoid having to _re-discover_ a given host's capabilities each time we will save a cache file with a host's capabilities. These capabilities are broken down by the provider's which are found available on the host and will be saved to a cache file located at `${HOME}/.biscuit-speaks-cache`.

#### Creating and Cache Busting

- the `biscuit-speaks` library will expose two utility functions:
    - `read_from_cache() -> HostTtsCapability`
    - and `update_provider_in_cache(provider, capabilities)` (primary user will be the )
    - and `bust_host_capability_cache()`
- the `so-you-speak` CLI will offer a `--force` flag which will bust the cache and force host rediscovery
- the `biscuit-speaks` providers will by default leverage the cached capabilities, so a caller of `Speak` struct should first call `bust_host_capability_cache()` if they want to ensure that the cache is refreshed.



### Adding Providers

We need to add the following providers:

- `SAPI` (on Windows)
- `gTTS`
- `echogarden`
- `kokoro-tts`

All of these will need a mechanism to procure the metadata around voices, voice quality, languages, etc. For "echogarden" we'll also rely on ENV variables to know where to get the model and voices.

#### SAPI

Windows Speech API (SAPI) provides comprehensive COM-based access to TTS voices and their metadata.

**Detection & Readiness**:

- Check for SAPI availability via `windows` crate COM interfaces
- Voices stored in registry under `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens`
- Third-party voices may register as **token enumerators** (dynamic generation)

**Voice Metadata Available**:

| Field | Registry Key | Example |
|-------|--------------|---------|
| Voice Name | (Default) | "Microsoft David Desktop" |
| Voice ID | CLSID | GUID string |
| Language | Language | "409" (en-US), "809" (en-GB) |
| Gender | Gender | "Male", "Female", "Neutral" |
| Age | Age | "Adult", "Child", "Senior" |
| Vendor | Vendor | "Microsoft", "Cepstral" |

**Implementation Approach**:

```rust
// Use windows-rs crate with COM interfaces
use windows::Win32::Media::Speech::{
    ISpObjectTokenCategory, SpObjectTokenCategory, SPCAT_VOICES,
};

// 1. Initialize COM (COINIT_MULTITHREADED)
// 2. Create SpObjectTokenCategory for SPCAT_VOICES
// 3. Enumerate tokens via EnumTokens()
// 4. Extract attributes from each token's "Attributes" subkey
// 5. Release COM resources
```

**Gender Resolution**: SAPI provides explicit gender metadata in the `Gender` attribute. Map "Male"/"Female"/"Neutral" to our `Gender` enum directly.

**Quality Assignment**: SAPI has no explicit "quality" attribute. Use registry location and vendor to infer quality:

- **OneCore voices** (from `HKLM\SOFTWARE\Microsoft\Speech_OneCore\Voices\Tokens`) → `VoiceQuality::Excellent`
  - These are Microsoft's neural voices, token names follow pattern `MSTTS_V110_*`
- **Standard Microsoft SAPI5 voices** (from `HKLM\SOFTWARE\Microsoft\Speech\Voices\Tokens`, Vendor="Microsoft") → `VoiceQuality::Good`
- **Third-party neural voices** (known vendors: Nuance, Cepstral, Acapela) → `VoiceQuality::Good`
- **Legacy/unknown voices** → `VoiceQuality::Moderate`

**Implementation Note**: To access OneCore voices via SAPI, enumerate from the `Speech_OneCore` registry path:
```rust
// OneCore voices require reading from alternate registry location
let onecore_path = "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Speech_OneCore\\Voices\\Tokens";
// Standard SAPI5 voices
let sapi5_path = "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Speech\\Voices\\Tokens";
```

**Caveats**:

- COM initialization must use RAII patterns for proper resource cleanup
- Token enumerators (third-party voices) require dynamic enumeration, not static registry reads
- Language codes are LCID hex values (e.g., "409" = en-US), need conversion to BCP-47
- OneCore voices may not be directly accessible via standard SAPI5 COM interfaces without registry path override

#### gTTS

gTTS (Google Text-to-Speech) is a Python CLI wrapper around Google Translate's TTS service.

**Detection & Readiness**:

- Check if `gtts-cli` executable is in PATH
- Requires internet connectivity (cloud-only service)
- No API key required (uses public Google Translate endpoint)

**Voice Metadata Available**:

| Field | Source | Example |
|-------|--------|---------|
| Language Code | CLI output | "en", "fr-CA", "zh-CN" |
| Language Name | CLI output | "English", "French (Canada)" |

**Limited metadata**: gTTS only provides language selection, not individual voice selection within languages. No gender, age, or quality indicators are available.

**Implementation Approach**:

```rust
use std::process::Command;

// Execute: gtts-cli --all
// Parse output format: "  code: Name" (one per line)
// Example: "  fr-CA: French (Canada)"

fn parse_gtts_line(line: &str) -> Option<GttsLanguage> {
    let line = line.trim();
    let (code, name) = line.split_once(": ")?;
    Some(GttsLanguage { code: code.to_string(), name: name.to_string() })
}
```

**Gender Resolution**: gTTS does not expose voice gender. Use the `gender_guesser` crate on the language name or default to `Gender::Unknown`. Since Google's voices are often neural and gender-neutral, this is acceptable.

**Quality Assignment**: All gTTS voices use the same Google Translate backend, so quality is **uniform** across all languages. Assign `VoiceQuality::Good` to all voices - Google's TTS is neural-based and reasonably high quality, though not as natural as dedicated services like ElevenLabs.

**Regional Accents via TLD**: The `tld` parameter controls regional accent (not quality). This can be used to offer accent variants for the same language:

| Accent | Language | TLD |
|--------|----------|-----|
| Australian English | en | com.au |
| British English | en | co.uk |
| American English | en | us |
| Indian English | en | co.in |
| Canadian French | fr | ca |
| Brazilian Portuguese | pt | com.br |
| Mexican Spanish | es | com.mx |

```bash
# CLI usage for accent control
gtts-cli "Hello" --lang en --tld co.uk --output hello.mp3
```

**Caveats**:

- **Network required**: Will fail offline; `is_ready()` must verify internet connectivity
- **Rate limiting**: Google may throttle heavy usage
- **No voice selection**: Cannot choose between male/female voices for same language
- **Privacy**: Text is sent to Google servers
- **Python dependency**: Requires `gtts` Python package installed (`pip install gTTS`)

#### echogarden

Echogarden is a multi-engine TTS CLI (via npm) that abstracts multiple TTS backends. For `biscuit-speaks`, we only use the **vits** and **kokoro** engines (high-quality local neural TTS).

**Detection & Readiness**:

- Check if `echogarden` executable is in PATH (installed via `npm install -g echogarden`)
- Fully offline for local engines (vits, kokoro)
- No ENV variables required for basic operation

**Voice Metadata Available**:

| Field | Source | Example |
|-------|--------|---------|
| Identifier | CLI output | "Heart", "en_GB-alba-medium" |
| Languages | CLI output | "American English (en-US), English (en)" |
| Gender | CLI output | "female", "male", "unknown" |
| Speaker Count | CLI output (optional) | 12 (for multi-speaker models) |

**Implementation Approach**:

```rust
use std::process::Command;

// Step 1: List available engines
// Execute: echogarden list-engines speak
// Filter to only "vits" and "kokoro" (local high-quality engines)

// Step 2: List voices per engine
// Execute: echogarden list-voices kokoro
// Output format (blank-line separated records):
//   Identifier: Heart
//   Languages: American English (en-US), English (en)
//   Gender: female

// Execute: echogarden list-voices vits
// Output format includes quality suffix in identifier:
//   Identifier: en_GB-alba-medium
//   Languages: British English (en-GB), English (en)
//   Gender: female

fn parse_echogarden_voice_block(block: &str) -> Option<EchogardenVoice> {
    let mut identifier = None;
    let mut languages = Vec::new();
    let mut gender = Gender::Unknown;

    for line in block.lines() {
        if let Some(id) = line.strip_prefix("Identifier: ") {
            identifier = Some(id.to_string());
        } else if let Some(langs) = line.strip_prefix("Languages: ") {
            // Parse "American English (en-US), English (en)" format
            languages = parse_language_list(langs);
        } else if let Some(g) = line.strip_prefix("Gender: ") {
            gender = match g {
                "female" => Gender::Female,
                "male" => Gender::Male,
                _ => Gender::Unknown,
            };
        }
    }

    Some(EchogardenVoice { identifier: identifier?, languages, gender })
}
```

**Gender Resolution**: Echogarden provides explicit gender in the output ("female"/"male"/"unknown"). Map directly to our `Gender` enum.

**Quality Assignment**:

- **kokoro engine**: All voices → `VoiceQuality::Excellent`
- **vits engine**: Quality encoded in identifier suffix:
    - `-low` suffix → Filter out (do not include)
    - `-medium` suffix → `VoiceQuality::Moderate`
    - `-high` suffix → `VoiceQuality::Good`

**Speaking**:

```bash
echogarden speak "Hello World." --engine=kokoro --voice=Michael
echogarden speak "Hello World." --engine=vits --voice=en_GB-alba-medium
```

**Caveats**:

- Must specify both `--engine` and `--voice` when speaking
- Output is blank-line separated records, not line-by-line
- vits voices have quality suffix in identifier; kokoro voices do not
- Some vits voices have `Speaker count: N` for multi-speaker models
- Requires Node.js runtime (npm package)

#### kokoro-tts

Kokoro TTS is a high-quality neural TTS CLI based on the StyleTTS 2 architecture. Requires explicit model and voice file paths.

**Detection & Readiness**:

- Check if `kokoro-tts` executable is in PATH
- **Required ENV variables**:
    - `KOKORO_MODEL` → path to `kokoro-v1.0.onnx` (or similar)
    - `KOKORO_VOICES` → path to `voices-v1.0.bin` (or similar)
- Both ENV variables must be set for `is_ready()` to return true
- Fully offline, no network required

**Voice Metadata Available**:

| Field | Source | Example |
|-------|--------|---------|
| Voice Name | CLI output | "af_bella", "am_michael", "bf_emma" |
| Gender | Inferred from prefix | "f" = female, "m" = male |
| Region/Accent | Inferred from prefix | "a" = American, "b" = British, etc. |

**Voice Naming Convention**:
The two-letter prefix encodes region and gender:

- First letter: region (`a`=American, `b`=British, `e`=Spanish, `f`=French, `h`=Hindi, `i`=Italian, `j`=Japanese, `p`=Portuguese, `z`=Chinese)
- Second letter: gender (`f`=female, `m`=male)

Examples: `af_bella` (American female), `bm_george` (British male), `jf_nezumi` (Japanese female)

**Supported Languages**: `cmn`, `en-gb`, `en-us`, `fr-fr`, `it`, `ja`

**Implementation Approach**:

```rust
use std::process::Command;
use std::env;

// Check readiness
fn is_ready() -> bool {
    env::var("KOKORO_MODEL").is_ok() && env::var("KOKORO_VOICES").is_ok()
}

// Execute: kokoro-tts --model "${KOKORO_MODEL}" --voices "${KOKORO_VOICES}" --help-voices
// Output format (numbered list after header):
//   Supported voices:
//       1. af_alloy
//       2. af_aoede
//       ...

fn parse_kokoro_voice_line(line: &str) -> Option<KokoroVoice> {
    // Line format: "    1. af_bella"
    let line = line.trim();
    let voice_name = line.split(". ").nth(1)?.to_string();

    // Parse gender from second character of prefix
    let gender = match voice_name.chars().nth(1) {
        Some('f') => Gender::Female,
        Some('m') => Gender::Male,
        _ => Gender::Unknown,
    };

    Some(KokoroVoice { name: voice_name, gender })
}

// List languages: kokoro-tts --help-languages
// Output: cmn, en-gb, en-us, fr-fr, it, ja
```

**Gender Resolution**: Infer from the second character of the voice name prefix (`f`=Female, `m`=Male).

**Quality Assignment**: All kokoro voices → `VoiceQuality::Excellent` (high-quality neural synthesis based on StyleTTS 2).

**Speaking** (outputs WAV file):

```rust
use tempfile::tempdir;
use std::fs;
use std::process::Command;

fn speak(text: &str, voice: &str) -> Result<PathBuf, Error> {
    let dir = tempdir()?;
    let input_path = dir.path().join("input.txt");
    let output_path = dir.path().join("output.wav");

    fs::write(&input_path, text)?;

    Command::new("kokoro-tts")
        .arg(&input_path)
        .arg(&output_path)
        .arg("--model").arg(env::var("KOKORO_MODEL")?)
        .arg("--voices").arg(env::var("KOKORO_VOICES")?)
        .arg("--voice").arg(voice)
        .status()?;

    Ok(output_path)
}
```

**Caveats**:

- Outputs WAV file (not direct audio playback) - requires separate audio player
- Must use `tempfile` crate for input text file (CLI reads from file, not stdin)
- Voice/language mismatch produces error with helpful message listing valid languages
- The `--lang` flag sets output language but voice accent is determined by voice prefix
- CLI outputs informational message about `pymupdf_layout` package (can be ignored)

### Updating Existing Single Model TTS providers

#### say

macOS native text-to-speech via the `say` command-line tool.

**Detection & Readiness**:

- Check if `say` executable exists (macOS only, always present)
- No network or API keys required (fully local)
- Additional voices can be installed via System Preferences > Accessibility > Spoken Content

**Voice Metadata Available**:

| Field | Source | Example |
|-------|--------|---------|
| Voice Name | CLI output | "Albert", "Allison (Enhanced)", "Bad News" |
| Locale Code | CLI output | "en_US", "it_IT", "fr_CA" |
| Sample Text | CLI output | "Hello! My name is Albert." |

**Limited metadata**: No gender, age, or quality indicators are directly provided. Must be inferred.

**Implementation Approach**:

```rust
use std::process::Command;

// Execute: say -v '?'
// Output format: <name><whitespace><locale><whitespace># <sample_text>
// Example: "Allison (Enhanced)  en_US    # Hello! My name is Allison."

fn parse_say_line(line: &str) -> Option<SayVoice> {
    // Find " # " marker separating metadata from sample text
    let hash_pos = line.find(" # ")?;
    let (metadata, sample_with_marker) = line.split_at(hash_pos);
    let sample_text = sample_with_marker.trim_start_matches(" # ").to_string();

    // Locale is last whitespace-separated token before the hash
    let metadata = metadata.trim_end();
    let last_space = metadata.rfind(char::is_whitespace)?;
    let locale = metadata[last_space..].trim().to_string();
    let name = metadata[..last_space].trim().to_string();

    Some(SayVoice { name, locale, sample_text })
}
```

**Gender Resolution**: Use `gender_guesser` crate on voice name (e.g., "Allison" → Female, "Albert" → Male). Names like "Bad News" or "Bells" should default to `Gender::Unknown`.

**Quality Assignment**:

- Voices with "(Enhanced)" or "(Premium)" suffix → `VoiceQuality::Good`
- Eloquence voices → `VoiceQuality::Low` (filter out from `list_voices()`)
- All other voices → `VoiceQuality::Moderate`

**Caveats**:

- Voice names can contain spaces and parentheses (e.g., "Aman (English (India))")
- Output format is fixed-width but column positions vary by name length
- macOS updates may add/remove voices or change output format
- Installed voices via System Preferences may not always appear in CLI output

#### eSpeak

Cross-platform open-source TTS engine (eSpeak NG is the actively maintained fork).

**Detection & Readiness**:

- Check if `espeak-ng` or `espeak` executable is in PATH
- Fully offline, no network or API keys required
- Voice data files typically bundled with installation

**Voice Metadata Available**:

| Field | Source | Example |
|-------|--------|---------|
| Priority | CLI output | 5, 4, 3 (higher = preferred) |
| Language Code | CLI output | "en", "en-us", "af" |
| Age | CLI output | 20, or "--" (unset) |
| Gender | CLI output | "M" (Male), "F" (Female), "--" (unset) |
| Voice Name | CLI output | "English_(America)" → "English (America)" |
| File Path | CLI output | "gmw/af", "gmw/en-US" |
| Other Languages | CLI output | "(en-r 3)(en 5)" |

**Implementation Approach**:

```rust
use std::process::Command;

// Execute: espeak-ng --voices (or espeak --voices as fallback)
// Output format (fixed-width columns, skip header line):
// Pty Language       Age/Gender VoiceName          File                 Other Languages
//  5  en-us           --/M      English_(America)  gmw/en-US            (en-r 3)(en 5)

fn parse_espeak_line(line: &str) -> Option<EspeakVoice> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 { return None; }

    let priority = parts[0].parse::<u8>().ok()?;
    let language = parts[1].to_string();
    let (age, gender) = parse_age_gender(parts[2]); // e.g., "--/M" → (None, Male)
    let name = parts[3].replace('_', " "); // Underscores represent spaces
    let file = parts[4].to_string();

    // Remaining parts are other languages like "(en-r 3)(en 5)"
    let other_languages = if parts.len() > 5 {
        parse_other_languages(&parts[5..].join(" "))
    } else {
        Vec::new()
    };

    Some(EspeakVoice { priority, language, age, gender, name, file, other_languages })
}

fn parse_age_gender(field: &str) -> (Option<u8>, Gender) {
    let parts: Vec<&str> = field.split('/').collect();
    let age = parts.get(0).and_then(|s| s.parse().ok());
    let gender = match parts.get(1) {
        Some(&"M") => Gender::Male,
        Some(&"F") => Gender::Female,
        _ => Gender::Unknown,
    };
    (age, gender)
}
```

**Gender Resolution**: eSpeak provides explicit gender in the Age/Gender field ("M"/"F"/"--"). Map directly to our `Gender` enum.

**Quality Assignment**: Assign `VoiceQuality::Low` - eSpeak uses formant synthesis which sounds robotic compared to neural TTS. It's fast and works offline but not natural-sounding.

**Voice Variants**: eSpeak supports voice modifiers appended to the base voice:

- Male variants: `+m1`, `+m2`, `+m3`, `+m4`, `+m5`, `+m6`, `+m7`
- Female variants: `+f1`, `+f2`, `+f3`, `+f4`
- Effect variants: `+croak`, `+whisper`
- Usage: `espeak -v en+f3 "Hello world"`

**Caveats**:

- Try `espeak-ng` first (actively maintained), fall back to `espeak`
- Underscores in voice names represent spaces
- Other languages field uses parenthesized format with priority numbers
- Output format may vary slightly between versions




