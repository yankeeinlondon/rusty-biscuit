# Playa CLI

Playa CLI plays audio files by delegating to installed host players. It provides
subcommands for playing effects, listing players, and filtering sound effects.

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

List built-in sound effects:

```bash
playa list-effects             # All 85 effects
playa list-effects cartoon     # Filter by name, description, or category
```

Show available players table:

```bash
playa players
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
| `players` | Show table of available players and their capabilities |

## Playback Options

These options apply to `play` and `effect` subcommands, as well as the default mode:

| Option | Description |
|--------|-------------|
| `--meta` | Display playback metadata (player, volume, speed, codec, format) |
| `--fast` | Play at 1.25x speed |
| `--slow` | Play at 0.75x speed |
| `--quiet` | Play at 50% volume |
| `--loud` | Play at 150% volume |
| `--speed <N>` | Custom playback speed (0.5 to 2.0) |
| `--volume <N>` | Custom volume level (0.0 to 2.0) |

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

The `players` subcommand renders a markdown table with these columns:

- Software (markdown link to the official website)
- Codec Support
- File Formats

Missing players are dimmed in grey with a note at the bottom.

## Notes

- Rendering uses the `darkmatter-lib` markdown terminal renderer for tables.
- Playback uses the Playa library's detection and player matching.
- This CLI enables the full `sound-effects` feature by default (~31MB binary).
