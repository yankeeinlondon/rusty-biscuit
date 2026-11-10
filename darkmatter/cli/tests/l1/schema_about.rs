//! Integration tests for `md schema about`.

use crate::common;

use common::CliProcessFixture;
use darkmatter::markdown::schemas::{
    alphanumeric_option_descriptors, schema_constraint_descriptors, schema_type_descriptors,
};
use darkmatter::testing::strip_ansi_codes;
use predicates::prelude::*;

/// Whether `keyword` renders as the left-most (`Type`) column cell of a row in
/// the type table `section`.
///
/// AC8 requires each type to appear as its own row, not merely somewhere in the
/// report. Isolating the first pipe-delimited cell and comparing it exactly is
/// what makes this a row-level check: a keyword that only shows up inside a
/// description or code example (e.g. `yaml`/`json` in the `expression` blurb, or
/// the `number` inside `numberlike`) cannot satisfy it.
fn type_table_has_row(section: &str, keyword: &str) -> bool {
    section.lines().any(|line| {
        let trimmed = line.trim_start();
        let Some(after_border) = trimmed.strip_prefix('│') else {
            return false;
        };
        after_border
            .split('│')
            .next()
            .is_some_and(|cell| cell.trim() == keyword)
    })
}

#[test]
fn schema_about_prints_simplified_schema_reference() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
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
        "kind: schema-trigger",
        "Vacuous-match lint",
        "Match-safe constraints only",
        "use --verbose to see additional details",
    ] {
        assert!(stdout.contains(needle), "schema about missing `{needle}`");
    }
    for absent in [
        "Nested Objects",
        "Compose-time Coercion",
        "Validation Notes",
    ] {
        assert!(
            !stdout.contains(absent),
            "schema about should not show `{absent}` without --verbose"
        );
    }
}

/// The constraint table's `Meaning` cell for the row whose `Write` cell is
/// `keyword`, unwrapped back into a single line.
///
/// The column is word-wrapped to the rendered table width, so a `contains`
/// check against the descriptor's own sentence cannot work; the continuation
/// rows (empty `Write` cell) must be rejoined first.
fn constraint_row_meaning(plain: &str, keyword: &str) -> String {
    let mut meaning: Vec<&str> = Vec::new();
    for line in plain.lines() {
        let Some(after_border) = line.trim_start().strip_prefix('│') else {
            if meaning.is_empty() {
                continue;
            }
            break;
        };
        let cells: Vec<&str> = after_border.split('│').collect();
        let (Some(write), Some(description)) = (cells.first(), cells.get(2)) else {
            continue;
        };
        let write = write.trim();
        if write == keyword {
            meaning.push(description.trim());
        } else if !meaning.is_empty() {
            if !write.is_empty() {
                break;
            }
            meaning.push(description.trim());
        }
    }
    meaning.join(" ")
}

#[test]
fn schema_about_renders_the_shipped_eager_descriptor_verbatim() {
    let eager = schema_constraint_descriptors()
        .iter()
        .find(|descriptor| descriptor.keyword == "eager")
        .expect("the catalog declares `eager`");
    // Prose rendering lowers the descriptor's inline-code backticks to styled
    // spans, so the plain-text comparison drops them.
    let expected = eager.description.replace('`', "");

    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success(), "schema about should succeed");
    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));

    let rendered = constraint_row_meaning(&plain, "eager");
    assert_eq!(
        rendered, expected,
        "the shipped catalog description must reach `md schema about` verbatim: {plain}"
    );
    // The row is word-wrapped to the table column, so the retired claim can only
    // be searched for in the rejoined cell, never in the raw report text.
    assert!(
        !rendered.contains("Requires the containing property"),
        "the retired eager-implies-required claim must not ship: {rendered}"
    );
    assert!(
        rendered.contains("absence is allowed unless required is also declared"),
        "the shipped report must state that eager does not control presence: {rendered}"
    );
    assert_eq!(
        constraint_row_meaning(&plain, "required"),
        schema_constraint_descriptors()
            .iter()
            .find(|descriptor| descriptor.keyword == "required")
            .expect("the catalog declares `required`")
            .description
            .replace('`', ""),
        "the presence axis keeps its own separate row: {plain}"
    );
}

#[test]
fn verbose_schema_about_projects_shipped_git_context_descriptors() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["--verbose", "schema", "about"])
        .output()
        .expect("run verbose md schema about");
    assert!(
        output.status.success(),
        "verbose schema about should succeed"
    );
    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    let normalized = stdout.split_whitespace().collect::<Vec<_>>().join(" ");

    for needle in [
        "ctx.branch",
        "Current local Git branch name, or null outside a repository or at detached HEAD.",
        "ctx.worktree",
        "Current linked Git worktree name, or null in the main worktree or outside a repository.",
        "ctx.merge_conflicts",
        "Repository-relative paths currently in an unresolved Git index state.",
    ] {
        assert!(
            normalized.contains(needle),
            "schema about missing `{needle}`"
        );
    }
}

#[test]
fn schema_about_lists_every_supported_type_keyword() {
    let process = CliProcessFixture::new();
    let mut cmd = process.command();
    let output = cmd
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success(), "schema about should succeed");
    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));

    // The type table is the authoritative surface for AC8; bound the search to
    // it so keyword mentions in later sections (constraints, code examples)
    // cannot stand in for a missing row.
    let type_table = plain
        .split("Type System")
        .nth(1)
        .and_then(|after_heading| after_heading.split("Any of the above types").next())
        .expect("type system table should render before the array note");

    // Exhaustive: every keyword in the public descriptor catalog must appear as
    // a distinct row, so a renderer regression that drops any row fails here.
    for descriptor in schema_type_descriptors() {
        assert!(
            type_table_has_row(type_table, descriptor.keyword),
            "type table missing a row for `{}`",
            descriptor.keyword
        );
    }

    // Explicit AC8 guard: the two feature keywords must be visible in the test
    // even though the loop above already covers them via the descriptor list.
    for keyword in ["literal", "expression", "type-definition", "schema"] {
        assert!(
            type_table_has_row(type_table, keyword),
            "type table missing the `{keyword}` row required by AC8"
        );
    }
}

/// The table cells of the type-table row whose `Type` cell is `keyword`,
/// followed by its continuation rows, one `Vec` of trimmed cells per line.
fn table_row_lines(section: &str, keyword: &str) -> Vec<Vec<String>> {
    let mut lines = Vec::new();
    for line in section.lines() {
        let Some(after_border) = line.trim_start().strip_prefix('│') else {
            continue;
        };
        let cells: Vec<String> = after_border
            .split('│')
            .map(|cell| cell.trim().to_string())
            .collect();
        let first = cells.first().map(String::as_str).unwrap_or_default();
        if first == keyword {
            lines.push(cells);
        } else if !lines.is_empty() {
            if !first.is_empty() && !first.starts_with("alias:") && !first.starts_with('-') {
                break;
            }
            lines.push(cells);
        }
    }
    lines
}

/// AC10: the core schema types reach `md schema about` from the shared
/// catalog — `unknown` with its `any` alias, `null`, `shell`, and
/// `alphanumeric` with every type option and its effect.
#[test]
fn schema_about_lists_core_types_aliases_and_alphanumeric_options() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success(), "schema about should succeed");
    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));

    let type_table = plain
        .split("Type System")
        .nth(1)
        .and_then(|after_heading| after_heading.split("Any of the above types").next())
        .expect("type system table should render before the array note");
    for keyword in ["unknown", "null", "alphanumeric", "shell"] {
        assert!(type_table_has_row(type_table, keyword), "type table missing `{keyword}`");
    }
    let unknown = table_row_lines(type_table, "unknown");
    assert_eq!(
        unknown.get(1).and_then(|cells| cells.first()).map(String::as_str),
        Some("alias: any"),
        "`unknown` names its `any` alias under the keyword: {unknown:#?}"
    );
    assert!(
        !type_table_has_row(type_table, "any"),
        "`any` is an alias of `unknown`, not a row of its own"
    );

    let options_section = plain
        .split("Type Options: alphanumeric")
        .nth(1)
        .and_then(|after_heading| after_heading.split("Constraint Vocabulary").next())
        .expect("alphanumeric options render between the type notes and the constraints");
    let effects: Vec<(String, String)> = options_section
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix('│'))
        .map(|row| {
            let cells: Vec<&str> = row.split('│').map(str::trim).collect();
            (cells[0].to_string(), cells.get(1).copied().unwrap_or_default().to_string())
        })
        .filter(|(option, _)| !option.is_empty() && option != "Option")
        .collect();
    let rendered: Vec<&str> = effects.iter().map(|(option, _)| option.as_str()).collect();
    let catalog: Vec<&str> = alphanumeric_option_descriptors()
        .iter()
        .map(|option| option.keyword())
        .collect();
    assert_eq!(rendered, catalog, "every option, in canonical order: {options_section}");
    for ((_, effect), option) in effects.iter().zip(alphanumeric_option_descriptors()) {
        assert!(
            effect.eq_ignore_ascii_case(&option.effect().replace('`', "")),
            "`{}` shows its effect: {effect:?}",
            option.keyword()
        );
    }
    assert!(
        options_section.contains("- redundant without allow-dashes")
            && options_section.contains("- redundant with leading-alpha"),
        "redundancy rules are shown beside the options: {options_section}"
    );
}

#[test]
fn schema_about_describes_semantic_meta_types() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success(), "schema about should succeed");
    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    let normalized = plain.split_whitespace().collect::<Vec<_>>().join(" ");

    for needle in [
        "type-definition",
        "schema",
        "mapping",
        "sequence. Parse-only",
        "Parse-only",
        "DMLS",
    ] {
        assert!(
            normalized.contains(needle),
            "schema about missing `{needle}`"
        );
    }
}

#[test]
fn schema_about_mentions_inline_object_rules() {
    let process = CliProcessFixture::new();
    process
        .command()
        .args(["--verbose", "schema", "about"])
        .assert()
        .success()
        .stdout(predicate::str::contains("additionalProperties: false"))
        .stdout(predicate::str::contains("32"))
        .stdout(predicate::str::contains("Object shape"));
}

#[test]
fn schema_about_exits_zero() {
    let process = CliProcessFixture::new();
    process
        .command()
        .args(["schema", "about"])
        .assert()
        .success();
}

#[test]
fn schema_about_is_documentation_only() {
    let process = CliProcessFixture::new();
    // Running `md schema about` must not require or read any markdown file.
    // We assert that:
    //   1. Running from a separate empty working directory produces the same
    //      key sections as running from the fixture's default working directory.
    //   2. Both invocations exit with status `0`.
    // (Strict byte-for-byte equality is not portable: terminal capability
    // detection in the prose renderer can change the exact escape sequences
    // emitted between sessions, but the textual content is stable.)
    let empty = process.workspace_path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let output_a = process
        .command_builder()
        .ambient_context(&empty)
        .build()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about from temp dir");
    let output_b = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about from project root");

    assert!(output_a.status.success(), "first invocation should succeed");
    assert!(
        output_b.status.success(),
        "second invocation should succeed"
    );

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
fn schema_about_on_a_pipe_is_plain_text_with_literal_fences() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains('\x1b'), "no escape bytes on a pipe: {stdout:?}");
    assert!(stdout.contains("```yaml"), "code examples keep their fences: {stdout}");
    assert!(stdout.contains("Type System"), "{stdout}");
    assert!(stdout.contains('┌'), "the sections keep their tables: {stdout}");
}

#[test]
fn schema_about_verbose_prints_advanced_sections_as_readable_lists() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
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
    assert!(plain.contains("- Booleans. When the schema expects boolean, text such as"));
    assert!(plain.contains("  - Behavior: Boolean words become real booleans"));
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

/// More-context plan Phase 4, end to end through the shipped catalogs: the
/// added `ctx.*` variables and typed function signatures (including the
/// catalog's named `ip-address`, `agentic-cli`, and `recent-commits` types and
/// the `unstable` return member) reach `md --verbose schema about`.
#[test]
fn verbose_schema_about_projects_more_context_descriptors() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["--verbose", "schema", "about"])
        .output()
        .expect("run verbose md schema about");
    assert!(output.status.success());
    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    let normalized = stdout.split_whitespace().collect::<Vec<_>>().join(" ");

    for needle in [
        "ctx.self",
        "ctx.hash",
        "ctx.id",
        "ctx.sid",
        "ctx.hostname",
        "ctx.tailnet",
        "ctx.gateway_v6",
        "ctx.recent_commits",
        "recent_commits(count: number(integer;min(1))) -> recent-commits | error",
        "ping(address: ip-address, [timeout: number]) -> boolean | null | error",
        "-> boolean | enum(unstable) | null | error",
        "has_agentic_cli(agent: agentic-cli | null) -> boolean | error",
        "has_binary(name_or_path: string | null) -> boolean",
        "as_markdown(content: string | null) -> string | null | error",
    ] {
        assert!(normalized.contains(needle), "verbose schema about missing `{needle}`");
    }
}

#[test]
fn schema_about_verbose_prints_context_and_expression_sections() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
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
        "as_csv(list: unknown[] | null) -> string | null",
    ] {
        assert!(
            plain.contains(needle),
            "verbose schema about missing `{needle}`"
        );
    }

    // Every overload is its own list item.
    for overload in [
        "- `link(file: file | null) -> string | null | error` \u{2014} Creates a Markdown link to a local file,",
        "- `link(target: file | null, desc: string | null) -> string | null | error` \u{2014} Creates a Markdown link",
    ] {
        assert!(plain.contains(overload), "verbose schema about must list {overload:?}:\n{plain}");
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
    let process = CliProcessFixture::new();
    process
        .command()
        .args(["schema", "about", "--code-block", "light"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SimplifiedSchema"));
}

#[test]
fn schema_about_rejects_invalid_code_block_value() {
    let process = CliProcessFixture::new();
    process
        .command()
        .args(["schema", "about", "--code-block", "sideways"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'sideways'"));
}

#[test]
fn schema_about_code_block_variant_leaves_piped_output_unstyled() {
    let process = CliProcessFixture::new();
    for variant in ["dark", "light"] {
        let output = process
            .command()
            .args(["schema", "about", "--code-block", variant])
            .output()
            .expect("run md schema about");
        assert!(output.status.success());
        assert!(!output.stdout.contains(&0x1b), "--code-block {variant}: no escape bytes on a pipe");
    }
}

#[test]
fn schema_about_forced_color_still_has_no_escape_bytes_on_a_pipe() {
    let process = CliProcessFixture::new();
    let output = process
        .command_builder()
        .rendering_input("FORCE_COLOR", "1")
        .rendering_input_removed("NO_COLOR")
        .build()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about with color forced");
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b), "no escape bytes on a pipe, whatever the environment");
}

/// A trigger grammar form sits in an inline code span, whose contents are
/// literal, so its brackets must render without Markdown escapes.
#[test]
fn schema_about_trigger_grammar_forms_render_literally() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success(), "schema about should succeed");
    let stdout = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));
    for form in ["all: [condition, ...]", "match: [condition, ...]", "group: [condition, ...]"] {
        assert!(stdout.contains(form), "schema about must show `{form}` literally:\n{stdout}");
    }
    assert!(!stdout.contains(r"\["), "a Markdown escape leaked into a code span:\n{stdout}");
}

/// The constraint table's `Meaning` cell lines for the row whose `Write` cell
/// is `keyword`, one entry per rendered table line, trimmed.
fn constraint_row_meaning_lines(plain: &str, keyword: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in plain.lines() {
        let Some(after_border) = line.trim_start().strip_prefix('│') else {
            if lines.is_empty() {
                continue;
            }
            break;
        };
        let cells: Vec<&str> = after_border.split('│').collect();
        let (Some(write), Some(meaning)) = (cells.first(), cells.get(2)) else {
            continue;
        };
        let write = write.trim();
        if write == keyword || (!lines.is_empty() && write.is_empty()) {
            lines.push(meaning.trim().to_string());
        } else if !lines.is_empty() {
            break;
        }
    }
    lines
}

#[test]
fn min_and_max_meanings_start_each_type_on_its_own_row() {
    let process = CliProcessFixture::new();
    let output = process
        .command()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about");
    assert!(output.status.success());
    let plain = strip_ansi_codes(&String::from_utf8_lossy(&output.stdout));

    for (keyword, summary) in [("min(number)", "Sets a lower bound."), ("max(number)", "Sets an upper bound.")] {
        let lines = constraint_row_meaning_lines(&plain, keyword);
        assert_eq!(lines.first().map(String::as_str), Some(summary), "{keyword}: {lines:#?}");
        for type_name in ["string", "number", "array"] {
            assert_eq!(
                lines.iter().filter(|line| line.starts_with(&format!("- {type_name}:"))).count(),
                1,
                "{keyword}: `- {type_name}:` must start a row of its own: {lines:#?}"
            );
        }
    }
}
