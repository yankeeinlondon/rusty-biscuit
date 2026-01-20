# Platform Specifics

Each operating system handles system tray icons differently. Understanding these differences is crucial for a polished experience.

## Windows

### Build Requirements
```toml
# Hide console window in release builds
[profile.release]
windows_subsystem = "windows"
```

Or in code:
```rust
#![windows_subsystem = "windows"]
```

### Icon Requirements
- **Format**: ICO or PNG
- **Size**: 256x256 works well (automatically scaled)
- **Location**: System tray (bottom-right)

### Behavior
- **Right-click**: Shows menu
- **Left-click**: Custom action (you define)
- **Hover**: Shows tooltip

### Windows-Specific Code
```rust
#[cfg(target_os = "windows")]
{
    // Windows-specific behavior
    if event.event == ClickType::Left {
        // Common: toggle window visibility
    }
}
```

## macOS

### Icon Requirements
- **Format**: PNG only
- **Size**: 22x22 pixels (44x44 for Retina @2x)
- **Location**: Menu bar (top)
- **Color**: Template images (black with alpha) recommended

### Important Constraints
```rust
fn main() {
    // macOS REQUIRES the event loop on the main thread
    let event_loop = EventLoop::new().unwrap();

    // This assertion helps catch issues early
    #[cfg(target_os = "macos")]
    assert!(
        std::thread::current().name() == Some("main"),
        "Event loop must run on main thread on macOS"
    );
}
```

### Behavior Differences
- **Left-click**: Shows menu (different from Windows!)
- **Right-click**: Also shows menu
- **No tooltips**: macOS doesn't support tray tooltips

### macOS Icon Sizing
```rust
#[cfg(target_os = "macos")]
fn prepare_mac_icon(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes)
        .unwrap()
        .resize_exact(22, 22, image::imageops::FilterType::Lanczos3)
        .into_rgba8();

    Icon::from_rgba(image.into_raw(), 22, 22).unwrap()
}
```

## Linux

### Build Dependencies
```bash
# Ubuntu/Debian
sudo apt-get install libgtk-3-dev libappindicator3-dev

# Fedora
sudo dnf install gtk3-devel libappindicator-gtk3-devel

# Arch
sudo pacman -S gtk3 libappindicator-gtk3
```

### Runtime Requirements
Modern GNOME removed system tray support. Users need:
- **GNOME**: "AppIndicator and KStatusNotifierItem Support" extension
- **KDE**: Works out of the box
- **XFCE/MATE**: Native support

### Fallback Strategy
```rust
// Provide alternative access when tray fails
fn main() {
    match TrayIconBuilder::new()
        .with_icon(icon)
        .build()
    {
        Ok(tray) => {
            println!("Tray icon created successfully");
        }
        Err(e) => {
            eprintln!("Failed to create tray icon: {}", e);
            eprintln!("Running in fallback mode...");
            // Start with window visible instead
        }
    }
}
```

### Linux Icon Paths
Some desktop environments prefer themed icons:
```rust
#[cfg(target_os = "linux")]
{
    // Try system theme icon first
    if let Ok(icon) = Icon::from_name("application-icon") {
        builder.with_icon(icon);
    } else {
        // Fall back to embedded icon
        builder.with_icon(load_icon(ICON_BYTES));
    }
}
```

## Cross-Platform Menu Behavior

| Feature | Windows | macOS | Linux |
|---------|---------|--------|-------|
| **Menu Trigger** | Right-click | Left-click | Usually right-click |
| **Checkboxes** | ✓ Supported | ✓ Supported | ✓ Supported |
| **Submenus** | ✓ Supported | ✓ Supported | ✓ Supported |
| **Icons in Menu** | ✓ Supported | ✓ Supported | ✗ Often ignored |
| **Separators** | ✓ Supported | ✓ Supported | ✓ Supported |
| **Disabled Items** | ✓ Greyed out | ✓ Greyed out | ✓ Greyed out |
| **Tooltips** | ✓ Supported | ✗ Not supported | ✓ Varies by DE |

## Platform Detection

```rust
#[cfg(target_os = "windows")]
const PLATFORM: &str = "windows";

#[cfg(target_os = "macos")]
const PLATFORM: &str = "macos";

#[cfg(target_os = "linux")]
const PLATFORM: &str = "linux";

fn configure_for_platform(builder: TrayIconBuilder) -> TrayIconBuilder {
    match PLATFORM {
        "windows" => {
            // Windows: tooltip and right-click menu
            builder.with_tooltip("Right-click for menu")
        },
        "macos" => {
            // macOS: no tooltip, left-click shows menu
            builder
        },
        "linux" => {
            // Linux: check desktop environment
            if std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .contains("GNOME")
            {
                println!("Warning: GNOME may require extensions for tray support");
            }
            builder.with_tooltip("Application menu")
        },
        _ => builder,
    }
}
```

## Platform-Specific Workarounds

### Windows: Balloon Notifications
```rust
#[cfg(target_os = "windows")]
{
    // Native Windows notifications
    tray_icon.show_notification(
        "Title",
        "Message body",
        Some(icon)
    );
}
```

### macOS: Template Images
For proper dark mode support:
```rust
#[cfg(target_os = "macos")]
{
    // Convert to template image (black pixels with alpha)
    let mut image = image::load_from_memory(ICON_BYTES)
        .unwrap()
        .into_rgba8();

    // Make all pixels black, preserve alpha
    for pixel in image.pixels_mut() {
        pixel[0] = 0;  // R
        pixel[1] = 0;  // G
        pixel[2] = 0;  // B
        // pixel[3] unchanged (alpha)
    }
}
```

### Linux: Desktop Entry
Create a `.desktop` file for better integration:
```ini
[Desktop Entry]
Type=Application
Name=My Tray App
Exec=/usr/bin/my-tray-app
Icon=my-tray-app
Categories=Utility;
X-GNOME-Autostart-enabled=true
```

## Testing Across Platforms

### GitHub Actions Matrix
```yaml
strategy:
  matrix:
    platform:
      - os: ubuntu-latest
        deps: sudo apt-get install -y libgtk-3-dev libappindicator3-dev
      - os: windows-latest
        deps: echo "No deps needed"
      - os: macos-latest
        deps: echo "No deps needed"

steps:
  - run: ${{ matrix.deps }}
  - run: cargo test
```

### Local Testing Tips
1. **Windows**: Test with/without admin rights
2. **macOS**: Test on multiple macOS versions (behavior changes)
3. **Linux**: Test on GNOME, KDE, and XFCE