# Phase 5 generator drift profile

Date: 2026-08-01
Host: native Windows, branch `fix/claudine-windows`

## Measurement setup

The generator test binaries were built with Cargo build concurrency capped at
four:

```powershell
cargo build -p claudine-gen --tests -j 4
```

Each affected drift test was then run alone three times, serially, under the CI
profile with retries disabled. Every invocation included
`--build-jobs 4 -j 4`:

```powershell
cargo nextest run -p claudine-gen --profile ci --retries 0 `
  --build-jobs 4 -j 4 -E 'test(=<exact-name>)'
```

The initial test build passed in 43.069 seconds. The following measurements are
per-invocation wall times, including nextest startup and its approximately
1.2-second no-op build check.

| Exact test | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| `committed_data_matches_regenerated_inputs` | 48.059s | 47.399s | 47.886s |
| `committed_catalog_matches_regenerated_inputs` | 47.616s | 47.207s | 47.428s |
| `committed_families_match_regenerated_inputs` | 47.315s | 47.284s | 65.921s |

The selected test bodies were normally 44.3-44.8 seconds. The third families
wall-time outlier did not recur and did not explain the stable 44-second floor.

## Phase breakdown

Temporary opt-in timers divided input loading into roster, facts, overrides,
each research document's Markdown parse and effective-schema resolution,
serialization, validation, and coercion. One instrumented data drift run had a
44.04-second test body.

- `DarkmatterSchemas::effective_for` took 533-574ms for each research document.
- Markdown parsing took 0-5ms per document.
- Frontmatter serialization, validation, and coercion each rounded to 0ms.
- One provider's eight research documents took 4.426-4.433 seconds to load.
- The data drift test repeats that work for ten wired providers, accounting for
  essentially the entire 44-second test body.

The cost was repeated repository/package-area discovery. Without an explicit
`FileResolutionContext`, each schema reference reached Darkmatter's repository
structure detection. Rendering, comparison, subprocess startup, and ordinary
filesystem variation were not material contributors.

The temporary timers and their environment switch were removed before the
final build.

## Decision and implementation

The measurement selected decision-gate branch 1: batch repeated discovery in
the production generator path.

`inputs::load` now resolves its area to one absolute request boundary, captures
the repository root and Claudine package-area root when a repository exists,
and reuses one `DarkmatterSchemas` instance across the provider's research
topics. Relative and absolute area spellings therefore share the same boundary.
An area outside a repository keeps the previous discovery-compatible default.
Failure to absolutize the area is returned as `GenError::Io`.

This is the production loading path used by generation and checking, not a
test-only shortcut. No nextest timeout override was added and the tests were not
renamed with `slow_`: optimized isolated bodies are below one second.

GitNexus could not resolve the Rust symbols and reported risk `UNKNOWN`. Direct
caller inventory found two production callers of `inputs::load`, two production
callers plus one test caller of `load_validated_frontmatter`, and
two production callers plus two test call sites of `generate_for_area`. No HIGH or
CRITICAL graph result was available.

## Post-change evidence

The same nine exact invocations all passed:

| Exact test | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| `committed_data_matches_regenerated_inputs` | 7.319s | 3.790s | 3.983s |
| `committed_catalog_matches_regenerated_inputs` | 3.825s | 3.781s | 3.766s |
| `committed_families_match_regenerated_inputs` | 3.637s | 3.610s | 3.588s |

The first post-change data invocation rebuilt the changed test binary. A later
five-test focused run reported 0.667-0.717 seconds for the three drift tests,
0.307 seconds for the no-repository pipeline, and 0.239 seconds for the relative
area inside a temporary repository. All five passed with retries disabled.

The required generator gate passed on the first attempt after the relative-area
regression fix:

```powershell
just test-gen --profile ci --no-fail-fast --build-jobs 4 -j 4
```

- generator nextest: 154 run, 154 passed, 1 skipped; 12.967 seconds;
- affected drift tests in the full run: all passed below one second;
- signal fixture replay: 83 records, 83 positives passed, 83 negatives passed,
  17 documented exclusions applied, 0 failures; and
- total recipe wall time: 25.425 seconds.

The build emitted the known unchanged `messenger` Windows dead-code warnings
for BurntToast and SnoreToast helpers. They did not affect the generator gate.

`just sanity` passed all five Claudine packages in 146.028 seconds:
`claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and
`claudine-gen`.

The area-wide `just lint` reached `claudine` and stopped after 39.186 seconds on
three unchanged `messenger` failures outside this fix's scope:

- dead code: `BurntToastHelper::{mark_app_id_registered, app_id_registered}`;
- dead code: `SnoreToastHelper::{mark_app_id_registered, app_id_registered}`;
  and
- `clippy::collapsible_if` in `messenger/lib/src/provider/desktop/windows.rs`.

The preceding transport and lifecycle-document guards passed, as did the
catalog-types lint. A direct `just _lint claudine-gen` then passed with warnings
denied, proving the changed package is lint-clean.
