# OS & Environment Detection

Detection functions for operating system, Linux distributions, CI environments, fonts, and locale.

## OS Detection

```rust
use biscuit_terminal::discovery::os_detection::{detect_os_type, OsType};

match detect_os_type() {
    OsType::MacOS => println!("macOS"),
    OsType::Linux => println!("Linux"),
    OsType::Windows => println!("Windows"),
    OsType::FreeBSD => println!("FreeBSD"),
    OsType::Android => println!("Android"),
    _ => println!("Other OS"),
}
```

### OsType Enum

```rust
pub enum OsType {
    Windows,
    Linux,
    MacOS,
    FreeBSD,
    NetBSD,
    OpenBSD,
    DragonFly,
    Illumos,
    Android,
    Ios,
    Unknown,
}
```

## Linux Distribution

```rust
use biscuit_terminal::discovery::os_detection::{detect_linux_distro, LinuxDistro, LinuxFamily};

if let Some(distro) = detect_linux_distro() {
    println!("ID: {}", distro.id);           // "ubuntu"
    println!("Name: {}", distro.name);       // "Ubuntu 22.04 LTS"
    println!("Family: {:?}", distro.family); // LinuxFamily::Debian

    if let Some(ver) = &distro.version {
        println!("Version: {}", ver);
    }
    if let Some(code) = &distro.codename {
        println!("Codename: {}", code);
    }
}
```

### LinuxFamily Enum

```rust
pub enum LinuxFamily {
    Debian,   // Ubuntu, Debian, Mint, Pop!_OS
    RedHat,   // Fedora, RHEL, CentOS, Rocky
    Arch,     // Arch, Manjaro, EndeavourOS
    SUSE,     // openSUSE, SLES
    Alpine,
    Gentoo,
    Void,
    NixOS,
}
```

Detection: Parses `/etc/os-release`

## CI Environment Detection

```rust
use biscuit_terminal::discovery::os_detection::is_ci;

if is_ci() {
    // Disable interactive features
    // Simplify output
}
```

Detects 50+ CI services including:
- GitHub Actions (`GITHUB_ACTIONS`)
- GitLab CI (`GITLAB_CI`)
- Jenkins (`JENKINS_URL`)
- CircleCI (`CIRCLECI`)
- Travis CI (`TRAVIS`)
- Azure Pipelines (`TF_BUILD`)
- Bitbucket Pipelines (`BITBUCKET_BUILD_NUMBER`)
- And many more...

## Font Detection

Fonts are detected by parsing terminal configuration files.

### Font Name

```rust
use biscuit_terminal::discovery::fonts::font_name;

if let Some(name) = font_name() {
    println!("Font: {}", name);  // "JetBrains Mono"
}
```

### Font Size

```rust
use biscuit_terminal::discovery::fonts::font_size;

if let Some(size) = font_size() {
    println!("Size: {}pt", size);
}
```

### Nerd Font Detection

```rust
use biscuit_terminal::discovery::fonts::detect_nerd_font;

match detect_nerd_font() {
    Some(true) => println!("Nerd Font detected"),
    Some(false) => println!("Explicitly not Nerd Font"),
    None => println!("Unknown"),
}
```

Detection order:
1. `NERD_FONT` env var (explicit declaration)
2. Font name pattern matching (69 known Nerd Font families)

### Ligature Support

```rust
use biscuit_terminal::discovery::fonts::ligature_support_likely;

if ligature_support_likely() {
    // Font likely supports ligatures
}
```

Heuristic based on font name (Fira Code, JetBrains Mono, etc.)

### Cell Size

```rust
use biscuit_terminal::discovery::fonts::cell_size;

if let Some(cs) = cell_size() {
    println!("Cell: {}x{} pixels", cs.width, cs.height);
}
// Default: 8×16 if detection fails
```

Used for accurate image aspect ratio calculation.

### Config Parsing by Terminal

| Terminal | Config Format | Font Setting | Size Setting |
|----------|--------------|--------------|--------------|
| WezTerm | Lua | `config.font = wezterm.font("Name")` | `config.font_size = N` |
| Ghostty | Key=Value | `font-family = Name` | `font-size = N` |
| Kitty | Conf | `font_family Name` | `font_size N` |
| Alacritty | TOML | `[font.normal] family = "Name"` | `[font] size = N` |
| iTerm2 | macOS defaults | System preferences | System preferences |

## Locale Detection

```rust
use biscuit_terminal::discovery::locale::{CharEncoding, TerminalLocale};

let term = Terminal::new();

// Raw locale string
if let Some(raw) = term.locale.raw() {
    println!("Locale: {}", raw);  // "en_US.UTF-8"
}

// Normalized BCP47 tag
if let Some(tag) = term.locale.tag() {
    println!("Tag: {}", tag);     // "en-US"
}

// Character encoding
match term.char_encoding {
    CharEncoding::UTF8 => println!("UTF-8"),
    CharEncoding::ASCII => println!("ASCII"),
    CharEncoding::Latin1 => println!("Latin-1"),
    CharEncoding::Other(s) => println!("Other: {}", s),
}
```

## Config File Path

```rust
use biscuit_terminal::discovery::config_paths::get_terminal_config_path;
use biscuit_terminal::discovery::detection::get_terminal_app;

let app = get_terminal_app();
if let Some(path) = get_terminal_config_path(&app) {
    println!("Config: {}", path.display());
}
```

Returns paths like:
- WezTerm: `~/.wezterm.lua` or `~/.config/wezterm/wezterm.lua`
- Kitty: `~/.config/kitty/kitty.conf`
- Alacritty: `~/.config/alacritty/alacritty.toml`
- Ghostty: `~/.config/ghostty/config`

## Complete Example

```rust
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::discovery::os_detection::is_ci;

fn print_environment() {
    let term = Terminal::new();

    println!("=== Environment ===");
    println!("OS: {:?}", term.os);

    if let Some(distro) = &term.distro {
        println!("Distro: {} ({:?})", distro.name, distro.family);
    }

    println!("CI: {}", term.is_ci);

    println!("\n=== Font ===");
    if let Some(font) = &term.font {
        println!("Name: {}", font);
    }
    if let Some(size) = term.font_size {
        println!("Size: {}pt", size);
    }
    if let Some(nerd) = term.is_nerd_font {
        println!("Nerd Font: {}", nerd);
    }

    println!("\n=== Locale ===");
    if let Some(raw) = term.locale.raw() {
        println!("Raw: {}", raw);
    }
    println!("Encoding: {:?}", term.char_encoding);
}
```

## Related

- [Terminal Struct](./terminal-struct.md) - Contains these as properties
- [Detection Functions](./discovery.md) - Terminal-specific detection
