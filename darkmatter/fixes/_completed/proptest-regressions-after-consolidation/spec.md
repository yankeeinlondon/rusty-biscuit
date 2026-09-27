---
area: darkmatter
status: unscheduled
created: 2026-09-23
owner: Ken Snyder <ken@ken.net>
origin: 2026-09-22-consolidated-test-binaries-wave-2 (Phase 1, S2 hazard H-P1)
related:
    - 2026-09-21-consolidated-test-binaries
    - 2026-09-22-consolidated-test-binaries-wave-2
packages:
    - darkmatter
---

# Darkmatter's proptest regression seeds stopped replaying after consolidation

## Evidence

`2026-09-21-consolidated-test-binaries` moved two committed regression files,
5 `cc` seeds in all, beside their modules:

- `darkmatter/lib/tests/l1/schema_quoting_safety.proptest-regressions`
- `darkmatter/lib/tests/l1/schemas_grammar_proptest.proptest-regressions`

Proptest 1.11.0, the locked version, persists failures with the default
`FileFailurePersistence::SourceParallel("proptest-regressions")`
(`proptest-1.11.0/src/test_runner/failure_persistence/file.rs:77-81` and
`:336-367`). It walks up from the test's source file to the nearest ancestor
directory that contains a `lib.rs` or `main.rs`.

- **Before the move:** `darkmatter/lib/tests/` had no `main.rs`, so the walk
  found nothing and fell back to `<source>.proptest-regressions` beside the
  file. The old layout worked by that fallback.
- **After the move:** `darkmatter/lib/tests/l1/main.rs` exists. The files are
  now read from and written to
  `darkmatter/lib/tests/proptest-regressions/<stem>.txt`. The two files beside
  the modules are never read.

Neither test overrides `failure_persistence`, and
`darkmatter/lib/tests/proptest-regressions/` does not exist, so the 5 seeds are
silently not replayed. The first time either property fails again, proptest
will write a new file at the new location. The old file will still look
authoritative.

## Fix

- Rename each file byte-for-byte:
  - `darkmatter/lib/tests/l1/schema_quoting_safety.proptest-regressions` →
    `darkmatter/lib/tests/proptest-regressions/schema_quoting_safety.txt`
  - `darkmatter/lib/tests/l1/schemas_grammar_proptest.proptest-regressions` →
    `darkmatter/lib/tests/proptest-regressions/schemas_grammar_proptest.txt`
- Show the seeds replay. One way is to run each test once with a temporarily
  planted seed that fails, confirm it fails, then discard the plant.
- `2026-09-22-consolidated-test-binaries-wave-2` rulings R18 applies the same
  rule to `biscuit-file`'s `yaml_mutation` seeds. Its `consolidation.py move`
  is expected to perform this relocation mechanically, so this fix can reuse
  it.

## Closes when

No `*.proptest-regressions` file remains under any consolidated target
directory (`tests/<target>/`). Each moved seed file sits at proptest's
`SourceParallel` location, and its replay has been shown.
