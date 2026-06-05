---
ready: false
agent: codex
model: ""
---

# Review 8: URL Referencing

## Findings

### High: eager expression discovery can fetch URLs from non-expressions

The spec scopes eager remote discovery to URL-typed expression-function
arguments, not arbitrary prose or code examples
([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-06-01-url-referencing/spec.md:115)).
The implementation currently calls `discover_remote_urls_from_expressions` on
the whole Markdown body before interpolation runs
([mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:514)).
That function then scans every line for substrings like `frontmatter(`,
`file_exists(`, and `markdown_title(` without parsing `{{ ... }}` expression
boundaries or checking identifier boundaries
([remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:256),
[remote.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/remote.rs:367)).

With `--allow-host example.com`, ordinary text such as
`` `frontmatter("https://example.com/doc.md")` `` or
`not_frontmatter("https://example.com/doc.md")` can be registered and fetched
even though the evaluator would not execute that expression. This is a
network-egress and privacy bug: an allowlisted host should only be contacted for
actual compose inputs.

Fix by driving discovery from the interpolation parser/evaluator, or at minimum
extract only parsed `{{ ... }}` expressions and require exact function
identifiers. Add Level 1 tests that prove no request is made for prose, fenced
code, inline code, and longer identifiers containing a supported function name.

### Medium: remote `prologue` / `epilogue` references are accepted but never registered

The compose pipeline has a frontmatter transclusion path for `prologue` and
`epilogue`, and `prepare_frontmatter_reference` accepts URL references when
remote transclusion is enabled
([mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1649),
[mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1713)).
Unlike directive-based remote transclusion, this path does not call
`remote_fetch.register_nested(url.clone())` before pushing
`PreparedTransclusion::RemoteFile`. The directive path does register the URL
for exactly this reason
([mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1552)).

At resolution time, `PreparedTransclusion::RemoteFile` only waits on an
existing slot; if the URL was not registered, it fails with
`URL was not registered for fetching`
([mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1902)).
So an allowed remote `prologue: https://.../intro.md` or `epilogue:
https://.../outro.md` is routed into remote transclusion but cannot succeed.

Fix by either registering the URL in `prepare_frontmatter_reference` or by
explicitly rejecting remote frontmatter transclusions as out of scope before
they become `PreparedTransclusion::RemoteFile`. The former matches the existing
local frontmatter transclusion behavior. Add Level 1 library and CLI tests for
allowed and denied remote `prologue` / `epilogue` references.

### Low: remote-read field docs are stale

`ComposeOptions::remote_read_config` still says the field is only a future
configuration surface and will be wired by a later phase
([types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs:510)).
The code is now wired into eager fetch, expression resolution, and persistent
cache. This is comment drift; the code is the source of truth, so update or
delete the stale phase wording.

## Test Rigor

I ran:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
```

Result: pass. `darkmatter` ran 103 remote-filtered tests, `darkmatter-cli` ran 8
remote CLI tests, and one Level 2 terminal style test selected by the `remote`
filter passed. `biscuit-file` had no matching remote-named tests in this
filtered command, though its filtered test binaries completed successfully.

Level 1 is the appropriate verification level for URL classification, host
policy, cache freshness, remote transclusion, expression-function reads, CLI
flags, and the two gaps above. The spec does not assert terminal input,
modifier-key, paste, IME, mouse, or terminal-rendering behavior for this
feature, so I do not see a Level 2 or Level 3 mismatch.

## Recommendation

Do not mark this feature production-ready yet. The false-positive eager scanner
can cause unintended allowed-host network egress, and the accepted remote
frontmatter-transclusion path is broken at point-of-use.
