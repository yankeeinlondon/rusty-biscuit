//! Plan-aware install rendering for the CLI.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md`
//! section "CLI: Updated `install` Behavior" for the messaging contract.

use std::error::Error;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::programs::{InstallPlan, InstallPlanOption, InstallPlanReason, InstallationMethod};

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
///
/// Uses a `Terminal::default()` for rendering. To supply a real detected
/// terminal, use [`render_install_plan_with`].
#[allow(dead_code)]
pub fn render_install_plan(plan: &InstallPlan, verbose: bool) -> String {
    let terminal = Terminal::default();
    render_install_plan_with(plan, verbose, &terminal)
}

/// Render the plan to a `String` using the supplied `terminal` for width and
/// capability detection.
///
/// See [`render_install_plan`] for a description of the rendering branches.
pub fn render_install_plan_with(plan: &InstallPlan, verbose: bool, terminal: &Terminal) -> String {
    let mut out = String::new();

    if plan.successful {
        if verbose {
            for opt in plan.failed_with_reason() {
                let line = format!(
                    "- <dim>skipped {} — <i>{}</i></dim>",
                    opt.kind.manager_name(),
                    opt.reason
                );
                out.push_str(&Prose::new(line).render(terminal));
                out.push('\n');
            }
            if !plan.failed_with_reason().is_empty() {
                out.push('\n');
            }
        }
        let chosen = plan.chosen().expect("successful plan has a chosen option");
        let success_line = render_success_line(&plan.program, chosen);
        out.push_str(&Prose::new(success_line).render(terminal));
        out.push('\n');
    } else {
        out.push_str(&render_failure_block(plan, terminal));
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
        format!("The <blue>{program}</blue> will be installed using the <b>{method}</b>.")
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

/// Returns true if the plan's chosen option is a `RemoteBash` method, in
/// which case the CLI must prompt for a second explicit confirmation even
/// when `--yes` is passed.
#[allow(dead_code)]
pub fn should_require_remote_bash_consent(plan: &InstallPlan) -> bool {
    plan.chosen()
        .is_some_and(|o| matches!(o.kind, InstallationMethod::RemoteBash(_)))
}

/// Exit code for ctrl-c / interrupted prompts.
#[allow(dead_code)]
pub const EXIT_INTERRUPTED: i32 = 130;

/// Full "render + confirm + execute" flow. Called by the CLI dispatcher for
/// `sniff <category> install <name>`.
pub fn execute_install_flow(
    plan: &InstallPlan,
    dry_run: bool,
    skip_confirm: bool,
    plain: bool,
) -> Result<(), Box<dyn Error>> {
    use sniff::programs::{
        InstallInterviewInput, InstallInterviewOptions, InstallInterviewOutcome,
        run_install_interview,
    };

    let input = InstallInterviewInput {
        program: plan.program.clone(),
        website: plan.website,
        plan: plan.clone(),
    };

    let terminal = Terminal::new();
    let mut ui = crate::install_ui::CliInstallUi::new(terminal, plain);

    let mut opts = InstallInterviewOptions::default();
    opts.install.dry_run = dry_run;
    opts.install.skip_confirm = skip_confirm;
    opts.install.approve_remote_bash = false; // delegate asks the user
    opts.install.timeout_secs = 120;
    opts.prompt_on_failure = true;

    match run_install_interview(&input, &opts, &mut ui)? {
        InstallInterviewOutcome::Installed { .. }
        | InstallInterviewOutcome::DryRun { .. }
        | InstallInterviewOutcome::AbortedByUser
        | InstallInterviewOutcome::NotInstallable => Ok(()),
        InstallInterviewOutcome::Failed { .. } => Err("installation failed".into()),
        // The interview already emitted the detached-descendant warning; this
        // message only has to keep the exit non-zero and name the cause.
        InstallInterviewOutcome::TimedOut { .. } => {
            Err(format!("installation timed out after {}s", opts.install.timeout_secs).into())
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

use crate::args::{InstallCommandArgs, InstallCommandKind};
use crate::install::{ResolveError, ResolvedProgram, resolve_program_in_category};
use crate::output::OutputFilter;

/// Force the plan to select the method whose `manager_name()` matches
/// `via_manager`. Returns an error string if no method matches or if the
/// matched method was not eligible (not runnable on this host).
fn apply_via(plan: &mut InstallPlan, via_manager: &str) -> Result<(), String> {
    let matching_indices: Vec<usize> = plan
        .options
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind.manager_name() == via_manager)
        .map(|(i, _)| i)
        .collect();

    if matching_indices.is_empty() {
        let valid: Vec<&str> = plan
            .known_installations()
            .into_iter()
            .map(|m| m.manager_name())
            .collect();
        return Err(format!(
            "Unknown manager '{}'. Valid manager names for this program: {}",
            via_manager,
            valid.join(", ")
        ));
    }
    if matching_indices.len() > 1 {
        return Err(format!(
            "--via {} is ambiguous for this program (more than one method uses the same manager)",
            via_manager
        ));
    }

    let idx = matching_indices[0];
    if !plan.options[idx].choose {
        let current_reason = plan.options[idx].reason_type;
        if current_reason != InstallPlanReason::LowerPriorityAlternative {
            return Err(format!(
                "--via {} cannot override an unavailable method (reason: {:?})",
                via_manager, current_reason
            ));
        }
    }

    // Un-choose everything, then choose the matched option.
    for o in &mut plan.options {
        if o.choose {
            o.choose = false;
            o.reason_type = InstallPlanReason::LowerPriorityAlternative;
            o.reason = format!("{} was forced via --via", via_manager);
        }
    }
    plan.options[idx].choose = true;
    plan.options[idx].reason_type = InstallPlanReason::Selected;
    plan.options[idx].reason = format!("forced via --via {}", via_manager);
    plan.successful = true;
    Ok(())
}

/// Build a plan for a resolved program, honoring `--force` (cache bypass) and
/// `--no-sudo` (forces `can_sudo = false`). Uses verification-aware detection
/// so the pnpm verified bucket can fire.
pub fn build_plan_for_args(resolved: &ResolvedProgram, args: &InstallCommandArgs) -> InstallPlan {
    use sniff::programs::HostCapabilities;
    let mut host = HostCapabilities::load_or_detect_with_verification(args.force);
    if args.no_sudo {
        host.can_sudo = false;
    }
    use sniff::programs::build_install_plan;
    match resolved {
        ResolvedProgram::Editor(p) => build_install_plan(p, &host),
        ResolvedProgram::Utility(p) => build_install_plan(p, &host),
        ResolvedProgram::LanguagePackageManager(p) => build_install_plan(p, &host),
        ResolvedProgram::OsPackageManager(p) => build_install_plan(p, &host),
        ResolvedProgram::TtsClient(p) => build_install_plan(p, &host),
        ResolvedProgram::TerminalApp(p) => build_install_plan(p, &host),
        ResolvedProgram::HeadlessAudio(p) => build_install_plan(p, &host),
        ResolvedProgram::AiCli(p) => build_install_plan(p, &host),
    }
}

/// Top-level dispatch for `sniff <category> install …` and
/// `sniff <category> install-plan …`.
///
/// `filter` scopes name resolution to the category so that error messages name
/// the correct category (e.g., "Unknown editor" instead of "Unknown program").
/// Pass `OutputFilter::Programs` to search across all categories.
pub fn dispatch(
    kind: InstallCommandKind,
    args: &InstallCommandArgs,
    filter: OutputFilter,
    json: bool,
    plain: bool,
    performance: Option<&sniff::PerformanceReport>,
) -> Result<(), Box<dyn Error>> {
    let name = args
        .program
        .as_deref()
        .ok_or("--program is required for plan-aware install commands")?;

    let resolved = resolve_program_in_category(name, filter).map_err(|e: ResolveError| {
        let boxed: Box<dyn Error> = Box::new(e);
        boxed
    })?;
    let mut plan = build_plan_for_args(&resolved, args);

    if let Some(via) = args.via.as_deref() {
        apply_via(&mut plan, via).map_err(|s| -> Box<dyn Error> { s.into() })?;
    }

    if json {
        let value = serde_json::to_value(&plan)?;
        crate::output::print_json_value(value, performance);
        return Ok(());
    }

    match kind {
        InstallCommandKind::InstallPlan => {
            let terminal = Terminal::new();
            let rendered = render_install_plan_with(&plan, /* verbose */ true, &terminal);
            crate::output::emit_text(&rendered, plain);
            Ok(())
        }
        InstallCommandKind::Install => execute_install_flow(&plan, args.dry_run, args.yes, plain),
    }
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

    #[test]
    fn should_require_remote_bash_consent_returns_true_for_remote_bash() {
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
        assert!(should_require_remote_bash_consent(&plan));
    }

    #[test]
    fn should_require_remote_bash_consent_false_for_brew() {
        let plan = InstallPlan {
            program: "vim".into(),
            website: "https://www.vim.org",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::Brew("vim"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "default OS package manager".into(),
            }],
        };
        assert!(!should_require_remote_bash_consent(&plan));
    }

    fn plan_with_brew_chosen_and_cargo_alternative() -> InstallPlan {
        InstallPlan {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            successful: true,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Brew("bat"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "default OS package manager".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Cargo("bat"),
                    requires_sudo: false,
                    choose: false,
                    reason_type: InstallPlanReason::LowerPriorityAlternative,
                    reason: "brew was chosen".into(),
                },
            ],
        }
    }

    #[test]
    fn apply_via_overrides_chosen_with_lower_priority_alternative() {
        let mut plan = plan_with_brew_chosen_and_cargo_alternative();
        apply_via(&mut plan, "cargo").expect("cargo should be a valid override");
        let chosen = plan.chosen().expect("a chosen option");
        assert!(matches!(chosen.kind, InstallationMethod::Cargo(_)));
        assert_eq!(chosen.reason_type, InstallPlanReason::Selected);
        assert!(plan.successful);
        // Previously-chosen brew becomes a LowerPriorityAlternative.
        let brew = plan
            .options
            .iter()
            .find(|o| matches!(o.kind, InstallationMethod::Brew(_)))
            .unwrap();
        assert!(!brew.choose);
        assert_eq!(
            brew.reason_type,
            InstallPlanReason::LowerPriorityAlternative
        );
    }

    #[test]
    fn apply_via_is_noop_when_manager_already_chosen() {
        let mut plan = plan_with_brew_chosen_and_cargo_alternative();
        apply_via(&mut plan, "brew").expect("brew is already selected and should succeed");
        let chosen = plan.chosen().expect("a chosen option");
        assert!(matches!(chosen.kind, InstallationMethod::Brew(_)));
    }

    #[test]
    fn apply_via_unknown_manager_lists_valid_managers() {
        let mut plan = plan_with_brew_chosen_and_cargo_alternative();
        let err = apply_via(&mut plan, "definitely-fake").unwrap_err();
        assert!(err.to_lowercase().contains("unknown manager"));
        assert!(err.contains("brew"));
        assert!(err.contains("cargo"));
    }

    #[test]
    fn apply_via_rejects_ineligible_method() {
        // Apt is in the plan but blocked by ManagerNotInstalled — --via apt
        // cannot silently override that.
        let mut plan = InstallPlan {
            program: "vim".into(),
            website: "https://www.vim.org",
            successful: true,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Brew("vim"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "default OS package manager".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Apt("vim"),
                    requires_sudo: true,
                    choose: false,
                    reason_type: InstallPlanReason::ManagerNotInstalled,
                    reason: "apt is not installed on this host".into(),
                },
            ],
        };
        let err = apply_via(&mut plan, "apt").unwrap_err();
        assert!(err.contains("cannot override an unavailable method"));
    }

    #[test]
    fn render_install_plan_with_real_terminal_contains_program_and_manager() {
        let plan = fake_success_plan(false);
        let terminal = Terminal::default();
        let rendered = render_install_plan_with(&plan, false, &terminal);
        assert!(rendered.contains("Vim"));
        assert!(rendered.to_lowercase().contains("brew"));
    }

    #[test]
    fn render_install_plan_default_wrapper_still_works() {
        let plan = fake_success_plan(false);
        let rendered = render_install_plan(&plan, false);
        assert!(rendered.contains("Vim"));
    }

    #[test]
    fn execute_install_flow_dry_run_delegates_to_interview_runner() {
        // Drives the flow in dry-run/plain mode. We can't assert on emitted text
        // from execute_install_flow directly (it writes to stdout), but we can
        // observe that the function returns Ok.
        let plan = fake_success_plan(false); // uses Brew, no sudo
        let result = execute_install_flow(
            &plan, /*dry_run*/ true, /*skip_confirm*/ true, /*plain*/ true,
        );
        assert!(
            result.is_ok(),
            "dry-run should succeed, got: {:?}",
            result.err().map(|e| e.to_string())
        );
    }
}
