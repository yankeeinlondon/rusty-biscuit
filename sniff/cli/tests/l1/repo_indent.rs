//! `sniff repo indent` and the `indent` key of `sniff repo --json`, spawned
//! against fixture worktrees.
//!
//! These prove the command's wiring: argument validation, base-directory
//! discovery, stdout-only JSON in the library's shape, plain rendering, and
//! one positive and one malformed configuration per document loader. The
//! full field and document robustness matrix is tested once, in-process, by
//! `sniff/lib/tests/l1/indentation.rs`.

use std::path::PathBuf;

use serde_json::Value;

use crate::common::SniffCliFixture;

/// A fixture whose `cwd` holds a Git worktree at `repo/`, so the launch
/// directory itself is outside any worktree.
struct IndentFixture {
    fixture: SniffCliFixture,
}

impl IndentFixture {
    fn new() -> Self {
        let fixture = SniffCliFixture::named("sniff-repo-indent");
        std::fs::create_dir_all(fixture.cwd().join("repo")).unwrap();
        git2::Repository::init(fixture.cwd().join("repo")).unwrap();
        Self { fixture }
    }

    fn repo(&self) -> PathBuf {
        self.fixture.cwd().join("repo")
    }

    fn with(self, relative: &str, content: &str) -> Self {
        let path = self.repo().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
        self
    }

    /// Runs `sniff --base repo <args>` from the fixture's launch directory.
    fn run(&self, args: &[&str]) -> std::process::Output {
        let mut command = self.fixture.command();
        command.arg("--base").arg(self.repo()).args(args);
        command.output().unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty(), "JSON mode writes only stdout: {}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice(&output.stdout).expect("stdout is one JSON document")
    }

    fn plain(&self, args: &[&str]) -> String {
        let output = self.run(&[&["--plain"], args].concat());
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    }
}

fn skipped_paths(value: &Value) -> Vec<&str> {
    value["skipped"].as_array().unwrap().iter().map(|entry| entry["path"].as_str().unwrap()).collect()
}

// ---------------------------------------------------------------------------
// Specification examples
// ---------------------------------------------------------------------------

#[test]
fn this_repositorys_editorconfig_gives_yaml_four_spaces_from_editorconfig() {
    let fixture = IndentFixture::new().with(".editorconfig", include_str!("../../../../.editorconfig"));
    let text = fixture.plain(&["repo", "indent", "yaml"]);
    assert!(text.starts_with("4 spaces  (.editorconfig [*], [*.{"), "{text}");
    assert!(text.ends_with("consensus: yes\n"), "{text}");
}

#[test]
fn a_rustfmt_tab_spaces_of_two_reports_rust_from_rustfmt_and_yaml_from_the_default() {
    let fixture = IndentFixture::new().with("rustfmt.toml", "tab_spaces = 2\n");
    let rust = fixture.json(&["repo", "indent", "rs", "--json"]);
    assert_eq!((rust["indent_using"].as_str(), rust["indent"].as_u64()), (Some("space"), Some(2)));
    assert_eq!(rust["authority"]["source"], "rustfmt");
    assert_eq!(rust["authority"]["path"], "rustfmt.toml");
    assert_eq!(rust["authority"]["via"], serde_json::json!(["tab_spaces"]));

    let yaml = fixture.plain(&["repo", "indent", ".yaml"]);
    assert_eq!(yaml, "4 spaces  (default)\nconsensus: yes\n");
}

#[test]
fn a_bare_biome_json_gives_tabs_until_the_editorconfig_outranks_it_without_consensus() {
    let fixture = IndentFixture::new().with("biome.json", "{}");
    let alone = fixture.json(&["repo", "indent", "ts", "--json"]);
    assert_eq!(alone["indent_using"], "tab");
    assert_eq!(alone["authority"]["source"], "biome");
    assert_eq!(alone["authority"]["via"], serde_json::json!([]));

    let fixture = fixture.with(".editorconfig", include_str!("../../../../.editorconfig"));
    let ranked = fixture.json(&["repo", "indent", "ts", "--json"]);
    assert_eq!((ranked["indent_using"].as_str(), ranked["indent"].as_u64()), (Some("space"), Some(4)));
    assert_eq!(ranked["authority"]["source"], "editorconfig");
    assert_eq!(ranked["alt_authorities"][0]["source"], "biome");
    assert_eq!(ranked["alt_authorities"][0]["indent_using"], "tab");
    assert_eq!(ranked["consensus"], false);

    let text = fixture.plain(&["repo", "indent", "ts"]);
    assert!(text.contains("\nalso: biome.json defaults  tabs (width 2)\n"), "{text}");
    assert!(text.ends_with("consensus: no\n"), "{text}");
}

// ---------------------------------------------------------------------------
// Partial opinions and the YAML adjustment
// ---------------------------------------------------------------------------

#[test]
fn a_style_only_section_names_a_separate_width_source() {
    let fixture = IndentFixture::new()
        .with(".editorconfig", "root = true\n\n[*]\nindent_style = space\n")
        .with("rustfmt.toml", "tab_spaces = 2\n");
    let value = fixture.json(&["repo", "indent", "rs", "--json"]);
    assert_eq!(value["indent"], 2);
    assert_eq!(value["authority"]["indent_using"], "space");
    assert!(value["authority"].get("indent").is_none(), "an unstated value is omitted: {value}");

    let text = fixture.plain(&["repo", "indent", "rs"]);
    assert!(text.starts_with("2 spaces  (style: .editorconfig [*]; width: rustfmt.toml tab_spaces)\n"), "{text}");
}

#[test]
fn yaml_reports_the_tab_adjustment_in_json_and_plain_output() {
    let fixture = IndentFixture::new().with(".editorconfig", "[*.yaml]\nindent_style = tab\nindent_size = 2\n");
    let value = fixture.json(&["repo", "indent", "yaml", "--json"]);
    assert_eq!((value["indent_using"].as_str(), value["indent"].as_u64()), (Some("space"), Some(2)));
    assert_eq!(value["adjustment"], serde_json::json!({ "requested": "tab", "reason": "yaml_forbids_tabs" }));
    assert_eq!(value["authority"]["indent_using"], "tab", "the opinion keeps what it asked for");

    let text = fixture.plain(&["repo", "indent", "yaml"]);
    assert!(text.contains("\nnote: tabs requested; using spaces because YAML does not allow tabs for indentation\n"), "{text}");
}

// ---------------------------------------------------------------------------
// Summary mode and the parent aggregate
// ---------------------------------------------------------------------------

#[test]
fn the_summary_lists_the_default_and_each_differing_extension() {
    let fixture = IndentFixture::new()
        .with(".editorconfig", "[*]\nindent_style = space\nindent_size = 4\n\n[*.{yaml,yml}]\nindent_size = 2\n");
    let value = fixture.json(&["repo", "indent", "--json"]);
    assert_eq!(value["default"]["indent"], 4);
    assert_eq!(value["default"]["authority"]["via"], serde_json::json!(["*"]));
    let overrides = value["overrides"].as_object().unwrap();
    assert_eq!(overrides["yaml"]["indent"], 2);
    assert_eq!(overrides["yml"]["indent"], 2);
    assert!(!overrides.contains_key("rs"), "a type matching the default is omitted: {value}");
    assert_eq!(value["skipped"], serde_json::json!([]));

    let text = fixture.plain(&["repo", "indent"]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "default  4 spaces  (.editorconfig [*])", "{text}");
    assert!(lines.contains(&"yaml     2 spaces  (.editorconfig [*], [*.{yaml,yml}])"), "{text}");
}

#[test]
fn the_parent_repo_json_carries_the_summary_under_indent() {
    let fixture = IndentFixture::new().with(".editorconfig", "[*]\nindent_style = tab\n");
    let leaf = fixture.json(&["repo", "indent", "--json"]);
    let parent = fixture.json(&["repo", "--json"]);
    assert_eq!(parent["indent"], leaf, "the parent embeds exactly the leaf's payload");
    assert_eq!(parent["indent"]["default"]["indent_using"], "tab");
}

#[test]
fn focused_repo_commands_keep_their_json_without_indent() {
    let fixture = IndentFixture::new();
    let name = fixture.json(&["repo", "name", "--json"]);
    assert!(name.get("indent").is_none(), "{name}");
    assert_eq!(name.as_object().unwrap().len(), 1, "{name}");
}

// ---------------------------------------------------------------------------
// Arguments and discovery
// ---------------------------------------------------------------------------

macro_rules! invalid_extension {
    ($($name:ident: $input:expr,)*) => {$(
        #[test]
        fn $name() {
            let fixture = IndentFixture::new();
            // After `--`, so a leading hyphen reaches the extension grammar.
            let output = fixture.run(&["repo", "indent", "--json", "--", $input]);
            assert_eq!(output.status.code(), Some(2), "usage error for {:?}", $input);
            assert!(output.stdout.is_empty(), "{}", String::from_utf8_lossy(&output.stdout));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("invalid file extension"), "{stderr}");
        }
    )*};
}

invalid_extension! {
    an_empty_extension_is_a_usage_error: "",
    a_lone_dot_is_a_usage_error: ".",
    two_leading_dots_are_a_usage_error: "..yaml",
    a_leading_hyphen_is_a_usage_error: "-yaml",
    a_path_separator_is_a_usage_error: "y/aml",
    a_wildcard_is_a_usage_error: "*.yaml",
}

#[test]
fn the_launch_directory_inside_a_worktree_is_used_without_base() {
    let fixture = IndentFixture::new().with("rustfmt.toml", "tab_spaces = 2\n");
    std::fs::create_dir_all(fixture.repo().join("src/deep")).unwrap();
    let output = fixture
        .fixture
        .command_builder()
        .ambient_context(&fixture.repo().join("src/deep"))
        .build()
        .args(["repo", "indent", "rs", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["indent"], 2, "the worktree is found from a subdirectory");
}

#[test]
fn outside_a_worktree_a_convention_or_the_default_applies() {
    let fixture = SniffCliFixture::named("sniff-repo-indent-outside");
    let plain_dir = fixture.cwd();
    // A formatter file outside any worktree is never read.
    std::fs::write(plain_dir.join("rustfmt.toml"), "tab_spaces = 2\n").unwrap();
    let run = |args: &[&str]| {
        let output = fixture.command().arg("--base").arg(plain_dir).args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let go = run(&["repo", "indent", "go", "--json"]);
    assert_eq!(go["indent_using"], "tab");
    assert_eq!(go["authority"]["source"], "convention");
    assert_eq!(go["authority"]["via"], serde_json::json!(["gofmt"]));
    let rust = run(&["repo", "indent", "rs", "--json"]);
    assert_eq!(rust["indent"], 4, "rustfmt.toml outside a worktree is ignored: {rust}");
    let summary = run(&["repo", "indent", "--json"]);
    assert_eq!(summary["default"]["authority"]["source"], "default");
    assert_eq!(summary["overrides"], serde_json::json!({}));
}

#[test]
fn help_and_completion_name_the_command() {
    let fixture = IndentFixture::new();
    let help = fixture.run(&["repo", "indent", "--help"]);
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("Usage: sniff repo indent [OPTIONS] [FILE-EXT]"), "{help}");

    let parent_help = fixture.run(&["repo", "--help"]);
    assert!(String::from_utf8_lossy(&parent_help.stdout).contains("sniff repo indent yaml"));

    let completion = fixture
        .fixture
        .command()
        .env("COMPLETE", "fish")
        .args(["--", "sniff", "repo", "ind"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&completion.stdout).starts_with("indent\t"), "{completion:?}");
}

// ---------------------------------------------------------------------------
// One positive and one malformed configuration per document loader
// ---------------------------------------------------------------------------

/// Each loader's control gives its stated width; its malformed copy is
/// skipped with a located diagnostic while the command still succeeds with
/// the lower-ranked result.
macro_rules! loader_wiring {
    ($($name:ident: $file:expr, $ext:expr, $valid:expr => $width:expr, $malformed:expr => $fallback:expr, $located:expr;)*) => {$(
        #[test]
        fn $name() {
            let control = IndentFixture::new().with($file, $valid);
            let value = control.json(&["repo", "indent", $ext, "--json"]);
            assert_eq!(value["indent"], $width, "control: {value}");
            assert!(!skipped_paths(&value).contains(&$file), "control: {value}");

            let broken = IndentFixture::new().with($file, $malformed);
            let value = broken.json(&["repo", "indent", $ext, "--json"]);
            assert_eq!(value["indent"], $fallback, "malformed: {value}");
            let entry = value["skipped"].as_array().unwrap().iter()
                .find(|entry| entry["path"] == $file)
                .unwrap_or_else(|| panic!("no diagnostic for {}: {value}", $file));
            assert!(entry["reason"].as_str().unwrap().contains($located), "{entry}");

            let text = broken.plain(&["repo", "indent", $ext]);
            assert!(text.contains("skipped configuration:"), "{text}");
            assert!(text.contains(&format!("- {}", $file)), "{text}");
        }
    )*};
}

loader_wiring! {
    strict_json_loader_is_wired: "biome.json", "ts",
        r#"{"formatter": {"indentStyle": "space", "indentWidth": 4}}"# => 4,
        "{\"formatter\": {\"indentWidth\": 4,\n \"indentWidth\": 6}}" => 4, "line 2 column 2";
    json5_loader_is_wired: "biome.jsonc", "ts",
        "// comment\n{formatter: {indentStyle: 'space', indentWidth: 6,},}" => 6,
        "{formatter: {}} }" => 4, "line 1";
    yaml_loader_is_wired: ".prettierrc.yaml", "ts",
        "tabWidth: 8\n" => 8,
        "tabWidth: [4\n" => 4, "line";
    toml_loader_is_wired: "rustfmt.toml", "rs",
        "tab_spaces = 2\n" => 2,
        "tab_spaces = 2 trailing\n" => 4, "not valid TOML";
    editorconfig_scanner_is_wired: ".editorconfig", "rs",
        "[*]\nindent_size = 3\n" => 3,
        "[*]\nindent_size = 0\n" => 4, "line 2";
}
