//! Integration tests for `md schema about`.

use assert_cmd::cargo::cargo_bin_cmd;
use darkmatter::testing::strip_ansi_codes;
use predicates::prelude::*;

fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

#[test]
fn schema_about_prints_simplified_schema_reference() {
    let output = md_cmd().args(["schema", "about"]).output().expect("run md schema about");
    assert!(output.status.success(), "schema about should succeed");
    // Strip ANSI: the `--verbose` hint styles the flag token inline, so the
    // phrase is only contiguous after color codes are removed.
    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    for needle in [
        "SimplifiedSchema",
        "Schema Shapes",
        "Type System",
        "Constraint Vocabulary",
        "Trigger Schemas",
        "kind: trigger-schema",
        "Vacuous-arm lint",
        "Match-safe constraints only",
        "use --verbose to see additional details",
    ] {
        assert!(stdout.contains(needle), "schema about missing `{needle}`");
    }
    for absent in ["Nested Objects", "Compose-time Coercion", "Validation Notes"] {
        assert!(
            !stdout.contains(absent),
            "schema about should not show `{absent}` without --verbose"
        );
    }
}

#[test]
fn schema_about_lists_every_supported_type_keyword() {
    let mut cmd = md_cmd();
    let output = cmd.args(["schema", "about"]).output().expect("run md schema about");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    for keyword in [
        "string",
        "date",
        "datetime",
        "time",
        "number",
        "numberlike",
        "boolean",
        "boolish",
        "object",
        "file",
        "enum",
        "url",
        "email",
        "any",
    ] {
        assert!(
            stdout.contains(keyword),
            "schema about report missing type keyword `{keyword}`"
        );
    }
}

#[test]
fn schema_about_mentions_inline_object_rules() {
    md_cmd()
        .args(["--verbose", "schema", "about"])
        .assert()
        .success()
        .stdout(predicate::str::contains("additionalProperties: false"))
        .stdout(predicate::str::contains("32"))
        .stdout(predicate::str::contains("Object shape"));
}

#[test]
fn schema_about_exits_zero() {
    md_cmd().args(["schema", "about"]).assert().success();
}

#[test]
fn schema_about_is_documentation_only() {
    // Running `md schema about` must not require or read any markdown file.
    // We assert that:
    //   1. Running from an empty / non-existent working directory produces
    //      the same key sections as running from the project root.
    //   2. Both invocations exit with status `0`.
    // (Strict byte-for-byte equality is not portable: terminal capability
    // detection in the prose renderer can change the exact escape sequences
    // emitted between sessions, but the textual content is stable.)
    let tmp = tempfile::TempDir::new().unwrap();
    let output_a = md_cmd()
        .args(["schema", "about"])
        .current_dir(tmp.path())
        .output()
        .expect("run md schema about from temp dir");
    let output_b = md_cmd()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about from project root");

    assert!(output_a.status.success(), "first invocation should succeed");
    assert!(output_b.status.success(), "second invocation should succeed");

    // Strip ANSI: the `--verbose` hint styles the flag token inline.
    let a = strip_ansi_codes(&String::from_utf8_lossy(&output_a.stdout));
    let b = strip_ansi_codes(&String::from_utf8_lossy(&output_b.stdout));
    for needle in [
        "SimplifiedSchema",
        "Type System",
        "Constraint Vocabulary",
        "use --verbose to see additional details",
    ] {
        assert!(
            a.contains(needle) && b.contains(needle),
            "schema about is missing `{needle}` from one of the two runs"
        );
    }
}

#[test]
fn schema_about_uses_readable_terminal_components() {
    let output = md_cmd()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success());

    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    let type_system = plain
        .split("Type System")
        .nth(1)
        .and_then(|after_heading| after_heading.split("Constraint Vocabulary").next())
        .expect("type system section should precede constraint vocabulary");

    assert!(plain.contains("┌"), "type/constraint sections should render real tables");
    assert!(
        !plain.contains("file://"),
        "schema examples must not be interpreted as Markdown links"
    );
    assert!(
        !plain.contains("```"),
        "code examples should render as code blocks instead of literal Markdown fences"
    );
    assert!(
        !plain.contains('`'),
        "inline code spans should render as styled text instead of raw backtick-delimited Markdown"
    );
    assert!(
        !plain.contains("Table could not be rendered"),
        "schema about tables should fit the report width"
    );
    assert!(
        !plain.contains("all types (and array properties"),
        "universal constraint targets should not special-case arrays awkwardly"
    );
    assert!(
        !plain.contains("array-level"),
        "array constraint rows should use plain `array` language"
    );
    assert!(
        !plain.contains("string context"),
        "constraint examples should not carry implementation-context labels"
    );
    assert!(
        !plain.contains("min (string)")
            && !plain.contains("min (number)")
            && !plain.contains("min (array)")
            && !plain.contains("max (string)")
            && !plain.contains("max (number)")
            && !plain.contains("max (array)"),
        "min/max should be documented once each instead of repeated per type"
    );
    assert!(
        plain.contains("min(number)")
            && plain.contains("max(number)")
            && plain.contains("minimum length")
            && plain.contains("minimum value")
            && plain.contains("minimum item count")
            && plain.contains("maximum length")
            && plain.contains("maximum value")
            && plain.contains("maximum item count"),
        "consolidated min/max rows should explain each applicable type in user-facing terms"
    );
    assert!(
        !plain.contains("│ Arity ") && !plain.contains("│ Form ") && !plain.contains("│ Target "),
        "constraint table headings should be author-facing"
    );
    assert!(
        plain.contains("│ Write ") && plain.contains("│ Applies To ") && plain.contains("│ Meaning "),
        "constraint table should name usage and applicability clearly"
    );
    assert!(
        !plain.contains("default(\n") && !plain.contains("pattern(\n"),
        "constraint usage examples should not wrap immediately after the opening parenthesis"
    );
    assert!(
        !plain.contains("This report is generated from the typed descriptor catalog")
            && !plain.contains("documentation-only: it performs no document parsing"),
        "awkward implementation footer should not be user-visible"
    );
    assert!(
        !plain.contains("\n\n\n"),
        "schema about should not emit more than one blank line between blocks"
    );
    assert!(
        plain.contains("page is rendered.\n\n Use this"),
        "paragraph blocks should be separated by a blank line"
    );
    assert!(
        plain.contains("frontmatter you expect.\n\n - Single object"),
        "unordered lists should have a blank line before them"
    );
    assert!(
        plain.contains("Type System\n\n ┌"),
        "tables should have a blank line before them"
    );
    assert!(
        type_system.contains("│ Type ") && !type_system.contains("│ Keyword "),
        "type table should use the author-facing `Type` column heading"
    );
    assert!(
        !type_system.contains("match(glob") && !type_system.contains("scheme(http"),
        "type table should list constraint names without constraint parameters"
    );
    assert!(
        plain.contains("┘\n\n Any of the above types can be made into an array"),
        "type vocabulary notes should start after the table with a blank line"
    );
    assert!(
        plain.contains("bar: string[]") && plain.contains("baz: \"string[](min(1))\""),
        "type vocabulary notes should include array and constrained-array examples"
    );
    assert!(
        plain.contains("Note: the values in schemas must be valid YAML strings"),
        "type notes should explain that schema values are YAML strings and may be quoted"
    );
    assert!(
        plain.contains("use --verbose to see additional details"),
        "non-verbose output should tell readers how to see additional details"
    );
    assert!(
        !plain.contains("Nested Objects")
            && !plain.contains("Compose-time Coercion")
            && !plain.contains("Validation Notes"),
        "advanced schema details should only render with --verbose"
    );
    assert!(
        plain.lines().filter(|line| !line.is_empty()).all(|line| line.starts_with(' ')),
        "nonblank report lines should use the document left margin"
    );
}

#[test]
fn schema_about_verbose_prints_advanced_sections_as_readable_lists() {
    let output = md_cmd()
        .args(["--verbose", "schema", "about"])
        .output()
        .expect("run verbose md schema about");
    assert!(output.status.success());

    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));

    assert!(plain.contains("Nested Objects"));
    assert!(plain.contains("Compose-time Coercion"));
    assert!(plain.contains("Validation Notes"));
    assert!(plain.contains("- Object shape. Write fields inside braces"));
    assert!(plain.contains("  - Details: { title: string, rating: number }"));
    assert!(plain.contains("- Booleans. When the schema expects boolean or boolish"));
    assert!(plain.contains("  - Behavior: true and false, in any case"));
    assert!(plain.contains("- Extra root keys. A document may contain frontmatter"));
    assert!(
        !plain.contains("<green>") && !plain.contains("<dim>"),
        "verbose details should not expose prose styling tags"
    );
    assert!(
        !plain.contains("whitespace inside { ... } is insignificant")
            && !plain.contains("leave original uncoerced"),
        "verbose details should use audience-facing wording instead of implementation notes"
    );
}

#[test]
fn schema_about_verbose_prints_context_and_expression_sections() {
    let output = md_cmd()
        .args(["--verbose", "schema", "about"])
        .output()
        .expect("run verbose md schema about");
    assert!(output.status.success());

    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));

    // The derived `ctx.*` catalog and typed expression-function signatures are
    // only reachable through `report.context_variables()` /
    // `report.expression_functions()`; asserting them at the binary level guards
    // against routing regressions the helper-level unit tests cannot catch.
    for needle in [
        "Context Variables",
        "ctx.now",
        "datetime",
        "ctx.packages",
        "string[]",
        "Expression Functions",
        "as_csv(list: any[]) -> string | error",
    ] {
        assert!(plain.contains(needle), "verbose schema about missing `{needle}`");
    }

    for retired in ["packages_list", "dirty_files_list"] {
        assert!(
            !plain.contains(retired),
            "verbose schema about must not surface retired `_list` catalog variable `{retired}`"
        );
    }
}

#[test]
fn schema_about_accepts_code_block_flag() {
    md_cmd()
        .args(["schema", "about", "--code-block", "light"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SimplifiedSchema"));
}

#[test]
fn schema_about_rejects_invalid_code_block_value() {
    md_cmd()
        .args(["schema", "about", "--code-block", "sideways"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'sideways'"));
}

#[test]
fn schema_about_code_block_dark_and_light_differ() {
    // The OneHalf code theme is paired, so forcing the dark vs light variant
    // must change the code-block background SGR. We assert each run carries a
    // background SGR the other does not, independent of exact RGB.
    let dark = md_cmd()
        .args(["schema", "about", "--code-block", "dark"])
        .env("FORCE_COLOR", "1")
        .env("COLORTERM", "truecolor")
        .env_remove("NO_COLOR")
        .output()
        .expect("run dark");
    let light = md_cmd()
        .args(["schema", "about", "--code-block", "light"])
        .env("FORCE_COLOR", "1")
        .env("COLORTERM", "truecolor")
        .env_remove("NO_COLOR")
        .output()
        .expect("run light");
    assert!(dark.status.success() && light.status.success());

    let dark_out = String::from_utf8_lossy(&dark.stdout);
    let light_out = String::from_utf8_lossy(&light.stdout);
    assert_ne!(
        dark_out, light_out,
        "forcing the dark vs light code-block variant must change the rendered output"
    );
}

#[test]
fn schema_about_emits_table_stripes_when_color_is_enabled() {
    let output = md_cmd()
        .args(["schema", "about"])
        .env("FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .output()
        .expect("run md schema about with color enabled");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[48"),
        "schema about tables should emit background stripes when color is enabled"
    );
}
