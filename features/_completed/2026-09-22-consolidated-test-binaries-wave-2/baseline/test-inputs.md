---
kind: baseline
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
rev: 208051f75
probe_script: baseline/test-input-probe.py
listing_script: baseline/test-input-listings.py
probe_output: baseline/test-input-probe-before.json
listings: baseline/test-inputs-listings-before.json
---

# Test-input probes (spec hazard 6), before migration

## Identities are derived, not stored

`scripts/ci/test_inputs.py` has no table of test identities to translate.

- **Targets.** `targets_from_metadata` (`:199-234`) builds each `Target` from
  `cargo metadata`: `package`, `kind`, `name`, and `src_path` as the root.
  `binary_id` is computed from those fields (`:136-142`,
  `<package>::<name>` for an integration test).
- **Module paths.** `scan` (`:554-647`) starts at each target root and walks
  `mod` declarations. It uses `_declarations` (`:526-551`) for files with no
  candidate literal, and `_scan_file` otherwise. Child files are resolved the
  way rustc resolves them: `#[path]`, `<dir>/<name>.rs`, or
  `<dir>/<name>/mod.rs`. Each file's module path is the chain of names that
  reached it.
- **The narrowed unit.** `_unit` (`:257-276`) builds `binary_id(<id>)`,
  `binary_id(<id>) & test(=<module::fn>)`, or
  `binary_id(<id>) & test(/^<module>::/)` from those derived values.

After a package's move, the same code walks `tests/<target>/main.rs → mod
<former>;`. So a reference that today yields `binary_id(pkg::former)` will
yield `binary_id(pkg::l1) & test(/^former::/)`, and a
`test(=fn)` unit gains the `former::` prefix. Nothing needs rewriting.

**One caveat (rulings R15):** `_unit` returns no narrowed unit for any
reference inside a test binary whose name starts with `level2`, `level3`,
`browser`, or `real` (`NON_L1_BINARIES`, `:254`). This probe shows it drops
references today in `biscuit-terminal-cli` (`level2_cursor_and_hygiene.rs:39`,
`level2_image.rs:53`). No in-scope package has a reference that moves from a
non-marker binary into a `level2`/`level3` one: `biscuit-tui-cli` has zero
references.

> **Superseded 2026-09-23 (review-1):** `NON_L1_BINARIES` is gone. `_unit`
> now decides tier from the test path alone and honors `required-features`,
> so the line numbers above describe the pre-review tree. The
> `biscuit-terminal-cli` controls still resolve to no unit, now because their
> modules carry the `level2_` marker.
>
> **Superseded 2026-09-23 (review-2):** a reference outside a test function
> now names its whole binary, `binary_id(<id>)`, because any module of the
> binary may call the helper that holds it. The `test(/^<module>::/)` form
> above is no longer produced, and a helper's unit is dropped only when its
> binary holds no L1 test. The `biscuit-terminal-cli` controls are helpers, so
> they now resolve to `binary_id(biscuit-terminal-cli::level2)`, which holds
> the L1 `prose_cells` tests.

## Method

1. `baseline/test-input-probe.py` runs the planner's own index over every
   tracked non-source path (`affected_scope.is_package_source_path` false),
   restricted to the ten packages' integration-test targets. It found 589
   references: `claudine` 368, `schematic-gen` 93, `dmls` 84,
   `biscuit-terminal-cli` 15, `sniff-cli` 12, `biscuit-file` 7,
   `claudine-gen` 7, `tree-hugger` 2, `sniff` 1, `biscuit-tui-cli` 0.
2. One probe path per package was picked from those references (below).
3. `baseline/test-input-listings.py` runs, for each unit,
   `cargo nextest list -p <pkg> --features <CI features> -E '(<unit>) & (<L1 tier filter>)'`.
   That is what a narrowed L1 cell selects.

## Probes and today's narrowed sets

| Package | Probe path | Unit(s) today | Tests selected |
|---|---|---|---:|
| `tree-hugger` | `tree-hugger/lib/tests/fixtures/corpus_manifest.yaml` | `binary_id(tree-hugger::tree_file)` | 89 |
| `claudine` | `claudine/cli/Cargo.toml` | `binary_id(claudine::boundary_lint)` | 7 |
| | | `binary_id(claudine::tts_phase5_contract) & test(=shipped_claudine_artifacts_enable_native_detached_audio)` | 1 |
| `sniff` | `sniff/lib/benches/ci-bench-ids.txt` | `binary_id(sniff::bench_ids_sync)` | 3 |
| `biscuit-file` | `biscuit-file/lib/tests/corpus/yaml_corpus.json` | `binary_id(biscuit-file::yaml_corpus)` | 13 |
| `schematic-gen` | `schematic/gen/tests/fixtures/complex_auth.json` | `binary_id(schematic-gen::openapi_import_test)` | 8 |
| | | `binary_id(schematic-gen::postman_golden)` | 6 |
| `biscuit-terminal-cli` | `biscuit-terminal/cli/README.md` | `binary_id(biscuit-terminal-cli::integration_test) & test(=public_docs_do_not_advertise_removed_atomic_tokens)` | 1 |
| `claudine-gen` | `claudine/docs/research/agent-errors/_schema.yaml` | `binary_id(claudine-gen::agent_errors_check)` | 10 |
| `dmls` | `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_docs.md` | `binary_id(dmls::mapping_only_corpus)` | 2 |
| `sniff-cli` | `sniff/cli/tests/snapshots/snapshots__cargo_monorepo_structure_json.snap` | `binary_id(sniff-cli::spawn_site_guard)` | 8 |
| `biscuit-tui-cli` | none: the package's tests read no tracked non-source file | — | 0 |

The full test lists are in `baseline/test-inputs-listings-before.json`.

## What each package's `test-inputs.md` must show after its move

- Re-run both scripts on the moved tree. For every probe, the after units must
  be the module-qualified form of the before units, and the selected test
  sets must be equal once each identity is normalized through the package's
  migration manifest.
- **`sniff-cli`:** the probe is an insta snapshot that the move renames
  (S2: `tests/l1/snapshots/l1__snapshots__…`). Use the renamed path from
  `snapshot-mapping.json`. `spawn_site_guard` reads the whole `tests/`
  directory, so the after unit is still the guard module.
- **Controls:** `biscuit-terminal-cli`'s fixture references in
  `level2_image.rs` and `level2_cursor_and_hygiene.rs` resolve to no unit
  before the move and must still resolve to none after it. `biscuit-tui-cli`
  must still have zero references after its move.
- A probe unit that disappears or selects a different set stops that
  package's migration (rulings R15).
