---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 6 closes the non-zero-origin SVG assembly, cache schema upgrade, and
mocked search/listing coverage from iteration 5. The feature is not ready for
production: the image Level 2 test still cannot distinguish a rendered icon
from normal terminal text, the default build still includes `resvg`, online
icon listings silently stop at 20 results, and the canonical lint gate fails.

## Findings

### 1. High: The image Level 2 assertion passes without proving that an icon was painted

The test now captures the WezTerm window as pixels, but it treats any pixel with
an RGB component above 30 as proof of image rendering. The captured window also
contains the shell prompt, the command, and the printed icon name, so ordinary
text satisfies this predicate even when the graphics protocol path emits
nothing. The test also returns success when `capture_window_png()` returns
`None`, including under a required Level 2 run.

Evidence:

- `cli/tests/level2_terminal.rs:165-175` deliberately leaves command and prompt text in the captured window.
- `cli/tests/level2_terminal.rs:177-184` converts a failed/unavailable pixel capture into a passing test.
- `cli/tests/level2_terminal.rs:186-198` scans the entire window for any non-near-black pixel rather than checking an image-specific region or visual witness.
- `just test-l2` reported this test as passing, but the predicate is already satisfied by the visible shell text.

Capture a baseline before rendering and compare a tightly bounded icon region,
or render a distinctive solid-color witness at known coordinates and assert its
pixel geometry/color. When Level 2 is required, an unavailable screenshot must
fail through the test-level gate rather than return normally.

### 2. High: The default-off `image` feature still does not remove `resvg` from the default graph

`biscuit-terminal` now gates its direct `resvg` dependency, but its unconditional
`biscuit-visualized` dependency brings in two `resvg` versions through
`biscuit-visualized` and `mermaid-rs-renderer`. This still violates the spec's
dependency boundary that enabling `image` pulls in `resvg`; the default build
compiled both versions during validation. Updating the dependency documentation
to describe the contradiction does not implement the specified behavior.

Evidence:

- `lib/Cargo.toml:10-12` forwards `image` only to `biscuit-terminal/image`.
- `biscuit-terminal/lib/Cargo.toml:24-29` leaves `biscuit-visualized` unconditional.
- `docs/dependencies.md:10` explicitly acknowledges that `resvg` remains transitive.
- `cargo tree -p biscuit-icon --no-default-features -i resvg@0.45.1` reaches it through `biscuit-visualized`.
- The same command for `resvg@0.46.0` reaches it through `mermaid-rs-renderer` and `biscuit-visualized`.

Make the relevant visualization/rasterization dependencies optional and forward
features through the dependency chain, or revise the specification before
release. The current implementation does not satisfy the accepted design.

### 3. High: Online icon listings silently truncate matching results to 20

The CLI contract says `icons [filter]` lists matching icons and the full online
catalog is available when connected. The implementation hard-codes a limit of
20, ignores the response's total/limit metadata, and implements no pagination.
Users therefore receive an incomplete result set with no indication that more
matches exist. The optional empty filter uses the same search call and limit, so
it cannot provide an online catalog listing either.

Evidence:

- `cli/src/commands.rs:13` fixes `ONLINE_ICON_LIMIT` at 20.
- `cli/src/commands.rs:75-93` fetches exactly one search response.
- `lib/src/iconify/client.rs:76-83` deserializes but intentionally discards response metadata.
- `lib/src/iconify/client.rs:155-176` returns only the first response page.
- CLI tests use one- or two-result fixtures and do not cover totals above the requested limit.

Paginate until all matches are consumed, or expose and document an explicit
limit with truncation feedback. Add a wiremock test whose total exceeds one page
and an online test for the no-filter command.

### 4. Medium: The canonical lint gate fails after the feature-gating changes

`just lint` fails the default build because `Cursor` and `TerminalImage` remain
unconditionally imported while all of their uses are behind `feature = "image"`.
Image-enabled test compilation also reports an unused `SHARED_KITTY` static and
an unused `body_json` fixture.

Evidence:

- `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs:15` has an unused `Cursor` import.
- `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs:18` has an unused `TerminalImage` import.
- `cli/tests/level2_terminal.rs:25` retains the no-longer-used Kitty harness static.
- `cli/tests/cli.rs:248-253` constructs an unused JSON fixture.

Gate the imports with the feature or move them into the gated block, and remove
the unused test artifacts. Production readiness requires the repository's
canonical lint recipe to pass.

### 5. Medium: Direct online lookup bypasses the injected client and lacks deterministic CLI coverage

The new client injection is used by search and set listing, but a direct
`prefix:name` command still calls `Icon::iconify`, which constructs the public
client internally. Consequently `ICONIFY_BASE_URL` and `run_with_client` do not
apply to this user-facing path, and it cannot be tested through the CLI against
wiremock. Existing direct-lookup CLI tests cover only an embedded icon or reject
the request before lookup.

Evidence:

- `cli/src/commands.rs:53-62` routes direct identifiers through `lookup_icon` without the injected client/cache.
- `cli/src/commands.rs:169-176` calls `Icon::iconify(id)` for non-embedded identifiers.
- `cli/tests/cli.rs:66-88` tests only `--from` rejection for direct identifiers.
- No CLI test proves a direct network miss is fetched, cached, and then served offline.

Open the command cache once and pass both it and the injected client through all
lookup paths. Add a direct custom-id wiremock test followed by an offline cache
hit. This also ensures ordinary L1 tests cannot accidentally reach the public API.

### 6. Medium: Set catalog caching is partial and its new test does not verify persistence

`fetch_collections()` returns the complete collection map, but `sets()` stores
only entries matching the current display filter. A successful online request
therefore does not make the fetched catalog available for later offline filters.
The test named `sets_merges_online_and_caches` checks only first-run stdout and
never re-runs offline or inspects the cache.

Evidence:

- `cli/src/commands.rs:114-150` places `cache.put_set` inside the display-filter condition.
- `cli/tests/cli.rs:300-324` does not verify any persisted row or offline second run.
- The spec's listing boundary says online catalog results are cached thereafter.

Cache every successfully fetched collection, then filter only for presentation.
Verify a second command using a dead endpoint can list a different prefix from
the previously fetched collection response.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate |
| Local SVG styling and non-zero origins | Level 1 | Appropriate |
| Cache-first network lookup | Level 1 wiremock | Appropriate at library level |
| Previous cache schema compatibility | Level 1 | Appropriate for the supplied previous-schema fixture |
| Online icon search merge/filter/cache | Level 1 wiremock | Partial: pagination/no-filter/direct lookup are missing |
| Online set listing and persistence | Level 1 wiremock partial | Output verified; full-catalog persistence is not |
| Browser and Markdown inline SVG | Level 1 | Appropriate |
| Terminal Unicode/Nerd Font/text output | Level 2 | Appropriate |
| Image-protocol fallback | Level 2 attempted | **Level mismatch: pixel assertion is not image-specific and capture failure passes** |
| Styled CLI errors | Level 2 | Appropriate |
| OS keyboard behavior | Not applicable | No Level 3 requirement |

## Validation

- `just test`: passed 92 library tests and 21 CLI tests.
- Image-enabled non-Level-2 run: passed 108 tests with 6 Level 2 tests skipped.
- `just test-l2`: reported 6 passes, including the image test, but that test's assertion is not discriminating.
- `just lint`: failed on two unused imports in the default `biscuit-terminal` build.
- Default `cargo tree` probes confirmed `resvg` 0.45.1 and 0.46.0 remain reachable without the `image` feature.

