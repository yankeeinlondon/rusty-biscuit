# Implementation Plan: Review-4 Follow-ups for Apple Terminal Integration

This plan addresses every actionable recommendation in
`biscuit-terminal/features/2026-05-02-apple-terminal/review-4.md`.

Review-4 confirmed the feature is **production-ready**: the four review-3
findings have all landed, AC-1 through AC-6 are met, and the Level-2 suite was
verified end-to-end against a real Terminal.app on the reviewer's host. The
remaining findings are cosmetic / ergonomic / edge-case and do **not** block
shipping. This plan tightens the loose ends without changing observable
behaviour for any current production caller.

## Scope summary

| # | Finding | Severity | Phase |
|---|---------|----------|-------|
| 1 | Level-2 link-fallback test lacks `__BT_START__` / `__BT_END__` sentinels | Low | Phase 1 |
| 2 | `disable_color_forcing` is a workaround, not a fix (harness ergonomics) | Low | Phase 2 |
| 3 | Double-underline degradation policy duplicated between block tag and atomic token | Low | Phase 3 |
| 4 | `<a>` markdown fallback does not escape `]` in description content | Low | Phase 4 (OPTIONAL — see decision below) |
| - | Curly / dotted / dashed underline capability awareness | Info / future | **Out of scope** (post-merge per review-4 §Recommendations 4) |

### Decision: Phase 4 (`]` escape in `<a>` markdown fallback)

**Included as an optional, well-isolated final phase.**

The reviewer flagged it as a "portable correctness papercut" with no current
production or test caller exercising bracketed descriptions. The fix is
trivially achievable (one `replace` call plus one unit test), the blast radius
is bounded to one branch of `block_tag_to_escape`, and the change has zero
interaction with phases 1-3. Including it now closes the only currently-known
correctness gap without risking ship readiness — but Phase 4 is structured so
it can be **dropped at execution time** (skip the phase, skip its commit) if
the executor decides to defer. Phases 1-3 do not depend on Phase 4, and Phase
4 does not depend on Phases 1-3.

If deferred, file an issue referencing review-4 §Findings ("Low — `<a>`
markdown fallback does not escape `]` in description content") and the
recommended `.replace(']', "\\]")` fix.

---

## Working directory & monorepo conventions

Working directory for all commands: the rusty-biscuit worktree root
(`/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/`).

> **Workspace gotcha:** never run `cargo build` / `cargo test` / `cargo clippy`
> at the repo root without `-p`. Every command in this plan is scoped with
> `-p` flags.

> **Lint policy:** clippy must run across **all three packages affected by
> this plan** as a single invocation (`-p biscuit-terminal -p
> biscuit-terminal-cli -p biscuit-test-harness --all-targets -- -D warnings`)
> regardless of which package owns the warning. A clippy regression in any of
> the three breaks the whole feature area.

> **Commits:** no `Co-Authored-By:` trailer (per global instructions).

---

## Phase ordering & cross-phase dependencies

```
        ┌───────────┐
        │  Phase 1  │  Sentinel bracket Level-2 link test
        └─────┬─────┘
              │ (no dep)
              ▼
        ┌───────────┐
        │  Phase 2  │  preserve_capabilities() opt-out + migrate Level-2
        └─────┬─────┘  prose tests; remove disable_color_forcing
              │ (depends on Phase 1: same test file gets edited)
              ▼
        ┌───────────┐
        │  Phase 3  │  Extract degraded_underline_open() helper (refactor only)
        └─────┬─────┘  ── independent of 1 & 2; can run in parallel
              │
              ▼
        ┌───────────┐
        │  Phase 4  │  (OPTIONAL) `]` escape in <a> markdown fallback
        └─────┬─────┘  ── independent of all prior phases
              │
              ▼
        ┌───────────┐
        │  Phase 5  │  Final cross-package verification (build/lint/fmt/test)
        └───────────┘
```

**Phase ordering rationale:**

- Phase 1 → Phase 2: both edit `cli/tests/level2_apple_terminal_prose.rs`.
  Doing them sequentially avoids merge conflicts on the same file. Phase 1 is
  the smallest mechanical change (test-only) so it lands first; Phase 2 then
  refactors the harness and migrates both Level-2 tests on top of Phase 1's
  sentinel-bracketed structure.
- Phase 3 is a pure-Rust library refactor with **no test changes**. It does
  not touch `level2_apple_terminal_prose.rs` or the harness. It can be
  developed and committed in any order relative to Phases 1, 2, and 4. We
  list it after Phase 2 only so the final test sweep in Phase 5 runs against
  one combined working tree.
- Phase 4 (optional) is an isolated change to one branch of
  `block_tag_to_escape` plus one unit test. Independent of all prior phases.
- Phase 5 is the final whole-area verification: it must run **after every
  earlier phase has landed** so any cross-phase interaction (e.g. clippy
  warnings introduced by helper extraction) is caught.

Each phase ends with a self-contained build/lint/test gate so it is
independently verifiable before its commit.

---

## Phase 1 — Sentinel-bracket the Level-2 link-fallback test

**Severity:** Low. Eliminates a structurally identical false-positive risk to
the one already closed for the double-underline test in review-3.

**Source of finding:** review-4 §Findings "Low — Level-2 link-fallback test
does not use sentinels"; review-4 §Recommendations item 1.

### Files to modify

- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs` (function
  `level2_apple_terminal_link_fallback_visible`, currently lines 87-132)

### Reference pattern

The double-underline test at the same file's lines **162-189** is the canonical
template:

```rust
harness
    .send_text(
        b"printf '__BT_START__\\n'; bt prose '<...>'; printf '\\n__BT_END__\\n'\n",
    )
    .expect("send_text failed");
harness.settle();
std::thread::sleep(Duration::from_millis(400));

let frame = harness.capture().expect("capture failed");

let bounded = frame
    .plain
    .split("__BT_START__\n")
    .nth(1)
    .and_then(|s| s.split("\n__BT_END__").next())
    .unwrap_or("");

assert!(!bounded.is_empty(), "...");
assert!(bounded.contains("..."), "...");
```

### Changes

Replace the `send_bt_command(&mut harness, "prose '<a href=…>click here</a>'")`
call (currently at lines 100-103) with a `harness.send_text(...)` call that
brackets the `bt prose` invocation in `__BT_START__` / `__BT_END__` printf
sentinels, mirroring the double-underline test exactly.

The shell command sent must be (single line, with the inner double-quotes
escaped at the Rust string level, then again at the shell level — the
double-underline test demonstrates the working pattern):

```sh
printf '__BT_START__\n'; bt prose '<a href="https://example.com">click here</a>'; printf '\n__BT_END__\n'
```

After capture, slice the bounded region the same way the double-underline
test does:

```rust
let bounded = frame
    .plain
    .split("__BT_START__\n")
    .nth(1)
    .and_then(|s| s.split("\n__BT_END__").next())
    .unwrap_or("");
```

Migrate every existing assertion (visible label, `(https://example.com)`,
negative `\x1b]8;;`, negative `8;;https://example.com`) to run against
`bounded` rather than `frame.plain`. Add the standard sentinel-empty guard
that the double-underline test uses:

```rust
assert!(
    !bounded.is_empty(),
    "sentinel-bounded output is empty — bt prose likely crashed or emitted nothing.\n\
     full capture:\n{}",
    frame.plain,
);
```

The settle-and-sleep timing (`harness.settle()` followed by
`std::thread::sleep(Duration::from_millis(400))`) must match the
double-underline test's timing. Do not preserve the prior `send_bt_command`
indirection — sending the printf+bt+printf sequence directly via `send_text`
(as the double-underline test does) keeps the bracketing readable in one
place.

### Tests added / modified

- Modified: `level2_apple_terminal_link_fallback_visible` (assertions move
  from `frame.plain` to `bounded`; sentinels added).
- No new tests.

### Acceptance criteria

- `level2_apple_terminal_link_fallback_visible` still passes against a real
  Terminal.app on macOS (skip-clean off-macOS / in CI).
- The test no longer references `send_bt_command` for the bt invocation; the
  printf+bt+printf one-liner is sent via `harness.send_text`.
- `bounded` is a `&str` slice of `frame.plain` and every assertion runs
  against `bounded` (except the sentinel-empty guard which intentionally
  references `frame.plain` for the failure diagnostic).

### Verification commands

```sh
cargo build -p biscuit-terminal -p biscuit-terminal-cli
cargo nextest run -p biscuit-terminal-cli --test level2_apple_terminal_prose
cargo fmt --check
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness --all-targets -- -D warnings
```

If `cargo nextest` skips `level2_apple_terminal_link_fallback_visible` because
the host is off-macOS or in CI, that is the expected skip-clean path; rerun
the assertion manually on a macOS host before committing if practical.

### Commit message

```
test(biscuit-terminal): sentinel-bracket Level-2 link-fallback assertion

Mirrors the double-underline test's printf '__BT_START__' / '__BT_END__'
sentinel pattern so the link-fallback assertion runs against a bounded
slice of the capture rather than the full frame. Eliminates the
command-echo / prompt false-positive risk flagged in review-4.
```

---

## Phase 2 — Add `preserve_capabilities` opt-out to `AppleTerminalHarness`

**Severity:** Low. Removes the `disable_color_forcing` workaround and
prevents future capability-degradation tests from silently landing in the
`Terminal::new_forced` path.

**Source of finding:** review-4 §Findings "Low — `disable_color_forcing` is a
workaround, not a fix"; review-4 §Recommendations item 2.

### Files to modify

- `biscuit-test-harness/src/apple_terminal.rs` (add builder field + setter;
  gate the `FORCE_COLOR=1 CLICOLOR_FORCE=1` exports currently at line ~209)
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs` (migrate
  prose / capability tests to `preserve_capabilities(true)`; delete the
  `disable_color_forcing` helper and its call sites once unused)

### Design

Add a builder-style toggle that defaults to `false` (current behaviour
preserved — image and color tests keep their forced SGR contract). When set
to `true`, the spawned shell is started **without** the
`FORCE_COLOR=1 CLICOLOR_FORCE=1` exports so `bt`'s
`detect_terminal_honoring_force_color` runs natural detection and honours
Apple Terminal's actual capability profile.

```rust
pub struct AppleTerminalHarness {
    window_id: Option<i64>,
    preserve_capabilities: bool,
}

impl AppleTerminalHarness {
    pub fn new() -> Self {
        Self { window_id: None, preserve_capabilities: false }
    }

    /// Suppresses the `FORCE_COLOR=1 CLICOLOR_FORCE=1` exports the harness
    /// otherwise injects into the spawned shell, so capability detection
    /// runs against Terminal.app's natural profile.
    ///
    /// Use this for tests that exercise `Prose` graceful-degradation paths
    /// — Apple Terminal does not implement OSC8 hyperlinks or double
    /// underline, and forcing color flips `bt` into the
    /// `Terminal::new_forced` path which unconditionally enables both,
    /// defeating the very degradation being tested.
    ///
    /// Image / color tests should leave this at the default (`false`).
    pub fn preserve_capabilities(mut self, yes: bool) -> Self {
        self.preserve_capabilities = yes;
        self
    }
}
```

In `<AppleTerminalHarness as TerminalHarness>::spawn_shell`, gate the
`FORCE_COLOR` / `CLICOLOR_FORCE` push at the existing line ~209:

```rust
if !self.preserve_capabilities {
    shell_cmd.push_str("FORCE_COLOR=1 CLICOLOR_FORCE=1 ");
}
```

The `TERM` / `COLORTERM` blocks immediately below are **not** gated — they
do not affect capability detection (Apple Terminal already advertises a
truecolor `xterm-256color`-class TERM in normal use), and dropping them
would regress every existing image / color test that depends on a
deterministic `TERM` value. Document this explicitly with a `//` comment
inside the `if !self.preserve_capabilities` block so future readers do not
expand the gate.

Update the rustdoc on `spawn_shell` so the "Color-forcing env vars" paragraph
points at `preserve_capabilities` for the opt-out path.

### Migration of Level-2 prose tests

In `cli/tests/level2_apple_terminal_prose.rs`:

1. Replace each `let mut harness = AppleTerminalHarness::new();` in the prose
   tests (`level2_apple_terminal_link_fallback_visible` and
   `level2_apple_terminal_double_underline_plain_text_visible`) with:

   ```rust
   let mut harness = AppleTerminalHarness::new().preserve_capabilities(true);
   ```

2. Delete the `disable_color_forcing(&mut harness);` call after each
   `spawn_shell` in those tests.

3. Delete the `disable_color_forcing` helper function itself (currently
   `cli/tests/level2_apple_terminal_prose.rs:51-74`) once no remaining call
   site exists. Verify with a final grep:

   ```sh
   grep -n disable_color_forcing biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs
   ```

   This should return zero matches after the migration.

4. Do **not** add `preserve_capabilities(true)` to
   `level2_apple_terminal_harness_lifecycle` — that test only checks
   spawn / capture / Drop and does not depend on capability negotiation. Leave
   it on the default (forced color) so its behaviour is unchanged from
   review-4's verified state. The rationale in code:

   ```rust
   // Lifecycle test does not touch capability-aware paths; default
   // forced-color env keeps it consistent with the image / color tests.
   ```

### Tests added / modified

- Modified: `level2_apple_terminal_link_fallback_visible` (use
  `preserve_capabilities(true)`; drop `disable_color_forcing` call).
- Modified: `level2_apple_terminal_double_underline_plain_text_visible`
  (same).
- Removed: `disable_color_forcing` helper.
- New (harness unit test in `biscuit-test-harness/src/apple_terminal.rs`
  `mod tests`):

  ```rust
  #[test]
  fn preserve_capabilities_default_is_false() {
      let h = AppleTerminalHarness::new();
      assert!(!h.preserve_capabilities);
  }

  #[test]
  fn preserve_capabilities_setter_toggles() {
      let h = AppleTerminalHarness::new().preserve_capabilities(true);
      assert!(h.preserve_capabilities);
  }
  ```

  These are pure-builder tests — they do not invoke `spawn_shell` and run
  cleanly off-macOS / in CI. They guard against accidental field renames
  and against future refactors that re-introduce unconditional forcing.

### Acceptance criteria

- `AppleTerminalHarness` exposes `pub fn preserve_capabilities(self, bool)
  -> Self` and a private `preserve_capabilities: bool` field defaulting to
  `false`.
- `spawn_shell` only emits `FORCE_COLOR=1 CLICOLOR_FORCE=1 ` when
  `self.preserve_capabilities == false`.
- The `disable_color_forcing` helper is removed from
  `level2_apple_terminal_prose.rs`. `grep -n disable_color_forcing` returns
  zero matches across the whole monorepo (the symbol was test-local and is
  not used anywhere else).
- The two prose Level-2 tests pass on a real Terminal.app **without** any
  manual env-var manipulation in the test body.
- `level2_apple_terminal_harness_lifecycle` is unchanged.
- The two new harness builder unit tests pass on every platform (off-macOS,
  in CI, on macOS) — they exercise no Terminal.app code paths.

### Verification commands

```sh
cargo build -p biscuit-test-harness -p biscuit-terminal -p biscuit-terminal-cli
cargo nextest run -p biscuit-test-harness
cargo nextest run -p biscuit-terminal-cli --test level2_apple_terminal_prose
cargo fmt --check
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness --all-targets -- -D warnings
```

The Level-2 prose suite skips clean off-macOS and in CI. Where possible
re-run on a macOS host with Terminal.app to confirm the natural-detection
path still produces the expected `[click here](https://example.com)` and
plain-text double-underline.

### Commit message

```
feat(biscuit-test-harness): add preserve_capabilities opt-out to AppleTerminalHarness

Builder-style toggle suppresses the FORCE_COLOR=1 CLICOLOR_FORCE=1 exports
that bt's detect_terminal_honoring_force_color otherwise interprets as a
forced-color profile (which unconditionally enables osc_link_support and
supports_italic, defeating Apple Terminal graceful-degradation tests).

Migrates the Level-2 prose tests to preserve_capabilities(true) and
removes the disable_color_forcing workaround. Image / color tests keep
the default (forced) behaviour.
```

---

## Phase 3 — Extract `degraded_underline_open` helper

**Severity:** Low. Pure refactor. Identical four-arm match exists at two
sites; consolidating them is groundwork for the curly/dotted/dashed TODO at
`prose.rs:375-381` (post-merge work, **not** in this plan).

**Source of finding:** review-4 §Findings "Low — `block_tag_to_escape` and
`atomic_token_to_escape_with_term` duplicate the double-underline policy";
review-4 §Recommendations item 3.

### Files to modify

- `biscuit-terminal/lib/src/components/prose.rs` (call sites at lines
  **287-294** and **407-416**; new helper added near both)

### Current duplicated logic

Atomic token (`prose.rs:287-294`):

```rust
return match term {
    None => Some(Cow::Borrowed("\x1b[4:2m")),
    Some(t) if t.underline_support.double => Some(Cow::Borrowed("\x1b[4:2m")),
    Some(t) if t.underline_support.straight => Some(Cow::Borrowed("\x1b[4m")),
    Some(_) => None,
};
```

Block tag (`prose.rs:407-416`):

```rust
match term {
    None => Some(wrap_static("\x1b[4:2m", "\x1b[24m")),
    Some(t) if t.underline_support.double => {
        Some(wrap_static("\x1b[4:2m", "\x1b[24m"))
    }
    Some(t) if t.underline_support.straight => {
        Some(wrap_static("\x1b[4m", "\x1b[24m"))
    }
    Some(_) => Some(BlockTagAction::Suppress),
}
```

### Helper signature

Place the helper just **above** `atomic_token_to_escape_with_term` so both
call sites can see it. Helper returns the *opening* SGR only; the closing
SGR (`\x1b[24m`) is invariant across all double-underline variants and can
stay inline at each call site:

```rust
/// Resolves the opening SGR escape for a `<double-underline>` request
/// against the terminal's actual underline-support profile.
///
/// Returns:
///
/// - `Some("\x1b[4:2m")` when no terminal context is available (legacy
///   optimistic behavior) **or** when the terminal advertises
///   [`UnderlineSupport::double`].
/// - `Some("\x1b[4m")` when only [`UnderlineSupport::straight`] is
///   advertised — the canonical Apple Terminal path.
/// - `None` when neither variant is supported, signalling the caller to
///   suppress the underline entirely (no SGR at all, including no
///   `\x1b[0m` reset — `state.used_styles` must remain unchanged).
///
/// The closing SGR for any non-`None` return is always `"\x1b[24m"` and
/// is intentionally not returned by this helper.
fn degraded_double_underline_open(term: Option<&Terminal>) -> Option<&'static str> {
    match term {
        None => Some("\x1b[4:2m"),
        Some(t) if t.underline_support.double => Some("\x1b[4:2m"),
        Some(t) if t.underline_support.straight => Some("\x1b[4m"),
        Some(_) => None,
    }
}
```

> **Naming rationale:** the function name in the planning prompt was
> `degraded_underline_open(term, kind)` with a `kind` parameter
> anticipating curly/dotted/dashed. Per review-4 §Recommendations 4, that
> generalization is explicitly post-merge / future work and is **out of
> scope of this plan**. We pick the narrower `degraded_double_underline_open`
> so the helper accurately describes its current behaviour. When the future
> work lands, the helper is the obvious extension point: rename to
> `degraded_underline_open(term, UnderlineKind)` and add arms for `Curly`,
> `Dotted`, `Dashed`. This is signposted with a one-line `// TODO` comment
> immediately above the helper referencing
> `features/2026-05-02-apple-terminal/spec.md`.

### Refactor at the atomic-token site (`prose.rs:287-294`)

```rust
if token.eq_ignore_ascii_case("double-underline") {
    return degraded_double_underline_open(term).map(Cow::Borrowed);
}
```

### Refactor at the block-tag site (`prose.rs:407-416`)

```rust
"double-underline" | "uu" => match degraded_double_underline_open(term) {
    Some(open) => Some(wrap_static(open, "\x1b[24m")),
    None => Some(BlockTagAction::Suppress),
},
```

The doc comment at lines 402-406 describing the four-arm policy can stay
above the `"double-underline" | "uu"` arm verbatim — the policy is
unchanged; it has just been moved into the helper.

### Tests added / modified

This is a **pure refactor** with no observable-behaviour change. Existing
tests are the regression net:

- `lib/tests/level1_apple_terminal_prose.rs` (six tests covering both block
  and atomic forms with all four capability combinations, including the
  `atomic_double_underline_no_underline_support_emits_plain_text` test
  added in review-3).
- All `test_double_underline_suppressed_…` and
  `atomic_double_underline_suppressed_…` unit tests inside `prose.rs`.

**Optional tightening:** if it does not exist already, add a tiny
unit test directly on the helper (it is private but reachable from the
crate's `mod tests`):

```rust
#[test]
fn degraded_double_underline_open_no_term_is_optimistic() {
    assert_eq!(degraded_double_underline_open(None), Some("\x1b[4:2m"));
}
```

Skip this if `prose.rs`'s existing capability-matrix tests already pin the
behaviour at both call sites — duplicating coverage adds maintenance cost
for no signal.

### Acceptance criteria

- A new private helper `degraded_double_underline_open` exists in
  `prose.rs` with the documented signature and behaviour above.
- Both `atomic_token_to_escape_with_term` and `block_tag_to_escape` call
  the helper instead of carrying their own four-arm `match` blocks.
- The full Level-1 PTY suite
  (`cargo nextest run -p biscuit-terminal --test level1_apple_terminal_prose`)
  passes unchanged.
- The full library unit-test suite
  (`cargo nextest run -p biscuit-terminal --lib`) passes unchanged.
- Clippy emits zero new warnings on the entire feature area.

### Verification commands

```sh
cargo build -p biscuit-terminal
cargo nextest run -p biscuit-terminal --lib
cargo nextest run -p biscuit-terminal --test level1_apple_terminal_prose
cargo fmt --check
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness --all-targets -- -D warnings
```

### Commit message

```
refactor(biscuit-terminal): extract degraded_double_underline_open helper

Consolidates the four-arm double-underline degradation match that
appeared identically in both block_tag_to_escape and
atomic_token_to_escape_with_term. Pure refactor; the Level-1 PTY suite
and prose unit tests are unchanged.

Pre-emptive groundwork for the curly/dotted/dashed UnderlineSupport
TODO at prose.rs:375-381 (out of scope of the 2026-05-02 feature).
```

---

## Phase 4 — (OPTIONAL) Escape `]` in `<a>` markdown fallback description

**Severity:** Low. Out of scope of the spec's fixture set ("click here" /
"important text"); no current production caller exercises bracketed
descriptions. Reviewer flagged it as a "portable correctness papercut".

**Source of finding:** review-4 §Findings "Low — `<a>` markdown fallback does
not escape `]` in description content".

> **Skip rule:** if the executor decides at start-of-phase that this is not
> trivially achievable for any reason (unexpected interaction with the
> token-stream renderer, surprising width-counting impact, etc.), skip Phase
> 4 entirely, document the reason, and file an issue. Phases 1-3 and Phase 5
> do not depend on Phase 4 in any way.

### Files to modify

- `biscuit-terminal/lib/src/components/prose.rs` (the `else` branch at
  lines **447-458** of `block_tag_to_escape`'s `"a"` arm — the markdown
  fallback path)

### Current behaviour

`block_tag_to_escape` emits the markdown fallback as:

```rust
Some(BlockTagAction::Wrap {
    open: Cow::Borrowed("["),
    close: Cow::Owned(format!("]({})", resolved_href)),
})
```

The renderer then concatenates `open + inner + close`, where `inner` is the
literal description text (e.g. `array[0]`). When `inner` contains a `]`
the rendered output is `[array[0]](https://example.com)`, which standard
CommonMark parsers split incorrectly.

### Constraint

`BlockTagAction::Wrap`'s `open` and `close` are static-ish escape strings
applied around the **already-rendered inner content**. The renderer does
not currently transform `inner`. We therefore cannot escape `]` from inside
the open/close pair — the `]` is in `inner`, not in the wrap.

There are two viable shapes:

1. **Add a new `BlockTagAction` variant** (e.g.
   `BlockTagAction::WrapWithInnerTransform`) that carries an `fn(&str) ->
   Cow<str>` to apply to the inner content. Wires through the parser. This
   is **larger than the bug deserves** and crosses into the
   `Prose`/`Stylist` convergence work that the existing TODO at
   `prose.rs:371-374` already calls out as out of scope. **Reject.**
2. **Special-case the `<a>` markdown fallback inside `parse_tokens_inner`**
   (the renderer that consumes `BlockTagAction`) by detecting the markdown
   fallback wrap pattern (`open == "["` and `close.starts_with("](")` and
   `close.ends_with(")")`) and replacing `]` with `\]` in `inner` only for
   that path. Localized; one site.

We adopt **option 2** because it keeps the change confined to the same
function area (`block_tag_to_escape` + the immediate consumer) and does not
introduce a new public-ish enum variant. The exact implementation detail
(matching the wrap by structural fingerprint vs. introducing a private
marker) is the executor's call; the simpler structural fingerprint is
acceptable because there is exactly one site emitting `Cow::Borrowed("[")`
+ `Cow::Owned("](…)")`.

> **Alternative if structural fingerprinting is judged too fragile:** add
> a tiny private enum tag inside `BlockTagAction::Wrap` (e.g.
> `escape_inner_close_bracket: bool` field, defaulting to false) and set
> it `true` only at the markdown-fallback emit site. This is
> module-private, costs one bool field, and removes any reliance on the
> renderer recognising the wrap by content shape.

### Tests added

Add a single targeted unit test in `prose.rs`'s `mod tests`:

```rust
#[test]
fn link_markdown_fallback_escapes_bracket_in_description() {
    // When OSC8 is unsupported and the description contains a literal
    // `]`, the markdown fallback must escape it so downstream
    // CommonMark parsers do not mis-resolve the link.
    let term = Terminal::new_unsupported(); // or whichever helper yields
                                            // osc_link_support = false
    let rendered = Prose::new(r#"<a href="https://example.com">array[0]</a>"#)
        .render(Some(&term));
    assert!(
        rendered.contains(r"[array[0\]](https://example.com)"),
        "expected escaped `\\]` in markdown-fallback description; got:\n{rendered}",
    );
}
```

The exact `Terminal` constructor used should match whichever helper the
existing `apple_terminal_link_falls_back_to_markdown` test uses for its
`osc_link_support = false` profile — keeping the spoofing convention
identical avoids a second source of truth.

### Acceptance criteria

- Rendering `<a href="…">array[0]</a>` against a terminal with
  `osc_link_support = false` produces output containing `array[0\]`
  (escaped) rather than bare `array[0]` inside the markdown fallback's
  `[` `](…)` brackets.
- All existing prose tests continue to pass — the `[click here](url)`
  fixture (no `]` in description) renders byte-identically to its
  pre-change output.
- The Level-1 PTY test
  `apple_terminal_link_falls_back_to_markdown` is unchanged and still
  passes.

### Verification commands

```sh
cargo build -p biscuit-terminal
cargo nextest run -p biscuit-terminal --lib
cargo nextest run -p biscuit-terminal --test level1_apple_terminal_prose
cargo fmt --check
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness --all-targets -- -D warnings
```

### Commit message

```
fix(biscuit-terminal): escape `]` in <a> markdown fallback description

When osc_link_support is false, prose's <a> tag emits a CommonMark-style
[description](url) fallback. A literal `]` in the description text
previously broke downstream markdown parsers (e.g. `array[0]` rendered
as `[array[0]](url)` and parsed as `[array[`/`0]](url)`).

Escapes `]` inside the markdown-fallback description only. The OSC8
path, all SGR escapes, and the existing `[click here](https://example.com)`
fixture are unaffected.
```

---

## Phase 5 — Final cross-package verification

**Purpose:** prove that Phases 1-3 (and Phase 4 if included) compose
without regressions across the three packages they touch. This is the
single command sweep that must pass before the review-4 follow-up branch
is considered ready.

### Verification commands

Run each, in order, from the worktree root. Each must exit zero before the
next is run.

```sh
# 1. Targeted formatting check (single root rustfmt.toml).
cargo fmt --check

# 2. Build all three affected packages.
cargo build -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness

# 3. Lint sweep across the entire feature area as one invocation.
cargo clippy \
    -p biscuit-terminal \
    -p biscuit-terminal-cli \
    -p biscuit-test-harness \
    --all-targets \
    -- -D warnings

# 4. Library tests (prose + Level-1 PTY suite).
cargo nextest run -p biscuit-terminal

# 5. Harness unit tests (preserve_capabilities builder + applescript escape).
cargo nextest run -p biscuit-test-harness

# 6. CLI integration tests, including the Level-2 Apple Terminal suite.
cargo nextest run -p biscuit-terminal-cli
```

### Acceptance criteria

- All six commands above exit zero.
- The Level-2 suite either runs and passes (on macOS, outside CI) or skips
  cleanly via `skip_with_reason` (off-macOS or `CI=1`). No `#[ignore]`
  markers are introduced or removed by this plan.
- The combined diff across Phases 1-3 (and Phase 4 if included) introduces
  no new clippy warnings, no new `unsafe` blocks, no new public API
  surface beyond `AppleTerminalHarness::preserve_capabilities`, and no new
  workspace dependencies.

### Manual smoke test (recommended on macOS)

Where a developer macOS host with Terminal.app is available, run the
Level-2 suite end-to-end before merge to confirm review-4's verified
behaviour still holds after the migration off `disable_color_forcing`:

```sh
cargo nextest run -p biscuit-terminal-cli --test level2_apple_terminal_prose
```

Expected: all three Level-2 tests pass in roughly the same wall-clock as
review-4 reported (~30 s combined).

---

## Out-of-scope items (explicit non-goals)

The following are deliberately **not** addressed by this plan:

- **Curly / dotted / dashed underline capability awareness**
  (`prose.rs:375-381` TODO). Per review-4 §Recommendations 4, this is
  post-merge / future work. Phase 3's helper is shaped so it can absorb
  these variants when the work happens, but the work itself is not in this
  plan.
- **Refactoring `Prose` onto the `Style` / `Stylist` system**
  (`prose.rs:371-374` TODO). Explicitly out of scope of the 2026-05-02
  feature; the existing TODO already records this.
- **AC-5 lifecycle "Don't close window" preference**. Review-4 §Findings
  "Info — Lifecycle cleanup is best-effort against the 'Don't close
  window' Terminal preference" closes with "no action required". The
  current diagnostic warning + best-effort manual close is the correct
  shape; we do not change it.

---

## Summary

Five phases (four implementation + one verification). Phase 4 is optional
and isolated. Phases 1-3 are mandatory; Phase 5 is the final gate. The
plan ships the feature with all review-4 actionables addressed and leaves
the curly/dotted/dashed work cleanly stubbed for the post-merge follow-up.
