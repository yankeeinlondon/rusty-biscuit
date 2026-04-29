//! `messenger install` — install missing notification helpers.
//!
//! Builds on `sniff` for detection and the install pipeline. The flow:
//!
//! 1. Detect installed helpers and the host OS.
//! 2. Restrict the candidate set to helpers that apply to the host and are
//!    not yet installed (or to the names passed via `--helper`).
//! 3. Optionally pick a subset interactively via `inquire::MultiSelect`
//!    (skipped when `--yes` is set).
//! 4. Print the install plan with elevation badges.
//! 5. Confirm via `inquire::Confirm` (skipped when `--yes` or `--dry-run`).
//! 6. Execute via sniff's `execute_install` for each helper.
//! 7. Re-detect and print the updated `messenger info` table.

use std::io::Write;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use inquire::{Confirm, MultiSelect};
use sniff::os::OsType;
use sniff::programs::install_plan::{InstallPlan, InstallPlanOption};
use sniff::programs::installer::InstallOptions;
use sniff::programs::types::InstallationMethod;
use sniff::programs::{HostCapabilities, NotificationHelper, ProgramMetadata, build_install_plan};
use strum::IntoEnumIterator;

use crate::config::{Config, parse_helper_name};
use crate::info;

/// Top-level argument bag for the install command.
pub struct InstallArgs {
    pub yes: bool,
    pub helpers: Vec<String>,
    pub dry_run: bool,
}

/// Run the `messenger install` command.
pub fn run(args: InstallArgs) -> Result<()> {
    let term = Terminal::default();

    let host = HostCapabilities::load_or_detect_with_verification(false);
    let detector = sniff::programs::notification_helpers::InstalledNotificationHelpers::new();

    let candidates = candidate_helpers(&detector, host.os_type, &args.helpers)?;
    if candidates.is_empty() {
        println!(
            "{}",
            Prose::new(
                "<green>All applicable notification helpers are already installed.</green>"
            )
            .render(&term)
        );
        return Ok(());
    }

    let selected = if args.yes {
        candidates.clone()
    } else {
        prompt_selection(&candidates)?
    };

    if selected.is_empty() {
        println!(
            "{}",
            Prose::new("<dim>No helpers selected; nothing to do.</dim>").render(&term)
        );
        return Ok(());
    }

    let plans: Vec<(NotificationHelper, InstallPlan)> = selected
        .iter()
        .map(|helper| (*helper, build_install_plan(helper, &host)))
        .collect();

    print_plan_table(&plans, &term);

    if !args.yes && !args.dry_run && !confirm_proceed()? {
        println!(
            "{}",
            Prose::new("<dim>Aborted.</dim>").render(&term)
        );
        return Ok(());
    }

    let opts = InstallOptions {
        dry_run: args.dry_run,
        skip_confirm: true,
        timeout_secs: 120,
        approve_remote_bash: false,
    };

    for (helper, plan) in &plans {
        execute_helper_install(helper, plan, &opts, &term)?;
    }

    println!();
    println!(
        "{}",
        Prose::new("<b>Updated state</b>").render(&term)
    );
    println!();
    let config = Config::load().unwrap_or_default();
    let host_helpers = info::config_helpers_for_host(&config, sniff::os::detect_os_type());
    let report = info::build_report(&config, &host_helpers);
    print!("{}", info::render_text(&report));

    Ok(())
}

/// Build the candidate set for installation.
///
/// When `requested` is empty, the candidate set is "every helper that
/// applies to the current OS and is not yet installed". When `requested`
/// is non-empty, the set is "every name in `requested` that resolves to
/// a known helper, regardless of installed state" (so the user can
/// re-run `install` to repair a broken installation).
fn candidate_helpers(
    detector: &sniff::programs::notification_helpers::InstalledNotificationHelpers,
    os_type: OsType,
    requested: &[String],
) -> Result<Vec<NotificationHelper>> {
    if requested.is_empty() {
        return Ok(NotificationHelper::iter()
            .filter(|h| helper_applies_to_os(*h, os_type))
            .filter(|h| !detector.is_installed(*h))
            .collect());
    }

    let mut resolved: Vec<NotificationHelper> = Vec::new();
    for raw in requested {
        let helper = parse_helper_name(raw)
            .ok_or_else(|| eyre!("unknown notification helper: {raw}"))?;
        if !helper_applies_to_os(helper, os_type) {
            tracing::warn!(
                helper = %helper.binary_name(),
                os = %os_type,
                "skipping helper that does not apply to this host"
            );
            continue;
        }
        if !resolved.contains(&helper) {
            resolved.push(helper);
        }
    }
    Ok(resolved)
}

fn helper_applies_to_os(helper: NotificationHelper, os_type: OsType) -> bool {
    let availability = helper.info().os_availability;
    availability.is_empty() || availability.contains(&os_type)
}

/// Present a `MultiSelect` of candidate helpers and return the user's pick.
fn prompt_selection(candidates: &[NotificationHelper]) -> Result<Vec<NotificationHelper>> {
    let labels: Vec<String> = candidates
        .iter()
        .map(|h| {
            let display = h.display_name();
            let binary = h.binary_name();
            if display == binary {
                display.to_string()
            } else {
                format!("{display} ({binary})")
            }
        })
        .collect();

    let chosen_labels = match MultiSelect::new("Select notification helpers to install:", labels.clone())
        .with_help_message("Space to toggle, Enter to confirm, Esc to skip")
        .prompt()
    {
        Ok(values) => values,
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => Vec::new(),
        Err(error) => return Err(error.into()),
    };

    Ok(chosen_labels
        .into_iter()
        .filter_map(|label| {
            labels
                .iter()
                .position(|l| *l == label)
                .map(|idx| candidates[idx])
        })
        .collect())
}

fn print_plan_table(plans: &[(NotificationHelper, InstallPlan)], term: &Terminal) {
    println!();
    println!("{}", Prose::new("<b>Install plan</b>").render(term));
    for (helper, plan) in plans {
        let chosen = plan.chosen();
        match chosen {
            Some(option) => {
                let badges = badges_for_option(option);
                let summary = format!(
                    "  • <b>{}</b> <dim>via</dim> <green>{}</green>{badges}",
                    helper.display_name(),
                    option.kind.manager_name(),
                );
                println!("{}", Prose::new(summary).render(term));
                let cmd = sniff::programs::installer::get_install_command(&option.kind)
                    .unwrap_or_else(|_| {
                        format!("{} {}", option.kind.manager_name(), option.kind.package_name())
                    });
                println!(
                    "{}",
                    Prose::new(format!("    <dim>$</dim> <dim>{}</dim>", cmd)).render(term)
                );
            }
            None => {
                println!(
                    "{}",
                    Prose::new(format!(
                        "  • <b>{}</b> <red>no installable method on this host</red>",
                        helper.display_name()
                    ))
                    .render(term)
                );
            }
        }
    }
    println!();
}

fn badges_for_option(option: &InstallPlanOption) -> String {
    let mut parts = Vec::new();
    if option.requires_sudo {
        parts.push("<yellow>sudo</yellow>".to_string());
    }
    if matches!(
        option.kind,
        InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
    ) {
        parts.push("<yellow>remote-script</yellow>".to_string());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" <dim>[{}]</dim>", parts.join(" "))
    }
}

fn confirm_proceed() -> Result<bool> {
    match Confirm::new("Proceed with installation?")
        .with_default(true)
        .prompt()
    {
        Ok(value) => Ok(value),
        Err(inquire::InquireError::OperationCanceled)
        | Err(inquire::InquireError::OperationInterrupted) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn execute_helper_install(
    helper: &NotificationHelper,
    plan: &InstallPlan,
    opts: &InstallOptions,
    term: &Terminal,
) -> Result<()> {
    println!();
    let header = format!("<b>Installing {}…</b>", helper.display_name());
    println!("{}", Prose::new(header).render(term));
    std::io::stdout().flush().ok();

    let result = plan.execute(opts);
    match result {
        Ok(install_result) => {
            if !install_result.executed {
                println!(
                    "{}",
                    Prose::new(format!(
                        "  <dim>$ {}</dim> <dim>(dry-run, not executed)</dim>",
                        install_result.command
                    ))
                    .render(term)
                );
                return Ok(());
            }
            println!(
                "{}",
                Prose::new(format!("  <green>✓ {} installed</green>", helper.display_name()))
                    .render(term)
            );
            Ok(())
        }
        Err(error) => {
            println!(
                "{}",
                Prose::new(format!(
                    "  <red>✗ {} failed: {}</red>",
                    helper.display_name(),
                    error
                ))
                .render(term)
            );
            // Continue to the next helper instead of aborting the whole run;
            // a single helper failure should not block other selections.
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_applies_to_os_filters_by_availability() {
        assert!(helper_applies_to_os(NotificationHelper::Dunstify, OsType::Linux));
        assert!(!helper_applies_to_os(NotificationHelper::Dunstify, OsType::Windows));
        assert!(helper_applies_to_os(
            NotificationHelper::TerminalNotifier,
            OsType::MacOS
        ));
    }

    #[test]
    fn candidate_helpers_with_no_request_returns_uninstalled_for_os() {
        let detector =
            sniff::programs::notification_helpers::InstalledNotificationHelpers::default();
        // No helpers installed in the default detector → every host helper
        // for the requested OS is a candidate.
        let candidates = candidate_helpers(&detector, OsType::Linux, &[]).unwrap();
        assert!(candidates.contains(&NotificationHelper::Dunstify));
        assert!(candidates.contains(&NotificationHelper::NotifySend));
        assert!(!candidates.contains(&NotificationHelper::SnoreToast));
    }

    #[test]
    fn candidate_helpers_with_request_resolves_aliases() {
        let detector =
            sniff::programs::notification_helpers::InstalledNotificationHelpers::default();
        let candidates = candidate_helpers(
            &detector,
            OsType::Linux,
            &["notify-send".into(), "dunstify".into()],
        )
        .unwrap();
        assert_eq!(
            candidates,
            vec![NotificationHelper::NotifySend, NotificationHelper::Dunstify]
        );
    }

    #[test]
    fn candidate_helpers_drops_off_os_request() {
        let detector =
            sniff::programs::notification_helpers::InstalledNotificationHelpers::default();
        // Asking for a Windows helper on a Linux host drops the entry.
        let candidates = candidate_helpers(&detector, OsType::Linux, &["snore_toast".into()]).unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidate_helpers_returns_error_for_unknown_name() {
        let detector =
            sniff::programs::notification_helpers::InstalledNotificationHelpers::default();
        let err = candidate_helpers(&detector, OsType::Linux, &["definitely-not".into()])
            .unwrap_err();
        assert!(format!("{err}").contains("unknown"));
    }
}
