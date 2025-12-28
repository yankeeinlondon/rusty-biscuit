The `arboard` (Async Rust Board) crate is the standard Rust library for cross-platform clipboard access (supporting text and images). Because it handles the "raw" transfer of data to and from the OS, it is almost always paired with libraries that **format, process, or trigger** that data.

The most common libraries integrated with `arboard` are **`image`** (for visual data) and **`serde_json`** (for structured data).

---

### 1. `image`

The `image` crate is the most common companion for `arboard`. While `arboard` provides the bridge to the system clipboard, it requires image data in a specific raw RGBA pixel format. The `image` crate is used to decode common formats (PNG, JPEG, etc.) into the raw buffers `arboard` understands, or to encode raw clipboard data back into a file.

**Why they are used together:**
`arboard` cannot "read a PNG file"; it can only take a flat `Vec<u8>` of pixels. The `image` crate handles the complex math of converting a compressed image file into those pixels.

**Code Example:**
This example shows how to load a `.png` file and copy it to the system clipboard.

````rust
use arboard::{Clipboard, ImageData};
use image::GenericImageView;
use std::borrow::Cow;

fn main() -> anyhow::Result<()> {
    // 1. Use the 'image' crate to open a file
    let img = image::open("input.png")?;
    let (width, height) = img.dimensions();
    let rgba_pixels = img.to_rgba8().into_raw();

    // 2. Prepare the data for 'arboard'
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::from(rgba_pixels),
    };

    // 3. Use 'arboard' to put it on the clipboard
    let mut clipboard = Clipboard::new()?;
    clipboard.set_image(img_data)?;

    println!("Image copied to clipboard!");
    Ok(())
}
````

---

### 2. `serde_json` (and `serde`)

When developers want to copy complex data structures (like a user profile, a configuration block, or a spreadsheet row) to the clipboard, they use `serde_json` to serialize that data into a string.

**Why they are used together:**
The system clipboard primarily understands "Plain Text." To move a Rust `struct` through the clipboard, you must transform it into a string. `serde_json` provides the serialization, and `arboard` provides the transport. This is very common in developer tools or internal productivity apps.

**Code Example:**
This example shows how to copy a Rust struct to the clipboard as a JSON string so it can be pasted into another application (like a text editor or a web tool).

````rust
use arboard::Clipboard;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct AppConfig {
    theme: String,
    font_size: u32,
    enable_telemetry: bool,
}

fn main() -> anyhow::Result<()> {
    let config = AppConfig {
        theme: "Dark".to_string(),
        font_size: 14,
        enable_telemetry: false,
    };

    // 1. Serialize the struct to a JSON string using serde_json
    let json_text = serde_json::to_string_pretty(&config)?;

    // 2. Use arboard to copy the string
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(json_text)?;

    println!("Config copied as JSON!");
    Ok(())
}
````

---

### 3. `eframe` / `egui`

`egui` is a popular immediate-mode GUI library for Rust. While `egui` has a basic built-in clipboard handler, developers frequently use `arboard` alongside it for more robust, cross-platform clipboard support—especially when handling images or specific edge cases where the GUI framework's default handler is insufficient.

**Why they are used together:**
In a GUI application, clipboard actions are usually triggered by user interaction (like clicking a "Copy ID" button). `arboard` is used inside the GUI's update loop to perform the actual OS-level operation.

**Code Example:**
A simple UI snippet showing how `arboard` is invoked during a button click event in an `egui` app.

````rust
use arboard::Clipboard;
use eframe::egui;

struct MyDataApp {
    clipboard: Clipboard, // Keep the clipboard handle in the app state
}

impl eframe::App for MyDataApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Copy 'Hello World' to Clipboard").clicked() {
                // Trigger arboard action on click
                if self.clipboard.set_text("Hello World").is_ok() {
                    println!("Copied!");
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    let clipboard = Clipboard::new().expect("Failed to init clipboard");
    
    eframe::run_native(
        "Clipboard App",
        native_options,
        Box::new(|_| Box::new(MyDataApp { clipboard })),
    )
}
````

### Summary

|Library|Purpose with `arboard`|Key Integration Point|
|:------|:---------------------|:--------------------|
|**`image`**|Image Processing|Converts compressed files to/from raw `ImageData` bytes.|
|**`serde_json`**|Data Formatting|Converts Rust structs to/from strings for text-based transfer.|
|**`egui`**|User Interface|Provides the buttons and UI events that trigger clipboard actions.|