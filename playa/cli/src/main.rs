use clap::{Parser, ValueHint};
use sniff_lib::programs::InstalledHeadlessAudio;
use std::path::PathBuf;

use playa::{all_players, Audio, AudioFileFormat, AudioPlayer, Codec, PLAYER_LOOKUP};
use shared::markdown::output::terminal::{for_terminal, TerminalOptions};
use shared::markdown::Markdown;

#[derive(Parser)]
#[command(name = "playa")]
#[command(about = "Play audio using the host's installed players", long_about = None)]
struct Cli {
    /// Show a table of player metadata
    #[arg(long)]
    meta: bool,

    /// Audio file to play
    #[arg(
        value_name = "AUDIO_FILE",
        value_hint = ValueHint::FilePath,
        required_unless_present = "meta"
    )]
    audio_file: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    if cli.meta {
        let markdown = build_metadata_markdown();
        render_markdown(&markdown);
        return;
    }

    let Some(path) = cli.audio_file else {
        eprintln!("No audio file provided. Use `playa --meta` to show metadata.");
        std::process::exit(2);
    };

    let audio = match Audio::from_path(path.clone()) {
        Ok(audio) => audio,
        Err(error) => {
            eprintln!("Failed to detect audio format: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = audio.play() {
        eprintln!("Playback failed: {error}");
        std::process::exit(1);
    }
}

fn build_metadata_markdown() -> String {
    let installed = InstalledHeadlessAudio::new();
    let missing = collect_missing_players(&installed);
    build_metadata_markdown_with_missing(&missing)
}

fn build_metadata_markdown_with_missing(missing: &[String]) -> String {
    let mut lines = Vec::new();
    lines.push("| Software | Codec Support | File Formats |".to_string());
    lines.push("|---|---|---|".to_string());

    for player in all_players() {
        let Some(metadata) = PLAYER_LOOKUP.get(player) else {
            continue;
        };
        let website = metadata.website().trim();
        let software = if website.is_empty() {
            metadata.display_name().to_string()
        } else {
            format!("[{}]({})", metadata.display_name(), website)
        };
        let codecs = escape_markdown_cell(&format_codec_list(metadata.supported_codecs));
        let formats = escape_markdown_cell(&format_format_list(metadata.supported_formats));
        lines.push(format!("| {} | {} | {} |", software, codecs, formats));
    }

    lines.push(String::new());
    lines.push("Software not on this system:".to_string());
    lines.push(String::new());
    let missing_line = if missing.is_empty() {
        "none".to_string()
    } else {
        missing.join(", ")
    };
    lines.push(format!("- {}", missing_line));

    lines.join("\n")
}

fn collect_missing_players(installed: &InstalledHeadlessAudio) -> Vec<String> {
    all_players()
        .iter()
        .filter(|player| !installed.is_installed(player.as_headless_audio()))
        .map(|player| link_for_player(*player))
        .collect()
}

fn link_for_player(player: AudioPlayer) -> String {
    PLAYER_LOOKUP
        .get(&player)
        .map(|metadata| {
            let website = metadata.website().trim();
            if website.is_empty() {
                metadata.display_name().to_string()
            } else {
                format!("[{}]({})", metadata.display_name(), website)
            }
        })
        .unwrap_or_else(|| format!("{player:?}"))
}

fn format_codec_list(codecs: &[Codec]) -> String {
    if codecs.is_empty() {
        return "None".to_string();
    }
    codecs
        .iter()
        .map(|codec| format_codec_label(*codec))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_format_list(formats: &[AudioFileFormat]) -> String {
    if formats.is_empty() {
        return "None".to_string();
    }
    formats
        .iter()
        .map(|format| format_format_label(*format))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_codec_label(codec: Codec) -> String {
    match codec {
        Codec::Pcm => "PCM".to_string(),
        Codec::Flac => "FLAC".to_string(),
        Codec::Alac => "ALAC".to_string(),
        Codec::Mp3 => "MP3".to_string(),
        Codec::Aac => "AAC".to_string(),
        Codec::Vorbis => "Vorbis".to_string(),
        Codec::Opus => "Opus".to_string(),
    }
}

fn format_format_label(format: AudioFileFormat) -> String {
    match format {
        AudioFileFormat::Wav => ".wav".to_string(),
        AudioFileFormat::Aiff => ".aiff".to_string(),
        AudioFileFormat::Flac => ".flac".to_string(),
        AudioFileFormat::Mp3 => ".mp3".to_string(),
        AudioFileFormat::Ogg => ".ogg".to_string(),
        AudioFileFormat::M4a => ".m4a".to_string(),
        AudioFileFormat::Webm => ".webm".to_string(),
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn render_markdown(content: &str) {
    let markdown = Markdown::from(content.to_string());
    match for_terminal(&markdown, TerminalOptions::default()) {
        Ok(rendered) => print!("{}", rendered),
        Err(_) => println!("{}", markdown.content()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_meta_markdown_with_formatting_and_links() {
        let missing = vec![
            link_for_player(AudioPlayer::Mpv),
            link_for_player(AudioPlayer::FfPlay),
        ];

        let markdown = build_metadata_markdown_with_missing(&missing);

        assert!(markdown.contains("| Software | Codec Support | File Formats |"));
        assert!(markdown.contains("PCM"));
        assert!(markdown.contains("Vorbis"));
        assert!(markdown.contains(".wav"));
        assert!(markdown.contains("Software not on this system:\n\n- [mpv]("));
        assert!(markdown.contains(", [FFplay]("));
    }
}
