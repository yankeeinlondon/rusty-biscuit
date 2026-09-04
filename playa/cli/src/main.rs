use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

mod install_ui;

use clap::builder::PossibleValue;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use sniff::programs::{InstallInterviewOutcome, InstalledHeadlessAudio};
use strum::IntoEnumIterator;

#[cfg(feature = "sfx-native")]
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use playa::{AudioFileFormat, AudioPlayer, Codec, PLAYER_LOOKUP, Playa, SoundEffect, all_players};
#[cfg(feature = "sfx-native")]
use sniff::hardware::{AudioDeviceInfo, AudioDeviceKind, AudioDirection};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::TerminalOptions;
use darkmatter::testing::strip_ansi_codes;
#[cfg(feature = "audio-ducking")]
use playa::ducking::{DuckConfig, DuckGuard, backend_name, create_backend};

const MISSING_FG: &str = "\x1b[38;2;140;140;140m";
const RESET: &str = "\x1b[0m";
const TABLE_DIVIDER: char = '\u{2502}';

/// Print a styled error to stderr and exit.
fn error_exit(message: &str, code: i32) -> ! {
    let styled = format!("<b>Playa(<red-500>error</red-500>):</b> {message}");
    let rendered = Prose::new(styled).render(&Terminal::default());
    eprintln!("{rendered}");
    std::process::exit(code)
}

const AFTER_HELP: &str = "\
Shell Completions:
  Enable completions by adding one of the following to your shell config:

  # Bash (~/.bashrc)
  source <(COMPLETE=bash playa)

  # Zsh (~/.zshrc)
  source <(COMPLETE=zsh playa)

  # Fish (~/.config/fish/config.fish)
  COMPLETE=fish playa | source";

/// Play audio using the host's installed players
#[derive(Parser)]
#[command(name = "playa")]
#[command(about = "Play audio using the host's installed players", long_about = None)]
#[command(after_help = AFTER_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Audio file to play (shorthand for `playa play <file>`)
    #[arg(value_name = "AUDIO_FILE", value_parser = AudioFileParser)]
    audio_file: Option<String>,

    #[command(flatten)]
    playback: PlaybackOptions,
}

#[derive(Subcommand)]
enum Command {
    /// Play an audio file
    Play {
        /// Audio file to play
        #[arg(value_name = "AUDIO_FILE", value_parser = AudioFileParser)]
        audio_file: String,

        #[command(flatten)]
        playback: PlaybackOptions,
    },

    /// Play a built-in sound effect
    Effect {
        /// Name of the sound effect to play
        #[arg(value_name = "NAME", value_parser = EffectNameParser)]
        name: String,

        #[command(flatten)]
        playback: PlaybackOptions,
    },

    /// List available built-in sound effects
    ListEffects {
        /// Filter effects by name, description, or category (case-insensitive)
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },

    /// Show a table of available audio players, or install missing ones
    #[command(subcommand)]
    Players(PlayersCommand),

    /// Show available native output channels (audio devices)
    #[cfg(feature = "sfx-native")]
    OutputChannels,

    /// Show audio ducking backend info
    #[cfg(feature = "audio-ducking")]
    DuckInfo,

    /// Show pending detached audio and the bounded playback journal
    Spool,
}

#[derive(Subcommand)]
enum PlayersCommand {
    /// Show a table of available audio players (default)
    List,

    /// Interactively install missing headless audio players
    Install,
}

/// Value parser that provides sound effect names for shell completion
/// while accepting any string (preserving fuzzy matching in the handler).
#[derive(Clone)]
struct EffectNameParser;

impl clap::builder::TypedValueParser for EffectNameParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .map(String::from)
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        let values = SoundEffect::all()
            .into_iter()
            .map(|e| PossibleValue::new(e.name()).help(e.description()));
        Some(Box::new(values))
    }
}

const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "wave", "aif", "aiff", "flac", "mp3", "ogg", "oga", "m4a", "mp4", "webm",
];

/// Value parser that suggests audio files and directories for shell completion.
#[derive(Clone)]
struct AudioFileParser;

impl clap::builder::TypedValueParser for AudioFileParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .map(String::from)
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        std::env::var_os("COMPLETE")?;

        let last_arg = std::env::args().next_back().unwrap_or_default();
        let mut dir_path = PathBuf::from(".");
        let mut prefix = String::new();

        if let Some(pos) = last_arg.rfind(std::path::MAIN_SEPARATOR) {
            let (dir, _) = last_arg.split_at(pos + 1);
            dir_path = PathBuf::from(dir);
            prefix = dir.to_string();
        }

        let mut values = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let full_name = format!("{}{}", prefix, name);

                if path.is_dir() {
                    let name = format!("{}{}", full_name, std::path::MAIN_SEPARATOR);
                    let leaked: &'static str = Box::leak(name.into_boxed_str());
                    values.push(PossibleValue::new(leaked));
                } else if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                        let leaked: &'static str = Box::leak(full_name.into_boxed_str());
                        values.push(PossibleValue::new(leaked));
                    }
                }
            }
        }

        Some(Box::new(values.into_iter()))
    }
}

/// Value parser that suggests common volume levels for shell completion
/// while accepting any valid f32 in range.
#[derive(Clone)]
struct VolumeParser;

impl clap::builder::TypedValueParser for VolumeParser {
    type Value = f32;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value
            .to_str()
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))?;
        s.parse::<f32>()
            .map_err(|_| clap::Error::new(clap::error::ErrorKind::InvalidValue))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        let values = [
            ("0.25", "25% volume"),
            ("0.5", "50% volume"),
            ("0.75", "75% volume"),
            ("1", "100% volume"),
            ("1.25", "125% volume"),
            ("1.5", "150% volume"),
            ("2", "200% volume"),
        ]
        .into_iter()
        .map(|(val, help)| PossibleValue::new(val).help(help));
        Some(Box::new(values))
    }
}

/// Value parser that suggests available native output channels for shell completion.
#[derive(Clone)]
struct ChannelParser;

impl clap::builder::TypedValueParser for ChannelParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        value
            .to_str()
            .map(String::from)
            .ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        #[cfg(feature = "sfx-native")]
        {
            // Avoid probing the audio subsystem during normal `--help` rendering.
            // `clap_complete`'s `CompleteEnv` consumes `COMPLETE` before clap
            // parses, so we use `_CLAP_COMPLETE_INDEX` (which it forwards) to
            // detect "shell completion in progress".
            std::env::var_os("_CLAP_COMPLETE_INDEX")?;
            if let Ok(channels) = playa::get_output_channels() {
                let values = channels.into_iter().map(|c| {
                    let mut help = c.name;
                    if c.is_default_audio {
                        help.push_str(" (default audio)");
                    }
                    if c.is_default_sfx {
                        help.push_str(" (default sfx)");
                    }
                    let static_name: &'static str = Box::leak(c.id.into_boxed_str());
                    let static_help: &'static str = Box::leak(help.into_boxed_str());
                    PossibleValue::new(static_name).help(static_help)
                });
                return Some(Box::new(values));
            }
        }
        None
    }
}

/// Playback options shared between play and effect commands
#[derive(Parser, Clone)]
struct PlaybackOptions {
    /// Display playback metadata (player, volume, speed, codec, format)
    #[arg(long)]
    meta: bool,

    /// Play in the background and return control to the terminal immediately
    #[arg(long)]
    background: bool,

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
    #[arg(long, value_name = "LEVEL", conflicts_with_all = ["quiet", "loud"], value_parser = VolumeParser)]
    volume: Option<f32>,

    /// Specific output channel to use for playback, by name
    #[arg(long, value_name = "CHANNEL", value_parser = ChannelParser)]
    channel: Option<String>,

    /// Force host player playback (skip native decoder)
    #[arg(long)]
    force_host: bool,

    /// Disable audio ducking (attenuating other audio during playback)
    #[cfg(feature = "audio-ducking")]
    #[arg(long)]
    no_duck: bool,

    /// Ducking ramp duration in milliseconds (default: 1000)
    #[cfg(feature = "audio-ducking")]
    #[arg(long, value_name = "MS", default_value = "1000")]
    duck_ramp_ms: u32,

    /// Ducking floor level (0.0 = silent, 1.0 = no ducking, default: 0.2)
    #[cfg(feature = "audio-ducking")]
    #[arg(long, value_name = "LEVEL", default_value = "0.2")]
    duck_floor: f32,
}

impl PlaybackOptions {
    /// Convert CLI playback options to the library's `PlaybackOptions` type.
    fn to_lib_options(&self) -> playa::PlaybackOptions {
        let mut opts = playa::PlaybackOptions::new();

        if let Some(speed) = self.speed {
            opts = opts.with_speed(speed);
        } else if self.fast {
            opts = opts.with_speed(1.25);
        } else if self.slow {
            opts = opts.with_speed(0.75);
        }

        if let Some(volume) = self.volume {
            opts = opts.with_volume(volume);
        } else if self.quiet {
            opts = opts.with_volume(0.5);
        } else if self.loud {
            opts = opts.with_volume(1.5);
        }

        if let Some(channel) = &self.channel {
            opts = opts.with_channel(channel.clone());
        }

        opts
    }

    fn apply_to_playa(&self, mut playa: Playa) -> Playa {
        playa = playa.with_options(self.to_lib_options());

        if self.meta {
            playa = playa.show_meta();
        }

        if self.force_host {
            playa = playa.force_host();
        }

        #[cfg(feature = "audio-ducking")]
        if !self.no_duck {
            if let Ok(config) = DuckConfig::new(self.duck_ramp_ms, self.duck_floor) {
                playa = playa.with_ducked_audio(config);
            }
        }

        playa
    }

    fn to_detached_playback(&self) -> Result<playa::detached::DetachedPlayback, String> {
        #[cfg(feature = "audio-ducking")]
        let ducking = if self.no_duck {
            None
        } else {
            let config = DuckConfig::new(self.duck_ramp_ms, self.duck_floor)
                .map_err(|error| error.to_string())?;
            Some(playa::detached::DetachedDucking {
                ramp_ms: config.ramp_ms(),
                floor_scalar: config.floor_scalar(),
            })
        };
        #[cfg(not(feature = "audio-ducking"))]
        let ducking = None;

        Ok(playa::detached::DetachedPlayback {
            options: self.to_lib_options(),
            routing: if self.force_host {
                playa::detached::PlaybackRouting::ForceHost
            } else {
                playa::detached::PlaybackRouting::Auto
            },
            ducking,
        })
    }

    #[cfg(feature = "audio-ducking")]
    fn has_ducking(&self) -> bool {
        !self.no_duck
    }
}

fn background_requested(cli: &Cli) -> bool {
    cli.playback.background
        || match &cli.command {
            Some(Command::Play { playback, .. }) => playback.background,
            Some(Command::Effect { playback, .. }) => playback.background,
            _ => false,
        }
}

fn has_playback_target(cli: &Cli) -> bool {
    match &cli.command {
        Some(Command::Play { .. }) | Some(Command::Effect { .. }) => true,
        None => cli.audio_file.is_some(),
        _ => false,
    }
}

fn maybe_enqueue_background(cli: &Cli) -> bool {
    if !background_requested(cli) {
        return false;
    }

    if !has_playback_target(cli) {
        error_exit(
            "--background can only be used when playing an audio file or sound effect",
            2,
        );
    }

    if let Err(error) = enqueue_background_target(cli) {
        error_exit(&format!("failed to enqueue background playback: {error}"), 1);
    }

    true
}

fn enqueue_background_target(cli: &Cli) -> Result<playa::detached::JobId, String> {
    let (target, options) = match &cli.command {
        Some(Command::Play {
            audio_file,
            playback,
        }) => (BackgroundTarget::File(audio_file.as_str()), playback),
        Some(Command::Effect { name, playback }) => {
            (BackgroundTarget::Effect(name.as_str()), playback)
        }
        None => (
            BackgroundTarget::File(
                cli.audio_file
                    .as_deref()
                    .ok_or_else(|| "missing audio file".to_string())?,
            ),
            &cli.playback,
        ),
        _ => return Err("background playback requires an audio target".to_string()),
    };

    let detached = options.to_detached_playback()?;
    match target {
        BackgroundTarget::File(reference) => {
            let path = resolve_audio_reference(reference)?;
            playa::detached::enqueue(playa::detached::SpoolJob::PlayFile {
                path: playa::detached::OsValue::from(path.into_os_string()),
                playback: detached,
                delete_after: false,
            })
            .map_err(|error| error.to_string())
        }
        BackgroundTarget::Effect(name) => {
            let effect = SoundEffect::from_name(name).ok_or_else(|| {
                format!(
                    "unknown sound effect: {name}. Use `playa list-effects` to see available effects"
                )
            })?;
            playa::detached::enqueue_bytes(effect.bytes(), detached)
                .map_err(|error| error.to_string())
        }
    }
}

enum BackgroundTarget<'a> {
    File(&'a str),
    Effect(&'a str),
}

fn resolve_audio_reference(reference: &str) -> Result<PathBuf, String> {
    biscuit_file::FileReference::new(reference)
        .map_err(|error| error.to_string())?
        .resolve()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("audio file reference did not resolve: {reference}"))
}

fn show_spool() {
    let snapshot = match playa::detached::snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => error_exit(&format!("failed to inspect detached spool: {error}"), 1),
    };
    let terminal = Terminal::default();
    let root_name = snapshot
        .root
        .file_name()
        .map(Path::new)
        .and_then(biscuit_file::try_portable_string)
        .unwrap_or_else(|| "<private spool>".to_string());
    print!(
        "{}",
        Prose::new(format!("<b>Detached audio spool</b> <dim>{root_name}</dim>"))
            .display(&terminal)
    );

    let mut table = Table::new().with_columns(vec![
        TableColumn::new("Sequence"),
        TableColumn::new("Job"),
        TableColumn::new("State"),
        TableColumn::new("Source"),
        TableColumn::new("Outcome"),
    ]);
    let has_rows = !snapshot.pending.is_empty() || !snapshot.journal.is_empty();
    for pending in snapshot.pending {
        table.add_row(vec![
            pending.sequence.to_string().into(),
            pending.job_id.to_string().into(),
            pending.state.into(),
            format!("{:?}", pending.source_kind).to_lowercase().into(),
            "pending".into(),
        ]);
    }
    for entry in snapshot.journal {
        table.add_row(vec![
            entry.sequence.to_string().into(),
            entry.job_id.to_string().into(),
            "finished".into(),
            format!("{:?}", entry.source_kind).to_lowercase().into(),
            format!("{:?}", entry.outcome).into(),
        ]);
    }
    if !has_rows {
        print!("{}", Prose::new("<dim>No detached audio jobs.</dim>").display(&terminal));
    } else {
        print!("{}", table.display(&terminal));
    }
}

#[cfg(feature = "audio-ducking")]
#[tokio::main]
async fn main() {
    if let Some(code) = playa::detached::run_if_worker() {
        std::process::exit(code);
    }
    run_cli().await;
}

#[cfg(not(feature = "audio-ducking"))]
fn main() {
    if let Some(code) = playa::detached::run_if_worker() {
        std::process::exit(code);
    }
    run_cli_sync();
}

#[cfg(feature = "audio-ducking")]
async fn run_cli() {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    if maybe_enqueue_background(&cli) {
        return;
    }

    match cli.command {
        Some(Command::ListEffects { filter }) => {
            list_sound_effects(filter.as_deref());
        }
        Some(Command::Players(players_cmd)) => match players_cmd {
            PlayersCommand::List => {
                let (markdown, missing) = build_metadata_markdown();
                render_markdown(&markdown, &missing);
            }
            PlayersCommand::Install => {
                install_players();
            }
        },
        #[cfg(feature = "sfx-native")]
        Some(Command::OutputChannels) => {
            list_output_channels();
        }
        Some(Command::DuckInfo) => {
            print_duck_info().await;
        }
        Some(Command::Spool) => show_spool(),
        Some(Command::Effect { name, playback }) => {
            play_effect(&name, &playback).await;
        }
        Some(Command::Play {
            audio_file,
            playback,
        }) => {
            play_file(Path::new(&audio_file), &playback).await;
        }
        None => {
            // Default: play the audio file if provided
            if let Some(ref audio_file) = cli.audio_file {
                play_file(Path::new(audio_file), &cli.playback).await;
            } else {
                // No subcommand and no file - show help
                let _ = Cli::command().print_help();
                println!();
            }
        }
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn run_cli_sync() {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    if maybe_enqueue_background(&cli) {
        return;
    }

    match cli.command {
        Some(Command::ListEffects { filter }) => {
            list_sound_effects(filter.as_deref());
        }
        Some(Command::Players(players_cmd)) => match players_cmd {
            PlayersCommand::List => {
                let (markdown, missing) = build_metadata_markdown();
                render_markdown(&markdown, &missing);
            }
            PlayersCommand::Install => {
                install_players();
            }
        },
        #[cfg(feature = "sfx-native")]
        Some(Command::OutputChannels) => {
            list_output_channels();
        }
        Some(Command::Spool) => show_spool(),
        Some(Command::Effect { name, playback }) => {
            play_effect_sync(&name, &playback);
        }
        Some(Command::Play {
            audio_file,
            playback,
        }) => {
            play_file_sync(Path::new(&audio_file), &playback);
        }
        None => {
            // Default: play the audio file if provided
            if let Some(ref audio_file) = cli.audio_file {
                play_file_sync(Path::new(audio_file), &cli.playback);
            } else {
                // No subcommand and no file - show help
                let _ = Cli::command().print_help();
                println!();
            }
        }
    }
}

#[cfg(feature = "audio-ducking")]
async fn play_file(path: &Path, opts: &PlaybackOptions) {
    let playa = match Playa::from_path(path) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to detect audio format: {error}"), 1);
        }
    };

    if opts.has_ducking() {
        if let Err(error) = playa.play_async().await {
            error_exit(&format!("playback failed: {error}"), 1);
        }
    } else if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

#[cfg(feature = "audio-ducking")]
async fn play_effect(name: &str, opts: &PlaybackOptions) {
    let Some(effect) = SoundEffect::from_name(name) else {
        error_exit(
            &format!(
                "unknown sound effect: {name}. Use `playa list-effects` to see available effects"
            ),
            2,
        );
    };

    // Use native SFX playback when available (with OS audio channel routing).
    // Ducking is set up before SFX playback so both paths benefit.
    #[cfg(feature = "sfx-native")]
    if !opts.force_host {
        #[cfg(feature = "audio-ducking")]
        let guard = if opts.has_ducking() {
            let backend = create_backend();
            match DuckConfig::new(opts.duck_ramp_ms, opts.duck_floor) {
                Ok(config) => DuckGuard::new(backend, config).await.ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        if playa::sfx_player::play_sfx(effect.bytes(), &opts.to_lib_options()).is_ok() {
            #[cfg(feature = "audio-ducking")]
            if let Some(guard) = guard {
                guard.restore().await;
            }
            return;
        }
        // Fall through to Playa builder path on error
    }

    let playa = match Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to load sound effect: {error}"), 1);
        }
    };

    if opts.has_ducking() {
        if let Err(error) = playa.play_async().await {
            error_exit(&format!("playback failed: {error}"), 1);
        }
    } else if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn play_file_sync(path: &Path, opts: &PlaybackOptions) {
    let playa = match Playa::from_path(path) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to detect audio format: {error}"), 1);
        }
    };

    if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

#[cfg(not(feature = "audio-ducking"))]
fn play_effect_sync(name: &str, opts: &PlaybackOptions) {
    let Some(effect) = SoundEffect::from_name(name) else {
        error_exit(
            &format!(
                "unknown sound effect: {name}. Use `playa list-effects` to see available effects"
            ),
            2,
        );
    };

    // Use native SFX playback when available.
    #[cfg(feature = "sfx-native")]
    if !opts.force_host
        && playa::sfx_player::play_sfx(effect.bytes(), &opts.to_lib_options()).is_ok()
    {
        return;
    }

    let playa = match Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(p) => opts.apply_to_playa(p),
        Err(error) => {
            error_exit(&format!("failed to load sound effect: {error}"), 1);
        }
    };

    if let Err(error) = playa.play() {
        error_exit(&format!("playback failed: {error}"), 1);
    }
}

fn list_sound_effects(filter: Option<&str>) {
    let effects = SoundEffect::all();
    if effects.is_empty() {
        error_exit(
            "no sound effects are enabled in this build. Rebuild with `cargo build -p playa-cli --features sound-effects`",
            1,
        );
    }

    // Apply fuzzy filter: case-insensitive, strip non-alphanumeric, match against
    // name, description, and category
    let effects: Vec<SoundEffect> = match filter {
        Some(query) => {
            let normalized_query = fuzzy_normalize(query);
            effects
                .into_iter()
                .filter(|effect| {
                    fuzzy_normalize(effect.name()).contains(&normalized_query)
                        || fuzzy_normalize(effect.description()).contains(&normalized_query)
                        || fuzzy_normalize(effect.category()).contains(&normalized_query)
                })
                .collect()
        }
        None => effects,
    };

    if effects.is_empty() {
        error_exit(
            &format!("no effects match filter {:?}", filter.unwrap_or("")),
            1,
        );
    }

    // Group effects by category
    let mut categories: BTreeMap<&str, Vec<SoundEffect>> = BTreeMap::new();
    for effect in &effects {
        categories
            .entry(effect.category())
            .or_default()
            .push(*effect);
    }

    // Build the nested list structure
    let mut top_list = UnorderedList::empty();

    for (category, cat_effects) in categories {
        // Category header as a Prose with blue bold styling
        let header = Prose::new(format!("<blue><bold>{}</bold></blue>", category));
        top_list.add(header);

        // Build the inner list of effects
        let effect_items: Vec<RenderableTerminalContent> = cat_effects
            .iter()
            .map(|effect| {
                let styled = Prose::new(format!(
                    "{} <dim><italic>{}</italic></dim> [<dim>{}</dim>]",
                    effect.name(),
                    effect.description(),
                    format_duration(effect.duration_ms())
                ));
                RenderableTerminalContent::Component(Rc::new(styled))
            })
            .collect();

        let inner_list = UnorderedList::from(effect_items);
        top_list.add(inner_list);
    }

    // Render and print
    let output = top_list.render_optimistic(None);
    println!("{}", output);
}

/// Normalize a string for fuzzy matching: lowercase, strip non-alphanumeric.
fn fuzzy_normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn format_duration(duration_ms: Option<u32>) -> String {
    match duration_ms {
        Some(ms) if ms < 1000 => format!("{}ms", ms),
        Some(ms) => format!("<orange>{:.2}s</orange>", ms as f64 / 1000.0),
        None => "unknown".to_string(),
    }
}

#[cfg(feature = "audio-ducking")]
async fn print_duck_info() {
    let backend = create_backend();
    let name = backend_name();

    println!("Audio Ducking Backend Info");
    println!("==========================");
    println!("Selected backend: {}", name);
    println!(
        "Available: {}",
        if backend.is_available() { "yes" } else { "no" }
    );
    println!();

    match name {
        "macos-coreaudio" => {
            println!("Strategy: Volume control via CoreAudio");
            println!("  - Fades system audio volume down during playback");
            println!("  - Restores original volume after playback");
            println!("  - Works with devices that expose software volume control");
        }
        "macos-media-keys" => {
            println!("Strategy: Media key pause/resume (fallback)");
            println!("  - Detects if media is playing before pausing");
            println!("  - Only resumes if media was playing before ducking");
            println!("  - Used because your output device doesn't support software volume");
            println!();

            // Portable PATH lookup via sniff's executable index, so this works
            // the same way regardless of the host's shell or `which` binary.
            let has_nowplaying = sniff::executable_index::ExecutableIndex::build_path_only()
                .find("nowplaying-cli")
                .is_some();

            if has_nowplaying {
                println!("Detection: Using nowplaying-cli (all apps supported)");
            } else {
                println!("Detection: AppleScript fallback (Spotify, Music, TIDAL, VLC, Podcasts)");
                println!("  Tip: Install `nowplaying-cli` for browser/universal detection:");
                println!("       brew install nowplaying-cli");
            }
            println!();

            // Show current playback state
            let snapshot = backend.snapshot().await;
            match snapshot {
                Ok(snap) => {
                    let is_playing = snap
                        .entries
                        .first()
                        .and_then(|e| e.channels.first())
                        .map(|v| *v > 0.5)
                        .unwrap_or(false);
                    println!(
                        "Current state: {}",
                        if is_playing {
                            "Media is PLAYING"
                        } else {
                            "No media playing (or paused)"
                        }
                    );
                }
                Err(e) => {
                    println!("Could not detect playback state: {}", e);
                }
            }
        }
        "linux-pulse" => {
            println!("Strategy: Per-application volume control via PulseAudio/PipeWire");
            println!("  - Ducks individual applications (sink inputs)");
            println!("  - Excludes Playa's own audio from ducking");
            println!("  - Works with PipeWire's PulseAudio compatibility layer");
            println!();

            // Show current applications
            let snapshot = backend.snapshot().await;
            match snapshot {
                Ok(snap) => {
                    if snap.is_empty() {
                        println!("Current state: No other applications playing audio");
                    } else {
                        println!("Applications that would be ducked ({}):", snap.len());
                        for entry in &snap.entries {
                            if let playa::ducking::SessionId::PulseSinkInput { index, name } =
                                &entry.id
                            {
                                let vol = entry.channels.first().copied().unwrap_or(0.0) * 100.0;
                                println!("  [{}] {} - {:.0}%", index, name, vol);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Could not list applications: {}", e);
                }
            }
        }
        "windows-wasapi" => {
            println!("Strategy: Per-application volume control via WASAPI");
            println!("  - Ducks individual audio sessions on the default render endpoint");
            println!("  - Excludes Playa's own audio from ducking");
            println!("  - Only ducks sessions present when playback starts");
            println!();

            // Show current sessions
            let snapshot = backend.snapshot().await;
            match snapshot {
                Ok(snap) => {
                    if snap.is_empty() {
                        println!("Current state: No other applications playing audio");
                    } else {
                        println!("Sessions that would be ducked ({}):", snap.len());
                        for entry in &snap.entries {
                            if let playa::ducking::SessionId::WasapiSession { pid, key } = &entry.id
                            {
                                let vol = entry.channels.first().copied().unwrap_or(0.0) * 100.0;
                                println!("  PID {} [{}] - {:.0}%", pid, key, vol);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Could not list sessions: {}", e);
                }
            }
        }
        "noop" => {
            println!("Strategy: No ducking (disabled or unavailable)");
            println!("  - Audio playback will not affect other audio sources");
            if cfg!(target_os = "linux") {
                println!("  - On Linux this typically means PulseAudio/PipeWire is unavailable");
            }
        }
        _ => {
            println!("Strategy: {}", name);
        }
    }
}

fn install_players() {
    use sniff::programs::{
        HeadlessAudio, HostCapabilities, InstallInterviewInput, InstallInterviewOptions,
        ProgramDetector, ProgramMetadata, build_install_plan, run_install_interview,
    };

    let detector = InstalledHeadlessAudio::new();
    let all_players_list = HeadlessAudio::iter().collect::<Vec<_>>();

    let installed: Vec<_> = all_players_list
        .iter()
        .filter(|p| detector.is_installed(**p))
        .collect();

    if !installed.is_empty() {
        let names: Vec<_> = installed.iter().map(|p| p.display_name()).collect();
        println!("\x1b[2mAlready installed: {}\x1b[2m\n", names.join(", "));
    }

    let not_installed: Vec<_> = all_players_list
        .iter()
        .filter(|p| !detector.is_installed(**p))
        .collect();

    if not_installed.is_empty() {
        let styled = Prose::new("All supported audio players are already installed.");
        println!("{}", styled.render(&Terminal::default()));
        return;
    }

    let installable: Vec<(&HeadlessAudio, String)> = not_installed
        .iter()
        .filter_map(|p| {
            if !detector.installable(**p) {
                return None;
            }
            let binary = p.binary_name();
            let display = p.display_name();
            let label = if binary == display {
                binary.to_string()
            } else {
                format!("{} ({})", display, binary)
            };
            Some((*p, label))
        })
        .collect();

    if installable.is_empty() {
        let styled = Prose::new(
            "No installable audio players found for this OS. \
             Install a package manager (e.g., Homebrew on macOS) and try again.",
        );
        println!("{}", styled.render(&Terminal::default()));
        return;
    }

    let options: Vec<String> = installable.iter().map(|(_, label)| label.clone()).collect();

    let selected =
        match inquire::MultiSelect::new("Select audio players to install:", options.clone())
            .with_help_message("Space to toggle, Enter to confirm, Esc to skip")
            .prompt()
        {
            Ok(sel) => sel,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => return,
            Err(e) => {
                error_exit(&format!("selection failed: {e}"), 1);
            }
        };

    if selected.is_empty() {
        return;
    }

    let host = HostCapabilities::load_or_detect_with_verification(false);
    let terminal = Terminal::new();
    let mut ui = install_ui::CliInstallUi::new(terminal, false);

    for label in &selected {
        let idx = options.iter().position(|o| o == label).unwrap();
        let program = installable[idx].0;

        let plan = build_install_plan(program, &host);
        let input = InstallInterviewInput {
            program: plan.program.clone(),
            website: plan.website,
            plan,
        };
        let mut opts = InstallInterviewOptions::default();
        opts.install.timeout_secs = 120;

        if let Err(failure) = run_install_command(
            label,
            opts.install.timeout_secs,
            &mut ui,
            |delegate| run_install_interview(&input, &opts, delegate),
        ) {
            error_exit(&failure.message, failure.exit_code);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct InstallCommandFailure {
    message: String,
    exit_code: i32,
}

impl InstallCommandFailure {
    fn new(message: String) -> Self {
        Self {
            message,
            exit_code: 1,
        }
    }
}

/// Preserves delegate errors and non-success outcomes as a non-zero terminal verdict.
fn run_install_command<D, R>(
    program: &str,
    timeout_secs: u64,
    delegate: &mut D,
    run_interview: R,
) -> Result<(), InstallCommandFailure>
where
    D: sniff::programs::InstallInterviewDelegate,
    R: FnOnce(&mut D) -> Result<InstallInterviewOutcome, sniff::error::SniffInstallationError>,
{
    let outcome = run_interview(delegate)
        .map_err(|error| InstallCommandFailure::new(format!("installation failed: {error}")))?;
    install_outcome_verdict(program, &outcome, timeout_secs).map_err(InstallCommandFailure::new)
}

/// Terminal verdict for one installer outcome at the `playa install` boundary.
///
/// The interview has already narrated itself through [`install_ui::CliInstallUi`]
/// — including the `TimeoutWarning` detached-descendant caveat that precedes
/// [`InstallInterviewOutcome::TimedOut`] — so this seam only decides whether the
/// selected-install loop may continue and, when it may not, what to say.
///
/// ## Returns
///
/// `Ok(())` when the player may be treated as handled, or `Err(message)` with
/// the text the CLI should render before exiting non-zero.
fn install_outcome_verdict(
    program: &str,
    outcome: &InstallInterviewOutcome,
    timeout_secs: u64,
) -> Result<(), String> {
    match outcome {
        InstallInterviewOutcome::Installed { .. } | InstallInterviewOutcome::DryRun { .. } => Ok(()),
        // Declining one player in a multi-select run is a user choice, not a
        // command failure — the remaining selections still deserve a turn.
        InstallInterviewOutcome::AbortedByUser => Ok(()),
        InstallInterviewOutcome::Failed { .. } => Err(format!("{program} installation failed")),
        // The selection list is pre-filtered by `ProgramDetector::installable`,
        // so reaching this arm means the host disagreed with that filter.
        InstallInterviewOutcome::NotInstallable => {
            Err(format!("{program} cannot be installed on this host"))
        }
        InstallInterviewOutcome::TimedOut { .. } => Err(format!(
            "{program} installation timed out after {timeout_secs}s"
        )),
    }
}

fn build_metadata_markdown() -> (String, Vec<String>) {
    let installed = InstalledHeadlessAudio::new();
    let missing = collect_missing_players(&installed);
    let markdown = build_metadata_markdown_table(&installed);
    (markdown, missing)
}

fn build_metadata_markdown_table(installed: &InstalledHeadlessAudio) -> String {
    let mut lines = Vec::new();
    lines.push("| I | Software | Codec Support | File Formats |".to_string());
    lines.push("|---|---|---|---|".to_string());

    for player in all_players() {
        let Some(metadata) = PLAYER_LOOKUP.get(player) else {
            continue;
        };
        let is_installed = installed.is_installed(player.as_headless_audio());
        let indicator = if is_installed { "\u{2705}" } else { "\u{274c}" };
        // Only show clickable links for installed players
        let software = if is_installed {
            link_for_player(*player)
        } else {
            display_name_for_player(*player)
        };
        let codecs = escape_markdown_cell(&format_codec_list(metadata.supported_codecs));
        let formats = escape_markdown_cell(&format_format_list(metadata.supported_formats));
        lines.push(format!(
            "| {} | {} | {} | {} |",
            indicator, software, codecs, formats
        ));
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
    match markdown.as_terminal(TerminalOptions::default()) {
        Ok(rendered) => {
            let output = dim_missing_rows(&rendered, missing_players);
            print!("{}", output);
        }
        Err(_) => println!("{}", markdown.content()),
    }
    append_native_playback_note();
}

fn append_native_playback_note() {
    #[cfg(feature = "native-playback")]
    {
        let mut list = UnorderedList::empty();
        list.add(Prose::new(
            "<italic>native playback is enabled and will be used for \
             .wav, .aiff, .mp3, .flac, .m4a, .ogg, .webm; \
             falling back to host players where needed</italic>",
        ));
        print!("{}", list.render_optimistic(None));
    }
    #[cfg(all(feature = "sfx-native", not(feature = "native-playback")))]
    {
        let mut list = UnorderedList::empty();
        list.add(Prose::new(
            "<italic>native playback is enabled for .wav, .mp3, .ogg; \
             falling back to host players where needed</italic>",
        ));
        print!("{}", list.render_optimistic(None));
    }
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
            if let Some(cell) = software_table_cell(&plain_line) {
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
    // Split by table divider, strip all ANSI/OSC8 from cells 2+ (skip I column),
    // and replace with uniform grey. This prevents darkmatter's inline colors
    // (text color, hyperlink blue) from overriding the dim effect.
    let parts: Vec<&str> = line.split(TABLE_DIVIDER).collect();
    let mut output = String::with_capacity(line.len() + 32);

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            output.push(TABLE_DIVIDER);
        }
        // Cells 0 (before first │) and 1 (I column) stay untouched
        if i >= 2 && i < parts.len() - 1 {
            let stripped = strip_osc8_sequences(&strip_ansi_codes(part));
            output.push_str(MISSING_FG);
            output.push_str(&stripped);
            output.push_str(RESET);
        } else {
            output.push_str(part);
        }
    }

    output
}

/// Returns the second table cell (Software column, skipping the I column).
fn software_table_cell(line: &str) -> Option<&str> {
    let mut parts = line.split(TABLE_DIVIDER);
    parts.next()?; // before first divider
    parts.next()?; // I column
    parts.next() // Software column
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

#[cfg(feature = "sfx-native")]
fn list_output_channels() {
    let devices = sniff::hardware::detect_audio_devices();
    let output_devices: Vec<AudioDeviceInfo> = devices
        .into_iter()
        .filter(|device| {
            matches!(
                device.direction,
                AudioDirection::Output | AudioDirection::InputOutput
            )
        })
        .collect();

    if output_devices.is_empty() {
        println!("No native audio output channels found.");
        return;
    }

    let sfx_default_name = playa::get_default_sfx_device_name();
    let sfx_flags = build_default_sfx_flags(&output_devices, sfx_default_name.as_deref());

    let terminal = Terminal::new();
    print!(
        "{}",
        render_output_channels(&output_devices, &sfx_flags, &terminal)
    );
}

/// For each device, decide whether it is the default sound-effects channel.
///
/// When `sfx_default_name` is `Some`, matches by name; when several devices
/// share a name, only the first (sorted by UID) is marked. When `None`
/// (non-macOS or unavailable), the SFX default mirrors the audio default.
#[cfg(feature = "sfx-native")]
fn build_default_sfx_flags(
    devices: &[AudioDeviceInfo],
    sfx_default_name: Option<&str>,
) -> Vec<bool> {
    let Some(target) = sfx_default_name else {
        return devices.iter().map(|d| d.is_default_output).collect();
    };

    let mut flags = vec![false; devices.len()];
    let mut matches: Vec<usize> = devices
        .iter()
        .enumerate()
        .filter_map(|(i, d)| (d.name == target).then_some(i))
        .collect();
    matches.sort_by(|a, b| devices[*a].uid.cmp(&devices[*b].uid));
    if let Some(&first) = matches.first() {
        flags[first] = true;
    }
    flags
}

#[cfg(feature = "sfx-native")]
fn render_output_channels(
    devices: &[AudioDeviceInfo],
    is_default_sfx: &[bool],
    terminal: &Terminal,
) -> String {
    let suffixes = build_audio_device_name_suffixes(devices);
    let mut ordered: Vec<(usize, &AudioDeviceInfo)> = devices.iter().enumerate().collect();
    ordered.sort_by_key(|(_, device)| device.name.to_lowercase());

    let mut children = UnorderedList::empty();
    for (idx, device) in ordered {
        let sfx_default = is_default_sfx.get(idx).copied().unwrap_or(false);
        children.add(Prose::new(format_output_device_line(
            device,
            &suffixes[idx],
            sfx_default,
        )));
    }

    let mut output_group = UnorderedList::empty();
    output_group.add(Prose::new("<b>Output</b>"));
    output_group.add(children);

    let mut outer = UnorderedList::empty();
    outer.add(output_group);

    let mut doc = Compose::default();
    doc.add_prose(Prose::new("<b><uu>Audio Devices</uu></b>"));
    doc.add_text("\n\n");
    doc.add_unordered_list(outer);

    let mut output = String::from("\n");
    output.push_str(&doc.display(terminal).to_string());
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push('\n');

    let audio_legend = Prose::new(
        "<i> <b><yellow>*</yellow></b> <dim>is the default audio output</dim></i>",
    );
    output.push_str(&audio_legend.display(terminal).to_string());
    if !output.ends_with('\n') {
        output.push('\n');
    }

    let sfx_legend = Prose::new(
        "<i> <b><green>*</green></b> <dim>is the default sound effects channel</dim></i>",
    );
    output.push_str(&sfx_legend.display(terminal).to_string());
    if !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

#[cfg(feature = "sfx-native")]
fn build_audio_device_name_suffixes(devices: &[AudioDeviceInfo]) -> Vec<String> {
    let mut groups: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
    for (idx, device) in devices.iter().enumerate() {
        groups.entry(device.name.as_str()).or_default().push(idx);
    }

    let mut suffixes = vec![String::new(); devices.len()];
    for indices in groups.into_values() {
        if indices.len() < 2 {
            continue;
        }

        let mut ordered = indices;
        ordered.sort_by(|a, b| devices[*a].uid.cmp(&devices[*b].uid));
        for (rank, idx) in ordered.iter().enumerate() {
            suffixes[*idx] = format!("<dim>:{}</dim>", rank + 1);
        }
    }

    suffixes
}

#[cfg(feature = "sfx-native")]
fn format_output_device_line(
    device: &AudioDeviceInfo,
    name_suffix: &str,
    is_default_sfx: bool,
) -> String {
    let kind = style_audio_device_kind(device.kind);
    let rates = format_audio_device_rates(device);

    let audio_star = "<b><yellow>*</yellow></b>";
    let sfx_star = "<b><green>*</green></b>";
    let marker = match (device.is_default_output, is_default_sfx) {
        (true, true) => format!(" {audio_star}{sfx_star}"),
        (true, false) => format!(" {audio_star}"),
        (false, true) => format!(" {sfx_star}"),
        (false, false) => String::new(),
    };

    let name = match (device.is_default_output, is_default_sfx) {
        (true, _) => format!("<b><yellow>{}</yellow></b>", device.name),
        (false, true) => format!("<b><green>{}</green></b>", device.name),
        (false, false) => device.name.clone(),
    };

    let parens = if rates.is_empty() {
        format!("({kind})")
    } else {
        format!("({kind}, {rates})")
    };

    format!("{}{} {}{}", name, name_suffix, parens, marker)
}

#[cfg(feature = "sfx-native")]
fn style_audio_device_kind(kind: AudioDeviceKind) -> String {
    match kind {
        AudioDeviceKind::BuiltIn => "<dim>Built-in</dim>".to_string(),
        AudioDeviceKind::Usb => "<blue>USB</blue>".to_string(),
        AudioDeviceKind::Bluetooth => "<blue>Bluetooth</blue>".to_string(),
        AudioDeviceKind::Thunderbolt => "<yellow>Thunderbolt</yellow>".to_string(),
        AudioDeviceKind::Hdmi => "<yellow>HDMI</yellow>".to_string(),
        AudioDeviceKind::Virtual => "<dim><i>Virtual</i></dim>".to_string(),
        AudioDeviceKind::Unknown => "Unknown".to_string(),
    }
}

#[cfg(feature = "sfx-native")]
fn format_audio_device_rates(device: &AudioDeviceInfo) -> String {
    let mut rates = device.available_sample_rates.clone();
    if device.sample_rate > 0.0 {
        rates.push(device.sample_rate);
    }

    rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rates.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    rates
        .iter()
        .map(|rate| {
            let label = format_sample_rate_khz(*rate);
            if device.sample_rate > 0.0 && (*rate - device.sample_rate).abs() < 0.01 {
                format!("<b>{label}</b>")
            } else {
                format!("<dim>{label}</dim>")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(feature = "sfx-native")]
fn format_sample_rate_khz(rate_hz: f64) -> String {
    let khz = rate_hz / 1000.0;
    if (khz.fract()).abs() < 0.01 {
        format!("{}k", khz as u32)
    } else if (khz * 10.0).fract().abs() < 0.01 {
        format!("{khz:.1}k")
    } else {
        format!("{khz:.2}k")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::programs::{
        ExecutableSource, InstallInterviewDelegate, InstallInterviewEvent, InstallStatusKind,
        InstallationMethod,
    };

    /// Builds a detector that reports exactly `players` as installed,
    /// independent of what the host actually has.
    fn detector_with(players: &[AudioPlayer]) -> InstalledHeadlessAudio {
        players
            .iter()
            .fold(InstalledHeadlessAudio::default(), |detector, player| {
                detector.with_program(
                    player.as_headless_audio(),
                    PathBuf::from("/test/bin"),
                    ExecutableSource::Path,
                )
            })
    }

    #[test]
    fn builds_meta_markdown_with_formatting_and_links() {
        let markdown = build_metadata_markdown_table(&detector_with(&[AudioPlayer::Mpv]));

        assert!(markdown.contains("| I | Software | Codec Support | File Formats |"));
        assert!(markdown.contains("PCM"));
        assert!(markdown.contains("Vorbis"));
        assert!(markdown.contains(".wav"));
        assert!(markdown.contains(&link_for_player(AudioPlayer::Mpv)));
        assert!(markdown.contains('\u{2705}'));
    }

    #[test]
    fn meta_markdown_leaves_uninstalled_players_unlinked() {
        let markdown = build_metadata_markdown_table(&detector_with(&[]));

        assert!(markdown.contains(&display_name_for_player(AudioPlayer::Mpv)));
        assert!(!markdown.contains(&link_for_player(AudioPlayer::Mpv)));
        assert!(!markdown.contains('\u{2705}'));
        assert!(markdown.contains('\u{274c}'));
    }

    #[test]
    fn missing_players_omits_the_installed_ones() {
        let missing = collect_missing_players(&detector_with(&[AudioPlayer::Mpv]));

        assert!(!missing.contains(&display_name_for_player(AudioPlayer::Mpv)));
        assert!(missing.contains(&display_name_for_player(AudioPlayer::Vlc)));
    }

    #[test]
    fn playback_options_default() {
        let opts = PlaybackOptions {
            meta: false,
            background: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        };
        assert!(!opts.fast);
        assert!(!opts.slow);
        assert!(!opts.quiet);
        assert!(!opts.loud);
        assert!(opts.speed.is_none());
        assert!(opts.volume.is_none());
    }

    #[test]
    fn playback_options_fast() {
        let opts = PlaybackOptions {
            meta: false,
            background: false,
            fast: true,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        };
        assert!(opts.fast);
    }

    #[test]
    fn playback_options_custom_speed_and_volume() {
        let opts = PlaybackOptions {
            meta: false,
            background: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: Some(0.9),
            volume: Some(0.3),
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        };
        assert_eq!(opts.speed, Some(0.9));
        assert_eq!(opts.volume, Some(0.3));
    }

    fn base_playback_options() -> PlaybackOptions {
        PlaybackOptions {
            meta: false,
            background: false,
            fast: false,
            slow: false,
            quiet: false,
            loud: false,
            speed: None,
            volume: None,
            channel: None,
            force_host: false,
            #[cfg(feature = "audio-ducking")]
            no_duck: false,
            #[cfg(feature = "audio-ducking")]
            duck_ramp_ms: 1000,
            #[cfg(feature = "audio-ducking")]
            duck_floor: 0.2,
        }
    }

    #[test]
    fn background_flag_detected_for_playback_target() {
        let cli = Cli {
            command: Some(Command::Play {
                audio_file: "tone.wav".to_string(),
                playback: PlaybackOptions {
                    background: true,
                    ..base_playback_options()
                },
            }),
            audio_file: None,
            playback: base_playback_options(),
        };

        assert!(background_requested(&cli));
        assert!(has_playback_target(&cli));
    }

    #[test]
    fn background_flag_rejected_without_playback_target() {
        let cli = Cli {
            command: Some(Command::Players(PlayersCommand::List)),
            audio_file: None,
            playback: PlaybackOptions {
                background: true,
                channel: None,
                ..base_playback_options()
            },
        };

        assert!(background_requested(&cli));
        assert!(!has_playback_target(&cli));
    }

    #[test]
    fn help_generation_does_not_probe_output_channels() {
        let help = Cli::command().render_help().to_string();

        assert!(help.contains("--channel <CHANNEL>"));
        assert!(!help.contains("default audio"));
        assert!(!help.contains("default sfx"));
    }

    #[test]
    fn help_shows_players_subcommands() {
        let help = Cli::command().render_help().to_string();
        assert!(help.contains("players"));
        assert!(help.contains("install"));
    }

    #[cfg(feature = "sfx-native")]
    #[test]
    fn output_channels_render_with_terminal_width() {
        let devices = vec![AudioDeviceInfo {
            name: "Schiit Bifrost 2 Unison USB".to_string(),
            uid: "bifrost".to_string(),
            kind: AudioDeviceKind::Usb,
            direction: AudioDirection::Output,
            is_default_input: false,
            is_default_output: true,
            sample_rate: 48_000.0,
            available_sample_rates: vec![44_100.0, 48_000.0, 88_200.0, 96_000.0],
            input_channels: 0,
            output_channels: 2,
        }];
        let sfx_flags = vec![true];
        let terminal = Terminal::new_optimistic(160);

        let rendered =
            strip_ansi_codes(&render_output_channels(&devices, &sfx_flags, &terminal));

        assert!(rendered.contains("Audio Devices"), "{rendered}");
        assert!(rendered.contains("- Output"), "{rendered}");
        assert!(
            rendered.contains("- Schiit Bifrost 2 Unison USB (USB, 44.1k 48k 88.2k 96k) **"),
            "{rendered}"
        );
        assert!(
            rendered.contains("* is the default audio output"),
            "{rendered}"
        );
        assert!(
            rendered.contains("* is the default sound effects channel"),
            "{rendered}"
        );
    }

    #[cfg(feature = "sfx-native")]
    #[test]
    fn output_channels_render_distinct_audio_and_sfx_devices() {
        let devices = vec![
            AudioDeviceInfo {
                name: "Schiit Bifrost 2 Unison USB".to_string(),
                uid: "bifrost".to_string(),
                kind: AudioDeviceKind::Usb,
                direction: AudioDirection::Output,
                is_default_input: false,
                is_default_output: true,
                sample_rate: 48_000.0,
                available_sample_rates: vec![44_100.0, 48_000.0],
                input_channels: 0,
                output_channels: 2,
            },
            AudioDeviceInfo {
                name: "MacBook Pro Speakers".to_string(),
                uid: "speakers".to_string(),
                kind: AudioDeviceKind::BuiltIn,
                direction: AudioDirection::Output,
                is_default_input: false,
                is_default_output: false,
                sample_rate: 48_000.0,
                available_sample_rates: vec![48_000.0],
                input_channels: 0,
                output_channels: 2,
            },
        ];
        let sfx_flags = vec![false, true];
        let terminal = Terminal::new_optimistic(160);

        let rendered =
            strip_ansi_codes(&render_output_channels(&devices, &sfx_flags, &terminal));

        assert!(
            rendered.contains("- Schiit Bifrost 2 Unison USB (USB, 44.1k 48k) *"),
            "audio default has single yellow star: {rendered}"
        );
        assert!(
            !rendered.contains("Schiit Bifrost 2 Unison USB (USB, 44.1k 48k) **"),
            "audio-only default must not have double star: {rendered}"
        );
        assert!(
            rendered.contains("- MacBook Pro Speakers (Built-in, 48k) *"),
            "sfx default has single green star: {rendered}"
        );
    }

    #[cfg(feature = "sfx-native")]
    #[test]
    fn build_default_sfx_flags_falls_back_to_audio_default_when_unknown() {
        let devices = vec![
            AudioDeviceInfo {
                name: "A".to_string(),
                uid: "a".to_string(),
                kind: AudioDeviceKind::BuiltIn,
                direction: AudioDirection::Output,
                is_default_input: false,
                is_default_output: true,
                ..Default::default()
            },
            AudioDeviceInfo {
                name: "B".to_string(),
                uid: "b".to_string(),
                kind: AudioDeviceKind::Usb,
                direction: AudioDirection::Output,
                is_default_input: false,
                is_default_output: false,
                ..Default::default()
            },
        ];
        assert_eq!(build_default_sfx_flags(&devices, None), vec![true, false]);
        assert_eq!(
            build_default_sfx_flags(&devices, Some("B")),
            vec![false, true]
        );
    }

    #[test]
    fn players_list_parses() {
        let cli = Cli::try_parse_from(["playa", "players", "list"]);
        assert!(cli.is_ok());
        assert!(matches!(
            cli.unwrap().command,
            Some(Command::Players(PlayersCommand::List))
        ));
    }

    #[test]
    fn players_install_parses() {
        let cli = Cli::try_parse_from(["playa", "players", "install"]);
        assert!(cli.is_ok());
        assert!(matches!(
            cli.unwrap().command,
            Some(Command::Players(PlayersCommand::Install))
        ));
    }

    #[test]
    fn install_verdict_timeout_fails_and_names_the_deadline() {
        let outcome = InstallInterviewOutcome::TimedOut {
            attempted: vec![InstallationMethod::Brew("mpv")],
        };

        let verdict = install_outcome_verdict("mpv", &outcome, 120);

        let message = verdict.expect_err("a timed-out installer must not be a success");
        assert!(message.contains("timed out"), "got: {message}");
        assert!(message.contains("120s"), "got: {message}");
        // The timeout verdict must stay distinguishable from a plain failure.
        assert!(!message.contains("installation failed"), "got: {message}");
    }

    #[test]
    fn install_command_timeout_warns_and_returns_nonzero_verdict() {
        let warning =
            "The installer was stopped at its deadline; it may have left partial changes.";
        let mut ui = install_ui::CliInstallUi::with_writer(Terminal::default(), true, Vec::new());

        let result = run_install_command("mpv", 120, &mut ui, |delegate| {
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text: "mpv installation failed".to_string(),
            })?;
            delegate.on_event(&InstallInterviewEvent::TimeoutWarning {
                prose: warning.to_string(),
            })?;
            Ok(InstallInterviewOutcome::TimedOut {
                attempted: vec![InstallationMethod::Brew("mpv")],
            })
        });

        let failure = result.expect_err("a timeout must fail at the command boundary");
        assert_eq!(failure.exit_code, 1);
        assert!(failure.message.contains("timed out"), "got: {}", failure.message);

        let rendered = String::from_utf8(ui.into_writer()).expect("CLI output must be UTF-8");
        let plain = biscuit_terminal::prelude::strip_escape_codes(&rendered);
        assert!(plain.contains("mpv installation failed"), "got: {plain}");
        assert!(plain.contains("stopped at its deadline"), "got: {plain}");
    }

    #[test]
    fn install_verdict_failed_is_a_failure() {
        let outcome = InstallInterviewOutcome::Failed {
            attempted: vec![InstallationMethod::Brew("mpv")],
        };

        let message =
            install_outcome_verdict("mpv", &outcome, 120).expect_err("Failed must not succeed");
        assert!(message.contains("installation failed"), "got: {message}");
    }

    #[test]
    fn install_verdict_not_installable_is_a_failure() {
        let message = install_outcome_verdict("mpv", &InstallInterviewOutcome::NotInstallable, 120)
            .expect_err("NotInstallable must not succeed");
        assert!(message.contains("cannot be installed"), "got: {message}");
    }

    #[test]
    fn install_verdict_installed_and_dry_run_succeed() {
        assert!(
            install_outcome_verdict(
                "mpv",
                &InstallInterviewOutcome::Installed {
                    method: InstallationMethod::Brew("mpv"),
                },
                120,
            )
            .is_ok()
        );
        assert!(
            install_outcome_verdict(
                "mpv",
                &InstallInterviewOutcome::DryRun {
                    method: InstallationMethod::Brew("mpv"),
                },
                120,
            )
            .is_ok()
        );
    }

    #[test]
    fn install_verdict_user_abort_continues_the_loop() {
        assert!(install_outcome_verdict("mpv", &InstallInterviewOutcome::AbortedByUser, 120).is_ok());
    }

}
