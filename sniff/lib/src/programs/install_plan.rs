//! Install plan data types and builder.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md` and
//! `tech-design.md` for the contract this module implements.

use serde::Serialize;

use crate::error::SniffInstallationError;
use crate::os::OsType;
use crate::programs::enums::LanguagePackageManager;
use crate::programs::host_capability::HostCapabilities;
use crate::programs::installer::{InstallOptions, InstallResult, method_available};
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

    /// Execute the chosen option. Task 13 replaces this stub.
    pub fn execute(
        &self,
        opts: &InstallOptions,
    ) -> Result<InstallResult, SniffInstallationError> {
        let _ = opts;
        Err(SniffInstallationError::NoViableMethod {
            pkg: self.program.clone(),
            detail: "InstallPlan::execute not implemented yet".to_string(),
        })
    }
}

/// Why a given method would run (or not) on this host, before priority is
/// applied. Used internally by the bucket selector in Task 12.
#[derive(Debug, Clone)]
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
    let manager_installed = method_available(method, &host.os_pkg_mgrs, &host.lang_pkg_mgrs)
        || (method.is_remote_bash() && host.has_bash);
    let requires_sudo = method_requires_sudo(method, host);
    let lang_manager_verified = is_lang_manager_verified(method, host);

    let mut blocking_reason = None;
    let mut eligible = true;

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
    let unix_sudo_method =
        matches!(method, InstallationMethod::Apt(_) | InstallationMethod::Nala(_) | InstallationMethod::Dnf(_) | InstallationMethod::Pacman(_));
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
        InstallationMethod::Npm(_) => {
            host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Npm)
        }
        InstallationMethod::Pnpm(_) => {
            host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Pnpm)
        }
        InstallationMethod::Yarn(_) => {
            host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Yarn)
        }
        InstallationMethod::Bun(_) => {
            host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Bun)
        }
        InstallationMethod::Cargo(_) => {
            host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Cargo)
        }
        _ => false,
    }
}

/// Stub plan builder. Tasks 11 and 12 replace this with the real implementation.
pub fn build_install_plan<P: ProgramMetadata>(
    program: &P,
    _host: &HostCapabilities,
) -> InstallPlan {
    InstallPlan {
        program: program.display_name().to_string(),
        website: program.website(),
        successful: false,
        options: Vec::new(),
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
        assert_eq!(fact.blocking_reason, Some(InstallPlanReason::ManagerNotInstalled));
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
