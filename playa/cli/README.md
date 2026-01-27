# Playa CLI

Playa CLI plays audio files by delegating to installed host players. It can also
print a markdown table describing supported players, codecs, and file formats.

## Usage

Play a file:

```bash
playa path/to/audio.wav
```

Play a built-in sound effect:

```bash
playa --effect sad-trombone
```

List built-in sound effects:

```bash
playa --list-effects
```

Show player metadata:

```bash
playa --meta
```

## Output (metadata)

The `--meta` flag renders a markdown table with these columns:

- Software (markdown link to the official website)
- Codec Support
- File Formats

After the table, a single markdown list item is printed listing the programs not
found on the host.

## Notes

- Rendering uses the `shared::markdown::Markdown` terminal renderer, which handles
  tables and formatting.
- Playback uses the Playa library's detection and player matching.
- This CLI enables the full `sound-effects` set, which adds ~30MB to the binary.
