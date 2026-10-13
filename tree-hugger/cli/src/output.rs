//! The output modes of the declaration commands and the rule that exactly one
//! is active: default terminal text, `--json`, `--json-full`, `--jsonl`,
//! `--count`, `--stats`, or an explicit `--format`. `--stats` is the one
//! mode that composes, with `--json` (the stats envelope) or a terminal
//! format. The user-facing rules are in `docs/symbol-reports.md`.

use clap::ValueEnum;

/// `--format`: a terminal layout or a machine/document export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum FormatArg {
    /// The default layout without styles, links or wrapping
    Plain,
    /// One table row per declaration: location, kind, visibility, name and
    /// signature
    Table,
    /// Tab-separated declaration rows
    Tsv,
    /// CommonMark declaration report
    Markdown,
    /// Prose markup declaration report
    Prose,
    /// Extended tags navigation file
    Ctags,
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

    /// Output format: `plain`, `table`, `tsv`, `markdown`, `prose` or `ctags`
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
    pub(crate) comments: bool,
    pub(crate) plain: bool,
    /// `--sort` was given (any value).
    pub(crate) sort: bool,
    /// The command is `classes`, which is already an outline.
    pub(crate) classes: bool,
}

impl GivenOutput {
    pub(crate) fn new(json: bool, json_full: bool, args: &ReportArgs, sort: bool, classes: bool, comments: bool, plain: bool) -> Self {
        Self {
            json,
            json_full,
            jsonl: args.jsonl,
            count: args.count,
            stats: args.stats,
            format: args.format,
            signatures: args.signatures,
            tree: args.tree,
            comments,
            plain,
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

/// A non-terminal declaration export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    Tsv,
    Markdown,
    Prose,
    Ctags,
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
    /// A UTF-8 declaration export independent of terminal capabilities.
    Export { format: ExportFormat, signatures: SignatureMode, plain: bool, comments: bool },
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
        if matches!(given.format, Some(FormatArg::Tsv | FormatArg::Markdown | FormatArg::Prose | FormatArg::Ctags)) {
            return Err("`--stats` cannot be combined with `--format`".to_string());
        }
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
            return Err(format!("`--tree` requires a list or outline format and cannot be combined with `{mode}`"));
        }
        if let Some(format @ (FormatArg::Table | FormatArg::Tsv | FormatArg::Ctags)) = given.format {
            let name = match format {
                FormatArg::Table => "table",
                FormatArg::Tsv => "tsv",
                FormatArg::Ctags => "ctags",
                _ => unreachable!(),
            };
            return Err(format!("`--tree` cannot be combined with `--format {name}`"));
        }
    }

    if let Some(format) = given.format {
        let format = match format {
            FormatArg::Tsv => Some(ExportFormat::Tsv),
            FormatArg::Markdown => Some(ExportFormat::Markdown),
            FormatArg::Prose => Some(ExportFormat::Prose),
            FormatArg::Ctags => Some(ExportFormat::Ctags),
            _ => None,
        };
        if let Some(format) = format {
            if format == ExportFormat::Tsv && given.comments {
                return Err("`--comments` cannot be combined with `--format tsv`".to_string());
            }
            if format == ExportFormat::Ctags {
                if given.signatures.is_some() {
                    return Err("`--signatures` cannot be combined with `--format ctags`".to_string());
                }
                if given.comments {
                    return Err("`--comments` cannot be combined with `--format ctags`".to_string());
                }
            }
            return Ok(OutputMode::Export {
                format,
                signatures: given.signatures.unwrap_or_default(),
                plain: given.plain,
                comments: given.comments,
            });
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
            "--format tsv" => given.format = Some(FormatArg::Tsv),
            "--format markdown" => given.format = Some(FormatArg::Markdown),
            "--format prose" => given.format = Some(FormatArg::Prose),
            "--format ctags" => given.format = Some(FormatArg::Ctags),
            "--tree" => given.tree = true,
            "--sort" => given.sort = true,
            "--signatures" => given.signatures = Some(SignatureMode::Full),
            "--comments" => given.comments = true,
            other => panic!("unknown flag {other}"),
        }
        given
    }

    const MODES: [&str; 11] = [
        "--json",
        "--json-full",
        "--jsonl",
        "--count",
        "--stats",
        "--format plain",
        "--format table",
        "--format tsv",
        "--format markdown",
        "--format prose",
        "--format ctags",
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
            OutputMode::Export { format: ExportFormat::Tsv, signatures: SignatureMode::Short, plain: false, comments: false },
            OutputMode::Export { format: ExportFormat::Markdown, signatures: SignatureMode::Short, plain: false, comments: false },
            OutputMode::Export { format: ExportFormat::Prose, signatures: SignatureMode::Short, plain: false, comments: false },
            OutputMode::Export { format: ExportFormat::Ctags, signatures: SignatureMode::Short, plain: false, comments: false },
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
    fn tree_accepts_only_list_and_outline_layouts() {
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

    #[test]
    fn new_formats_reject_every_other_output_mode_and_stats() {
        for format in ["tsv", "markdown", "prose", "ctags"] {
            let format_flag = format!("--format {format}");
            for mode in ["--json", "--json-full", "--jsonl", "--count"] {
                let err = resolve(with(mode, with(&format_flag, GivenOutput::default()))).unwrap_err();
                assert!(err.contains("--format") && err.contains(mode), "{format} with {mode}: {err}");
            }
            let err = resolve(with("--stats", with(&format_flag, GivenOutput::default()))).unwrap_err();
            assert!(err.contains("--stats") && err.contains("--format"), "{format}: {err}");
        }
    }

    #[test]
    fn new_format_tree_and_explicit_flag_rules() {
        for format in ["tsv", "ctags"] {
            let err = resolve(with("--tree", with(&format!("--format {format}"), GivenOutput::default()))).unwrap_err();
            assert!(err.contains("--tree") && err.contains("--format"), "{format}: {err}");
        }
        for format in ["markdown", "prose"] {
            assert!(resolve(with("--tree", with(&format!("--format {format}"), GivenOutput::default()))).is_ok());
        }
        let err = resolve(with("--comments", with("--format tsv", GivenOutput::default()))).unwrap_err();
        assert!(err.contains("--comments") && err.contains("--format tsv"), "{err}");
        for flag in ["--comments", "--signatures"] {
            let err = resolve(with(flag, with("--format ctags", GivenOutput::default()))).unwrap_err();
            assert!(err.contains(flag) && err.contains("--format ctags"), "{err}");
        }
    }

    #[test]
    fn new_formats_accept_plain_and_signatures_where_supported() {
        for (name, format) in [
            ("tsv", ExportFormat::Tsv),
            ("markdown", ExportFormat::Markdown),
            ("prose", ExportFormat::Prose),
            ("ctags", ExportFormat::Ctags),
        ] {
            let flag = format!("--format {name}");
            let plain = GivenOutput { plain: true, ..with(&flag, GivenOutput::default()) };
            assert_eq!(resolve(plain), Ok(OutputMode::Export { format, signatures: SignatureMode::Short, plain: true, comments: false }));
            if format != ExportFormat::Ctags {
                let full = with("--signatures", with(&flag, GivenOutput::default()));
                assert!(matches!(resolve(full), Ok(OutputMode::Export { signatures: SignatureMode::Full, .. })));
            }
        }
    }

    #[test]
    fn help_lists_all_output_formats() {
        use clap::CommandFactory;

        let help = crate::Cli::command()
            .find_subcommand_mut("functions")
            .unwrap()
            .render_long_help()
            .to_string();
        for value in ["plain", "table", "tsv", "markdown", "prose", "ctags"] {
            assert!(help.contains(value), "missing --format value {value} in help");
        }
    }
}
