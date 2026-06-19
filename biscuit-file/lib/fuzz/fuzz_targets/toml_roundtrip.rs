#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(toml) = biscuit_file::Toml::from_str(input) else {
        return;
    };

    // Exercise conversion paths
    let _ = toml.as_json();
    let _ = toml.as_yaml();
    let _ = toml.to_toml_string();
    let _ = toml.validate();
});
