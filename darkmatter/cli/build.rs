fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Windows executables reserve a 1 MiB main-thread stack by default, while
    // Unix platforms commonly provide 8 MiB. Clap's generated `Cli` command
    // graph exceeds the Windows default before argument dispatch, including
    // for lightweight paths such as `md --version`.
    let linker_arg = match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => "/STACK:8388608",
        _ => "-Wl,--stack,8388608",
    };
    println!("cargo:rustc-link-arg-bin=md={linker_arg}");
}
