#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(json5) = biscuit_file::Json5::from_str(input) else {
        return;
    };

    // Exercise conversion paths
    let _ = json5.as_json();
    let _ = json5.as_yaml();
    let _ = json5.as_toml();
    let _ = json5.as_json5();
    let _ = json5.as_json5_compact();
});
