# Sniff: Installation and better Type Safety

> **Status: IMPLEMENTED** - This feature set has been implemented as of January 2026.

## Summary

This feature set provides:

- **Type Safety**: All program detection structs now implement the `ProgramDetector` trait
- **Rich Metadata**: `PROGRAM_LOOKUP` contains metadata for 126 programs across all categories
- **Installation Support**: Safe installation via package managers with `installable()` checks
- **Enhanced CLI Output**: `--json-format full` and `--markdown` program tables

---

## Implementation Status

### Type Safety ✅

All 7 `Installed*` structs now implement the `ProgramDetector` trait:

- `InstalledEditors`
- `InstalledUtilities`
- `InstalledLanguagePackageManagers`
- `InstalledOsPackageManagers`
- `InstalledTtsClients`
- `InstalledTerminalApps`
- `InstalledHeadlessAudio`

The `ProgramDetector` trait provides a unified API surface:

```rust
pub trait ProgramDetector {
    type Program: ProgramMetadata + Copy;

    fn refresh(&mut self);
    fn is_installed(&self, program: Self::Program) -> bool;
    fn path(&self, program: Self::Program) -> Option<PathBuf>;
    fn version(&self, program: Self::Program) -> Result<String, ProgramError>;
    fn website(&self, program: Self::Program) -> &'static str;
    fn description(&self, program: Self::Program) -> &'static str;
    fn description_for_terminal(&self, program: Self::Program) -> String; // OSC8 hyperlinks
    fn installed(&self) -> Vec<Self::Program>;
    fn installable(&self, program: Self::Program) -> bool;
    fn install(&self, program: Self::Program) -> Result<(), SniffInstallationError>;
    fn install_version(&self, program: Self::Program, version: &str) -> Result<(), SniffInstallationError>;
}
```

### Program Inventory ✅

The `Program` enum in `inventory.rs` now has full coverage and `PROGRAM_LOOKUP` contains metadata for 126 programs:

- **Editors (26)**: vi, Vim, Neovim, Helix, VSCode, Zed, JetBrains IDEs, TextMate, BBEdit, and more
- **Utilities (30)**: ripgrep, bat, fd, fzf, eza, jq, gh, lazygit, delta, starship, and more
- **Package Managers (27)**: npm, pnpm, yarn, bun, cargo, apt, dnf, winget, nix, and more
- **TTS Clients (15)**: say, espeak/espeak-ng, piper, sherpa-onnx, gTTS, Kokoro TTS, and more
- **Audio Players (11)**: mpv, ffplay, sox, vlc, mpg123, pipewire, and more
- **Terminal Apps (17)**: alacritty, kitty, wezterm, iTerm2, Ghostty, Warp, Windows Terminal, and more

### Installation Module ✅

New `installer.rs` module provides safe command execution:

```rust
use sniff_lib::programs::{execute_install, InstallOptions, InstallationMethod};

// Dry-run mode (shows command without executing)
let method = InstallationMethod::Brew("ripgrep");
let result = execute_install(&method, &InstallOptions::dry_run())?;
println!("Would run: {}", result.command); // "brew install ripgrep"

// Get versioned install command
let cmd = get_versioned_install_command(&InstallationMethod::Cargo("bat"), "0.24.0")?;
// "cargo install bat --version 0.24.0"
```

**Security features:**
- Input sanitization (rejects shell metacharacters)
- `RemoteBash` blocked for automated execution (requires manual confirmation)
- Dry-run mode for previewing commands

### CLI Enhancements ✅

**New `--json-format` flag:**

```bash
# Backward-compatible simple format (default)
sniff --programs --json

# Rich metadata format
sniff --programs --json --json-format full
```

**Rich JSON output includes:**
- `name`: Display name
- `binary_name`: Executable name
- `installed`: Boolean status
- `path`: Path to binary (if installed)
- `version`: Version string (if installed)
- `description`: One-line description
- `website`: Official URL

**New `--markdown` flag:**

```bash
# Render program detection as a markdown table
sniff --programs --markdown
sniff --programs --markdown -v
sniff --programs --markdown -vv
```

Markdown tables include Name, Installed, Description, Website columns and expand
with Binary/Path and Version at higher verbosity levels.

---

## Files Changed

| File | Changes |
|------|---------|
| `sniff/lib/src/programs/types.rs` | `ProgramDetails` struct, `ProgramDetector` trait, `InstallationMethod` helpers |
| `sniff/lib/src/programs/inventory.rs` | `Program` enum with `Copy`, `PROGRAM_LOOKUP` with 31 entries |
| `sniff/lib/src/programs/installer.rs` | **NEW** - Safe installation execution |
| `sniff/lib/src/programs/*.rs` | All 7 `Installed*` structs implement `ProgramDetector` |
| `sniff/cli/src/main.rs` | Added `--json-format` flag |
| `sniff/cli/src/output.rs` | Rich JSON output with `ProgramJsonEntry` |

---

## Implemented Work

- Added remaining programs to `PROGRAM_LOOKUP` (full coverage)
- Implemented `installable()` checks based on detected package managers
- Added markdown table output for text mode via biscuit
- Added Linux (apt/dnf) and Windows (winget) installation support
