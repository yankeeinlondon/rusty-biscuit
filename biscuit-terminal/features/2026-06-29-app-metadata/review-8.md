---
ready: false
implemented: true
agent: codex/default
created: "2026-07-01T23:34:31"
---

# Review 8: App Metadata

Not production ready. The review-7 `--plain` leak is fixed for the reported non-diagram path and now has a focused Level 1 regression, but the implementation still violates the spec's explicit Warp / `ConfigFormat::None` acceptance gate.

## Findings

### High: Warp is still modeled as `ConfigFormat::None`, contrary to the spec's floor-bound app rule

The spec explicitly reserves `ConfigFormat::None` for apps outside the app-coverage floor and calls out Warp as not allowed to use it:

- [spec.md](spec.md:138) defines `None` as "no parseable file AND outside the coverage floor; not Warp".
- [spec.md](spec.md:359) says `format: None` is reserved for apps that both have no parseable file and are not in the coverage floor.
- [spec.md](spec.md:572) says Warp is floor-bound, must keep its candidate, and must be reclassified from `format: None` to its real on-disk format.

The seed data does the opposite:

- [seed.rs](../../lib/src/discovery/app_metadata/seed.rs:556) gives Warp's `~/.warp` candidate a `Some(ConfigFormat::None)` override.
- [seed.rs](../../lib/src/discovery/app_metadata/seed.rs:564) sets Warp's primary `config.format` to `ConfigFormat::None`.
- [types.rs](../../lib/src/discovery/app_metadata/types.rs:39) was also changed to permit a floor-bound app to use `None`, which directly contradicts the spec.
- [about.rs](../../cli/tests/about.rs:292) now locks in this behavior with `test_about_warp_json_treats_warp_directory_as_locator_only`, asserting `/config/format == "None"`.

This is not just a documentation disagreement. `bt about warp --json` reports `"format": "None"` and every core setting as locator-only even when `~/.warp` exists. The spec required the implementation to confirm what `warp_config_path(&home, os)` points at, classify the real on-disk format, and extract values where v1 extraction supports that format. If only part of `~/.warp` is parseable, the seed data should still model that real format/candidate honestly and document which settings remain unavailable; it should not use the reserved `None` escape hatch for a floor-bound app.

Verification level: Level 1 is sufficient here because the behavior is pure metadata / filesystem / JSON report output. The current Level 1 coverage is backwards: it verifies the non-compliant Warp behavior instead of preventing it.

## Verification Run

- `cargo check -p biscuit-terminal --color never`: passed.
- `cargo nextest run -p biscuit-terminal app_metadata --color never`: passed, 33/33 focused Level 1 tests.
- `cargo nextest run -p biscuit-terminal-cli --test about --color never`: passed, 19/19 Level 1 CLI integration tests.
- `cargo nextest run -p biscuit-terminal-cli test_block_plain_overrides_force_color --color never`: passed, 1/1 focused Level 1 regression.

No Level 2 or Level 3 coverage is required for the app-metadata resolver/extractor, `bt about` JSON/plain contract, or `--plain` precedence. These are deterministic file/env/rendering decisions that are appropriately verified at Level 1.
