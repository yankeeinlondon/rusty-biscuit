---
ready: false
agent: codex
model: ""
implemented: true
---

# Review: Context Variables and Expression Function Additions

## Findings

### High: `has_skill` / `has_local_skill` do not search the full local-scoped root set for recognized agents

The spec lists all local-scoped roots as `.claude/skills`, `.opencode/skill`, `.codex/skills`, and `.agents/skills`, with only unknown agents restricted to the generic `.agents/skills` and `.codex/skills` roots. The implementation narrows recognized agents to an agent-specific subset:

- Claude: `.claude/skills`, `.agents/skills`, `.codex/skills`
- OpenCode: `.opencode/skill`, `.agents/skills`, `.codex/skills`
- Codex: `.codex/skills`, `.agents/skills`

See `SkillRoots::local_roots` in `darkmatter/lib/src/markdown/compose/expression/functions.rs:137`. This means, for example, `has_skill("foo")` returns `false` for a local `.opencode/skill/foo` when `ctx.agent == "claude"`, even though `.opencode/skill` is one of the specified local-scoped roots. The existing tests only assert the current narrowed behavior for Claude and unknown agents (`functions.rs:3405`), so they lock in the under-search rather than proving the spec.

Required fix: search the complete local-scoped root list for recognized agents, while preserving the spec's unknown-agent generic fallback. Add Level 1 tests for each recognized agent proving all four local roots are searched by `has_skill` / `has_local_skill`, and that unknown agents remain limited to `.agents/skills` and `.codex/skills`.

Verification level: current strongest coverage is Level 1, but it covers only the narrowed behavior. Level 1 is the appropriate level for this filesystem lookup contract; the gap is missing/incorrect Level 1 coverage.

### High: HTTP(S) detection is case-sensitive, so valid URLs can be treated as local paths

`is_remote_url` only checks `raw.starts_with("http://") || raw.starts_with("https://")` (`darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:124`). URL schemes are case-insensitive, and the spec says HTTP(S) URL strings must be accepted by `link(target, desc)` and rejected by file-only helpers. A value like `HTTPS://example.com/doc.md` is not recognized as a URL. Consequences:

- File-shape helpers such as `basename`, `join`, and one-argument `link` can incorrectly treat uppercase-scheme URLs as local filesystem paths instead of returning the required HTTP(S) error (`functions.rs:1035`, `functions.rs:1381`).
- Two-argument `link("HTTPS://example.com/doc.md", "Doc")` can emit a local path destination instead of the supplied URL, because the URL parse branch is skipped (`functions.rs:1399`).

Required fix: parse the candidate URL, then check `scheme().eq_ignore_ascii_case("http") || scheme().eq_ignore_ascii_case("https")`, or otherwise normalize the scheme before classification. Add Level 1 tests for uppercase and mixed-case schemes across one file-only helper, `join`, one-argument `link`, and two-argument `link`.

Verification level: current strongest coverage is Level 1 and only covers lowercase `https://`. Level 1 is appropriate for this expression-function contract; the gap is incomplete Level 1 coverage.

## Test Rigor Assessment

The feature is mostly pure expression evaluation, deterministic rendering through `Prose`, and filesystem path/root discovery. Level 1 coverage is the right verification level for these requirements. I did not find requirements that need Level 2 real-terminal capture or Level 3 OS keyboard injection; `terminal(string)` returns a string from a deterministic non-interactive renderer rather than asserting live terminal behavior.

Existing tests cover many happy paths, arity errors, null propagation, descriptor parity, representative interpolation, and `when=` usage. The two findings above are still production blockers because their user-observable contracts are either implemented too narrowly or not tested at the required edge.

## Verification

- Source review of the spec, context capture, expression dispatch, path helpers, skill helpers, descriptor catalogs, docs, and regression tests.
- Attempted `cargo test -p darkmatter --lib markdown::compose::expression::functions::fn_phase5 --color=never`; it compiled successfully but matched `0` tests, so it did not verify behavior.
- Attempted a multi-filter Cargo test command for agent context tests; Cargo rejected the command because it accepts only one test filter before `--`.

## Production Readiness

Not ready for production until the two findings are fixed and covered by Level 1 tests.
