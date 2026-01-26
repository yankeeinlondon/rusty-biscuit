use std::fs::File;
use std::io::Read;
use std::path::Path;

use reqwest::header::RANGE;
use reqwest::Client;
use url::Url;

use crate::error::DetectionError;
use crate::types::{AudioFileFormat, AudioFormat, Codec};

const MIN_DETECTION_BYTES: usize = 12;
const URL_RANGE_BYTES: &str = "bytes=0-511";

/// Detect audio format from raw bytes (header-only detection).
pub fn detect_audio_format_from_bytes(data: &[u8]) -> Result<AudioFormat, DetectionError> {
    if data.len() < MIN_DETECTION_BYTES {
        return Err(DetectionError::InsufficientData {
            required: MIN_DETECTION_BYTES,
            actual: data.len(),
        });
    }

    let kind = infer::get(data).ok_or(DetectionError::UnknownFormat)?;
    let mime = kind.mime_type();
    if !mime.starts_with("audio/") {
        return Err(DetectionError::NotAudio {
            mime: mime.to_string(),
        });
    }

    let file_format = format_from_mime(mime).ok_or(DetectionError::UnknownFormat)?;
    Ok(AudioFormat::new(file_format, codec_from_format(file_format)))
}

/// Detect audio format from a file path.
pub fn detect_audio_format_from_path(path: &Path) -> Result<AudioFormat, DetectionError> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 512];
    let bytes_read = file.read(&mut buffer)?;
    let detection = detect_audio_format_from_bytes(&buffer[..bytes_read]);

    match detection {
        Ok(format) => Ok(format),
        Err(error) => match audio_format_from_extension(path) {
            Some(format) => Ok(format),
            None => Err(error),
        },
    }
}

/// Detect audio format from a URL using an HTTP range request.
pub async fn detect_audio_format_from_url(url: &str) -> Result<AudioFormat, DetectionError> {
    let parsed = Url::parse(url)?;
    let client = Client::new();
    let response = client
        .get(parsed.as_str())
        .header(RANGE, URL_RANGE_BYTES)
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;

    let detection = detect_audio_format_from_bytes(&bytes);
    match detection {
        Ok(format) => Ok(format),
        Err(error) => match audio_format_from_extension(Path::new(parsed.path())) {
            Some(format) => Ok(format),
            None => Err(error),
        },
    }
}

fn format_from_mime(mime: &str) -> Option<AudioFileFormat> {
    match mime {
        "audio/mpeg" => Some(AudioFileFormat::Mp3),
        "audio/flac" | "audio/x-flac" => Some(AudioFileFormat::Flac),
        "audio/ogg" => Some(AudioFileFormat::Ogg),
        "audio/wav" | "audio/x-wav" => Some(AudioFileFormat::Wav),
        "audio/x-aiff" | "audio/aiff" => Some(AudioFileFormat::Aiff),
        "audio/mp4" | "audio/m4a" => Some(AudioFileFormat::M4a),
        "audio/webm" => Some(AudioFileFormat::Webm),
        _ => None,
    }
}

fn codec_from_format(format: AudioFileFormat) -> Option<Codec> {
    match format {
        AudioFileFormat::Wav | AudioFileFormat::Aiff => Some(Codec::Pcm),
        AudioFileFormat::Flac => Some(Codec::Flac),
        AudioFileFormat::Mp3 => Some(Codec::Mp3),
        AudioFileFormat::Ogg | AudioFileFormat::M4a | AudioFileFormat::Webm => None,
    }
}

fn audio_format_from_extension(path: &Path) -> Option<AudioFormat> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    let file_format = match extension.as_str() {
        "wav" | "wave" => Some(AudioFileFormat::Wav),
        "aif" | "aiff" => Some(AudioFileFormat::Aiff),
        "flac" => Some(AudioFileFormat::Flac),
        "mp3" => Some(AudioFileFormat::Mp3),
        "ogg" | "oga" => Some(AudioFileFormat::Ogg),
        "m4a" | "mp4" => Some(AudioFileFormat::M4a),
        "webm" => Some(AudioFileFormat::Webm),
        _ => None,
    }?;

    Some(AudioFormat::new(file_format, codec_from_format(file_format)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wav_from_bytes() {
        let data = b"RIFF\x24\0\0\0WAVEfmt ";
        let format = detect_audio_format_from_bytes(data).expect("wav detection");
        assert_eq!(format.file_format, AudioFileFormat::Wav);
        assert_eq!(format.codec, Some(Codec::Pcm));
    }

    #[test]
    fn detects_flac_from_bytes() {
        let data = b"fLaC\0\0\0\x22\0\0\0\0";
        let format = detect_audio_format_from_bytes(data).expect("flac detection");
        assert_eq!(format.file_format, AudioFileFormat::Flac);
        assert_eq!(format.codec, Some(Codec::Flac));
    }

    #[test]
    fn detects_ogg_from_bytes() {
        let data = b"OggS\0\x02\0\0\0\0\0\0\0\0";
        let format = detect_audio_format_from_bytes(data).expect("ogg detection");
        assert_eq!(format.file_format, AudioFileFormat::Ogg);
        assert_eq!(format.codec, None);
    }

    #[test]
    fn detects_mp3_from_bytes() {
        let data = b"ID3\x04\0\0\0\0\0\x10\0\0";
        let format = detect_audio_format_from_bytes(data).expect("mp3 detection");
        assert_eq!(format.file_format, AudioFileFormat::Mp3);
        assert_eq!(format.codec, Some(Codec::Mp3));
    }

    #[test]
    fn falls_back_to_extension() {
        let format = audio_format_from_extension(Path::new("track.m4a"))
            .expect("extension fallback");
        assert_eq!(format.file_format, AudioFileFormat::M4a);
        assert_eq!(format.codec, None);
    }
}
