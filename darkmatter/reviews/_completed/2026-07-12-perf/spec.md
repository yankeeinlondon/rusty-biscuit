---
repo: "rusty-biscuit"
scope: "darkmatter library + md CLI (with cross-package findings in sniff and biscuit-terminal where darkmatter's hot paths land there)"
created: "2026-07-12"
method: "4 parallel code-review passes (compose, rendering, schemas, CLI/IO) + empirical verification with hyperfine, RUST_LOG tracing, and `md compose --perf` against a release build of the current branch"
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-14
---

# Performance Review: Darkmatter Library and CLI

> **Reader's note (2026-07-14 review):** The original draft identified the
> right hot paths, but several suggested optimizations silently changed public
> or side-effect semantics. The reviewed design keeps `sniff::detect_timezone()`
> as the full, NTP-reporting API while Darkmatter explicitly requests local-only
> timezone data; caches expensive terminal facts rather than a whole ambient
> `Terminal`; validates a schema from one resolved/coerced instance; preserves
> source-ordered body-shell execution; and retains collision-resistant validator
> cache identity through `biscuit-hash`. It also adds the missing delivery,
> reproducibility, compatibility, and cross-platform contracts below.

## Executive Summary

The render path is in good shape — syntect sets, themes, and regexes are all
process-cached, plain renders of a 110 KB document complete in ~15 ms, and the
compose pipeline's expensive machinery (transclusion, frontmatter shell
expansion, remote fetch) is already parallelized and single-flighted. The
problems are concentrated in four places:

1. **Every `md compose` performs a live network NTP round-trip** (~60 ms,
   up to 3 s offline) via the "always-on" datetime context capture. This is
   the single largest cost on a typical compose and is empirically confirmed.
2. **Terminal capability detection is repeated and partially uncached** — the
   OSC 10 text-color query round-trips the tty on every `Terminal`
   construction, and one CLI invocation can construct `Terminal::default()`
   up to four times.
3. **The schema stage does its work twice per compose** — the effective
   schema is resolved twice, coercion runs twice, and coercion compiles
   validators that are never cached, all inside one `schema_validation::run`.
4. **`Markdown::toc()` is O(n²)** — empirically confirmed: 4× the document
   size costs 11× the time. It sits under `md toc`, structured/detailed
   hashing, and the reference graph's heading indexes.

### Finding counts

- **Critical: 1** (network call on every compose)
- **High: 5**
- **Medium: 12**
- **Low: 15**

## Scope and Delivery Contract

This document is a measured optimization backlog, not authorization to land all
33 findings as one undifferentiated refactor. Each numbered finding is an
independent change and must pass its own before/after checkpoint. Findings that
do not show a repeatable improvement outside measurement noise should not land.

### In scope

- The `darkmatter` library and `md` CLI hot paths named in the findings.
- The narrow `sniff` and `biscuit-terminal` changes required by Darkmatter's
  measured paths.
- Benchmarks, regression tests, and documentation needed to keep the gains and
  behavior stable.

### Non-goals

- Changing compose output, validation results, rendering output, CLI JSON
  shapes, or public Rust signatures merely to make an optimization easier.
- Making arbitrary shell directives concurrent.
- Turning `Terminal` into a process-global snapshot of the current working
  directory, environment, stream attachment, or terminal configuration.
- Changing directory-hash membership without an explicit compatibility ruling
  (Open Question 4).
- Replacing bounded run-local caches with unbounded process-global caches.

### Compatibility and correctness invariants

1. Except for Finding 1's intentional removal of Darkmatter's unused NTP
   probe, public behavior is byte-for-byte and error-for-error compatible.
   Snapshot/golden tests must cover composed Markdown, terminal output, JSON,
   diagnostics, and exit status where the changed path can affect them.
2. Public Rust APIs remain source-compatible. When an internal borrowed or
   `Cow` path is useful (Finding 13), retain the existing owned public facade.
3. Cache identity must include every input that can affect the cached result.
   Schema validators include schema bytes, document `base_dir`, and
   `file_ref_fallback_dir`; file-backed caches also retain their existing
   dependency/content invalidation contract. Cache hits may never change a
   validation answer.
4. Caches must be bounded and safe under concurrent library use. A performance
   fix must report its capacity/eviction policy and cannot introduce an
   environment- or current-directory-dependent global singleton.
5. Body shell directives retain source-order execution and failure semantics.
   Stable output order alone is not sufficient: commands may observe or mutate
   shared filesystem, environment, and process-external state.
6. All implementations must compile on macOS, Linux, and Windows. OS-specific
   probes require target-gated tests; Unix PTY/OSC tests must not make Windows
   test targets compile Unix-only imports.

### Evidence and completion criteria

A finding is complete only when all of the following hold:

- A release-build baseline and post-change result use the same fixture bytes,
  command, environment, TTY mode, warm-up, and sample policy. Record median (or
  mean), dispersion, and sample count; do not compare numbers from different
  hosts as a pass/fail gate.
- CLI fixtures are committed or generated by a checked-in deterministic command
  with recorded byte sizes and content hashes. The uncommitted ad-hoc fixtures
  used for the original measurements are evidence for the finding, but are not
  a reproducible regression harness.
- Microbenchmarks prove the local mechanism and an end-to-end command proves
  that the gain survives process startup and surrounding work. TTY findings
  include a real PTY/L2 measurement; piped hyperfine runs cannot validate OSC
  latency.
- Results are compared against a predeclared threshold: no statistically
  credible regression on unaffected benchmark groups, and a repeatable win
  larger than the harness noise on the target group. Absolute millisecond goals
  in this document are directional because host and terminal latency vary.
- `just test`, `just test-l2`, and `just lint` pass in each touched package area.
  Cross-package changes also run the root package recipe for every touched
  package and `cargo check --workspace`. Read-only formatting checks are
  permitted; this work must not run write-mode `cargo fmt`.

### Measured baseline (release build, macOS, stdout piped)

| Command | Mean | Notes |
|---------|------|-------|
| `md --help` | 11.6 ms | startup floor; no syntect/terminal work — good |
| `md small.md` (render) | 11.5 ms | 133-byte doc |
| `md large.md` (render) | 15.5 ms | 110 KB doc — render scales well |
| `md hash small.md` | 12.6 ms | |
| `md compose small.md` | 127.5 ms | trivial doc, no `ctx.*`, no transclusion |
| `md compose --no-trigger-schemas` | 115.2 ms | trigger discovery ≈ 12 ms |
| `md compose` no baseline + no triggers | 68.0 ms | baseline schema ≈ 47 ms wall |
| `md toc --json` 81 KB / 326 KB / 1.3 MB | 203 ms / 2.24 s / 45.3 s | **quadratic** (4× size → 11× time) |

`md compose --perf` attribution for the 127 ms compose: **capture context
59.6 ms**, validate references 7.2 ms, build options 8.1 ms, compose pipeline
20.0 ms (of which schema validation 4.6 ms). The 47 ms baseline-schema delta
vs the 4.6 ms attributed to the pipeline's schema stage corroborates findings
5–8: the baseline conversion/validation work runs multiple times across the
validation pass, options build, and pipeline.

---

## Critical

### 1. Every compose spawns `sntp time.apple.com` — a live network NTP probe — from the "always-on" datetime capture

**Severity:** critical (empirically confirmed)
**Category:** blocking I/O / network
**Location:** `darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:123` → `sniff/lib/src/os/time.rs:503-505`, `:366-386`

**Problem.** `populate_datetime` runs on **every** compose — including
documents that reference zero `ctx.*` values — under a comment that claims it
is free (`capture/mod.rs:42`: "DateTime is always included (zero-cost local
computation)"). It calls:

```rust
let tz_info = sniff::os::detect_timezone();          // datetime.rs:123
```

and sniff's `detect_timezone()` defaults to `detect_timezone_with_options(true)`
— `probe_ntp: true`. On macOS that runs:

```rust
let output = run_command_with_timeout("sntp", &[&server], NTP_TIMEOUT_SECS);  // time.rs:380
```

a subprocess spawn plus a **real network round-trip to `time.apple.com`**
(or `/etc/ntp.conf`'s server), with `NTP_TIMEOUT_SECS = 3`. Linux runs
`timedatectl` (local, but still a spawn); Windows runs `w32tm`.

**Measured impact.** `md compose --perf` attributes **59.6 ms** to "capture
context" on a document that uses no context at all — RUST_LOG tracing shows
the same 59.7 ms silent gap. That is ~47 % of the total 127 ms compose. On an
offline or firewalled machine every compose stalls up to **3 seconds**. The
`ntp_status` value is never surfaced by any `ctx.*` datetime key — the result
is computed and thrown away.

**Fix.** Call `sniff::os::detect_timezone_with_options(false)` in
`populate_datetime` (the timezone-abbreviation derivation is the only thing
Darkmatter needs, and it never uses NTP status). Keep sniff's bare
`detect_timezone()` behavior unchanged: its documented contract is the full
time-information convenience API, including NTP status, while
`detect_timezone_with_options(false)` and `OsRequest::summary()` are the
established local-only surfaces. Changing the convenience default would be a
cross-package semantic break unrelated to removing Darkmatter's unused work;
if desired, that belongs in a separate sniff specification with a caller audit.
Also correct the stale "zero-cost"/"purely local" capture comment in the same
change. Expected directional result: compose drops from ~127 ms toward ~70 ms
on the original host, with no `sntp`, `timedatectl`, or `w32tm` child spawned by
Darkmatter on macOS, Linux, or Windows.

---

## High

### 2. OSC 10 text-color query is uncached — every `Terminal` construction round-trips the tty

**Severity:** high (interactive/TTY runs; library hosts rendering many docs)
**Category:** blocking I/O
**Location:** `biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:93-100`, consumed at `biscuit-terminal/lib/src/terminal.rs:51`; hit from darkmatter at `render_tree/entrypoints.rs:574` and `layout/page.rs:1479-1481`

**Problem.** The OSC 11 background query is cached; the OSC 10 foreground
query is not:

```rust
pub fn bg_color() -> Option<RgbValue> {
    *BG_COLOR_CACHE.get_or_init(|| query::query_osc_color(11))   // cached
}
pub fn text_color() -> Option<RgbValue> {
    query::query_osc_color(10)                                    // NOT cached
}
```

`query_osc_color` opens `/dev/tty`, toggles raw mode, and polls in a 10 ms
sleep loop with a 100 ms timeout. Every `Terminal::new()`/`Terminal::default()`
pays one full round-trip (typically 10–30 ms, 100 ms worst case). Because
`is_tty()` is `stdout OR stderr`, this also fires when stdout is piped but
stderr is a terminal. `render_tree_terminal` constructs a fresh
`Terminal::default()` on **every render call** (`entrypoints.rs:573-574`), so
library callers (claudine) rendering many documents per process pay it per
document.

**Fix.** Add a `TEXT_COLOR_CACHE: OnceLock` mirroring `BG_COLOR_CACHE`
(ideally batch OSC 10+11 in one raw-mode session). Cache only the default-query
path; the explicit `*_with_timeout` APIs remain uncached. Do **not** cache a
whole detected `Terminal` in the Darkmatter library: it includes
current-directory, environment, stream-attachment, color-mode, and config-file
facts whose lifetime is construction-scoped. `Terminal::width()` and
`height()` are intentionally dynamic. For width-only call sites such as
`ambient_terminal_width`, call the dynamic dimensions detector directly. For
full rendering, add/use a caller-supplied-terminal path so long-running hosts
can reuse a snapshot intentionally, while the zero-config convenience path
retains fresh construction semantics. The exact lifetime of default OSC/color
probe caching is resolved in Open Question 1.

### 3. One CLI invocation constructs `Terminal::default()` up to four times

**Severity:** high (verbose compose), medium elsewhere
**Category:** repeated detection
**Location:** `darkmatter/cli/src/commands/compose.rs:294,505,522,580,592`; `cli/src/main.rs:82,119`; `cli/src/commands/frontmatter.rs:153`; `cli/src/commands/mod.rs:158-171`

**Problem.** `run_compose` builds a fresh `Terminal::default()` for the
verbose summary (line 505), the `-vv` perf metrics (line 522), the warnings
footer (line 580), and the deferred validation report (line 592). Each is the
*full* detection: the uncached OSC 10 round-trip (finding 2), a git-root
walk-up, terminal config-file font parsing (iTerm2 spawns `defaults read`
twice; Ghostty spawns `ghostty +show-config`), and connection/CI detection.
`md toc --json` also constructs a `Terminal` it never uses
(`commands/mod.rs:158-171` — built before the `json` branch).

**Fix.** Detect once per **CLI invocation**, after argument parsing, and pass a
borrowed terminal context to every human-rendered branch that runs in that
invocation. Do not use a process-global `LazyLock<Terminal>`: tests and library
hosts may change the current directory, environment, or attached streams
within one process. Construct lazily so pure Markdown/HTML/JSON paths do not
detect a terminal at all. Move `Terminal::new()` inside the non-JSON branch of
`toc`.

### 4. `Markdown::toc()` line numbers are O(n²)

**Severity:** high (empirically confirmed quadratic)
**Category:** algorithmic
**Location:** `darkmatter/lib/src/markdown/toc/mod.rs:210-211` (and `:232`)

**Problem.**

```rust
for (event, range) in parser.into_offset_iter() {
    let line_number = content[..range.start].lines().count() + 1;
```

The prefix scan runs for **every parser event** unconditionally — events that
never use `line_number` included. Event count grows with document size, so
the total is O(E × N). Measured: 81 KB → 203 ms, 326 KB → 2.24 s (4× size,
11× time), 1.3 MB → **45.3 s**. This sits under `md toc`,
`md hash --kind structured/detailed`, `md hash --diff/--save` explanations,
and the reference graph's heading indexes.

**Fix.** Precompute a sorted line-start offset table once and binary-search
only in event arms that consume a line number, or track the line incrementally
because pulldown-cmark offsets are emitted in source order. Preserve the shared
span contract: 1-indexed lines, byte offsets, `\r\n` counting as one line break,
and offsets clamped to the source. Prefer the incremental O(n) design when it
stays simple; the table design is O(n log n) and is acceptable if it is clearer.
Do not replace these calls with repeated `span::line_of_offset`, which is also a
prefix scan and therefore retains the quadratic behavior.

### 5. Schema stage resolves the effective schema twice and coerces twice per compose

**Severity:** high
**Category:** duplicated work
**Location:** `darkmatter/lib/src/markdown/compose/schema_validation.rs:112`, `:128`, `:146`; `schemas/mod.rs:507-520`, `:616`

**Problem.** `schema_validation::run` calls `schemas.effective_for(markdown)`
(line 112), then `schemas.validate(markdown)` (line 146) — and `validate`
internally calls `effective_for` **again** (`schemas/mod.rs:510`).
`effective_for` is the expensive end of the chain: `$schema` YAML parse,
grammar parse, import expansion (disk reads per import), example resolution
(disk reads + validator builds), and layer merging with deep clones. Only the
final validator compile hits the instance cache. Coercion also runs twice:
explicitly at line 128, then again inside `validate_with_positions`
(`schemas/mod.rs:616`) — the second pass is an idempotent no-op being paid for.
With triggers + shell expansion the whole `run()` executes a second time
(`pipeline/mod.rs:294`), quadrupling the resolution work.

**Fix.** `run()` already holds the `EffectiveSchema` — validate through it
directly (`effective.validate_with_positions(...)`; both helpers exist in
`schemas/mod.rs`) instead of calling `schemas.validate()`. Halves resolution
work and removes one full coercion pass with no behavior change.

### 6. Coercion compiles validators that are never cached — up to 2 × union-arms cold `jsonschema` compiles per compose

**Severity:** high
**Category:** missing cache
**Location:** `darkmatter/lib/src/markdown/schemas/coerce.rs:313` (`coerce_root_union`), `:436` (`coerce_property_union`)

**Problem.**

```rust
let Ok(validator) = build_validator(&wrapped, None, None) else {   // coerce.rs:313
    continue;
};
```

These run on **every** coercion pass — one compile per union arm until an arm
accepts, per property for property-level unions — and coercion runs twice per
compose (finding 5). The module's own docs price a compile at "several
milliseconds" (`schemas/validate.rs:13-16`). `effective_for` already compiles
and caches identically-wrapped arm validators via `build_arm_validators`
(`schemas/mod.rs:1118-1132`), so these would be near-guaranteed cache hits.

**Fix.** Thread the `ValidatorCache` (or the prebuilt `arm_validators`) into
`coerce_frontmatter_with_pending` / `coerce_property_union`.

---

## Medium

### 7. `md compose` walks and prepares the document tree three times (validate → preflight → compose)

**Category:** duplicated work — high impact for multi-file compositions
**Location:** `darkmatter/cli/src/commands/compose.rs:267-310`, `:399-424`, `:426`; `lib/src/markdown/reference/graph.rs:804-825`

The reference-validation pass builds a full graph and runs an InlinePre
`compose_with` (frontmatter interpolation, page blocks, interpolation, shell
scan) **per node**; preflight then does its own collection walk; the real
compose runs last. Only preflight→compose is deduplicated
(`with_preflight_graph`). The validation graph and its per-node prepared
content are discarded — interpolation/page-block work runs ~2× per node on
every `md compose <file>`. Fix: reuse the preflight graph for validation, or
share the `RunLocalCache`/prepared content between passes.

### 8. Validator cache is per-instance; compose creates a fresh `DarkmatterSchemas` (and re-runs trigger discovery) every invocation

**Category:** cache lifetime
**Location:** `compose/schema_validation.rs:76`, `:96`; `schemas/validate.rs:49-55`; `pipeline/mod.rs:215`, `:294`

`ValidatorCache` is documented "process-wide" but is per-instance, and
`run()` does `DarkmatterSchemas::new()` each time — so nothing amortizes
across composes, and the pipeline's own second `run()` (trigger re-assembly)
recompiles everything and repeats the full trigger filesystem scan (ancestor
walk + read/parse every trigger + resolve every payload; measured ≈ 12 ms on
a trivial tree). Fix: process-level `LazyLock` cache (the key already includes
`base_dir`); reuse the first `run()`'s registry for the second invocation via
the existing `with_trigger_registry` API. Fix the doc claim either way.

### 9. Baseline schema converted (and deep-cloned) per compose despite process-level caches existing

**Category:** missing cache reuse
**Location:** `compose/schema_validation.rs:80-81`; `schemas/mod.rs:128-140`, `:161-167`, `:236-243`, `:388`

`with_darkmatter_baseline()` deep-clones the parsed base schema out of its
`OnceLock`, and `with_baseline` re-runs the full `to_json_schema` conversion
per compose — the already-cached `BASE_JSON_SCHEMA` (`mod.rs:161-167`) is never
consulted on this path. `effective_for` then clones the baseline JSON into
`base_layers` per call — twice per `run()` (finding 5). Measured: the baseline
schema accounts for ~47 ms of wall on a trivial compose. Fix: feed
`with_baseline_json_schema(darkmatter_base_json_schema())` when the baseline
is the darkmatter base; hold layers as `Arc<Value>`.

### 10. Full ctx-values map deep-cloned on every frontmatter `ctx.*` lookup

**Category:** allocation
**Location:** `compose/context/runtime.rs:219-221`, `:255-256`; callers `frontmatter_interpolation.rs:95`, `:144`

```rust
pub(crate) fn get_effective(&self, key: &str) -> Option<serde_json::Value> {
    self.values_with_env_agent_overrides().get(key).cloned()
}
fn values_with_env_agent_overrides(&self) -> Map<String, Value> {
    let mut values = self.inner.values.clone();
```

Every `ctx.*` resolution — and every mere *presence test* in the ctx-typo AST
walk (`is_valid_context_variable`) — clones the entire captured context map
(which can include the repo group for a 48-member monorepo). Fix: memoize the
overridden map once (`OnceLock`) at construction; serve lookups by reference.

### 11. O(k²) re-cloning and re-parsing in the frontmatter interpolation fixpoint loop

**Category:** algorithmic
**Location:** `compose/frontmatter_interpolation.rs:457-529`, `:565-566`

Inside `loop { for key in &templated_keys { ... } }`:
`extract_frontmatter_key_refs` re-scans and re-parses every expression in the
key's value on every fixpoint pass (the refs never change — they derive from
`original_values`), and each resolved key clones the entire seed map plus the
`ResolutionContext` (which carries a full ctx-values map). k templated keys in
a dependency chain → O(k²) map clones and expression parses. Fix: precompute
refs per key before the loop; build one seed state and mutate incrementally.

### 12. `ResolutionContext` (with full ctx-values map) cloned on every expression function call

**Category:** allocation
**Location:** `compose/expression/mod.rs:687`; `context/effective_state.rs:374-376`; `context/options.rs:861-903`

`resolution_context()` returns `Some(self.resolution_context.clone())` — a
copy of `magic_paths`, `ctx_values` (itself a full map clone at construction),
and PathBufs — for **every** function call in every expression, including pure
functions like `length()` that never use it (the `dispatch_fs` probe runs
first). Fix: return `Option<&ResolutionContext>` or `Arc`; fetch only after
the fs-function name match confirms it's needed.

### 13. Text-replacement scanner is O(n × rules) with a per-compose char-index Vec

**Category:** algorithmic
**Location:** `compose/replacement.rs:92`, `:165-198`

At every character position all rules are probed with `starts_with` (no
first-byte bucket, no Aho-Corasick): 100 KB doc × 20 keys ≈ 2 M probes, plus
an ~8-bytes-per-char `char_starts` Vec per invocation — and this also runs
inside `render_code_transclusion` per code file. `apply_replacements` returns
`content.to_string()` even with zero matches. Fix: `aho_corasick`
leftmost-longest built once from the keys; return `Cow<str>`.

### 14. Body interpolation re-runs the pulldown-cmark parse 2-3× and always makes two full-body copies

**Category:** re-parsing
**Location:** `compose/interpolation/rewrite.rs:69-80`, `:104-110`, `:147`; `expression/lexer.rs:284-308`

`ExpressionFinder::new` runs a full `Parser::new_ext` pass to find code
regions; a doc with ≥1 replacement pays scan pass 1, rescan pass 2 (finds
nothing), and `convert_literals` (parse #3, plus an unconditional
`input.to_string()` even with zero `{{{ }}}` literals). Per-location
`replace_range` shifts the tail → O(n·m) for m expressions. Fix: return
expressions and literals from one scan; skip `convert_literals` when empty;
emit-between-spans instead of `replace_range`.

### 15. Parent document fully re-parsed once per `::file`/`::url` directive

**Category:** re-parsing
**Location:** `compose/transclusion/engine.rs:59-73`, `:1000-1002`, `:1418-1420`

`find_preceding_heading_level` runs `Parser::new(content).into_offset_iter()`
from byte 0 for each directive — d directives → O(d·n). Fix: extract parent
headings once per transclusion phase and binary-search by offset.

### 16. Preflight collection composes every visited document a second time

**Category:** duplicated work (every claudine-driven compose)
**Location:** `compose/preflight/collect.rs:241`, `:355`, `:375`, `:392-393`, `:509`

Discovery runs frontmatter interpolation + replacement + body interpolation
per visited doc (`compose_with(inline_options)`), then the terminal compose
repeats the identical work — the preflight graph caches only target
resolution, not interpolated content. Each `PreflightGraphNode` is also
deep-cloned into both `edges[i].child` and `children`, and `scan_one_frontmatter`
clones the whole doc. Fix: carry prepared content in the graph; store children
as indices or `Arc`.

### 17. Body `::shell` directives execute strictly serially with a 10 ms poll loop

**Category:** missing concurrency
**Location:** `compose/inline/shell_expansion.rs:41-58`; `shell_expansion/executor.rs:240-309`

Each body directive runs back-to-back, and the `try_wait` loop sleeps 10 ms
per poll — up to +10 ms latency per command. The frontmatter path already
parallelizes the same machinery with rayon
(`frontmatter_shell_expansion.rs:1407-1419`). Fix: reuse the prepare-serial /
execute-parallel split; replace the sleep-poll with a blocking `wait` on a
thread + channel timeout. (Ordering may be intentionally serial — the split
preserves output order.)

### 18. `md graph --validate` builds the reference graph twice (three times with `--fragments`), and fragment validation has no memoization

**Category:** duplicated work
**Location:** `lib/src/markdown/reference/file_tree/mod.rs:225-247`; `reference/validate.rs:315-318`, `:605`, `:616-625`, `:687-689`

`validate()` calls `build_reference_graph` internally rather than accepting
the already-built graph; `--fragments` builds it a third time and re-runs
`prepare_content_for_validation` per node with fresh `Markdown::try_from`
reads that bypass the `RunLocalCache`. `validate_cross_doc_fragment` re-reads
and re-composes the same target once per `path#fragment` reference. Fix: a
`validate_graph(&graph, ...)` entry point; cache prepared heading slugs per
canonical target path for the run.

---

## Low

### 19. `==` anywhere in the source triggers a second full document parse

**Location:** `render_tree/inline_extension.rs:241-260`, `:355`
`scan_delimiters` scans the raw source including code fences, and `a == b` is
ubiquitous in code — so most technical documents pay `protected_ranges` (a
complete second pulldown-cmark parse) only to discard every candidate as
protected. Fix: track fence state during the initial O(n) scan, or require a
plausible delimiter pair before parsing. *(Rated low because renders measure
fast in absolute terms; it's the largest single win on the render path after
finding 2.)*

### 20. Whole event stream collected into a Vec, then every text event re-allocated by `split_disclosure_directives`

**Location:** `render_tree/fold.rs:514-519`, `:768-769`, `:890-891`
For documents with zero disclosure directives (the common case), every
borrowed `Event::Text` is copied into a fresh boxed `String`, roughly doubling
text allocation per render, plus three `rest.find(dir)` scans per text event.
Fix: push the original `(event, range)` through unchanged when no directive
matches.

### 21. macOS: `defaults read -g AppleInterfaceStyle` spawned per `Terminal` construction in fully-redirected contexts

**Location:** `biscuit-terminal/lib/src/discovery/detection/color.rs:193-208`
`color_mode()` forks the subprocess whenever `bg_color()` is `None` — exactly
the both-streams-redirected case — uncached, multiplied by finding 3's
repeated constructions. Fix: cache per process; skip the spawn when not a TTY.

### 22. `md hash <dir>` walks vendored trees

**Location:** `lib/src/markdown/fs.rs:20-29`
Only dot-directories are pruned: `node_modules` (thousands of README.md) and
`target`/`vendor` are fully walked **and hashed into the aggregate** — slow
and probably not the intended fingerprint. Fix: skip well-known vendored dirs
or use `ignore`-based walking. (Per-file hashing is already rayon-parallel.)

### 23. syntect `Theme` cloned per code block

**Location:** `highlighting/themes.rs:527-531`; `render_tree/code_renderer.rs:243`, `:365`; `entrypoints.rs:649-651`
`THEME_SET.get(name).clone()` deep-copies the theme's scope-selector tables
for every code block (plus once per render for inline-code colors), and
`code_theme_from_env` re-reads env vars per block. Fix: hold
`&'static SyntectTheme` in `CodeHighlighter`.

### 24. Per-token `format!` + `push_str` churn in code-block emission

**Location:** `output/code_block.rs:101-104`, `:136-142`, `:273-279`, `:310-316`; `highlighting/mod.rs:189-193`
Every syntect styled range (5–20 per line) allocates a temporary `String`.
Fix: `write!(output, ...)` straight into the buffer — mechanical change in the
hottest per-line loop.

### 25. Cleanup pipeline: ~10 sequential full-document passes, including four back-to-back whole-string `String::replace` calls

**Location:** `markdown/cleanup/mod.rs:214-314`; `cleanup/emphasis.rs:111-115`
All passes are linear (no quadratic behavior found) but total ~10× the
document in traversals/allocations. Fix: fuse the four placeholder replaces
into one scan; fold line-based passes into one iterator. Not on the render
path — only `Markdown::cleanup*` / `md clean` / compose's Cleanup stage.

### 26. `canonical_hash` serializes the full schema + SHA-256 on every validator-cache lookup

**Location:** `schemas/validate.rs:148`, `:948-967`
Hit or miss, every lookup serializes the entire merged schema and SHA-256s it
(merged + N arms, ×2 per compose via finding 5). Fix: xxHash via
`biscuit-hash` (repo convention; accidental-collision resistance suffices),
hash once per stable schema.

### 27. Named-type imports re-read and re-parse the target file per import site; `@this` clones the whole namespace

**Location:** `schemas/resolve.rs:817-824`, `:871-873`, `:897-899`, `:909-929`
`A@types.yaml`, `B@types.yaml`, `C@types.yaml` → three reads + three full
parses of `types.yaml` (plus a `canonicalize` syscall each), repeated on the
second `effective_for` (finding 5). Fix: memoize namespaces per canonical path
on `ImportEngine`.

### 28. `example()` returns-target validator rebuilt per reference per resolution

**Location:** `schemas/resolve.rs:1271-1283`; `schemas/example.rs:224-252`
The envelope validation is content-hash memoized (good), but the target-schema
gate rebuilds a validator per `example(...)` reference on every resolution
(×2 per compose), and the example file is re-read each time. Mostly subsumed
by fixing finding 5.

### 29. `effective_for` clones every schema layer per call

**Location:** `schemas/mod.rs:386-393`, `:477-479`
Baseline JSON, trigger payloads, and document JSON are cloned per call — 2×
per `run()`, 4× with trigger re-assembly. Falls out of findings 5/8/9;
otherwise `Arc<Value>` layers.

### 30. `doc.*` namespace lookups rebuild the entire effective state as a `Value::Object`

**Location:** `context/effective_state.rs:182-184`; `frontmatter_interpolation.rs:88-91`
Every `doc.foo` reference clones the full merged state map. Fix: a map-based
resolver that walks by reference and clones only the leaf.

### 31. Every variable interpolation performs the state lookup twice

**Location:** `compose/interpolation/evaluator.rs:242-247`
The non-array arm discards the fetched value and re-runs the full lookup
(nested-path walk, ctx fallback, clones) via `get_string`. Fix: stringify the
first `get` result directly.

### 32. Per-directive snapshot clones whole whitelist/blacklist rule sets

**Location:** `shell_expansion/mod.rs:188`; `shell_expansion/types.rs:1060-1067`
Every non-pre-approved directive clones three rule collections for read-only
checks. Fix: check under the shared lock, or snapshot once per stage.

### 33. Remote-URL expression discovery duplicates the interpolation stage's scan, with O(n) line computation per expression

**Location:** `compose/pipeline/mod.rs:72-78`; `compose/remote.rs:287-307`
When remote reads are enabled, pipeline start full-parses and expression-parses
the document (work Interpolation repeats), and `byte_offset_to_line` is O(n)
per expression. Fix: single forward pass for lines; skip discovery when a
`memchr` probe finds no `http`.

### 34. Cleanup stage clones the whole body just to detect change

**Location:** `compose/pipeline/phases.rs:80`, `:114`
Full-body copy + full-body compare per compose purely for the
`cleanup_changed` report flag. Fix: compare xxHash before/after, or have the
cleanup passes report modification.

### 35. Assorted single-copy costs

- `effective_state_hash` re-canonicalizes the full state per `::file`
  directive (`transclusion/engine.rs:1316-1319`, `cache/hashing.rs:87-101`) —
  hash once per transclusion phase.
- `relevel_with_overflow` copies the whole child per heading;
  `extract_headings` counts lines from byte 0 per heading
  (`transclusion/engine.rs:149-154`, `:164`, `:197`).
- `RemoteFetchRuntime::get_content` clones the full response body per consumer
  (`remote_fetch.rs:408`) — store `Arc<str>`.
- `::toc-linking` targets read twice, bypassing the run cache
  (`reference/graph.rs:919` vs `:458`).
- `md hash --diff/--save` computes the document hash 2-3×, each rebuilding
  the TOC — finding 4 multiplies this (`cli/commands/hash.rs:151-186`;
  `hash/explain.rs:449-502`).
- `md delta` clones both full documents for the report
  (`cli/commands/mod.rs:189-190`).
- `normalize_body_rhythm` allocates an ANSI-stripped copy per output line,
  twice for trailing lines (`layout/page.rs:1244-1269`).
- `apply_link_policy`/`apply_image_policy` clone URL + title per link node
  even with an empty policy context (`render_tree/build_context.rs:375-379`,
  `:411-415`).

---

## Reference: measured pass/copy counts

**Compose (typical single-file, no transclusion):** ~8–10 full-body String
copies across stages (compose_with clone → replacement copy → interpolation
×2 → cleanup ×3–4 → normalization → link stages), plus ~4–6 full
pulldown-cmark re-parses (interpolation ×2–3, shell code regions,
transclusion parse, page-blocks/link stages). Noise at prompt sizes; dominant
at 100 KB+.

**Render (`as_terminal` / `DarkmatterPage::render`):** 1–2 full parses (2 only
when a `==` candidate exists — finding 19) + 1 build fold + 1 target fold. The
page path builds the Document once and reuses it (no re-parse, no re-fold) —
good.

**Schema (one compose with document `$schema`):** parse→convert→merge chain
×2 (×4 with triggers+shell), 1 merged + N arm cached compiles, up to 2 × N
**uncached** coercion compiles (findings 5/6).

---

## Quick wins

| # | Change | Location | Expected benefit |
|---|--------|----------|-----------------|
| 1 | `detect_timezone_with_options(false)` in datetime capture | `capture/datetime.rs:123` | −60 ms on every compose; removes offline 3 s stall |
| 2 | Cache OSC 10 like OSC 11 | `osc_queries/mod.rs:98-100` | −10–100 ms per Terminal construction on TTYs |
| 3 | Process-level `LazyLock<Terminal>` in CLI + lib entry points | `cli` + `entrypoints.rs:574` | removes 3–4 redundant full detections per command |
| 4 | Line-offset table in `toc()` | `toc/mod.rs:210` | O(n²) → O(n log n); 2.2 s → ms at 326 KB |
| 5 | Validate through the held `EffectiveSchema` | `schema_validation.rs:146` | halves schema-stage work |
| 6 | Thread `ValidatorCache` into coercion | `coerce.rs:313`, `:436` | removes up to 2×arms cold compiles per compose |
| 7 | Reuse cached `BASE_JSON_SCHEMA` for the default baseline | `schema_validation.rs:80` | large share of the measured ~47 ms baseline delta |
| 8 | `write!` instead of `format!`+`push_str` in code-block emit | `output/code_block.rs` | mechanical; cuts hottest-loop allocations |

## Suggested implementation order

1. **Finding 1** (NTP) — one-line semantic fix, biggest single win, removes a
   network dependency from a pure-local operation.
2. **Findings 2 + 3 + 21** (terminal detection caching) — coordinated change in
   biscuit-terminal + darkmatter entry points; largest interactive-latency win.
3. **Findings 5 + 6 + 9** (schema double-work) — self-contained in the schema
   stage; measurable on every schema-bearing compose. Then 8 (cache lifetime)
   as the follow-on design change.
4. **Finding 4** (toc O(n²)) — isolated algorithmic fix with a clear benchmark.
5. **Finding 7** (compose triple walk) — biggest architectural item; benefits
   claudine most; needs care around cache-key correctness.
6. Allocation-tier items (10–16, 19–20, 23–24) opportunistically, ideally with
   a compose/render criterion benchmark in place first.

## Benchmarking recommendations

- Add a criterion bench for `compose_with` on a fixture with frontmatter
  interpolation + `$schema` + one transclusion, and one for `as_terminal` on a
  code-heavy 100 KB doc — the two paths this review found regressions in are
  exactly the ones with no bench coverage today.
- Keep `md compose --perf` in the loop: it attributed the NTP stall precisely
  and is the right harness for validating fixes 1/5/6/7.
- Re-run the `toc` scaling test (81 KB / 326 KB / 1.3 MB) after fix 4; the
  1.3 MB tier currently takes 45 s.

## Good patterns observed (do not "fix")

- **Syntect loaded once**: `SYNTAX_SET`/`THEME_SET` behind `lazy_static`, with
  two-face lazy per-theme decode; pointer stability pinned by a test. No
  startup cost for `--help`/non-highlighting commands.
- **`HighlightLines` reused across lines** per code block; global
  `ScopeCache`; pre-sized output buffers throughout the code-block renderer.
- **Zero regexes on the render path**; all compose regexes are `LazyLock`
  statics.
- **Non-TTY render short-circuit**: piped `md <file>` skips all terminal
  detection; OSC 11 cached; OSC queries correctly skipped under
  CI/multiplexer/non-TTY.
- **Demand-driven context capture** skips sniff git/OS/hardware/GPU probes
  when no `ctx.*` group is referenced (the NTP probe in finding 1 is the one
  leak past this gate); `ComposeContext` is `Arc`-backed so child pipelines
  clone pointers.
- **`RunLocalCache`** (DashMap + condvar single-flight, persistent fallback,
  stale-serve) dedupes child loads; **`RemoteFetchRuntime`** single-flights
  URL fetches on one shared runtime with semaphore-capped concurrency, shared
  between preflight and compose.
- **Transclusion resolution and frontmatter shell expansion are
  rayon-parallel**; per-compose shell command memoization with volatile
  opt-out; reverse-order span replacement keeps offsets stable.
- **Preflight graph reuse** into the compose stage (`with_preflight_graph`).
- **Single-read document load**; `md hash` never builds an AST for fm/body/
  simple kinds; directory hashing is rayon-parallel; render output is one
  buffered `write_all`.
- **Perf instrumentation fully gated** (`perf.is_enabled().then(Instant::now)`).
- **Trigger matcher** is pure with thread-local glob and regex caches; trigger
  scan pre-resolves payloads once per registry.
- **`ValidatorCache` LRU** itself is well built (schema+base_dir key, build
  outside the lock, env-tunable capacity) — the gaps are instance lifetime
  (finding 8) and the paths that bypass it (findings 6, 26).
