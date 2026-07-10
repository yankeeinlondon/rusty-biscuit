# Phase 7 closeout

## Test inventory

The final Darkmatter inventory contains 5,378 tests, ten more than the Phase 1 baseline of 5,368.
The additions are the three shared-reference parser regressions, five context-capture ownership and
GPU/demand regressions, and two expression-registration invariants. The Level-2 render-tree target
still contains exactly 20 tests, matching the Phase 1 count; its module-qualified names changed
only because Phase 3 moved the tests into responsibility-specific modules.

## Scope and portability review

The complete implementation diff adds no generic pass trait, plugin framework, or production
platform-command discovery. Production behavior changes remain limited to UTF-8-safe reference
title parsing, GPU-only context population, and the documented expression-catalog API migration.
The extracted code uses `Path`/`PathBuf`, platform-neutral module paths and fixtures, guarded
environment mutation, and `sniff` for repository and host discovery. No new Unix-only production
command, separator, or filesystem assumption was found.

Active comments, rustdoc, topic documentation, and the Darkmatter skill consistently identify the
compose pipeline, domain-owned expression registrations, and the public
`expression_function_descriptors()` accessor. Removed transform APIs and expression constants now
occur only in historical records, baselines, or explicit migration inventories.

## Validation

- `cargo check -p darkmatter -p darkmatter-cli -p dmls -p claudine -p claudine-cli`: passed.
- Darkmatter `just test`: passed.
- Darkmatter `just test-l2`: passed (19 Darkmatter and 69 Darkmatter CLI tests; DMLS had no matching
  Level-2 tests).
- Darkmatter `just lint`: passed.
- Focused context-capture nextest run: 19 passed.
- Claudine library nextest run: 3,146 passed; Claudine contract: 47 passed.
- Claudine `just lint`: passed for the library, contract, and CLI.

The broad Claudine CLI test target was stopped after it exceeded the non-interactive session's
per-command time limit while continuing to pass. Phase 6 had already recorded its canonical
package checks; Phase 7 changed only whitespace in Darkmatter source and plan documentation.

## Deferred Phase 5 structure

The Phase 5 commit extracted per-domain key ownership but left capture orchestration, population,
and most tests in `capture/mod.rs`. The corresponding Phase 5 plan tasks remain unchecked rather
than being misreported as complete.
