# Basic Setup

Getting started with system tray applications in Rust.

## Dependencies

The recommended stack combines three key crates:

```toml
[dependencies]
tray-icon = "0.19"   # Tray abstraction (by Tauri team)
winit = "0.29"       # Event loop and windowing
webbrowser = "1.0"   # Cross-platform URL opening
image = "0.25"       # Icon loading
```

## Complete Minimal Example

This creates a tray icon that opens a URL when clicked:

```rust
use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    // 1. Create the event loop
    let event_loop = EventLoop::new().unwrap();

    // 2. Load icon from file
    let icon = load_icon("icon.png");

    // 3. Create menu (right-click options)
    let tray_menu = Menu::new();
    let quit_i = MenuItem::new("Quit", true, None);
    tray_menu.append(&quit_i).unwrap();

    // 4. Build tray icon
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Click to open website")
        .with_icon(icon)
        .build()
        .unwrap();

    // 5. Setup event channels
    let menu_channel = tray_icon::menu::MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    // 6. Run event loop
    event_loop.run(move |_event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        // Handle tray icon clicks
        if let Ok(event) = tray_channel.try_recv() {
            if event.event == tray_icon::ClickType::Left {
                println!("Opening URL...");
                let _ = webbrowser::open("https://www.rust-lang.org");
            }
        }

        // Handle menu selections
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == quit_i.id() {
                println!("Quitting...");
                elwt.exit();
            }
        }
    }).unwrap();
}

// Helper to load icon
fn load_icon(path: &str) -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to create icon")
}
```

## Embedding Icons

Ship a single executable by embedding the icon at compile time:

```rust
// Embed icon bytes at compile time
const ICON_BYTES: &[u8] = include_bytes!("icon.png");

fn main() {
    // Load from embedded bytes instead of file
    let icon = load_icon_from_bytes(ICON_BYTES);

    // Rest of code remains the same...
}

fn load_icon_from_bytes(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes)
        .expect("Failed to parse icon data")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    Icon::from_rgba(rgba, width, height)
        .expect("Failed to create icon")
}
```

## Event Handling

### Tray Click Types

```rust
use tray_icon::ClickType;

match event.event {
    ClickType::Left => {
        // Single left click
    },
    ClickType::Right => {
        // Right click (usually shows menu automatically)
    },
    ClickType::Double => {
        // Double click
    },
}
```

### Menu Events

Each menu item has a unique ID for matching:

```rust
// Store menu item references
let settings_i = MenuItem::new("Settings", true, None);
let about_i = MenuItem::new("About", true, None);

// In event loop
if let Ok(event) = menu_channel.try_recv() {
    if event.id == settings_i.id() {
        // Open settings
    } else if event.id == about_i.id() {
        // Show about dialog
    }
}
```

## Basic Patterns

### Toggle Visibility on Click

```rust
// Track visibility state
let mut visible = false;

if event.event == ClickType::Left {
    visible = !visible;
    if visible {
        // Show your window/UI
    } else {
        // Hide your window/UI
    }
}
```

### Dynamic Tooltip Updates

```rust
// Keep reference to tray icon
let tray_icon = TrayIconBuilder::new()
    .with_tooltip("Status: Disconnected")
    .with_icon(icon)
    .build()
    .unwrap();

// Update later
tray_icon.set_tooltip(Some("Status: Connected"));
```

### Multiple Actions Menu

```rust
let menu = Menu::new();

// Group similar actions
let file_menu = Submenu::new("File", true);
file_menu.append(&MenuItem::new("Open", true, None)).unwrap();
file_menu.append(&MenuItem::new("Save", true, None)).unwrap();

// Add separator
menu.append(&file_menu).unwrap();
menu.append(&PredefinedMenuItem::separator()).unwrap();
menu.append(&MenuItem::new("Quit", true, None)).unwrap();
```

## Common Issues

### Icon Not Appearing

1. **File not found**: Use absolute paths or embed with `include_bytes!`
2. **Wrong format**: Use PNG for best compatibility
3. **Size issues**: Keep icons reasonable (256x256 or smaller)

### Event Loop Blocking

```rust
// BAD: Blocks the event loop
event_loop.run(move |event, elwt| {
    std::thread::sleep(Duration::from_secs(1)); // Don't do this!
});

// GOOD: Use ControlFlow::WaitUntil
elwt.set_control_flow(ControlFlow::WaitUntil(
    Instant::now() + Duration::from_secs(1)
));
```

### Menu Not Showing

Keep menu references alive:
```rust
// The menu must outlive the tray icon
let menu = Menu::new();
let _tray = TrayIconBuilder::new()
    .with_menu(Box::new(menu))
    .build()
    .unwrap();
// Don't drop _tray!
```