use std::collections::HashMap;
use std::sync::LazyLock;

pub use sniff_lib::programs::{Program, PROGRAM_LOOKUP};

pub enum AudioPlayer {
  Mpv,
  FfPlay,
  Vlc,
  MPlayer,
  GstreamerGstPlay,
  Sox,
  Mpg123,
  Ogg123,
  AlsaAplay,
  PulseaudioPaplay,
  Pipewire
}


/// **Player**
///
/// A struct which defines the extended metadata needed for this library
/// as well provide the appropriate `program` enum to lookup metadata from
/// the sniff library as well.
pub struct Player {
    /// the enum identifying this player
    pub id: AudioPlayer,
    /// the enum identifying this player as a program in the Sniff library
    pub program: Program,
    /// the codec's which this player supports
    pub supported_codecs: Vec<Codec>,
    /// the file format's this player supports
    pub file_formats: Vec<AudioFileFormat>,

    /// whether the player can take a stream input rather than a file
    pub takes_stream_input: bool,

    /// whether the player can optionally stream the output audio over the wire
    /// instead of just to the host's speaker.
    pub supplies_stream_output: bool,



}


pub static PLAYER_LOOKUP: LazyLock<HashMap<AudioPlayer, Player>> = LazyLock::new(|| {
    let mut m = HashMap::with_capacity(20);

    todo!();

    m
});


