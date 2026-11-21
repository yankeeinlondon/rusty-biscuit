//! `sniff::filesystem::indentation` through its public API: fixture
//! worktrees on disk, every formatter row, the shared field validators, each
//! document loader, and repository discovery. No formatter binary runs; each
//! row's control fixture follows that formatter's documented configuration
//! example.

use std::path::Path;

use sniff::filesystem::indentation::{
    AdjustmentReason, IndentAuthority, IndentStyle, IndentationDiagnostic, RepoIndentation, ResolvedIndentation,
    normalize_extension, resolve_for_extension, resolve_for_path, resolve_summary,
};
use tempfile::TempDir;

use IndentStyle::{Spaces, Tabs};

/// A temporary directory, optionally a Git worktree, with files written in.
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn git() -> Self {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        Self { dir }
    }

    fn plain() -> Self {
        Self { dir: TempDir::new().unwrap() }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn with(self, relative: &str, content: &str) -> Self {
        write(self.root(), relative, content);
        self
    }

    fn ext(&self, ext: &str) -> ResolvedIndentation {
        resolve_for_extension(self.root(), ext).unwrap()
    }

    fn path(&self, relative: &str) -> ResolvedIndentation {
        resolve_for_path(&self.root().join(relative)).unwrap()
    }
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn value(resolved: &ResolvedIndentation) -> (IndentStyle, u8) {
    (resolved.indent_using, resolved.indent.get())
}

fn opinion_value(opinion: &sniff::filesystem::indentation::IndentOpinion) -> (Option<IndentStyle>, Option<u8>) {
    (opinion.indent_using, opinion.indent.map(|width| width.get()))
}

/// `(path, key)` of every diagnostic, for compact assertions.
fn skipped(resolved: &ResolvedIndentation) -> Vec<(String, Option<String>)> {
    resolved
        .skipped
        .iter()
        .map(|diagnostic| (portable(&diagnostic.path), diagnostic.key.clone()))
        .collect()
}

fn portable(path: &Path) -> String {
    path.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

fn has_skip(resolved: &ResolvedIndentation, path: &str, key: Option<&str>) -> bool {
    skipped(resolved).contains(&(path.to_string(), key.map(str::to_string)))
}

fn reason<'a>(resolved: &'a ResolvedIndentation, path: &str) -> &'a str {
    resolved
        .skipped
        .iter()
        .find(|diagnostic| portable(&diagnostic.path) == path)
        .map_or_else(|| panic!("no diagnostic for {path}: {:?}", resolved.skipped), |diagnostic| diagnostic.reason.as_str())
}

/// This repository's root `.editorconfig`, copied into fixtures.
const REPOSITORY_EDITORCONFIG: &str = include_str!("../../../../.editorconfig");

// ---------------------------------------------------------------------------
// The specification's success criteria
// ---------------------------------------------------------------------------

#[test]
fn this_repositorys_editorconfig_gives_yaml_four_spaces_from_editorconfig() {
    let fixture = Fixture::git().with(".editorconfig", REPOSITORY_EDITORCONFIG);
    let yaml = fixture.ext("yaml");
    assert_eq!(value(&yaml), (Spaces, 4));
    assert_eq!(yaml.authority.source, IndentAuthority::EditorConfig);
    assert_eq!(yaml.authority.path.as_deref(), Some(Path::new(".editorconfig")));
    assert_eq!(yaml.authority.via, ["*", "*.{js,ts,jsx,tsx,vue,json,yaml,yml,toml}"]);
    assert!(yaml.consensus);
    assert_eq!(yaml.indentation(), biscuit_file::Indentation::default());
}

#[test]
fn a_rustfmt_tab_spaces_of_two_gives_rust_two_spaces_and_yaml_the_default() {
    let fixture = Fixture::git().with("rustfmt.toml", "tab_spaces = 2\n");
    let rs = fixture.ext("rs");
    assert_eq!(value(&rs), (Spaces, 2));
    assert_eq!(rs.authority.source, IndentAuthority::Rustfmt);
    assert_eq!(rs.authority.path.as_deref(), Some(Path::new("rustfmt.toml")));
    assert_eq!(rs.authority.via, ["tab_spaces"]);
    // A formatter opinion suppresses the rustfmt convention.
    assert!(rs.alt_authorities.is_empty());

    let yaml = fixture.ext("yaml");
    assert_eq!(value(&yaml), (Spaces, 4));
    assert_eq!(yaml.authority.source, IndentAuthority::Default);
    assert!(yaml.alt_authorities.is_empty());
    assert!(yaml.consensus);
}

#[test]
fn a_bare_biome_json_gives_tabs_and_editorconfig_outranks_it_without_consensus() {
    let fixture = Fixture::git().with("biome.json", "{\"linter\": {\"enabled\": true}}");
    let ts = fixture.ext("ts");
    assert_eq!(value(&ts), (Tabs, 2));
    assert_eq!(ts.authority.source, IndentAuthority::Biome);
    assert!(ts.authority.via.is_empty(), "defaults supplied every value");

    let fixture = fixture.with(".editorconfig", REPOSITORY_EDITORCONFIG);
    let ts = fixture.ext(".ts");
    assert_eq!(value(&ts), (Spaces, 4));
    assert_eq!(ts.authority.source, IndentAuthority::EditorConfig);
    assert_eq!(ts.alt_authorities.len(), 1);
    assert_eq!(ts.alt_authorities[0].source, IndentAuthority::Biome);
    assert_eq!(opinion_value(&ts.alt_authorities[0]), (Some(Tabs), Some(2)));
    assert!(!ts.consensus);

    // The JSON shape the specification shows.
    let json = serde_json::to_value(&ts).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "indent_using": "space",
            "indent": 4,
            "authority": {
                "source": "editorconfig",
                "path": ".editorconfig",
                "via": ["*", "*.{js,ts,jsx,tsx,vue,json,yaml,yml,toml}"],
                "indent_using": "space",
                "indent": 4
            },
            "alt_authorities": [
                {"source": "biome", "path": "biome.json", "via": [], "indent_using": "tab", "indent": 2}
            ],
            "consensus": false,
            "skipped": []
        })
    );
    let back: ResolvedIndentation = serde_json::from_value(json).unwrap();
    assert_eq!(back, ts);
}

// ---------------------------------------------------------------------------
// EditorConfig
// ---------------------------------------------------------------------------

#[test]
fn a_style_only_section_takes_its_width_from_the_next_opinion_and_is_partial() {
    let fixture = Fixture::git()
        .with(".editorconfig", "[*]\nindent_style = space\n")
        .with(".prettierrc.json", r#"{"tabWidth": 3}"#);
    let ts = fixture.ext("ts");
    assert_eq!(value(&ts), (Spaces, 3));
    assert_eq!(opinion_value(&ts.authority), (Some(Spaces), None));
    assert_eq!(ts.alt_authorities[0].source, IndentAuthority::Prettier);
    assert!(ts.consensus, "the partial opinion agrees on the one value it states");

    let json = serde_json::to_value(&ts.authority).unwrap();
    assert!(json.get("indent").is_none(), "an unstated value is omitted: {json}");
}

#[test]
fn a_size_only_section_takes_its_style_from_the_next_opinion_or_the_fallback() {
    let fixture = Fixture::git().with(".editorconfig", "[*.go]\nindent_size = 2\n[*.txt]\nindent_size = 3\n");
    let go = fixture.ext("go");
    assert_eq!(value(&go), (Tabs, 2), "gofmt supplies tabs");
    assert!(!go.consensus, "gofmt's width 8 disagrees");
    assert_eq!(value(&fixture.ext("txt")), (Spaces, 3));
}

#[test]
fn unset_clears_an_earlier_value_and_the_clearing_section_is_listed() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_style = tab\nindent_size = 2\n[*.md]\nindent_size = unset\nindent_style = unset\n");
    let md = fixture.ext("md");
    assert_eq!(md.authority.source, IndentAuthority::Default);
    assert_eq!(value(&md), (Spaces, 4));
    assert_eq!(value(&fixture.ext("txt")), (Tabs, 2));

    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_style = tab\nindent_size = 2\n[*.md]\nindent_size = unset\n");
    let md = fixture.ext("md");
    assert_eq!(opinion_value(&md.authority), (Some(Tabs), None));
    assert_eq!(md.authority.via, ["*", "*.md"]);
}

#[test]
fn indent_size_tab_uses_tab_width_whatever_the_style() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_style = space\nindent_size = tab\ntab_width = 6\n");
    assert_eq!(value(&fixture.ext("txt")), (Spaces, 6));
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_size = tab\n");
    let txt = fixture.ext("txt");
    assert_eq!(txt.authority.source, IndentAuthority::Default, "no tab_width: normal fallback");
}

#[test]
fn tab_style_without_a_size_uses_tab_width_and_a_space_style_does_not() {
    let tabs = Fixture::git().with(".editorconfig", "[*]\nindent_style = tab\ntab_width = 8\n").ext("txt");
    assert_eq!(value(&tabs), (Tabs, 8));
    let spaces = Fixture::git().with(".editorconfig", "[*]\nindent_style = space\ntab_width = 8\n").ext("txt");
    assert_eq!(opinion_value(&spaces.authority), (Some(Spaces), None));
    assert_eq!(value(&spaces), (Spaces, 4));
}

#[test]
fn a_numeric_indent_size_wins_over_a_different_tab_width() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_style = tab\nindent_size = 4\ntab_width = 8\n");
    assert_eq!(value(&fixture.ext("txt")), (Tabs, 4));
}

#[test]
fn an_invalid_setting_is_diagnosed_and_leaves_the_earlier_value_in_place() {
    let fixture = Fixture::git().with(
        ".editorconfig",
        "[*]\nindent_style = space\nindent_size = 2\n[*.txt]\nindent_size = 0\nindent_style = tabs\ntab_width = 300\n",
    );
    let txt = fixture.ext("txt");
    assert_eq!(value(&txt), (Spaces, 2));
    assert_eq!(
        skipped(&txt),
        [
            (".editorconfig".to_string(), Some("indent_size".to_string())),
            (".editorconfig".to_string(), Some("indent_style".to_string())),
            (".editorconfig".to_string(), Some("tab_width".to_string())),
        ]
    );
    assert!(txt.skipped[0].reason.contains("line 5 in [*.txt]"), "{}", txt.skipped[0].reason);
    assert_eq!(txt.authority.via, ["*"], "a section with only invalid settings is not listed");
}

#[test]
fn an_invalid_section_pattern_is_diagnosed_and_matches_nothing() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_size = 2\n[*.txt\\]\nindent_size = 8\n");
    let txt = fixture.ext("txt");
    assert_eq!(value(&txt), (Spaces, 2));
    assert!(has_skip(&txt, ".editorconfig", Some("[*.txt\\]")), "{:?}", txt.skipped);
}

#[test]
fn repeated_assignments_merge_in_order_and_lines_without_an_assignment_are_ignored() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_size = 2\nnot an assignment\nindent_size = 3\n");
    let txt = fixture.ext("txt");
    assert_eq!(value(&txt), (Spaces, 3));
    assert!(txt.skipped.is_empty());
}

#[test]
fn path_scoped_and_filename_sections_apply_to_paths_but_not_to_extension_lookups() {
    let fixture = Fixture::git().with(
        ".editorconfig",
        "[*]\nindent_size = 4\n[docs/**.md]\nindent_size = 2\n[Makefile]\nindent_style = space\n[file.*]\nindent_size = 7\n",
    );
    assert_eq!(value(&fixture.path("docs/guide/intro.md")), (Spaces, 2));
    assert_eq!(value(&fixture.path("README.md")), (Spaces, 4));
    assert_eq!(value(&fixture.ext("md")), (Spaces, 4), "directory rules are left to path lookup");
    // `[file.*]` would match the synthetic `file.md` basename; it is not a
    // suffix rule, so it is excluded.
    assert_eq!(fixture.ext("md").authority.via, ["*"]);
    let makefile = fixture.path("build/Makefile");
    assert_eq!(makefile.authority.via, ["*", "Makefile"]);
}

#[test]
fn sections_match_forward_slash_paths_on_every_platform() {
    let fixture = Fixture::git().with(".editorconfig", "[src/nested/*.rs]\nindent_size = 3\n");
    // Built with the host separator; matched with `/`.
    let path = fixture.root().join("src").join("nested").join("lib.rs");
    assert_eq!(resolve_for_path(&path).unwrap().indent.get(), 3);
    assert_eq!(fixture.path("src/other/lib.rs").authority.via, ["rustfmt"]);
}

#[test]
fn section_matching_and_the_extension_query_keep_their_case() {
    let fixture = Fixture::git().with(".editorconfig", "[*.MD]\nindent_size = 2\n").with(".prettierrc.json", "{}");
    assert_eq!(fixture.ext("md").authority.source, IndentAuthority::Prettier);
    let upper = fixture.ext("MD");
    assert_eq!(upper.authority.source, IndentAuthority::EditorConfig);
    // File-type selection is case-insensitive: Prettier still applies.
    assert_eq!(upper.alt_authorities[0].source, IndentAuthority::Prettier);
}

// ---------------------------------------------------------------------------
// Each formatter row: a documented-example control, and a file that states
// no indentation (the formatter's default applies, not Sniff's fallback)
// ---------------------------------------------------------------------------

macro_rules! row {
    ($name:ident, $file:literal, $content:expr, $ext:literal, $source:ident, ($style:ident, $width:literal), [$($via:literal),*]) => {
        #[test]
        fn $name() {
            let resolved = Fixture::git().with($file, $content).ext($ext);
            assert_eq!(resolved.authority.source, IndentAuthority::$source, "{resolved:?}");
            assert_eq!(resolved.authority.path.as_deref(), Some(Path::new($file)));
            assert_eq!(value(&resolved), ($style, $width));
            let via: Vec<&str> = vec![$($via),*];
            assert_eq!(resolved.authority.via, via);
            assert!(resolved.skipped.is_empty(), "{:?}", resolved.skipped);
        }
    };
}

row!(rustfmt_control, "rustfmt.toml", "hard_tabs = true\ntab_spaces = 2\nedition = \"2021\"\n", "rs", Rustfmt, (Tabs, 2), ["hard_tabs", "tab_spaces"]);
row!(rustfmt_defaults, ".rustfmt.toml", "edition = \"2021\"\n", "rs", Rustfmt, (Spaces, 4), []);
row!(
    prettier_control,
    ".prettierrc.json",
    r#"{"trailingComma": "es5", "tabWidth": 4, "semi": false, "singleQuote": true}"#,
    "ts", Prettier, (Spaces, 4), ["tabWidth"]
);
row!(prettier_defaults, ".prettierrc.yaml", "semi: false\n", "css", Prettier, (Spaces, 2), []);
row!(
    biome_control,
    "biome.json",
    r#"{"$schema": "https://biomejs.dev/schemas/2.0.0/schema.json", "formatter": {"enabled": true, "indentStyle": "space", "indentWidth": 4}, "javascript": {"formatter": {"quoteStyle": "double"}}}"#,
    "ts", Biome, (Spaces, 4), ["formatter.enabled", "formatter.indentStyle", "formatter.indentWidth"]
);
row!(biome_defaults, "biome.jsonc", "// lint only\n{\"linter\": {\"enabled\": true},}", "json", Biome, (Tabs, 2), []);
row!(
    deno_control,
    "deno.json",
    r#"{"fmt": {"useTabs": true, "lineWidth": 80, "indentWidth": 4, "semiColons": false, "singleQuote": true, "proseWrap": "preserve"}}"#,
    "md", Deno, (Tabs, 4), ["fmt.useTabs", "fmt.indentWidth"]
);
row!(deno_defaults, "deno.jsonc", "{\"tasks\": {\"dev\": \"deno run main.ts\"}}", "ts", Deno, (Spaces, 2), []);
row!(
    dprint_control,
    "dprint.json",
    r#"{"lineWidth": 80, "typescript": {"indentWidth": 4}, "json": {}, "excludes": [], "plugins": ["https://plugins.dprint.dev/typescript-0.x.wasm"]}"#,
    "tsx", Dprint, (Spaces, 4), ["typescript.indentWidth"]
);
row!(dprint_defaults, ".dprint.jsonc", "{\"plugins\": []}", "json", Dprint, (Spaces, 2), []);
row!(
    oxfmt_control,
    ".oxfmtrc.json",
    r#"{"$schema": "./node_modules/oxfmt/configuration_schema.json", "printWidth": 100, "useTabs": true, "tabWidth": 4}"#,
    "toml", Oxfmt, (Tabs, 4), ["useTabs", "tabWidth"]
);
row!(oxfmt_defaults, ".oxfmtrc.jsonc", "{/* nothing */}", "vue", Oxfmt, (Spaces, 2), []);
row!(
    ruff_control,
    "ruff.toml",
    "line-length = 88\nindent-width = 2\n\n[format]\nquote-style = \"double\"\nindent-style = \"tab\"\n",
    "py", Ruff, (Tabs, 2), ["indent-width", "format.indent-style"]
);
row!(ruff_defaults, "pyproject.toml", "[tool.ruff]\nline-length = 100\n", "pyi", Ruff, (Spaces, 4), []);
row!(
    tombi_control,
    ".tombi.toml",
    "toml-version = \"v1.0.0\"\n\n[format.rules]\nindent-style = \"space\"\nindent-width = 4\n",
    "toml", Tombi, (Spaces, 4), ["format.rules.indent-style", "format.rules.indent-width"]
);
row!(tombi_defaults, ".config/tombi.toml", "toml-version = \"v1.0.0\"\n", "toml", Tombi, (Spaces, 2), []);
row!(
    taplo_control,
    ".taplo.toml",
    "[formatting]\nalign_entries = false\nindent_string = \"    \"\n",
    "toml", Taplo, (Spaces, 4), ["formatting.indent_string"]
);
row!(taplo_defaults, "taplo.toml", "include = [\"**/*.toml\"]\n", "toml", Taplo, (Spaces, 2), []);
row!(
    yamlfmt_control,
    ".yamlfmt",
    "formatter:\n  type: basic\n  indent: 4\n  include_document_start: true\n",
    "yml", Yamlfmt, (Spaces, 4), ["formatter.indent"]
);
row!(yamlfmt_defaults, "yamlfmt.yml", "formatter:\n  type: basic\n", "yaml", Yamlfmt, (Spaces, 2), []);
row!(
    stylua_control,
    "stylua.toml",
    "column_width = 120\nline_endings = \"Unix\"\nindent_type = \"Spaces\"\nindent_width = 2\nquote_style = \"AutoPreferDouble\"\n",
    "lua", Stylua, (Spaces, 2), ["indent_type", "indent_width"]
);
row!(stylua_defaults, ".stylua.toml", "column_width = 120\n", "luau", Stylua, (Tabs, 4), []);
row!(
    clang_format_control,
    ".clang-format",
    "---\n# We'll use defaults from the LLVM style, but with 4 columns indentation.\nBasedOnStyle: LLVM\nIndentWidth: 4\n---\nLanguage: Cpp\n# Force pointers to the type for C++.\nDerivePointerAlignment: false\nPointerAlignment: Left\n---\nLanguage: JavaScript\nColumnLimit: 100\n---\nLanguage: Proto\nDisableFormat: true\n...\n",
    "cpp", ClangFormat, (Spaces, 4), ["IndentWidth", "BasedOnStyle"]
);
row!(clang_format_defaults, "_clang-format", "ColumnLimit: 100\n", "c", ClangFormat, (Spaces, 2), []);
row!(
    swift_format_control,
    ".swift-format",
    r#"{"version": 1, "lineLength": 100, "indentation": {"spaces": 4}, "maximumBlankLines": 1, "respectsExistingLineBreaks": true}"#,
    "swift", SwiftFormat, (Spaces, 4), ["indentation.spaces"]
);
row!(swift_format_defaults, ".swift-format", r#"{"version": 1}"#, "swift", SwiftFormat, (Spaces, 2), []);
row!(
    rubocop_control,
    ".rubocop.yml",
    "AllCops:\n  NewCops: enable\nLayout/IndentationWidth:\n  Width: 4\nLayout/IndentationStyle:\n  EnforcedStyle: tabs\n",
    "rb", Rubocop, (Tabs, 4), ["Layout/IndentationStyle.EnforcedStyle", "Layout/IndentationWidth.Width"]
);
row!(rubocop_defaults, ".config/rubocop/config.yml", "AllCops:\n  NewCops: enable\n", "gemspec", Rubocop, (Spaces, 2), []);
row!(
    csharpier_control,
    ".csharpierrc.json",
    r#"{"printWidth": 100, "useTabs": true, "indentSize": 2, "endOfLine": "auto"}"#,
    "cs", Csharpier, (Tabs, 2), ["useTabs", "indentSize"]
);
row!(csharpier_defaults, ".csharpierrc.yaml", "printWidth: 100\n", "csx", Csharpier, (Spaces, 4), []);

/// Every row has its pair above; this list keeps a new row from being
/// added without one.
#[test]
fn every_formatter_authority_has_a_control_and_a_defaults_fixture() {
    let covered = [
        IndentAuthority::Rustfmt,
        IndentAuthority::Prettier,
        IndentAuthority::Biome,
        IndentAuthority::Deno,
        IndentAuthority::Dprint,
        IndentAuthority::Oxfmt,
        IndentAuthority::Ruff,
        IndentAuthority::Tombi,
        IndentAuthority::Taplo,
        IndentAuthority::Yamlfmt,
        IndentAuthority::Stylua,
        IndentAuthority::ClangFormat,
        IndentAuthority::SwiftFormat,
        IndentAuthority::Rubocop,
        IndentAuthority::Csharpier,
    ];
    // An exhaustive match: a new authority fails to compile here until it is
    // listed (and given fixtures).
    for authority in covered {
        match authority {
            IndentAuthority::EditorConfig | IndentAuthority::Convention | IndentAuthority::Default => unreachable!(),
            IndentAuthority::Rustfmt
            | IndentAuthority::Prettier
            | IndentAuthority::Biome
            | IndentAuthority::Deno
            | IndentAuthority::Dprint
            | IndentAuthority::Oxfmt
            | IndentAuthority::Ruff
            | IndentAuthority::Tombi
            | IndentAuthority::Taplo
            | IndentAuthority::Yamlfmt
            | IndentAuthority::Stylua
            | IndentAuthority::ClangFormat
            | IndentAuthority::SwiftFormat
            | IndentAuthority::Rubocop
            | IndentAuthority::Csharpier => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in conventions
// ---------------------------------------------------------------------------

#[test]
fn each_convention_applies_when_no_formatter_speaks() {
    let fixture = Fixture::git();
    for (ext, name, expected) in [
        ("go", "gofmt", (Tabs, 8)),
        ("rs", "rustfmt", (Spaces, 4)),
        ("py", "PEP 8", (Spaces, 4)),
        ("pyi", "PEP 8", (Spaces, 4)),
        ("zig", "zig fmt", (Spaces, 4)),
        ("dart", "dart format", (Spaces, 2)),
        ("ex", "mix format", (Spaces, 2)),
        ("exs", "mix format", (Spaces, 2)),
        ("tf", "terraform fmt", (Spaces, 2)),
        ("tfvars", "terraform fmt", (Spaces, 2)),
        ("elm", "elm-format", (Spaces, 4)),
        ("gleam", "gleam format", (Spaces, 2)),
        ("mk", "make", (Tabs, 8)),
    ] {
        let resolved = fixture.ext(ext);
        assert_eq!(resolved.authority.source, IndentAuthority::Convention, "{ext}");
        assert_eq!(resolved.authority.path, None);
        assert_eq!(resolved.authority.via, [name], "{ext}");
        assert_eq!(value(&resolved), expected, "{ext}");
    }
}

#[test]
fn exact_filename_and_path_suffix_conventions_match_only_in_path_lookup() {
    let fixture = Fixture::git();
    for name in ["Makefile", "makefile", "GNUmakefile"] {
        assert_eq!(fixture.path(name).authority.via, ["make"], "{name}");
    }
    assert_eq!(fixture.path("MAKEFILE").authority.source, IndentAuthority::Default);
    let tftest = fixture.path("infra/main.tftest.hcl");
    assert_eq!(tftest.authority.via, ["terraform fmt"]);
    assert_eq!(fixture.path("infra/main.TFMOCK.hcl").authority.via, ["terraform fmt"]);
    assert_eq!(fixture.ext("hcl").authority.source, IndentAuthority::Default);
    assert_eq!(fixture.path("infra/other.hcl").authority.source, IndentAuthority::Default);
}

#[test]
fn a_makefile_reports_the_projects_general_preference_without_an_adjustment() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_style = space\nindent_size = 2\n");
    let makefile = fixture.path("Makefile");
    assert_eq!(value(&makefile), (Spaces, 2));
    assert_eq!(makefile.adjustment, None);
    assert_eq!(makefile.alt_authorities[0].via, ["make"]);
    assert!(!makefile.consensus);
}

#[test]
fn a_formatter_without_an_opinion_falls_back_to_the_convention() {
    // `pyproject.toml` without `[tool.ruff]` declares nothing.
    let py = Fixture::git().with("pyproject.toml", "[project]\nname = \"x\"\n").ext("py");
    assert_eq!(py.authority.via, ["PEP 8"]);
    assert!(py.skipped.is_empty());
    // A malformed rustfmt file is skipped; the convention still applies.
    let rs = Fixture::git().with("rustfmt.toml", "tab_spaces = \n").ext("rs");
    assert_eq!(rs.authority.via, ["rustfmt"]);
    assert!(reason(&rs, "rustfmt.toml").contains("not valid TOML"));
}

// ---------------------------------------------------------------------------
// Ranking, filling, and consensus
// ---------------------------------------------------------------------------

#[test]
fn formatters_rank_in_table_order_and_values_fill_from_the_highest_opinion_stating_them() {
    let fixture = Fixture::git()
        .with(".prettierrc.json", r#"{"useTabs": true, "tabWidth": 8}"#)
        .with("biome.json", r#"{"formatter": {"indentStyle": "space"}}"#)
        .with("deno.json", "{}");
    let ts = fixture.ext("ts");
    let sources: Vec<_> = std::iter::once(&ts.authority).chain(&ts.alt_authorities).map(|opinion| opinion.source).collect();
    assert_eq!(sources, [IndentAuthority::Prettier, IndentAuthority::Biome, IndentAuthority::Deno]);
    assert_eq!(value(&ts), (Tabs, 8));
    assert!(!ts.consensus);
}

#[test]
fn opinions_that_agree_on_every_value_they_state_are_a_consensus() {
    let fixture = Fixture::git()
        .with(".editorconfig", "[*]\nindent_style = space\nindent_size = 2\n")
        .with(".prettierrc.json", "{}")
        .with("deno.json", "{}");
    let ts = fixture.ext("ts");
    assert_eq!(ts.alt_authorities.len(), 2);
    assert!(ts.consensus);
}

#[test]
fn the_yaml_constraint_turns_tabs_into_spaces_after_ranking_and_consensus() {
    let fixture = Fixture::git()
        .with(".editorconfig", "[*.yaml]\nindent_style = tab\nindent_size = 2\n")
        .with(".prettierrc.json", r#"{"useTabs": true, "tabWidth": 2}"#);
    let yaml = fixture.ext("yaml");
    assert_eq!(value(&yaml), (Spaces, 2));
    let adjustment = yaml.adjustment.unwrap();
    assert_eq!(adjustment.requested, Tabs);
    assert_eq!(adjustment.reason, AdjustmentReason::YamlForbidsTabs);
    assert_eq!(opinion_value(&yaml.authority), (Some(Tabs), Some(2)), "opinions keep their own values");
    assert!(yaml.consensus, "computed on the opinions, before adjustment");
    assert_eq!(yaml.indentation(), biscuit_file::Indentation::spaces(std::num::NonZeroU8::new(2).unwrap()));
    let json = serde_json::to_value(&yaml).unwrap();
    assert_eq!(json["adjustment"], serde_json::json!({"requested": "tab", "reason": "yaml_forbids_tabs"}));

    // Other formats keep tabs.
    let json_ext = Fixture::git().with(".editorconfig", "[*]\nindent_style = tab\n").ext("json");
    assert_eq!(json_ext.indent_using, Tabs);
    assert_eq!(json_ext.adjustment, None);
}

// ---------------------------------------------------------------------------
// Candidate selection: shadowing, disabled formatters, unsupported forms
// ---------------------------------------------------------------------------

#[test]
fn later_candidates_in_a_row_are_reported_as_shadowed() {
    let fixture = Fixture::git()
        .with(".rustfmt.toml", "tab_spaces = 2\n")
        .with("rustfmt.toml", "tab_spaces = 8\n");
    let rs = fixture.ext("rs");
    assert_eq!(rs.indent.get(), 2);
    assert_eq!(skipped(&rs), [("rustfmt.toml".to_string(), None)]);
    assert_eq!(reason(&rs, "rustfmt.toml"), "is shadowed by `.rustfmt.toml`");
}

#[test]
fn prettier_reads_package_json_before_package_yaml_and_skips_a_manifest_without_its_key() {
    let fixture = Fixture::git()
        .with("package.json", r#"{"name": "x", "prettier": {"tabWidth": 3}}"#)
        .with("package.yaml", "prettier:\n  tabWidth: 5\n");
    let ts = fixture.ext("ts");
    assert_eq!(ts.authority.path.as_deref(), Some(Path::new("package.json")));
    assert_eq!(ts.authority.via, ["prettier.tabWidth"]);
    assert_eq!(ts.indent.get(), 3);
    assert!(has_skip(&ts, "package.yaml", None));

    let fixture = Fixture::git()
        .with("package.json", r#"{"name": "x"}"#)
        .with(".prettierrc", "tabWidth: 5\n");
    let ts = fixture.ext("ts");
    assert_eq!(ts.authority.path.as_deref(), Some(Path::new(".prettierrc")));
    assert!(ts.skipped.is_empty(), "a manifest without the key is not shadowed: {:?}", ts.skipped);
}

#[test]
fn a_string_prettier_key_is_an_unsupported_preset_that_gives_no_opinion() {
    let fixture = Fixture::git()
        .with("package.json", r#"{"prettier": "@company/prettier-config"}"#)
        .with(".prettierrc.json", r#"{"tabWidth": 8}"#);
    let ts = fixture.ext("ts");
    assert_eq!(ts.authority.source, IndentAuthority::Default);
    assert!(reason(&ts, "package.json").contains("shared preset `@company/prettier-config`"));
    assert!(has_skip(&ts, ".prettierrc.json", None), "the preset still shadows: {:?}", ts.skipped);
}

#[test]
fn a_wrong_type_declaration_is_skipped_and_the_row_continues() {
    let fixture = Fixture::git()
        .with("package.json", r#"{"prettier": 42}"#)
        .with(".prettierrc.json", r#"{"tabWidth": 8}"#);
    let ts = fixture.ext("ts");
    assert_eq!(ts.authority.path.as_deref(), Some(Path::new(".prettierrc.json")));
    assert_eq!(ts.indent.get(), 8);
    assert!(has_skip(&ts, "package.json", Some("prettier")));
    assert!(reason(&ts, "package.json").contains("must be a mapping, found the integer `42`"));

    let py = Fixture::git().with("ruff.toml", "x = 1\n").with("pyproject.toml", "[tool]\nruff = 3\n").ext("py");
    assert_eq!(py.authority.path.as_deref(), Some(Path::new("ruff.toml")), "ruff.toml selects first");
    let py = Fixture::git().with("pyproject.toml", "[tool]\nruff = 3\n").ext("py");
    assert!(has_skip(&py, "pyproject.toml", Some("tool.ruff")));
    assert_eq!(py.authority.via, ["PEP 8"]);
}

#[test]
fn executable_configurations_are_reported_even_beside_a_selected_local_file() {
    let fixture = Fixture::git()
        .with("prettier.config.mjs", "export default { tabWidth: 8 };\n")
        .with(".prettierrc.json", r#"{"tabWidth": 3}"#)
        .with("oxfmt.config.ts", "export default {};\n");
    let ts = fixture.ext("ts");
    assert_eq!(ts.indent.get(), 3);
    assert!(has_skip(&ts, "prettier.config.mjs", None));
    assert!(has_skip(&ts, "oxfmt.config.ts", None));
    assert!(reason(&ts, "prettier.config.mjs").contains("executable configuration"));

    let alone = Fixture::git().with(".prettierrc.cjs", "module.exports = {};\n").ext("ts");
    assert_eq!(alone.authority.source, IndentAuthority::Default, "an executable config alone gives no opinion");
}

#[test]
fn a_disabled_biome_language_wins_over_global_enablement_and_still_shadows() {
    let fixture = Fixture::git()
        .with("biome.json", r#"{"formatter": {"enabled": true, "indentStyle": "space"}, "javascript": {"formatter": {"enabled": false}}}"#)
        .with(".biome.json", r#"{"formatter": {"indentWidth": 8}}"#);
    let ts = fixture.ext("ts");
    assert_eq!(ts.authority.source, IndentAuthority::Default);
    assert!(has_skip(&ts, ".biome.json", None));
    // JSON keeps the global switch.
    let json = fixture.ext("json");
    assert_eq!(json.authority.source, IndentAuthority::Biome);
    assert_eq!(value(&json), (Spaces, 2));

    let disabled = Fixture::git().with("biome.json", r#"{"formatter": {"enabled": false}, "css": {"formatter": {"enabled": true}}}"#);
    assert_eq!(disabled.ext("js").authority.source, IndentAuthority::Default);
    assert_eq!(disabled.ext("css").authority.source, IndentAuthority::Biome);
}

#[test]
fn biome_and_dprint_language_blocks_layer_over_the_global_settings() {
    let biome = Fixture::git().with(
        "biome.json",
        r#"{"formatter": {"indentStyle": "space", "indentWidth": 4}, "json": {"formatter": {"indentWidth": 2}}, "graphql": {"formatter": {"indentStyle": "tab"}}}"#,
    );
    assert_eq!(value(&biome.ext("jsonc")), (Spaces, 2));
    assert_eq!(value(&biome.ext("gql")), (Tabs, 4));
    assert_eq!(value(&biome.ext("ts")), (Spaces, 4));

    let dprint = Fixture::git().with("dprint.json", r#"{"useTabs": true, "indentWidth": 3, "json": {"useTabs": false}}"#);
    assert_eq!(value(&dprint.ext("json")), (Spaces, 3));
    assert_eq!(value(&dprint.ext("ts")), (Tabs, 3));
}

#[test]
fn unsupported_inheritance_and_path_overrides_are_diagnosed_and_local_values_still_apply() {
    let rubocop = Fixture::git()
        .with(".rubocop.yml", "inherit_from: ../.rubocop.yml\ninherit_gem:\n  shared: config.yml\nLayout/IndentationWidth:\n  Width: 4\n")
        .ext("rb");
    assert_eq!(value(&rubocop), (Spaces, 4));
    assert!(has_skip(&rubocop, ".rubocop.yml", Some("inherit_from")));
    assert!(has_skip(&rubocop, ".rubocop.yml", Some("inherit_gem")));

    let biome = Fixture::git()
        .with("biome.json", r#"{"extends": ["./shared.json"], "overrides": [{"includes": ["*.ts"], "formatter": {"indentWidth": 8}}], "formatter": {"indentWidth": 3}}"#)
        .ext("ts");
    assert_eq!(value(&biome), (Tabs, 3));
    assert!(has_skip(&biome, "biome.json", Some("extends")));
    assert!(has_skip(&biome, "biome.json", Some("overrides")));

    for (file, content, ext, key) in [
        ("dprint.json", r#"{"extends": "https://x/y.json"}"#, "ts", "extends"),
        (".csharpierrc.json", r#"{"overrides": [{"files": "*.cs"}]}"#, "cs", "overrides"),
        ("taplo.toml", "[[rule]]\ninclude = [\"**/Cargo.toml\"]\n", "toml", "rule"),
        ("tombi.toml", "[[overrides]]\nfiles.include = [\"a.toml\"]\n", "toml", "overrides"),
    ] {
        let resolved = Fixture::git().with(file, content).ext(ext);
        assert!(has_skip(&resolved, file, Some(key)), "{file}: {:?}", resolved.skipped);
        assert_eq!(resolved.authority.path.as_deref(), Some(Path::new(file)), "{file} still gives its opinion");
    }
}

#[test]
fn deno_reads_legacy_fmt_options_only_where_the_flat_key_is_absent() {
    let legacy = Fixture::git().with("deno.json", r#"{"fmt": {"options": {"useTabs": true, "indentWidth": 8}}}"#).ext("ts");
    assert_eq!(value(&legacy), (Tabs, 8));
    assert_eq!(legacy.authority.via, ["fmt.options.useTabs", "fmt.options.indentWidth"]);
    let mixed = Fixture::git()
        .with("deno.json", r#"{"fmt": {"indentWidth": 3, "options": {"useTabs": true, "indentWidth": 8}}}"#)
        .ext("ts");
    assert_eq!(value(&mixed), (Tabs, 3));
}

#[test]
fn value_mappings_for_tombi_taplo_swift_format_and_clang_format() {
    let tombi = Fixture::git().with("tombi.toml", "[format.rules]\nindent-style = \"tab\"\nindent-width = 8\n").ext("toml");
    assert_eq!(value(&tombi), (Tabs, 2), "Tombi's width counts spaces only");
    let taplo = Fixture::git().with("taplo.toml", "[formatting]\nindent_string = \"\\t\"\n").ext("toml");
    assert_eq!(value(&taplo), (Tabs, 2));
    let mixed = Fixture::git().with("taplo.toml", "[formatting]\nindent_string = \" \\t\"\n").ext("toml");
    assert_eq!(value(&mixed), (Spaces, 2));
    assert!(has_skip(&mixed, "taplo.toml", Some("formatting.indent_string")));

    let swift = Fixture::git().with(".swift-format", r#"{"indentation": {"tabs": 1}}"#).ext("swift");
    assert_eq!(value(&swift), (Tabs, 8), "one tab times the default tabWidth of 8");
    let swift = Fixture::git().with(".swift-format", r#"{"indentation": {"tabs": 2}, "tabWidth": 4}"#).ext("swift");
    assert_eq!(value(&swift), (Tabs, 8));
    let both = Fixture::git().with(".swift-format", r#"{"indentation": {"tabs": 1, "spaces": 2}}"#).ext("swift");
    assert!(has_skip(&both, ".swift-format", Some("indentation")));
    assert_eq!(value(&both), (Spaces, 2));

    for (mode, expected) in [
        ("Never", Spaces),
        ("ForIndentation", Tabs),
        ("ForContinuationAndIndentation", Tabs),
        ("AlignWithSpaces", Tabs),
        ("Always", Tabs),
    ] {
        let resolved = Fixture::git().with(".clang-format", &format!("UseTab: {mode}\n")).ext("c");
        assert_eq!(resolved.indent_using, expected, "{mode}");
    }
    let unknown = Fixture::git().with(".clang-format", "BasedOnStyle: WebKit\nUseTab: Sometimes\n").ext("c");
    assert_eq!(value(&unknown), (Spaces, 4), "the preset's style and width");
    assert!(has_skip(&unknown, ".clang-format", Some("UseTab")));
    let preset = Fixture::git().with(".clang-format", "BasedOnStyle: InheritParentConfig\n").ext("h");
    assert_eq!(value(&preset), (Spaces, 2), "LLVM defaults");
    assert!(has_skip(&preset, ".clang-format", Some("BasedOnStyle")));
    for (style, width) in [("llvm", 2), ("Google", 2), ("Chromium", 2), ("Mozilla", 2), ("GNU", 2), ("WebKit", 4), ("Microsoft", 4)] {
        let resolved = Fixture::git().with(".clang-format", &format!("BasedOnStyle: {style}\n")).ext("cc");
        assert_eq!(value(&resolved), (Spaces, width), "{style}");
    }
}

#[test]
fn clang_format_layers_the_language_document_over_the_unqualified_one() {
    let config = "---\nBasedOnStyle: Microsoft\nUseTab: Always\n---\nLanguage: ObjC\nIndentWidth: 8\n---\nLanguage: Proto\nUseTab: Never\nIndentWidth: 3\n---\nLanguage: Proto\nIndentWidth: 9\n...\n";
    let fixture = Fixture::git().with(".clang-format", config);
    assert_eq!(value(&fixture.ext("m")), (Tabs, 8));
    assert_eq!(value(&fixture.ext("mm")), (Tabs, 8));
    let proto = fixture.ext("proto");
    assert_eq!(value(&proto), (Spaces, 3), "the first Proto document wins");
    assert!(has_skip(&proto, ".clang-format", Some("Language")));
    assert_eq!(value(&fixture.ext("cpp")), (Tabs, 4), "Cpp: unqualified document and its preset");

    let other_language = Fixture::git()
        .with(".clang-format", "Language: JavaScript\nIndentWidth: 8\n")
        .with("_clang-format", "IndentWidth: 6\n");
    let cpp = other_language.ext("cpp");
    assert_eq!(cpp.authority.source, IndentAuthority::Default, "no applicable document: no opinion");
    assert!(has_skip(&cpp, "_clang-format", None), "the selected file still shadows");
}

// ---------------------------------------------------------------------------
// Prettier and oxfmt overrides
// ---------------------------------------------------------------------------

#[test]
fn matching_overrides_apply_in_file_order_with_later_values_winning_per_property() {
    let fixture = Fixture::git().with(
        ".prettierrc.json",
        r#"{"tabWidth": 2, "overrides": [
            {"files": "*.ts", "options": {"tabWidth": 4, "useTabs": true}},
            {"files": ["legacy/**/*.ts"], "excludeFiles": "legacy/keep/**", "options": {"tabWidth": 8}}
        ]}"#,
    );
    assert_eq!(value(&fixture.path("src/a.ts")), (Tabs, 4));
    assert_eq!(value(&fixture.path("legacy/old/a.ts")), (Tabs, 8));
    assert_eq!(value(&fixture.path("legacy/keep/a.ts")), (Tabs, 4), "excluded from the second entry");
    assert_eq!(value(&fixture.path("src/a.js")), (Spaces, 2));
    let legacy = fixture.path("legacy/old/a.ts");
    assert_eq!(
        legacy.authority.via,
        ["tabWidth", "overrides[0].options.useTabs", "overrides[0].options.tabWidth", "overrides[1].options.tabWidth"]
    );

    // An extension lookup applies only the suffix-only entry; the path entry
    // is deferred whole.
    assert_eq!(value(&fixture.ext("ts")), (Tabs, 4));
}

#[test]
fn oxfmt_overrides_use_the_same_rules() {
    let fixture = Fixture::git().with(".oxfmtrc.json", r#"{"overrides": [{"files": ["*.json"], "options": {"tabWidth": 4}}]}"#);
    assert_eq!(value(&fixture.ext("json")), (Spaces, 4));
    assert_eq!(value(&fixture.ext("ts")), (Spaces, 2));
}

// ---------------------------------------------------------------------------
// Field shapes, once per shared validator (`.prettierrc.json` / biome.json)
// ---------------------------------------------------------------------------

fn prettier(content: &str) -> ResolvedIndentation {
    Fixture::git().with(".prettierrc.json", content).ext("ts")
}

#[test]
fn the_width_validator_accepts_exact_integers_from_1_through_255() {
    assert_eq!(value(&prettier(r#"{"tabWidth": 1}"#)), (Spaces, 1));
    assert_eq!(value(&prettier(r#"{"tabWidth": 255}"#)), (Spaces, 255));
    let absent = prettier("{}");
    assert_eq!(value(&absent), (Spaces, 2), "absent takes the row default");
    assert!(absent.skipped.is_empty());
    for invalid in ["null", "\"4\"", "4.0", "4.5", "0", "256", "-1", "true", "[4]", "{}"] {
        let resolved = prettier(&format!(r#"{{"tabWidth": {invalid}}}"#));
        assert_eq!(value(&resolved), (Spaces, 2), "{invalid}");
        assert!(has_skip(&resolved, ".prettierrc.json", Some("tabWidth")), "{invalid}: {:?}", resolved.skipped);
        assert!(resolved.authority.via.is_empty(), "{invalid}");
    }
    assert!(reason(&prettier(r#"{"tabWidth": null}"#), ".prettierrc.json").contains("found null"));
}

#[test]
fn the_boolean_validator_accepts_only_booleans() {
    assert_eq!(value(&prettier(r#"{"useTabs": true}"#)), (Tabs, 2));
    assert_eq!(value(&prettier(r#"{"useTabs": false}"#)), (Spaces, 2));
    for invalid in ["null", "\"true\"", "1", "[]"] {
        let resolved = prettier(&format!(r#"{{"useTabs": {invalid}}}"#));
        assert_eq!(value(&resolved), (Spaces, 2), "{invalid}");
        assert!(has_skip(&resolved, ".prettierrc.json", Some("useTabs")), "{invalid}");
    }
}

#[test]
fn the_style_validator_accepts_only_the_formatters_exact_spellings() {
    let biome = |style: &str| Fixture::git().with("biome.json", &format!(r#"{{"formatter": {{"indentStyle": {style}}}}}"#)).ext("ts");
    assert_eq!(biome("\"space\"").indent_using, Spaces);
    assert_eq!(biome("\"tab\"").indent_using, Tabs);
    for invalid in ["null", "\"Tab\"", "\"tabs\"", "\"\"", "1", "true"] {
        let resolved = biome(invalid);
        assert_eq!(value(&resolved), (Tabs, 2), "{invalid}");
        assert!(has_skip(&resolved, "biome.json", Some("formatter.indentStyle")), "{invalid}: {:?}", resolved.skipped);
    }
}

#[test]
fn a_nested_settings_block_of_the_wrong_type_is_diagnosed_and_defaults_apply() {
    for invalid in ["null", "3", "[]", "\"x\""] {
        let resolved = Fixture::git().with("biome.json", &format!(r#"{{"formatter": {invalid}}}"#)).ext("ts");
        assert_eq!(value(&resolved), (Tabs, 2), "{invalid}");
        assert!(has_skip(&resolved, "biome.json", Some("formatter")), "{invalid}");
    }
    let empty = Fixture::git().with("biome.json", r#"{"formatter": {}}"#).ext("ts");
    assert!(empty.skipped.is_empty(), "an empty block is valid and states nothing");
    let empty_declaration = Fixture::git().with(".prettierrc.json", "{}").ext("ts");
    assert_eq!(empty_declaration.authority.source, IndentAuthority::Prettier, "an empty declaring object selects defaults");
}

fn override_files(files: &str) -> ResolvedIndentation {
    let content = format!(
        r#"{{"overrides": [{{"files": {files}, "options": {{"tabWidth": 4}}}}, {{"files": "*.ts", "options": {{"useTabs": true}}}}]}}"#
    );
    Fixture::git().with(".prettierrc.json", &content).path("src/a.ts")
}

#[test]
fn the_glob_field_validator_takes_a_glob_or_a_non_empty_list_and_one_bad_element_spoils_it() {
    assert_eq!(value(&override_files("\"*.ts\"")), (Tabs, 4));
    assert_eq!(value(&override_files("[\"*.js\", \"src/*.ts\"]")), (Tabs, 4));
    for invalid in ["null", "123", "\"\"", "[]", "[\"*.ts\", 1]", "[1]", "[\"\"]", "\"[\"", "{}"] {
        let resolved = override_files(invalid);
        // The invalid entry is dropped; the other entry still applies.
        assert_eq!(value(&resolved), (Tabs, 2), "{invalid}");
        assert!(has_skip(&resolved, ".prettierrc.json", Some("overrides[0].files")), "{invalid}: {:?}", resolved.skipped);
    }
}

#[test]
fn override_entries_are_validated_one_by_one() {
    let entries = |entries: &str| Fixture::git().with(".prettierrc.json", &format!(r#"{{"overrides": {entries}}}"#)).path("a.ts");
    let missing_files = entries(r#"[{"options": {"tabWidth": 4}}, {"files": "*.ts", "options": {"useTabs": true}}]"#);
    assert_eq!(value(&missing_files), (Tabs, 2));
    assert!(has_skip(&missing_files, ".prettierrc.json", Some("overrides[0]")));
    let not_mapping = entries(r#"[3, {"files": "*.ts", "options": {"useTabs": true}}]"#);
    assert_eq!(value(&not_mapping), (Tabs, 2));
    assert!(has_skip(&not_mapping, ".prettierrc.json", Some("overrides[0]")));
    let bad_exclude = entries(r#"[{"files": "*.ts", "excludeFiles": [], "options": {"tabWidth": 4}}]"#);
    assert_eq!(value(&bad_exclude), (Spaces, 2));
    assert!(has_skip(&bad_exclude, ".prettierrc.json", Some("overrides[0].excludeFiles")));
    let bad_options = entries(r#"[{"files": "*.ts", "options": 4}]"#);
    assert!(has_skip(&bad_options, ".prettierrc.json", Some("overrides[0].options")));
    let no_options = entries(r#"[{"files": "*.ts"}]"#);
    assert!(no_options.skipped.is_empty(), "an entry without options applies nothing");
    let empty = entries("[]");
    assert!(empty.skipped.is_empty(), "an empty overrides list applies no overrides");
    for invalid in ["null", "{}", "\"*.ts\""] {
        let resolved = entries(invalid);
        assert!(has_skip(&resolved, ".prettierrc.json", Some("overrides")), "{invalid}");
        assert_eq!(value(&resolved), (Spaces, 2));
    }
}

// ---------------------------------------------------------------------------
// Every wiring site: a control and one invalid value
// ---------------------------------------------------------------------------

macro_rules! wiring {
    ($name:ident, $file:literal, $ext:literal, $valid:expr => ($style:ident, $width:literal), $invalid:expr => $key:literal, default ($dstyle:ident, $dwidth:literal)) => {
        #[test]
        fn $name() {
            let control = Fixture::git().with($file, $valid).ext($ext);
            assert_eq!(value(&control), ($style, $width), "control: {control:?}");
            assert!(control.skipped.is_empty(), "control: {:?}", control.skipped);
            let invalid = Fixture::git().with($file, $invalid).ext($ext);
            assert_eq!(value(&invalid), ($dstyle, $dwidth), "invalid: {invalid:?}");
            assert!(has_skip(&invalid, $file, Some($key)), "{:?}", invalid.skipped);
        }
    };
}

wiring!(wiring_rustfmt_hard_tabs, "rustfmt.toml", "rs", "hard_tabs = true\n" => (Tabs, 4), "hard_tabs = 1\n" => "hard_tabs", default (Spaces, 4));
wiring!(wiring_rustfmt_tab_spaces, "rustfmt.toml", "rs", "tab_spaces = 3\n" => (Spaces, 3), "tab_spaces = \"3\"\n" => "tab_spaces", default (Spaces, 4));
wiring!(wiring_prettier_package_json, "package.json", "ts", r#"{"prettier": {"useTabs": true}}"# => (Tabs, 2), r#"{"prettier": {"useTabs": "yes"}}"# => "prettier.useTabs", default (Spaces, 2));
wiring!(wiring_prettier_package_yaml, "package.yaml", "ts", "prettier:\n  tabWidth: 6\n" => (Spaces, 6), "prettier:\n  tabWidth:\n" => "prettier.tabWidth", default (Spaces, 2));
wiring!(wiring_prettier_toml, ".prettierrc.toml", "ts", "tabWidth = 6\n" => (Spaces, 6), "tabWidth = 6.0\n" => "tabWidth", default (Spaces, 2));
wiring!(wiring_prettier_json5, ".prettierrc.json5", "ts", "{tabWidth: 6}" => (Spaces, 6), "{tabWidth: '6'}" => "tabWidth", default (Spaces, 2));
wiring!(wiring_prettier_override_use_tabs, ".prettierrc.yml", "ts", "overrides:\n  - files: '*.ts'\n    options:\n      useTabs: true\n" => (Tabs, 2), "overrides:\n  - files: '*.ts'\n    options:\n      useTabs: 'true'\n" => "overrides[0].options.useTabs", default (Spaces, 2));
wiring!(wiring_prettier_override_tab_width, ".prettierrc.yml", "ts", "overrides:\n  - files: '*.ts'\n    options:\n      tabWidth: 3\n" => (Spaces, 3), "overrides:\n  - files: '*.ts'\n    options:\n      tabWidth: 0\n" => "overrides[0].options.tabWidth", default (Spaces, 2));
wiring!(wiring_biome_indent_width, "biome.json", "ts", r#"{"formatter": {"indentWidth": 4}}"# => (Tabs, 4), r#"{"formatter": {"indentWidth": 0}}"# => "formatter.indentWidth", default (Tabs, 2));
wiring!(wiring_biome_enabled, "biome.json", "ts", r#"{"formatter": {"enabled": true}}"# => (Tabs, 2), r#"{"formatter": {"enabled": "no"}}"# => "formatter.enabled", default (Tabs, 2));
wiring!(wiring_biome_language_style, "biome.json", "css", r#"{"css": {"formatter": {"indentStyle": "space"}}}"# => (Spaces, 2), r#"{"css": {"formatter": {"indentStyle": "spaces"}}}"# => "css.formatter.indentStyle", default (Tabs, 2));
wiring!(wiring_biome_language_width, "biome.json", "ts", r#"{"javascript": {"formatter": {"indentWidth": 3}}}"# => (Tabs, 3), r#"{"javascript": {"formatter": {"indentWidth": null}}}"# => "javascript.formatter.indentWidth", default (Tabs, 2));
wiring!(wiring_biome_language_enabled, "biome.json", "json", r#"{"json": {"formatter": {"enabled": true}}}"# => (Tabs, 2), r#"{"json": {"formatter": {"enabled": 0}}}"# => "json.formatter.enabled", default (Tabs, 2));
wiring!(wiring_biome_language_block, "biome.json", "graphql", r#"{"graphql": {}}"# => (Tabs, 2), r#"{"graphql": 3}"# => "graphql", default (Tabs, 2));
wiring!(wiring_deno_use_tabs, "deno.json", "ts", r#"{"fmt": {"useTabs": true}}"# => (Tabs, 2), r#"{"fmt": {"useTabs": null}}"# => "fmt.useTabs", default (Spaces, 2));
wiring!(wiring_deno_indent_width, "deno.jsonc", "ts", r#"{"fmt": {"indentWidth": 4}}"# => (Spaces, 4), r#"{"fmt": {"indentWidth": 4.5}}"# => "fmt.indentWidth", default (Spaces, 2));
wiring!(wiring_deno_legacy_use_tabs, "deno.json", "ts", r#"{"fmt": {"options": {"useTabs": true}}}"# => (Tabs, 2), r#"{"fmt": {"options": {"useTabs": 1}}}"# => "fmt.options.useTabs", default (Spaces, 2));
wiring!(wiring_deno_legacy_indent_width, "deno.json", "ts", r#"{"fmt": {"options": {"indentWidth": 4}}}"# => (Spaces, 4), r#"{"fmt": {"options": {"indentWidth": "4"}}}"# => "fmt.options.indentWidth", default (Spaces, 2));
wiring!(wiring_deno_fmt_block, "deno.json", "ts", r#"{"fmt": {}}"# => (Spaces, 2), r#"{"fmt": true}"# => "fmt", default (Spaces, 2));
wiring!(wiring_dprint_use_tabs, "dprint.json", "ts", r#"{"useTabs": true}"# => (Tabs, 2), r#"{"useTabs": "true"}"# => "useTabs", default (Spaces, 2));
wiring!(wiring_dprint_indent_width, "dprint.json", "ts", r#"{"indentWidth": 4}"# => (Spaces, 4), r#"{"indentWidth": 256}"# => "indentWidth", default (Spaces, 2));
wiring!(wiring_dprint_plugin_use_tabs, "dprint.json", "ts", r#"{"typescript": {"useTabs": true}}"# => (Tabs, 2), r#"{"typescript": {"useTabs": null}}"# => "typescript.useTabs", default (Spaces, 2));
wiring!(wiring_dprint_plugin_indent_width, "dprint.json", "json", r#"{"json": {"indentWidth": 4}}"# => (Spaces, 4), r#"{"json": {"indentWidth": -4}}"# => "json.indentWidth", default (Spaces, 2));
wiring!(wiring_oxfmt_use_tabs, ".oxfmtrc.json", "ts", r#"{"useTabs": true}"# => (Tabs, 2), r#"{"useTabs": null}"# => "useTabs", default (Spaces, 2));
wiring!(wiring_oxfmt_tab_width, ".oxfmtrc.jsonc", "ts", r#"{"tabWidth": 4}"# => (Spaces, 4), r#"{"tabWidth": "4"}"# => "tabWidth", default (Spaces, 2));
wiring!(wiring_ruff_indent_width, "ruff.toml", "py", "indent-width = 2\n" => (Spaces, 2), "indent-width = \"2\"\n" => "indent-width", default (Spaces, 4));
wiring!(wiring_ruff_indent_style, ".ruff.toml", "py", "[format]\nindent-style = \"tab\"\n" => (Tabs, 4), "[format]\nindent-style = \"tabs\"\n" => "format.indent-style", default (Spaces, 4));
wiring!(wiring_ruff_pyproject, "pyproject.toml", "py", "[tool.ruff]\nindent-width = 2\n" => (Spaces, 2), "[tool.ruff]\nindent-width = 0\n" => "tool.ruff.indent-width", default (Spaces, 4));
wiring!(wiring_tombi_indent_style, "tombi.toml", "toml", "[format.rules]\nindent-style = \"tab\"\n" => (Tabs, 2), "[format.rules]\nindent-style = 1\n" => "format.rules.indent-style", default (Spaces, 2));
wiring!(wiring_tombi_indent_width, "tombi.toml", "toml", "[format.rules]\nindent-width = 4\n" => (Spaces, 4), "[format.rules]\nindent-width = 0\n" => "format.rules.indent-width", default (Spaces, 2));
wiring!(wiring_tombi_pyproject, "pyproject.toml", "toml", "[tool.tombi.format.rules]\nindent-width = 4\n" => (Spaces, 4), "[tool.tombi.format]\nrules = 3\n" => "tool.tombi.format.rules", default (Spaces, 2));
wiring!(wiring_taplo_indent_string, ".taplo.toml", "toml", "[formatting]\nindent_string = \"   \"\n" => (Spaces, 3), "[formatting]\nindent_string = 4\n" => "formatting.indent_string", default (Spaces, 2));
wiring!(wiring_yamlfmt_indent, ".yamlfmt.yaml", "yaml", "formatter:\n  indent: 4\n" => (Spaces, 4), "formatter:\n  indent: four\n" => "formatter.indent", default (Spaces, 2));
wiring!(wiring_stylua_indent_type, "stylua.toml", "lua", "indent_type = \"Spaces\"\n" => (Spaces, 4), "indent_type = \"spaces\"\n" => "indent_type", default (Tabs, 4));
wiring!(wiring_stylua_indent_width, "stylua.toml", "lua", "indent_width = 2\n" => (Tabs, 2), "indent_width = 2.5\n" => "indent_width", default (Tabs, 4));
wiring!(wiring_clang_format_indent_width, ".clang-format", "c", "IndentWidth: 4\n" => (Spaces, 4), "IndentWidth: '4'\n" => "IndentWidth", default (Spaces, 2));
wiring!(wiring_clang_format_use_tab, ".clang-format", "c", "UseTab: Always\n" => (Tabs, 2), "UseTab: true\n" => "UseTab", default (Spaces, 2));
wiring!(wiring_clang_format_based_on_style, ".clang-format", "c", "BasedOnStyle: WebKit\n" => (Spaces, 4), "BasedOnStyle: 4\n" => "BasedOnStyle", default (Spaces, 2));
wiring!(wiring_swift_format_spaces, ".swift-format", "swift", r#"{"indentation": {"spaces": 4}}"# => (Spaces, 4), r#"{"indentation": {"spaces": 0}}"# => "indentation.spaces", default (Spaces, 2));
wiring!(wiring_swift_format_tab_width, ".swift-format", "swift", r#"{"indentation": {"tabs": 1}, "tabWidth": 4}"# => (Tabs, 4), r#"{"indentation": {"spaces": 3}, "tabWidth": null}"# => "tabWidth", default (Spaces, 3));
wiring!(wiring_rubocop_style, ".rubocop.yml", "rb", "Layout/IndentationStyle:\n  EnforcedStyle: tabs\n" => (Tabs, 2), "Layout/IndentationStyle:\n  EnforcedStyle: tab\n" => "Layout/IndentationStyle.EnforcedStyle", default (Spaces, 2));
wiring!(wiring_rubocop_width, ".config/.rubocop.yml", "ru", "Layout/IndentationWidth:\n  Width: 4\n" => (Spaces, 4), "Layout/IndentationWidth:\n  Width:\n" => "Layout/IndentationWidth.Width", default (Spaces, 2));
wiring!(wiring_csharpier_use_tabs, ".csharpierrc", "cs", "useTabs: true\n" => (Tabs, 4), "useTabs: yes\n" => "useTabs", default (Spaces, 4));
wiring!(wiring_csharpier_indent_size, ".csharpierrc", "cs", "{\"indentSize\": 2}" => (Spaces, 2), "{\"indentSize\": \"2\"}" => "indentSize", default (Spaces, 4));

// ---------------------------------------------------------------------------
// Document shapes, once per loader
// ---------------------------------------------------------------------------

/// A row with a fallback candidate, so a rejected file visibly falls through.
fn biome_pair(first: &str, first_content: &str) -> ResolvedIndentation {
    Fixture::git().with(first, first_content).with(".biome.jsonc", r#"{"formatter": {"indentWidth": 8}}"#).ext("ts")
}

#[test]
fn strict_json_documents_reject_duplicate_keys_comments_and_trailing_content() {
    let control = biome_pair("biome.json", r#"{"formatter": {"indentWidth": 4}}"#);
    assert_eq!(control.indent.get(), 4);
    for invalid in [
        "{\"formatter\": {\"indentWidth\": 4, \"indentWidth\": 6}}",
        "{\"formatter\": {\"indentWidth\": 4}} trailing",
        "// comment\n{\"formatter\": {\"indentWidth\": 4}}",
        "{\"formatter\": {\"indentWidth\": 4},}",
    ] {
        let resolved = biome_pair("biome.json", invalid);
        assert_eq!(resolved.indent.get(), 8, "{invalid}: falls through to the next candidate");
        assert!(has_skip(&resolved, "biome.json", None), "{invalid}");
    }
    let duplicate = biome_pair("biome.json", "{\"formatter\": {\"indentWidth\": 4,\n \"indentWidth\": 6}}");
    assert_eq!(reason(&duplicate, "biome.json"), "repeats the key `indentWidth` at line 2 column 2; skipped");
    assert!(reason(&biome_pair("biome.json", "{} x"), "biome.json").contains("line 1 column 4"));
}

#[test]
fn json5_documents_allow_comments_and_trailing_commas_and_reject_duplicates_and_garbage() {
    let fixture = |content: &str| Fixture::git().with("biome.jsonc", content).with(".biome.json", r#"{"formatter": {"indentWidth": 8}}"#).ext("ts");
    let control = fixture("// comment\n{formatter: {indentWidth: 4, /* x */},}");
    assert_eq!(control.indent.get(), 4);
    assert!(has_skip(&control, ".biome.json", None), "shadowed");
    for invalid in ["{formatter: {indentWidth: 4, indentWidth: 6}}", "{formatter: {}} }"] {
        let resolved = fixture(invalid);
        assert_eq!(resolved.indent.get(), 8, "{invalid}");
        assert!(has_skip(&resolved, "biome.jsonc", None), "{invalid}");
    }
}

#[test]
fn yaml_documents_read_implicit_null_and_reject_duplicates_second_documents_and_garbage() {
    let fixture = |content: &str| Fixture::git().with(".prettierrc.yaml", content).with(".prettierrc.yml", "tabWidth: 8\n").ext("ts");
    let control = fixture("tabWidth: 4\n");
    assert_eq!(control.indent.get(), 4);
    let implicit_null = fixture("tabWidth:\n");
    assert_eq!(implicit_null.indent.get(), 2, "`tabWidth:` is null, which is invalid, not absent");
    assert!(reason(&implicit_null, ".prettierrc.yaml").contains("found null"));
    for invalid in ["tabWidth: 4\ntabWidth: 6\n", "tabWidth: 4\n---\ntabWidth: 6\n", "tabWidth: [4\n"] {
        let resolved = fixture(invalid);
        assert_eq!(resolved.indent.get(), 8, "{invalid}");
        assert!(has_skip(&resolved, ".prettierrc.yaml", None), "{invalid}");
    }
    let empty = fixture("# no settings\n");
    assert_eq!(empty.authority.path.as_deref(), Some(Path::new(".prettierrc.yaml")), "an empty file declares the row");
    // Native YAML types survive: a quoted number is a string.
    let quoted = fixture("tabWidth: '4'\n");
    assert!(has_skip(&quoted, ".prettierrc.yaml", Some("tabWidth")));
}

#[test]
fn toml_documents_reject_duplicate_keys_and_trailing_content() {
    let fixture = |content: &str| Fixture::git().with(".rustfmt.toml", content).with("rustfmt.toml", "tab_spaces = 8\n").ext("rs");
    assert_eq!(fixture("tab_spaces = 2\n").indent.get(), 2);
    for invalid in ["tab_spaces = 2\ntab_spaces = 3\n", "tab_spaces = 2 trailing\n"] {
        let resolved = fixture(invalid);
        assert_eq!(resolved.indent.get(), 8, "{invalid}");
        assert!(reason(&resolved, ".rustfmt.toml").contains("not valid TOML"), "{invalid}");
    }
}

#[test]
fn a_yaml_stream_reads_every_document_and_rejects_a_bad_one() {
    let fixture = |content: &str| Fixture::git().with(".clang-format", content).with("_clang-format", "IndentWidth: 8\n").ext("cpp");
    assert_eq!(fixture("---\nIndentWidth: 3\n---\nLanguage: Cpp\nUseTab: Always\n").indent_using, Tabs);
    for invalid in ["IndentWidth: 3\nIndentWidth: 4\n", "---\nIndentWidth: 3\n---\n[\n"] {
        let resolved = fixture(invalid);
        assert_eq!(resolved.indent.get(), 8, "{invalid}");
        assert!(has_skip(&resolved, ".clang-format", None), "{invalid}");
    }
    let scalars = fixture("--- 3\n--- text\n");
    assert_eq!(scalars.indent.get(), 8, "a stream with no mapping document is skipped");
}

#[test]
fn an_extensionless_config_is_strict_json_when_it_opens_a_collection_and_yaml_otherwise() {
    assert_eq!(Fixture::git().with(".prettierrc", "  {\"tabWidth\": 6}").ext("ts").indent.get(), 6);
    assert_eq!(Fixture::git().with(".prettierrc", "tabWidth: 6\n").ext("ts").indent.get(), 6);
    let json5 = Fixture::git().with(".prettierrc", "{tabWidth: 6}").ext("ts");
    assert!(reason(&json5, ".prettierrc").contains("not valid JSON"));
    assert_eq!(Fixture::git().with(".csharpierrc", "indentSize: 2\n").ext("cs").indent.get(), 2);
}

#[test]
fn an_unreadable_candidate_is_diagnosed_and_skipped() {
    let fixture = Fixture::git().with(".rustfmt.toml/inner", "").with("rustfmt.toml", "tab_spaces = 2\n");
    let rs = fixture.ext("rs");
    assert_eq!(rs.indent.get(), 2);
    assert!(reason(&rs, ".rustfmt.toml").contains("directory"));
    let editorconfig = Fixture::git().with(".editorconfig/inner", "").ext("rs");
    assert!(has_skip(&editorconfig, ".editorconfig", None));
    assert_eq!(editorconfig.authority.via, ["rustfmt"]);
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

#[test]
fn the_summary_lists_only_extensions_that_differ_from_the_generic_default() {
    let fixture = Fixture::git()
        .with(".editorconfig", "[*]\nindent_size = 2\n[*.{yaml,yml}]\nindent_size = 4\n[*.custom]\nindent_style = tab\n[*.t?ml]\nindent_size = 9\n")
        .with("biome.json", "{\"formatter\": {\"indentWidth\": 2, \"indentWidth\": 3}}");
    let summary = resolve_summary(fixture.root()).unwrap();
    assert_eq!(value(&summary.default), (Spaces, 2));
    assert_eq!(summary.default.authority.via, ["*"]);
    let keys: Vec<&str> = summary.overrides.keys().map(String::as_str).collect();
    assert!(keys.contains(&"yaml") && keys.contains(&"yml") && keys.contains(&"custom") && keys.contains(&"go"), "{keys:?}");
    assert!(!keys.contains(&"rs"), "same as the default: {keys:?}");
    assert!(!keys.contains(&"ts"), "biome.json is skipped, so ts falls back to [*]: {keys:?}");
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);
    // The duplicate-key diagnostic surfaces once, though only extensions
    // equal to the default read it.
    let biome: Vec<&IndentationDiagnostic> = summary.skipped.iter().filter(|diagnostic| portable(&diagnostic.path) == "biome.json").collect();
    assert_eq!(biome.len(), 1, "{:?}", summary.skipped);

    let snapshot = RepoIndentation::load(fixture.root()).unwrap();
    assert_eq!(snapshot.summary(), summary);
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json.as_object().unwrap().keys().collect::<Vec<_>>(), ["default", "overrides", "skipped"]);
}

#[test]
fn a_summary_outside_a_worktree_is_the_fallback_with_no_overrides() {
    let fixture = Fixture::plain().with(".editorconfig", "[*]\nindent_size = 2\n");
    let summary = resolve_summary(fixture.root()).unwrap();
    assert_eq!(value(&summary.default), (Spaces, 4));
    assert_eq!(summary.default.authority.source, IndentAuthority::Default);
    assert!(summary.overrides.is_empty());
    assert!(summary.skipped.is_empty());
}

// ---------------------------------------------------------------------------
// Repository discovery and paths
// ---------------------------------------------------------------------------

#[test]
fn a_path_outside_a_worktree_reads_no_configuration_but_keeps_its_convention() {
    let fixture = Fixture::plain().with(".editorconfig", "[*]\nindent_size = 2\n").with("rustfmt.toml", "tab_spaces = 2\n");
    let go = fixture.path("cmd/main.go");
    assert_eq!(go.authority.via, ["gofmt"]);
    let rs = fixture.path("src/lib.rs");
    assert_eq!(value(&rs), (Spaces, 4));
    assert_eq!(rs.authority.via, ["rustfmt"]);
    assert_eq!(fixture.path("a.yaml").authority.source, IndentAuthority::Default);
    assert_eq!(fixture.ext("go").authority.via, ["gofmt"]);
}

#[test]
fn a_new_path_resolves_from_its_nearest_existing_ancestor_against_the_full_destination() {
    let fixture = Fixture::git().with(".editorconfig", "[generated/**.yaml]\nindent_size = 2\n");
    let resolved = fixture.path("generated/deep/new/config.yaml");
    assert!(!fixture.root().join("generated").exists());
    assert_eq!(value(&resolved), (Spaces, 2));
    assert_eq!(fixture.path("generated/./x/../y.yaml").indent.get(), 2);
}

#[test]
fn a_linked_worktree_reads_its_own_root() {
    let fixture = Fixture::git();
    let main = fixture.root().join("main");
    std::fs::create_dir(&main).unwrap();
    let repo = git2::Repository::init(&main).unwrap();
    write(&main, ".editorconfig", "[*]\nindent_size = 2\n");
    let signature = git2::Signature::now("Test", "test@example.com").unwrap();
    let tree = repo.find_tree(repo.index().unwrap().write_tree().unwrap()).unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[]).unwrap();
    let linked = fixture.root().join("linked");
    repo.worktree("linked", &linked, None).unwrap();
    write(&linked, ".editorconfig", "[*]\nindent_size = 3\n");

    assert_eq!(resolve_for_path(&main.join("a.txt")).unwrap().indent.get(), 2);
    assert_eq!(resolve_for_path(&linked.join("sub/a.txt")).unwrap().indent.get(), 3);
    assert_eq!(resolve_for_extension(&linked.join("missing"), "txt").unwrap().indent.get(), 3);
}

#[test]
fn a_nested_repository_is_discovered_by_the_free_functions_but_not_by_a_snapshot() {
    let fixture = Fixture::git().with(".editorconfig", "[*]\nindent_size = 2\n");
    let nested = fixture.root().join("vendor/lib");
    std::fs::create_dir_all(&nested).unwrap();
    git2::Repository::init(&nested).unwrap();
    write(&nested, ".editorconfig", "[*]\nindent_size = 6\n");

    assert_eq!(resolve_for_path(&nested.join("a.txt")).unwrap().indent.get(), 6);
    let outer = RepoIndentation::load(fixture.root()).unwrap();
    assert_eq!(outer.resolve_for_path(&nested.join("a.txt")).unwrap().indent.get(), 2);
    let inner = RepoIndentation::load(&nested).unwrap();
    assert_eq!(inner.resolve_for_path(&nested.join("a.txt")).unwrap().indent.get(), 6);
}

#[test]
fn a_snapshot_refuses_relative_and_outside_paths() {
    let fixture = Fixture::git();
    let snapshot = RepoIndentation::load(fixture.root()).unwrap();
    assert!(snapshot.resolve_for_path(Path::new("a.txt")).is_err());
    let other = TempDir::new().unwrap();
    let error = snapshot.resolve_for_path(&other.path().join("a.txt")).unwrap_err();
    assert!(error.to_string().contains("outside the worktree"), "{error}");
    assert!(RepoIndentation::load(&fixture.root().join("missing")).is_err());
}

#[test]
fn a_path_that_names_no_file_or_crosses_a_file_is_an_error() {
    let fixture = Fixture::git().with("file.txt", "x");
    assert!(resolve_for_path(&fixture.root().join("file.txt/child.yaml")).is_err());
    assert!(resolve_for_path(Path::new("/")).is_err());
}

#[test]
fn results_are_deterministic_across_repeated_lookups() {
    let fixture = Fixture::git()
        .with(".editorconfig", REPOSITORY_EDITORCONFIG)
        .with("package.json", r#"{"prettier": {"tabWidth": 3}}"#)
        .with("biome.json", "{}")
        .with("deno.json", "{}")
        .with("dprint.json", "{}")
        .with(".oxfmtrc.json", "{}")
        .with("prettier.config.js", "");
    let first = fixture.ext("ts");
    for _ in 0..5 {
        assert_eq!(fixture.ext("ts"), first);
    }
    let snapshot = RepoIndentation::load(fixture.root()).unwrap();
    assert_eq!(snapshot.resolve_for_extension("ts"), first);
    assert_eq!(snapshot.resolve_for_path(&snapshot.root().join("a.ts")).unwrap(), fixture.path("a.ts"));
}

#[test]
fn opinion_and_diagnostic_paths_serialize_with_forward_slashes() {
    let fixture = Fixture::git().with(".config/tombi.toml", "x = = 1\n").with(".config/rubocop/config.yml", "{}\n");
    let toml = fixture.ext("toml");
    let json = serde_json::to_value(&toml).unwrap();
    assert_eq!(json["skipped"][0]["path"], ".config/tombi.toml");
    let rb = serde_json::to_value(fixture.ext("rb")).unwrap();
    assert_eq!(rb["authority"]["path"], ".config/rubocop/config.yml");
}

#[test]
fn extensions_follow_the_documented_grammar() {
    for valid in ["yaml", ".yaml", "c++", "d-ts", "x_1", "7z", "YAML"] {
        assert!(normalize_extension(valid).is_ok(), "{valid}");
    }
    assert_eq!(normalize_extension(".yaml").unwrap(), "yaml");
    for invalid in ["", ".", "..yaml", "-x", "_x", "a b", "a.b", "ä", "a/b"] {
        assert!(normalize_extension(invalid).is_err(), "{invalid}");
        let fixture = Fixture::git();
        assert!(resolve_for_extension(fixture.root(), invalid).is_err(), "{invalid}");
    }
}
