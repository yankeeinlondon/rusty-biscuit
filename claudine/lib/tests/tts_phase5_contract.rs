use std::fs;
use std::path::{Path, PathBuf};

fn area_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine lib should be nested in the package area")
        .to_path_buf()
}

#[test]
fn shipped_claudine_artifacts_enable_native_detached_audio() {
    let area = area_root();
    for manifest in [area.join("lib/Cargo.toml"), area.join("cli/Cargo.toml")] {
        let source = fs::read_to_string(&manifest).expect("manifest should be readable");
        let parsed: toml::Value = toml::from_str(&source).expect("manifest should parse");
        let playa_features = parsed["dependencies"]["playa"]["features"]
            .as_array()
            .expect("playa dependency should declare features");
        assert!(
            playa_features
                .iter()
                .any(|feature| feature.as_str() == Some("native-playback")),
            "{} must enable playa/native-playback",
            manifest.display()
        );
        assert!(
            !playa_features
                .iter()
                .any(|feature| feature.as_str() == Some("sfx-native-audio")),
            "{} must not enable sfx-native-audio",
            manifest.display()
        );
        let native = parsed["package"]["metadata"]["ci"]["native"]["ubuntu-latest"]
            .as_array()
            .expect("Linux CI native packages should be declared");
        assert!(
            native
                .iter()
                .any(|package| package.as_str() == Some("libasound2-dev")),
            "{} must provision ALSA headers",
            manifest.display()
        );
    }

    let main = fs::read_to_string(area.join("cli/src/main.rs")).expect("main should be readable");
    assert!(
        main.contains("biscuit_speaks::run_if_worker"),
        "Claudine must install the biscuit-speaks preparation seam"
    );
    assert!(
        main.contains("playa::detached::run_if_worker"),
        "Claudine must install the Playa scheduler and delegate seam"
    );
    let main_body = &main[main.find("fn main() -> Result<()>").expect("main should exist")..];
    let worker = main_body
        .find("run_audio_worker_if_requested")
        .expect("main must intercept audio worker modes");
    for later in [
        "rustls::crypto::ring::default_provider",
        "color_eyre::install",
        "completion::maybe_complete",
        "LaunchContext::capture",
    ] {
        if let Some(index) = main_body.find(later) {
            assert!(worker < index, "audio worker interception must precede {later}");
        }
    }
}

#[test]
fn notification_recipes_keep_their_background_cli_contract() {
    let area = area_root();
    let root = area
        .parent()
        .expect("package area should be nested in the workspace");
    let notify = fs::read_to_string(root.join("just/notify.just"))
        .expect("notification recipes should be readable");
    assert!(notify.contains("so-you-say \"{{args}}\" --background"));
    assert!(notify.contains("playa effect \"{{effect}}\" --background"));
}
