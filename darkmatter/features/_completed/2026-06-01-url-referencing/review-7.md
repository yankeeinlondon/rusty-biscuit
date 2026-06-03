---
ready: true
agent: codex
model: ""
---

# Review 7: URL Referencing

## Findings

### High: library remote-read configuration does not enable expression URL reads

The spec treats remote reads as a single feature surface covering both
transclusion and read-side expression functions (`frontmatter(url)`,
`file_exists(url)`, `markdown_title(url)`, ...). The public topic doc says
library callers configure remote reads with
`ComposeOptions::with_remote_read_config(...)`
([darkmatter/docs/topics/remote-url-references.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/topics/remote-url-references.md:35)).

In the implementation, `with_remote_read_config(...)` only stores policy and
freshness settings; it does not enable the remote runtime for expression
functions ([darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs:1076)).
The actual gate is still `allow_remote_transclusion`: eager expression URL
discovery is skipped unless that flag is true
([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:495)),
and expression resolution only attaches `remote_fetch` when
`allow_remote_transclusion` is true
([darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs:1024)).

That means a library caller following the documented API can set an allowed
host and still get `markdown_title() remote reads are not enabled` for a URL
expression. The CLI masks this by always calling
`with_allow_remote_transclusion(true)` after building the remote config
([darkmatter/cli/src/commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/commands.rs:696)),
and the relevant expression-only regression test also has to set the
transclusion flag even though block transclusion is disabled
([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:5646)).

Fix by separating the capability gate from transclusion naming, for example
`with_allow_remote_reads(true)`, or by having any non-default remote read config
explicitly enable the remote-fetch context while preserving deny-all policy as
the safety default. Add a Level 1 library regression test that uses only
`with_remote_read_config(RemoteReadConfig { allowed_hosts: ... })` and
`only(&[ComposeOperation::Interpolation])` to read `markdown_title("https://...")`
successfully, with no `with_allow_remote_transclusion(true)` call.

## Test Rigor

I ran the targeted remote suite:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
```

Result: pass. The run executed 103 remote-filtered `darkmatter` tests and 8
remote CLI integration tests. The `biscuit-file` remote-filtered targets had no
matching test names in this command, but their filtered binaries completed
successfully. One Level 2 terminal test whose name contains `remote` was also
selected and passed.

Level 1 is the appropriate verification level for URL classification, fetch
policy, remote transclusion, expression-function reads, cache freshness, CLI
flags, and the library API issue above. The spec does not require terminal
encoder behavior, key input, paste, IME, mouse, or modifier-key behavior, so I
do not see a Level 2 or Level 3 requirement gap for production readiness.

## Recommendation

Do not mark this production-ready until the public library remote-read API
matches the documented and specified behavior for expression-only URL reads.

## Resolution

### High: library remote-read configuration now enables expression URL reads

The read-side expression capability is unbundled from the transclusion flag.
A new `ComposeOptions::remote_reads_enabled()` returns true when
`allow_remote_transclusion` is set **or** the remote read config carries a
non-empty allowlist
([darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs:1093)).
Expression resolution now attaches `remote_fetch` whenever
`remote_reads_enabled()` is true
([darkmatter/lib/src/markdown/compose/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs:1024)),
and eager URL discovery is gated the same way, with directive (`::file`/`::code`)
discovery additionally requiring the explicit `allow_remote_transclusion` opt-in
([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:496)).
An empty allowlist with the flag unset keeps remote reads fully disabled, so the
deny-all default is preserved.

This follows the documented API: the topic doc already directs library callers
to configure remote reads with `with_remote_read_config(...)`. Block
transclusion keeps its own `with_allow_remote_transclusion(true)` gate because it
injects fetched bodies into the document; the topic doc now states this split
explicitly
([darkmatter/docs/topics/remote-url-references.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/topics/remote-url-references.md:35)).

The expression-only regression test
`interpolation_only_discovers_and_reads_remote_expression_url`
([darkmatter/lib/src/markdown/compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:5625))
now reads `markdown_title("https://...")` with `only(&[Interpolation])` and a
host-only `RemoteReadConfig`, **without** any `with_allow_remote_transclusion(true)`
call. It fails before the fix (URL left unexpanded) and passes after.

### Verification

The reviewer's exact command passes:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
```

`darkmatter` runs 103 remote tests, 0 failed; `darkmatter-cli` runs 8 remote CLI
tests, 0 failed; `biscuit-file` reports 0 failures; one Level 2 terminal test
selected by the `remote` filter passes. Clean `clippy` on `darkmatter`.
