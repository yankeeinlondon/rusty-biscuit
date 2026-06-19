---
ready: false
agent: codex
model: ""
---

# Review: Compose Pipeline v2 — Iteration 6

## Verdict

Not ready for production.

Iteration 6 fixes the iteration-5 recursive graph-reuse gap for local Markdown
transclusions: a local child compose now receives its own preflight subgraph,
and there is focused L1 coverage for root → child → grandchild reuse. The
approval/execution split, cache behavior, dynamic command-shape rejection, and
CLI `--shell` listing are covered at the appropriate verification level.

I found one remaining design-completion gap in the same graph-reuse area. It is
not a shell-safety or output-correctness blocker, but the implementation still
falls short of the design's reusable graph-metadata contract for remote recursive
transclusions.

## Findings

### Medium: recursive preflight graph reuse still drops remote child subgraphs

The design says the pre-flight walk caches resolved child paths/URLs and graph
metadata so final graph composition can reuse the structure instead of
re-discovering it:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:121`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:123`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:256`

The graph model also explicitly supports URL edges and recursive child reuse:

- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:53`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:57`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:64`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:76`

Collection does build URL edges with child nodes:

- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:352`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:358`
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs:366`

But the final transclusion engine only threads a matching child subgraph into
the local-file recursive compose path:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1340`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1353`

The remote Markdown path recursively composes the fetched child, but it only
sets `child_options.source = child_source`; it does not attach the matching
preflight child node before calling `run_compose_pipeline_internal(...)`:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:894`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:920`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:971`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:974`

Result: root-level remote directive target resolution can use the root
resolution cache, but a remote child that itself transcludes a grandchild falls
back to parsing/resolving its own outgoing edges from scratch. That is the same
class of partial graph reuse fixed for local files, just left open for URL
children.

Add a URL-aware lookup, for example `child_for_url(&Url)` or a source-key helper
that handles both `ComposeSource::File` and `ComposeSource::Url`, and thread
that node into the remote child `child_options.preflight_graph`. Add an L1 test
for root `::url child.md` → child `::url grandchild.md` showing the graph is
recursive and graph-seeded output matches the baseline. If remote recursive
reuse is intentionally out of scope for v2, narrow the design/API docs so
"resolved child paths/URLs" does not promise recursive URL reuse.

Verification level: Level 1 is appropriate. This is compose orchestration and
target-resolution reuse, not terminal rendering, terminal input encoding, or
keyboard UX.

## Test Coverage Assessment

- Level 1 present: condition-blind approval collection across frontmatter, page
  blocks, shell blocks, local and remote transclusions; condition-aware
  execution; `execution_set ⊆ approval_set`; loop stability; pre-execution
  membership validation; cache default; body/frontmatter/shell-block no-cache
  parsing and execution; volatile warnings; dynamic command-shape rejection;
  approval lifecycle; CLI `--shell` listing; stale-span graph reuse; local
  recursive graph reuse.
- Level 1 gap: no test proves recursive preflight graph reuse for remote
  transclusion children.
- Level 2/Level 3: not required for this feature. The reviewed behavior is
  document transformation and shell-approval orchestration, not real-terminal
  rendering or OS keyboard input.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter markdown::compose::preflight --lib
cargo test --color=never -p darkmatter markdown::compose::shell_expansion --lib
cargo test --color=never -p darkmatter-cli test_compose_shell_reports_discovered_commands_without_executing --test cli
```

Results:

- `darkmatter` preflight unit slice: passed, 66 tests.
- `darkmatter` shell-expansion unit slice: passed, 257 tests; 1 ignored alias
  test.
- `darkmatter-cli` targeted CLI integration test: passed, 1 test.
