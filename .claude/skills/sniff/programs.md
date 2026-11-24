# Program Detection

## Contents

- Categories
- Usage
- Editor launch facts
- macOS App Bundle Fallback
- Windows Fallback Chain
- CLI Subcommands
- Extension route

Use heading search to jump to the listed subsystem.


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

## Editor launch facts

`programs/enums/editor_traits.rs` holds `EDITOR_TRAITS`, indexed by
`*editor as usize` like `EDITOR_INFO` (a count test guards both). It stays out
of `ProgramInfo` on purpose: `ProgramInfo` is shared and serialized, so editor
facts there would change every category's shape.

- `Editor::surface()` → `EditorSurface::{Terminal, Gui, Both}`;
  `wait_args()` (`--wait`, Kate `--block`); `can_block()` is false only for a
  GUI editor with no wait args (Lapce, Geany).
- `Editor::identify(program)` compares the final path component of one
  program token (callers split `$EDITOR` first), stripping only
  `.exe/.cmd/.bat/.com`, against binary name, alternates, and
  `identify_also`; case-insensitive only under `cfg(windows)`.
- Adding an `Editor` variant: add its `EDITOR_TRAITS` row in the same position
  and its arm in the test's wildcard-free `expected` match.
- Sniff is the only editor-flag table; consumers (Darkmatter's editor module)
  must not keep their own.
- The display session (`sniff::os::detect_display_session`, `os/display.rs`)
  is the GUI-availability companion: macOS/Windows always available,
  Linux/WSL by non-empty `DISPLAY`/`WAYLAND_DISPLAY`. On-demand only.

## macOS App Bundle Fallback

PATH lookup with `/Applications` fallback:

```rust
use sniff::programs::{ExecutableSource, find_program_with_source};

if let Some((path, source)) = find_program_with_source("code") {
    match source {
        ExecutableSource::Path => { /* Found in PATH */ }
        ExecutableSource::MacOsAppBundle => { /* Found in an app bundle */ }
        ExecutableSource::WindowsAppPaths
        | ExecutableSource::WindowsInstallRoot
        | ExecutableSource::ProjectLocal => { /* Other platform/project source */ }
    }
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

`programs::windows_shells` (`detect`, `probe`) is the per-shell view of the
same question: which PowerShell, MSYS (Git Bash before MSYS2), and Cygwin
shell is installed, classified by the runtime DLL beside `bash.exe`, and
whether each, started without startup files under the 2 s
`timeouts::SHELL_LOOKUP` through `process::run_command_in_tree` (windowless
`TreeConfig`), finds a name. Selection rules are pure over `DiscoveryInputs`
and tested on every OS with fake layouts; `detect()` is empty off native
Windows. User docs: [Windows shells](../../../sniff/docs/topics/windows-shells.md).

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

## Extension route

Use [extending.md](./extending.md) to add or change a program category,
detector, fallback, or installation implementation. This topic owns the current
catalog and user-facing CLI surface; `extending.md` owns contributor steps.
