---
agent: claude
model: ""
ready: false
---

# Review: Apple Terminal Integration Tests & Prose Graceful Degradation

## Summary

Phases 1–3 (capability confirmation, Prose degradation, Level-1 PTY tests) are
implemented and green. Phase 4 (the `AppleTerminalHarness` module) was written
but is **not wired into the crate**, and Phases 5 (Level-2 Terminal.app tests)
and the `justfile` integration of Phase 5 are **not implemented at all**.

As a result, three of the six acceptance criteria (AC-1 / AC-2 at Level-2, plus
AC-5 and AC-6) have **no end-to-end verification** against the real Terminal.app
display path. The graceful-degradation logic in `Prose` itself looks correct and
is well-tested at Level 1.

## Verification level per acceptance criterion

| AC | Requirement | Strongest existing test | Required level | Status |
|----|-------------|------------------------|----------------|--------|
| AC-1 | OSC8 → markdown fallback | Level-1 unit + Level-1 PTY | Level-2 (per spec §"Tier 2 — AppleScript Harness") | **Gap** |
| AC-2 | Double → straight underline | Level-1 unit + Level-1 PTY | Level-2 (per spec §"Tier 2") | **Gap** |
| AC-3 | No underline escapes when neither supported | Level-1 unit + Level-1 PTY (env override) | Level 1 ceiling (Apple Terminal supports straight) | OK |
| AC-4 | Level-1 PTY w/ `TERM_PROGRAM=Apple_Terminal` | Level-1 PTY (`level1_apple_terminal_prose.rs`) | Level 1 | OK |
| AC-5 | Level-2 AppleScript harness lifecycle | None — module unwired, no consumer test | Level 2 | **Not implemented** |
| AC-6 | Skip on CI / Terminal.app unavailable | Unit test exists but is never compiled | Level 1 | **Not verifiable** |

## Findings

### High severity

#### H1 — `AppleTerminalHarness` module is not wired into `biscuit-test-harness`

`biscuit-test-harness/src/apple_terminal.rs` is **untracked** (`git status`
shows it as the only new file) and `biscuit-test-harness/src/lib.rs` does
**not** declare `pub mod apple_terminal;`:

```text
biscuit-test-harness/src/lib.rs:28-31
pub mod cliclick;
pub mod kitty;
pub mod tmux;
pub mod wezterm;
```

Consequences:

- The module's `cargo` build has never run; only the workspace's *other* code
  compiles. `cargo test -p biscuit-test-harness` runs 10 tests, none from
  `apple_terminal::tests`.
- The 8 unit tests inside `apple_terminal.rs` (escape, shell-quote, CI gate,
  off-macOS gate) are dead code.
- No downstream crate can `use biscuit_test_harness::apple_terminal::AppleTerminalHarness`.

**Fix:** add `pub mod apple_terminal;` to `biscuit-test-harness/src/lib.rs` and
`git add` the file. Re-run `cargo test -p biscuit-test-harness` and confirm the
new tests are listed in the run output.

#### H2 — Phase 5 (Level-2 Terminal.app tests) is entirely missing

Plan §Phase 5 calls for `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`
with three tests:

- `level2_apple_terminal_link_fallback_visible`
- `level2_apple_terminal_double_underline_plain_text_visible`
- `level2_apple_terminal_harness_lifecycle`

The file does not exist. Without it:

- AC-1 has no Level-2 verification — the spec explicitly requires "Tier 2 —
  AppleScript Harness (Real Display Path)" with content-match assertions
  (`spec.md:98-114, 124-138`).
- AC-2 has no Level-2 verification — same source.
- AC-5 has no harness-lifecycle assertion at all.
- AC-6 has no observable skip behaviour anywhere.

The fixtures are already specified in `spec.md`; the work is to translate them
into `cli/tests/level2_apple_terminal_prose.rs` using
`AppleTerminalHarness::available()` + `skip_with_reason(...)` + the shared
`common::send_bt_command` helper, with `#[serial_test::serial(level2_terminal)]`
to match the existing convention used in `level2_image.rs`.

#### H3 — `justfile test-l2` does not include the new test target

`biscuit-terminal/justfile:60` currently lists exactly four `--test` targets:

```text
@cargo test -p biscuit-terminal-cli \
  --test level2_cursor_and_hygiene \
  --test level2_diagrams \
  --test level2_image \
  --test level2_prose_styling {{ args }}
```

Per Plan §Phase 5 step 7, `level2_apple_terminal_prose` must be appended.
Without this, `just test-l2` will not exercise the Apple Terminal Level-2 suite
even after H2 is fixed.

#### H4 — Level-2 coverage is missing for both user-observable rendering requirements

Per the test-rigor rules at the top of this review, AC-1 ("`<a>` renders as
`[desc](url)` instead of OSC8") and AC-2 ("double-underline renders as straight
underline") are user-observable rendering requirements. Their strongest existing
verification is Level 1 (we generate / decode the bytes ourselves), which
*cannot* catch a bug like "Terminal.app actually does interpret OSC8 in some
configurations" or "Apple Terminal renders `\x1b[4:2m` as visible garbage". Both
are exactly the regressions Tier 2 is meant to detect.

This is the consequence of H2 + H3, but is called out separately so it is not
discounted as "merely a missing test file" — it is a missing verification level
for two production-ready claims.

### Medium severity

#### M1 — Atomic-token form `{{double-underline}}` is not capability-aware

`ATOMIC_TOKEN_TABLE` (`biscuit-terminal/lib/src/components/prose.rs:175`) maps
`double-underline` directly to the static escape `"\x1b[4:2m"`. The capability
check that protects `<double-underline>...</double-underline>` (block tag) at
`prose.rs:334-350` has no equivalent for the `{{double-underline}}` atomic-token
path at `prose.rs:1406-1408`.

Plan §Phase 1 step 4 explicitly acknowledged this seam ("treat atomic token
degradation as a follow-up unless the implementation can be made local without
refactoring the parser"), but neither the implementation nor the spec carries a
TODO or comment marking the gap. A user who writes `{{double-underline}}txt`
on Apple Terminal will still emit the offending sequence — the Level-2 capture
would not show it (Terminal.app strips SGR), but the bytes leak through.

**Fix options:**

1. Extend `ATOMIC_TOKEN_TABLE` lookup to be capability-aware by routing the
   atomic-token branch in `parse_tokens_inner` through a small
   `atomic_token_to_escape_with_term(token, term)` shim.
2. At minimum, add a `TODO` comment near `ATOMIC_TOKEN_TABLE` and a `#[ignore]`
   regression test that documents the current behaviour and links to the
   follow-up issue.

#### M2 — `discovery_probe` duplicates OSC8 detection policy

`biscuit-terminal/lib/examples/discovery_probe.rs:288-291`:

```rust
let osc_link_support = match force_osc8 {
    Some(v) => v,
    None => !matches!(app, TerminalApp::AppleTerminal | TerminalApp::Wast),
};
```

This is a second copy of the detection policy. The canonical
`detection::osc8_link_support()` uses an *allowlist* of supporting terminals;
the probe uses a *denylist* of unsupported terminals. They happen to agree for
`Apple_Terminal`, but for any unknown `TERM_PROGRAM` value they disagree (probe:
`true`, runtime: `false`). This is fragile and will drift the moment a new
terminal is added to one list but not the other.

The comment on `probe_prose` says this avoids the viuer cascade, but
`osc8_link_support()` does not call viuer — only `Terminal::new()`'s image
detection does. Calling `detection::osc8_link_support()` directly would
eliminate the duplicated policy without re-introducing the blocking probe.

#### M3 — `applescript_escape` does not document the bytes it cannot escape

`biscuit-test-harness/src/apple_terminal.rs:261-273` handles `\\`, `"`, `\n`,
`\t`. NUL (0x00), BEL (0x07), ESC (0x1B), Unicode line/paragraph separators
(U+2028 / U+2029), and CR (0x0D) all pass through unchanged. AppleScript treats
those as unterminated string errors at parse time. The spec fixtures don't
exercise them, but the harness should document the contract:

> Bytes outside printable UTF-8 + LF/HT are not escaped and may produce an
> AppleScript syntax error.

A debug-assertion or an `if !ch.is_ascii() && !is_safe_unicode(ch)` rejection
would also be acceptable.

#### M4 — `spawn_shell` waits for the prompt with a fixed 800 ms sleep

`biscuit-test-harness/src/apple_terminal.rs:190` uses
`std::thread::sleep(Duration::from_millis(800))` after spawning. Other harnesses
poll for a `$`/`#`/`%` trailing character via `wait_for_prompt` (defined in
`biscuit-test-harness/src/lib.rs:408`). On a slow host (CI loaner, low-power
Mac) the 800 ms ceiling is racy; on a fast host it wastes time. Recommend
calling the existing `wait_for_prompt(self)` (or a Terminal.app-specific
equivalent that polls `capture()`) instead of a fixed sleep.

#### M5 — `serial_test::serial(level2_terminal)` convention is not enforced anywhere

Plan §Phase 5 step 6 requires the new tests to use
`#[serial_test::serial(level2_terminal)]` so multiple Terminal.app windows do
not race. Because the test file is missing, the convention isn't established.
When H2 is fixed, the new tests **must** use the same group key as
`level2_image.rs` to share the same serial lock with image tests that also rely
on macOS GUI focus.

### Low severity

#### L1 — `prose.md` and `README.md` were not updated

Plan §Phase 6 step 5 calls for updates to
`biscuit-terminal/docs/components/prose.md` and `biscuit-terminal/README.md`
documenting the new fallback semantics. A `git log -- docs README.md` for the
Apple Terminal feature commits shows none. The TODO comment at
`prose.rs:319-322` is the only public documentation; users will not learn about
the markdown fallback or underline degradation from the README.

#### L2 — `PROBE_FORCE_OSC8` is documented but unused

`discovery_probe.rs:269` accepts `PROBE_FORCE_OSC8` but no Level-1 test
exercises it. AC-1's "OSC8 supported still emits OSC8" path is covered by a
unit test, not the PTY test, so the override is theoretically reachable but
practically unverified. Either add a positive-case PTY test using
`PROBE_FORCE_OSC8=true` or drop the override and document its absence.

#### L3 — `Drop::drop` swallows osascript errors silently

`AppleTerminalHarness::close_window()` ignores the exit status of the cleanup
osascript. Best-effort cleanup is correct; consider an `eprintln!("warning:
failed to close Terminal.app window {id}: {err}")` so a stuck window in CI is
diagnosable. Cosmetic.

### Ergonomics / performance

#### E1 — `<a>` markdown fallback allocates per-tag

`block_tag_to_escape` returns `format!("]({})", resolved_href)` and a one-byte
`"["` open string for every `<a>` tag on a fallback terminal. This is fine for
typical document rendering but is a measurable cost in tight loops. Either
return `(Cow<'static, str>, String)` or fold the open into the close so the
caller emits a single allocated suffix.

#### E2 — Block-tag suppression returns two `String::new()` allocations

When `<double-underline>` is suppressed (`prose.rs:348`), the function returns
`Some((String::new(), String::new()))`. Empty `String::new()` is cheap, but the
sentinel could be promoted to a `&'static str` constant or replaced by a
distinct enum variant `BlockTagAction::Suppress` so the parser doesn't have to
re-check `open.is_empty() && close.is_empty()` at line 1505.

#### E3 — `applescript_escape` is `O(n)` allocations for line/tab-heavy strings

Each `\n` or `\t` produces `" & linefeed & "` / `" & tab & "` via repeated
`push_str`. For typical fixtures this is fine, but a multi-line prose input
will reallocate. Pre-allocating with `String::with_capacity(s.len() * 2)` for
strings containing newlines, or batching the concatenation, would reduce
churn. Cosmetic.

## Strengths

- **Prose degradation logic is well-factored.** `block_tag_to_escape` keeps
  capability awareness local to `<a>` and `<double-underline>` and the
  parse_tokens_inner suppression path elegantly handles the empty-open/empty-close
  sentinel.
- **Level-1 PTY tests are robust.** The `drain` helper polls until the
  `---END---` marker AND a zero-byte read coincide, with a 3 s deadline; this
  avoids the fixed-sleep races that often plague PTY tests.
- **Probe overrides are well-documented.** The `PROBE_FORCE_*` env vars are
  documented in `discovery_probe.rs`'s module-level doc comment and the no-
  underline test exercises the override path.
- **`available()` skip semantics are clean.** Off-macOS, in-CI, and
  Terminal.app-missing all return false without shelling out unnecessarily.

## Recommendations (in priority order)

1. **Wire the harness module** — add `pub mod apple_terminal;` to
   `biscuit-test-harness/src/lib.rs` and `git add` the file. Verify
   `cargo test -p biscuit-test-harness` runs the 8 new unit tests.
2. **Implement Phase 5** — write `cli/tests/level2_apple_terminal_prose.rs`
   with the three tests from the plan, gated on
   `AppleTerminalHarness::available()` and serialized via
   `#[serial_test::serial(level2_terminal)]`.
3. **Update `justfile`** — add `--test level2_apple_terminal_prose` to the
   `test-l2` recipe.
4. **Replace probe's OSC8 policy with `osc8_link_support()`** to remove the
   duplicated detection logic.
5. **Decide on atomic-token degradation** — either fix `{{double-underline}}`
   to be capability-aware or add a TODO + ignored regression test pinning the
   current behaviour.
6. **Update `docs/components/prose.md` and `README.md`** with the new fallback
   semantics.
7. **Replace the fixed 800 ms sleep in `spawn_shell` with `wait_for_prompt`**.

## Verdict

**Not ready for production.**

Phases 1–3 are solid, but the Phase 4 module is unwired and Phase 5 is missing.
Two of the six acceptance criteria (AC-5, AC-6) have no observable verification
at any level, and two more (AC-1, AC-2) are at Level 1 where the spec explicitly
demands a Level-2 real-Terminal.app capture. After fixing H1–H4 and re-running
`just test-l2` on macOS, the feature should clear the bar.
