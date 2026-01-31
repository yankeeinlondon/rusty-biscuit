# Program Detection

Parallel detection across 8 categories with macOS app bundle support.

## Categories

| Category | Examples |
|----------|----------|
| `editors` | VS Code, Neovim, Sublime Text |
| `utilities` | ripgrep, fd, jq, fzf |
| `pkg_managers` | brew, cargo, npm, pip |
| `tts_clients` | say, espeak, piper |
| `headless_audio` | mpv, ffplay, sox |
| `browsers` | Chrome, Firefox, Safari |
| `terminals` | iTerm2, Wezterm, Alacritty |
| `shells` | zsh, bash, fish |

## Usage

```rust
use sniff_lib::programs::ProgramsInfo;

let programs = ProgramsInfo::detect();
println!("Editors: {:?}", programs.editors);
println!("Utilities: {:?}", programs.utilities);
```

## macOS App Bundle Fallback

PATH lookup with `/Applications` fallback:

```rust
use sniff_lib::programs::find_program_with_source;

let (path, source) = find_program_with_source("code");
match source {
    ExecutableSource::Path => { /* Found in PATH */ }
    ExecutableSource::MacOsBundle(bundle) => { /* Found in /Applications */ }
    ExecutableSource::NotFound => { /* Not installed */ }
}
```

Searches:
1. `$PATH` directories
2. `/Applications/*.app/Contents/MacOS/`
3. `~/Applications/*.app/Contents/MacOS/`

## CLI Subcommands

```bash
sniff programs             # All categories (text output)
sniff editors              # Just editors
sniff utilities            # Just utilities
sniff tts-clients          # TTS programs
sniff audio                # Audio players
sniff programs --json      # JSON output
sniff programs --markdown  # Markdown table output
```

## Parallel Detection

Programs are detected in parallel using rayon for performance:

```rust
// Internal implementation uses parallel iteration
let results: Vec<_> = categories.par_iter()
    .map(|cat| detect_category(cat))
    .collect();
```
