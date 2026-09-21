---
kind: spike-record
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 1
status: complete
nextest_resolved: cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)
---

# S2 — Nextest listing fidelity against the resolved version

All measurements below are local (macOS, aarch64-apple-darwin), no CI. The
resolved binary on this host:

```
$ cargo nextest --version
cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)
release: 0.9.136
commit-hash: 1d5bf1ec99c918517d2c0f0cad2c1075c48d4b20
commit-date: 2026-05-16
host: aarch64-apple-darwin
```

This is the exact string the completion record should retain per R4 (the
first line; the remaining lines are recoverable from it).

## Fixture

A scratch workspace (single crate `s2-probe`, two integration test binaries
`alpha` and `beta`) built for the five listing questions. The scratch tree
was ephemeral (system temp); the transcript below is complete enough to
rebuild it. The shipped `scripts/ci/fixtures/archive-portability/` fixture
was also exercised for the archive leg — see the host note at the end.

`alpha.rs` carries: `plain`, `ignored_by_attribute` (`#[ignore = "…"]`),
`level2_marked`, `slow_marked`, and `linux_only` under
`#[cfg(target_os = "linux")]` (compiled out on this macOS host).
`beta.rs` carries: `plain` (same test name as alpha's — the collision case),
`beta_only`, `ignored_in_beta` (`#[ignore]`).

Filters are the repository's own `_tier_filter` strings
(`just/devops.just:1017`), applied verbatim.

## The exact JSON shape `completion.py` parses

```
$ cargo nextest list --message-format json [-E <filter>]
```

emits one JSON document (single line) with three top-level keys:
`rust-build-meta`, `rust-suites`, `test-count`.

- `rust-suites` is an object keyed by **binary name** (`s2-probe::alpha`).
  Each value carries at least `package-name` and `binary-id` (identical to
  the key here) and a `testcases` object keyed by **test name**.
- Each testcase value is exactly (no other fields observed):

  ```json
  {"kind": "test", "ignored": <bool>,
   "filter-match": {"status": "matches"|"mismatch", "reason": "ignored"|"expression"}}
  ```

- A lib target with zero unit tests appears as a suite entry with
  `testcases: {}` (`s2-probe` above) — `rust-suites` is not sparse.
- `test-count` counts **every discovered test** (7 in the fixture), not the
  matches (3 under the L1 filter). It must never be used as the expected-set
  size.
- `rust-build-meta.target-directory` points at the workspace target dir — or
  at nextest's **extraction temp dir** in archive mode. It is producer
  locality, not identity; the manifest must not record it.

## Answers to the five questions

### 1. An `#[ignore]`d test

Listed, with full identity, as a mismatch:

```
s2-probe::alpha  ignored_by_attribute  {"kind":"test","ignored":true,"filter-match":{"status":"mismatch","reason":"ignored"}}
```

The `reason` distinguishes it from a filter mismatch — the v2 manifest can
record ignored identities explicitly (today's v1 drops both).

### 2. A test excluded by `cfg` on this target

**Absent from the document entirely.** `linux_only` does not appear in any
suite's `testcases`, under any filter. A cfg-excluded test is absent from
that target's expected set, exactly as the specification states — which is
why the manifest must record its target triple and why a listing from
another target is not a valid comparison.

### 3. A test excluded by the tier filter

Listed, with full identity, as a mismatch with a different reason:

```
s2-probe::alpha  level2_marked  {"kind":"test","ignored":false,"filter-match":{"status":"mismatch","reason":"expression"}}
```

Under the L2 filter the verdicts invert (`level2_marked` matches; `plain`,
`slow_marked` mismatch by `expression`). Identities never disappear when the
filter changes — only the verdict does.

### 4. Two binaries sharing a test name

Both appear; the `binary-id` disambiguates:

```
s2-probe::alpha  plain  … "matches"
s2-probe::beta   plain  … "matches"
```

Identity for comparison is `<binary-id>::<test name>` — the same convention
`_expected_manifest` and the rollup's JUnit rebuild use today. Two
`plain` results are two distinct expected identities.

### 5. The same package listed from an archive with `--workspace-remap`

`cargo nextest archive` (workspace mode) then, from a **different directory**
containing a copy of the workspace (a stand-in consumer checkout):

```
$ cargo nextest list --message-format json \
    --archive-file /tmp/s2-scratch.tar.zst \
    --workspace-remap <consumer-checkout> -E <L1 filter>
```

emits **the same document shape, the same `binary-id`s, the same testcase
entries, and the same filter-match verdicts** as the workspace listing. Two
operational facts matter:

- `--workspace-remap` requires a real workspace at the remap path: a bare
  directory fails with `error: workspace root manifest at …/Cargo.toml does
  not exist` (exit 96). A consumer always has its checkout, so this is free,
  but the failure is worth a coded reason in `completion.py`.
- The binaries inside the archive are the **producing host's**; the
  cfg-absence question (2) is decided at archive-build time, so an archive
  manifest's target provenance is the producer's target, not the consumer's.

## Decision — is `filter-match.status == "matches"` alone sufficient?

**Yes, for building the expected set, under four conditions the v2 manifest
must enforce:**

1. **No `--run-ignored`** anywhere in the invocation. With
   `--run-ignored=ignored|all`, ignored tests flip to `matches`; the
   repository's canonical recipes never pass it, and the manifest should
   record enough of the invocation to detect it if one ever does.
2. **Identity is `<binary-id>::<test name>`** so same-name tests across
   binaries never collide (question 4).
3. **Target provenance is recorded** (target triple, environment, tier, and
   whether the listing came from an archive with a remap) so a manifest from
   another target is refused for the comparison — the cfg-absence behavior
   (question 2) makes a cross-target diff meaningless.
4. **The expected count is derived by enumerating `testcases`**, never from
   `test-count` (which counts discoveries, not matches).

The `reason` field (`ignored` vs `expression`) is additionally retained so
the v2 manifest can distinguish ignored tests from tier-filtered exclusions
explicitly — the thing today's v1 cannot express and Phase 4 Wave 1 requires.

## Host note — the shipped archive-portability fixture

`cargo nextest archive` directly against a copy of
`scripts/ci/fixtures/archive-portability/` fails to build on this host
(`cannot satisfy dependencies so 'std' only shows up once` — its `dylib`
crate-type mixes with static linking under a plain `cargo test --no-run`).
Its own integration suite drives it through `ci-build produce`
(`scripts/ci-build-archive-tests.rs::produce_fixture`), which is the
supported path and the one CI exercises on all three OSes; the spike's
archive leg therefore used the scratch workspace. Nothing about the listing
shape differs — archive mode is a transport, not a different listing
semantic. If Phase 4's fixtures want the portability crate's payload classes
behind `completion.py`, drive it the way its suite does.
