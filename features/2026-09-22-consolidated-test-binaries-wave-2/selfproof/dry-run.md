# `move` dry run on the ten real packages

Revision `9d44e7988`, in a throwaway detached worktree (`/tmp/wave2-move-dryrun`, removed
afterward). Nothing here touched this checkout. Manifests came from `plan --listings capture-a`
for each package. They carry no hand edits: no dispositions and no `keep_inner_cfg`.

## After-move checks (all ten packages moved together)

| Package | Modules | Roots | Path repairs | Seeds relocated | `body-diff` files / identical / other | `check-attributes` failures | `check-proptest` failures |
|---|---:|---|---:|---:|---|---:|---:|
| `tree-hugger` | 10 | `l1` | 41 | 0 | 10 / 8 / 0 | 0 | 0 |
| `claudine` | 15 | `l1` | 2 | 0 | 15 / 13 / 0 | 0 | 0 |
| `sniff` | 19 | `l1` | 7 | 0 | 19 / 8 / 0 | 0 | 0 |
| `biscuit-file` | 15 | `l1-fetch`, `l1` | 0 | 1 | 15 / 15 / 0 | 0 | 0 |
| `schematic-gen` | 14 | `l1`, `level2` | 3 | 0 | 14 / 11 / 0 | 2 | 0 |
| `biscuit-terminal-cli` | 14 | `l1`, `level2` (+`common`) | 9 | 0 | 14 / 5 / 0 | 0 | 0 |
| `claudine-gen` | 11 | `l1`, `level2` | 0 | 0 | 11 / 10 / 0 | 2 | 0 |
| `dmls` | 11 | `l1` (+`common`), `level2` | 11 | 0 | 11 / 7 / 0 | 1 | 0 |
| `sniff-cli` | 11 | `l1` (+`common`), `level2` (+`common`) | 11 | 0 | 11 / 0 / 0 | 0 | 0 |
| `biscuit-tui-cli` | 15 | `l1` (+`common`), `level2` (+`common`), `level3` | 10 | 0 | 15 / 3 / 0 | 2 | 0 |

Every `check-attributes` failure below is a hazard S2 predicted. Each needs a manifest
`dispositions` entry in its package's phase (rulings R19, and S2's `biscuit-tui-cli` row).
None of them is a mover defect:

- `schematic-gen`: schematic/gen/tests/e2e_generation.rs: uses `crate::`, which re-points to the consolidated root once the file is a module; record a disposition
- `schematic-gen`: schematic/gen/tests/http_client.rs: uses `crate::`, which re-points to the consolidated root once the file is a module; record a disposition
- `claudine-gen`: claudine/gen/tests/pipeline.rs: uses `crate::`, which re-points to the consolidated root once the file is a module; record a disposition
- `claudine-gen`: claudine/gen/tests/level2/level2_report_terminal.rs:16: identity-sensitive construct exact_path_string depends on the test path or binary (R9); record a disposition
- `dmls`: darkmatter/dmls/tests/l1/stdio_subprocess.rs:85: identity-sensitive construct exact_arg depends on the test path or binary (R9); record a disposition
- `biscuit-tui-cli`: biscuit-tui/cli/tests/choose_cli.rs: uses `crate::`, which re-points to the consolidated root once the file is a module; record a disposition
- `biscuit-tui-cli`: biscuit-tui/cli/tests/keyboard_protocol.rs: uses `crate::`, which re-points to the consolidated root once the file is a module; record a disposition

`biscuit-file`'s seed file moved to `biscuit-file/lib/tests/proptest-regressions/yaml_mutation.txt`
(R18). `sniff`'s `integration.rs` gained `#[path = "../fixtures.rs"] mod fixtures;` (R19).

## Built and compared (`tree-hugger`, `biscuit-file`)

The other eight packages were restored to `HEAD`. These two got `autotests = false` and the
`[[test]]` entries `move` printed. `biscuit-file`'s old `[[test]] fetch_integration` entry was
removed by hand (an SPP 3 edit). Both were then captured for every feature set and compared
with `capture-a` under their manifests:

| Package | Feature set | Present | L1 selected | Differences |
|---|---|---:|---:|---|
| `biscuit-file` | `fetch` | 777 | 777 | none |
| `biscuit-file` | `none` | 752 | 752 | none |
| `tree-hugger` | `none` | 483 | 483 | none |

Verdict: `identical`, with 0 failures and 0 notes across every selector (tiers and overrides).
Raw listing digests differ, as expected, because the binary names changed.
This is not either package's migration evidence: Phase 3 and Phase 4 redo it in the real tree, with layout gates and Linux captures.
