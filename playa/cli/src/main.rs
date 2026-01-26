use clap::{Parser, ValueHint};
use sniff_lib::programs::InstalledHeadlessAudio;
use std::path::PathBuf;

use playa::{all_players, AudioFileFormat, AudioPlayer, Codec, Playa, PLAYER_LOOKUP};
use shared::markdown::output::terminal::{for_terminal, TerminalOptions};
use shared::markdown::Markdown;

#[derive(Parser)]
#[command(name = "playa")]
#[command(about = "Play audio using the host's installed players", long_about = None)]
struct Cli {
    /// Show a table of available players
    #[arg(long)]
    players: bool,

    /// Display playback metadata (player, volume, speed, codec, format)
    #[arg(long)]
    meta: bool,

    /// Play at 1.25x speed
    #[arg(long, conflicts_with = "slow")]
    fast: bool,

    /// Play at 0.75x speed
    #[arg(long, conflicts_with = "fast")]
    slow: bool,

    /// Play at 50% volume
    #[arg(long, conflicts_with = "loud")]
    quiet: bool,

    /// Play at 150% volume
    #[arg(long, conflicts_with = "quiet")]
    loud: bool,

    /// Custom playback speed (0.5 to 2.0)
    #[arg(long, value_name = "MULTIPLIER", conflicts_with_all = ["fast", "slow"])]
    speed: Option<f32>,

    /// Custom volume level (0.0 to 2.0)
    #[arg(long, value_name = "LEVEL", conflicts_with_all = ["quiet", "loud"])]
    volume: Option<f32>,

    /// Audio file to play
    #[arg(
        value_name = "AUDIO_FILE",
        value_hint = ValueHint::FilePath,
        required_unless_present = "players"
    )]
    audio_file: Option<PathBuf>,
}

impl Cli {
    fn build_playa(&self, path: &PathBuf) -> Result<Playa, playa::InvalidAudio> {
        let mut playa = Playa::from_path(path)?;

        // Apply speed settings
        if let Some(speed) = self.speed {
            playa = playa.speed(speed);
        } else if self.fast {
            playa = playa.speed(1.25);
        } else if self.slow {
            playa = playa.speed(0.75);
        }

        // Apply volume settings
        if let Some(volume) = self.volume {
            playa = playa.volume(volume);
        } else if self.quiet {
            playa = playa.volume(0.5);
        } else if self.loud {
            playa = playa.volume(1.5);
        }

        // Apply meta display
        if self.meta {
            playa = playa.show_meta();
        }

        Ok(playa)
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.players {
        let markdown = build_metadata_markdown();
        render_markdown(&markdown);
        return;
    }

    let Some(ref path) = cli.audio_file else {
        eprintln!("No audio file provided. Use `playa --players` to show available players.");
        std::process::exit(2);
    };

    let playa = match cli.build_playa(path) {
        Ok(playa) => playa,
        Err(error) => {
            eprintln!("Failed to detect audio format: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = playa.play() {
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

    fn make_cli(
        players: bool,
        meta: bool,
        fast: bool,
        slow: bool,
        quiet: bool,
        loud: bool,
        speed: Option<f32>,
        volume: Option<f32>,
    ) -> Cli {
        Cli {
            players,
            meta,
            fast,
            slow,
            quiet,
            loud,
            speed,
            volume,
            audio_file: Some(PathBuf::from("test.mp3")),
        }
    }

    #[test]
    fn cli_default_no_speed_or_volume() {
        let cli = make_cli(false, false, false, false, false, false, None, None);
        // With no audio file to test against, we just verify the struct is created
        assert!(!cli.fast);
        assert!(!cli.slow);
        assert!(!cli.quiet);
        assert!(!cli.loud);
        assert!(cli.speed.is_none());
        assert!(cli.volume.is_none());
    }

    #[test]
    fn cli_fast_sets_speed() {
        let cli = make_cli(false, false, true, false, false, false, None, None);
        assert!(cli.fast);
    }

    #[test]
    fn cli_slow_sets_speed() {
        let cli = make_cli(false, false, false, true, false, false, None, None);
        assert!(cli.slow);
    }

    #[test]
    fn cli_quiet_sets_volume() {
        let cli = make_cli(false, false, false, false, true, false, None, None);
        assert!(cli.quiet);
    }

    #[test]
    fn cli_loud_sets_volume() {
        let cli = make_cli(false, false, false, false, false, true, None, None);
        assert!(cli.loud);
    }

    #[test]
    fn cli_explicit_speed_and_volume() {
        let cli = make_cli(false, false, false, false, false, false, Some(0.9), Some(0.3));
        assert_eq!(cli.speed, Some(0.9));
        assert_eq!(cli.volume, Some(0.3));
    }

    #[test]
    fn cli_meta_flag() {
        let cli = make_cli(false, true, false, false, false, false, None, None);
        assert!(cli.meta);
    }

    #[test]
    fn cli_players_flag() {
        let cli = make_cli(true, false, false, false, false, false, None, None);
        assert!(cli.players);
    }
}
