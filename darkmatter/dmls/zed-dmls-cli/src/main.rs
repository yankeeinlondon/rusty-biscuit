use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use zed_dmls_cli::{
    HostDiscovery, PathOverrides, ReportLevel, ReportLine, StageStatus, checked_in_extension_dir,
    default_paths, doctor, registration_path, render_lines, should_run_doctor, stage_extension,
};

#[derive(Debug, Parser)]
#[command(name = "zed-dmls", version, about = "Stage and diagnose the DMLS Zed extension")]
struct Cli {
    #[arg(long, global = true)]
    staging_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    zed_data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    zed_log: Option<PathBuf>,
    #[arg(long, global = true, help = "Emit deterministic plain text")]
    plain: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Stage {
        #[arg(long, help = "Do nothing when no Zed data directory exists")]
        if_zed_present: bool,
    },
    Doctor {
        #[arg(long, help = "Skip diagnostics when no Zed data or DMLS registration exists")]
        if_zed_present: bool,
    },
}

fn main() -> ExitCode {
    if let Err(error) = color_eyre::install() {
        eprintln!("failed to install error reporter: {error}");
        return ExitCode::FAILURE;
    }
    let cli = Cli::parse();
    let plain = cli.plain;
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!(
                "{}",
                render_lines(
                    &[ReportLine {
                        level: ReportLevel::Failure,
                        text: error.to_string(),
                    }],
                    plain,
                )
            );
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let overrides = PathOverrides {
        staging_dir: cli.staging_dir,
        zed_data_dir: cli.zed_data_dir,
        zed_log: cli.zed_log,
    };
    let host = HostDiscovery::capture(&overrides)?;
    let paths = default_paths(host.os, &host.roots, &overrides)?;
    match cli.command {
        Command::Stage { if_zed_present } => {
            if if_zed_present && !paths.zed_data_dir.exists() {
                return Ok(ExitCode::SUCCESS);
            }
            let report = stage_extension(&checked_in_extension_dir(), &paths)?;
            let mut lines = vec![ReportLine {
                level: ReportLevel::Success,
                text: format!("staged the DMLS Zed extension at `{}`", report.staging_dir.display()),
            }];
            let code = match report.status {
                StageStatus::AlreadyRegistered => {
                    lines.push(ReportLine {
                        level: ReportLevel::Success,
                        text: format!(
                            "Zed's dev-extension registration `{}` already points at it",
                            registration_path(&paths).display()
                        ),
                    });
                    ExitCode::SUCCESS
                }
                StageStatus::Registered => {
                    lines.push(ReportLine {
                        level: ReportLevel::Success,
                        text: format!(
                            "registered it as Zed's `dmls` dev extension at `{}`; restart Zed if it is running",
                            registration_path(&paths).display()
                        ),
                    });
                    ExitCode::SUCCESS
                }
                StageStatus::ManualRegistrationRequired(reason) => {
                    lines.push(ReportLine {
                        level: ReportLevel::Warning,
                        text: format!(
                            "manual registration required ({reason}): in Zed run `zed: install dev extension` and select `{}`",
                            report.staging_dir.display()
                        ),
                    });
                    ExitCode::from(3)
                }
            };
            println!("{}", render_lines(&lines, cli.plain));
            Ok(code)
        }
        Command::Doctor { if_zed_present } => {
            if if_zed_present && !should_run_doctor(&paths) {
                return Ok(ExitCode::SUCCESS);
            }
            let report = doctor(&paths, &host);
            println!("{}", render_lines(&report.lines, cli.plain));
            Ok(if report.healthy {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_overrides_before_or_after_subcommand() {
        let before = Cli::try_parse_from([
            "zed-dmls",
            "--plain",
            "--staging-dir",
            "stage-dir",
            "doctor",
            "--if-zed-present",
            "--zed-log",
            "Zed.log",
        ])
        .unwrap();
        assert!(before.plain);
        assert_eq!(before.staging_dir, Some(PathBuf::from("stage-dir")));
        assert_eq!(before.zed_log, Some(PathBuf::from("Zed.log")));
        assert!(matches!(
            before.command,
            Command::Doctor {
                if_zed_present: true
            }
        ));
    }
}
