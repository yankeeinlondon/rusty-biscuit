use std::fs::File;
use std::io::Cursor;
use std::time::Duration;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

use crate::audio::AudioData;
use crate::report::ProbedAudioMetadata;

/// Probe duration and channel count without decoding or fetching remote audio.
pub fn probe_audio_metadata(audio: &AudioData) -> Option<ProbedAudioMetadata> {
    let (source, extension): (Box<dyn MediaSource>, Option<&str>) = match audio {
        AudioData::FilePath(path) => (
            Box::new(File::open(path).ok()?),
            path.extension().and_then(|value| value.to_str()),
        ),
        AudioData::Bytes(bytes) => (Box::new(Cursor::new(bytes.as_ref().clone())), None),
        AudioData::Url(_) => return None,
    };

    let mut hint = Hint::new();
    if let Some(extension) = extension {
        hint.with_extension(extension);
    }
    let stream = MediaSourceStream::new(source, Default::default());
    let probed = get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let params = &probed.format.default_track()?.codec_params;
    let sample_rate = params.sample_rate?;
    let frames = params.n_frames?;
    let channels = u16::try_from(params.channels?.count()).ok()?;
    let duration = Duration::from_secs_f64(frames as f64 / f64::from(sample_rate));
    Some(ProbedAudioMetadata { duration, channels })
}
