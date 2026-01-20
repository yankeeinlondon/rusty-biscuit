# Menu System

Creating native system menus that integrate seamlessly with the OS.

## Basic Menu Structure

```rust
use tray_icon::menu::{Menu, MenuItem, MenuEvent};

// Create root menu
let menu = Menu::new();

// Add simple items
let open_i = MenuItem::new("Open", true, None);
let save_i = MenuItem::new("Save", true, None);
let quit_i = MenuItem::new("Quit", true, None);

// Build menu hierarchy
menu.append(&open_i).unwrap();
menu.append(&save_i).unwrap();
menu.append(&PredefinedMenuItem::separator()).unwrap();
menu.append(&quit_i).unwrap();
```

## Menu Item Types

### Standard MenuItem
```rust
// MenuItem::new(text, enabled, accelerator)
let item = MenuItem::new("Settings", true, None);

// With keyboard shortcut (display only)
let item = MenuItem::new("Copy", true, Some("Ctrl+C"));

// Disabled item (grayed out)
let status = MenuItem::new("Status: Connected", false, None);
```

### CheckMenuItem (Toggles)
```rust
use tray_icon::menu::CheckMenuItem;

// CheckMenuItem::new(text, enabled, checked, accelerator)
let dark_mode = CheckMenuItem::new("Dark Mode", true, false, None);
let auto_start = CheckMenuItem::new("Run on Startup", true, true, None);

// Toggle state in event handler
if event.id == dark_mode.id() {
    let is_checked = dark_mode.is_checked();
    dark_mode.set_checked(!is_checked);
}
```

### Submenu (Nested Menus)
```rust
use tray_icon::menu::Submenu;

// Create submenu
let view_menu = Submenu::new("View", true);

// Add items to submenu
let zoom_in = MenuItem::new("Zoom In", true, Some("Ctrl++"));
let zoom_out = MenuItem::new("Zoom Out", true, Some("Ctrl+-"));

view_menu.append(&zoom_in).unwrap();
view_menu.append(&zoom_out).unwrap();

// Add submenu to main menu
menu.append(&view_menu).unwrap();
```

### Predefined Items
```rust
use tray_icon::menu::PredefinedMenuItem;

// Separator line
menu.append(&PredefinedMenuItem::separator()).unwrap();

// Platform-specific items (macOS)
#[cfg(target_os = "macos")]
{
    menu.append(&PredefinedMenuItem::quit()).unwrap();
    menu.append(&PredefinedMenuItem::about("My App")).unwrap();
}
```

## Complex Menu Example

```rust
fn build_app_menu() -> (Menu, MenuItems) {
    let menu = Menu::new();

    // Status section
    let status = MenuItem::new("Status: Connected", false, None);
    menu.append(&status).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Connection submenu
    let connection = Submenu::new("Connection", true);
    let connect = MenuItem::new("Connect", true, None);
    let disconnect = MenuItem::new("Disconnect", false, None);
    let auto_connect = CheckMenuItem::new("Auto-connect", true, true, None);

    connection.append(&connect).unwrap();
    connection.append(&disconnect).unwrap();
    connection.append(&PredefinedMenuItem::separator()).unwrap();
    connection.append(&auto_connect).unwrap();

    menu.append(&connection).unwrap();

    // Server selection submenu
    let servers = Submenu::new("Select Server", true);
    let us_east = MenuItem::new("US East", true, None);
    let eu_west = MenuItem::new("EU West", true, None);
    let asia = MenuItem::new("Asia Pacific", true, None);

    servers.append_items(&[&us_east, &eu_west, &asia]).unwrap();
    menu.append(&servers).unwrap();

    // Settings and quit
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    let settings = MenuItem::new("Settings...", true, None);
    let quit = MenuItem::new("Quit", true, None);

    menu.append(&settings).unwrap();
    menu.append(&quit).unwrap();

    // Return menu and items for event handling
    let items = MenuItems {
        connect,
        disconnect,
        auto_connect,
        servers: vec![us_east, eu_west, asia],
        settings,
        quit,
    };

    (menu, items)
}
```

## Event Handling

### Basic Pattern
```rust
// Store menu IDs for matching
struct MenuIds {
    quit_id: MenuId,
    settings_id: MenuId,
    connect_id: MenuId,
}

// In event loop
let menu_channel = MenuEvent::receiver();

if let Ok(event) = menu_channel.try_recv() {
    match event.id {
        id if id == ids.quit_id => {
            elwt.exit();
        },
        id if id == ids.settings_id => {
            open_settings_window();
        },
        id if id == ids.connect_id => {
            connect_to_server();
        },
        _ => {}
    }
}
```

### Managing Multiple Items
```rust
use std::collections::HashMap;

// Map IDs to actions
let mut actions: HashMap<MenuId, Box<dyn Fn()>> = HashMap::new();

actions.insert(quit_i.id().clone(), Box::new(|| {
    std::process::exit(0);
}));

actions.insert(open_i.id().clone(), Box::new(|| {
    webbrowser::open("https://example.com").unwrap();
}));

// In event loop
if let Ok(event) = menu_channel.try_recv() {
    if let Some(action) = actions.get(&event.id) {
        action();
    }
}
```

## Dynamic Menu Updates

### Changing Text
```rust
// Keep reference to menu item
let status_item = MenuItem::new("Status: Disconnected", false, None);

// Update later
status_item.set_text("Status: Connected");
status_item.set_enabled(true);
```

### Enabling/Disabling Items
```rust
// Initially disabled
let disconnect_btn = MenuItem::new("Disconnect", false, None);

// Enable after connection
fn on_connected() {
    disconnect_btn.set_enabled(true);
    connect_btn.set_enabled(false);
}
```

### Adding/Removing Items
```rust
// Add item dynamically
let new_item = MenuItem::new("New Server", true, None);
menu.append(&new_item).unwrap();

// Remove item
menu.remove(&old_item).unwrap();
```

## Menu Best Practices

### 1. Group Related Items
```rust
// Good: Logical grouping with separators
menu.append(&file_menu).unwrap();
menu.append(&edit_menu).unwrap();
menu.append(&PredefinedMenuItem::separator()).unwrap();
menu.append(&settings).unwrap();
menu.append(&PredefinedMenuItem::separator()).unwrap();
menu.append(&quit).unwrap();
```

### 2. Use Standard Ordering
- Status/info items at top
- Main actions in middle
- Settings near bottom
- Quit at very bottom

### 3. Keyboard Shortcuts
Show shortcuts even if not handled by menu:
```rust
let copy = MenuItem::new("Copy", true, Some("Ctrl+C"));
let paste = MenuItem::new("Paste", true, Some("Ctrl+V"));
```

### 4. Disable Unavailable Actions
```rust
// Better UX than hiding items
let paste_item = MenuItem::new("Paste", false, Some("Ctrl+V"));

// Enable when clipboard has content
if clipboard_has_content() {
    paste_item.set_enabled(true);
}
```

## Platform Menu Differences

### Windows
```rust
#[cfg(target_os = "windows")]
{
    // Right-click shows menu by default
    // Can intercept to show custom menu position
}
```

### macOS
```rust
#[cfg(target_os = "macos")]
{
    // Left-click shows menu
    // Consider using macOS-style menu items
    menu.append(&PredefinedMenuItem::about("My App")).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&PredefinedMenuItem::services()).unwrap();
}
```

### Linux
```rust
#[cfg(target_os = "linux")]
{
    // Behavior varies by desktop environment
    // Test on GNOME, KDE, XFCE
}
```

## Common Gotchas

### Menu Not Showing
Keep menu alive:
```rust
// BAD: Menu dropped immediately
{
    let menu = Menu::new();
    tray.set_menu(Some(Box::new(menu)));
} // menu dropped here!

// GOOD: Keep menu in scope
let menu = Menu::new();
let _tray = TrayIconBuilder::new()
    .with_menu(Box::new(menu))
    .build()
    .unwrap();
```

### ID Comparison
Store IDs for reliable comparison:
```rust
// Store the ID, not the item
let quit_id = quit_item.id().clone();

// Compare in handler
if event.id == quit_id {
    // Handle quit
}
```

### Thread Safety
Menu updates from other threads:
```rust
use std::sync::{Arc, Mutex};

let status_item = Arc::new(Mutex::new(status_item));

// From another thread
let item = status_item.clone();
std::thread::spawn(move || {
    item.lock().unwrap().set_text("Updated from thread");
});
```