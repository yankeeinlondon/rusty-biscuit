---
name: rust-tray-icon
description: Expert knowledge for building system tray applications in Rust using tray-icon, winit, and egui - includes platform-specific setup, native menus, auto-launch, and GUI integration
---

# Rust System Tray Applications

Comprehensive expertise for creating cross-platform system tray applications in Rust using the `tray-icon` crate ecosystem.

## Core Principles

- **Use `tray-icon` by the Tauri team** - Most robust and maintained solution
- **Pair with `winit` for event loops** - Handles OS-specific windowing requirements
- **Embed icons at compile time** - Ship single executables with `include_bytes!`
- **Native menus over custom UI** - Better OS integration and accessibility
- **Handle platform differences explicitly** - Each OS has unique tray behaviors

## Quick Start

### Dependencies
```toml
[dependencies]
tray-icon = "0.19"
winit = "0.29"
webbrowser = "1.0"
image = "0.25"
```

### Minimal Example
```rust
use tray_icon::{menu::Menu, Icon, TrayIconBuilder};
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let icon = load_icon(include_bytes!("icon.png"));

    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("Click me!")
        .with_icon(icon)
        .build()
        .unwrap();

    event_loop.run(move |_event, elwt| {
        // Handle events
    }).unwrap();
}
```

## Platform Requirements

| Platform | Build Dependencies | Runtime Requirements | Icon Format |
|----------|-------------------|---------------------|-------------|
| **Windows** | None | None | ICO/PNG (256x256) |
| **macOS** | None | Main thread only | PNG (22x22) |
| **Linux** | `libgtk-3-dev`, `libappindicator3-dev` | GNOME extension | PNG (varies) |

## Key Patterns

### Hide on Close (Windows)
```rust
#![windows_subsystem = "windows"]  // Hide console

// In event handler:
Event::WindowEvent {
    event: WindowEvent::CloseRequested,
    ..
} => {
    window.set_visible(false);  // Don't exit!
}
```

### Icon Loading Helper
```rust
fn load_icon(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes)
        .expect("Failed to parse icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height)
        .expect("Failed to create icon")
}
```

## Detailed Topics

- [Basic Setup](./basic-setup.md) - Dependencies, minimal examples, event handling
- [Platform Specifics](./platform-specifics.md) - OS differences, gotchas, requirements
- [Menu System](./menu-system.md) - Native menus, checkboxes, submenus, dynamic updates
- [Advanced Features](./advanced-features.md) - GUI integration, auto-launch, cross-compilation

## Common Use Cases

- **URL Launcher** - Click tray to open website
- **Settings Panel** - Hide/show configuration window
- **Background Service** - Monitor with status updates
- **Quick Actions** - Right-click menu for common tasks

## Resources

- [tray-icon Documentation](https://docs.rs/tray-icon)
- [winit Documentation](https://docs.rs/winit)
- [Tauri GitHub](https://github.com/tauri-apps/tray-icon)