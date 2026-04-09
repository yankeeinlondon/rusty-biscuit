# Sound Effects

88 embedded sound effects across 6 categories, feature-gated to control binary size.

## Feature Flags

| Feature | Effects | Size |
|---------|---------|------|
| `sfx-ui` | 43 UI sounds | ~4MB |
| `sfx-cartoon` | 13 cartoon effects | ~8MB |
| `sfx-reactions` | 9 reactions | ~4MB |
| `sfx-scifi` | 11 sci-fi effects | ~3MB |
| `sfx-atmosphere` | 5 atmosphere sounds | ~7MB |
| `sfx-motion` | 7 motion effects | ~5MB |
| `sound-effects` | All 88 effects | ~31MB |

## Usage

```rust
// Get effect by name
let effect = SoundEffect::from_name("sad-trombone").expect("effect enabled");
effect.play()?;

// List all available effects
for effect in SoundEffect::all() {
    println!("{}", effect.name());
}
```

## Native SFX Routing

When `sfx-native` is enabled, Playa prefers a native SFX backend before falling back to regular playback:

- macOS: route through the configured system sound device when possible
- Windows: use WASAPI with `AudioCategory_SoundEffects`
- Linux: use PulseAudio/PipeWire with `media.role=event`

Fallback behavior:

- native SFX errors fall back to the regular Playa path or host players
- native device-open deadlines are bounded
- a native device-open timeout trips the process-local native-audio breaker so future attempts fail fast and fallback directly

Use `--force-host` when you want to skip native SFX routing entirely.

## Example Effects by Category

**UI**: click, beep, notification, error, success
**Cartoon**: boing, pop, whoosh, splat, slide-whistle
**Reactions**: applause, sad-trombone, drumroll, rimshot
**Sci-Fi**: laser, teleport, power-up, alarm
**Atmosphere**: wind, rain, thunder, fire
**Motion**: swoosh, impact, bounce, roll

## CLI

```bash
playa effect sad-trombone                 # Play effect
playa effect click --channel "Headphones" # Route to a specific native output
playa list-effects                        # List all effects
playa list-effects cartoon                # Filter by name, description, or category
playa output-channels                     # Inspect native output devices (with `sfx-native`)
```

Effect names autocomplete in Bash, Zsh, and Fish shells.
