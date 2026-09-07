# Playa CLI

Playa CLI plays audio through the native OS backend first and falls back to
capability-ranked installed host players. It also provides durable background
playback, completion reports, embedded effects, and player diagnostics.

## Usage

Play a file:

```bash
playa path/to/audio.wav
playa play path/to/audio.wav   # Explicit subcommand
```

Play with speed/volume control:

```bash
playa --fast audio.mp3        # 1.25x speed
playa --slow audio.mp3        # 0.75x speed
playa --quiet audio.mp3       # 50% volume
playa --loud audio.mp3        # 150% volume
playa --speed 1.5 audio.mp3   # Custom speed (0.5-2.0)
playa --volume 0.8 audio.mp3  # Custom volume (0.0-2.0)
```

Play a built-in sound effect:

```bash
playa effect sad-trombone
playa effect sad-trombone --loud
```

Play in the background and return immediately:

```bash
playa --background audio.wav
playa effect sad-trombone --background
```

Background jobs are durably published to a private per-user spool, globally
serialized across Playa, biscuit-speaks, and Claudine processes, and survive the
requesting process exiting. Delivery is best-effort and at-most-once once
playback starts. Inspect redacted queue and journal outcomes with:

```bash
playa spool
```

List built-in sound effects:

```bash
playa list-effects             # All 88 effects
playa list-effects cartoon     # Filter by name, description, or category
```

Show available players table, or install the missing ones:

```bash
playa players list
playa players install   # interactive
```

Display playback metadata during playback:

```bash
playa --meta audio.wav
```

## Subcommands

| Subcommand | Description |
|------------|-------------|
| `play <FILE>` | Play an audio file (also the default when no subcommand given) |
| `effect <NAME>` | Play a built-in sound effect by name |
| `list-effects [FILTER]` | List available sound effects, optionally filtered |
| `players list` | Show table of available players and their capabilities |
| `players install` | Interactively install missing headless audio players |
| `output-channels` | Show output audio devices in the same grouped format as `sniff audio-devices` (requires `sfx-native` feature) |
| `duck-info` | Show audio ducking backend info (requires `audio-ducking` feature) |
| `spool` | Show redacted pending, in-flight, failed, and recent journal outcomes for detached audio |

`players` has no default subcommand — bare `playa players` prints the subcommand
help.

## Playback Options

These options apply to `play` and `effect` subcommands, as well as the default mode:

| Option | Description |
|--------|-------------|
| `--meta` | Display playback metadata (player, volume, speed, codec, format) |
| `--background` | Publish playback to the ordered per-user spool and return after durable handoff |
| `--fast` | Play at 1.25x speed |
| `--slow` | Play at 0.75x speed |
| `--quiet` | Play at 50% volume |
| `--loud` | Play at 150% volume |
| `--speed <N>` | Custom playback speed (0.5 to 2.0) |
| `--volume <N>` | Custom volume level (0.0 to 2.0) |
| `--channel <CHANNEL>` | Output channel (audio device) to play through, by name |
| `--force-host` | Force host player playback, skipping the native decoder |
| `--no-duck` | Disable audio ducking (requires `audio-ducking` feature) |
| `--duck-ramp-ms <MS>` | Ducking ramp duration in milliseconds (default: 1000) |
| `--duck-floor <LEVEL>` | Ducking floor level, 0.0–1.0 (default: 0.2) |

## Shell Completions

Enable shell completions by adding one of the following to your shell config:

```bash
# Bash (~/.bashrc)
source <(COMPLETE=bash playa)

# Zsh (~/.zshrc)
source <(COMPLETE=zsh playa)

# Fish (~/.config/fish/config.fish)
COMPLETE=fish playa | source
```

Effect names autocomplete when typing `playa effect <TAB>`.

## Output (players)

`players list` renders a markdown table with these columns:

- `I` — whether the player is installed on this host
- Software (markdown link to the official website)
- Codec Support
- File Formats

Rows for players that are not installed are dimmed in grey. When a native
playback feature is enabled, a note below the table names the formats that
bypass host players.

## Notes

- Rendering uses the `darkmatter-lib` markdown terminal renderer for tables.
- Automatic playback is native-first; `--force-host` skips the native route.
- The report-returning library APIs and detached journal retain route,
  expected/elapsed duration, and completion verdict.
- Positively probed mono input gains mpv's stereo-output workaround; unknown,
  stereo, and multichannel input is passed through unchanged.
- `PLAYA_DRY_RUN=1` performs no source, cache, spool, journal, or subprocess I/O.
- Spool output redacts private job details, including speech preparation text.
- This CLI enables the full `sound-effects` feature by default, embedding all 88
  effects (~27MB of audio under `playa/effects`) into the binary.
