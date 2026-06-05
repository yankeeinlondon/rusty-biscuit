---
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/context/capture.rs
docs_updated_during_phase_1:
  - darkmatter/docs/topics/context-variables.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/context/capture.rs
docs_updated_during_phase_2:
  - darkmatter/docs/topics/context-variables.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_3:
  - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/src/effects/mod.rs
  - darkmatter/lib/src/effects/error.rs
  - darkmatter/lib/src/effects/fs_write.rs
  - darkmatter/lib/src/effects/verbs.rs
  - darkmatter/lib/src/lib.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/lib/src/effects/verbs.rs
  - darkmatter/lib/src/effects/fs_write.rs
  - darkmatter/lib/tests/effects_integration.rs
docs_updated_during_phase_7:
  - darkmatter/docs/topics/side-effects.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages:
  - darkmatter
---

# More Context Variables — Local Parts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement every part of the `more-context-variables` spec that has **no network dependency** — new context variables, the `repo_root` fix, the local expression functions, and the local side-effect engine.

**Architecture:** Three independent subsystems, each a group of phases that ships working, testable code on its own: (1) context variables in the compose `context::capture` layer, (2) expression functions in `compose::expression`, (3) a brand-new top-level `darkmatter::effects` module (a library surface only — never wired into the compose pipeline, so `md compose` stays pure). The network-dependent slice (`http_post`, URL-accepting function variants) is deferred to `plan-remote.md`, which runs after the `url-referencing` spec is implemented.

**Tech Stack:** Rust, `chrono 0.4`, `serde_json::Value`, `IndexMap`, `biscuit-hash` (xxHash), `biscuit-file` (`FileReference`), `sniff` (repo/package data), `tempfile` (tests).

**Phases are independent** — execute and commit each in order (Phase 1 → 7), but a reviewer can sign off phase-by-phase.

---

## File Structure

| File | Responsibility | Phase |
|---|---|---|
| `darkmatter/lib/src/markdown/compose/context/capture.rs` (modify) | Add `repo_root` fix, `time_utc`/`time_military_utc`, monorepo vars | 1, 2 |
| `darkmatter/lib/src/markdown/compose/expression/functions.rs` (modify) | `date()` formatter; filesystem function impls + dispatch | 3, 5 |
| `darkmatter/lib/src/markdown/compose/expression/mod.rs` (modify) | Thread a `ResolutionContext` into filesystem-function evaluation | 4 |
| `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs` (create) | `ResolutionContext` type + filesystem-aware dispatch | 4, 5 |
| `darkmatter/lib/src/effects/mod.rs` (create) | `EffectEngine`, builder, dispatch, public API | 6, 7 |
| `darkmatter/lib/src/effects/error.rs` (create) | `EffectError` | 6 |
| `darkmatter/lib/src/effects/fs_write.rs` (create) | Atomic write + mutation-root guard + auto-rehash | 6 |
| `darkmatter/lib/src/effects/verbs.rs` (create) | Frontmatter + file/dir verb implementations | 7 |
| `darkmatter/lib/src/lib.rs` (modify) | `pub mod effects;` | 6 |
| `darkmatter/lib/tests/effects_integration.rs` (create) | tempfile-based engine tests | 6, 7 |

**Test command convention:** run from `darkmatter/` package area. Single test: `cargo test -p darkmatter <test_name>`. Full area: `just test`.

---

## Phase 1 — Small Context-Variable Changes

Grounding: `context/capture.rs`. `ContextGroup::for_key()` (≈ lines 68–165) maps a variable name to its capture group. `populate_datetime()` (≈ 609–794) and `populate_repo()` (≈ 816–912) insert values. Both `now_local` and `now_utc` are already captured (≈ 610–613). Existing time formats: `%I:%M %p` (`time`) and `%H:%M` (`time_military`). The `repo_root` insert is at ≈ 827–831 via `r.to_string_lossy()`.

### Task 1.1: Fix `repo_root` trailing slash

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/context/capture.rs` (the `repo_root` insert, ≈ 827–831)
- Test: same file's `#[cfg(test)]` module

- [ ] **Step 1: Write the failing test**

Add to the tests module in `capture.rs`:

```rust
#[test]
fn repo_root_has_no_trailing_slash() {
    let mut values = Map::new();
    // Populate repo group against this repository's root.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    populate_repo_for_test(&mut values, root);
    if let Some(Value::String(rr)) = values.get("repo_root") {
        assert!(
            !rr.ends_with('/'),
            "repo_root must not end with '/': got {rr:?}"
        );
        assert!(!rr.is_empty(), "repo_root should be non-empty in a repo");
    }
}
```

> Note: if `populate_repo` is not directly callable with an injected root, instead assert via the existing repo-population test harness used by neighboring tests (mirror the pattern of `populate_datetime_produces_all_expected_keys`). Use whichever entry the surrounding tests already use to drive `populate_repo`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter repo_root_has_no_trailing_slash`
Expected: FAIL (value currently ends with `/`).

- [ ] **Step 3: Implement the fix**

Replace the `repo_root` insert so the path is normalized to strip any trailing separator before stringifying:

```rust
values.insert(
    "repo_root".into(),
    cap.repo_root.as_ref().map_or(Value::Null, |r| {
        Value::String(strip_trailing_sep(&r.to_string_lossy()))
    }),
);
```

Add this helper near the other free functions in `capture.rs`:

```rust
/// Removes a single trailing path separator so `repo_root` is join-safe and
/// matches `sniff repo root` (which also omits the trailing `/`).
fn strip_trailing_sep(s: &str) -> String {
    let trimmed = s.strip_suffix('/').or_else(|| s.strip_suffix('\\'));
    trimmed.unwrap_or(s).to_string()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter repo_root_has_no_trailing_slash`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/context/capture.rs
git commit -m "fix(darkmatter): drop trailing slash from repo_root context variable"
```

### Task 1.2: Add `time_utc` and `time_military_utc`

**Files:**
- Modify: `capture.rs` — `ContextGroup::for_key()` DateTime arm (≈ 90–105) and `populate_datetime()` (≈ 696–704)
- Test: `capture.rs` tests module

- [ ] **Step 1: Write the failing test**

Extend the existing datetime test (or add a focused one):

```rust
#[test]
fn populate_datetime_includes_utc_time_variants() {
    let mut values = Map::new();
    populate_datetime(&mut values);
    let tu = values.get("time_utc").and_then(Value::as_str).unwrap_or("");
    let tmu = values
        .get("time_military_utc")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(tu.ends_with(" (UTC)"), "time_utc must end with ' (UTC)': {tu:?}");
    assert!(
        tmu.ends_with(" (UTC)"),
        "time_military_utc must end with ' (UTC)': {tmu:?}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter populate_datetime_includes_utc_time_variants`
Expected: FAIL (keys missing).

- [ ] **Step 3: Implement**

In `populate_datetime()`, right after the existing `time` / `time_military` inserts, add:

```rust
values.insert(
    "time_utc".into(),
    Value::String(format!("{} (UTC)", now_utc.format("%I:%M %p"))),
);
values.insert(
    "time_military_utc".into(),
    Value::String(format!("{} (UTC)", now_utc.format("%H:%M"))),
);
```

In `ContextGroup::for_key()`, add the two names to the DateTime arm alongside `"time" | "time_military"`:

```rust
"time"
| "time_military"
| "time_utc"
| "time_military_utc"
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter populate_datetime_includes_utc_time_variants`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/context/capture.rs
git commit -m "feat(darkmatter): add time_utc and time_military_utc context variables"
```

---

## Phase 2 — Monorepo Context Variables

Grounding: `sniff::filesystem::repo::{Package, RepoInfo}` is already imported in `capture.rs`. `Package` exposes `relative: String`, `package_area: String`, `name: String`, `depends_on: Vec<String>`, `used_by: Vec<String>`. `RepoInfo.packages: Option<Vec<Package>>`. `ContextCapture` (≈ 218–237) already holds `repo_info`, `current_package`, `current_package_area`, `repo_root`. New vars go in the **Repo** group: add names to `ContextGroup::for_key()` Repo arm and populate in `populate_repo()`.

Spec semantics recap:
- `area`: "" if not monorepo; package name if in a package; package-area name if in an area but not a package; "" at repo root.
- `area_description`: "" if not monorepo; `"{package} package"` in a package; `"{package-area} package area"` in an area; "" at root.
- `area_root`: repo root if not monorepo; else absolute path to the `area` root, no trailing `/`.
- `current_packages`: Markdown bullet list of packages under the current directory: `- {name} ({relative})`.
- `depends_on`: nested Markdown list, scoped to `area`. Top: `'{package}' depends on:` then nested `- {dep}`; or `'{package}' has no dependencies on other packages in this monorepo`.
- `used_by`: same shape, `'{package}' is used by:` / `'{package}' is not used by other packages in this monorepo`.

### Task 2.1: `area`, `area_description`, `area_root`

**Files:**
- Modify: `capture.rs` — Repo arm of `for_key()`; `populate_repo()`
- Test: `capture.rs` tests module

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn area_vars_empty_when_not_monorepo() {
    // Drive populate_repo with a non-monorepo capture; mirror the harness the
    // neighboring repo tests use to construct a ContextCapture.
    let cap = ContextCapture::for_test_non_monorepo();
    let mut values = Map::new();
    populate_repo(&mut values, &cap);
    assert_eq!(values.get("area"), Some(&Value::String(String::new())));
    assert_eq!(
        values.get("area_description"),
        Some(&Value::String(String::new()))
    );
    // area_root falls back to repo root (no trailing slash) when not a monorepo.
    if let Some(Value::String(ar)) = values.get("area_root") {
        assert!(!ar.ends_with('/'));
    }
}
```

> If `ContextCapture` has no test constructor yet, add a minimal `#[cfg(test)] fn for_test_non_monorepo()` that builds the struct with `repo_info: None`, `current_package: None`, `current_package_area: None`, and `repo_root: Some(<this repo>)`. Mirror field names exactly from the struct definition (≈ 218–237).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter area_vars_empty_when_not_monorepo`
Expected: FAIL (keys missing).

- [ ] **Step 3: Implement**

Add to `populate_repo()`:

```rust
let is_monorepo = cap
    .repo_info
    .as_ref()
    .map(|r| r.is_monorepo)
    .unwrap_or(false);

let (area, area_description, area_root) = if !is_monorepo {
    (
        String::new(),
        String::new(),
        cap.repo_root
            .as_ref()
            .map(|r| strip_trailing_sep(&r.to_string_lossy()))
            .unwrap_or_default(),
    )
} else if let Some(pkg) = cap.current_package.as_ref() {
    // Inside a package folder: area is the package itself.
    (
        pkg.name.clone(),
        format!("{} package", pkg.name),
        strip_trailing_sep(&pkg.path.to_string_lossy()),
    )
} else if let Some(area_name) = cap.current_package_area.as_deref().filter(|a| *a != "root") {
    // Inside a package area but not a package folder.
    let area_path = cap
        .repo_root
        .as_ref()
        .map(|r| r.join(area_name))
        .map(|p| strip_trailing_sep(&p.to_string_lossy()))
        .unwrap_or_default();
    (
        area_name.to_string(),
        format!("{area_name} package area"),
        area_path,
    )
} else {
    // Monorepo root.
    (
        String::new(),
        String::new(),
        cap.repo_root
            .as_ref()
            .map(|r| strip_trailing_sep(&r.to_string_lossy()))
            .unwrap_or_default(),
    )
};

values.insert("area".into(), Value::String(area));
values.insert("area_description".into(), Value::String(area_description));
values.insert("area_root".into(), Value::String(area_root));
```

Add `"area" | "area_description" | "area_root"` to the Repo arm of `ContextGroup::for_key()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter area_vars_empty_when_not_monorepo`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/context/capture.rs
git commit -m "feat(darkmatter): add area, area_description, area_root context variables"
```

### Task 2.2: `current_packages`

**Files:** `capture.rs` (`for_key()` Repo arm, `populate_repo()`); tests module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn current_packages_lists_packages_under_cwd_as_markdown() {
    let cap = ContextCapture::for_test_monorepo_with_packages(&[
        ("alpha", "alpha/lib"),
        ("alpha-cli", "alpha/cli"),
    ]);
    let mut values = Map::new();
    populate_repo(&mut values, &cap);
    let listing = values
        .get("current_packages")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(listing.contains("- alpha (alpha/lib)"), "got: {listing}");
    assert!(listing.contains("- alpha-cli (alpha/cli)"), "got: {listing}");
}
```

> Add a `#[cfg(test)] fn for_test_monorepo_with_packages(pkgs: &[(&str, &str)])` test constructor that builds `repo_info` with `is_monorepo: true` and a `packages` vec whose `name`/`relative` come from the tuples, and a `base_dir` at the repo root so all packages are "under" the current directory.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter current_packages_lists_packages_under_cwd_as_markdown`
Expected: FAIL.

- [ ] **Step 3: Implement**

In `populate_repo()`, build a bullet list of packages whose absolute `path` is within the capture's working directory (`cap.base_dir` — use whatever field holds the directory the capture was built for; mirror the field used by the existing `current_package` detection at ≈ 473–500):

```rust
let current_packages = cap
    .repo_info
    .as_ref()
    .and_then(|r| r.packages.as_ref())
    .map(|pkgs| {
        pkgs.iter()
            .filter(|p| p.path.starts_with(&cap.base_dir))
            .map(|p| format!("- {} ({})", p.name, p.relative))
            .collect::<Vec<_>>()
            .join("\n")
    })
    .unwrap_or_default();
values.insert("current_packages".into(), Value::String(current_packages));
```

Add `"current_packages"` to the Repo arm of `for_key()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter current_packages_lists_packages_under_cwd_as_markdown`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/context/capture.rs
git commit -m "feat(darkmatter): add current_packages context variable"
```

### Task 2.3: `depends_on` and `used_by`

**Files:** `capture.rs`; tests module.

These share a nested-list shape. Implement a shared helper, then both vars.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn depends_on_renders_nested_list_scoped_to_area() {
    let cap = ContextCapture::for_test_monorepo_in_package(
        "alpha",                       // current package
        &["beta", "gamma"],            // alpha.depends_on
        &[],                           // alpha.used_by
    );
    let mut values = Map::new();
    populate_repo(&mut values, &cap);
    let s = values.get("depends_on").and_then(Value::as_str).unwrap_or("");
    assert!(s.contains("'alpha' depends on:"), "got: {s}");
    assert!(s.contains("- beta"), "got: {s}");
    assert!(s.contains("- gamma"), "got: {s}");
}

#[test]
fn used_by_renders_empty_message_when_no_dependents() {
    let cap = ContextCapture::for_test_monorepo_in_package("alpha", &[], &[]);
    let mut values = Map::new();
    populate_repo(&mut values, &cap);
    let s = values.get("used_by").and_then(Value::as_str).unwrap_or("");
    assert!(
        s.contains("'alpha' is not used by other packages in this monorepo"),
        "got: {s}"
    );
}
```

> Add `for_test_monorepo_in_package(name, depends_on, used_by)` building a `current_package` `Package` with those fields and a single-package `repo_info`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter depends_on_renders_nested_list_scoped_to_area used_by_renders_empty_message_when_no_dependents`
Expected: FAIL (keys missing).

- [ ] **Step 3: Implement**

Add a shared renderer and both inserts in `populate_repo()`. "Scope" = the packages of the current `area`: when inside a package, that one package; when inside an area, all packages in the area; else all packages. Resolve that scope set first.

```rust
/// Renders a nested dependency listing for the scoped packages.
/// `edges(pkg)` returns the related package names; `verb`/`empty_line`
/// supply the wording (e.g. "depends on" / "has no dependencies on other
/// packages in this monorepo").
fn render_dependency_list<'a>(
    scope: &[&'a Package],
    edges: impl Fn(&Package) -> &[String],
    verb: &str,
    empty_line: impl Fn(&str) -> String,
) -> String {
    let mut out = Vec::new();
    for pkg in scope {
        let deps = edges(pkg);
        if deps.is_empty() {
            out.push(empty_line(&pkg.name));
        } else {
            out.push(format!("- '{}' {}:", pkg.name, verb));
            for d in deps {
                out.push(format!("    - {d}"));
            }
        }
    }
    out.join("\n")
}
```

Then, after computing the scoped package slice `scope: Vec<&Package>` (current package → `[pkg]`; area → packages whose `package_area == area`; else all packages):

```rust
let depends_on = render_dependency_list(
    &scope,
    |p| &p.depends_on,
    "depends on",
    |name| format!("'{name}' has no dependencies on other packages in this monorepo"),
);
values.insert("depends_on".into(), Value::String(depends_on));

let used_by = render_dependency_list(
    &scope,
    |p| &p.used_by,
    "is used by",
    |name| format!("'{name}' is not used by other packages in this monorepo"),
);
values.insert("used_by".into(), Value::String(used_by));
```

Add `"depends_on" | "used_by"` to the Repo arm of `for_key()`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter depends_on_renders_nested_list_scoped_to_area used_by_renders_empty_message_when_no_dependents`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/context/capture.rs
git commit -m "feat(darkmatter): add depends_on and used_by context variables"
```

---

## Phase 3 — `date(iso, format)` Expression Function

Grounding: `functions.rs` already has `parse_iso_date(s) -> Option<NaiveDate>` and `parse_date_or_datetime(s, assume_utc) -> Option<NaiveDate>`. `chrono::{Datelike, Local}` are imported. No ordinal-suffix helper exists. Functions return `Result<Value, String>` and are wired via `dispatch()` (lines 511–558). `date` takes 2 args.

Supported format tokens (canonical names + aliases) from spec: `"MMMM Do"`/`short`, `"MMMM Do [YYYY]"`/`short-optional`, `"MMMM Do YYYY"`, `"D MMMM [YYYY]"`, `"D MMMM YYYY"`, `"ddd, MMMM Do, YYYY"`/`long`. `[YYYY]` = include year only when it differs from the current year.

### Task 3.1: ordinal + year-aware formatting helpers

**Files:** Modify `functions.rs`; tests in its `#[cfg(test)]` module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn ordinal_suffix_covers_teens_and_units() {
    assert_eq!(ordinal_suffix(1), "st");
    assert_eq!(ordinal_suffix(2), "nd");
    assert_eq!(ordinal_suffix(3), "rd");
    assert_eq!(ordinal_suffix(4), "th");
    assert_eq!(ordinal_suffix(11), "th");
    assert_eq!(ordinal_suffix(12), "th");
    assert_eq!(ordinal_suffix(13), "th");
    assert_eq!(ordinal_suffix(21), "st");
    assert_eq!(ordinal_suffix(22), "nd");
    assert_eq!(ordinal_suffix(23), "rd");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter ordinal_suffix_covers_teens_and_units`
Expected: FAIL (function not defined).

- [ ] **Step 3: Implement**

```rust
/// English ordinal suffix for a day-of-month (1..=31). 11/12/13 are "th".
fn ordinal_suffix(day: u32) -> &'static str {
    match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter ordinal_suffix_covers_teens_and_units`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add ordinal_suffix helper for date formatting"
```

### Task 3.2: `date(iso, format)` function

**Files:** `functions.rs`; tests module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn date_formats_known_patterns() {
    let d = |iso: &str, fmt: &str| {
        date_fn(&[json!(iso), json!(fmt)]).unwrap()
    };
    assert_eq!(d("2026-07-12", "MMMM Do"), json!("July 12th"));
    assert_eq!(d("2026-07-12", "short"), json!("July 12th"));
    assert_eq!(d("2026-07-12", "MMMM Do YYYY"), json!("July 12th 2026"));
    assert_eq!(d("2021-07-12", "D MMMM YYYY"), json!("12 July 2021"));
    assert_eq!(d("2021-07-12", "long"), json!("Mon, July 12th, 2021"));
    // [YYYY] optional-year extension: omit year when it equals the current year.
    let current_year = chrono::Local::now().format("%Y").to_string();
    let same_year = format!("{current_year}-07-12");
    assert_eq!(
        date_fn(&[json!(same_year), json!("MMMM Do [YYYY]")]).unwrap(),
        json!("July 12th")
    );
    assert_eq!(
        date_fn(&[json!("1999-07-12"), json!("MMMM Do [YYYY]")]).unwrap(),
        json!("July 12th 1999")
    );
}

#[test]
fn date_errors_on_invalid_iso() {
    let err = date_fn(&[json!("not-a-date"), json!("short")]);
    assert!(err.is_err(), "expected error for invalid ISO input");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter date_formats_known_patterns date_errors_on_invalid_iso`
Expected: FAIL (function not defined).

- [ ] **Step 3: Implement**

Add the function (note `date` is the public name; Rust fn is `date_fn` to avoid the type clash, matching the existing `min_fn`/`max_fn` convention):

```rust
/// Reformats an ISO date/datetime string into a named human format.
pub fn date_fn(args: &[Value]) -> Result<Value, String> {
    require_args("date", args, 2)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let iso = require_string("date", &args[0])?;
    let fmt = require_string("date", &args[1])?;
    let parsed = parse_date_or_datetime(iso, false)
        .ok_or_else(|| format!("date() invalid ISO date or datetime: {iso:?}"))?;

    use chrono::{Datelike, Local};
    let month = parsed.format("%B").to_string(); // "July"
    let dow = parsed.format("%a").to_string(); // "Mon"
    let day = parsed.day();
    let day_ord = format!("{day}{}", ordinal_suffix(day));
    let year = parsed.year();
    let current_year = Local::now().year();
    // Year token honoring the `[YYYY]` optional-year extension.
    let opt_year = if year == current_year {
        String::new()
    } else {
        year.to_string()
    };

    let out = match fmt {
        "MMMM Do" | "short" => format!("{month} {day_ord}"),
        "MMMM Do [YYYY]" | "short-optional" => {
            join_nonempty(&[format!("{month} {day_ord}"), opt_year])
        }
        "MMMM Do YYYY" => format!("{month} {day_ord} {year}"),
        "D MMMM [YYYY]" => join_nonempty(&[format!("{day} {month}"), opt_year]),
        "D MMMM YYYY" => format!("{day} {month} {year}"),
        "ddd, MMMM Do, YYYY" | "long" => format!("{dow}, {month} {day_ord}, {year}"),
        other => return Err(format!("date() unknown format: {other:?}")),
    };
    Ok(Value::String(out))
}

/// Joins non-empty parts with a single space (used for the optional year).
fn join_nonempty(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}
```

Wire it into `dispatch()` (in the date section):

```rust
"date" => date_fn,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter date_formats_known_patterns date_errors_on_invalid_iso`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add date(iso, format) expression function"
```

---

## Phase 4 — Resolution Context for Filesystem Functions

Problem: `functions::dispatch(name, args)` is context-free, but `absolute`/`relative`/`frontmatter`/`file_exists`/`markdown_*`/`validate_schema` need the **document directory** (for relative + `@` resolution). The evaluator entry `evaluate_function(name, args, lookup)` already holds the `lookup`. We add an optional resolution context to the `EvaluationLookup` trait and a filesystem-aware dispatch branch.

### Task 4.1: `ResolutionContext` + trait method

**Files:**
- Create: `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`
- Modify: `darkmatter/lib/src/markdown/compose/expression/mod.rs` (declare module, extend trait)

- [ ] **Step 1: Write the failing test**

In `resolve_ctx.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalizes_file_scheme_and_double_slashes() {
        assert_eq!(normalize_path_arg("file://foo/bar"), "foo/bar");
        assert_eq!(normalize_path_arg("foo//bar"), "foo/bar");
        assert_eq!(normalize_path_arg("./a//b"), "./a/b");
    }

    #[test]
    fn resolution_context_default_is_cwd_no_magic() {
        let ctx = ResolutionContext::new(PathBuf::from("/tmp/docdir"));
        assert_eq!(ctx.base_dir, PathBuf::from("/tmp/docdir"));
        assert!(ctx.magic_paths.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter normalizes_file_scheme_and_double_slashes`
Expected: FAIL (module/type not defined).

- [ ] **Step 3: Implement**

`resolve_ctx.rs`:

```rust
//! Resolution context for filesystem-aware expression functions.
//!
//! Read-only: these helpers resolve and read paths; they never mutate.

use biscuit_file::PathPosition;
use std::path::PathBuf;

/// The document-relative resolution environment passed to filesystem
/// expression functions (`absolute`, `relative`, `frontmatter`, …).
#[derive(Clone, Debug, Default)]
pub struct ResolutionContext {
    /// Directory the current document lives in; relative/`@` refs resolve here.
    pub base_dir: PathBuf,
    /// Magic (`@`) search paths, mirroring the compose link-resolution config.
    pub magic_paths: Vec<(PathBuf, PathPosition)>,
}

impl ResolutionContext {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            magic_paths: Vec::new(),
        }
    }
}

/// Normalizes a filepath argument: strips a leading `file://` scheme and
/// collapses doubled `/` separators (per the spec's normalization note).
pub fn normalize_path_arg(raw: &str) -> String {
    let stripped = raw.strip_prefix("file://").unwrap_or(raw);
    // Collapse repeated slashes without touching a leading "./" or "../".
    let mut out = String::with_capacity(stripped.len());
    let mut prev_slash = false;
    for ch in stripped.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}
```

In `mod.rs`, declare the module and extend the trait with a defaulted method (non-breaking for existing impls):

```rust
pub mod resolve_ctx;
pub use resolve_ctx::ResolutionContext;
```

Add to the `EvaluationLookup` trait definition:

```rust
/// Returns the document-relative resolution context for filesystem
/// functions. Defaults to `None` (filesystem functions then error or treat
/// paths as CWD-relative).
fn resolution_context(&self) -> Option<ResolutionContext> {
    None
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter normalizes_file_scheme_and_double_slashes resolution_context_default_is_cwd_no_magic`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs darkmatter/lib/src/markdown/compose/expression/mod.rs
git commit -m "feat(darkmatter): add ResolutionContext seam for filesystem expression functions"
```

### Task 4.2: filesystem-aware dispatch branch

**Files:** Modify `mod.rs` `evaluate_function`; add `functions::dispatch_fs`.

- [ ] **Step 1: Write the failing test**

In `functions.rs` tests:

```rust
#[test]
fn dispatch_fs_returns_none_for_non_fs_names() {
    use crate::markdown::compose::expression::ResolutionContext;
    let ctx = ResolutionContext::new(std::path::PathBuf::from("."));
    assert!(dispatch_fs("lower", &[json!("x")], &ctx).is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter dispatch_fs_returns_none_for_non_fs_names`
Expected: FAIL (function not defined).

- [ ] **Step 3: Implement**

In `functions.rs`, add a context-aware dispatcher (verb impls land in Phase 5; for now it matches names and returns `None` otherwise):

```rust
use crate::markdown::compose::expression::ResolutionContext;

/// Context-aware dispatch for filesystem/document functions. Returns `None`
/// for names this dispatcher does not own (caller then tries `dispatch`).
pub fn dispatch_fs(
    name: &str,
    args: &[Value],
    ctx: &ResolutionContext,
) -> Option<Result<Value, String>> {
    let result = match name {
        "absolute" => absolute_fn(args, ctx),
        "relative" => relative_fn(args, ctx),
        "file_exists" | "fileexists" => file_exists_fn(args, ctx),
        "frontmatter" => frontmatter_fn(args, ctx),
        "markdown_body_empty" | "markdownbodyempty" => markdown_body_empty_fn(args, ctx),
        "markdown_title" | "markdowntitle" => markdown_title_fn(args, ctx),
        "validate_schema" | "validateschema" => validate_schema_fn(args, ctx),
        _ => return None,
    };
    Some(result)
}
```

In `mod.rs` `evaluate_function`, change the fall-through `other =>` arm to try the filesystem dispatcher first when a context is available:

```rust
other => {
    let evaluated: Vec<Value> = args
        .iter()
        .map(|arg| evaluate(arg, lookup))
        .collect::<Result<_, _>>()?;
    if let Some(ctx) = lookup.resolution_context() {
        if let Some(result) = functions::dispatch_fs(other, &evaluated, &ctx) {
            return result;
        }
    }
    functions::dispatch(other, &evaluated)
        .unwrap_or_else(|| Err(format!("Unknown function: {name}")))
}
```

> The verb functions (`absolute_fn`, …) are created in Phase 5. To keep this task compiling, sequence Phase 5 Task 5.1 immediately and treat Task 4.2 + Task 5.1 as one commit: do not commit Task 4.2 until Task 5.1's first verb compiles. Adjust the commit below accordingly.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter dispatch_fs_returns_none_for_non_fs_names`
Expected: PASS (after Phase 5 verbs exist).

- [ ] **Step 5: Commit** (combined with Task 5.1 if needed)

```bash
git add darkmatter/lib/src/markdown/compose/expression/mod.rs darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): route filesystem expression functions through ResolutionContext"
```

---

## Phase 5 — Filesystem Expression Functions

Grounding: resolve via `biscuit_file::FileReference::new(raw)?.add_magic_path(..).resolve_from(base_dir)` returning `Result<Option<PathBuf>, _>` (pattern from `link_resolve.rs:127–143`). Read documents via `Markdown::try_from_content(content)` / `Markdown::try_from(path)`; `md.frontmatter().as_map()`, `md.fm_get::<String>("title")`, `md.content()`. Schema via `DarkmatterSchemas::new().validate(&md) -> Result<ValidationReport, _>` with `report.valid`.

Add one shared helper for resolution, then the verbs. Each verb normalizes its path arg with `normalize_path_arg` first.

### Task 5.1: shared resolve helper + `absolute` + `file_exists`

**Files:** `functions.rs`; tests module (unit tests with tempfile preferred here).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn absolute_and_file_exists_resolve_relative_to_base_dir() {
    use crate::markdown::compose::expression::ResolutionContext;
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.md"), "# A\n").unwrap();
    let ctx = ResolutionContext::new(dir.path().to_path_buf());

    let abs = absolute_fn(&[json!("a.md")], &ctx).unwrap();
    assert_eq!(
        abs,
        json!(dir.path().join("a.md").to_string_lossy().to_string())
    );

    assert_eq!(file_exists_fn(&[json!("a.md")], &ctx).unwrap(), json!(true));
    assert_eq!(
        file_exists_fn(&[json!("missing.md")], &ctx).unwrap(),
        json!(false)
    );
    // Invalid path string → file_exists is false (never errors).
    assert_eq!(file_exists_fn(&[json!("\0bad")], &ctx).unwrap(), json!(false));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter absolute_and_file_exists_resolve_relative_to_base_dir`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
use crate::markdown::compose::expression::resolve_ctx::normalize_path_arg;
use std::path::PathBuf;

/// Resolves a filepath argument to an absolute path using FileReference rules
/// and the document-relative base dir. `Ok(None)` means "not found"; `Err`
/// means the reference string itself was invalid.
fn resolve_arg(
    raw: &str,
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, String> {
    let normalized = normalize_path_arg(raw);
    let mut file_ref = biscuit_file::FileReference::new(&normalized)
        .map_err(|e| format!("invalid file path {raw:?}: {e}"))?;
    for (path, position) in &ctx.magic_paths {
        file_ref = file_ref.add_magic_path(path, *position);
    }
    file_ref
        .resolve_from(&ctx.base_dir)
        .map_err(|e| format!("invalid file path {raw:?}: {e}"))
}

/// `absolute(file) -> file | Error::InvalidFilePath`
pub fn absolute_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("absolute", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("absolute", &args[0])?;
    match resolve_arg(raw, ctx)? {
        Some(p) => Ok(Value::String(p.to_string_lossy().to_string())),
        None => Err(format!("absolute() invalid file path: {raw:?}")),
    }
}

/// `file_exists(file) -> bool` — invalid paths return `false`, never error.
pub fn file_exists_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("file_exists", args, 1)?;
    if any_null(args) {
        return Ok(Value::Bool(false));
    }
    let raw = match require_string("file_exists", &args[0]) {
        Ok(s) => s,
        Err(_) => return Ok(Value::Bool(false)),
    };
    let exists = match resolve_arg(raw, ctx) {
        Ok(Some(p)) => p.exists(),
        _ => false,
    };
    Ok(Value::Bool(exists))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter absolute_and_file_exists_resolve_relative_to_base_dir`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add absolute() and file_exists() expression functions"
```

### Task 5.2: `relative`

**Files:** `functions.rs`; tests module.

Grounding: reuse the compose relative-path logic in `link_normalization.rs` if a reusable function exists; otherwise compute relative-to-repo-root, falling back to `~`/env aliases per spec. Inspect `link_normalization.rs` for an existing `to_portable_path`-style helper and call it; only implement fresh logic if none is reusable.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn relative_returns_repo_or_cwd_relative_path() {
    use crate::markdown::compose::expression::ResolutionContext;
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/a.md"), "# A\n").unwrap();
    let ctx = ResolutionContext::new(dir.path().to_path_buf());
    let rel = relative_fn(&[json!("sub/a.md")], &ctx).unwrap();
    assert_eq!(rel, json!("sub/a.md"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter relative_returns_repo_or_cwd_relative_path`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
/// `relative(file) -> file | Error::InvalidFilePath`
pub fn relative_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("relative", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("relative", &args[0])?;
    let abs = match resolve_arg(raw, ctx)? {
        Some(p) => p,
        None => return Err(format!("relative() invalid file path: {raw:?}")),
    };
    // Prefer repo-root-relative; fall back to base_dir-relative; then ~ / env.
    let rel = make_relative(&abs, &ctx.base_dir);
    Ok(Value::String(rel))
}
```

Add `make_relative` by reusing the compose helper. If `link_normalization.rs` exposes a portable-path function, call it; otherwise add:

```rust
/// Best-effort relative rendering: repo-root relative when inside the repo,
/// else base_dir-relative, else `~`-aliased home path, else the absolute path.
fn make_relative(abs: &std::path::Path, base_dir: &std::path::Path) -> String {
    if let Ok(repo) = git_repo_root(base_dir) {
        if let Ok(stripped) = abs.strip_prefix(&repo) {
            return stripped.to_string_lossy().to_string();
        }
    }
    if let Ok(stripped) = abs.strip_prefix(base_dir) {
        return stripped.to_string_lossy().to_string();
    }
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = abs.strip_prefix(&home) {
            return format!("~/{}", stripped.to_string_lossy());
        }
    }
    abs.to_string_lossy().to_string()
}
```

> `git_repo_root(base_dir)`: reuse the repo-root discovery the context layer already uses (`sniff::filesystem::git::GitRepo::discover` or the existing helper in `compose/mod.rs:144` that walks for `.git`). Do not add a new git dependency — call the existing path. Confirm `dirs` is already a dependency (it is used elsewhere in the workspace); if not present in `darkmatter/lib/Cargo.toml`, prefer the existing home-dir helper rather than adding a dep.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter relative_returns_repo_or_cwd_relative_path`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add relative() expression function"
```

### Task 5.3: `frontmatter(file)` / `frontmatter(file, prop)`

**Files:** `functions.rs`; tests module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn frontmatter_reads_whole_map_and_single_prop() {
    use crate::markdown::compose::expression::ResolutionContext;
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("d.md"),
        "---\ntitle: Hi\nstatus: draft\n---\nBody\n",
    )
    .unwrap();
    let ctx = ResolutionContext::new(dir.path().to_path_buf());

    let whole = frontmatter_fn(&[json!("d.md")], &ctx).unwrap();
    assert_eq!(whole["title"], json!("Hi"));

    let one = frontmatter_fn(&[json!("d.md"), json!("status")], &ctx).unwrap();
    assert_eq!(one, json!("draft"));

    // Missing prop → null (not error).
    let missing = frontmatter_fn(&[json!("d.md"), json!("nope")], &ctx).unwrap();
    assert_eq!(missing, Value::Null);

    // Invalid filepath → error.
    assert!(frontmatter_fn(&[json!("does-not-exist.md")], &ctx).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter frontmatter_reads_whole_map_and_single_prop`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
use crate::markdown::Markdown;

/// Loads a Markdown file via the resolution context. `Err` if the path is
/// invalid or unreadable.
fn load_markdown(raw: &str, ctx: &ResolutionContext, fname: &str) -> Result<Markdown, String> {
    let path = resolve_arg(raw, ctx)?
        .ok_or_else(|| format!("{fname}() invalid file path: {raw:?}"))?;
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("{fname}() invalid file path {raw:?}: {e}"))?;
    Markdown::try_from_content(content)
        .map_err(|e| format!("{fname}() failed to parse {raw:?}: {e}"))
}

/// `frontmatter(file)` → object; `frontmatter(file, prop)` → value | null.
pub fn frontmatter_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err("frontmatter() requires 1 or 2 arguments".to_string());
    }
    if matches!(args.first(), Some(Value::Null)) {
        return Ok(Value::Null);
    }
    let raw = require_string("frontmatter", &args[0])?;
    let md = load_markdown(raw, ctx, "frontmatter")?;
    let map = md.frontmatter().as_map();
    if args.len() == 1 {
        let obj: serde_json::Map<String, Value> =
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        return Ok(Value::Object(obj));
    }
    let prop = require_string("frontmatter", &args[1])?;
    Ok(map.get(prop).cloned().unwrap_or(Value::Null))
}
```

The `dispatch_fs` arity was already wired in Task 4.2.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter frontmatter_reads_whole_map_and_single_prop`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add frontmatter(file[, prop]) expression function"
```

### Task 5.4: `markdown_body_empty` + `markdown_title`

**Files:** `functions.rs`; tests module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn markdown_body_empty_and_title() {
    use crate::markdown::compose::expression::ResolutionContext;
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("empty.md"), "---\ntitle: T\n---\n\n   \n").unwrap();
    std::fs::write(dir.path().join("full.md"), "---\n---\n# Heading\n\nWords\n").unwrap();
    std::fs::write(dir.path().join("fm_title.md"), "---\ntitle: FM\n---\n# H1\n").unwrap();
    let ctx = ResolutionContext::new(dir.path().to_path_buf());

    assert_eq!(
        markdown_body_empty_fn(&[json!("empty.md")], &ctx).unwrap(),
        json!(true)
    );
    assert_eq!(
        markdown_body_empty_fn(&[json!("full.md")], &ctx).unwrap(),
        json!(false)
    );
    // Frontmatter title wins over H1.
    assert_eq!(
        markdown_title_fn(&[json!("fm_title.md")], &ctx).unwrap(),
        json!("FM")
    );
    // No frontmatter title → first H1.
    assert_eq!(
        markdown_title_fn(&[json!("full.md")], &ctx).unwrap(),
        json!("Heading")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter markdown_body_empty_and_title`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
/// `markdown_body_empty(file) -> bool | Error` — body has only whitespace.
pub fn markdown_body_empty_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("markdown_body_empty", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("markdown_body_empty", &args[0])?;
    let md = load_markdown(raw, ctx, "markdown_body_empty")?;
    Ok(Value::Bool(md.content().trim().is_empty()))
}

/// `markdown_title(file) -> string | null | Error` — frontmatter `title`,
/// else first H1. Multiple H1s: first wins, warning to STDERR.
pub fn markdown_title_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    require_args("markdown_title", args, 1)?;
    if any_null(args) {
        return Ok(Value::Null);
    }
    let raw = require_string("markdown_title", &args[0])?;
    let md = load_markdown(raw, ctx, "markdown_title")?;
    if let Some(t) = md
        .frontmatter()
        .as_map()
        .get("title")
        .and_then(Value::as_str)
    {
        return Ok(Value::String(t.to_string()));
    }
    let h1s: Vec<String> = md
        .content()
        .lines()
        .filter_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string()))
        .collect();
    match h1s.as_slice() {
        [] => Ok(Value::Null),
        [single] => Ok(Value::String(single.clone())),
        [first, ..] => {
            eprintln!(
                "markdown_title(): multiple H1 headings in {raw:?}; using the first"
            );
            Ok(Value::String(first.clone()))
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter markdown_body_empty_and_title`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add markdown_body_empty() and markdown_title() functions"
```

### Task 5.5: `validate_schema(file)` / `validate_schema(file, obj)`

**Files:** `functions.rs`; tests module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn validate_schema_true_when_no_schema_property() {
    use crate::markdown::compose::expression::ResolutionContext;
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("plain.md"), "---\ntitle: T\n---\nBody\n").unwrap();
    let ctx = ResolutionContext::new(dir.path().to_path_buf());
    assert_eq!(
        validate_schema_fn(&[json!("plain.md")], &ctx).unwrap(),
        json!(true)
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter validate_schema_true_when_no_schema_property`
Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
use crate::markdown::schemas::DarkmatterSchemas;

/// `validate_schema(file)` / `validate_schema(file, obj)` -> bool | Error.
/// Returns `true` when the document declares no `$schema`.
pub fn validate_schema_fn(args: &[Value], ctx: &ResolutionContext) -> Result<Value, String> {
    if args.is_empty() || args.len() > 2 {
        return Err("validate_schema() requires 1 or 2 arguments".to_string());
    }
    if matches!(args.first(), Some(Value::Null)) {
        return Ok(Value::Null);
    }
    let raw = require_string("validate_schema", &args[0])?;
    let md = load_markdown(raw, ctx, "validate_schema")?;
    // No `$schema` → always valid (per spec).
    if !md.frontmatter().as_map().contains_key("$schema") {
        return Ok(Value::Bool(true));
    }
    let schemas = DarkmatterSchemas::new();
    let report = schemas
        .validate(&md)
        .map_err(|e| format!("validate_schema() error for {raw:?}: {e}"))?;
    Ok(Value::Bool(report.valid))
}
```

> The 2-arg form (`validate_schema(file, obj)`) validates an explicit object against the file's schema. If `DarkmatterSchemas` lacks a "validate arbitrary object against this doc's schema" entry point, add a focused method there in a follow-up task; for v1, the 2-arg form may validate the document as in the 1-arg form and is acceptable to land after the 1-arg form. Track this gap explicitly rather than leaving a silent stub.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter validate_schema_true_when_no_schema_property`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/expression/functions.rs
git commit -m "feat(darkmatter): add validate_schema() expression function"
```

---

## Phase 6 — Effects Engine Scaffold

A brand-new top-level module `darkmatter::effects`. It is a **library surface only** — never referenced by the compose pipeline. Grounding: frontmatter model `FrontmatterMap = IndexMap<String, serde_json::Value>`; `Markdown::frontmatter_mut().as_map_mut()`, `Markdown::as_string()`; hashing via `Markdown::plan_hash_save(stored, &MdHashOptions) -> SaveDecision` + `apply_hash_save(&decision, &opts, today) -> Option<String>`. No atomic write exists — implement temp-file + rename. Mirror loop-DSL semantics from `claudine/lib/src/composition/loop_actions.rs` (do NOT depend on claudine).

### Task 6.1: module + `EffectError`

**Files:**
- Create: `darkmatter/lib/src/effects/error.rs`, `darkmatter/lib/src/effects/mod.rs`
- Modify: `darkmatter/lib/src/lib.rs` (add `pub mod effects;` after `pub mod editor;`)

- [ ] **Step 1: Write the failing test**

In `effects/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_variants_render_messages() {
        let e = EffectError::OutsideMutationRoot {
            path: "/etc/passwd".into(),
            root: "/repo".into(),
        };
        let msg = e.to_string();
        assert!(msg.contains("/etc/passwd"));
        assert!(msg.contains("/repo"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter error_variants_render_messages`
Expected: FAIL (type not defined).

- [ ] **Step 3: Implement**

`effects/error.rs`:

```rust
//! Error type for the side-effect engine.

use std::path::PathBuf;
use thiserror::Error;

/// Errors raised by [`crate::effects::EffectEngine`] operations.
#[derive(Debug, Error)]
pub enum EffectError {
    #[error("invalid file path: {0}")]
    InvalidFilePath(String),

    #[error("refusing to write outside the mutation root: {path} (root: {root})")]
    OutsideMutationRoot { path: PathBuf, root: PathBuf },

    #[error("network host not allowed: {0}")]
    HostNotAllowed(String),

    #[error("frontmatter property {prop:?} has the wrong type for {op}")]
    PropertyType { op: &'static str, prop: String },

    #[error("io error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("markdown error: {0}")]
    Markdown(String),
}
```

`effects/mod.rs` (initial):

```rust
//! Side-effect engine: a callable catalog of *mutating* operations.
//!
//! Unlike the read-only expression engine, these operations change external
//! state. The engine is deliberately **not** wired into the compose pipeline —
//! composing a document never invokes a side effect. Only an external
//! orchestrator (e.g. Claudine's lifecycle stack) drives it.

mod error;
mod fs_write;
mod verbs;

pub use error::EffectError;
```

In `lib.rs`, add:

```rust
pub mod effects;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter error_variants_render_messages`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/effects/ darkmatter/lib/src/lib.rs
git commit -m "feat(darkmatter): scaffold effects module and EffectError"
```

### Task 6.2: atomic write + mutation-root guard

**Files:** `effects/fs_write.rs`.

- [ ] **Step 1: Write the failing test**

In `effects/fs_write.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_inside_root_succeeds_and_outside_is_refused() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let inside = root.join("ok.txt");
        atomic_write_guarded(&root, &inside, b"hi").unwrap();
        assert_eq!(std::fs::read_to_string(&inside).unwrap(), "hi");

        let outside = root.parent().unwrap().join("escape.txt");
        let err = atomic_write_guarded(&root, &outside, b"no").unwrap_err();
        assert!(matches!(err, crate::effects::EffectError::OutsideMutationRoot { .. }));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter write_inside_root_succeeds_and_outside_is_refused`
Expected: FAIL.

- [ ] **Step 3: Implement**

`effects/fs_write.rs`:

```rust
//! Mutation-root-guarded atomic file writes.

use crate::effects::EffectError;
use std::path::Path;

/// Writes `bytes` to `target` atomically (temp file in the same directory +
/// rename), but only if `target` resolves inside `root`. Creates parent
/// directories under `root` as needed.
pub(crate) fn atomic_write_guarded(
    root: &Path,
    target: &Path,
    bytes: &[u8],
) -> Result<(), EffectError> {
    let normalized = normalize_within(root, target)?;
    if let Some(parent) = normalized.parent() {
        std::fs::create_dir_all(parent).map_err(|source| EffectError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut tmp = tempfile::NamedTempFile::new_in(
        normalized.parent().unwrap_or(root),
    )
    .map_err(|source| EffectError::Io {
        path: normalized.clone(),
        source,
    })?;
    use std::io::Write;
    tmp.write_all(bytes).map_err(|source| EffectError::Io {
        path: normalized.clone(),
        source,
    })?;
    tmp.persist(&normalized).map_err(|e| EffectError::Io {
        path: normalized.clone(),
        source: e.error,
    })?;
    Ok(())
}

/// Resolves `target` and verifies it is contained within `root`. Uses
/// lexical containment after joining relative targets onto `root`.
fn normalize_within(root: &Path, target: &Path) -> Result<std::path::PathBuf, EffectError> {
    let joined = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    // Reject `..` escapes lexically (the path may not exist yet, so canonicalize
    // only the existing prefix).
    let cleaned = lexically_clean(&joined);
    if !cleaned.starts_with(root) {
        return Err(EffectError::OutsideMutationRoot {
            path: cleaned,
            root: root.to_path_buf(),
        });
    }
    Ok(cleaned)
}

/// Removes `.` and resolves `..` segments lexically without touching disk.
fn lexically_clean(path: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Public-in-crate containment check returning the cleaned in-root path.
pub(crate) fn ensure_within(
    root: &Path,
    target: &Path,
) -> Result<std::path::PathBuf, EffectError> {
    normalize_within(root, target)
}
```

> Confirm `tempfile` is a non-dev dependency of `darkmatter/lib` (it appears as both a normal and dev dep per exploration). If it is dev-only, move it to `[dependencies]` since the engine uses it at runtime, and update `docs/dependencies.md`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter write_inside_root_succeeds_and_outside_is_refused`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/effects/fs_write.rs darkmatter/lib/Cargo.toml
git commit -m "feat(darkmatter): add mutation-root-guarded atomic write for effects"
```

### Task 6.3: `EffectEngine` builder + auto-rehash config

**Files:** `effects/mod.rs`.

- [ ] **Step 1: Write the failing test**

In `effects/mod.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builder_sets_defaults() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = EffectEngine::builder()
            .mutation_root(dir.path())
            .build();
        assert!(engine.auto_rehash());
        assert!(engine.allowed_hosts().is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter builder_sets_defaults`
Expected: FAIL.

- [ ] **Step 3: Implement**

Append to `effects/mod.rs`:

```rust
use std::path::{Path, PathBuf};

/// The mutating side-effect engine. Construct via [`EffectEngine::builder`].
#[derive(Clone, Debug)]
pub struct EffectEngine {
    mutation_root: PathBuf,
    allowed_hosts: Vec<String>,
    auto_rehash: bool,
}

impl EffectEngine {
    pub fn builder() -> EffectEngineBuilder {
        EffectEngineBuilder::default()
    }
    pub fn mutation_root(&self) -> &Path {
        &self.mutation_root
    }
    pub fn allowed_hosts(&self) -> &[String] {
        &self.allowed_hosts
    }
    pub fn auto_rehash(&self) -> bool {
        self.auto_rehash
    }
}

/// Builder for [`EffectEngine`].
#[derive(Debug)]
pub struct EffectEngineBuilder {
    mutation_root: PathBuf,
    allowed_hosts: Vec<String>,
    auto_rehash: bool,
}

impl Default for EffectEngineBuilder {
    fn default() -> Self {
        Self {
            mutation_root: PathBuf::from("."),
            allowed_hosts: Vec::new(), // deny-all by default
            auto_rehash: true,
        }
    }
}

impl EffectEngineBuilder {
    pub fn mutation_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.mutation_root = root.into();
        self
    }
    pub fn allowed_hosts<I, S>(mut self, hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }
    pub fn auto_rehash(mut self, on: bool) -> Self {
        self.auto_rehash = on;
        self
    }
    pub fn build(self) -> EffectEngine {
        EffectEngine {
            mutation_root: self.mutation_root,
            allowed_hosts: self.allowed_hosts,
            auto_rehash: self.auto_rehash,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter builder_sets_defaults`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/effects/mod.rs
git commit -m "feat(darkmatter): add EffectEngine builder with mutation root, hosts, auto-rehash"
```

---

## Phase 7 — Effect Verbs (Local)

Grounding: load a `Markdown`, mutate `frontmatter_mut().as_map_mut()` (an `IndexMap`, preserves key order), optionally re-hash, then `as_string()` and `atomic_write_guarded`. All verbs are methods on `EffectEngine`. Paths are normalized via a small engine-local `normalize_path_arg` (kept here to avoid coupling to the expression module). Argument values are `serde_json::Value`.

### Task 7.1: load/save helper with auto-rehash + `set_frontmatter`

**Files:** `effects/verbs.rs`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectEngine;
    use serde_json::{json, Value};

    #[test]
    fn set_frontmatter_writes_and_rehashes() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("d.md");
        std::fs::write(&file, "---\ntitle: T\nhash: stale\n---\nBody\n").unwrap();
        let eng = EffectEngine::builder().mutation_root(dir.path()).build();

        let prior = eng.set_frontmatter("d.md", "status", json!("in-progress")).unwrap();
        assert_eq!(prior, Value::Null); // status did not exist before

        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("status: in-progress"));
        assert!(written.contains("title: T"));
        // hash was recomputed (no longer the literal "stale").
        assert!(!written.contains("hash: stale"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter set_frontmatter_writes_and_rehashes`
Expected: FAIL.

- [ ] **Step 3: Implement**

`effects/verbs.rs`:

```rust
//! Verb implementations for the side-effect engine.

use crate::effects::error::EffectError;
use crate::effects::fs_write::atomic_write_guarded;
use crate::effects::EffectEngine;
use crate::markdown::hash::MdHashOptions;
use crate::markdown::Markdown;
use serde_json::Value;
use std::path::PathBuf;

impl EffectEngine {
    /// Resolves a (normalized) path argument against the mutation root.
    fn resolve(&self, raw: &str) -> Result<PathBuf, EffectError> {
        let normalized = normalize_path_arg(raw);
        let joined = std::path::Path::new(&normalized);
        let abs = if joined.is_absolute() {
            joined.to_path_buf()
        } else {
            self.mutation_root().join(joined)
        };
        Ok(abs)
    }

    /// Loads a Markdown file for mutation.
    fn load(&self, raw: &str) -> Result<(PathBuf, Markdown), EffectError> {
        let path = self.resolve(raw)?;
        let content = std::fs::read_to_string(&path).map_err(|source| EffectError::Io {
            path: path.clone(),
            source,
        })?;
        let md = Markdown::try_from_content(content)
            .map_err(|e| EffectError::Markdown(e.to_string()))?;
        Ok((path, md))
    }

    /// Serializes, optionally re-hashes, and atomically writes the document.
    fn save(&self, raw_path: &str, md: &Markdown) -> Result<(), EffectError> {
        let path = self.resolve(raw_path)?;
        let serialized = if self.auto_rehash()
            && md.frontmatter().as_map().contains_key("hash")
        {
            let opts = MdHashOptions::default();
            let decision = md
                .plan_hash_save(None, &opts)
                .map_err(|e| EffectError::Markdown(e.to_string()))?;
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            md.apply_hash_save(&decision, &opts, &today)
                .unwrap_or_else(|| md.as_string())
        } else {
            md.as_string()
        };
        atomic_write_guarded(self.mutation_root(), &path, serialized.as_bytes())
    }

    /// `set_frontmatter(file, prop, value)` → prior value (or null).
    pub fn set_frontmatter(
        &self,
        file: &str,
        prop: &str,
        value: Value,
    ) -> Result<Value, EffectError> {
        let (_, mut md) = self.load(file)?;
        let prior = md
            .frontmatter()
            .as_map()
            .get(prop)
            .cloned()
            .unwrap_or(Value::Null);
        md.frontmatter_mut().as_map_mut().insert(prop.to_string(), value);
        self.save(file, &md)?;
        Ok(prior)
    }
}

/// Strips a leading `file://` and collapses doubled `/` (engine-local copy of
/// the expression-engine normalization, kept here to avoid coupling).
fn normalize_path_arg(raw: &str) -> String {
    let stripped = raw.strip_prefix("file://").unwrap_or(raw);
    let mut out = String::with_capacity(stripped.len());
    let mut prev_slash = false;
    for ch in stripped.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter set_frontmatter_writes_and_rehashes`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/effects/verbs.rs
git commit -m "feat(darkmatter): add set_frontmatter effect verb with auto-rehash"
```

### Task 7.2: remaining frontmatter verbs

Implement `merge_frontmatter`, `delete_frontmatter`, `increment_frontmatter`, `decrement_frontmatter`, `append_frontmatter`, `prepend_frontmatter`. Mirror loop-DSL semantics (increment/decrement: missing→1/-1, numeric strings parsed; merge: shallow, target object-or-null; append/prepend: array mutation).

**Files:** `effects/verbs.rs`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn frontmatter_mutation_verbs() {
    use serde_json::json;
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("d.md");
    std::fs::write(&file, "---\nphase: 1\ntags: [a]\n---\nBody\n").unwrap();
    let eng = EffectEngine::builder().mutation_root(dir.path()).build();

    assert_eq!(eng.increment_frontmatter("d.md", "phase").unwrap(), json!(2));
    assert_eq!(eng.decrement_frontmatter("d.md", "phase").unwrap(), json!(1));
    assert_eq!(
        eng.append_frontmatter("d.md", "tags", json!("b")).unwrap(),
        json!(["a", "b"])
    );
    assert_eq!(
        eng.prepend_frontmatter("d.md", "tags", json!("z")).unwrap(),
        json!(["z", "a", "b"])
    );
    let merged = eng.merge_frontmatter("d.md", json!({"owner": "ken"})).unwrap();
    assert_eq!(merged["owner"], json!("ken"));
    let removed = eng.delete_frontmatter("d.md", "owner").unwrap();
    assert_eq!(removed, json!("ken"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter frontmatter_mutation_verbs`
Expected: FAIL.

- [ ] **Step 3: Implement**

Add these methods inside `impl EffectEngine` in `verbs.rs`:

```rust
/// `merge_frontmatter(file, obj)` → merged object (shallow).
pub fn merge_frontmatter(&self, file: &str, obj: Value) -> Result<Value, EffectError> {
    let incoming = obj.as_object().ok_or(EffectError::PropertyType {
        op: "merge_frontmatter",
        prop: "<obj>".to_string(),
    })?;
    let (_, mut md) = self.load(file)?;
    {
        let map = md.frontmatter_mut().as_map_mut();
        for (k, v) in incoming {
            map.insert(k.clone(), v.clone());
        }
    }
    self.save(file, &md)?;
    let merged: serde_json::Map<String, Value> = md
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(Value::Object(merged))
}

/// `delete_frontmatter(file, prop)` → removed value (or null).
pub fn delete_frontmatter(&self, file: &str, prop: &str) -> Result<Value, EffectError> {
    let (_, mut md) = self.load(file)?;
    let removed = md
        .frontmatter_mut()
        .as_map_mut()
        .shift_remove(prop)
        .unwrap_or(Value::Null);
    self.save(file, &md)?;
    Ok(removed)
}

/// `increment_frontmatter(file, prop)` → new number (missing → 1).
pub fn increment_frontmatter(&self, file: &str, prop: &str) -> Result<Value, EffectError> {
    self.bump(file, prop, 1)
}

/// `decrement_frontmatter(file, prop)` → new number (missing → -1).
pub fn decrement_frontmatter(&self, file: &str, prop: &str) -> Result<Value, EffectError> {
    self.bump(file, prop, -1)
}

fn bump(&self, file: &str, prop: &str, delta: i64) -> Result<Value, EffectError> {
    let (_, mut md) = self.load(file)?;
    let current = md.frontmatter().as_map().get(prop).cloned();
    let n: i64 = match current {
        None | Some(Value::Null) => 0,
        Some(Value::Number(num)) => num.as_i64().ok_or(EffectError::PropertyType {
            op: "increment_frontmatter",
            prop: prop.to_string(),
        })?,
        Some(Value::String(s)) => s.trim().parse::<i64>().map_err(|_| {
            EffectError::PropertyType {
                op: "increment_frontmatter",
                prop: prop.to_string(),
            }
        })?,
        Some(_) => {
            return Err(EffectError::PropertyType {
                op: "increment_frontmatter",
                prop: prop.to_string(),
            })
        }
    };
    let next = Value::Number((n + delta).into());
    md.frontmatter_mut().as_map_mut().insert(prop.to_string(), next.clone());
    self.save(file, &md)?;
    Ok(next)
}

/// `append_frontmatter(file, prop, value)` → new array.
pub fn append_frontmatter(&self, file: &str, prop: &str, value: Value) -> Result<Value, EffectError> {
    self.array_mutate(file, prop, value, true)
}

/// `prepend_frontmatter(file, prop, value)` → new array.
pub fn prepend_frontmatter(&self, file: &str, prop: &str, value: Value) -> Result<Value, EffectError> {
    self.array_mutate(file, prop, value, false)
}

fn array_mutate(
    &self,
    file: &str,
    prop: &str,
    value: Value,
    append: bool,
) -> Result<Value, EffectError> {
    let (_, mut md) = self.load(file)?;
    let mut arr = match md.frontmatter().as_map().get(prop).cloned() {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(EffectError::PropertyType {
                op: if append { "append_frontmatter" } else { "prepend_frontmatter" },
                prop: prop.to_string(),
            })
        }
    };
    if append {
        arr.push(value);
    } else {
        arr.insert(0, value);
    }
    let new_value = Value::Array(arr);
    md.frontmatter_mut().as_map_mut().insert(prop.to_string(), new_value.clone());
    self.save(file, &md)?;
    Ok(new_value)
}
```

> `IndexMap` removal is `shift_remove` (preserves order) — confirm the `indexmap` version in use exposes it; if it is an older version, use `.remove(...)` (older versions preserve order via swap — prefer `shift_remove` if available).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter frontmatter_mutation_verbs`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/effects/verbs.rs
git commit -m "feat(darkmatter): add merge/delete/increment/decrement/append/prepend frontmatter verbs"
```

### Task 7.3: file & directory verbs

Implement `ensure_file(file)` / `ensure_file_with_content(file, content)`, `ensure_dir(dir)`, `append_line(file, text)`, `append_jsonl(file, obj)`. All guarded by the mutation root.

**Files:** `effects/verbs.rs`; integration test `darkmatter/lib/tests/effects_integration.rs`.

- [ ] **Step 1: Write the failing tests**

`darkmatter/lib/tests/effects_integration.rs`:

```rust
use darkmatter::effects::EffectEngine;
use serde_json::json;

#[test]
fn file_and_dir_verbs() {
    let dir = tempfile::TempDir::new().unwrap();
    let eng = EffectEngine::builder().mutation_root(dir.path()).build();

    // ensure_dir
    let made = eng.ensure_dir("out/logs").unwrap();
    assert!(std::path::Path::new(&made).is_dir());

    // ensure_file idempotent: returns absolute path, leaves existing untouched
    let p = eng.ensure_file("out/state.md").unwrap();
    std::fs::write(&p, "preexisting").unwrap();
    let p2 = eng.ensure_file("out/state.md").unwrap();
    assert_eq!(p, p2);
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "preexisting");

    // ensure_file with content only writes when creating
    let c = eng.ensure_file_with_content("out/seed.md", "seed").unwrap();
    assert_eq!(std::fs::read_to_string(&c).unwrap(), "seed");

    // append_line
    eng.append_line("out/logs/run.log", "first").unwrap();
    eng.append_line("out/logs/run.log", "second").unwrap();
    let log = std::fs::read_to_string(dir.path().join("out/logs/run.log")).unwrap();
    assert_eq!(log, "first\nsecond\n");

    // append_jsonl
    eng.append_jsonl("out/logs/events.jsonl", json!({"ok": true})).unwrap();
    let jsonl = std::fs::read_to_string(dir.path().join("out/logs/events.jsonl")).unwrap();
    assert_eq!(jsonl.trim(), r#"{"ok":true}"#);

    // mutation-root escape is refused
    assert!(eng.ensure_file("../escape.md").is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter --test effects_integration file_and_dir_verbs`
Expected: FAIL.

- [ ] **Step 3: Implement**

Add to `impl EffectEngine` in `verbs.rs`:

```rust
/// `ensure_file(file)` → absolute path; creates an empty file if missing.
pub fn ensure_file(&self, file: &str) -> Result<String, EffectError> {
    self.ensure_file_inner(file, None)
}

/// `ensure_file(file, content)` → absolute path; writes `content` only when
/// creating a missing file (existing files are left unchanged).
pub fn ensure_file_with_content(&self, file: &str, content: &str) -> Result<String, EffectError> {
    self.ensure_file_inner(file, Some(content))
}

fn ensure_file_inner(&self, file: &str, content: Option<&str>) -> Result<String, EffectError> {
    let path = self.resolve(file)?;
    // Verify containment up front (covers both the create and no-op paths).
    let cleaned = crate::effects::fs_write::ensure_within(self.mutation_root(), &path)?;
    if !cleaned.exists() {
        atomic_write_guarded(
            self.mutation_root(),
            &cleaned,
            content.unwrap_or("").as_bytes(),
        )?;
    }
    Ok(cleaned.to_string_lossy().to_string())
}

/// `ensure_dir(dir)` → absolute path (`mkdir -p`).
pub fn ensure_dir(&self, dir: &str) -> Result<String, EffectError> {
    let path = self.resolve(dir)?;
    let cleaned = crate::effects::fs_write::ensure_within(self.mutation_root(), &path)?;
    std::fs::create_dir_all(&cleaned).map_err(|source| EffectError::Io {
        path: cleaned.clone(),
        source,
    })?;
    Ok(cleaned.to_string_lossy().to_string())
}

/// `append_line(file, text)` → absolute path.
pub fn append_line(&self, file: &str, text: &str) -> Result<String, EffectError> {
    let path = self.resolve(file)?;
    let cleaned = crate::effects::fs_write::ensure_within(self.mutation_root(), &path)?;
    let mut existing = std::fs::read_to_string(&cleaned).unwrap_or_default();
    existing.push_str(text);
    existing.push('\n');
    atomic_write_guarded(self.mutation_root(), &cleaned, existing.as_bytes())?;
    Ok(cleaned.to_string_lossy().to_string())
}

/// `append_jsonl(file, obj)` → absolute path.
pub fn append_jsonl(&self, file: &str, obj: Value) -> Result<String, EffectError> {
    let line = serde_json::to_string(&obj)
        .map_err(|e| EffectError::Markdown(e.to_string()))?;
    self.append_line(file, &line)
}
```

`ensure_within` was added to `fs_write.rs` in Task 6.2.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter --test effects_integration file_and_dir_verbs`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/effects/verbs.rs darkmatter/lib/tests/effects_integration.rs
git commit -m "feat(darkmatter): add ensure_file/ensure_dir/append_line/append_jsonl effect verbs"
```

---

## Final Verification

- [ ] **Run the full area test suite**

Run: `just test` (from `darkmatter/`)
Expected: all pass.

- [ ] **Lint**

Run: `just lint` (from `darkmatter/`)
Expected: clean.

- [ ] **Docs drift pass**

Per repo `CLAUDE.md` "Drift Maintenance": update `darkmatter/docs/topics/context-variables.md` (new vars), `darkmatter-expressions.md` (new functions), and confirm `side-effects.md` matches the shipped local verb set. Regenerate any skill `hash:` frontmatter with `md hash <file>` if a skill was edited.

---

## Self-Review Notes (spec coverage)

- New context vars: `area`, `area_description`, `area_root` (Task 2.1), `current_packages` (Task 2.2), `depends_on`, `used_by` (Task 2.3), `time_utc`, `time_military_utc` (Task 1.2). ✅
- `repo_root` trailing-slash fix (Task 1.1). ✅
- Expression functions: `absolute`, `relative`, `file_exists`, `frontmatter`, `markdown_body_empty`, `markdown_title`, `validate_schema` (Tasks 5.1–5.5), `date` (Task 3.2). ✅
- Filepath normalization note (file://, //) — `normalize_path_arg` (Task 4.1), reused in effects (Task 7.1). ✅
- Side effects local catalog: frontmatter verbs (Tasks 7.1–7.2), file/dir verbs (Task 7.3); engine + mutation-root + auto-rehash (Phase 6). ✅
- **Deferred to `plan-remote.md`:** `http_post` and URL-accepting variants of the read functions (require `url-referencing`). ✅
- **Known follow-up flagged, not stubbed silently:** `validate_schema(file, obj)` 2-arg form (Task 5.5).
</content>
