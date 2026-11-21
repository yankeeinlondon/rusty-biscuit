//! What Sniff knows about formatters and language conventions: the files
//! each formatter reads at the worktree root, in Sniff's search order, the
//! file types it formats, and its defaults.
//!
//! Row order is rank order: an earlier row's opinion outranks a later one's.
//! See `sniff/docs/topics/indentation.md`.

use super::{IndentAuthority, IndentStyle};

/// The root `.editorconfig`, the only EditorConfig file Sniff reads.
pub(crate) const EDITORCONFIG: &str = ".editorconfig";

/// How a candidate file's bytes are parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialect {
    /// Strict JSON: no comments or trailing commas.
    Json,
    /// JSON5, which also reads JSONC.
    Json5,
    Toml,
    /// One YAML document; a second document is an error.
    Yaml,
    /// Every document of a YAML stream (clang-format).
    YamlDocuments,
    /// An extensionless file: strict JSON when its first non-whitespace
    /// character is `{` or `[`, YAML otherwise.
    JsonOrYaml,
}

/// What makes a parsed candidate declare its formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Declaration {
    /// The file exists for this formatter, so it declares the row.
    Bare,
    /// A shared manifest declares the row only through this key path
    /// (`package.json`'s `prettier`, `pyproject.toml`'s `tool.ruff`).
    Key(&'static [&'static str]),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Candidate {
    /// Root-relative path with `/` separators.
    pub(crate) path: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) declaration: Declaration,
}

/// One formatter row of the specification's table.
#[derive(Debug)]
pub(crate) struct FormatterRow {
    pub(crate) authority: IndentAuthority,
    /// Lowercase file extensions the formatter's indentation applies to.
    pub(crate) extensions: &'static [&'static str],
    /// Readable configuration files, in Sniff's search order.
    pub(crate) candidates: &'static [Candidate],
    /// Executable configurations Sniff never runs. Each one that exists is
    /// reported, even beside a selected local file.
    pub(crate) executable: &'static [&'static str],
    pub(crate) default_style: IndentStyle,
    pub(crate) default_width: u8,
}

impl FormatterRow {
    pub(crate) fn applies_to(&self, extension: &str) -> bool {
        self.extensions.contains(&extension)
    }
}

const fn bare(path: &'static str, dialect: Dialect) -> Candidate {
    Candidate { path, dialect, declaration: Declaration::Bare }
}

const fn keyed(path: &'static str, dialect: Dialect, key: &'static [&'static str]) -> Candidate {
    Candidate { path, dialect, declaration: Declaration::Key(key) }
}

/// The JavaScript and TypeScript family every JS formatter row shares.
const JS: [&str; 8] = ["js", "jsx", "ts", "tsx", "mjs", "cjs", "mts", "cts"];

/// Concatenates extension lists at compile time.
macro_rules! extensions {
    ($($part:expr),* $(,)?) => {{
        const PARTS: &[&[&str]] = &[$(&$part),*];
        const LEN: usize = {
            let mut total = 0;
            let mut index = 0;
            while index < PARTS.len() {
                total += PARTS[index].len();
                index += 1;
            }
            total
        };
        const ALL: [&str; LEN] = {
            let mut out = [""; LEN];
            let mut slot = 0;
            let mut part = 0;
            while part < PARTS.len() {
                let mut item = 0;
                while item < PARTS[part].len() {
                    out[slot] = PARTS[part][item];
                    slot += 1;
                    item += 1;
                }
                part += 1;
            }
            out
        };
        &ALL
    }};
}

use Dialect::{Json, Json5, JsonOrYaml, Toml, Yaml, YamlDocuments};

/// Every supported formatter, in rank order.
pub(crate) const ROWS: &[FormatterRow] = &[
    FormatterRow {
        authority: IndentAuthority::Rustfmt,
        extensions: &["rs"],
        candidates: &[bare(".rustfmt.toml", Toml), bare("rustfmt.toml", Toml)],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 4,
    },
    FormatterRow {
        authority: IndentAuthority::Prettier,
        extensions: extensions!(
            JS,
            ["json", "jsonc", "json5", "yaml", "yml", "md", "mdx", "css", "scss", "less", "graphql", "gql", "html", "vue"]
        ),
        candidates: &[
            keyed("package.json", Json, &["prettier"]),
            keyed("package.yaml", Yaml, &["prettier"]),
            bare(".prettierrc", JsonOrYaml),
            bare(".prettierrc.json", Json),
            bare(".prettierrc.yaml", Yaml),
            bare(".prettierrc.yml", Yaml),
            bare(".prettierrc.json5", Json5),
            bare(".prettierrc.toml", Toml),
        ],
        executable: &[
            ".prettierrc.js",
            ".prettierrc.cjs",
            ".prettierrc.mjs",
            ".prettierrc.ts",
            ".prettierrc.cts",
            ".prettierrc.mts",
            "prettier.config.js",
            "prettier.config.cjs",
            "prettier.config.mjs",
            "prettier.config.ts",
            "prettier.config.cts",
            "prettier.config.mts",
        ],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Biome,
        extensions: extensions!(JS, ["json", "jsonc", "css", "graphql", "gql"]),
        candidates: &[
            bare("biome.json", Json),
            bare("biome.jsonc", Json5),
            bare(".biome.json", Json),
            bare(".biome.jsonc", Json5),
        ],
        executable: &[],
        default_style: IndentStyle::Tabs,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Deno,
        extensions: extensions!(
            JS,
            ["json", "jsonc", "yaml", "yml", "md", "css", "scss", "sass", "less", "html", "vue"]
        ),
        candidates: &[bare("deno.json", Json), bare("deno.jsonc", Json5)],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Dprint,
        extensions: extensions!(JS, ["json", "jsonc"]),
        candidates: &[
            bare("dprint.json", Json),
            bare("dprint.jsonc", Json5),
            bare(".dprint.json", Json),
            bare(".dprint.jsonc", Json5),
        ],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Oxfmt,
        extensions: extensions!(
            JS,
            [
                "json", "jsonc", "json5", "yaml", "yml", "toml", "md", "mdx", "css", "scss", "less", "graphql",
                "gql", "html", "vue"
            ]
        ),
        candidates: &[bare(".oxfmtrc.json", Json), bare(".oxfmtrc.jsonc", Json5)],
        executable: &["oxfmt.config.ts", "oxfmt.config.mts"],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Ruff,
        extensions: &["py", "pyi"],
        candidates: &[
            bare(".ruff.toml", Toml),
            bare("ruff.toml", Toml),
            keyed("pyproject.toml", Toml, &["tool", "ruff"]),
        ],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 4,
    },
    FormatterRow {
        authority: IndentAuthority::Tombi,
        extensions: &["toml"],
        candidates: &[
            bare(".tombi.toml", Toml),
            bare("tombi.toml", Toml),
            bare(".config/tombi.toml", Toml),
            keyed("pyproject.toml", Toml, &["tool", "tombi"]),
        ],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Taplo,
        extensions: &["toml"],
        candidates: &[bare(".taplo.toml", Toml), bare("taplo.toml", Toml)],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Yamlfmt,
        extensions: &["yaml", "yml"],
        candidates: &[
            bare(".yamlfmt", Yaml),
            bare(".yamlfmt.yaml", Yaml),
            bare(".yamlfmt.yml", Yaml),
            bare("yamlfmt.yaml", Yaml),
            bare("yamlfmt.yml", Yaml),
        ],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Stylua,
        extensions: &["lua", "luau"],
        candidates: &[bare("stylua.toml", Toml), bare(".stylua.toml", Toml)],
        executable: &[],
        default_style: IndentStyle::Tabs,
        default_width: 4,
    },
    FormatterRow {
        authority: IndentAuthority::ClangFormat,
        extensions: &["c", "h", "cc", "cpp", "cxx", "hh", "hpp", "hxx", "m", "mm", "proto"],
        candidates: &[bare(".clang-format", YamlDocuments), bare("_clang-format", YamlDocuments)],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::SwiftFormat,
        extensions: &["swift"],
        candidates: &[bare(".swift-format", Json)],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Rubocop,
        extensions: &["rb", "rake", "gemspec", "ru"],
        candidates: &[
            bare(".rubocop.yml", Yaml),
            bare(".config/.rubocop.yml", Yaml),
            bare(".config/rubocop/config.yml", Yaml),
        ],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 2,
    },
    FormatterRow {
        authority: IndentAuthority::Csharpier,
        extensions: &["cs", "csx"],
        candidates: &[
            bare(".csharpierrc", JsonOrYaml),
            bare(".csharpierrc.json", Json),
            bare(".csharpierrc.yaml", Yaml),
        ],
        executable: &[],
        default_style: IndentStyle::Spaces,
        default_width: 4,
    },
];

/// A built-in language convention.
#[derive(Debug)]
pub(crate) struct Convention {
    /// Reported in the opinion's `via`.
    pub(crate) name: &'static str,
    /// Lowercase file extensions.
    pub(crate) extensions: &'static [&'static str],
    /// Lowercase basename suffixes, matched only in path lookup.
    pub(crate) suffixes: &'static [&'static str],
    /// Exact basenames, matched only in path lookup.
    pub(crate) basenames: &'static [&'static str],
    pub(crate) style: IndentStyle,
    pub(crate) width: u8,
}

pub(crate) const CONVENTIONS: &[Convention] = &[
    // gofmt writes one tab per level and aligns as if a tab were 8 columns.
    convention("gofmt", &["go"], IndentStyle::Tabs, 8),
    convention("rustfmt", &["rs"], IndentStyle::Spaces, 4),
    convention("PEP 8", &["py", "pyi"], IndentStyle::Spaces, 4),
    convention("zig fmt", &["zig"], IndentStyle::Spaces, 4),
    convention("dart format", &["dart"], IndentStyle::Spaces, 2),
    convention("mix format", &["ex", "exs"], IndentStyle::Spaces, 2),
    Convention {
        name: "terraform fmt",
        extensions: &["tf", "tfvars"],
        suffixes: &[".tftest.hcl", ".tfmock.hcl", ".tfquery.hcl"],
        basenames: &[],
        style: IndentStyle::Spaces,
        width: 2,
    },
    convention("elm-format", &["elm"], IndentStyle::Spaces, 4),
    convention("gleam format", &["gleam"], IndentStyle::Spaces, 2),
    // A tab is Make's recipe prefix; 8 is the tab width the tab displays at.
    Convention {
        name: "make",
        extensions: &["mk"],
        suffixes: &[],
        basenames: &["Makefile", "makefile", "GNUmakefile"],
        style: IndentStyle::Tabs,
        width: 8,
    },
];

const fn convention(
    name: &'static str,
    extensions: &'static [&'static str],
    style: IndentStyle,
    width: u8,
) -> Convention {
    Convention { name, extensions, suffixes: &[], basenames: &[], style, width }
}

/// File types whose format forbids tab indentation.
pub(crate) const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];

/// Every candidate path any row reads, deduplicated, in first-seen order.
pub(crate) fn all_candidate_paths() -> Vec<(&'static str, Dialect)> {
    let mut paths: Vec<(&'static str, Dialect)> = Vec::new();
    for row in ROWS {
        for candidate in row.candidates {
            if !paths.iter().any(|(path, _)| *path == candidate.path) {
                paths.push((candidate.path, candidate.dialect));
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_path_is_read_in_one_dialect_by_every_row() {
        for row in ROWS {
            for candidate in row.candidates {
                for other in ROWS.iter().flat_map(|row| row.candidates) {
                    if other.path == candidate.path {
                        assert_eq!(other.dialect, candidate.dialect, "{} has two dialects", candidate.path);
                    }
                }
            }
        }
    }

    #[test]
    fn extension_lists_are_lowercase_and_concatenated_in_order() {
        let prettier = &ROWS[1];
        assert_eq!(prettier.authority, IndentAuthority::Prettier);
        assert_eq!(&prettier.extensions[..9], &["js", "jsx", "ts", "tsx", "mjs", "cjs", "mts", "cts", "json"]);
        assert_eq!(prettier.extensions.last(), Some(&"vue"));
        for row in ROWS {
            for extension in row.extensions {
                assert_eq!(*extension, extension.to_ascii_lowercase());
            }
        }
    }
}
