use std::sync::LazyLock;

use sniff_lib::programs::InstalledTtsClients;
use url::Url;

/// the volume level which the TTS
/// audio will be spoken at.
pub enum VolumeLevel {
    Loud,
    Soft,
    Normal,
    Explicit(f32),
}

/// Language preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Language {
    /// English language (any variant: en-US, en-GB, etc.).
    #[default]
    English,
    /// Custom language code (BCP-47 format recommended, e.g., "fr-FR", "es-MX").
    Custom(String),
}

impl Language {
    /// Returns the language code prefix for voice matching.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_speaks::Language;
    ///
    /// assert_eq!(Language::English.code_prefix(), "en");
    /// assert_eq!(Language::Custom("fr-FR".into()).code_prefix(), "fr-FR");
    /// ```
    pub fn code_prefix(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Custom(code) => code,
        }
    }
}

/// Gender preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Gender {
    /// Prefer a male voice.
    Male,
    /// Prefer a female voice.
    Female,
    /// No gender preference (use any available voice).
    #[default]
    Any,
}

/// The TTS providers which may reside already on a host system.
///
/// These providers roughly fit into one of two categories:
///
/// 1. All-in-One - the CLI, model(s), and voices are packaged
///    together. If you have the binary CLI then you have all
///    you need to use this CLI.
/// 2.
pub enum HostTtsProvider {
    /// all macos clients support the `say` CLI although
    /// the voices available on each system will vary
    Say,
    /// high quality and flexible speech processing engine: [Website](https://echogarden.io/)
    EchoGarden,

    /// the `sherpa-onnx-offline-tts` CLI -- which is part of the
    /// [sherpa-ONNX]() project -- requires you point to a model
    /// file (`--vits-model <model>` or `--kokoro-model <model>`).
    ///
    /// #### Example Usage
    /// ```sh
    /// ./bin/sherpa-onnx-offline-tts \
    ///     --vits-model=/path/to/model.onnx \
    ///     --vits-tokens=/path/to/tokens.txt \
    ///     --vits-data-dir=/path/to/espeak-ng-data \
    ///     --output-filename=./test.wav \
    ///     "Your text goes here."
    /// ```
    ///
    /// For this host TTS solution to be usable in **biscuit-speaks**
    /// we expect the environment variables `SHERPA_MODEL` and `SHERPA_TOKENS`
    /// to be set and point to a valid model and tokens respectively
    Sherpa,

    /// Common on many Linux distros and should be easily
    /// added with the distro's package manager if not already present.
    ///
    /// Quality is ok but language support is massive.
    ESpeak,

    /// Should be available on all (or almost all) Windows installations.
    ///
    /// Quality varies based on OS version. Never amazing but modern Windows
    /// has decent quality voices (though not as good as macOS's `say`)
    SAPI,

    /// General multi-lingual speech synthesis system.
    ///
    /// [Website](http://www.cstr.ed.ac.uk/projects/festival/)
    Festival,
    Pico2Wave,

    /// Developed by Mycroft AI. It's a neural TTS that can run completely offline
    /// and supports SSML (Speech Synthesis Markup Language) for fine-tuning.
    ///
    /// [Website](https://github.com/MycroftAI/mycroft-mimic3-tts)
    Mimic3,

    /// Kokoro TTS models are very popular today because of their high quality but
    /// relatively undemanding hardware requirements. To help people use these
    /// models there are several CLI's which exist, the most feature rich of them
    /// is the [`kokoro-tts`](https://github.com/nazdridoy/kokoro-tts) CLI.
    ///
    /// You can install using **uv** with:
    ///
    /// ```sh
    /// uv tool install git+https://github.com/nazdridoy/kokoro-tts
    /// ```
    ///
    /// ### CLI Usage
    ///
    /// Usage:
    ///     - `kokoro-tts <input_text_file> [<output_audio_file>] [options]`
    ///     - `echo "hello world" kokoro-tts - --stream --voice af_sarah`
    /// Commands:
    ///     -h, --help         Show this help message
    ///     --help-languages   List all supported languages
    ///     --help-voices      List all available voices
    ///     --merge-chunks     Merge existing chunks in split-output directory into chapter files
    ///
    /// Options:
    ///     --stream            Stream audio instead of saving to file
    ///     --speed <float>     Set speech speed (default: 1.0)
    ///     --lang <str>        Set language (default: en-us)
    ///     --voice <str>       Set voice or blend voices (default: interactive selection)
    ///     --split-output <dir> Save each chunk as separate file in directory
    ///     --format <str>      Audio format: wav or mp3 (default: wav)
    ///     --debug             Show detailed debug information
    ///     --model <path>      Path to kokoro-v1.0.onnx model file (default: ./kokoro-v1.0.onnx)
    ///     --voices <path>     Path to voices-v1.0.bin file (default: ./voices-v1.0.bin)
    ///
    /// Input formats:
    ///     .txt               Text file input
    ///     .epub              EPUB book input (will process chapters)
    ///     .pdf               PDF document input (extracts chapters from TOC or content)
    ///
    /// Examples:
    ///     kokoro-tts input.txt output.wav --speed 1.2 --lang en-us --voice af_sarah
    ///     kokoro-tts input.epub --split-output ./chunks/ --format mp3
    ///     kokoro-tts input.pdf output.wav --speed 1.2 --lang en-us --voice af_sarah
    ///     kokoro-tts input.pdf --split-output ./chunks/ --format mp3
    ///     kokoro-tts input.txt --stream --speed 0.8
    ///     kokoro-tts input.txt output.wav --voice "af_sarah:60,am_adam:40"
    ///     kokoro-tts input.txt --stream --voice "am_adam,af_sarah" # 50-50 blend
    ///     kokoro-tts --merge-chunks --split-output ./chunks/ --format wav
    ///     kokoro-tts --help-voices
    ///     kokoro-tts --help-languages
    ///     kokoro-tts input.epub --split-output ./chunks/ --debug
    ///     kokoro-tts input.txt output.wav --model /path/to/model.onnx --voices /path/to/voices.bin
    ///     kokoro-tts input.txt --model ./models/kokoro-v1.0.onnx --voices ./models/voices-v1.0.bin
    ///
    /// ### Supported Voices
    ///
    /// - `en-us::female`: af_alloy, af_aoede, af_bella, af_heart, af_jessica, af_kore, af_nicole, af_nova, af_river, af_sarah, af_sky
    /// - `en-us::male`: am_adam, am_echo, am_eric, am_fenrir, am_liam, am_michael, am_onyx, am_puck
    /// - `en-gb`: 	bf_alice, bf_emma, bf_isabella, bf_lily, bm_daniel, bm_fable, bm_george, bm_lewis
    /// - `fr-fr`: ff_siwis
    /// - `it`: if_sara, im_nicola
    /// - `ja`: jf_alpha, jf_gongitsune, jf_nezumi, jf_tebukuro, jm_kumo
    /// - `cmn`: zf_xiaobei, zf_xiaoni, zf_xiaoxiao, zf_xiaoyi, zm_yunjian, zm_yunxi, zm_yunxia, zm_yunyang
    ///
    /// ### Required Model and Data files
    ///
    /// ```sh
    /// wget https://github.com/nazdridoy/kokoro-tts/releases/download/v1.0.0/kokoro-v1.0.onnx
    /// wget https://github.com/nazdridoy/kokoro-tts/releases/download/v1.0.0/voices-v1.0.bin
    /// ```
    ///
    /// - use the `--model <model>` and `--voices <voices>` CLI switches if the onnx model and voices
    /// binary are _not_ in the directory you're running the CLI from
    KokoroTts,

    /// Google Text-to-Speech CLI tool. [Website](https://github.com/pndurette/gTTS)
    Gtts,

    /// The CLI client for Speech Dispatcher. On many Linux desktops, this acts
    /// as a layer that routes text to whatever engine is currently active
    /// (eSpeak, Festival, etc.).
    SpdSay,
}

impl HostTtsProvider {
    /// given the current environment represented by `InstalledTtsClients`
    /// this function will indicate whether the CLI program has been found
    /// on the host system.
    pub fn is_available(self, avail: &InstalledTtsClients) -> bool {
        todo!()
    }
}

static LINUX_TTS_STACK: LazyLock<Vec<HostTtsProvider>> = LazyLock::new(|| {
    let stack = vec![
        HostTtsProvider::EchoGarden,
        HostTtsProvider::KokoroTts,
        HostTtsProvider::Sherpa,
        HostTtsProvider::ESpeak,
        HostTtsProvider::SpdSay
    ];

    stack
});

static MACOS_TTS_STACK: LazyLock<Vec<HostTtsProvider>> = LazyLock::new(|| {
    let stack = vec![
        HostTtsProvider::EchoGarden,
        HostTtsProvider::KokoroTts,
        HostTtsProvider::Sherpa,
        HostTtsProvider::Say,
        HostTtsProvider::ESpeak,
    ];

    stack
});

static WINDOWS_TTS_STACK: LazyLock<Vec<HostTtsProvider>> = LazyLock::new(|| {
    let stack = vec![
        HostTtsProvider::EchoGarden,
        HostTtsProvider::KokoroTts,
        HostTtsProvider::Sherpa,
        HostTtsProvider::SAPI,
        HostTtsProvider::ESpeak,
    ];

    stack
});

/// whether a given program is available on a specific
/// operating system.
pub enum OsAvailability {
    Always,
    Never,
    Sometimes,
}

pub struct InstallVariant {
    shell_cmd: String,
    requires_program: Option<String>,
    requires_powershell: bool,
}

/// information about a TTS CLI program
/// which might be found on the host system
pub struct TtsCli {
    /// name of the TTS CLI
    name: String,
    /// the main informational URL for the TTS CLI
    url: Url,

    /// available on Windows
    windows: OsAvailability,
    /// available on macOS
    mac_os: OsAvailability,
    /// available on Linux
    linux: OsAvailability,

    /// Supports the SSML standard
    ssml: bool,
}
