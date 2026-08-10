use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tauri_config() -> Value {
    let path = manifest_dir().join("tauri.conf.json");
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

#[test]
fn configured_desktop_icons_exist_and_match_their_formats() {
    let config = tauri_config();
    let icons = config
        .pointer("/bundle/icon")
        .and_then(Value::as_array)
        .expect("bundle.icon must be an array");
    let mut formats = BTreeSet::new();

    for icon in icons {
        let relative = icon.as_str().expect("bundle.icon entries must be strings");
        let path = manifest_dir().join(relative);
        let bytes = fs::read(&path).unwrap_or_else(|error| {
            panic!("configured icon {} is unreadable: {error}", path.display())
        });
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .expect("configured icons must have file extensions");
        formats.insert(extension.to_owned());

        match extension {
            "png" => assert!(
                bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "{} is not a PNG file",
                path.display()
            ),
            "ico" => assert!(
                bytes.starts_with(b"\0\0\x01\0"),
                "{} is not a Windows icon file",
                path.display()
            ),
            "icns" => assert!(
                bytes.starts_with(b"icns"),
                "{} is not a macOS icon file",
                path.display()
            ),
            other => panic!("unsupported configured icon format: {other}"),
        }
    }

    assert_eq!(
        formats,
        BTreeSet::from(["icns".to_owned(), "ico".to_owned(), "png".to_owned()])
    );
}

#[test]
fn configured_frontend_contains_an_entry_point() {
    let config = tauri_config();
    let relative = config
        .pointer("/build/frontendDist")
        .and_then(Value::as_str)
        .expect("build.frontendDist must be a string");
    let entry_point = manifest_dir().join(relative).join("index.html");

    assert!(
        entry_point.is_file(),
        "configured frontend entry point is missing: {}",
        entry_point.display()
    );
}
