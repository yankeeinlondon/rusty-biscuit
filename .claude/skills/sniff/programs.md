# Program Detection

Parallel detection across 10 categories with macOS app bundle and Windows fallback support. A single shared `ExecutableIndex` scans `PATH` and platform-specific fallback directories once, then all categories perform O(1) HashMap lookups against it in parallel via `rayon::join` pairs.

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
| Notification Helpers | `notification_helpers` | `NotificationHelper` | notify-send, terminal-notifier, dunstify |
| Test Runners | `test_runners` | `TestRunner` | cargo test, vitest, pytest, go test |

## Usage

```rust
use sniff::programs::ProgramsInfo;

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
use sniff::programs::find_program_with_source;

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

## Windows Fallback Chain

On Windows, the executable search expands beyond PATH:

1. **PATH** — `CreateProcess`-compatible, returns `ExecutableSource::Path`.
2. **App Paths registry** — `HKCU` then `HKLM`
   (`SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths`). HKCU wins ties.
   Env vars expanded via `ExpandEnvironmentStringsW`; orphaned entries filtered.
   Returns `ExecutableSource::WindowsAppPaths`.
3. **Install-root walk** — one level deep under `%ProgramFiles%`,
   `%ProgramFiles(x86)%`, and `%LocalAppData%\Programs`. Returns
   `ExecutableSource::WindowsInstallRoot`.

Combined Windows scan cost: 40–80 ms warm cache, built once inside
`ExecutableIndex::build()`.

## CLI Subcommands

```bash
sniff software                          # All categories (text output)
sniff software editors                  # Just editors
sniff software utilities                # CLI utilities
sniff software language-package-managers  # Language package managers
sniff software os-package-managers      # OS package managers
sniff software tts-clients              # TTS programs
sniff software terminal-apps            # Terminal emulators
sniff software audio-players            # Headless audio players
sniff software notification-helpers     # Desktop notification helpers
sniff software agents                   # AI agent/CLI tools
sniff software test-runners             # Host test-runner availability
```

**JSON output:**
```bash
sniff software --json                   # JSON with full metadata
```

**Install subcommand (eight installable categories or aggregate):**
```bash
sniff software editors install          # Interactive picker
sniff software editors install nvim     # Install specific program
sniff software install                  # Pick from all installable categories
```

Notification helpers and test runners are report-only categories; they do not expose `install` or `install-plan` actions.

## Adding a Program Category

See [extending.md](./extending.md) for step-by-step instructions.
