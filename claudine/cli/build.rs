fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Windows executables reserve a 1 MiB main-thread stack by default, while
    // Unix platforms commonly provide 8 MiB. A debug-build compose of a proxied,
    // looping document peaks near that 1 MiB on the main thread (the harness
    // loop and composition frames run to tens of KiB each), so Windows
    // overflowed where macOS and Linux did not.
    let linker_arg = match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => "/STACK:8388608",
        _ => "-Wl,--stack,8388608",
    };
    println!("cargo:rustc-link-arg-bin=claudine={linker_arg}");
}
