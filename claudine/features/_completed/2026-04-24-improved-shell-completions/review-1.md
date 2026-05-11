---
ready: false
---

# Review 1

## Findings

### 1. Setter-value completion returns non-Markdown files

`setter_value::gather_value_candidates` walks the `docs/`, `features/`, `fixes/`, and `reviews/` scopes and accepts every non-directory entry without an extension gate ([setter_value.rs](../../cli/src/completion/setter_value.rs:144)). The spec requires this path to fuzzy-search "any Markdown file" only ([spec.md](./spec.md:102)), and the design repeats that only `.md` / `.markdown` files should be surfaced. Today a `docs/spec.txt`, `features/plan.yaml`, or generated binary-like file can be returned as `spec='docs/spec.txt'` whenever its basename matches.

This is user-visible and production-blocking because setter values are inserted as quoted file references and look valid even though they are outside the declared contract.

Recommended fix: add a case-insensitive `.md` / `.markdown` extension gate before matching, and add unit plus integration coverage proving `.txt`, `.yaml`, extensionless, and uppercase Markdown behavior.

### 2. Directory suggestions are scoped to high-profile roots instead of the repo/CWD

For uncommitted composition partials, `gather_empty_or_word` only iterates `resolve_compose_scopes(...).iter_scopes()` ([composition.rs](../../cli/src/completion/composition.rs:135)). Directory candidates are then rendered only if they appear under those high-profile roots ([composition.rs](../../cli/src/completion/composition.rs:174)). The spec says directory matches after a short typed prefix are searched "within the current repo (or CWD if not in a repo)" ([spec.md](./spec.md:49)), and for longer prefixes users should see high-profile prompt files plus all matching directories ([spec.md](./spec.md:53)).

The result is that `claudine compose cla<TAB>` will not offer a repo directory such as `claudine/` unless it happens to live below `prompts/`, `.claudine/prompts/`, `docs/`, or a skill scope. That breaks the designed drill-down flow for selecting arbitrary directories before switching to committed-directory completion.

Recommended fix: add a separate repo/CWD directory walk for directory candidates, independent of high-profile file scopes. Keep file discovery in the curated roots, but render directory candidates relative to the repo root or CWD. Add integration tests for directories outside `prompts/`, including short-prefix prefix matching and long-prefix fuzzy matching.

### 3. One- and two-character directory completion is missing

The implementation only allows directories when `PartialLen::Long`, i.e. at three or more characters ([fuzzy.rs](../../cli/src/completion/fuzzy.rs:134)). The integration tests explicitly lock in "short prefix matches filenames no dirs" ([completion_compose.rs](../../cli/tests/completion_compose.rs:193)). The spec, however, requires directory matches after the caller has typed 1, 2, or 3 characters, with starting-substring matching at that stage ([spec.md](./spec.md:49)).

This is a direct spec gap. It is especially noticeable for common short directory names such as `docs/`, `fixes/`, `src/`, or package-area names where the user naturally types one or two characters before tabbing.

Recommended fix: split directory matching from file matching. For 1-3 characters, include repo/CWD directory candidates with case-insensitive prefix matching; for more than three characters, switch directory matching to fuzzy. Update the current tests that assert no short-prefix directories.

### 4. `@` magic paths do not support typed path-shaped magic prefixes

The spec examples use path-shaped magic inputs such as `@prompts/plan.md` and expect them to resolve to concrete insertions like `prompts/plan.md` ([spec.md](./spec.md:43)). The implementation strips only the leading `@`, then compares the full remaining string against each candidate basename/stem ([composition.rs](../../cli/src/completion/composition.rs:278)). A query like `@prompts/plan` is matched against `plan`, so it will not match.

This makes the documented magic-path form fail while the tested abbreviation-only form (`@plan`) passes.

Recommended fix: classify magic partials that contain `/` similarly to `PartialPath`: use the path prefix to constrain the scope-relative match and the last segment for basename matching, then render the same resolved path. Add integration tests for `@prompts/plan`, `@.claudine/prompts/plan`, and user-global `@prompts/plan` fallback.

### 5. Magic-path priority for inline/sequence extras is lower than user-global prompts

`ScopeSet::iter_scopes()` orders `user_claudine` before `extras`, and inline/sequence add `docs/` and skill scopes as extras ([scopes.rs](../../cli/src/completion/scopes.rs:238)). Therefore an `inline-compose` or `sequence` magic query can rank `~/.claudine/prompts/...` before repo-local `docs/...` or repo-local skill files. The spec says project-specific prompts should take precedence over user-global ones ([spec.md](./spec.md:42)), and the design describes magic resolution as repo-local directories first, repo `.claudine` second, user-global last.

Recommended fix: either move repo-local extras ahead of `user_claudine` for magic resolution only, or create an explicit magic-scope iterator whose ordering is repo prompt/package/doc/skills, repo `.claudine`, then user-global. Add a regression where a user-global prompt and repo-local `docs/` candidate both match and the repo-local candidate sorts first.

### 6. Compose accepts oversized/unreadable Markdown candidates despite the size-guard design

The performance design says files larger than `MAX_FRONTMATTER_BYTES` should skip parsing and be dropped. `is_valid_compose` does the opposite: when `read_text_within_size_cap` returns `None`, it returns `true` ([frontmatter.rs](../../cli/src/completion/frontmatter.rs:67)). That means oversized Markdown files, unreadable files, and non-UTF-8 Markdown files are still surfaced in `compose`, even though they may contain `prompt:` frontmatter or be expensive/noisy candidates.

Recommended fix: make the size/read failure behavior consistent across modes by rejecting failures, or explicitly revise the design/docs if accepting unreadable compose files is intentional. Add tests for oversized compose files and non-UTF-8 Markdown.

## Test Coverage Gaps

- No setter-value integration test proves non-Markdown files are excluded.
- No composition integration test covers directory candidates outside high-profile roots.
- No integration test covers 1-2 character directory suggestions per the spec.
- No magic-path integration test uses the documented path-shaped `@prompts/...` form.
- No magic-path priority test covers repo-local extras versus user-global prompts.

## Production Readiness

Not ready for production. The feature has good module structure and a large amount of coverage, but the remaining gaps are not just polish: they affect visible completion output and contradict the specified discovery model.
