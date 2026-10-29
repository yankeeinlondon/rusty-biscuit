## Implementation of Review Findings #1

> **started at:** 2026-10-26T14:52:55-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/review-1.md'
- this is iteration 1 of the review-to-implement cycle 

## Follow-up Pass for Review #1

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/review-1.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/spec.md'

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| O1 | Fingerprint helpers bypass the repository hashing library (`DebugHasher` over `std`'s `DefaultHasher` in `dmls/src/overlay/analysis.rs` and `lib/src/markdown/schemas/simplified/query.rs`) | **fixed**, with sibling sites | Defect class: production hashing through `std::collections::hash_map::DefaultHasher` instead of `biscuit-hash`. Added streaming `biscuit_hash::XxHasher` (implements `std::hash::Hasher` and `std::fmt::Write`) so `write!` still streams `Debug` text without building it; both `DebugHasher` wrappers are removed. Sibling sites fixed in the same class: `lib/src/markdown/compose/body_origin.rs::hash_text` (now `xx_hash`) and `lib/src/render/link.rs::generate_popover_id` (now `XxHasher`). Tests: `biscuit-hash` `xx_hasher_matches_one_shot_hash_across_splits`, `xx_hasher_streams_formatted_text`, `xx_hasher_finish_does_not_consume_state`; `darkmatter` `render::link::tests::popover_ids_are_stable_and_keep_url_and_display_apart`; one `hashing_guard::production_source_hashes_through_biscuit_hash` L1 guard each in `darkmatter`, `dmls`, and `darkmatter-cli` that rejects `DefaultHasher` anywhere in the package's `src/`. Existing fingerprint, body-origin, and snapshot tests still pass |

- decision: **where the streaming hasher lives**
        - chosen: a public `XxHasher` in `biscuit-hash` (XXH64, seed 0, same digest as `xx_hash_bytes` over the concatenated input), as the review recommended
        - alternatives considered: build the `Debug` string and call `xx_hash` (reintroduces the large allocations the fix removed); a private wrapper over `xxhash-rust` inside darkmatter (still bypasses the library)
        - downsides: popover element ids in rendered HTML change value once (still deterministic); no test or doc pinned the old values
- decision: **guard scope**
        - chosen: one source-scan guard per package that owns production `src/` in the area (`darkmatter`, `dmls`, `darkmatter-cli`), each reading only its own `src/`, so CI schedules it without a `source-inputs` declaration
        - alternatives considered: one guard reading every package's sources (needs cross-package `source-inputs`); per-site behavior tests only (would not catch a new `DefaultHasher` site)
        - downsides: the guard is textual, so a comment naming `DefaultHasher` also fails it; none exists today
- docs: `biscuit-hash/README.md` and `.claude/skills/biscuit-hash/library-api.md` document `XxHasher`; no darkmatter `docs/` page described the hashing internals
- verification
        - `just test` in `biscuit-hash/`: 77 passed; `just lint`: clean
        - `just test` in `darkmatter/`: 12,575 passed, 10 skipped
        - `just lint` in `darkmatter/`: clean, including the Zed WASM check
        - `just check-tier-coverage darkmatter`: zero stranded tests
        - cross-OS check skipped: the change is pure in-process hashing with no path, process, or OS-specific code
