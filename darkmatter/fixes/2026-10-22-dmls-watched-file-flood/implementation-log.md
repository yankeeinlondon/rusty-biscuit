# Implementation log: 2026-10-22-dmls-watched-file-flood

## 2026-10-22: implementation

- **R1:** the graph build records every planned candidate of a path
  reference that located nothing (`ReferenceResolver::awaited`, exposed as
  `WorkspaceGraph::awaits`). `WorkspaceIndex::unlisted_changed` rebuilds for
  a created unlisted Markdown file only when the snapshot awaits it.
- **R2:** `WorkspaceIndex::begin_batch` / `end_batch` defer every rebuild
  inside a batch to one at its end. `ServerState::apply_disk_changes`
  applies one notification's changes as one batch, under one index lock,
  storing the snapshot once.
- **R3:** `ListedKinds` compiles the discovery filter once and reads each
  directory's ignore-filtered entries at most once per notification;
  `listed_kind` is now a one-shot use of it.

## Measurements (A3)

The reproduction is one `workspace/didChangeWatchedFiles` notification
creating 200 Markdown files under a `.gitignore`d `target/` in a
3,000-document workspace. It ran as an L1 test (debug build) against the
real server through `LspFixture`, timed from the notification to the
answer of a request queued behind it, on macOS with load 7–26.

| Build | Round 1 | Round 2 |
| --- | --- | --- |
| `main` at `3884a0743` | 8.92 s | 8.28 s |
| this fix | 53 ms | 52 ms |

The experiment test was not kept, because its timing is
host-dependent. A1 and A2 assert the behavior structurally, by counting
index generations:

- `test_only_an_awaited_unlisted_creation_rebuilds`
- `test_a_batch_of_joined_files_rebuilds_once`

Each test fails under its mutation: dropping the `awaits` check in the
first, or rebuilding inside a batch in the second.

## Verification (A4)

- `cargo nextest run -p dmls`: 1,268 passed.
- `cd darkmatter && just test`: 10,586 passed.
- On the idle Linux host (`just cross-check … --os linux`): `darkmatter`
  passed 8,144 and `dmls` passed 1,268, with no test over 2.5 s.
- `cargo clippy -p dmls --all-targets -D warnings`: clean.
