//! Install plan data types and builder.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md` and
//! `tech-design.md` for the contract this module implements.

use serde::Serialize;

use crate::error::SniffInstallationError;
use crate::programs::host_capability::HostCapabilities;
use crate::programs::installer::{InstallOptions, InstallResult};
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
