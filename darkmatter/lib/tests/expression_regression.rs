//! End-to-end regression tests for the Darkmatter Expression Syntax.
//!
//! Validates that arithmetic, type predicates, bracket access, date helpers,
//! string mutation helpers, and collection helpers work together in real
//! documents through the full compose pipeline.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use serde_json::json;
use std::path::Path;

/// Runs a closure with `AGENT` set to `value`, restoring the previous value
/// (or unset state) afterwards.
fn with_agent_env<F, R>(value: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let previous = std::env::var("AGENT").ok();
    unsafe {
        std::env::set_var("AGENT", value);
    }
    let result = f();
    unsafe {
        match previous {
            Some(v) => std::env::set_var("AGENT", v),
            None => std::env::remove_var("AGENT"),
        }
    }
    result
}

// ── Arithmetic in interpolation ────────────────────────────────────

#[test]
fn regression_arithmetic_and_comparison_in_interpolation() {
    let content = r#"---
price: 100
discount: 15
quantity: 3
---
Total: {{ (price - discount) * quantity }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Total: 255"));
}

#[test]
fn regression_string_concatenation_in_interpolation() {
    let content = r#"---
greeting: "Hello"
name: "World"
---
Message: {{ greeting + ", " + name + "!" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Message: Hello, World!"));
}

// ── Bracket access in interpolation ────────────────────────────────

#[test]
fn regression_bracket_index_and_member_access() {
    let content = r#"---
items:
  - name: "First"
  - name: "Second"
  - name: "Third"
---
Last item: {{ items[-1].name }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Last item: Third"));
}

#[test]
fn regression_bracket_string_key_access() {
    let content = r#"---
config:
  theme: "dark"
  lang: "en"
---
Theme: {{ config["theme"] }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Theme: dark"));
}

#[test]
fn regression_chained_bracket_and_dot_access() {
    let content = r#"---
data:
  users:
    - profile:
        name: "Alice"
    - profile:
        name: "Bob"
---
First user: {{ data["users"][0].profile.name }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("First user: Alice"));
}

// ── Type predicates in interpolation ───────────────────────────────

#[test]
fn regression_type_predicates_in_ternary() {
    let content = r#"---
value: "hello"
list:
  - 1
  - 2
---
Value is string: {{ is_string(value) ? "yes" : "no" }}
List is array: {{ is_array(list) ? "yes" : "no" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Value is string: yes"));
    assert!(composed.content().contains("List is array: yes"));
}

#[test]
fn regression_isempty_with_various_types() {
    let content = r#"---
empty_str: ""
full_str: "hello"
empty_arr: []
full_arr:
  - 1
---
Empty string: {{ is_empty(empty_str) ? "empty" : "not" }}
Full string: {{ is_empty(full_str) ? "empty" : "not" }}
Empty array: {{ is_empty(empty_arr) ? "empty" : "not" }}
Full array: {{ is_empty(full_arr) ? "empty" : "not" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Empty string: empty"));
    assert!(composed.content().contains("Full string: not"));
    assert!(composed.content().contains("Empty array: empty"));
    assert!(composed.content().contains("Full array: not"));
}

// ── String mutation helpers ────────────────────────────────────────

#[test]
fn regression_string_mutations_in_interpolation() {
    let content = r#"---
title: "hello world"
kebab: "helloWorld"
---
Upper: {{ upper(title) }}
Lower: {{ lower("HELLO") }}
Capitalize: {{ capitalize(title) }}
Kebab: {{ kebab_case(kebab) }}
Camel: {{ camel_case("hello-world") }}
Pascal: {{ pascal_case("hello_world") }}
Snake: {{ snake_case("helloWorld") }}
Title: {{ title_case("hello world") }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Upper: HELLO WORLD"));
    assert!(composed.content().contains("Lower: hello"));
    assert!(composed.content().contains("Capitalize: Hello world"));
    assert!(composed.content().contains("Kebab: hello-world"));
    assert!(composed.content().contains("Camel: helloWorld"));
    assert!(composed.content().contains("Pascal: HelloWorld"));
    assert!(composed.content().contains("Snake: hello_world"));
    assert!(composed.content().contains("Title: Hello World"));
}

#[test]
fn regression_string_predicates_in_conditions() {
    let content = r#"---
filename: "report.pdf"
prefix: "rep"
---
{{ starts_with(filename, prefix) ? "Matching prefix" : "No match" }}
{{ ends_with(filename, ".pdf") ? "Is PDF" : "Not PDF" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Matching prefix"));
    assert!(composed.content().contains("Is PDF"));
}

// ── Math helpers ───────────────────────────────────────────────────

#[test]
fn regression_math_helpers_in_interpolation() {
    let content = r#"---
a: 5
b: -3
---
Min: {{ min(a, b) }}
Max: {{ max(a, b) }}
Abs: {{ abs(b) }}
Combined: {{ max(a, abs(b)) + min(a, abs(b)) }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Min: -3"));
    assert!(composed.content().contains("Max: 5"));
    assert!(composed.content().contains("Abs: 3"));
    assert!(composed.content().contains("Combined: 8"));
}

// ── Collection helpers ─────────────────────────────────────────────

#[test]
fn regression_collection_helpers_with_bracket_access() {
    let content = r#"---
items:
  - "a"
  - "b"
  - "c"
---
First: {{ first(items) }}
Last: {{ last(items) }}
First via index: {{ items[0] }}
Last via index: {{ items[-1] }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("First: a"));
    assert!(composed.content().contains("Last: c"));
    assert!(composed.content().contains("First via index: a"));
    assert!(composed.content().contains("Last via index: c"));
}

// ── Date validators ────────────────────────────────────────────────

#[test]
fn regression_date_validators_in_ternary() {
    let content = r#"---
date_str: "2026-05-09"
invalid: "not-a-date"
---
Valid date: {{ is_date(date_str) ? "yes" : "no" }}
Invalid date: {{ is_date(invalid) ? "yes" : "no" }}
DateTime: {{ is_datetime("2026-05-09T10:30:00Z") ? "yes" : "no" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Valid date: yes"));
    assert!(composed.content().contains("Invalid date: no"));
    assert!(composed.content().contains("DateTime: yes"));
}

#[test]
fn regression_all_strict_date_validators_in_interpolation() {
    let content = r#"---
date_str: "2024-06-15"
datetime_str: "2024-06-15T12:30:00Z"
bad_str: "not-a-date"
num: 123
---
IsDate: {{ is_date(date_str) ? "yes" : "no" }}
IsDateUtc: {{ is_date_utc(date_str) ? "yes" : "no" }}
IsDateTime: {{ is_datetime(datetime_str) ? "yes" : "no" }}
IsDateTimeUtc: {{ is_datetime_utc(datetime_str) ? "yes" : "no" }}
IsDateBad: {{ is_date(bad_str) ? "yes" : "no" }}
IsDateNum: {{ is_date(num) ? "yes" : "no" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("IsDate: yes"));
    assert!(composed.content().contains("IsDateUtc: yes"));
    assert!(composed.content().contains("IsDateTime: yes"));
    assert!(composed.content().contains("IsDateTimeUtc: yes"));
    assert!(composed.content().contains("IsDateBad: no"));
    assert!(composed.content().contains("IsDateNum: no"));
}

#[test]
fn regression_relative_date_validators_in_interpolation() {
    let content = r#"---
distant: "1900-01-01"
---
IsToday: {{ is_today(distant) ? "yes" : "no" }}
IsTodayUtc: {{ is_today_utc(distant) ? "yes" : "no" }}
IsYesterday: {{ is_yesterday(distant) ? "yes" : "no" }}
IsYesterdayUtc: {{ is_yesterday_utc(distant) ? "yes" : "no" }}
IsTomorrow: {{ is_tomorrow(distant) ? "yes" : "no" }}
IsTomorrowUtc: {{ is_tomorrow_utc(distant) ? "yes" : "no" }}
IsThisMonth: {{ is_this_month(distant) ? "yes" : "no" }}
IsThisMonthUtc: {{ is_this_month_utc(distant) ? "yes" : "no" }}
IsThisYear: {{ is_this_year(distant) ? "yes" : "no" }}
IsThisYearUtc: {{ is_this_year_utc(distant) ? "yes" : "no" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("IsToday: no"));
    assert!(composed.content().contains("IsTodayUtc: no"));
    assert!(composed.content().contains("IsYesterday: no"));
    assert!(composed.content().contains("IsYesterdayUtc: no"));
    assert!(composed.content().contains("IsTomorrow: no"));
    assert!(composed.content().contains("IsTomorrowUtc: no"));
    assert!(composed.content().contains("IsThisMonth: no"));
    assert!(composed.content().contains("IsThisMonthUtc: no"));
    assert!(composed.content().contains("IsThisYear: no"));
    assert!(composed.content().contains("IsThisYearUtc: no"));
}

#[test]
fn regression_date_helpers_in_condition_mode() {
    let data = json!({
        "date_str": "2024-06-15",
        "datetime_str": "2024-06-15T12:30:00Z",
        "bad_str": "not-a-date",
        "distant": "1900-01-01"
    });

    // Strict validators - true cases
    assert!(evaluate_condition_against("is_date(date_str)", &data, Path::new(".")).unwrap());
    assert!(evaluate_condition_against("is_date_utc(date_str)", &data, Path::new(".")).unwrap());
    assert!(
        evaluate_condition_against("is_datetime(datetime_str)", &data, Path::new(".")).unwrap()
    );
    assert!(
        evaluate_condition_against("is_datetime_utc(datetime_str)", &data, Path::new(".")).unwrap()
    );

    // Strict validators - false cases
    assert!(!evaluate_condition_against("is_date(bad_str)", &data, Path::new(".")).unwrap());
    assert!(!evaluate_condition_against("is_datetime(bad_str)", &data, Path::new(".")).unwrap());

    // Relative validators - false cases with distant date
    assert!(!evaluate_condition_against("is_today(distant)", &data, Path::new(".")).unwrap());
    assert!(!evaluate_condition_against("is_today_utc(distant)", &data, Path::new(".")).unwrap());
    assert!(!evaluate_condition_against("is_yesterday(distant)", &data, Path::new(".")).unwrap());
    assert!(!evaluate_condition_against("is_this_month(distant)", &data, Path::new(".")).unwrap());
    assert!(!evaluate_condition_against("is_this_year(distant)", &data, Path::new(".")).unwrap());
}

#[test]
fn regression_page_block_with_date_helper() {
    let content = r#"---
date_str: "2024-06-15"
---
::block when="is_date(date_str)"
Valid date detected
::end-block"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Valid date detected"));

    let content_bad = r#"---
bad_str: "not-a-date"
---
::block when="is_date(bad_str)"
Valid date detected
::end-block"#;
    let md_bad: Markdown = content_bad.into();
    let (composed_bad, _) = md_bad.compose().unwrap();
    assert!(!composed_bad.content().contains("Valid date detected"));
}

// ── Object bracket access with invalid index types ────────────────

#[test]
fn regression_object_bracket_with_non_string_index_returns_null() {
    let content = r#"---
config:
  theme: "dark"
  items:
    - "a"
    - "b"
---
Numeric key: {{ config[0] }}
Float key: {{ config[1.5] }}
Bool true key: {{ config[true] }}
Bool false key: {{ config[false] }}
Null key: {{ config[missing] }}
Array key: {{ config[items] }}
Object key: {{ config[config] }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Numeric key:"));
    assert!(!composed.content().contains("Numeric key: dark"));
    assert!(composed.content().contains("Float key:"));
    assert!(!composed.content().contains("Float key: dark"));
    assert!(composed.content().contains("Bool true key:"));
    assert!(!composed.content().contains("Bool true key: dark"));
    assert!(composed.content().contains("Bool false key:"));
    assert!(!composed.content().contains("Bool false key: dark"));
    assert!(composed.content().contains("Null key:"));
    assert!(!composed.content().contains("Null key: dark"));
    assert!(composed.content().contains("Array key:"));
    assert!(!composed.content().contains("Array key: dark"));
    assert!(composed.content().contains("Object key:"));
    assert!(!composed.content().contains("Object key: dark"));
}

#[test]
fn regression_object_bracket_string_key_and_chained_access_preserved() {
    let content = r#"---
config:
  theme: "dark"
  nested:
    key: "value"
  items:
    - name: "first"
    - name: "second"
---
String key: {{ config["theme"] }}
Missing key: {{ config["missing"] }}
Nested: {{ config["nested"]["key"] }}
Chained: {{ config["items"][0].name }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("String key: dark"));
    assert!(composed.content().contains("Missing key:"));
    assert!(!composed.content().contains("Missing key: value"));
    assert!(composed.content().contains("Nested: value"));
    assert!(composed.content().contains("Chained: first"));
}

// ── Complex combined expressions ───────────────────────────────────

#[test]
fn regression_complex_expression_with_all_features() {
    let content = r#"---
users:
  - name: "Alice"
    age: 30
    tags:
      - "admin"
      - "active"
  - name: "Bob"
    age: 25
    tags: []
config:
  min_age: 18
  prefix: "User: "
---
First admin: {{ users[0].name + " (" + length(users[0].tags) + " tags)" }}
Adult check: {{ users[0].age >= config["min_age"] ? "adult" : "minor" }}
Empty tags: {{ is_empty(users[-1].tags) ? users[-1].name + " has no tags" : "has tags" }}
Formatted: {{ config.prefix + lower(users[0].name) }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(
        composed
            .content()
            .contains(r#"First admin: Alice (2 tags)"#)
    );
    assert!(composed.content().contains("Adult check: adult"));
    assert!(composed.content().contains("Empty tags: Bob has no tags"));
    assert!(composed.content().contains("Formatted: User: alice"));
}

// ── Page block conditions with new syntax ──────────────────────────

#[test]
fn regression_page_block_with_arithmetic_and_bracket_access() {
    let content = r#"---
scores:
  - 85
  - 90
  - 78
passing: 80
---
::block when="scores[-1] >= passing"
Latest score passes
::end-block"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(!composed.content().contains("Latest score passes"));

    let content_pass = r#"---
scores:
  - 85
  - 90
  - 82
passing: 80
---
::block when="scores[-1] >= passing"
Latest score passes
::end-block"#;
    let md_pass: Markdown = content_pass.into();
    let (composed_pass, _) = md_pass.compose().unwrap();
    assert!(composed_pass.content().contains("Latest score passes"));
}

#[test]
fn regression_page_block_with_type_predicates() {
    let content = r#"---
settings:
  debug: true
  features:
    - "auth"
    - "billing"
---
::block when="is_array(settings.features) && length(settings.features) > 0"
Features enabled
::end-block"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Features enabled"));
}

// ── Shortcut API with new syntax ───────────────────────────────────

#[test]
fn regression_shortcut_api_with_arithmetic_and_access() {
    let data = json!({
        "items": ["a", "b", "c"],
        "count": 3,
        "config": { "max": 5 }
    });

    let result = evaluate_condition_against(
        "items[-1] == 'c' && count + 2 <= config.max",
        &data,
        Path::new("."),
    )
    .unwrap();
    assert!(result);

    let result2 = evaluate_condition_against(
        "is_array(items) && !is_empty(items) && length(items) * 2 > count",
        &data,
        Path::new("."),
    )
    .unwrap();
    assert!(result2);
}

// ── Null propagation and error handling ────────────────────────────

#[test]
fn regression_null_propagation_in_complex_expressions() {
    let content = r#"---
user: null
items: []
---
Missing user name: {{ user.name || "anonymous" }}
Missing bracket: {{ user["email"] || "no email" }}
Empty first: {{ first(items) || "none" }}
Empty last: {{ last(items) || "none" }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Missing user name: anonymous"));
    assert!(composed.content().contains("Missing bracket: no email"));
    assert!(composed.content().contains("Empty first: none"));
    assert!(composed.content().contains("Empty last: none"));
}

#[test]
fn regression_division_by_zero_error_in_interpolation() {
    let content = r#"---
numerator: 10
denominator: 0
---
Result: {{ numerator / denominator }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    // Division by zero produces an error string in interpolation output
    assert!(composed.content().contains("Result:"));
    assert!(!composed.content().contains("Result: inf"));
}

// ── Fallback and ternary with new helpers ──────────────────────────

#[test]
fn regression_fallback_with_type_predicates_and_mutations() {
    let content = r#"---
name: ""
title: null
---
Name: {{ name || "unnamed" }}
Title: {{ title || "untitled" }}
Display: {{ is_empty(name) ? "No name provided" : upper(name) }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Name: unnamed"));
    assert!(composed.content().contains("Title: untitled"));
    assert!(composed.content().contains("Display: No name provided"));
}

// ── Compose error reporting ────────────────────────────────────────

#[test]
fn regression_division_by_zero_default_non_fail_fast() {
    let content = r#"---
numerator: 10
denominator: 0
---
Result: {{ numerator / denominator }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    // Original expression is preserved in output when evaluation fails
    assert!(
        composed
            .content()
            .contains("Result: {{ numerator / denominator }}")
    );
    // Warning contains the expression and clear reason
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Division by zero"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

// ── Phase 7 integration: new expression functions in compose ─────────

#[test]
fn regression_basename_in_interpolation() {
    let content = r#"---
path: foo/bar/baz/test.md
---
Basename: {{ basename(doc.path) }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Basename: test.md"));
}

#[test]
fn regression_terminal_in_interpolation() {
    let content = r#"---
Terminal: {{ terminal("<bold>x</bold>") }}"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    // Prose renders <bold>x</bold> with ANSI SGR sequences. In composed
    // Markdown output the `[` metacharacter is escaped, so the bold open
    // sequence appears as "\x1b\\[1m" rather than the raw "\x1b[1m".
    let content = composed.content();
    assert!(
        content.contains("\x1b\\[1m"),
        "terminal() output should contain an escaped bold SGR sequence, got: {:?}",
        content
    );
    assert!(
        content.contains("\x1b\\[0m"),
        "terminal() output should contain an escaped reset SGR sequence, got: {:?}",
        content
    );
}

#[test]
#[serial_test::serial(env_agent_model)]
fn regression_ctx_agent_in_interpolation() {
    let content = r#"---
---
Agent: {{ ctx.agent }}"#;
    let md: Markdown = content.into();
    let (composed, _) = with_agent_env("opencode", || md.compose().unwrap());
    assert!(composed.content().contains("Agent: opencode"));
}

#[test]
fn regression_ctx_agent_uses_compose_env_override() {
    let content = r#"---
---
Agent: {{ ctx.agent }}
Model: {{ ctx.model }}"#;
    let md: Markdown = content.into();
    let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
    ctx.env_mut()
        .insert("AGENT".to_string(), "  codex  ".to_string());
    ctx.env_mut()
        .insert("MODEL".to_string(), "  gpt-5  ".to_string());

    let (composed, _) = md
        .compose_with(ComposeOptions::new_with_context(ctx))
        .unwrap();

    assert!(composed.content().contains("Agent: codex"));
    assert!(composed.content().contains("Model: gpt-5"));
}

#[test]
fn regression_page_block_with_is_indexed_file() {
    let content = r#"---
path: foo/review-1.md
---
::block when="is_indexed_file(doc.path)"
Indexed file detected
::end-block"#;
    let md: Markdown = content.into();
    let (composed, _) = md.compose().unwrap();
    assert!(composed.content().contains("Indexed file detected"));
}

#[test]
#[serial_test::serial(env_agent_model)]
fn regression_page_block_with_has_skill() {
    let dir = tempfile::tempdir().unwrap();
    // Pin the tempdir as the git root so `has_skill` resolves its local skill
    // root to this directory deterministically. Without a `.git` marker here,
    // `find_git_root_from` walks all the way up and a `.git` in any shared
    // ancestor (e.g. another concurrent test's `$TMPDIR/.git`) would hijack the
    // local root, hiding the skill below and flaking this test under load.
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    let skill_root = dir.path().join(".claude").join("skills").join("foo");
    std::fs::create_dir_all(&skill_root).unwrap();
    std::fs::write(skill_root.join("SKILL.md"), "# Foo Skill\n").unwrap();

    let source_path = dir.path().join("source.md");
    let content = r#"::block when="has_skill('foo')"
Skill found
::end-block"#;
    std::fs::write(&source_path, content).unwrap();

    let md =
        darkmatter::markdown::Markdown::try_from_content(std::fs::read_to_string(&source_path).unwrap())
            .unwrap();
    // Build the options *inside* `with_agent_env`: `ComposeOptions::new()`
    // snapshots the process environment (via `ComposeContext::capture`) at
    // construction time, and `ctx.agent()` prefers that captured snapshot over
    // the live env. Constructing it before the override would freeze the agent
    // to the ambient `AGENT` value; an unrecognized agent searches neither the
    // user `.claude/skills` root nor the local one, hiding the pinned skill and
    // failing this test whenever the caller's ambient `AGENT` is not a
    // recognized agent name.
    let (composed, _) = with_agent_env("claude", || {
        let options = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source_path);
        md.compose_with(options).unwrap()
    });
    assert!(composed.content().contains("Skill found"));
}

#[test]
fn regression_remainder_by_zero_default_non_fail_fast() {
    let content = r#"---
numerator: 10
denominator: 0
---
Result: {{ numerator % denominator }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    // Original expression is preserved in output when evaluation fails
    assert!(
        composed
            .content()
            .contains("Result: {{ numerator % denominator }}")
    );
    // Warning contains the expression and clear reason
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Remainder by zero"));
    assert!(
        warning.is_some(),
        "Expected warning containing 'Remainder by zero', got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_division_by_zero_fail_fast() {
    let content = r#"---
numerator: 10
denominator: 0
---
Result: {{ numerator / denominator }}"#;
    let md: Markdown = content.into();
    let options = ComposeOptions::new().with_fail_fast(true);
    let result = md.compose_with(options);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Division by zero"),
        "Expected error containing 'Division by zero', got: {}",
        err_msg
    );
}

#[test]
fn regression_remainder_by_zero_fail_fast() {
    let content = r#"---
numerator: 10
denominator: 0
---
Result: {{ numerator % denominator }}"#;
    let md: Markdown = content.into();
    let options = ComposeOptions::new().with_fail_fast(true);
    let result = md.compose_with(options);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Remainder by zero"),
        "Expected error containing 'Remainder by zero', got: {}",
        err_msg
    );
}

#[test]
fn regression_arithmetic_type_mismatch_boolean_non_fail_fast() {
    let content = r#"---
flag: true
---
Result: {{ flag + 1 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    // Original expression is preserved in output
    assert!(composed.content().contains("Result: {{ flag + 1 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Addition requires numeric operands"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_arithmetic_type_mismatch_array_non_fail_fast() {
    let content = r#"---
items:
  - 1
  - 2
---
Result: {{ items + 1 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    // Original expression is preserved in output
    assert!(composed.content().contains("Result: {{ items + 1 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Addition requires numeric operands"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_arithmetic_type_mismatch_object_non_fail_fast() {
    let content = r#"---
obj:
  a: 1
---
Result: {{ obj + 1 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    // Original expression is preserved in output
    assert!(composed.content().contains("Result: {{ obj + 1 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Addition requires numeric operands"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_arithmetic_type_mismatch_boolean_fail_fast() {
    let content = r#"---
flag: true
---
Result: {{ flag + 1 }}"#;
    let md: Markdown = content.into();
    let options = ComposeOptions::new().with_fail_fast(true);
    let result = md.compose_with(options);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Addition requires numeric operands"),
        "Expected error about non-numeric operand, got: {}",
        err_msg
    );
}

#[test]
fn regression_arithmetic_type_mismatch_array_fail_fast() {
    let content = r#"---
items:
  - 1
  - 2
---
Result: {{ items + 1 }}"#;
    let md: Markdown = content.into();
    let options = ComposeOptions::new().with_fail_fast(true);
    let result = md.compose_with(options);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Addition requires numeric operands"),
        "Expected error about non-numeric operand, got: {}",
        err_msg
    );
}

#[test]
fn regression_arithmetic_type_mismatch_object_fail_fast() {
    let content = r#"---
obj:
  a: 1
---
Result: {{ obj + 1 }}"#;
    let md: Markdown = content.into();
    let options = ComposeOptions::new().with_fail_fast(true);
    let result = md.compose_with(options);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Addition requires numeric operands"),
        "Expected error about non-numeric operand, got: {}",
        err_msg
    );
}

#[test]
fn regression_arithmetic_type_mismatch_subtraction_non_fail_fast() {
    let content = r#"---
flag: true
---
Result: {{ flag - 1 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    assert!(composed.content().contains("Result: {{ flag - 1 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Subtraction requires numeric operands"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_arithmetic_type_mismatch_multiplication_non_fail_fast() {
    let content = r#"---
flag: true
---
Result: {{ flag * 2 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    assert!(composed.content().contains("Result: {{ flag * 2 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report.warnings.iter().find(|w| {
        w.message
            .contains("Multiplication requires numeric operands")
    });
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_arithmetic_type_mismatch_division_non_fail_fast() {
    let content = r#"---
flag: true
---
Result: {{ flag / 2 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    assert!(composed.content().contains("Result: {{ flag / 2 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Division requires numeric operands"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}

#[test]
fn regression_arithmetic_type_mismatch_remainder_non_fail_fast() {
    let content = r#"---
flag: true
---
Result: {{ flag % 2 }}"#;
    let md: Markdown = content.into();
    let (composed, report) = md.compose().unwrap();
    assert!(composed.content().contains("Result: {{ flag % 2 }}"));
    assert!(!report.warnings.is_empty());
    let warning = report
        .warnings
        .iter()
        .find(|w| w.message.contains("Remainder requires numeric operands"));
    assert!(
        warning.is_some(),
        "Expected warning about non-numeric operand, got: {:?}",
        report.warnings
    );
}
