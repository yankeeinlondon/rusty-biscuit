# Sound Effects

88 embedded sound effects across 6 categories, feature-gated to control binary size.

## Feature Flags

| Feature | Effects | Size |
|---------|---------|------|
| `sfx-ui` | 43 UI sounds | ~4MB |
| `sfx-cartoon` | 13 cartoon effects | ~8MB |
| `sfx-reactions` | 9 reactions | ~4MB |
| `sfx-scifi` | 11 sci-fi effects | ~3MB |
| `sfx-atmosphere` | 5 atmosphere | ~7MB |
| `sfx-motion` | 7 motion effects | ~5MB |
| `sound-effects` | All 88 effects | ~31MB |

## Usage

```rust
// Get effect by name
let effect = SoundEffect::from_name("sad-trombone")?;
effect.play()?;

// List all available effects
for effect in SoundEffect::all() {
    println!("{}", effect.name());
}
```

## Example Effects by Category

**UI**: click, beep, notification, error, success
**Cartoon**: boing, pop, whoosh, splat, slide-whistle
**Reactions**: applause, sad-trombone, drumroll, rimshot
**Sci-Fi**: laser, teleport, power-up, alarm
**Atmosphere**: wind, rain, thunder, fire
**Motion**: swoosh, impact, bounce, roll

## CLI

```bash
playa effect sad-trombone      # Play effect
playa list-effects             # List all effects
playa list-effects cartoon     # Filter by name, description, or category
```

Effect names autocomplete in Bash, Zsh, and Fish shells.
