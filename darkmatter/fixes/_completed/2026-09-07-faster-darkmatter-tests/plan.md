---
total_phases: 11
created: 2026-09-07
phase: 1
agent: claude/default
yolo: "true"
fix: 2026-09-07-faster-darkmatter-tests
spec: darkmatter/fixes/2026-09-07-faster-darkmatter-tests/spec.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - zed-dmls-cli
---

# Execution plan — Faster Darkmatter tests through isolated composition and complete evaluation

Converts [spec.md](spec.md) into eleven ordered phases. Spec sections are
referenced as **RB1**–**RB5** (Required behavior 1–5) and acceptance bullets as
**AC1**–**AC8**, in the order they appear in the spec.

## Grounding facts (verified against this working tree, 2026-09-07)

Checked, not assumed. A phase that contradicts one of these should stop and
re-derive rather than proceed — the spec is explicit that it is a draft and not
evidence that every suspected cost is present.

- **The `md` spawn population is 504 sites across 38 files.**
  `grep -c 'md_cmd('` over `darkmatter/cli/tests` returns 510 occurrences; 8 of
  those are the `fn md_cmd()` definitions themselves, leaving **502 call
  sites**. `schema_validate_baseline.rs` carries **2 more** inline
  `assert_cmd::Command::cargo_bin("md")` spawns (lines 74, 101) with no helper
  at all — the spec's evidence section misses these.
- **The seven private `md_cmd()` copies are confirmed**, each an exact
  `assert_cmd::Command::cargo_bin("md").unwrap()` with no CWD/home/PATH
  defaults: `schema_triggers.rs:7`, `code_block.rs:18`, `schema_about.rs:7`,
  `compose_schema_file_rewrite.rs:23`, `compose_schema.rs:14`,
  `schema_validate.rs:8`, `schema_detect.rs:8`. The shared one is
  `common/mod.rs:11`.
- **Per-file `md_cmd()` census** (the guard's own count supersedes this in
  Phase 5; the seven private-definition files are marked ★):

  | file | sites | file | sites |
  |---|---|---|---|
  | `clean.rs` | 48 | `clean_schema.rs` | 11 |
  | `schema_validate.rs` ★ | 38 | `compose_base_schema.rs` | 9 |
  | `hash_kind_save_diff.rs` | 32 | `validate_refs.rs` | 8 |
  | `code_block.rs` ★ | 32 | `rm.rs` | 8 |
  | `get_set_rm.rs` | 31 | `help.rs` | 8 |
  | `clean_frontmatter.rs` | 30 | `compose_shell.rs` | 8 |
  | `layout_flags.rs` | 29 | `compose_basic.rs` | 8 |
  | `compose_state_set.rs` | 26 | `layout_style_frontmatter.rs` | 7 |
  | `hash_directory.rs` | 23 | `compose_transclusion.rs` | 7 |
  | `schema_about.rs` ★ | 17 | `compose_interpolation.rs` | 7 |
  | `render_basic.rs` | 17 | `schema_detect.rs` ★ | 6 |
  | `hash.rs` | 15 | `clean_json.rs` | 6 |
  | `compose_schema.rs` ★ | 15 | `compose_refs_and_missing.rs` | 5 |
  | `schema_triggers.rs` ★ | 14 | `layout_fill.rs` | 4 |
  | `graph.rs` | 12 | `toc.rs` / `delta.rs` | 3 each |
  | `compose_remote_caching.rs` | 12 | `compose_terminal_detection.rs` | 2 |
  | | | `compose_schema_file_rewrite.rs` ★ | 2 |
  | | | `compose_perf.rs` | 2 |
  | | | `compose_page_blocks.rs` | 2 |
  | | | `compose_layout.rs` | 2 |

- **`zed-dmls-cli` is 6 raw spawns in one 119-line file** (`tests/cli.rs`), and
  they are *already partly isolated*: every one that touches state passes
  explicit `--staging-dir` / `--zed-data-dir` / `--zed-log` into a per-test
  `TempDir`, and two set `PATH=""`. None controls CWD or home. This is the
  cohort the spec allows to close by **justified disposition** rather than
  migration; decide it on evidence in Phase 6D, not by default.
- **The HTTP fixture has exactly one consumer.** `mock_http_server`
  (`cli/tests/common/mod.rs:42-83`) detaches a `thread::spawn` accept loop with
  no join handle, no shutdown, and no bound other than the response vector; it
  is used only by `compose_remote_caching.rs` (8 servers). Failure before the
  expected accept count strands the thread for the life of the test binary
  process.
- **Process/protocol sites outside the CLI**: `dmls/tests/stdio_subprocess.rs`
  spawns `bin_exe!("dmls")` raw (`:69`) and polls on a 20 ms sleep (`:141`);
  `dmls/tests/level2_editor_neovim.rs` shells out to `nvim` (`:86`, `:151`) and
  polls its capture deadline on a fixed 200 ms sleep (`:377`);
  `cli/tests/level2_schema_about.rs:93` shells out to `tmux`;
  `cli/tests/common/level2.rs` sleeps at `:179`, `:285`, `:410`;
  `cli/tests/level2_errors.rs` at `:42`, `:58`;
  `lib/tests/level3_popover.rs` has six `tokio::time::sleep` calls and
  `lib/tests/level3_image_painting.rs:108` a 400 ms one.
- **Test population is ≈7,879 attributes** across the four packages
  (`darkmatter` lib 6466 = 5695 embedded + 771 in `tests/`; `darkmatter-cli`
  737 = 139 + 598; `dmls` 648 = 519 + 129; `zed-dmls-cli` 28 = 22 + 6).
  One hand-written row per identity is not a credible deliverable; RB1's
  "reviewed row **or explicitly enumerated family**" has to be closed
  mechanically.
- **The area's nextest override surface is empty.** `.config/nextest.toml`
  carries 18 per-test `slow-timeout` overrides and 3 `test-group` bindings, and
  **none of them scopes darkmatter, darkmatter-cli, dmls, or zed-dmls-cli** —
  they are all claudine, sniff, or biscuit-terminal. AC5-equivalent work here is
  therefore *preventive*: `git diff main -- .config/nextest.toml` must stay
  **empty** for the whole fix, not merely free of additions.
- **Two distinct L1 cohorts already exist by construction.** `_tier_filter L1`
  excludes `level2_/level3_/browser_/real_/slow_`; `darkmatter/justfile`'s
  `test` recipe sets `BISCUIT_L1_INCLUDE_SLOW=1` under CI, which drops the
  `slow_` exclusion. The area has exactly **one** `slow_`-prefixed test and
  **11** `#[ignore]` attributes, so the local/CI population delta is small and
  fully enumerable — RB5's "distinct cohorts" requirement is cheap to satisfy
  precisely here.
- **All four packages declare `[package.metadata.ci.tests]`** — lib: tiers
  `L1, L2, browser`, features `terminal-tests, browser-tests`, backend
  `wezterm`, `l1-include-slow`; cli: `L1, L2`, `terminal-tests`, backends
  `tmux, wezterm`; dmls: `L1, L2`, `terminal-tests`, backend `tmux`,
  runner-tools `neovim, zed-extension`; zed-dmls-cli: `L1` only. No declaration
  gap to close.
- **The Zed WASM extension IS reachable from CI.** `_package-ci.yml:519` runs
  `cd darkmatter && just zed-verify`, and `dmls`'s `runner-tools` declares
  `zed-extension`. The spec presumes it is outside recipe selection; correct
  that in the inventory — the route exists, it is just not `just test`.
- **Bench and fuzz entry points**: `darkmatter/lib/Cargo.toml` declares **16**
  `[[bench]]` targets (`schema_validation`, `effective_schema_ownership`,
  `render_tree`, `compose_pipeline`, `render_pipeline`, `prose_highlighter`,
  `render_pipeline_steps`, `compose_schema_transclusion`, `render_code_heavy`,
  `reference_graph`, `phase6_interpolation`, `phase8_render`, `phase9_remote`,
  `phase10_residuals`, `phase11_evidence`, `clean_hot_paths`), plus a real
  `darkmatter/lib/fuzz` directory driven by `just fuzz` and
  `.github/workflows/fuzz-nightly.yml`. The area also owns `just bench-dmls`,
  which generates a synthetic corpus under `../target/dmls-bench/`.
- **A prior remediation already landed and must not be redone.**
  `lib/tests/ambient_ctx_capture.rs` opens with a module doc stating it builds a
  purpose-built two-package fixture repository *precisely because* a full
  `ComposeContext::capture()` walks the whole checkout ("seconds on a developer
  machine and over a minute on a cold two-core CI guest"). Phase 7 verifies and
  records it; it does not rewrite it.
- **`common/mod.rs` is 323 lines.** claudine's equivalent
  (`claudine/cli/tests/common/mod.rs`) is **782**. Adding a comparable fixture
  inline would push the shared module to ~770 and past the 500-line soft cap
  that `just lint-files` reports (roots `./cli/src`, `./cli/tests`). That recipe
  is advisory — it is not wired into `just lint` and does not exit non-zero —
  but the fixture still belongs in its own `common/fixture.rs` module rather
  than inline.
- **`just lint` calls `check-zed`**, which hard-fails when the `wasm32-wasip2`
  rustup target is absent. Provision it once before the first lint checkpoint,
  or every checkpoint reads as a false red.
- **The reference implementation's shape**: `CliProcessFixture` (claudine
  `common/mod.rs:179`) plus `ClaudineCommandBuilder` (`:361`), with named
  escapes `fake_only_path()`, `host_path()`, `ambient_context(dir)`,
  `inherit_no_env()`, and a `checkout_containment_error()` precondition. The
  environment policy is applied inline in `build()` (`:453`).

## Assumptions and stated decisions

1. **Fixture placement is settled by the spec**: area-local, in
   `darkmatter/cli/tests/common/`, mirroring claudine — plus a tracked
   promotion follow-up (spec § Open Questions, third design). Phase 11 files
   that follow-up; no phase here promotes anything into `test_toolkit`.
2. **Naming mirrors the reference so the later promotion is a rename, not a
   redesign**: `CliProcessFixture` + `MdCommandBuilder`, with the same escape
   vocabulary. Divergence in escape names is the main thing that would make the
   follow-up expensive.
3. **The inventory is family-based with a mechanical completeness proof.** A
   TypeScript reconciler (house convention — TS scripts, never Python) joins
   `cargo nextest list --message-format json` against the inventory's declared
   families and **fails** on any identity assigned to zero or more than one row.
4. **Sibling-fix contamination is a recorded variable, not a gate.** This branch
   also carries `claudine/fixes/2026-09-07-faster-claudine-tests`. The two touch
   disjoint packages, but `claudine-cli` consumes darkmatter's `md` binary as a
   CI runner-tool fixture. Every baseline and candidate measurement records the
   exact revision and whether sibling test-only changes were present; a
   darkmatter budget is never derived from a run whose revision is unrecorded.
5. **Scope discipline on assertions.** Fixture migration, assertion repair, and
   test-population changes land in **separate reviewable commits**
   (`CLAUDE.md` § Scope discipline). A migration commit whose diff also changes
   an assertion gets split before review.
6. **No new override, retry, tier change, timeout increase, or disabled
   assertion** may be introduced at any point (AC6, AC7).
   `git diff main -- .config/nextest.toml` must be **empty** at every
   checkpoint — darkmatter owns none today and must own none at the end.
7. **Nothing is committed or pushed by an implementing agent.** Commit, push,
   and merge are separate operator actions per `CLAUDE.md`.

---

## Phase 1 — Baseline, measurement substrate, and evidence discipline

Establishes the attribution window. Everything numeric downstream is compared
against what this phase captures, so it lands before any source change.

- [ ] Record the baseline identity: revision SHA, `git status --porcelain`
      (dirty state), `rustc`/`cargo`/`cargo-nextest` versions, profile, host
      platform and core count, and whether sibling-fix changes are present in
      the tree (assumption 4).
- [ ] Provision `rustup target add wasm32-wasip2` so `just lint` → `check-zed`
      does not read as a false red at any checkpoint; record if it was already
      present.
- [ ] Confirm `git diff main -- .config/nextest.toml` is empty and capture that
      as the fix's starting invariant.
- [ ] Warm the build (`just build` from `darkmatter/`), then collect **five
      alternating warm local runs** of the local-default L1 cohort (`just test`)
      and the sanity cohort (`just sanity`). Keep **build/setup time, runner
      elapsed time, and summed per-test duration as three separate columns** —
      a serial sum is not wall clock under nextest concurrency.
- [ ] Collect one baseline run each for `just doctest`, `just test-l2`,
      `just test-browser`, and `just test-l3`. Where a harness (WezTerm, tmux,
      Chrome, neovim) is absent, record **"pending — harness unavailable"**
      explicitly; a clean skip is not evidence of passing.
- [ ] Collect the **CI-selected** cohort separately by re-running L1 with
      `BISCUIT_L1_INCLUDE_SLOW=1`, and enumerate the exact identity delta
      against the local cohort (expected: the single `slow_` test — confirm,
      do not assume).
- [ ] Read `ci.yml`'s matrix to enumerate which environment legs are actually
      configured for the four darkmatter packages. **Do not presume four legs**;
      write down what the workflow declares (`_package-ci.yml` natives plus the
      `_wsl-ci.yml` guest, if selected).
- [ ] Collect **three consecutive CI runs per configured leg** — same workflow
      definition and runner image, no intervening workflow edits — and store
      the JUnit artifacts under
      `darkmatter/fixes/2026-09-07-faster-darkmatter-tests/baseline/<run-id>/`.
      Record every intervening failed attempt with its cause; selecting only
      the green attempts is disallowed.
- [ ] Write the metrics/reconciler script (TypeScript, in this fix directory).
      It must **fail** on malformed reports, missing expected artifacts or
      tests, duplicate identities, invalid durations, and failed runs. A script
      that prints a miss and exits 0 is not a gate.
- [ ] Identify the work counters available for later proof — the library's
      `effects-instrumentation` feature (process-wide effect counters),
      `MockHttpServer::request_count()`, and any compose/discovery counters the
      inventory finds — and record which of the four required signals
      (discovery, composition, effects, HTTP requests) each covers. Note gaps as
      Phase 7/8 work, not as a reason to fall back on timing.

**Validation checkpoint 1** — baseline identity recorded; five alternating warm
local runs exist for both L1 cohorts with the three costs separated; three
consecutive CI runs exist per configured leg with failures disclosed; the
reconciler reproduces the baseline table from the stored artifacts; unavailable
tiers are named as pending rather than assumed green.

---

## Phase 2 — Complete inventory (RB1, AC1) ‖ runs during Phase 1's CI window

Document-only. No source changes. Produces `inventory.md` in this fix directory.

- [ ] Build the enumeration substrate: `cargo nextest list --message-format
      json` for each of the four packages under **every** feature selection a
      canonical recipe or CI leg uses — bare, `terminal-tests`,
      `browser-tests`, `effects-instrumentation` — recording the command,
      revision, toolchain, and features beside each capture.
- [ ] Capture the source-side population separately (attribute scan over
      `#[test]`, `#[tokio::test]`, `#[rstest]`, plus `#[ignore]` and `#[cfg]`
      gates; expected ≈7,879 attributes) and **diff it against runner
      discovery**. Every source test the runner never lists becomes a
      cfg/feature exclusion row carrying its reason and actual execution route.
- [ ] Enumerate the 11 `#[ignore]` sites and the single `slow_` test
      individually, each with its execution route and why it is gated.
- [ ] Inventory the non-nextest entry points as first-class rows:
      `just doctest` (lib + cli), the **16** `lib/benches` targets,
      `darkmatter/lib/fuzz` (via `just fuzz` and `fuzz-nightly.yml`), and
      `just bench-dmls` with its generated corpus under `../target/dmls-bench/`.
- [ ] Give the Zed WASM extension rows carrying its **real** route:
      `just check-zed` (standalone `cargo check --target wasm32-wasip2`,
      workspace-excluded) and `just zed-verify` (package + manifest/archive
      assertions), the latter executed in CI at `_package-ci.yml:519`. Correct
      the spec's presumption that it sits outside recipe selection.
- [ ] Record `dmls/vscode-dmls` as a **documented absence** row: no in-repo
      automated tests, manual packaging check only via
      `just install-vscode-package`. Absence is not coverage.
- [ ] Inventory shared fixture machinery as first-class rows: `cli/tests/common/
      mod.rs` (`md_cmd`, `md_file`, `mock_http_server`, `baseline::*`,
      `layout::*`), `cli/tests/common/level2.rs`,
      `lib/tests/level2_render_tree_terminal/support/mod.rs`,
      `lib/tests/layout_matrix_support/mod.rs`, and
      `lib/tests/error_snapshots/helpers.rs`.
- [ ] Write the per-family rows. Each records: behavior proved and whether the
      assertions distinguish a plausible failure; required boundary and external
      inputs (CWD, home/config/cache, environment, repository, installed tools,
      network, terminal, browser); shared setup; effect execution; waiting and
      timing floors; resource ownership and cleanup; shared-state coordination
      (including the `#[serial]` sites in `clean_counters.rs`,
      `layout_snapshots.rs`, `reference_integration.rs`,
      `level2_render_tree_terminal/file_links.rs`); tier, features, platforms;
      canonical recipe; measured cost with provenance; disposition.
- [ ] A family row is valid **only** when its members are explicitly listed and
      share the same setup and proof. Anything that differs gets its own row.
- [ ] Record the override census as its own short table and state the finding
      plainly: **darkmatter owns zero nextest overrides today**, so the
      obligation is preventive (assumption 6), and the one L1 timing floor in
      the area is the local/CI `slow_` split, not a runner override.
- [ ] Reconcile canonical recipes against the justfile: note that `test-real`
      is a declared no-op for this area, and confirm `sanity`, `test`,
      `test-l2`, `test-l3`, `test-browser`, `doctest`, `coverage`, `bench`,
      `fuzz`, `lint`, `all` all exist and select the four packages.
- [ ] Run the Phase 1 reconciler over the listings plus the declared families;
      paste its output into `inventory.md`. It must exit 0 with every identity
      assigned to exactly one row.
- [ ] Give every row a disposition: satisfactory, remediation in this fix, or
      linked follow-up naming the unmet requirement and the deferral reason.
      **No row may be dispositioned by a timing threshold** — a fast test is
      still reviewed for correctness and accidental effects.

**Validation checkpoint 2** — the reconciler exits 0; the inventory covers all
four packages, every tier, doctests, all 16 benches, the fuzz target, gated and
ignored tests, the WASM-extension route, the vscode absence row, and every
shared helper; every row carries a disposition and an execution route.

---

## Phase 3 — Attribution and ratified budgets (RB5, first half)

Document-only. Depends on Phases 1 and 2. Resolves the spec's four
baseline-review decisions **with numbers**.

- [ ] Attribute cost by family against the Phase 1 baseline, keeping build,
      elapsed, and summed-duration columns separate throughout.
- [ ] Answer decision 1 — *how much cost belongs to composition, discovery,
      rendering, and test setup?* Split the 504-site `md` cohort's cost into
      process launch, ambient discovery, and actual command work; a
      per-invocation launch cost multiplied by 504 is a hypothesis until
      measured, not a finding.
- [ ] Answer decision 2 — *which command paths genuinely require host tools
      beyond a minimal set?* Enumerate them from the inventory
      (`compose_shell.rs`'s shell expansion, git-dependent context tests,
      the tmux/WezTerm/neovim/Chrome tiers) and mark each as a named fixture
      escape or a genuine tool test.
- [ ] Answer decision 3 — *which DMLS/extension checks are reachable from
      existing recipes?* Using the Phase 2 route rows, state for each dmls and
      zed-dmls surface which recipe reaches it and which CI leg runs it.
- [ ] Answer decision 4 — *which repeated corpus/setup operations can be
      consolidated without reducing diagnostic quality?* Measure the shipped
      corpus scans (`example-docs` consumers in `lib/tests/style_frontmatter.rs`
      and `cli/tests/level2_frontmatter_tables.rs`, plus the `baseline::` JSON
      fixture loaders) and the repeated schema-corpus work in
      `meta_schema_*.rs` / `schemas_*_table.rs` before proposing any merge.
- [ ] Ratify per-family numeric budgets **on the existing CI runners**, written
      into `inventory.md` **beside the baseline table** so budget review and
      evidence review are one act. Local timing may attribute cost; it may not
      set a target.
- [ ] Do **not** extrapolate a speedup from an unprofiled substage. Cite
      [the redundant-walk results](../2026-07-16-redundant-walk/results.md) in
      the attribution section as the standing reason attribution precedes any
      percentage target.
- [ ] For any **production** defect surfaced during attribution (composition,
      expression semantics, cache freshness), open a linked spec with the
      evidence and leave the code alone — production changes are out of scope.

**Validation checkpoint 3** — each of the spec's four decisions has a written
answer backed by a measurement; budgets sit next to the baseline in
`inventory.md`; no budget was derived from a local run alone; production
findings are filed, not fixed.

---

## Phase 4 — The deterministic CLI fixture (RB2, infrastructure)

First code phase. Requires checkpoint 1. Everything in Phases 5–6 depends on it,
so it lands alone and green.

- [ ] Add `darkmatter/cli/tests/common/fixture.rs` and re-export from
      `common/mod.rs`. Keeping it out of `mod.rs` holds the shared module near
      its current 323 lines instead of ~770, which the area's 500-line soft-cap
      report would otherwise flag.
- [ ] Implement `CliProcessFixture` with a per-test disposable root that stays
      alive through process completion **and** cleanup — a `NamedTempFile`
      dropped before `assert()` is the classic version of this bug. Each test
      constructs its own fixture and lends it to the builder; no two parallel
      tests share a root.
- [ ] Make every launch input explicit in `MdCommandBuilder::build()`:
      **CWD** (fixture root, never the runner's); **home/config/cache**
      (`HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and on Windows
      `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, `APPDATA`, `LOCALAPPDATA`);
      **darkmatter cache root** (`--cache-root` / its env equivalent — remote
      artifacts must never land in the developer's real cache);
      **rendering inputs** (`COLUMNS`, `LINES`, `TERM`, `COLORTERM`,
      `NO_COLOR`, `FORCE_COLOR`); **application environment** (any `DM_*` /
      darkmatter-namespaced variables, swept by prefix); **Git plumbing**
      (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_CONFIG_*`, plus
      `GIT_CONFIG_NOSYSTEM=1`); and **PATH**.
- [ ] Default `PATH` to a minimal native tool set plus fixture stubs. Preserve
      **Windows executable resolution** — `PATHEXT` and the console variables
      must survive `env_clear`, or every Windows spawn silently fails to
      resolve.
- [ ] Reject temporary roots inside the checkout, **including via symlink**:
      canonicalize both the fixture root and the checkout root before the
      containment comparison, and fail with a named error (mirror claudine's
      `checkout_containment_error`). On macOS this also normalizes
      `/tmp` → `/private/tmp`.
- [ ] Provide named escapes with call-site-readable intent — `host_path()`
      (a real host tool is the subject), `fake_only_path()` (absence is the
      assertion), `ambient_context(dir)` (inherited-context behavior is the
      subject), `inherit_no_env()` — and require each use to carry a comment
      naming the tool or proof it needs.
- [ ] Provide topology builders for the tests that need real structure:
      disposable git repository init (repository-local identity only, no host
      hooks or signing), nested-directory schema lookup, and relative-reference
      layouts. **Copy shipped content byte-for-byte and preserve relative
      reference relationships when isolation requires relocation** — do not
      rewrite references to absolute paths, which would erase relative-
      resolution coverage entirely.
- [ ] Keep `md_file()` and `baseline::*` working, or migrate their callers in
      the same change; do not leave two competing temp-file conventions.
- [ ] Prove the fixture non-vacuous: write a fixture-level test that a hostile
      inherited environment (`GIT_DIR` at a throwaway repo, relocated `HOME`,
      `COLUMNS=44`, `FORCE_COLOR=1`, a poisoned `PATH`, and a checkout-ancestor
      `TMPDIR`) leaves the fixture's observed defaults unchanged. Use
      **disposable state only** — never edit the real checkout or the user's
      configuration.
- [ ] Verify Windows behavior by compiling the Windows arms
      (`cargo check -p darkmatter-cli --tests --target x86_64-pc-windows-gnu`)
      where the mingw toolchain is present; otherwise name `windows-latest` as
      the authority and record the check as **pending**. Prefer `cfg!(windows)`
      over `#[cfg]` where both arms should compile everywhere.

**Validation checkpoint 4** — `just test` and `just lint` green from
`darkmatter/`; the fixture's hostile-environment test passes and has a
demonstrated failing form; `common/mod.rs` stays under the soft cap;
`git diff main -- .config/nextest.toml` still empty.

---

## Phase 5 — Structural protection against new bypasses (RB2)

Depends on Phase 4. Lands with a **temporary, reasoned** migration exemption
list that Phase 6 burns to zero.

- [ ] Add the smallest maintainable guard: a source-scanning test in
      `darkmatter/cli/tests` that detects (a) `assert_cmd::Command::cargo_bin
      ("md")` / `Command::cargo_bin("md")` outside the fixture, and (b) a
      private `fn md_cmd()` redefinition. Model it on claudine's
      `spawn_site_guard.rs`, not on a new mechanism.
- [ ] Add the isolation arm: a fixture-built command that is then handed
      `.current_dir(...)`, `.env("PATH", …)`, `.env_remove("PATH")`, or
      `.env_clear()` is an attempt to undo the defaults and is a violation
      unless it goes through a named escape.
- [ ] Seed `SPAWN_ALLOWLIST` with the 38 files Phase 6 will migrate, each
      entry carrying a reason string. Implement the **stale-entry** arm: an
      allow-list entry for a file that no longer violates must **fail**, so the
      list cannot rot.
- [ ] Write the two negative tests AC2/AC3 require — one raw-spawn violation
      and one attempt to undo defaults — and prove both detectors non-vacuous:
      apply a neuter, show the named failure, restore, and `diff` the file back
      to identical. Transcribe the neuter/restore into the phase log.
- [ ] Add detector negatives so the guard does not fire on prose, string
      literals, comments, or a legitimate `bin_exe!("dmls")` in another package.
- [ ] Decide and record the guard's scope for `dmls` and `zed-dmls-cli`: either
      extend the same guard to those test directories or state in the inventory
      why their spawn populations (1 and 6 sites) are governed differently.

**Validation checkpoint 5** — the guard runs in L1, prints its census, and
fails on both planted violations and on a stale allow-list entry;
`just test` and `just lint` green.

---

## Phase 6 — Migration burn-down (RB2, AC2, AC3)

Batches **6A, 6B, 6C are mutually parallelizable** — disjoint file sets, each
deleting only its own allow-list entries. **6D is independent** and may run
alongside. **6E is the closure gate** and runs last. Each batch ends green
before the next merges; the guard's stale-entry arm catches a mis-merge.

### Phase 6A — the seven private helpers plus the two orphan spawns (‖ 6B, 6C)

The spec's named target. Highest risk of drift, so it goes first.

- [ ] Delete the private `fn md_cmd()` in `schema_validate.rs` (38 sites),
      `code_block.rs` (32), `schema_about.rs` (17), `compose_schema.rs` (15),
      `schema_triggers.rs` (14), `schema_detect.rs` (6),
      `compose_schema_file_rewrite.rs` (2), routing every call through a
      per-test `CliProcessFixture`.
- [ ] Migrate `schema_validate_baseline.rs`'s two inline
      `assert_cmd::Command::cargo_bin("md")` spawns (`:74`, `:101`) — these are
      **not** in the spec's evidence list and are otherwise easy to miss.
- [ ] Schema-lookup tests build their required directory topology inside the
      fixture. Where a test relied on the runner's CWD to find a schema, make
      that dependency explicit rather than deleting the case.
- [ ] Delete this batch's `SPAWN_ALLOWLIST` entries and confirm the stale-entry
      arm would fire if one were left behind.

### Phase 6B — clean / hash / frontmatter family (‖ 6A, 6C)

- [ ] Migrate `clean.rs` (48), `hash_kind_save_diff.rs` (32),
      `get_set_rm.rs` (31), `clean_frontmatter.rs` (30),
      `hash_directory.rs` (23), `hash.rs` (15), `clean_schema.rs` (11),
      `clean_json.rs` (6), `rm.rs` (8), `delta.rs` (3), `toc.rs` (3).
- [ ] `hash_directory.rs` and `graph.rs` walk directories: give each a fixture
      topology it owns, so the walked tree is the fixture's and not whatever
      the runner's CWD happens to contain.
- [ ] Preserve the **repeated read/write/read** round trip for persisted values
      (frontmatter set/rm, hash save) — the fixture relocation must not
      collapse a round trip into a single write.
- [ ] Delete this batch's allow-list entries.

### Phase 6C — compose / layout / render / graph family (‖ 6A, 6B)

- [ ] Migrate `layout_flags.rs` (29), `compose_state_set.rs` (26),
      `render_basic.rs` (17), `graph.rs` (12), `compose_remote_caching.rs` (12),
      `compose_base_schema.rs` (9), `validate_refs.rs` (8), `help.rs` (8),
      `compose_shell.rs` (8), `compose_basic.rs` (8),
      `layout_style_frontmatter.rs` (7), `compose_transclusion.rs` (7),
      `compose_interpolation.rs` (7), `compose_refs_and_missing.rs` (5),
      `layout_fill.rs` (4), `compose_terminal_detection.rs` (2),
      `compose_perf.rs` (2), `compose_page_blocks.rs` (2),
      `compose_layout.rs` (2).
- [ ] `compose_shell.rs` genuinely needs a shell: route it through
      `host_path()` with a call-site comment, and keep its real shell-expansion
      coverage. Do not stub the behavior under test.
- [ ] `compose_terminal_detection.rs` and the `layout_*` files assert rendering
      **policy**: make their terminal capability inputs explicit fixture values
      rather than inherited ones, which is also what makes them stop depending
      on the operator's terminal.
- [ ] `compose_refs_and_missing.rs`, `validate_refs.rs`, and `graph.rs` cover
      relative-reference and source-context behavior: build disposable
      repositories inside the fixture and **keep the references relative**
      (AC3's named requirement).
- [ ] `compose_remote_caching.rs` migrates its `md` spawns here; its HTTP
      fixture is Phase 8's work — do not entangle the two commits.
- [ ] Delete this batch's allow-list entries.

### Phase 6D — `zed-dmls-cli` disposition (‖ 6A–6C)

- [ ] Evaluate the 6 `Command::cargo_bin("zed-dmls")` sites against the fixture
      contract. They already isolate staging/data/log directories into a
      per-test `TempDir` and two of them null out `PATH`; what they do **not**
      control is CWD and home.
- [ ] Either migrate them to a `zed-dmls`-flavored fixture, or record a
      **specifically justified disposition** in `inventory.md` naming exactly
      which contract inputs are already explicit, which are not, and why the
      residual exposure is acceptable. A generic "already uses TempDir" is not
      a justification.
- [ ] Whichever route is taken, add the missing CWD/home control or state in
      writing that the commands provably ignore both.

### Phase 6E — burn-down closure (depends on 6A–6D)

- [ ] Assert `SPAWN_ALLOWLIST` contains **zero generic migration exemptions**
      (AC2). Any survivor carries a specific technical necessity **and** an
      equivalent isolation proof, written at the entry.
- [ ] Confirm the guard's governed population now covers every migrated file
      and that its census matches the Phase 5 count minus the migrations.
- [ ] Add the contamination probes AC3 requires, using **disposable state
      only**: hostile inherited CWD; relocated home/config/cache; `GIT_DIR` /
      `GIT_WORK_TREE` pointed at a throwaway repo; poisoned `PATH`; inherited
      rendering inputs (`COLUMNS=44`, `FORCE_COLOR=1`, `NO_COLOR=1`);
      darkmatter application variables; and a checkout-ancestor `TMPDIR`. Each
      probe must leave unrelated test results **unchanged**.
- [ ] Reconcile the test count: report additions and removals **separately**,
      never netted, so a lost test cannot read as an optimization.

**Validation checkpoint 6** — `just test`, `just lint`, `just test-l2` green
from `darkmatter/` (Phase 6 touches `common/`, which every L2 binary compiles);
zero generic exemptions remain; every contamination probe passes; test-count
reconciliation shows additions and removals separately;
`git diff main -- .config/nextest.toml` still empty.

---

## Phase 7 — Composition boundary and passive-path proof (RB3, AC5) ‖ with Phase 6

Library-side. Mostly disjoint from Phase 6's files; serialize only where both
touch the same binary.

- [ ] Audit the library test consumers of `ComposeContext::capture()` —
      `shell_block_integration.rs`, `reference_integration.rs`,
      `ambient_ctx_capture.rs`, `expression_regression.rs`,
      `git_context_integration.rs` — plus the embedded-test capture sites in
      `compose/context/`, `compose/tests/`, `compose/cache/hashing.rs`, and
      `reference/file_tree/`.
- [ ] Supply explicit test context through the existing `ComposeOptions` request
      API wherever discovery is **incidental**. Never recapture CWD or
      repository state downstream — that is a standing area boundary, not a
      test convenience.
- [ ] **Verify and record, do not rewrite**, `ambient_ctx_capture.rs`: its
      module doc already documents a purpose-built fixture repository chosen for
      exactly this reason. Re-remediating it is the assume-then-falsify pattern
      the spec warns against.
- [ ] **Retain** representative end-to-end tests for context capture,
      lazy demand-driven capture (only referenced `ctx.*` groups observed),
      source provenance, and CLI-to-library wiring. Name each retained test in
      the inventory so a later reader can see the coverage was deliberate.
- [ ] Retain real composition wherever shell expansion, transclusion,
      interpolation, hashing, or persisted state **is** the behavior under test.
- [ ] Prove passive paths are effect-free with **counters, not speed**: use the
      library's `effects-instrumentation` feature to assert schema validation,
      trigger matching, completion, and hover construct no `EffectEngine` and
      attempt no network access. Cover the DMLS side via
      `dmls/tests/no_side_effects.rs` and extend it where the inventory found
      an unguarded passive path. A fast result is not proof of absence of I/O.
- [ ] Extend the shared passive shipped-artifact corpus coverage rather than
      adding another full-corpus scan. Where Phase 3 justified consolidating
      repeated corpus work (`meta_schema_*`, `schemas_*_table`,
      `example-docs` consumers), keep one shared passive corpus test that is
      *extended* per regression, plus representative normal-invocation tests
      through the real shipped artifacts.
- [ ] Where an exhaustive representation matrix can prove the same contract at a
      cheaper API boundary, move it — and record the replacement coverage and
      the additional failure the replacement still detects.
- [ ] Repair tautological or stale assertions in **separately reviewable
      commits**. For each: the original failing input where one exists, the
      defect the old assertion could not distinguish, and what the replacement
      now detects.

**Validation checkpoint 7** — `just test`, `just doctest`, `just lint` green;
every passive-path claim is backed by a counter or sentinel assertion, not a
timing observation; shipped-artifact coverage is still present and named;
every assertion or population change carries a replacement-proof explanation.

---

## Phase 8 — Resource ownership: network, process, protocol, rendering (RB4, AC4) ‖ with Phase 7

- [ ] Rewrite `mock_http_server` (`cli/tests/common/mod.rs:42-83`) to **own its
      worker**: retain the `JoinHandle`, bind a local ephemeral endpoint, bound
      request handling, and expose explicit shutdown. Implement `Drop` (or an
      explicit close) so that **failure before the expected request count does
      not strand the accept loop**.
- [ ] Give the server a documented termination bound and prove it: a test that
      never issues the expected request must still tear the server down within
      that bound. A detached worker is not accepted as normal cleanup.
- [ ] Keep and extend the request-count/content verification in
      `compose_remote_caching.rs` (8 servers), preserving coverage of remote
      consent (`--allow-host`), cache freshness, refresh, TTL, and error
      behavior — all without public-network access.
- [ ] Replace the fixed 200 ms deadline poll in
      `dmls/tests/level2_editor_neovim.rs:377` with synchronization on the
      **final asserted condition**, with a deadline. Same for the 20 ms poll in
      `dmls/tests/stdio_subprocess.rs:141`.
- [ ] Make DMLS protocol tests synchronize on **protocol responses** rather than
      arbitrary sleeps, and isolate their workspace and cache state per test —
      `lsp_session.rs` is 5,530 lines and is the main beneficiary.
- [ ] Give `dmls/tests/stdio_subprocess.rs`'s raw `bin_exe!("dmls")` child
      bounded cleanup on success, failure, **and cancellation**; no orphan
      child on a panicking assertion.
- [ ] Audit the L2 sleep sites — `cli/tests/common/level2.rs:179,285,410`,
      `cli/tests/level2_errors.rs:42,58`,
      `cli/tests/level2_schema_about.rs:38,63`,
      `lib/tests/level2_render_tree_terminal/support/mod.rs:48` — and convert
      each readiness sleep into bounded observation of the final asserted
      condition. Retain any sleep that **is** the timeout contract and justify
      its budget, polling cadence, and shutdown margin in the inventory row.
- [ ] Audit the L3 sleeps (`lib/tests/level3_popover.rs` ×6,
      `lib/tests/level3_image_painting.rs:108`) the same way. L3 stays opt-in;
      where the harness is unavailable, record the evidence as **pending**.
- [ ] Rendering policy tests use explicit terminal capabilities or document
      models where sufficient; **keep** real-terminal tests for terminal
      behavior and headless browser tests for computed layout and style.
- [ ] Verify **no terminal or browser window gains focus** in any tier touched,
      and that browser tests remain headless with no host input injection.
- [ ] Prove cleanup empirically, not by inspection: run the affected tiers under
      nextest's leak detection and confirm no survivors. Treat a spurious
      LEAK-FAIL as the known nextest artifact it is and say so rather than
      papering over it.

**Validation checkpoint 8** — `just test`, `just test-l2`, `just test-browser`
green (or explicitly pending with the missing harness named); the HTTP fixture
terminates within its documented bound when the expected request never occurs;
no detached worker or orphan child survives a failing run; no focus was raised.

---

## Phase 9 — Local measurement (RB5, first evidence tranche)

Requires Phases 4–8 complete and green.

- [ ] Warm each revision's artifacts, then collect **five alternating warm local
      runs per revision** for every changed cohort **and** the full relevant L1
      suite. Alternate baseline/candidate on the same host with matching
      toolchain, features, profile, concurrency, and fixture inputs.
- [ ] Record for every run: revision, dirty state, platform, cache state,
      environment, exact commands, test identities, failures, and skips.
- [ ] Keep the **local-default** and **CI-selected** (`BISCUIT_L1_INCLUDE_SLOW=1`)
      cohorts as separate populations end to end; never compare one against the
      other.
- [ ] Measure any cold-build claim in an **isolated build directory** — never by
      clearing the developer's working cache.
- [ ] Re-run every changed timeout or synchronization case repeatedly **under
      representative suite load**, not in isolation, and report the spread.
- [ ] Prove eliminated discovery, composition, effects, and HTTP requests with
      **work counters or sentinel effects**, independent of timing. A timing
      improvement is not evidence that a walk was removed.
- [ ] Keep build/setup, runner elapsed, and summed test duration separate in
      every table, alongside identities, counts, failures, skips, timeouts,
      retries, and slow cases.
- [ ] Record local numbers as **attribution only**. They establish no CI target
      (Phase 3's budgets do).

**Validation checkpoint 9** — five alternating runs exist per changed cohort and
for the full L1 suite; repeated runs exist for every changed synchronization
case; each eliminated-work claim has a counter or sentinel behind it; both
cohorts are reported separately.

---

## Phase 10 — CI evidence (RB5, second tranche; AC6)

Operator-gated: push and read. No implementing agent commits or pushes.

- [ ] Run `just ci-local --lint-only`, then `just ci-local`, for the affected
      scope before requesting a push.
- [ ] Compile-verify the Windows arms where the mingw toolchain is present;
      state the limitation explicitly where it is not.
- [ ] Hand off for push, then collect **three consecutive candidate CI runs per
      configured leg** — same workflow definition and runner image, no
      intervening workflow edits. Record every intervening failed attempt with
      its cause.
- [ ] Compare **matched identities within each environment** against that
      environment's own Phase 1 baseline. Show added, removed, and gated tests
      **separately**; do not require identical cross-platform counts.
- [ ] Run the Phase 1 reconciler as the gate over both baseline and candidate
      sets; it must fail on malformed reports, missing artifacts or tests,
      duplicate identities, invalid durations, and failed runs.
- [ ] Compare against Phase 3's ratified budgets. **Report misses and their
      causes**; do not invent a universal speedup percentage and do not close a
      miss by adjusting the budget after the fact.
- [ ] Confirm CI slow-test and feature coverage is **unchanged**:
      `BISCUIT_L1_INCLUDE_SLOW=1` still selects the same population, the
      `terminal-tests` / `browser-tests` features are still enabled where
      declared, and `just zed-verify` still runs. Coverage may not be reduced to
      improve timing.
- [ ] Run the affected L2/L3/browser tiers through their canonical recipes only
      where the resources exist. Record unavailable runtime evidence as
      **pending**, never as passing.
- [ ] Re-confirm `git diff main -- .config/nextest.toml` is empty.

**Validation checkpoint 10** — three consecutive green runs exist per configured
leg, with intervening failures disclosed; every budget is met or its miss is
explained; no override, retry, timeout increase, tier change, or disabled
assertion was used to reach a number.

---

## Phase 11 — Closure: `results.md`, follow-up, drift, acceptance sweep

- [ ] Write `results.md` in this fix directory with: ratified budgets and the
      comparable timing and work-count evidence (three costs separate, per leg,
      per cohort); coverage changes (tests added, removed, moved boundary, and
      the replacement proof for each changed assertion); failures and skips;
      residual findings; and **separate** implementation / verified-locally /
      verified-on-CI completion claims.
- [ ] Give every deferred finding evidence, a reason, and a linked owner
      document. **Generic fixture migration may not be deferred** (AC2).
- [ ] File the tracked follow-up the spec's Open Questions require: a
      `darkmatter/features/_unscheduled/` (or `.claude`-appropriate) spec to
      promote the stabilized fixture core into `test_toolkit`, with claudine as
      the second consumer and a named owner. Without an owner, the third design
      decays into the first.
- [ ] Update the `darkmatter` and `rust-testing` skills plus area READMEs
      **only where the fixture or test workflow actually changed**
      (`CLAUDE.md` § Drift Maintenance). Where a comment or doc now contradicts
      the code, the code is correct — fix or delete the comment and say so.
- [ ] Confirm no production API changed. If one did, verify downstream
      consumers by impact analysis (Claudine consumes darkmatter), **not** by a
      workspace-wide default test run.
- [ ] Sweep the acceptance criteria explicitly, one subsection each:
  - [ ] **AC1** — every test/family, including extension and higher-tier
        surfaces, has an explicit disposition and execution route; no
        timing-based exclusions (reconciler output attached).
  - [ ] **AC2** — all deterministic CLI spawns use the fixture contract or a
        specifically justified equivalent; no generic migration exemptions
        remain; negative guard tests and stale-entry failure are present.
  - [ ] **AC3** — hostile inherited CWD/home/cache/Git/rendering/application
        inputs do not affect unrelated tests; relative-reference and
        source-context behavior remain covered using disposable repositories.
  - [ ] **AC4** — HTTP/process/protocol fixtures terminate within documented
        bounds when the expected interaction never occurs; no detached worker
        or child is accepted as cleanup.
  - [ ] **AC5** — passive paths are provably effect-free, shipped-artifact
        coverage is present, and every assertion/population change has a
        replacement proof.
  - [ ] **AC6** — `just test` and `just lint` pass; changed terminal and browser
        helpers verified through `just test-l2` and `just test-browser`; L3
        remains opt-in with unavailable evidence recorded; CI slow-test and
        feature coverage unreduced.
  - [ ] **AC7** — `results.md` complete, with ratified budgets, comparable
        evidence, failures/skips, and the three separate status claims; no
        retry or timeout-limit increase used as a performance fix.
  - [ ] **AC8** — area skills/READMEs describe the changed fixture and test
        workflow; downstream verification, if needed, was by impact.
- [ ] Final gate run from `darkmatter/`: `just sanity`, `just lint`,
      `just doctest`, `just test`, `just test-l2`, `just test-browser`
      (equivalently `just all`), plus `just check-zed`.

**Validation checkpoint 11** — all eight acceptance criteria are answered with
evidence or an explicitly linked deferral; `results.md` keeps the three
completion claims separate; the `test_toolkit` promotion follow-up exists with
an owner; no gate was weakened to close a criterion.

---

## Parallelism map

| Can run concurrently | Why it is safe |
|---|---|
| Phase 2 ‖ Phase 1's CI window | Phase 2 is document-only; it lands no code, so it cannot contaminate the attribution window. |
| Phase 6A ‖ 6B ‖ 6C ‖ 6D | Disjoint file sets; each batch deletes only its own allow-list entries, and the guard's stale-entry arm catches a mis-merge. |
| Phase 7 ‖ Phase 8 | Different concerns (composition boundary vs. resource ownership) and largely different files. |
| Phase 7/8 ‖ Phase 6 | Library and dmls files vs. CLI test binaries. Serialize only where both touch `cli/tests/common/`. |

**Strictly serial**: Phase 1 → Phase 4 (no code lands before the baseline is
captured); Phase 4 → Phase 5 → Phase 6 (the fixture must exist before the guard
can sanction it, and the guard before the burn-down can be proven complete);
Phases 6–8 → Phase 9 → Phase 10 → Phase 11 (evidence follows the change it
measures).

## Dependency order (summary)

```text
1 ──┬─→ 4 ─→ 5 ─→ 6A ─┐
    │           6B ─┤
    │           6C ─┼→ 6E ─┬→ 9 ─→ 10 ─→ 11
    │           6D ─┘      │
    └─→ 2 ─→ 3 ────────────┴→ 7 ‖ 8 ──┘
```

## Out of scope (guard rails)

Named so a phase does not quietly widen. Anything here that turns out to be
necessary becomes a linked follow-up, not an in-flight expansion:

- production composition changes, expression-semantics or cache-freshness
  changes, rendering redesign, editor feature development, and global CI
  infrastructure changes (spec § Evidence and scope);
- promoting the fixture into `test_toolkit` inside this fix, or migrating
  claudine as part of it — that is the tracked follow-up, filed in Phase 11;
- adding any nextest override, retry, tier change, timeout increase, or
  disabled assertion as a substitute for a fix — darkmatter owns zero overrides
  today and must own zero at the end;
- weakening the spawn or isolation detector, or dropping the stale-entry
  failure, to resolve a false positive — the sanctioned resolution is an
  allow-list entry naming the command the site targets;
- rewriting `ambient_ctx_capture.rs`, whose fixture-repository remediation
  already landed for exactly this reason;
- reducing CI slow-test or feature coverage, moving tests to a slower tier, or
  narrowing a population to improve a timing number;
- fixing a production defect found during attribution in place of filing it.
