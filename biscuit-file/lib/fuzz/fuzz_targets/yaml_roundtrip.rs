#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(yaml) = biscuit_file::Yaml::from_str(input) else {
        return;
    };

    // Exercise conversion paths
    let _ = yaml.as_json();
    let _ = yaml.as_toml();
    let _ = yaml.validate();
});
