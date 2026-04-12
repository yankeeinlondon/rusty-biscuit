//! Install plan data types and builder.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md` and
//! `tech-design.md` for the contract this module implements.

use serde::Serialize;

use crate::error::SniffInstallationError;
use crate::os::OsType;
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::host_capability::HostCapabilities;
use crate::programs::installer::{
    InstallOptions, InstallResult, astral_installer_url, execute_install, method_available,
};
use crate::programs::schema::ProgramMetadata;
use crate::programs::types::InstallationMethod;

/// Machine-readable reason an install plan option was selected or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPlanReason {
    /// This option was chosen.
    Selected,
    /// A higher-priority runnable method was chosen instead.
    LowerPriorityAlternative,
    /// The program's `os_availability` excludes the detected host OS.
    NoOsSupport,
    /// The package manager required by this method is not installed.
    ManagerNotInstalled,
    /// The method requires sudo and the host cannot sudo (or `--no-sudo`).
    RequiresSudoNotAvailable,
    /// A language PM is installed but not verified.
    RequiresUnverifiedLangManager,
    /// `Pip(_)` is blocked because `uv` is also installed and is preferred
    /// over pip for Python tool installs.
    UvPreferredOverPip,
    /// `UvWithInstall(_)` cannot run on a Unix host without bash (the astral
    /// installer requires it).
    BashNotAvailable,
    /// Catch-all for unexpected skip reasons.
    Unknown,
}

/// One evaluated installation method on an install plan.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlanOption {
    pub kind: InstallationMethod,
    pub requires_sudo: bool,
    pub choose: bool,
    pub reason_type: InstallPlanReason,
    pub reason: String,
}

/// A full evaluation of every installation method a program declares.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub program: String,
    pub website: &'static str,
    pub successful: bool,
    pub options: Vec<InstallPlanOption>,
}

impl InstallPlan {
    /// Every method considered, regardless of runnability.
    pub fn known_installations(&self) -> Vec<&InstallationMethod> {
        self.options.iter().map(|o| &o.kind).collect()
    }

    /// Every option that was not chosen.
    pub fn failed_with_reason(&self) -> Vec<&InstallPlanOption> {
        self.options.iter().filter(|o| !o.choose).collect()
    }

    /// The chosen option, if any.
    pub fn chosen(&self) -> Option<&InstallPlanOption> {
        self.options.iter().find(|o| o.choose)
    }

    /// Execute the chosen installation option.
    ///
    /// Returns `NoViableMethod` if no option was marked as chosen. For a
    /// chosen `RemoteBash` method, dry-runs render the shell command without
    /// approval, but non-dry-run execution returns
    /// `RemoteBashConsentRequired` unless `opts.approve_remote_bash` is
    /// `true`. Otherwise delegates to `execute_install`.
    pub fn execute(&self, opts: &InstallOptions) -> Result<InstallResult, SniffInstallationError> {
        let chosen = self
            .chosen()
            .ok_or_else(|| SniffInstallationError::NoViableMethod {
                pkg: self.program.clone(),
                detail: format!(
                    "no runnable method (considered {} option(s))",
                    self.options.len()
                ),
            })?;

        let needs_remote_consent = matches!(
            chosen.kind,
            InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
        );
        if needs_remote_consent && !opts.approve_remote_bash && !opts.dry_run {
            let url = match &chosen.kind {
                InstallationMethod::RemoteBash(u) => (*u).to_string(),
                InstallationMethod::UvWithInstall(_) => astral_installer_url().to_string(),
                _ => unreachable!(),
            };
            return Err(SniffInstallationError::RemoteBashConsentRequired {
                pkg: self.program.clone(),
                url,
            });
        }

        execute_install(&chosen.kind, opts)
    }
}

/// Why a given method would run (or not) on this host, before priority is
/// applied. Used internally by the bucket selector.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct MethodFact {
    pub kind: InstallationMethod,
    pub os_supported: bool,
    pub manager_installed: bool,
    pub requires_sudo: bool,
    pub lang_manager_verified: bool,
    pub eligible_without_priority: bool,
    pub blocking_reason: Option<InstallPlanReason>,
}

/// Derives a [`MethodFact`] for a single installation method against the
/// given host and OS availability list.
///
/// The fact captures raw eligibility signals (OS match, manager presence,
/// sudo requirement) without applying any priority ordering. Task 12 uses
/// these facts to select the winning bucket.
pub(crate) fn derive_method_fact(
    method: &InstallationMethod,
    os_availability: &[OsType],
    host: &HostCapabilities,
) -> MethodFact {
    let os_supported = os_availability.is_empty() || os_availability.contains(&host.os_type);
    let manager_installed = method_available(method, host);
    let requires_sudo = method_requires_sudo(method, host);
    let lang_manager_verified = is_lang_manager_verified(method, host);

    let mut blocking_reason = None;
    let mut eligible = true;

    // `UvWithInstall` has its own eligibility rule that differs from the
    // usual manager-installed check: it is runnable whenever bash is
    // available on Unix or the host is native Windows (PowerShell always
    // present). Whether `uv` itself is on PATH is deferred to execute time.
    if matches!(method, InstallationMethod::UvWithInstall(_)) {
        if !os_supported {
            eligible = false;
            blocking_reason = Some(InstallPlanReason::NoOsSupport);
        } else if host.os_type == OsType::Windows {
            // Native Windows (includes both `Windows` and WSL as detected).
            // WSL is `OsType::Linux` with `is_wsl = true`, so this matches
            // only real native Windows; PowerShell is always present.
        } else if !host.has_bash {
            eligible = false;
            blocking_reason = Some(InstallPlanReason::BashNotAvailable);
        }
        return MethodFact {
            kind: method.clone(),
            os_supported,
            manager_installed,
            requires_sudo,
            lang_manager_verified,
            eligible_without_priority: eligible,
            blocking_reason,
        };
    }

    // `Pip(_)` is eligible only when pip is installed AND uv is NOT
    // installed. When both are present, we prefer uv.
    if matches!(method, InstallationMethod::Pip(_))
        && manager_installed
        && host.lang_pkg_mgrs.is_installed(LanguagePackageManager::Uv)
    {
        return MethodFact {
            kind: method.clone(),
            os_supported,
            manager_installed,
            requires_sudo,
            lang_manager_verified,
            eligible_without_priority: false,
            blocking_reason: Some(InstallPlanReason::UvPreferredOverPip),
        };
    }

    if !os_supported {
        eligible = false;
        blocking_reason = Some(InstallPlanReason::NoOsSupport);
    } else if !manager_installed {
        eligible = false;
        blocking_reason = Some(InstallPlanReason::ManagerNotInstalled);
    } else if requires_sudo && !host.can_sudo {
        eligible = false;
        blocking_reason = Some(InstallPlanReason::RequiresSudoNotAvailable);
    }

    MethodFact {
        kind: method.clone(),
        os_supported,
        manager_installed,
        requires_sudo,
        lang_manager_verified,
        eligible_without_priority: eligible,
        blocking_reason,
    }
}

/// Returns whether this method needs `sudo` on the current host.
fn method_requires_sudo(method: &InstallationMethod, host: &HostCapabilities) -> bool {
    let unix_sudo_method = matches!(
        method,
        InstallationMethod::Apt(_)
            | InstallationMethod::Nala(_)
            | InstallationMethod::Dnf(_)
            | InstallationMethod::Pacman(_)
    );
    if unix_sudo_method {
        return true;
    }
    // On native Windows, winget elevation surfaces as requires_sudo = true.
    // On WSL it is still Linux and doesn't run winget.
    if matches!(method, InstallationMethod::Winget(_))
        && host.os_type == OsType::Windows
        && !host.is_wsl
    {
        return true;
    }
    false
}

fn is_lang_manager_verified(method: &InstallationMethod, host: &HostCapabilities) -> bool {
    match method {
        InstallationMethod::Npm(_) => host
            .verified_lang_pkg_mgrs
            .contains(&LanguagePackageManager::Npm),
        InstallationMethod::Pnpm(_) => host
            .verified_lang_pkg_mgrs
            .contains(&LanguagePackageManager::Pnpm),
        InstallationMethod::Yarn(_) => host
            .verified_lang_pkg_mgrs
            .contains(&LanguagePackageManager::Yarn),
        InstallationMethod::Bun(_) => host
            .verified_lang_pkg_mgrs
            .contains(&LanguagePackageManager::Bun),
        InstallationMethod::Cargo(_) => host
            .verified_lang_pkg_mgrs
            .contains(&LanguagePackageManager::Cargo),
        _ => false,
    }
}

/// Priority buckets for install plan selection. The earliest matching bucket
/// whose fact is eligible wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bucket {
    DefaultOsPm,
    VerifiedPnpm,
    NpmNoSudo,
    AltOsPm,
    RemoteBash,
    Cargo,
    SudoNpm,
    /// Plain `Pip(_)` or `Uv(_)` — the manager is already on the host.
    PipUvDirect,
    /// `UvWithInstall(_)` — bootstraps uv from astral.sh if absent.
    UvBootstrap,
    Other,
}

/// Returns true if the given OS package manager corresponds to the given
/// installation method. Uses direct enum-variant matching to avoid the
/// `manager_binary()` / `serde_key()` mismatch for Chocolatey (`"choco"` vs
/// `"chocolatey"`).
fn os_pm_matches_method(pm: &OsPackageManager, method: &InstallationMethod) -> bool {
    use InstallationMethod as IM;
    matches!(
        (pm, method),
        (OsPackageManager::Apt, IM::Apt(_))
            | (OsPackageManager::Nala, IM::Nala(_))
            | (OsPackageManager::Brew, IM::Brew(_))
            | (OsPackageManager::Dnf, IM::Dnf(_))
            | (OsPackageManager::Pacman, IM::Pacman(_))
            | (OsPackageManager::Winget, IM::Winget(_))
            | (OsPackageManager::Chocolatey, IM::Chocolatey(_))
            | (OsPackageManager::Scoop, IM::Scoop(_))
            | (OsPackageManager::Nix, IM::Nix(_))
    )
}

fn bucket_for(fact: &MethodFact, host: &HostCapabilities) -> Bucket {
    // Default OS PM bucket: method manager matches host default PM
    if fact.kind.is_os_package_manager()
        && host
            .default_os_package_manager
            .as_ref()
            .is_some_and(|pm| os_pm_matches_method(pm, &fact.kind))
    {
        return Bucket::DefaultOsPm;
    }
    match &fact.kind {
        InstallationMethod::Pnpm(_) if fact.lang_manager_verified => Bucket::VerifiedPnpm,
        InstallationMethod::Pnpm(_) => Bucket::Other,
        InstallationMethod::Npm(_) => match host.npm_global_prefix_writable {
            Some(false) if host.can_sudo => Bucket::SudoNpm,
            Some(false) => Bucket::Other,
            _ => Bucket::NpmNoSudo,
        },
        _ if fact.kind.is_os_package_manager() => Bucket::AltOsPm,
        InstallationMethod::RemoteBash(_) => Bucket::RemoteBash,
        InstallationMethod::Cargo(_) => Bucket::Cargo,
        InstallationMethod::Pip(_) | InstallationMethod::Uv(_) => Bucket::PipUvDirect,
        InstallationMethod::UvWithInstall(_) => Bucket::UvBootstrap,
        _ => Bucket::Other,
    }
}

fn bucket_order() -> [Bucket; 9] {
    [
        Bucket::DefaultOsPm,
        Bucket::VerifiedPnpm,
        Bucket::NpmNoSudo,
        Bucket::AltOsPm,
        Bucket::RemoteBash,
        Bucket::Cargo,
        Bucket::SudoNpm,
        Bucket::PipUvDirect,
        Bucket::UvBootstrap,
    ]
}

/// Synthesize a tail `UvWithInstall(pkg)` method for a program that declares
/// `Pip(_)` or `Uv(_)` but no explicit `UvWithInstall(_)`. The package name
/// is taken from the first `Uv(_)` if present, else from the first `Pip(_)`.
fn synthesize_uv_bootstrap(declared: &[InstallationMethod]) -> Option<InstallationMethod> {
    if declared
        .iter()
        .any(|m| matches!(m, InstallationMethod::UvWithInstall(_)))
    {
        return None;
    }
    let pkg = declared
        .iter()
        .find_map(|m| match m {
            InstallationMethod::Uv(p) => Some(*p),
            _ => None,
        })
        .or_else(|| {
            declared.iter().find_map(|m| match m {
                InstallationMethod::Pip(p) => Some(*p),
                _ => None,
            })
        })?;
    Some(InstallationMethod::UvWithInstall(pkg))
}

/// Build an install plan for a program against the given host capabilities.
pub fn build_install_plan<P: ProgramMetadata>(program: &P, host: &HostCapabilities) -> InstallPlan {
    let info = program.info();

    // Auto-append a synthesized UvWithInstall(pkg) when the program declares
    // Pip(_) or Uv(_) and does not already declare an explicit
    // UvWithInstall(_). Keeps program metadata untouched while closing the
    // install gap for Python-installable tools on all supported hosts.
    let declared: Vec<InstallationMethod> = info.installation_methods.to_vec();
    let mut effective_methods = declared.clone();
    if let Some(synth) = synthesize_uv_bootstrap(&declared) {
        effective_methods.push(synth);
    }

    let facts: Vec<MethodFact> = effective_methods
        .iter()
        .map(|m| derive_method_fact(m, info.os_availability, host))
        .collect();

    // Find the first bucket with an eligible fact.
    let mut chosen_index: Option<usize> = None;
    'outer: for bucket in bucket_order() {
        for (idx, fact) in facts.iter().enumerate() {
            if fact.eligible_without_priority && bucket_for(fact, host) == bucket {
                chosen_index = Some(idx);
                break 'outer;
            }
        }
    }

    let options: Vec<InstallPlanOption> = facts
        .iter()
        .enumerate()
        .map(|(i, fact)| {
            let choose = chosen_index == Some(i);
            let (reason_type, reason) = if choose {
                let text = match &fact.kind {
                    InstallationMethod::Uv(_) => {
                        "chosen — uv tool install (uv already on host)".to_string()
                    }
                    InstallationMethod::Pip(_) => {
                        "chosen — pip install (pip already on host; uv absent)".to_string()
                    }
                    _ => format!(
                        "chosen — {}{}",
                        bucket_description(bucket_for(fact, host)),
                        if fact.requires_sudo {
                            " (requires sudo)"
                        } else {
                            ""
                        }
                    ),
                };
                (InstallPlanReason::Selected, text)
            } else if fact.eligible_without_priority && bucket_for(fact, host) != Bucket::Other {
                // Eligible and in a real bucket, just outranked by a higher one.
                (
                    InstallPlanReason::LowerPriorityAlternative,
                    "a higher-priority method was chosen".to_string(),
                )
            } else {
                // Either ineligible, or eligible but relegated to Bucket::Other
                // (e.g. unverified pnpm).  Use blocking_reason_for for a precise
                // reason type.
                let reason_type = blocking_reason_for(fact, host);
                let reason = explain_blocking_reason(fact, reason_type);
                (reason_type, reason)
            };
            InstallPlanOption {
                kind: fact.kind.clone(),
                requires_sudo: fact.requires_sudo,
                choose,
                reason_type,
                reason,
            }
        })
        .collect();

    InstallPlan {
        program: program.display_name().to_string(),
        website: program.website(),
        successful: chosen_index.is_some(),
        options,
    }
}

fn bucket_description(bucket: Bucket) -> &'static str {
    match bucket {
        Bucket::DefaultOsPm => "default OS package manager",
        Bucket::VerifiedPnpm => "verified pnpm global",
        Bucket::NpmNoSudo => "user-writable npm global",
        Bucket::AltOsPm => "alternative OS package manager",
        Bucket::RemoteBash => "remote bash installer",
        Bucket::Cargo => "cargo install",
        Bucket::SudoNpm => "sudo-gated npm global",
        Bucket::PipUvDirect => "pip/uv already on host",
        Bucket::UvBootstrap => {
            "uv tool install (bootstraps uv via astral.sh if absent; requires remote-script consent)"
        }
        Bucket::Other => "other",
    }
}

fn blocking_reason_for(fact: &MethodFact, host: &HostCapabilities) -> InstallPlanReason {
    if let Some(reason) = fact.blocking_reason {
        return reason;
    }
    // Unverified pnpm: the manager is installed but has no globally-installed
    // packages, so we refuse to pick it blindly.
    if matches!(&fact.kind, InstallationMethod::Pnpm(_))
        && !fact.lang_manager_verified
        && host
            .lang_pkg_mgrs
            .is_installed(LanguagePackageManager::Pnpm)
    {
        return InstallPlanReason::RequiresUnverifiedLangManager;
    }
    InstallPlanReason::Unknown
}

fn explain_blocking_reason(fact: &MethodFact, reason: InstallPlanReason) -> String {
    match reason {
        InstallPlanReason::NoOsSupport => {
            format!("{} does not run on this OS", fact.kind.manager_name())
        }
        InstallPlanReason::ManagerNotInstalled => format!(
            "{} is not installed on this host",
            fact.kind.manager_binary()
        ),
        InstallPlanReason::RequiresSudoNotAvailable => format!(
            "{} requires sudo and the current user cannot sudo",
            fact.kind.manager_name()
        ),
        InstallPlanReason::RequiresUnverifiedLangManager => format!(
            "{} is installed but has no globally-installed packages — not choosing it blindly",
            fact.kind.manager_name()
        ),
        InstallPlanReason::UvPreferredOverPip => {
            "uv is installed; uv is preferred over pip for Python tools".to_string()
        }
        InstallPlanReason::BashNotAvailable => {
            "bash is not available; cannot run the astral uv installer".to_string()
        }
        InstallPlanReason::Unknown => "no other bucket accepted this method".to_string(),
        InstallPlanReason::Selected | InstallPlanReason::LowerPriorityAlternative => {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod fact_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::enums::OsPackageManager;
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::types::InstallationMethod;

    fn host_with_brew() -> HostCapabilities {
        let json = r#"{"brew": true}"#;
        let os_pkg_mgrs = serde_json::from_str(json).unwrap();
        HostCapabilities {
            os_type: OsType::MacOS,
            os_pkg_mgrs,
            default_os_package_manager: Some(OsPackageManager::Brew),
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn derive_fact_brew_on_macos_is_eligible() {
        let host = host_with_brew();
        let method = InstallationMethod::Brew("ripgrep");
        let os_availability = &[OsType::MacOS];
        let fact = derive_method_fact(&method, os_availability, &host);
        assert!(fact.os_supported);
        assert!(fact.manager_installed);
        assert!(!fact.requires_sudo);
        assert!(fact.eligible_without_priority);
    }

    #[test]
    fn derive_fact_apt_requires_sudo() {
        let host = HostCapabilities::default();
        let method = InstallationMethod::Apt("ripgrep");
        let fact = derive_method_fact(&method, &[], &host);
        assert!(fact.requires_sudo);
    }

    #[test]
    fn derive_fact_brew_not_installed_is_ineligible() {
        let host = HostCapabilities::default(); // no managers
        let method = InstallationMethod::Brew("ripgrep");
        let fact = derive_method_fact(&method, &[], &host);
        assert!(!fact.manager_installed);
        assert!(!fact.eligible_without_priority);
        assert_eq!(
            fact.blocking_reason,
            Some(InstallPlanReason::ManagerNotInstalled)
        );
    }

    #[test]
    fn derive_fact_unsupported_os_is_blocked_by_os() {
        let host = HostCapabilities {
            os_type: OsType::Linux,
            ..HostCapabilities::default()
        };
        let method = InstallationMethod::Brew("ripgrep");
        let fact = derive_method_fact(&method, &[OsType::MacOS], &host);
        assert!(!fact.os_supported);
        assert_eq!(fact.blocking_reason, Some(InstallPlanReason::NoOsSupport));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_plan_reason_selected_serializes_snake_case() {
        let json = serde_json::to_string(&InstallPlanReason::Selected).unwrap();
        assert_eq!(json, "\"selected\"");
    }

    #[test]
    fn install_plan_reason_lower_priority_serializes_snake_case() {
        let json = serde_json::to_string(&InstallPlanReason::LowerPriorityAlternative).unwrap();
        assert_eq!(json, "\"lower_priority_alternative\"");
    }

    #[test]
    fn empty_plan_reports_no_chosen_option() {
        let plan = InstallPlan {
            program: "vim".into(),
            website: "https://www.vim.org",
            successful: false,
            options: Vec::new(),
        };
        assert!(plan.chosen().is_none());
        assert!(plan.failed_with_reason().is_empty());
        assert!(plan.known_installations().is_empty());
    }

    #[test]
    fn chosen_returns_option_where_choose_is_true() {
        let plan = InstallPlan {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            successful: true,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Cargo("bat"),
                    requires_sudo: false,
                    choose: false,
                    reason_type: InstallPlanReason::LowerPriorityAlternative,
                    reason: "brew was chosen".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Brew("bat"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "default OS package manager".into(),
                },
            ],
        };
        let chosen = plan.chosen().expect("chosen option");
        assert!(matches!(chosen.kind, InstallationMethod::Brew("bat")));
        assert_eq!(plan.failed_with_reason().len(), 1);
    }
}

#[cfg(test)]
mod execute_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::installer::InstallOptions;
    use crate::programs::schema::{ProgramInfo, VersionFlag, VersionParseStrategy};

    static BREW_PKG: ProgramInfo = ProgramInfo {
        binary_name: "ripgrep",
        display_name: "ripgrep",
        description: "fast grep",
        website: "https://github.com/BurntSushi/ripgrep",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[OsType::MacOS],
        repo: None,
        installation_methods: &[InstallationMethod::Brew("ripgrep")],
    };

    struct FakeProgram;
    impl crate::programs::schema::ProgramMetadata for FakeProgram {
        fn info(&self) -> &'static ProgramInfo {
            &BREW_PKG
        }
    }

    fn host_with_brew() -> HostCapabilities {
        let os_pkg_mgrs = serde_json::from_str(r#"{"brew": true}"#).unwrap();
        HostCapabilities {
            os_type: OsType::MacOS,
            os_pkg_mgrs,
            default_os_package_manager: Some(crate::programs::enums::OsPackageManager::Brew),
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn dry_run_returns_ok_without_executing() {
        let plan = build_install_plan(&FakeProgram, &host_with_brew());
        let result = plan.execute(&InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert!(result.command.contains("brew"));
    }

    #[test]
    fn failed_plan_returns_no_viable_method() {
        let host = HostCapabilities {
            os_type: OsType::Linux, // brew not installed on this fake host
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram, &host);
        let err = plan.execute(&InstallOptions::dry_run()).unwrap_err();
        assert!(matches!(
            err,
            crate::error::SniffInstallationError::NoViableMethod { .. }
        ));
    }

    #[test]
    fn remote_bash_without_consent_errors() {
        let plan = InstallPlan {
            program: "rustup".into(),
            website: "https://rustup.rs",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "remote bash installer".into(),
            }],
        };
        let err = plan.execute(&InstallOptions::default()).unwrap_err();
        assert!(matches!(
            err,
            crate::error::SniffInstallationError::RemoteBashConsentRequired { .. }
        ));
    }

    #[test]
    fn remote_bash_dry_run_allowed_without_consent() {
        let plan = InstallPlan {
            program: "rustup".into(),
            website: "https://rustup.rs",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "remote bash installer".into(),
            }],
        };
        // Dry-run renders the curl|bash command without execution or consent.
        let result = plan.execute(&InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert!(
            result
                .command
                .contains("curl -sSfL 'https://sh.rustup.rs' | bash")
        );
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::schema::{
        ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy,
    };
    use crate::programs::types::InstallationMethod;

    struct FakeProgram {
        info: &'static ProgramInfo,
    }
    impl ProgramMetadata for FakeProgram {
        fn info(&self) -> &'static ProgramInfo {
            self.info
        }
    }

    static BREW_AND_CARGO: ProgramInfo = ProgramInfo {
        binary_name: "bat",
        display_name: "bat",
        description: "cat clone",
        website: "https://github.com/sharkdp/bat",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[OsType::MacOS, OsType::Linux],
        repo: None,
        installation_methods: &[
            InstallationMethod::Brew("bat"),
            InstallationMethod::Cargo("bat"),
        ],
    };

    fn host_macos_with_brew() -> HostCapabilities {
        let os_pkg_mgrs = serde_json::from_str(r#"{"brew": true}"#).unwrap();
        let lang_pkg_mgrs = serde_json::from_str(r#"{"cargo": true}"#).unwrap();
        HostCapabilities {
            os_type: OsType::MacOS,
            default_os_package_manager: Some(OsPackageManager::Brew),
            os_pkg_mgrs,
            lang_pkg_mgrs,
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn brew_wins_over_cargo_on_macos() {
        let host = host_macos_with_brew();
        let plan = build_install_plan(
            &FakeProgram {
                info: &BREW_AND_CARGO,
            },
            &host,
        );
        assert!(plan.successful);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Brew("bat")));
        assert_eq!(chosen.reason_type, InstallPlanReason::Selected);

        let cargo_opt = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::Cargo(_)))
            .unwrap();
        assert!(!cargo_opt.choose);
        assert_eq!(
            cargo_opt.reason_type,
            InstallPlanReason::LowerPriorityAlternative
        );
    }

    static LINUX_APT_ONLY: ProgramInfo = ProgramInfo {
        binary_name: "htop",
        display_name: "htop",
        description: "interactive process viewer",
        website: "https://htop.dev",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[OsType::Linux],
        repo: None,
        installation_methods: &[InstallationMethod::Apt("htop")],
    };

    #[test]
    fn apt_without_sudo_is_rejected_with_reason() {
        let os_pkg_mgrs = serde_json::from_str(r#"{"apt": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            default_os_package_manager: Some(OsPackageManager::Apt),
            os_pkg_mgrs,
            can_sudo: false,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(
            &FakeProgram {
                info: &LINUX_APT_ONLY,
            },
            &host,
        );
        assert!(!plan.successful);
        let apt = &plan.options[0];
        assert!(!apt.choose);
        assert_eq!(apt.reason_type, InstallPlanReason::RequiresSudoNotAvailable);
    }

    #[test]
    fn apt_with_sudo_is_selected() {
        let os_pkg_mgrs = serde_json::from_str(r#"{"apt": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            default_os_package_manager: Some(OsPackageManager::Apt),
            os_pkg_mgrs,
            can_sudo: true,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(
            &FakeProgram {
                info: &LINUX_APT_ONLY,
            },
            &host,
        );
        assert!(plan.successful);
        let apt = plan.chosen().unwrap();
        assert!(apt.requires_sudo);
    }

    static PNPM_AND_NPM: ProgramInfo = ProgramInfo {
        binary_name: "typescript",
        display_name: "TypeScript",
        description: "Typed JavaScript",
        website: "https://www.typescriptlang.org",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[
            InstallationMethod::Pnpm("typescript"),
            InstallationMethod::Npm("typescript"),
        ],
    };

    #[test]
    fn verified_pnpm_beats_npm() {
        let lang_pkg_mgrs = serde_json::from_str(r#"{"pnpm": true, "npm": true}"#).unwrap();
        let mut host = HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs,
            npm_global_prefix_writable: Some(true),
            ..HostCapabilities::default()
        };
        host.verified_lang_pkg_mgrs
            .insert(LanguagePackageManager::Pnpm);
        let plan = build_install_plan(
            &FakeProgram {
                info: &PNPM_AND_NPM,
            },
            &host,
        );
        let chosen = plan.chosen().unwrap();
        assert!(matches!(chosen.kind, InstallationMethod::Pnpm(_)));
    }

    #[test]
    fn unverified_pnpm_gets_unverified_reason_and_falls_through_to_npm() {
        let lang_pkg_mgrs = serde_json::from_str(r#"{"pnpm": true, "npm": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs,
            npm_global_prefix_writable: Some(true),
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(
            &FakeProgram {
                info: &PNPM_AND_NPM,
            },
            &host,
        );
        let chosen = plan.chosen().unwrap();
        assert!(matches!(chosen.kind, InstallationMethod::Npm(_)));

        let pnpm_opt = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::Pnpm(_)))
            .unwrap();
        assert_eq!(
            pnpm_opt.reason_type,
            InstallPlanReason::RequiresUnverifiedLangManager
        );
    }

    // =======================================================================
    // UvWithInstall — auto-append, selection rule, eligibility edges
    // =======================================================================

    static PIP_ONLY: ProgramInfo = ProgramInfo {
        binary_name: "conan",
        display_name: "conan",
        description: "C/C++ pkg mgr",
        website: "https://conan.io",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[InstallationMethod::Pip("conan")],
    };

    static UV_ONLY: ProgramInfo = ProgramInfo {
        binary_name: "kimi-cli",
        display_name: "kimi-cli",
        description: "Kimi CLI",
        website: "https://example.com",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[InstallationMethod::Uv("kimi-cli")],
    };

    static PIP_AND_UV: ProgramInfo = ProgramInfo {
        binary_name: "aider",
        display_name: "aider",
        description: "AI pair",
        website: "https://aider.chat",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[
            InstallationMethod::Pip("aider"),
            InstallationMethod::Uv("aider-chat"),
        ],
    };

    static BREW_AND_PIP: ProgramInfo = ProgramInfo {
        binary_name: "poetry",
        display_name: "poetry",
        description: "poetry",
        website: "https://python-poetry.org",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[
            InstallationMethod::Brew("poetry"),
            InstallationMethod::Pip("poetry"),
        ],
    };

    static BREW_VIM_ONLY: ProgramInfo = ProgramInfo {
        binary_name: "vim",
        display_name: "vim",
        description: "vim",
        website: "https://vim.org",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[InstallationMethod::Brew("vim")],
    };

    static PIP_AND_EXPLICIT_UVI: ProgramInfo = ProgramInfo {
        binary_name: "foo",
        display_name: "foo",
        description: "foo",
        website: "https://example.com",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[
            InstallationMethod::Pip("foo"),
            InstallationMethod::UvWithInstall("foo"),
        ],
    };

    static CARGO_AND_PIP: ProgramInfo = ProgramInfo {
        binary_name: "foo",
        display_name: "foo",
        description: "foo",
        website: "https://example.com",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[
            InstallationMethod::Cargo("foo"),
            InstallationMethod::Pip("foo"),
        ],
    };

    fn host_linux_with(
        pip: bool,
        uv: bool,
        has_bash: bool,
        cargo: bool,
        brew: bool,
    ) -> HostCapabilities {
        let lang_json = format!(r#"{{"pip": {}, "uv": {}, "cargo": {}}}"#, pip, uv, cargo);
        let os_json = format!(r#"{{"brew": {}}}"#, brew);
        HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs: serde_json::from_str(&lang_json).unwrap(),
            os_pkg_mgrs: serde_json::from_str(&os_json).unwrap(),
            default_os_package_manager: if brew {
                Some(OsPackageManager::Brew)
            } else {
                None
            },
            has_bash,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn auto_append_synthesizes_uv_with_install_from_pip_only() {
        let host = host_linux_with(true, false, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_ONLY }, &host);
        assert!(
            plan.options.iter().any(
                |o| matches!(o.kind, InstallationMethod::UvWithInstall(pkg) if pkg == "conan")
            )
        );
    }

    #[test]
    fn auto_append_synthesizes_uv_with_install_from_uv_only() {
        let host = host_linux_with(false, true, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &UV_ONLY }, &host);
        assert!(plan.options.iter().any(
            |o| matches!(o.kind, InstallationMethod::UvWithInstall(pkg) if pkg == "kimi-cli")
        ));
    }

    #[test]
    fn auto_append_prefers_uv_package_name_over_pip() {
        let host = host_linux_with(false, false, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_AND_UV }, &host);
        let synth = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::UvWithInstall(_)))
            .expect("synthesized UvWithInstall");
        assert!(
            matches!(synth.kind, InstallationMethod::UvWithInstall(pkg) if pkg == "aider-chat")
        );
    }

    #[test]
    fn auto_append_skipped_when_no_python_method() {
        let host = host_linux_with(false, false, true, false, true);
        let plan = build_install_plan(
            &FakeProgram {
                info: &BREW_VIM_ONLY,
            },
            &host,
        );
        assert!(
            !plan
                .options
                .iter()
                .any(|o| matches!(o.kind, InstallationMethod::UvWithInstall(_)))
        );
    }

    #[test]
    fn auto_append_skipped_when_explicit_uv_with_install_present() {
        let host = host_linux_with(false, false, true, false, false);
        let plan = build_install_plan(
            &FakeProgram {
                info: &PIP_AND_EXPLICIT_UVI,
            },
            &host,
        );
        let uvi_count = plan
            .options
            .iter()
            .filter(|o| matches!(o.kind, InstallationMethod::UvWithInstall(_)))
            .count();
        assert_eq!(
            uvi_count, 1,
            "no synthesis should occur when explicit UvWithInstall present"
        );
    }

    #[test]
    fn uv_wins_when_uv_installed_and_uv_declared() {
        // host has uv installed → Uv(_) in PipUvDirect wins.
        let host = host_linux_with(true, true, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_AND_UV }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Uv(pkg) if pkg == "aider-chat"));
    }

    #[test]
    fn uv_wins_when_uv_installed_and_only_pip_declared() {
        // Only Pip declared; host has uv → Pip blocked by UvPreferredOverPip,
        // synthesized UvWithInstall runs in UvBootstrap.
        let host = host_linux_with(true, true, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_ONLY }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(pkg) if pkg == "conan"));
        let pip_opt = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::Pip(_)))
            .unwrap();
        assert_eq!(pip_opt.reason_type, InstallPlanReason::UvPreferredOverPip);
    }

    #[test]
    fn pip_wins_when_pip_installed_uv_absent() {
        let host = host_linux_with(true, false, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_ONLY }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Pip(pkg) if pkg == "conan"));
        let uvi_opt = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::UvWithInstall(_)))
            .unwrap();
        assert_eq!(
            uvi_opt.reason_type,
            InstallPlanReason::LowerPriorityAlternative
        );
    }

    #[test]
    fn pip_wins_when_both_declared_and_pip_installed_uv_absent() {
        let host = host_linux_with(true, false, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_AND_UV }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Pip(pkg) if pkg == "aider"));
        let uv_opt = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::Uv(_)))
            .unwrap();
        assert_eq!(uv_opt.reason_type, InstallPlanReason::ManagerNotInstalled);
    }

    #[test]
    fn bootstrap_wins_when_neither_installed() {
        let host = host_linux_with(false, false, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &PIP_ONLY }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(pkg) if pkg == "conan"));
    }

    #[test]
    fn bootstrap_wins_when_only_uv_declared_and_only_pip_installed() {
        let host = host_linux_with(true, false, true, false, false);
        let plan = build_install_plan(&FakeProgram { info: &UV_ONLY }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(pkg) if pkg == "kimi-cli"));
    }

    #[test]
    fn brew_wins_over_pip_uv_direct_on_macos() {
        let os_pkg_mgrs = serde_json::from_str(r#"{"brew": true}"#).unwrap();
        let lang_pkg_mgrs = serde_json::from_str(r#"{"pip": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::MacOS,
            default_os_package_manager: Some(OsPackageManager::Brew),
            os_pkg_mgrs,
            lang_pkg_mgrs,
            has_bash: true,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(
            &FakeProgram {
                info: &BREW_AND_PIP,
            },
            &host,
        );
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Brew(_)));
    }

    #[test]
    fn cargo_wins_over_uv_bootstrap() {
        // host has cargo, no pip, no uv — cargo should win; UvBootstrap is
        // strictly later in bucket_order.
        let lang_pkg_mgrs = serde_json::from_str(r#"{"cargo": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs,
            has_bash: true,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(
            &FakeProgram {
                info: &CARGO_AND_PIP,
            },
            &host,
        );
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Cargo(_)));
    }

    #[test]
    fn uv_with_install_blocked_on_unix_without_bash() {
        let host = HostCapabilities {
            os_type: OsType::Linux,
            has_bash: false,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram { info: &PIP_ONLY }, &host);
        let uvi = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::UvWithInstall(_)))
            .unwrap();
        assert!(!uvi.choose);
        assert_eq!(uvi.reason_type, InstallPlanReason::BashNotAvailable);
    }

    #[test]
    fn uv_with_install_eligible_on_native_windows() {
        let host = HostCapabilities {
            os_type: OsType::Windows,
            has_bash: false,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram { info: &PIP_ONLY }, &host);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::UvWithInstall(_)));
    }
}
