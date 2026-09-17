//! Parser regressions for the commit-family subcommand arguments.
//!
//! Decision 13 of `sniff/features/2026-09-15-recent-commits/spec.md` gives the
//! commit-family subcommands `--verbose`/`-v` and `--compact`/`-c` even though
//! the top-level `Cli` already owns a global `-v`/`--verbose` counter. Clap
//! merges global argument values across every command level **by argument
//! id**, so a subcommand flag cannot independently shadow the global one:
//!
//! - same id, different type (`bool`) panics on access in every invocation;
//! - a different id loses `-v`/`--verbose` to the propagated global flag;
//! - same id and the same `Count` type works, and the two flags unify.
//!
//! The unified shape is harmless because the global counter drives styled
//! output verbosity only; tracing is owned by `--debug` (see `init_tracing`).

use chrono::{Duration, NaiveDate};
use clap::Parser;
use sniff::SniffError;
use sniff::filesystem::git::{
    RecentCommitsOptions, RecentCommitsProjection, RecentCommitsVerbosity,
};
use sniff::filesystem::path_kind::ChangeCategory;

use super::{Cli, Commands, RecentCommitsArgs, RepoSubcommand};

const SUBCOMMANDS: [&str; 3] = [
    "recent-commits",
    "source-code-changes",
    "documentation-changes",
];

fn try_parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("sniff").chain(args.iter().copied()))
}

fn commit_args(cli: Cli) -> RecentCommitsArgs {
    match cli.command {
        Some(Commands::Repo {
            repo_subcommand:
                Some(
                    RepoSubcommand::RecentCommits(args)
                    | RepoSubcommand::SourceCodeChanges(args)
                    | RepoSubcommand::DocumentationChanges(args),
                ),
            ..
        }) => args,
        other => panic!("not a commit-family subcommand: {other:?}"),
    }
}

fn options_for(args: &[&str]) -> RecentCommitsOptions {
    commit_args(try_parse(args).unwrap())
        .to_options(RecentCommitsProjection::All)
        .unwrap()
}

#[test]
fn no_arguments_select_the_last_ten_commits_at_normal_verbosity() {
    for subcommand in SUBCOMMANDS {
        let args = commit_args(try_parse(&["repo", subcommand]).unwrap());
        assert_eq!(args, RecentCommitsArgs::default(), "{subcommand}");
        assert_eq!(
            args.to_options(RecentCommitsProjection::All).unwrap(),
            RecentCommitsOptions::new().count(10),
            "{subcommand}"
        );
    }
}

#[test]
fn every_flag_maps_to_its_library_option() {
    let options = options_for(&[
        "repo",
        "recent-commits",
        "2026-04-01",
        "--operation",
        "feat",
        "--operation",
        "Planning",
        "--scope",
        "cli",
        "--author",
        "ada@example.com",
        "--branch",
        "origin/feature",
        "--package",
        "sniff-cli",
        "--package-area",
        "sniff",
        "--source-code",
        "--web",
        "--images",
        "--documentation",
        "--configuration",
        "--cicd",
        "--show-author",
    ]);

    let expected = RecentCommitsOptions::new()
        .date(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap())
        .operation("feat")
        .operation("Planning")
        .scope("cli")
        .author("ada@example.com")
        .branch("origin/feature")
        .package("sniff-cli")
        .package_area("sniff")
        .has_file_type(ChangeCategory::SourceCode)
        .has_file_type(ChangeCategory::WebAssets)
        .has_file_type(ChangeCategory::Images)
        .has_file_type(ChangeCategory::Documentation)
        .has_file_type(ChangeCategory::Configuration)
        .has_file_type(ChangeCategory::Cicd)
        .show_author(true);
    assert_eq!(options, expected);
}

#[test]
fn each_file_category_flag_selects_only_its_category() {
    for (flag, category) in [
        ("--source-code", ChangeCategory::SourceCode),
        ("--web", ChangeCategory::WebAssets),
        ("--images", ChangeCategory::Images),
        ("--documentation", ChangeCategory::Documentation),
        ("--configuration", ChangeCategory::Configuration),
        ("--cicd", ChangeCategory::Cicd),
    ] {
        assert_eq!(
            options_for(&["repo", "recent-commits", flag]),
            RecentCommitsOptions::new().has_file_type(category),
            "{flag}"
        );
    }
}

#[test]
fn period_forms_parse_through_the_library_selection() {
    assert_eq!(
        options_for(&["repo", "recent-commits", "25"]),
        RecentCommitsOptions::new().count(25)
    );
    assert_eq!(
        options_for(&["repo", "recent-commits", "3d"]),
        RecentCommitsOptions::new().duration(Duration::days(3))
    );
    assert_eq!(
        options_for(&["repo", "recent-commits", "abcdef1"]),
        RecentCommitsOptions::new().hash("abcdef1")
    );

    for period in ["invalid-period", "0"] {
        let args = commit_args(try_parse(&["repo", "recent-commits", period]).unwrap());
        assert!(
            matches!(
                args.to_options(RecentCommitsProjection::All),
                Err(SniffError::InvalidPeriod(_))
            ),
            "{period}"
        );
    }
}

#[test]
fn preset_projection_is_carried_into_the_options() {
    let args = commit_args(try_parse(&["repo", "source-code-changes"]).unwrap());
    assert_eq!(
        args.to_options(RecentCommitsProjection::SourceCode).unwrap(),
        RecentCommitsOptions::new().projection(RecentCommitsProjection::SourceCode)
    );
}

#[test]
fn subcommand_verbose_selects_verbose_report_without_enabling_tracing() {
    for subcommand in SUBCOMMANDS {
        for flag in ["-v", "--verbose"] {
            let cli = try_parse(&["repo", subcommand, flag]).unwrap();
            assert_eq!(cli.debug, 0, "{subcommand} {flag}");
            let options = commit_args(cli)
                .to_options(RecentCommitsProjection::All)
                .unwrap();
            assert_eq!(
                options,
                RecentCommitsOptions::new().verbosity(RecentCommitsVerbosity::Verbose),
                "{subcommand} {flag}"
            );
        }
    }
}

#[test]
fn verbose_is_position_independent_because_the_flags_unify() {
    for args in [
        ["-v", "repo", "recent-commits"],
        ["repo", "-v", "recent-commits"],
        ["repo", "recent-commits", "-v"],
    ] {
        let cli = try_parse(&args).unwrap();
        assert_eq!(cli.verbose, 1, "{args:?}");
        assert_eq!(cli.debug, 0, "{args:?}");
        assert_eq!(commit_args(cli).verbose, 1, "{args:?}");
    }
}

#[test]
fn repeated_verbose_across_positions_does_not_sum() {
    let cli = try_parse(&["-v", "repo", "recent-commits", "-v"]).unwrap();
    assert_eq!(cli.verbose, 1);
    assert_eq!(commit_args(cli).verbose, 1);

    let cli = try_parse(&["repo", "recent-commits", "-vv"]).unwrap();
    assert_eq!(commit_args(cli).verbose, 2);
}

#[test]
fn compact_short_and_long_select_compact_report() {
    for subcommand in SUBCOMMANDS {
        for flag in ["-c", "--compact"] {
            assert_eq!(
                options_for(&["repo", subcommand, flag]),
                RecentCommitsOptions::new().verbosity(RecentCommitsVerbosity::Compact),
                "{subcommand} {flag}"
            );
        }
    }
}

#[test]
fn compact_and_verbose_conflict_within_the_subcommand_in_either_order() {
    for subcommand in SUBCOMMANDS {
        for args in [
            ["repo", subcommand, "-c", "-v"],
            ["repo", subcommand, "--verbose", "--compact"],
        ] {
            let err = try_parse(&args)
                .err()
                .expect("compact and verbose must conflict");
            assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict, "{args:?}");
        }
    }
}

/// Clap does not report this conflict because the verbose value arrives by
/// global propagation, so the adapter resolves it: compact wins.
#[test]
fn global_verbose_with_subcommand_compact_selects_compact() {
    assert_eq!(
        options_for(&["-v", "repo", "recent-commits", "-c"]),
        RecentCommitsOptions::new().verbosity(RecentCommitsVerbosity::Compact)
    );
}

#[test]
fn removed_flags_are_rejected_on_every_commit_family_subcommand() {
    for subcommand in SUBCOMMANDS {
        for removed in [
            &["--action", "feat"][..],
            &["--no-error"][..],
            &["--on-error", "nothing"][..],
        ] {
            let mut args = vec!["repo", subcommand];
            args.extend_from_slice(removed);
            let err = try_parse(&args)
                .err()
                .unwrap_or_else(|| panic!("{args:?} must be rejected"));
            assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument, "{args:?}");
        }
    }
}

#[test]
fn help_advertises_the_new_flags_and_not_the_removed_ones() {
    use clap::CommandFactory;

    for subcommand in SUBCOMMANDS {
        let mut command = Cli::command();
        let help = command
            .find_subcommand_mut("repo")
            .and_then(|repo| repo.find_subcommand_mut(subcommand))
            .expect("subcommand exists")
            .render_long_help()
            .to_string();
        for flag in [
            "--operation",
            "--scope",
            "--author",
            "--branch",
            "--package",
            "--package-area",
            "--source-code",
            "--web",
            "--images",
            "--documentation",
            "--configuration",
            "--cicd",
            "--show-author",
            "--compact",
            "--verbose",
        ] {
            assert!(help.contains(flag), "{subcommand} help lacks {flag}:\n{help}");
        }
        for removed in ["--action", "--no-error", "--on-error", "3d)"] {
            assert!(!help.contains(removed), "{subcommand} help still has {removed}:\n{help}");
        }
    }

    let repo_help = Cli::command()
        .find_subcommand_mut("repo")
        .expect("repo exists")
        .render_long_help()
        .to_string();
    assert!(repo_help.contains("Show the last 10 commits"), "{repo_help}");
    assert!(!repo_help.contains("last 3 days"), "{repo_help}");
}

#[test]
fn operation_completion_suggests_common_values_without_restricting_input() {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    let args = ["sniff", "repo", "recent-commits", "--operation", ""]
        .map(std::ffi::OsString::from)
        .to_vec();
    let candidates = clap_complete::engine::complete(&mut cmd, args, 4, None).unwrap();
    let values: Vec<String> = candidates
        .iter()
        .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
        .collect();
    for common in ["feat", "fix", "chore", "docs"] {
        assert!(values.iter().any(|value| value == common), "{common} missing from {values:?}");
    }

    assert_eq!(
        options_for(&["repo", "recent-commits", "--operation", "planning"]),
        RecentCommitsOptions::new().operation("planning")
    );
}

mod rejected_shapes {
    use clap::{Parser, Subcommand};

    #[derive(Parser, Debug)]
    struct BoolShadowCli {
        #[arg(short, long, action = clap::ArgAction::Count, global = true)]
        verbose: u8,
        #[command(subcommand)]
        command: BoolShadowCommand,
    }

    #[derive(Subcommand, Debug)]
    enum BoolShadowCommand {
        RecentCommits {
            #[arg(short, long)]
            verbose: bool,
        },
    }

    #[test]
    #[should_panic(expected = "Mismatch between definition and access of `verbose`")]
    fn same_id_bool_flag_panics_even_without_verbose_input() {
        let _ = BoolShadowCli::try_parse_from(["sniff", "recent-commits"]);
    }

    #[derive(Parser, Debug)]
    struct DistinctIdCli {
        #[arg(short, long, action = clap::ArgAction::Count, global = true)]
        verbose: u8,
        #[command(subcommand)]
        command: DistinctIdCommand,
    }

    #[derive(Subcommand, Debug)]
    enum DistinctIdCommand {
        RecentCommits {
            #[arg(id = "report_verbose", short, long)]
            verbose: bool,
        },
    }

    #[test]
    fn distinct_id_flag_loses_verbose_to_the_global_counter() {
        let cli = DistinctIdCli::try_parse_from(["sniff", "recent-commits", "--verbose"]).unwrap();
        let DistinctIdCommand::RecentCommits { verbose } = cli.command;
        assert!(!verbose);
        assert_eq!(cli.verbose, 1);
    }
}
