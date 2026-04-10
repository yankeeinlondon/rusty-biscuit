//! Plan-aware install rendering for the CLI.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md`
//! section "CLI: Updated `install` Behavior" for the messaging contract.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use sniff::programs::{InstallPlan, InstallPlanOption, InstallationMethod};

/// Render the plan to a `String` ready for printing to stdout.
///
/// Branches:
/// - `plan.successful == true` and no `requires_sudo` on the chosen option →
///   short success line
/// - `plan.successful == true` and `requires_sudo` → sudo-warning line
/// - `plan.successful == true` and RemoteBash chosen → remote-bash notice
/// - `plan.successful == false` → "we know how to install X but none are
///   available" block with website fallback
///
/// When `verbose` is set, each failed option is rendered above the success
/// line so users can see what was skipped and why.
pub fn render_install_plan(plan: &InstallPlan, verbose: bool) -> String {
    let terminal = Terminal::default();
    let mut out = String::new();

    if plan.successful {
        if verbose {
            for opt in plan.failed_with_reason() {
                let line = format!(
                    "- <dim>skipped {} — <i>{}</i></dim>",
                    opt.kind.manager_name(),
                    opt.reason
                );
                out.push_str(&Prose::new(line).render(&terminal));
                out.push('\n');
            }
            if !plan.failed_with_reason().is_empty() {
                out.push('\n');
            }
        }
        let chosen = plan.chosen().expect("successful plan has a chosen option");
        let success_line = render_success_line(&plan.program, chosen);
        out.push_str(&Prose::new(success_line).render(&terminal));
        out.push('\n');
    } else {
        out.push_str(&render_failure_block(plan, &terminal));
    }

    out
}

fn render_success_line(program: &str, chosen: &InstallPlanOption) -> String {
    let method = chosen.kind.manager_name();
    if matches!(chosen.kind, InstallationMethod::RemoteBash(_)) {
        format!(
            "The <blue>{program}</blue> will be installed using a <b>remote bash installer</b>. You will be asked for explicit confirmation before the script runs."
        )
    } else if chosen.requires_sudo {
        format!(
            "The <blue>{program}</blue> is installable using <b>{method}</b> but it requires root privileges so we will include the use of <yellow>sudo</yellow> so this installation method will succeed."
        )
    } else {
        format!(
            "The <blue>{program}</blue> will be installed using the <b>{method}</b>."
        )
    }
}

fn render_failure_block(plan: &InstallPlan, terminal: &Terminal) -> String {
    let mut out = String::new();
    let header = format!(
        "We know how to install the <blue>{}</blue> program via the following methods but none are available to you for the stated reasons:",
        plan.program
    );
    out.push_str(&Prose::new(header).render(terminal));
    out.push_str("\n\n");
    for opt in &plan.options {
        let line = format!(
            "    - {} (reason: <i><dim><red>{}</red></dim></i>)",
            opt.kind.manager_name(),
            opt.reason
        );
        out.push_str(&Prose::new(line).render(terminal));
        out.push('\n');
    }
    out.push('\n');
    let fallback = format!(
        "While we weren't able to do this for you, it's likely that you can install it yourself by going to their website: <a href=\"{url}\">{url}</a>",
        url = plan.website
    );
    out.push_str(&Prose::new(fallback).render(terminal));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::programs::{InstallPlanOption, InstallPlanReason};

    fn fake_success_plan(requires_sudo: bool) -> InstallPlan {
        InstallPlan {
            program: "Vim".into(),
            website: "https://www.vim.org",
            successful: true,
            options: vec![InstallPlanOption {
                kind: if requires_sudo {
                    InstallationMethod::Apt("vim")
                } else {
                    InstallationMethod::Brew("vim")
                },
                requires_sudo,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "default OS package manager".into(),
            }],
        }
    }

    #[test]
    fn success_without_sudo_mentions_brew() {
        let rendered = render_install_plan(&fake_success_plan(false), false);
        assert!(rendered.contains("Vim"));
        assert!(rendered.to_lowercase().contains("brew"));
        assert!(!rendered.to_lowercase().contains("sudo"));
    }

    #[test]
    fn success_with_sudo_mentions_sudo_warning() {
        let rendered = render_install_plan(&fake_success_plan(true), false);
        assert!(rendered.to_lowercase().contains("sudo"));
        assert!(rendered.to_lowercase().contains("root privileges"));
    }

    #[test]
    fn failure_lists_all_options_and_website() {
        let plan = InstallPlan {
            program: "Vim".into(),
            website: "https://www.vim.org",
            successful: false,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Brew("vim"),
                    requires_sudo: false,
                    choose: false,
                    reason_type: InstallPlanReason::ManagerNotInstalled,
                    reason: "brew is not installed on this host".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Apt("vim"),
                    requires_sudo: true,
                    choose: false,
                    reason_type: InstallPlanReason::RequiresSudoNotAvailable,
                    reason: "apt requires sudo".into(),
                },
            ],
        };
        let rendered = render_install_plan(&plan, false);
        assert!(rendered.contains("brew"));
        assert!(rendered.contains("apt"));
        assert!(rendered.contains("https://www.vim.org"));
        assert!(rendered.contains("none are available"));
    }

    #[test]
    fn verbose_success_prints_skipped_options() {
        let mut plan = fake_success_plan(false);
        plan.options.push(InstallPlanOption {
            kind: InstallationMethod::Cargo("vim"),
            requires_sudo: false,
            choose: false,
            reason_type: InstallPlanReason::LowerPriorityAlternative,
            reason: "brew was chosen".into(),
        });
        let verbose = render_install_plan(&plan, true);
        assert!(verbose.contains("skipped cargo"));
    }
}
