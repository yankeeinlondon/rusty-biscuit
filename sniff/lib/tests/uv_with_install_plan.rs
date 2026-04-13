//! Integration test for the UvWithInstall auto-append + selection flow.
//!
//! Fabricates `HostCapabilities` permutations and asserts end-to-end plan
//! shape for a program declaring only `[Pip("aider-chat")]`. This guards the
//! scenario table in `sniff/features/2026-04-11-uv-with-install/spec.md`
//! from regressing.

use sniff::os::OsType;
use sniff::programs::{
    HostCapabilities, InstallPlanReason, InstallationMethod, ProgramInfo, ProgramMetadata,
    VersionFlag, VersionParseStrategy, build_install_plan,
};

static PIP_ONLY: ProgramInfo = ProgramInfo {
    binary_name: "aider",
    display_name: "aider",
    description: "AI pair programming",
    website: "https://aider.chat",
    version_flag: VersionFlag::Long,
    parse_strategy: VersionParseStrategy::FirstLine,
    version_regex: None,
    version_prefix: None,
    alternate_binary_names: &[],
    os_availability: &[],
    repo: None,
    installation_methods: &[InstallationMethod::Pip("aider-chat")],
};

struct FakeProgram;
impl ProgramMetadata for FakeProgram {
    fn info(&self) -> &'static ProgramInfo {
        &PIP_ONLY
    }
}

fn host_linux(pip: bool, uv: bool, has_bash: bool) -> HostCapabilities {
    let lang = format!(r#"{{"pip": {}, "uv": {}}}"#, pip, uv);
    HostCapabilities {
        os_type: OsType::Linux,
        lang_pkg_mgrs: serde_json::from_str(&lang).unwrap(),
        has_bash,
        ..HostCapabilities::default()
    }
}

#[test]
fn uv_installed_selects_bootstrap_for_pip_only_program() {
    // host has uv only → Pip blocked by UvPreferredOverPip, synthesized
    // UvWithInstall wins.
    let host = host_linux(false, true, true);
    let plan = build_install_plan(&FakeProgram, &host);
    let chosen = plan.chosen().expect("chosen");
    assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(_)));
}

#[test]
fn pip_installed_uv_absent_selects_pip() {
    let host = host_linux(true, false, true);
    let plan = build_install_plan(&FakeProgram, &host);
    let chosen = plan.chosen().expect("chosen");
    assert!(matches!(chosen.kind, InstallationMethod::Pip(_)));
}

#[test]
fn both_installed_uv_wins_via_pip_blocked() {
    let host = host_linux(true, true, true);
    let plan = build_install_plan(&FakeProgram, &host);
    // Pip is blocked by UvPreferredOverPip; synthesized UvWithInstall
    // wins (Uv(_) isn't declared for PIP_ONLY).
    let pip_opt = plan
        .options
        .iter()
        .find(|o| matches!(o.kind, InstallationMethod::Pip(_)))
        .unwrap();
    assert_eq!(pip_opt.reason_type, InstallPlanReason::UvPreferredOverPip);
    let chosen = plan.chosen().expect("chosen");
    assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(_)));
}

#[test]
fn neither_installed_selects_bootstrap() {
    let host = host_linux(false, false, true);
    let plan = build_install_plan(&FakeProgram, &host);
    let chosen = plan.chosen().expect("chosen");
    assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(_)));
}

#[test]
fn bash_missing_on_linux_blocks_bootstrap() {
    let host = host_linux(false, false, false);
    let plan = build_install_plan(&FakeProgram, &host);
    assert!(!plan.successful);
    let uvi = plan
        .options
        .iter()
        .find(|o| matches!(o.kind, InstallationMethod::UvWithInstall(_)))
        .unwrap();
    assert_eq!(uvi.reason_type, InstallPlanReason::BashNotAvailable);
}

#[test]
fn native_windows_selects_bootstrap_even_without_bash() {
    let host = HostCapabilities {
        os_type: OsType::Windows,
        has_bash: false,
        ..HostCapabilities::default()
    };
    let plan = build_install_plan(&FakeProgram, &host);
    let chosen = plan.chosen().expect("chosen");
    assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(_)));
}
