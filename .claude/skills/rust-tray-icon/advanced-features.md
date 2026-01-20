# Advanced Features

Advanced patterns and integrations for system tray applications.

## GUI Integration with egui

Replace raw `winit` with `eframe` for easy GUI:

### Dependencies
```toml
[dependencies]
tray-icon = "0.19"
eframe = "0.29"  # Includes winit
egui = "0.29"
image = "0.25"
```

### Complete Example
```rust
use eframe::egui;
use tray_icon::{menu::*, Icon, TrayIconBuilder, TrayIconEvent};

fn main() -> eframe::Result<()> {
    // Setup tray before GUI
    let icon = load_icon(include_bytes!("icon.png"));
    let menu = Menu::new();
    let show_i = MenuItem::new("Show", true, None);
    let quit_i = MenuItem::new("Quit", true, None);

    menu.append(&show_i).unwrap();
    menu.append(&quit_i).unwrap();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()
        .unwrap();

    // Configure window
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_visible(false),  // Start hidden
        ..Default::default()
    };

    // Run app
    eframe::run_native(
        "My Tray App",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(MyApp {
                show_id: show_i.id().clone(),
                quit_id: quit_i.id().clone(),
                url: "https://rust-lang.org".to_string(),
            }))
        }),
    )
}

struct MyApp {
    show_id: MenuId,
    quit_id: MenuId,
    url: String,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle tray events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_id {
                frame.set_visible(true);
            } else if event.id == self.quit_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Handle window close (hide instead)
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            frame.set_visible(false);
        }

        // Draw GUI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Settings");

            ui.label("URL to open:");
            ui.text_edit_singleline(&mut self.url);

            if ui.button("Open Now").clicked() {
                let _ = webbrowser::open(&self.url);
            }

            ui.separator();

            if ui.button("Hide to Tray").clicked() {
                frame.set_visible(false);
            }
        });
    }
}
```

## Auto-Launch on Startup

Use the `auto-launch` crate:

### Dependencies
```toml
[dependencies]
auto-launch = "0.5"
```

### Implementation
```rust
use auto_launch::AutoLaunchBuilder;
use std::env;

fn setup_auto_launch() {
    let app_path = env::current_exe()
        .expect("Failed to get exe path");

    let auto = AutoLaunchBuilder::new()
        .set_app_name("MyTrayApp")  // Must be unique
        .set_app_path(app_path.to_str().unwrap())
        .set_use_launch_agent(true)  // Required for macOS
        .build()
        .unwrap();

    // Enable if not already
    if !auto.is_enabled().unwrap() {
        auto.enable().unwrap();
        println!("Auto-launch enabled!");
    }
}
```

### Toggle in Menu
```rust
// Create toggle menu item
let auto_start = CheckMenuItem::new("Run on Startup", true, false, None);

// Check current state
let auto = create_auto_launch();
auto_start.set_checked(auto.is_enabled().unwrap());

// In event handler
if event.id == auto_start.id() {
    if auto.is_enabled().unwrap() {
        auto.disable().unwrap();
        auto_start.set_checked(false);
    } else {
        auto.enable().unwrap();
        auto_start.set_checked(true);
    }
}
```

### Platform Details
- **Windows**: Registry key in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- **macOS**: `.plist` file in `~/Library/LaunchAgents`
- **Linux**: `.desktop` file in `~/.config/autostart`

## Cross-Compilation

### GitHub Actions Workflow
```yaml
name: Build and Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build - ${{ matrix.platform.os_name }}
    runs-on: ${{ matrix.platform.os }}
    strategy:
      matrix:
        platform:
          - os_name: Linux-x86_64
            os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
            bin: my-app
            name: my-app-linux.tar.gz
          - os_name: Windows-x86_64
            os: windows-latest
            target: x86_64-pc-windows-msvc
            bin: my-app.exe
            name: my-app-windows.zip
          - os_name: macOS-x86_64
            os: macos-latest
            target: x86_64-apple-darwin
            bin: my-app
            name: my-app-macos.tar.gz

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform.target }}

      - name: Install Linux Dependencies
        if: matrix.platform.os == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libappindicator3-dev

      - name: Build
        run: cargo build --release --target ${{ matrix.platform.target }}

      - name: Package (Unix)
        if: matrix.platform.os != 'windows-latest'
        run: |
          cd target/${{ matrix.platform.target }}/release
          tar czf ../../../${{ matrix.platform.name }} ${{ matrix.platform.bin }}

      - name: Package (Windows)
        if: matrix.platform.os == 'windows-latest'
        run: |
          cd target/${{ matrix.platform.target }}/release
          7z a ../../../${{ matrix.platform.name }} ${{ matrix.platform.bin }}

      - name: Upload Release
        uses: softprops/action-gh-release@v1
        with:
          files: ${{ matrix.platform.name }}
```

## Hide on Close Pattern

Keep app running when window closes:

```rust
// With winit
Event::WindowEvent {
    event: WindowEvent::CloseRequested,
    ..
} => {
    window.set_visible(false);
    // Don't call elwt.exit()!
}

// With egui/eframe
if ctx.input(|i| i.viewport().close_requested()) {
    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    frame.set_visible(false);
}
```

## Launching External Applications

### Open Browser/PWA
```rust
// Standard browser
webbrowser::open("https://example.com").unwrap();

// PWA (Progressive Web App)
use std::process::Command;

#[cfg(target_os = "windows")]
Command::new("cmd")
    .args(["/C", "start", "chrome", "--app=https://example.com"])
    .spawn()
    .unwrap();

#[cfg(target_os = "macos")]
Command::new("open")
    .args(["-na", "Google Chrome", "--args", "--app=https://example.com"])
    .spawn()
    .unwrap();

#[cfg(target_os = "linux")]
Command::new("google-chrome")
    .arg("--app=https://example.com")
    .spawn()
    .unwrap();
```

### Launch Native Apps
```rust
#[cfg(target_os = "windows")]
Command::new("calc.exe").spawn().unwrap();

#[cfg(target_os = "macos")]
Command::new("open")
    .args(["-a", "Calculator"])
    .spawn()
    .unwrap();

#[cfg(target_os = "linux")]
Command::new("gnome-calculator").spawn().unwrap();
```

## System Integration

### Notifications
```rust
// Basic notification support
#[cfg(target_os = "windows")]
tray_icon.show_notification(
    "Title",
    "Message",
    Some(icon)
);

// For cross-platform notifications, use notify-rust
use notify_rust::Notification;

Notification::new()
    .summary("My App")
    .body("Task completed!")
    .icon("my-app")
    .show()
    .unwrap();
```

### Single Instance
Ensure only one instance runs:

```rust
use single_instance::SingleInstance;

fn main() {
    let instance = SingleInstance::new("my-tray-app").unwrap();

    if !instance.is_single() {
        eprintln!("Another instance is already running!");
        // Optional: send message to existing instance
        return;
    }

    // Continue with normal startup...
}
```

### Global Hotkeys
```rust
use global_hotkey::{GlobalHotKey, HotKeyManager};

let manager = HotKeyManager::new().unwrap();

// Register Ctrl+Shift+T
let hotkey = GlobalHotKey::new(
    Some(Modifiers::CONTROL | Modifiers::SHIFT),
    Code::KeyT
);

manager.register(hotkey).unwrap();

// In event loop
if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
    if event.id == hotkey.id() {
        // Toggle window visibility
    }
}
```

## Performance Tips

### 1. Lazy Window Creation
```rust
let mut window: Option<Window> = None;

// Only create when needed
if show_window && window.is_none() {
    window = Some(create_window(&event_loop));
}
```

### 2. Efficient Event Handling
```rust
// Use try_recv to avoid blocking
while let Ok(event) = channel.try_recv() {
    handle_event(event);
}
```

### 3. Resource Cleanup
```rust
impl Drop for MyApp {
    fn drop(&mut self) {
        // Clean up system resources
        if let Some(auto_launch) = &self.auto_launch {
            let _ = auto_launch.disable();
        }
    }
}
```