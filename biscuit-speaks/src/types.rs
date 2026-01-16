use url::Url;



/// the volume level which the TTS
/// audio will be spoken at.
pub enum VolumeLevel {
    Loud,
    Soft,
    Normal,
    Explicit(f32)
}




pub enum Gender {
    Male,
    Female
}

/// The TTS providers which may reside already on a host system.
pub enum HostTtsProvider {
    /// all macos clients support the `say` CLI although
    /// the voices available on each system will vary
    Say,
    /// high quality and flexible
    EchoGarden,
    /// Common on many Linux distros and should be easily
    /// added with the distro's package manager if not already present.
    ///
    /// Quality is ok but language support is massive.
    ESpeak,

    ///
    Sherpa,

    /// Should be available on all (or almost all) Windows installations.
    ///
    /// Quality varies based on OS version. Never amazing but modern Windows
    /// has good quality voices (though not as good as macOS's `say`)
    SAPI,

    Festival,
    Pico2Wave,

    /// Developed by Mycroft AI. It's a neural TTS that can run completely offline
    /// and supports SSML (Speech Synthesis Markup Language) for fine-tuning.
    Mimic3,

    /// A newer, trending lightweight model (82M parameters) that produces studio-quality audio.
    /// It’s often used via Python or a dedicated CLI wrapper.
    Kokoro,

    ///
    Gtty,


    /// The CLI client for Speech Dispatcher. On many Linux desktops, this acts
    /// as a layer that routes text to whatever engine is currently active
    /// (eSpeak, Festival, etc.).
    SpdSay
}

/// whether a given program is available on a specific
/// operating system.
pub enum OsAvailability {
    Always,
    Never,
    Sometimes
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
