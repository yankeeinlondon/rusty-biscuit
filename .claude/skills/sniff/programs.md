# Program Detection

Parallel detection across 8 categories with macOS app bundle support.

## Categories

| Category | Field | Enum | Examples |
|----------|-------|------|----------|
| Editors | `editors` | `Editor` | vim, VS Code, Cursor, IntelliJ, Sublime |
| Utilities | `utilities` | `Utility` | ripgrep, fzf, bat, jq, fd, delta |
| Language PMs | `language_package_managers` | `LanguagePackageManager` | cargo, npm, pip, poetry |
| OS PMs | `os_package_managers` | `OsPackageManager` | homebrew, apt, dnf, pacman |
| TTS Clients | `tts_clients` | `TtsClient` | say, espeak, piper |
| Terminal Apps | `terminal_apps` | `TerminalApp` | alacritty, wezterm, kitty, iTerm2 |
| Headless Audio | `headless_audio` | `HeadlessAudio` | afplay, pacat, aplay |
| AI CLI | `ai_clients` | `AiCli` | claude, aider, goose |

## Usage

```rust
use sniff_lib::programs::ProgramsInfo;

let programs = ProgramsInfo::detect();
println!("Editors: {:?}", programs.editors);
println!("Utilities: {:?}", programs.utilities);
println!("AI CLI tools: {:?}", programs.ai_clients);

// Access metadata
for editor in &programs.editors {
    println!("{}: {}", editor.display_name(), editor.description());
}
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
sniff programs                   # All categories (text output)
sniff editors                    # Just editors
sniff utilities                  # CLI utilities
sniff language-package-managers  # Language package managers
sniff os-package-managers        # OS package managers
sniff tts-clients                # TTS programs
sniff terminal-apps              # Terminal emulators
sniff audio                      # Headless audio players
sniff agents                     # AI agent/CLI tools
```

**JSON output:**
```bash
sniff programs --json                    # Simple format (backward compatible)
sniff programs --json --json-format full # Rich metadata (display name, description, website, version, source)
```

## Adding a Program Category

See [extending.md](./extending.md) for step-by-step instructions.
