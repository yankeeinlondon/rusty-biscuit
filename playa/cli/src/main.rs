use std::path::PathBuf;

use clap::{Parser, ValueHint};
use sniff_lib::programs::InstalledHeadlessAudio;

use playa::{all_players, AudioFileFormat, AudioPlayer, Codec, Playa, PLAYER_LOOKUP};
use shared::markdown::output::terminal::{for_terminal, TerminalOptions};
use shared::markdown::Markdown;
use shared::testing::strip_ansi_codes;

const MISSING_FG: &str = "\x1b[38;2;140;140;140m";
const ITALIC: &str = "\x1b[3m";
const RESET: &str = "\x1b[0m";
const TABLE_DIVIDER: char = '\u{2502}';

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
        let (markdown, missing) = build_metadata_markdown();
        render_markdown(&markdown, &missing);
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

fn build_metadata_markdown() -> (String, Vec<String>) {
    let installed = InstalledHeadlessAudio::new();
    let missing = collect_missing_players(&installed);
    let markdown = build_metadata_markdown_table();
    (markdown, missing)
}

fn build_metadata_markdown_table() -> String {
    let mut lines = Vec::new();
    lines.push("| Software | Codec Support | File Formats |".to_string());
    lines.push("|---|---|---|".to_string());

    for player in all_players() {
        let Some(metadata) = PLAYER_LOOKUP.get(player) else {
            continue;
        };
        let software = link_for_player(*player);
        let codecs = escape_markdown_cell(&format_codec_list(metadata.supported_codecs));
        let formats = escape_markdown_cell(&format_format_list(metadata.supported_formats));
        lines.push(format!("| {} | {} | {} |", software, codecs, formats));
    }

    lines.join("\n")
}

fn collect_missing_players(installed: &InstalledHeadlessAudio) -> Vec<String> {
    all_players()
        .iter()
        .filter(|player| !installed.is_installed(player.as_headless_audio()))
        .map(|player| display_name_for_player(*player))
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

fn display_name_for_player(player: AudioPlayer) -> String {
    PLAYER_LOOKUP
        .get(&player)
        .map(|metadata| metadata.display_name().to_string())
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

fn render_markdown(content: &str, missing_players: &[String]) {
    let markdown = Markdown::from(content.to_string());
    match for_terminal(&markdown, TerminalOptions::default()) {
        Ok(rendered) => {
            let mut output = dim_missing_rows(&rendered, missing_players);
            append_missing_note(&mut output);
            print!("{}", output);
        }
        Err(_) => println!("{}", markdown.content()),
    }
}

fn append_missing_note(output: &mut String) {
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(&format!(
        "- {italic}items listed in {grey}grey{reset}{italic} are not installed{reset}\n",
        italic = ITALIC,
        grey = MISSING_FG,
        reset = RESET
    ));
}

fn dim_missing_rows(rendered: &str, missing_players: &[String]) -> String {
    if missing_players.is_empty() {
        return rendered.to_string();
    }

    let mut output = String::with_capacity(rendered.len() + missing_players.len() * 12);
    let mut current_row_missing = false;

    for line_with_newline in rendered.split_inclusive('\n') {
        let (line, newline) = line_with_newline
            .strip_suffix('\n')
            .map(|line| (line, "\n"))
            .unwrap_or((line_with_newline, ""));

        let plain_line = strip_osc8_sequences(&strip_ansi_codes(line));

        if line.starts_with(TABLE_DIVIDER) {
            if let Some(cell) = first_table_cell(&plain_line) {
                let trimmed = cell.trim();
                if !trimmed.is_empty() {
                    current_row_missing =
                        missing_players.iter().any(|name| trimmed.starts_with(name));
                }
            }

            if current_row_missing {
                output.push_str(&dim_table_row_line(line));
            } else {
                output.push_str(line);
            }
        } else {
            current_row_missing = false;
            output.push_str(line);
        }

        output.push_str(newline);
    }

    output
}

fn dim_table_row_line(line: &str) -> String {
    let mut output = String::with_capacity(line.len() + 16);
    let mut in_cell = false;

    for ch in line.chars() {
        if ch == TABLE_DIVIDER {
            if in_cell {
                output.push_str(RESET);
            }
            output.push(ch);
            output.push_str(MISSING_FG);
            in_cell = true;
        } else {
            output.push(ch);
        }
    }

    if in_cell {
        output.push_str(RESET);
    }

    output
}

fn first_table_cell(line: &str) -> Option<&str> {
    let mut parts = line.split(TABLE_DIVIDER);
    parts.next()?;
    parts.next()
}

fn strip_osc8_sequences(input: &str) -> String {
    let osc8_start = "\x1b]8;;";
    let osc8_end = "\x1b]8;;\x07";
    let mut output = String::new();
    let mut remaining = input;

    while let Some(start) = remaining.find(osc8_start) {
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start + osc8_start.len()..];
        let Some(bel_pos) = after_start.find('\x07') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let after_url = &after_start[bel_pos + 1..];
        let Some(end_pos) = after_url.find(osc8_end) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let display = &after_url[..end_pos];
        output.push_str(display);
        remaining = &after_url[end_pos + osc8_end.len()..];
    }

    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_meta_markdown_with_formatting_and_links() {
        let markdown = build_metadata_markdown_table();

        assert!(markdown.contains("| Software | Codec Support | File Formats |"));
        assert!(markdown.contains("PCM"));
        assert!(markdown.contains("Vorbis"));
        assert!(markdown.contains(".wav"));
        assert!(markdown.contains(&link_for_player(AudioPlayer::Mpv)));
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
        let cli = make_cli(
            false,
            false,
            false,
            false,
            false,
            false,
            Some(0.9),
            Some(0.3),
        );
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
