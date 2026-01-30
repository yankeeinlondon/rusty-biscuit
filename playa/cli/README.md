# Playa CLI

Playa CLI plays audio files by delegating to installed host players. It can also
print a markdown table describing supported players, codecs, and file formats.

## Usage

Play a file:

```bash
playa path/to/audio.wav
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
playa --effect sad-trombone
```

List built-in sound effects:

```bash
playa --list-effects
```

Show available players table:

```bash
playa --players
```

Display playback metadata during playback:

```bash
playa --meta audio.wav
```

## CLI Options

| Option | Description |
|--------|-------------|
| `--players` | Show table of available players and their capabilities |
| `--meta` | Display playback metadata (player, volume, speed, codec, format) |
| `--effect <NAME>` | Play a built-in sound effect by name |
| `--list-effects` | List all available sound effects |
| `--fast` | Play at 1.25x speed |
| `--slow` | Play at 0.75x speed |
| `--quiet` | Play at 50% volume |
| `--loud` | Play at 150% volume |
| `--speed <N>` | Custom playback speed (0.5 to 2.0) |
| `--volume <N>` | Custom volume level (0.0 to 2.0) |

## Output (--players)

The `--players` flag renders a markdown table with these columns:

- Software (markdown link to the official website)
- Codec Support
- File Formats

Missing players are dimmed in grey with a note at the bottom.

## Notes

- Rendering uses the `darkmatter-lib` markdown terminal renderer for tables.
- Playback uses the Playa library's detection and player matching.
- This CLI enables the full `sound-effects` feature by default (~30MB binary).
