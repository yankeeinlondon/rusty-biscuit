# Sniff Package

## Features

### 1. Rename `filesystem/monorepo` to `filesystem/repo`

- Rename the `filesystem/monorepo` section to `filesystem/repo`
- Rename the property `monorepo.tool` to `repo.monorepo_tool`
- The `filesystem/packages` section should only appear when the repo IS a monorepo
- Dependency location is determined by repo type:
    - **Non-monorepo**: Dependencies appear directly on `filesystem/repo`:
        - `dev_dependencies`
        - `dependencies`
        - `peer_dependencies`
        - `optional_dependencies`
    - **Monorepo**: Dependencies appear at the package level within `filesystem/packages` (not at repo level)

> Each of these properties will have an array of dependencies; see [Dependency Properties](#dependency-properties) for details.

### 2. Remove `filesystem/dependencies` Section

- The `filesystem/dependencies` section is currently useless and should be removed
- For monorepos, each package in `filesystem/packages` should include:
    - `dev_dependencies`
    - `dependencies`
    - `peer_dependencies`
    - `optional_dependencies`

### 3. CLI Switches for Filtering Output

Add CLI switches to narrow down reporting scope. **All filter switches are mutually exclusive** - using more than one together raises an error with a clear description.

**Top-level switches:**

- `--filesystem` - Show all filesystem info
- `--hardware` - Show all hardware info
- `--os` - Show all OS-level info (package managers, locale, timezone, shell, terminal, etc.)

**Detail-level switches (also mutually exclusive with top-level switches):**

- `--git` - Show only `filesystem/git` info
- `--repo` - Show only `filesystem/repo` info
- `--language` - Show only `filesystem/language` info
- `--cpu` - Show only `hardware/cpu` info
- `--gpu` - Show only `hardware/gpu` info (NOTE: rename `hardware/gpus` to `hardware/gpu` for consistency)
- `--memory` - Show only `hardware/memory` info
- `--storage` - Show only `hardware/storage` info

**Error behavior:** If any combination of these switches is used together (e.g., `sniff --cpu --memory` or `sniff --filesystem --git`), the CLI should exit with an error explaining that these flags are mutually exclusive.

## Dependency Properties

Each dependency in the arrays (`dev_dependencies`, `dependencies`, `peer_dependencies`, `optional_dependencies`) has the following properties:

| Property | Description | Availability |
|----------|-------------|--------------|
| `targeted_version` | The version/version range specified in the manifest file (e.g., `package.json`, `Cargo.toml`) | Always |
| `actual_version` | The resolved version from the lock file | Only when lock file exists; **omitted** if no lock file |
| `package_manager` | String name of the package manager (e.g., `npm`, `pnpm`, `yarn`, `bun`, `cargo`, `pip`) | Always |
| `latest_version` | The latest available version from the registry | Only with `--deep` flag (requires network call) |

**Notes:**

- When no lock file is present (e.g., fresh clone), `actual_version` is simply omitted from the output
- The `--deep` flag enables network calls to fetch `latest_version` for all dependencies

## Sniff Library

This section will require some refactoring but where we can just re-organize versus refactor that is the approach we should take.

### OS Module

- Currently `os.rs` is nested under hardware, which is incorrect
- OS should be promoted to a top-level module at the same level as `filesystem`, `hardware`, and `network`
- The `--os` flag should show all OS-level information:
    - Package managers (OS-level: homebrew, apt, nala, nix, etc.)
    - Locale settings
    - Timezone
    - Shell
    - Terminal
    - Other OS-level details

### Package Module

- Resolving packages via package managers should be organized as its own top-level module
- It serves both OS and Filesystem reporting but deserves separate treatment

#### Package Manager Enums

Differentiate between OS-level and language-level package managers:

```rust
#[derive(Debug, Clone, strum::Display, strum::EnumString)]
#[strum(serialize_all = "PascalCase")]
pub enum OsPackageManager {
    Homebrew,
    Apt,
    Nala,
    Nix,
    // ...
}

#[derive(Debug, Clone, strum::Display, strum::EnumString)]
#[strum(serialize_all = "PascalCase")]
pub enum LanguagePackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Pip,
    Cargo,
    // ...
}

#[derive(Debug, Clone)]
pub enum PackageManager {
    Os(OsPackageManager),
    Language(LanguagePackageManager),
}
```

**Serialization:** Use `strum` to serialize variants as PascalCase strings (e.g., `LanguagePackageManager::Npm` → `"Npm"`).

Each enum variant should implement `get_manager()` to return the corresponding struct that implements `PackageManagerShape`.

#### PackageManagerShape Trait

All package manager operations are **async by default** since they may require network calls.

```rust
pub struct PackageInfo {
    /// The name of the package (in the given package manager)
    pub name: String,
    /// A short description of the package (if available)
    pub description: Option<String>,
    /// The package manager from which this package was found
    pub from: String,
    /// The latest version (typically semver, but varies by package manager)
    /// Only populated when --deep flag is used
    pub latest_version: Option<String>,
}

#[async_trait]
pub trait PackageManagerShape {
    /// The primary web URL to search for packages
    fn search_url(&self) -> Option<&str>;

    /// The primary web URL with information about the package manager
    fn info_url(&self) -> &str;

    /// The filename of the lock file (if any)
    fn lock_file(&self) -> Option<&str>;

    /// The CLI executable name on Windows
    fn windows_cli(&self) -> Option<&str>;

    /// The CLI executable name on macOS
    fn macos_cli(&self) -> Option<&str>;

    /// The CLI executable name on Linux
    fn linux_cli(&self) -> Option<&str>;

    /// Find a package by name (async, may require network call)
    async fn find_package(&self, pkg: &str) -> Option<PackageInfo>;

    /// Check if a package exists (async, may require network call)
    async fn has_package(&self, pkg: &str) -> bool;
}
```

#### Static Package Manager Registry

```rust
use std::sync::LazyLock;
use std::collections::HashMap;

static PACKAGE_MANAGERS: LazyLock<HashMap<String, Box<dyn PackageManagerShape + Send + Sync>>> =
    LazyLock::new(|| {
        let mut map: HashMap<String, Box<dyn PackageManagerShape + Send + Sync>> = HashMap::new();
        map.insert("Npm".into(), Box::new(Npm));
        map.insert("Pip".into(), Box::new(Pip));
        map.insert("Cargo".into(), Box::new(Cargo));
        // ...
        map
    });
```


