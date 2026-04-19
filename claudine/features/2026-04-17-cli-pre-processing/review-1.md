---
feature: 2026-04-17-cli-pre-processing
reviewer: Claude Opus 4.7 (1M context)
reviewed: 2026-04-17
ready: true
resolved: 2026-04-17
---

## Resolution notes (2026-04-17)

All fifteen items in this review were addressed. Key deliverables:

- **Gap 1 (HIGH):** Added Rule 4 to [`claudine/cli/src/argv.rs`](../../cli/src/argv.rs)
  — `hoist_composition_help` scans the argv between the composition
  subcommand and the first literal `--` for an exact `--help` or `-h`
  token and hoists it to position 1, so the root `cli.help` handler
  fires and the custom help screen renders. Rule 4 runs before Rule 3
  so `--help` is removed from the trailing setter region before the
  separator is inserted.
- **Gap 2 (HIGH):** `headline_compose_with_interleaved_flag_renders_help`
  in [`claudine/cli/tests/argv_normalization.rs`](../../cli/tests/argv_normalization.rs)
  now asserts `.success()`, the presence of the grouped help-screen
  content (`Compose a Markdown document`, `Composition`), and the
  absence of both the old clap tip and the secondary "expected at most
  one file reference" error. A simpler companion case
  (`headline_compose_with_trailing_help_renders_help`) locks in the
  `compose file.md --help` path.
- **Gap 3 (MEDIUM):** Rule 1 is now gated on composition subcommands.
  Wrapper argv like `claudine claude --gemini file.md` flows into the
  child CLI's argv unchanged. New regression tests
  (`rule_1_does_not_rewrite_provider_boolean_on_wrapper_subcommand`,
  `rule_1_does_not_rewrite_on_unknown_subcommand`,
  `rule_1_does_not_rewrite_on_hooks_subcommand`) lock this in.
- **Gap 4 (LOW):** `SharedComposeArgs` gained a doc-comment block
  explaining that the eight provider boolean fields are handled
  entirely by the argv normalizer and retained only as clap help
  entries.
- **Gap 5 (LOW):** `rule_3_skips_value_for_every_composition_flag_with_value`
  drives the full `COMPOSITION_FLAGS_WITH_VALUE` matrix through
  `normalize`.
- **Gap 6 (LOW):** `composition_flags_with_value_matches_clap_surface`
  iterates `ComposeArgs::augment_args(...)` and `SequenceArgs::augment_args(...)`
  to assert that every value-bearing clap flag is mirrored in
  `COMPOSITION_FLAGS_WITH_VALUE`.
- **Gap 7 (LOW):** `find_subcommand_handles_equals_form_after_composition_subcommand`
  locks the `--debug=LEVEL` equals form post-subcommand.
- **Gap 8 (LOW):** `passthrough_near_miss_provider_flag_surfaces_clap_unknown_error`
  exercises `claudine compose … --claud` end-to-end and asserts clap's
  native unknown-argument diagnostic.
- **Gaps 10, 11, 12, 13, 14, 15 (LOW):** argv-normalization.md pipeline
  diagram switched to mermaid and documents Rule 4; `as_utf8` gained a
  `///` doc; `PROVIDER_BOOLEAN_FLAGS` was replaced with a
  `Provider::cli_aliases`-derived lookup; `parse_cli_from` has a
  comment explaining the lenient-pass fallback safety; the test-only
  `normalize_with_completion` is now `#[cfg(test)]`; and the module
  docs call out Rule 4 and the composition-gated Rule 1.
- **Gap 9 (LOW):** skipped — the current while-loop form is simple,
  well-tested, and changing it to an iterator adapter would be churn
  without a clear readability win.

All 89 argv unit tests and all 8 `argv_normalization.rs` integration
tests pass; the `wrap_direct_argv`, `wrap_commands`, `sequence_cli`,
and `command_routing` regression suites still pass (95 + 2 +
composition tests green).
# Review 1 — CLI Argv Pre-Processing

## Summary

The implementation delivers the core of the feature well: a dedicated
`claudine/cli/src/argv.rs` module is wired as the single pre-clap seam in
`main.rs`, Rules 1 and 2 rewrite provider selectors into a canonical
`--provider <slug>` form, Rule 3 inserts a `--` separator on composition
subcommands, `SharedComposeArgs::explicit_provider()` is reduced to
reading `self.provider`, and the docs topic
`claudine/docs/topics/argv-normalization.md` is added and referenced
from `composition.md`. 76 unit tests and 6 integration tests pass; the
wrapper regression suites (`wrap_direct_argv`, `wrap_commands`,
`sequence_cli`, `command_routing`) still pass. Clippy is clean.

However, **the motivating acceptance criterion is not fully met**. The
spec's headline case — `claudine compose <file> --gemini name=Ken --help`
"exits with the compose help text" — still fails. The misleading clap
tip is gone (the narrow goal of the current integration test), but the
run now exits with a new, still-confusing Claudine error:

```text
Error: expected at most one file reference, but got multiple: <file>, --help
```

because after the inserted `--`, clap treats `--help` as a trailing
raw value, and the downstream positional parser
(`parse_composition_positionals`) misclassifies it as a second file
reference. The integration test `headline_compose_with_interleaved_flag_no_longer_trips_clap_help_tip`
passes only because it asserts string absence (`tip: to pass…`,
`unexpected argument '--help'`) — not the presence of help text or a
successful exit.

For this reason the feature is **not ready for production**. The gap
can be closed with a small additional rewrite pass (see
Recommendation 1 below) plus stricter acceptance tests.

## Gaps vs. spec

### 1. `--help` does not render help for composition subcommands [HIGH]

**Spec acceptance criterion** (`spec.md:201-202`):

> `claudine compose @file.md --gemini name=Ken --help` exits with the
> compose help text, not a clap "unexpected argument" error.

**Observed behavior** after normalization:

```sh
$ claudine compose smoke.md --gemini name=Ken --help
Error: expected at most one file reference, but got multiple: smoke.md, --help
```

and the simpler case is worse:

```sh
$ claudine compose smoke.md --help
error: unexpected argument '--help' found
  tip: to pass '--help' as a value, use '-- --help'
```

**Root causes:**

1. The root `Cli` struct sets `disable_help_flag = true` and declares
   its own `help: bool` *without* `global = true`
   (`claudine/cli/src/args.rs:31-49`). Subcommands therefore never
   inherit a `--help` handler.
2. After Rule 3 inserts `--` before the setter, `--help` lands in the
   trailing `args` vector (positional raw values) and is passed to
   `parse_composition_positionals`, which has no notion of help and
   classifies `--help` as a second file reference.

**Recommended fix**: either
- **(a)** extend the normalizer with a fourth rule that, on composition
  subcommands, detects `-h` / `--help` *anywhere in the pre-`--`
  argv* and moves it to **position 1** (before the subcommand) so the
  root `cli.help` handler fires and the custom help screen renders, or
- **(b)** pre-scan the argv inside `main.rs` (mirroring the existing
  `--plain` pre-scan at `main.rs:99`) and short-circuit to
  `commands::help::run()` when a composition subcommand argv contains
  `--help` / `-h`, or
- **(c)** teach `parse_composition_positionals` to recognize `-h` /
  `--help` tokens and render subcommand-specific help.

Option (a) or (b) is the cheapest and preserves "normalizer as the
single seam". The follow-up should ship with tests that assert the
help *content* is rendered (`combined.contains("Compose a Markdown
document")` or similar), not just absence of error strings.

### 2. Integration test is too narrow [HIGH]

`claudine/cli/tests/argv_normalization.rs:42-73` only asserts that
specific error substrings are absent:

```rust
assert!(!plain.contains("tip: to pass '--help' as a value"), …);
assert!(!plain.contains("unexpected argument '--help'"), …);
```

It never asserts exit status, nor that the help screen renders.
Consequently the test passes while the user-visible UX remains broken
(now with a different, still-confusing error). Tighten to one of:

- `assert.success()` plus `stdout.contains("<known help-text marker>")`,
- or at minimum `!plain.contains("expected at most one file reference")`
  so the current regression fails the suite.

### 3. Wrapper-safety claim is not fully enforced for Rule 1 [MEDIUM]

The design doc (`tech-design.md:328-334`) asserts:

> 3. provider boolean rewrites only trigger on Claudine-owned exact long flags

But Rule 1 is **not** gated on subcommand — it rewrites any exact match
of `--claude`, `--codex`, `--gemini`, `--goose`, `--kimi`, `--opencode`,
`--qwen`, `--roo` *anywhere before the first `--`*, including in
wrapper-subcommand argv. For example
`claudine claude --codex file.md` is rewritten to
`claudine claude --provider codex file.md`, and the two rewritten
tokens then flow into the wrapper's `passthrough` (via
`ignore_errors(true)`). The child Claude CLI receives
`--provider codex` instead of the original `--codex`. For Claude this
is probably inert, but for other child CLIs it could silently change
semantics.

**Recommendation:** either

- gate Rule 1 on composition subcommands the same way Rule 3 is
  (safest — matches the spec's "pipeline placement" diagram where Rule
  1 is applied only to composition surfaces), or
- document this explicitly in `argv-normalization.md` §"Pass-through
  guarantees" under a new "Known rewrite on wrapper argv" bullet and
  add a regression test confirming the current behavior.

Today there is no test for wrapper + provider-boolean interaction.
The existing `normalize_leaves_wrapper_passthrough_untouched`
(`argv.rs:829-834`) only covers post-`--` passthrough; add a companion
that exercises pre-`--` wrapper argv containing a provider boolean.

### 4. Dead user-facing flags [LOW — spec-acknowledged]

The eight boolean fields (`claude`, `codex`, `gemini`, `goose`,
`kimicode`, `opencode`, `qwen`, `roo`) on `SharedComposeArgs`
(`compose.rs:31-60`) are now purely user-visible sugar: the normalizer
rewrites them away before clap, and `explicit_provider()`
(`compose.rs:157-159`) reads only `self.provider`. They remain on
the struct as clap-declared help entries but their parsed values are
never read.

The spec explicitly defers retiring these as a follow-up
(`spec.md:48`, `spec.md:211`), so this is not a blocker, but worth
flagging: (a) the `compose_provider` mutual-exclusion group is no
longer load-bearing; (b) `--kimi` and `--qwen` appear twice in
user-facing help (once as the boolean, once as the slug accepted by
`--provider`); (c) a future contributor could reasonably assume the
bool fields are still read. A one-line doc comment on
`SharedComposeArgs` noting "boolean flags are handled entirely by the
argv normalizer; the fields are retained as clap help entries only"
would prevent confusion.

## Coverage gaps

### 5. Rule 3 misses value-bearing flags in test matrix [LOW]

The Rule 3 unit suite covers `--model gpt-4` (value-bearing, triggers
skip-2), `-m gpt-4` (short form), and `--provider=gemini` (equals form).
It does **not** cover:

- `--include FOO` between positional and setter (Vec<String>, value-bearing)
- `--exclude claude` between positional and setter (Vec<Provider>, value-bearing)
- `--timeout 30` (u64 value)
- `--operation ship` / `--op ship` (aliased flag)
- `--set '{"k":"v"}'` (JSON value, tricky because it contains quotes)
- `--use id1,id2` (comma-delimited Vec)
- `--asp` / `--rsp` (short aliases)

All of these are in `COMPOSITION_FLAGS_WITH_VALUE`, so the
skip-next-token logic already handles them, but without tests a future
refactor of the flag list can silently break them. Add one
parametric test per flag in the list (or one table-driven test over
all flags in `COMPOSITION_FLAGS_WITH_VALUE`) to lock the contract.

### 6. No drift test between `COMPOSITION_FLAGS_WITH_VALUE` and clap surface [LOW]

The design doc's primary risk (`tech-design.md:407-413`):

> Risk: the hard-coded list of composition flags that consume values can
> drift when new flags are added.

Mitigation listed is "call out the maintenance rule in module docs +
add regression tests whenever composition flags change." The module
doc does mention the maintenance rule (`argv.rs:56-59`), but there is
no automated check. A cheap defense-in-depth: iterate
`<ComposeArgs as CommandFactory>::command()` in a test, collect all
`Arg`s that have `num_args > 0` and a long name, and assert that every
value-bearing clap arg has a matching entry in
`COMPOSITION_FLAGS_WITH_VALUE`. This makes drift a test failure
instead of a latent bug.

### 7. No unit test for `find_subcommand` with `--debug=LEVEL` after subcommand [LOW]

`find_subcommand_handles_equals_form_for_global_value_flag`
(`argv.rs:473-480`) tests the equals form *before* the subcommand.
No test covers `--debug=trace` appearing *after* a composition
subcommand (which is legal, since `debug` is `global = true`). Rule 3
handles this correctly by construction (the `=` form is a single
token), but lock it in.

### 8. No test for the spec's explicit "unknown argument" acceptance [LOW]

`spec.md:82` says:

> `--claud` is not rewritten. clap will surface it as an unknown
> argument, which is the desired behavior.

The unit test `rule_1_does_not_fuzzy_match_near_miss_flags`
(`argv.rs:709-712`) confirms `--claud` survives normalization
unchanged, but there is no integration test confirming the end-to-end
clap error message. One `assert_cmd` case asserting the exit code is
non-zero and stderr contains `unrecognized argument '--claud'` (or
the current clap phrasing) would close the loop.

## Ergonomics & code quality

### 9. Normalizer state machine uses `while index < len` loops [LOW]

The main loop in `normalize_with_completion` (`argv.rs:119-176`) and
Rule 3 in `apply_composition_separator` (`argv.rs:199-259`) both use
manual index-tracking with `index += 1` / `index += 2`. This is
correct but error-prone — any future edit that forgets to advance the
cursor loops infinitely.

Consider lifting each into a small iterator/adapter pattern, e.g.:

```rust
let mut tokens = raw.into_iter().peekable();
while let Some(token) = tokens.next() {
    match classify(&token) {
        Classified::ProviderBool(p) => {
            out.push("--provider".into());
            out.push(p.as_slug().into());
        }
        Classified::ProviderFlag => {
            out.push(token);
            if let Some(next) = tokens.next_if(is_fuzzy_candidate) {
                out.push(normalize_provider_value(next));
            }
        }
        …
    }
}
```

Not urgent — the existing form is well-tested and readable — but worth
noting for the next maintainer.

### 10. `as_utf8` helper is one line and called once per token [LOW]

`as_utf8(token: &OsString) -> Option<&str>` (`argv.rs:329-331`) is a
1-line wrapper over `token.to_str()`. It's called often enough that
the abstraction has value (centralizes "we deliberately skip non-UTF-8
tokens"), but a `///` doc comment stating that intent — rather than
leaving the reader to infer it from module docs — would help.

### 11. `PROVIDER_BOOLEAN_FLAGS` could lean on `Provider::cli_aliases` [LOW]

`PROVIDER_BOOLEAN_FLAGS` in `argv.rs:84-93` is a hand-maintained table
that duplicates knowledge already encoded in
`Provider::cli_aliases()` (`lib/src/events/provider.rs:287-298`). For
example, `Provider::KimiCode.cli_aliases()` already returns
`["kimi", "kimicode", "kimi_code", "kimi-code"]` and the Rule 1 flag
is `--kimi`. Today both lists agree; a future provider added with new
aliases would require updating both. Consider deriving the table
from `Provider::cli_aliases()` at compile time (via `const` iteration
or a small helper), or at runtime once with `OnceLock`.

Not blocking; flagged for the follow-up that retires the boolean flags.

### 12. `main.rs:parse_cli_from` silently falls back twice [LOW]

`parse_cli_from` (`main.rs:65-89`) has two `unwrap_or_else` /
`Err(_) =>` fallbacks that call `Cli::parse_from(...)` on the
normalized argv when the lenient pass fails. In both cases the second
parse re-does the work clap already rejected, which (a) double-prints
errors if both fail and (b) swallows the lenient pass's diagnostic.
The current behavior is defensive and matches the prior shape, but a
comment explaining **why** the fallback is safe (or a `tracing::debug!`
on the fall-through) would help during future debugging.

### 13. `normalize_with_completion` visibility [LOW]

`normalize_with_completion` is test-only but declared as `fn` (module
private) rather than `#[cfg(test)] fn`. It compiles into the release
binary as dead code. Either gate it behind `#[cfg(test)]` or mark it
`#[doc(hidden)]` and reference from tests via a pub-hidden path.

## Performance

No performance concerns. All operations are O(argv) with small
constants, and argv is typically under 20 tokens.

## Documentation

### 14. `argv-normalization.md` is solid but one example is inconsistent [LOW]

In `argv-normalization.md:144` the "before" for Rule 3 shows
`claudine sequence file.md --gemini k=v --help` but the matching
"after" on line 152 shows
`claudine sequence file.md --provider gemini -- k=v --help`. Good.

The `--plain` example on line 145/153 is correct.

However the pipeline ASCII art (`argv-normalization.md:23-30`) drops
the leading `s` on the downstream arrow (`─────────▶` vs
`──────────▶`) and the box widths don't align — a cosmetic issue.
Consider regenerating as a mermaid diagram to match other topic docs.

### 15. Main.rs comment refers to removed `parse_cli` [LOW]

`main.rs:76-77`:

```rust
// Build a command tree where wrapper subcommands ignore unknown args.
let mut cmd = <Cli as CommandFactory>::command();
```

Good. But the function doc comment (`main.rs:57-64`) still says
"Pass 1: build the `Command` …". The actual implementation now does
this inside `parse_cli_from` — the comment is accurate, but the
cross-reference to "wrapper subcommands" in the lenient parse is
subtle. Consider adding: "Pass 2 is only invoked via
`Cli::parse_from` on the fallback paths if `from_arg_matches` fails."

## What's Good

1. The spec is implemented at the architectural level the author
   intended: `argv::normalize` is the single pre-clap seam, library
   code never touches argv, and the three rewrite rules are
   syntactically pure and testable in isolation.
2. Pass-through tests are comprehensive for what they cover — every
   rule has at least one matching "untouched argv" case, honoring the
   "pass-through tests required" contract in the module docs.
3. `find_subcommand` is a real improvement over the prior
   `raw_args.get(1)` heuristic (flagged in `tech-design.md:148-151`)
   and now correctly skips root globals like `--plain`, `--debug
   [LEVEL]`, and short boolean flags.
4. The `COMPLETE` env-var guard is correctly a no-op and has a clean
   dependency-injected test via `normalize_with_completion(..., true)`.
5. `explicit_provider()` simplification is done — the argv normalizer
   really is the sole translation point, modulo the dead boolean
   fields (see item 4).
6. Phase 5 smoke cases I re-ran manually work:
   - `--kimi` → `kimi_code` end-to-end ✓
   - `--provider=gem` → `gemini` end-to-end (with equals form) ✓
   - `--plain compose … --gemini name=Ken` (no `--help`) ✓
   - `compose file.md key=val --provider claude --dry-run` ✓

## Suggested Follow-up Order

1. **Blocker for production**: close gap 1 (render help for
   composition subcommands). Either add a Rule 4 in the normalizer
   that hoists `-h`/`--help` before the subcommand token, or add a
   pre-scan in `main.rs` mirroring the `--plain` pre-scan. Ship with
   tightened integration tests (gap 2) that assert *help text renders*,
   not just that old error strings are gone.
2. **High-value hardening**: close gap 3 (wrapper safety) by either
   gating Rule 1 on composition subcommands or adding a regression
   test + doc note locking in the current "Rule 1 applies to wrappers
   too" behavior.
3. **Coverage**: add the table-driven test for gap 5 and the drift
   test for gap 6; these are cheap and lock the flag surface.
4. **Cleanup**: items 4, 11, and 13 can fold into the follow-up that
   retires the boolean provider flags.

Once (1) and (2) are addressed, this feature is ready for production.
