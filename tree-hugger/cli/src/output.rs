//! The output modes of the declaration commands and the rule that exactly one
//! is active: default terminal text, `--json`, `--json-full`, `--jsonl`,
//! `--count`, `--stats`, or an explicit `--format`. `--stats` is the one
//! mode that composes, with `--json` (the stats envelope) or a terminal
//! format. The user-facing rules are in `docs/symbol-reports.md`.

use clap::ValueEnum;

/// `--format`: the terminal layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum FormatArg {
    /// The default layout without styles, links or wrapping
    Plain,
    /// One table row per declaration: location, kind, visibility, name and
    /// signature
    Table,
}

/// `--signatures`: how much of each declaration's signature is shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum SignatureMode {
    /// The declaration header as written in the source
    Full,
    /// The compact form; type field lists past four entries are shortened
    #[default]
    Short,
    /// Names only
    None,
}

/// The report flags of `functions`, `types`, `symbols` and `classes`.
#[derive(clap::Args, Debug, Clone, Default)]
pub(crate) struct ReportArgs {
    /// Write one JSON object per selected declaration (JSON Lines)
    #[arg(long, display_order = 50)]
    pub(crate) jsonl: bool,

    /// Print only the number of selected declarations
    #[arg(long, display_order = 51)]
    pub(crate) count: bool,

    /// Print counts of the selected declarations by kind, visibility and
    /// language, and their doc coverage (with `--json`, as JSON)
    #[arg(long, display_order = 52)]
    pub(crate) stats: bool,

    /// Terminal layout: `plain` (no styles, links or wrapping) or `table`
    #[arg(long, value_enum, value_name = "FORMAT", display_order = 53)]
    pub(crate) format: Option<FormatArg>,

    /// Signatures shown in text output: `full`, `short` (default) or `none`
    #[arg(long, value_enum, value_name = "MODE", display_order = 54)]
    pub(crate) signatures: Option<SignatureMode>,

    /// Nest members under their container (functions, types and symbols)
    #[arg(long, display_order = 55)]
    pub(crate) tree: bool,
}

/// The output flags of one run, global ones included.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GivenOutput {
    pub(crate) json: bool,
    pub(crate) json_full: bool,
    pub(crate) jsonl: bool,
    pub(crate) count: bool,
    pub(crate) stats: bool,
    pub(crate) format: Option<FormatArg>,
    pub(crate) signatures: Option<SignatureMode>,
    pub(crate) tree: bool,
    /// `--sort` was given (any value).
    pub(crate) sort: bool,
    /// The command is `classes`, which is already an outline.
    pub(crate) classes: bool,
}

impl GivenOutput {
    pub(crate) fn new(json: bool, json_full: bool, args: &ReportArgs, sort: bool, classes: bool) -> Self {
        Self {
            json,
            json_full,
            jsonl: args.jsonl,
            count: args.count,
            stats: args.stats,
            format: args.format,
            signatures: args.signatures,
            tree: args.tree,
            sort,
            classes,
        }
    }
}

/// The declaration listing's layout in terminal output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    List,
    Tree,
    Table,
}

/// Where `--stats` is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatsOutput {
    Json,
    /// The tables, unstyled under `--format plain`.
    Terminal { plain: bool },
}

/// The one active output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputMode {
    /// Terminal text; `plain` is `--format plain`.
    Terminal {
        view: View,
        signatures: SignatureMode,
        plain: bool,
    },
    Json,
    JsonFull,
    JsonLines,
    Count,
    Stats(StatsOutput),
}

/// Resolves the active mode, or the argument error that rejects the
/// combination.
pub(crate) fn resolve(given: GivenOutput) -> Result<OutputMode, String> {
    let modes: Vec<&str> = [
        (given.json, "--json"),
        (given.json_full, "--json-full"),
        (given.jsonl, "--jsonl"),
        (given.count, "--count"),
        (given.format.is_some(), "--format"),
    ]
    .into_iter()
    .filter_map(|(present, flag)| present.then_some(flag))
    .collect();

    if given.stats {
        if let Some(other) = modes.iter().find(|flag| !matches!(**flag, "--json" | "--format")) {
            return Err(format!("`--stats` cannot be combined with `{other}`"));
        }
        if given.json && given.format.is_some() {
            return Err("`--json` and `--format` are different output modes; give only one".to_string());
        }
        for (present, flag) in [
            (given.sort, "--sort"),
            (given.signatures.is_some(), "--signatures"),
            (given.tree, "--tree"),
        ] {
            if present {
                return Err(format!("`{flag}` has no meaning for `--stats`, which reports counts"));
            }
        }
        return Ok(OutputMode::Stats(if given.json {
            StatsOutput::Json
        } else {
            StatsOutput::Terminal {
                plain: given.format == Some(FormatArg::Plain),
            }
        }));
    }

    if let [first, second, ..] = modes.as_slice() {
        return Err(format!("`{first}` and `{second}` are different output modes; give only one"));
    }

    if given.tree {
        if given.classes {
            return Err("`--tree` applies to `functions`, `types` and `symbols`; `classes` already nests members".to_string());
        }
        if let Some(mode) = modes.first().filter(|flag| **flag != "--format") {
            return Err(format!("`--tree` is terminal output and cannot be combined with `{mode}`"));
        }
        if given.format == Some(FormatArg::Table) {
            return Err("`--tree` cannot be combined with `--format table`".to_string());
        }
    }

    Ok(if given.json {
        OutputMode::Json
    } else if given.json_full {
        OutputMode::JsonFull
    } else if given.jsonl {
        OutputMode::JsonLines
    } else if given.count {
        OutputMode::Count
    } else {
        OutputMode::Terminal {
            view: if given.tree {
                View::Tree
            } else if given.format == Some(FormatArg::Table) {
                View::Table
            } else {
                View::List
            },
            signatures: given.signatures.unwrap_or_default(),
            plain: given.format == Some(FormatArg::Plain),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode flags, each set alone on top of a base.
    fn with(flag: &str, mut given: GivenOutput) -> GivenOutput {
        match flag {
            "--json" => given.json = true,
            "--json-full" => given.json_full = true,
            "--jsonl" => given.jsonl = true,
            "--count" => given.count = true,
            "--stats" => given.stats = true,
            "--format plain" => given.format = Some(FormatArg::Plain),
            "--format table" => given.format = Some(FormatArg::Table),
            "--tree" => given.tree = true,
            "--sort" => given.sort = true,
            "--signatures" => given.signatures = Some(SignatureMode::Full),
            other => panic!("unknown flag {other}"),
        }
        given
    }

    const MODES: [&str; 7] = [
        "--json",
        "--json-full",
        "--jsonl",
        "--count",
        "--stats",
        "--format plain",
        "--format table",
    ];

    /// Every pair of mode flags: only `--stats` with `--json` or a format
    /// is accepted (the two formats cannot both be given).
    #[test]
    fn exactly_one_mode_except_stats_with_json_or_a_format() {
        for (index, left) in MODES.iter().enumerate() {
            for right in &MODES[index + 1..] {
                if left.starts_with("--format") && right.starts_with("--format") {
                    continue;
                }
                let given = with(right, with(left, GivenOutput::default()));
                let accepted = *left == "--stats" && matches!(*right, "--json" | "--format plain" | "--format table")
                    || *right == "--stats" && matches!(*left, "--json" | "--format plain" | "--format table");
                assert_eq!(resolve(given).is_ok(), accepted, "{left} with {right}: {:?}", resolve(given));
            }
        }
    }

    #[test]
    fn each_mode_alone_is_accepted() {
        let expected = [
            OutputMode::Json,
            OutputMode::JsonFull,
            OutputMode::JsonLines,
            OutputMode::Count,
            OutputMode::Stats(StatsOutput::Terminal { plain: false }),
            OutputMode::Terminal {
                view: View::List,
                signatures: SignatureMode::Short,
                plain: true,
            },
            OutputMode::Terminal {
                view: View::Table,
                signatures: SignatureMode::Short,
                plain: false,
            },
        ];
        for (flag, mode) in MODES.iter().zip(expected) {
            assert_eq!(resolve(with(flag, GivenOutput::default())), Ok(mode), "{flag}");
        }
        assert_eq!(
            resolve(GivenOutput::default()),
            Ok(OutputMode::Terminal {
                view: View::List,
                signatures: SignatureMode::Short,
                plain: false,
            })
        );
    }

    #[test]
    fn stats_composes_with_json_and_formats_and_rejects_ordering_flags() {
        let stats = with("--stats", GivenOutput::default());
        assert_eq!(resolve(with("--json", stats)), Ok(OutputMode::Stats(StatsOutput::Json)));
        assert_eq!(
            resolve(with("--format plain", stats)),
            Ok(OutputMode::Stats(StatsOutput::Terminal { plain: true }))
        );
        assert_eq!(
            resolve(with("--format table", stats)),
            Ok(OutputMode::Stats(StatsOutput::Terminal { plain: false }))
        );
        assert!(resolve(with("--format table", with("--json", stats))).is_err());
        for flag in ["--sort", "--signatures", "--tree"] {
            assert!(resolve(with(flag, stats)).is_err(), "--stats {flag}");
            assert!(resolve(with(flag, with("--json", stats))).is_err(), "--stats --json {flag}");
        }
    }

    #[test]
    fn tree_is_terminal_list_output_only() {
        let tree = with("--tree", GivenOutput::default());
        assert!(matches!(resolve(tree), Ok(OutputMode::Terminal { view: View::Tree, .. })));
        assert_eq!(
            resolve(with("--format plain", tree)),
            Ok(OutputMode::Terminal {
                view: View::Tree,
                signatures: SignatureMode::Short,
                plain: true,
            })
        );
        assert!(resolve(with("--sort", tree)).is_ok());
        for flag in ["--format table", "--json", "--json-full", "--jsonl", "--count", "--stats"] {
            assert!(resolve(with(flag, tree)).is_err(), "--tree {flag}");
        }
        let classes = GivenOutput {
            classes: true,
            ..tree
        };
        assert!(resolve(classes).is_err(), "classes --tree");
    }
}
