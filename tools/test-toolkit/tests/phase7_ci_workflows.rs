use std::{fs, path::PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

fn workflow(name: &str) -> String {
    let path = repo_root().join(".github/workflows").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn assert_required_area_caller(source: &str, area: &str, check_args: &[&str], expects_l2: bool) {
    assert!(
        source.contains("uses: ./.github/workflows/_area-ci.yml"),
        "{area} must use the durable shared area-CI workflow"
    );
    assert!(
        source.contains(&format!("area: {area}")),
        "{area} caller must identify its package area"
    );
    for package in check_args {
        assert!(
            source.contains(package),
            "{area} all-target check must include {package}"
        );
    }
    assert!(
        source.contains(r#"full-os: '["ubuntu-latest", "windows-latest"]'"#),
        "{area} must run L1 on native Linux and Windows"
    );
    assert!(
        source.contains(r#"check-os: '["macos-latest"]'"#),
        "{area} must compile all targets on native macOS"
    );
    assert!(
        source.contains("soft-os: '[]'"),
        "{area} Windows evidence must be required"
    );
    assert_eq!(
        source.contains("l2: true"),
        expects_l2,
        "{area} has the wrong Linux L2 policy"
    );
}

#[test]
fn shared_area_ci_treats_rust_warnings_as_failures() {
    let shared = workflow("_area-ci.yml");
    assert!(
        shared.contains(r#"RUSTFLAGS: "-D warnings""#),
        "shared compile and test jobs must reject Rust warnings"
    );
    assert!(
        shared.contains(r#"run: cd "${{ inputs.area }}" && just lint"#),
        "shared area coverage must execute each area's lint and documentation guards"
    );
}

#[test]
fn sniff_native_matrix_remains_complete() {
    let source = workflow("test.yml");
    assert!(
        source.contains("os: [macos-latest, ubuntu-latest, windows-latest]"),
        "Sniff must retain native macOS, Linux, and Windows evidence"
    );
    assert!(
        source.contains("cargo check --color=never -p sniff --all-targets --features remote"),
        "Sniff must compile all remote-enabled targets"
    );
    assert!(
        source.contains("run: cd sniff && just test"),
        "Sniff must execute its native L1 suite"
    );
    assert!(
        source.contains("runner.os != 'Windows'")
            && source.contains("run: cd sniff && just test-l2"),
        "Sniff must retain Unix managed-L2 evidence"
    );
    assert!(
        source.contains(r#"RUSTFLAGS: "-D warnings""#),
        "Sniff compile and test output must reject Rust warnings"
    );
}

#[test]
fn darkmatter_promotes_windows_and_enables_linux_slow_tiers() {
    let source = workflow("darkmatter-tests.yml");
    assert_required_area_caller(
        &source,
        "darkmatter",
        &["-p darkmatter", "-p darkmatter-cli", "-p dmls"],
        true,
    );
    assert!(
        source.contains("browser: true"),
        "Darkmatter must run its headless-browser tier on Linux"
    );
}

#[test]
fn biscuit_file_has_durable_required_native_coverage() {
    let source = workflow("biscuit-file-tests.yml");
    assert_required_area_caller(
        &source,
        "biscuit-file",
        &["-p biscuit-file", "-p biscuit-file-cli"],
        false,
    );
}

#[test]
fn claudine_has_required_native_coverage_and_preserves_specialized_gates() {
    let source = workflow("claudine-tests.yml");
    assert_required_area_caller(
        &source,
        "claudine",
        &[
            "-p claudine-catalog-types",
            "-p claudine",
            "-p claudine-contract",
            "-p claudine-cli",
            "-p claudine-gen",
        ],
        true,
    );
    assert!(
        source.contains("just test-gen"),
        "Claudine must preserve generator and signal drift verification"
    );

    let windows = workflow("claudine-windows-ctrl-c.yml");
    assert!(
        windows.contains("windows_ctrl_c_verification_record"),
        "Claudine must preserve the native Windows Ctrl+C runtime test"
    );
    assert!(
        windows.contains(r#"RUSTFLAGS: "-D warnings""#),
        "the specialized Windows runtime job must reject Rust warnings"
    );
}
