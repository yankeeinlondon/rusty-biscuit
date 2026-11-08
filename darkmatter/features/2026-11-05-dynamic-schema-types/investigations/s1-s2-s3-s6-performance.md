# Performance spikes S1, S2, S3, S6 — 2026-11-05-dynamic-schema-types

Run date: 2026-11-07. This is a quick sample on one host, with no statistical gate, as the spec asks.
Nothing in the repository was edited. The prototype and drivers live under
`/private/tmp/claude-501/-Volumes-coding-wt-rusty-biscuit-feat-better-research/39b9d345-4342-48e4-a772-fbc4d9855cca/scratchpad/perf/`:

- `src/main.rs`: the bench crate. It has path dependencies on `darkmatter` (with `work-counters`), `simplified-schema`,
  `biscuit-file`, and `biscuit-hash`, and builds into its own `target/`.
- `driver.py`: the load-gated, interleaved driver.
- `lsp_client.py`: a stdio LSP driver for DMLS.
- `samples.jsonl`: every raw sample, with its load average.
- `summarize.py`: produces the tables below.

## Headline

- **No slot position fails the budget.** No measurement gives grounds to narrow any position.
- **Cold:** a slot adds about 13–30 µs at the raw read/hash/parse level, or ≤ 91 µs at the evaluator's upper bound, for
  `os.yaml`. A 500-member data file adds 0.13–0.57 ms. Compare that with a cold `md schema validate` of 42 ms, a cold
  `md compose` of 58–334 ms, and a DMLS first-diagnostics time of 84–142 ms. In every surface that is ≤ 1.4 %.
- **Warm:** a process-wide cache hit costs about 0.04 µs per document when the snapshot is pinned for the command. A
  `stat` re-check costs 2.25 µs per document, which is 1.8 % of the 125 µs real per-document validation. Rehashing on
  every use costs 20.6 µs per document (16 %), so it is rejected as the default.
- **DMLS:** serving a published snapshot from memory costs about 4 ns. Today's completion and hover take 1.8–9.1 ms, so
  the added latency is under 0.001 %. A background refresh costs 2 µs (`stat` check) or 91 µs (full re-resolution).
- **S6 (lazy or partial resolution) is not a material win.** On the warm path it saves nothing. On a cold resolution it
  saves 1.7 % (unreached slots). Compiling the static parts once and substituting only the dynamic sub-validator could
  save about 1.8 ms per re-resolution, but only on the invalidation path and at real architectural cost.

## Host and method

| Fact | Value (from `sniff cpu/memory/os`) |
|---|---|
| Chip | Apple M4 Max, arm64 |
| Cores | 16 logical / 16 physical |
| RAM | 128 GB (102 GB available at the start) |
| OS | macOS 27.2 (Darwin 27.2.0), host `shazam.home` |
| Binaries | `target/release/md` and `target/release/dmls`, built from the current worktree (uncommitted changes included) before any timing; the bench crate is built `--release` |

**Load discipline.** All compiling finished before timing began. Before each batch, the driver polled `vm.loadavg`
once a minute until the 1-minute load was below 4.

- At the start, load was 15.6 (5-minute load 23). The driver **waited 7 minutes** before the CLI and lib batches.
- The gate passed at 3.72 / 3.73 / 3.73 / 3.51 / 3.58 for the five batches.
- Load drifted to **4.1–4.3 during the CLI batch**. Treat those CLI numbers as *mildly contended*; the spread is small
  (see the min/max columns). Every other batch ran at 3.3–3.7.
- The git-index figures in §S1.4 were taken earlier, at load 5.5, and are flagged as **contended**.

**Interleaving.** Every repetition runs each case once in a fixed rotation (A/B/A/B). Tables give median [min–max]
across repetitions:

- CLI and lib: n = 9.
- S2, DMLS, S3, and S6: n = 7.

Inside each in-process sample, warm figures are themselves medians of 21–51 inner iterations.

**What "dynamic" means here.** Today a `{{ }}` slot cannot load (it fails to parse), so a dynamic resolution is
simulated:

1. **Template read:** read and hash the template file with `biscuit_hash::xx_hash_bytes`, in one pass.
2. **Slot evaluation:** a missing-file probe, then read and hash `os.yaml`, parse it as YAML, and check that it is
   `string[]`. These are the observations S4 says the real read set lacks.
3. **Substitution:** insert the members into the template. The prototype does this textually; the real design
   inserts into the typed representation, which costs the same or less.
4. **Lowering and compile:** `SimplifiedSchema::try_from_yaml(...).to_json_schema()`, then
   `ValidatorCache::structural_validator_for`.

The real evaluator's cost is measured separately. It composes `{{ as_csv(load_data("./os.yaml","string[]")) }}` with a
fresh `DataRuntime` and subtracts a trivial composition. That gives an **upper bound** for evaluator overhead.

**Fixtures:**

- **Static baseline:** `sandboxing.yaml` copied with `enum(macOS, Linux, Windows; required)` inlined. Its template
  twin uses `enum({{ load_data("./os.yaml", "string[]") }}; required)`.
- **Larger shipped schema:** the real `claudine/docs/research/sandboxing/providers/_schema.yaml`, used in place. It
  imports `_types.yaml` and `content-policy.yaml`, lowers to 14.7 KB of JSON Schema, and is validated with the real
  `claude.md`.
- **Larger data file:** a generated list of 500 strings.

## S1. Baseline and budget

### S1.1 CLI (cold process; load 3.7–4.3, mildly contended)

| Case | Wall ms | `--perf` schema validation ms | compose pipeline ms | build options ms |
|---|---|---|---|---|
| `md schema validate` sandbox (static) | 42.5 [40.0–48.9] | — | — | — |
| `md schema validate claude.md` (provider) | 166.2 [163.0–200.6] | — | — | — |
| `md compose --perf` sandbox (static) | 58.4 [56.8–59.5] | 0.333 [0.317–0.380] | 10.4 [10.1–11.0] | 34.8 [33.9–35.9] |
| `md compose --perf claude.md` | 333.7 [325.9–358.8] | 3.80 [3.80–3.90] | 93.5 [91.9–112.1] | 152.4 [149.8–157.7] |

Schema validation is 0.6 % of the sandbox compose wall time and 1.1 % of the provider compose wall time. Command setup
(context capture, reference validation, options) dominates.

### S1.2 In-process library (`DarkmatterSchemas::validate`; load 3.73)

"Cold" means after `reset_process_caches_for_bench`, with process first-use (statics) primed and excluded. "Warm"
means a fresh `DarkmatterSchemas` with warm process caches.

| Case | Cold µs | Warm µs | Validator compiles (cold, work counter) | Stage: load+lower µs | Stage: compile µs | Stage: validate µs | Slot µs |
|---|---|---|---|---|---|---|---|
| sandbox static | 580 [503–960] | 137 [128–204] | 4 | 68.8 | 13.3 | 0.08 | — |
| sandbox + slot | 495 [454–702] | 125 [121–153] | 4 | 65.3 | 13.0 | 0.04 | 29.8 [25.6–44.7] |
| provider static | 2973 [2894–3148] | 1009 [996–1026] | 12 | 600 | 620 | 20.6 | — |
| provider + slot | 2925 [2837–3101] | 1008 [996–1179] | 12 | 595 | 650 | 20.5 | 27.2 [26.6–32.9] |

The static and dynamic cold medians differ by less than the run-to-run spread. The slot's 27–30 µs sits inside the
noise of a 0.5–3 ms cold validation. Process first use, which runs once per process, was 0.8–1.1 ms.

### S1.3 Resolution breakdown (prototype `resolve`; µs; load 3.73)

| Schema | Template read | Slot eval | Substitute | Lower | Compile | Total |
|---|---|---|---|---|---|---|
| sandbox static | 14 | — | — | 51.5 | 8.5 | 73.0 [69.6–75.1] |
| sandbox + `os.yaml` slot | 14 | 13.4 | 0.25 | 52.3 | 8.5 | 86.5 [83.6–88.2] |
| sandbox + 500-member slot | 14 | 128.2 | 4.8 | 231.3 | 32.9 | 405.1 [402.7–417.0] |

| Real evaluator (compose upper bound) | µs |
|---|---|
| trivial composition | 170.6 [166.3–175.8] |
| `load_data(os.yaml)` composition | 258.1 [248.5–345.3] → **net ≤ 91 µs** |
| `load_data(500 members)` composition | 741.8 [606.8–813.0] → **net ≤ 574 µs** |
| raw read+hash+parse, `os.yaml` / 500 members | 17.0 / 149.1 |

For the 500-member case, lowering and compile grow with the member count. A static 500-member enum would pay the same
cost, so the slot-specific part is the 133 µs of evaluation and substitution.

### S1.4 Repository-cleanliness cost for S5 (contended, load 5.5; upper bound through the git CLI, including about 1.6 ms of process spawn)

| Operation | ms |
|---|---|
| Index size / tracked files | 2.95 MB / 19,614 |
| `git ls-files --stage -- <file>` (index parse) | 5.1 ±0.1 |
| `git rev-parse HEAD:<file>` | 4.6 ±0.1 |
| `git hash-object <file>` | 3.8 ±0.1 |
| `git status --porcelain -uall` (full walk) | 109 ±2.4 |

An in-process index read plus a HEAD lookup should cost roughly 3 ms per request on this monorepo. A status walk costs
about 30× more. This supports S5's `path_cleanliness` and `verify_blob` design.

### S1.5 Proposed budgets

The spec says these are comparisons, not pass/fail gates. They are also the only grounds for narrowing a position.

| Budget | Definition | Measured | Verdict |
|---|---|---|---|
| **B1: warm overhead** | Per-document added cost of a dynamic schema on a cache hit ≤ **5 %** of the warm static per-document validation (125 µs real path) | pinned 0.04 µs (0.03 %); `stat` 2.25 µs (1.8 %); rehash 20.6 µs (16 %) | pinned and `stat` **pass**; per-use rehash **fails** and is not used |
| **B2: cold slot cost vs compile** | Slot-specific cold cost (evaluation + substitution + fingerprint) ≤ **2×** the lower+compile of the resulting concrete schema | `os.yaml`: raw 13.6 µs (0.2×), evaluator ≤ 91 µs (≤ 1.5×) vs 61 µs. 500 members: raw 133 µs (0.5×), evaluator ≤ 574 µs (≤ 2.2×) vs 264 µs | pass. The 500-member evaluator upper bound is just over, but it includes composition overhead; the raw cost passes at 0.5× |
| **B3: surface cold time** | Cold slot cost ≤ **1 %** of the surface's cold time | 91 µs vs 42 ms validate (0.2 %), 58 ms compose (0.16 %), 84 ms DMLS first diagnostics (0.1 %). 574 µs vs the same: ≤ 1.4 % / 1.0 % / 0.7 % | pass (500 members via `validate` is at the boundary, at the upper bound) |
| **B4: DMLS request path** | No I/O or evaluation on completion, hover, or diagnostics requests; added latency ≤ **0.1 ms** | 4 ns snapshot read | pass |
| **B5: DMLS refresh** | A background refresh does not exceed a cold validation of the same schema today | 2 µs check / 91 µs re-resolve vs DMLS cold validation of 45–66 ms (log) | pass |

**Work limit for S5.** The worst measured slot was 0.57 ms (500 members, evaluator upper bound). A per-slot wall
budget of about 25 ms in DMLS, which is roughly 40× headroom, combined with S5's visit, byte, and candidate caps,
cannot bind on realistic data. It would stop a pathological schema well before a request notices it. DMLS logs whole
milliseconds, so finer tuning needs the library counters.

## S2. Input-fingerprinted resolution cache

**Prototype.** The cache is process-wide:

- **Key:** schema content hash × context hash, mapping to `Arc<Snapshot{json: Arc<Value>, validator: Arc<Validator>, observations}>`.
- **Observations:** `File{path,len,mtime,xxhash}` (hashed while reading), `Missing{path}`, `Context{key, hash}` (never
  the value text), and `Catalog{version}`.
- **Re-check modes:**
  - *pinned:* trust the snapshot within a command.
  - *stat:* compare length and mtime, and rehash only on a mismatch.
  - *rehash:* read and hash every recorded file on every use.

**Workload.** A Claudine-style run of 500 documents in rotation across three schemas:

1. sandbox, the dynamic one;
2. the real provider `_schema.yaml`;
3. a small static schema.

The real per-document denominator is `DarkmatterSchemas::validate` over the same three real documents.

| Case (n = 7, load 3.5–3.7) | Total µs | Cold (3 misses) µs | Hits / misses | Dynamic-hit lookup median µs | Lookup Σ µs | Validate Σ µs |
|---|---|---|---|---|---|---|
| all static, pinned | 6879 [6714–7261] | 3366 | 497 / 3 | 0.04 | 7 | 3472 |
| dynamic, pinned | 6927 [6778–7208] | 3435 | 497 / 3 | 0.04 | 7 | 3448 |
| dynamic, `stat` | 7315 [7176–7710] | 3425 | 497 / 3 | **2.25** [2.21–2.71] | 382 | 3477 |
| dynamic, rehash | 10502 [10211–11319] | 3383 | 497 / 3 | **20.6** [20.5–23.0] | 3427 | 3585 |
| real path, `DarkmatterSchemas::validate` × 500 | 204561 [202841–210784] | — | — | — | — | 125 µs/document median |

| On-disk tier (n = 7) | µs |
|---|---|
| Write snapshot JSON | 33.9 |
| Disk hit: read + `stat` re-check + JSON parse | 40.3 |
| Disk hit + validator compile (validators cannot be persisted) | 49.4 |
| Compare: full re-resolution of the same template | 86.5 |

- **Hit rate.** 99.4 % (497 of 500); the only misses are one per schema. A warm hit returns the **same `Arc<Value>`**
  (`Arc::ptr_eq` holds), so the MERGES/phase memos and the validator cache keep working, as S7/S8 require.
- **Overhead.** Dynamic-pinned is indistinguishable from all-static: +0.7 % total, inside the spread. Dynamic-`stat`
  adds 1.1 µs per document averaged over all 500 documents, or 2.25 µs per dynamic hit. Rehash adds 6.9 µs averaged,
  or 20.6 µs per dynamic hit.
- **Recommendation: process-wide only, no on-disk tier.**
  - A disk hit saves about 37 µs per process (49 µs vs 86 µs). `md` process setup costs 40–330 ms, and the disk tier
    brings persistence, permission, and secret-hygiene obligations (spec: never persist secret-derived values), so it
    is not worth it.
  - Re-check policy:
    - Within one command, pin the snapshot (Q7's memo invalidation marks it stale on a recorded write).
    - Across requests in a long-running host (Claudine), re-check with `stat`, falling back to a rehash only when
      length or mtime differs.
    - Never rehash on every use.
  - Because `stat` cannot see a same-size write within mtime granularity, the hash stays the authority whenever a
    `stat` mismatch occurs. Hosts that need stronger guarantees can rehash on an explicit refresh.
  - Key the validator on the concrete snapshot content (S4 (c)), so a re-resolution that yields the same members
    skips the compile.

## S3. Invalidation and DMLS

**How DMLS learns of changes today** (code reading, not timed):

- `dmls/src/workspace/watch.rs` registers client watchers only for the configured include globs
  (`**/*.md`, `**/*.markdown`) plus `YAML_GLOB`/`YML_GLOB` (`**/*.[yY][aA][mM][lL]`, `**/*.[yY][mM][lL]`).
- `router.rs:1673-1743` registers schema-location watchers. When the client rejects watchers or cannot root them, it
  falls back to rescanning on save.
- A `.json`, `.toml`, or `.csv` data file, a missing candidate, or `.git/index`/`HEAD` (needed for S5's clean-file
  grant) produces no event today.

**Recommendation:**

- Register watchers **per recorded dependency path**, regardless of extension, from the snapshot's observations: hits,
  `Missing` probes, imports, and `<gitdir>/index` and `HEAD`. Use relative patterns where the client supports them.
- Where watchers are unavailable, use the existing save/rescan fallback, plus a `stat` fingerprint check of the
  open documents' snapshots on save and on focus. That costs 2 µs per snapshot.
- Unsaved data or schema buffers go through `SourceBuffers`.
- Environment and context changes need an explicit refresh, as the spec says.
- Run resolution on a dedicated lane, never in `RegistryJob::Document`; requests wait on that job (S5 F8).

**Measurements** (n = 7 sessions × 11 requests, load 3.3–3.7, `debounce_ms = 0`, `slow_validation_ms = 0`):

| Session | First diagnostics after didOpen, ms | Completion RTT ms | Hover RTT ms | didChange → diagnostics ms | DMLS log, first validation: resolve / compile / total ms |
|---|---|---|---|---|---|
| sandbox static (scratch workspace) | 84.0 [53.8–85.9] | 1.79 [1.76–1.89] (min 1.66, max 2.39) | 1.77 [1.73–1.86] | 3.83 [3.78–4.11] | 54 / 10 / 66 (cache miss) |
| provider `claude.md` (in repo) | 142.1 [125.7–149.1] | 9.11 [9.02–9.33] (5 enum items) | 8.70 [8.63–8.93] | 50.2 [49.5–51.4] | 28 / 14 / 45 (cache miss); later 1 / 0 / 3 (cache hit) |

| Snapshot serving (prototype, n = 7) | Value |
|---|---|
| Read of the published `RwLock<Option<(Arc<Snapshot>, stale)>>` | **4.2 ns** [4.2–5.0] |
| Background `stat` fingerprint check (5 observations) | 2.08 µs |
| Background rehash check | 19.3 µs |
| Background full re-resolution + atomic publish | 91.0 µs [89.0–91.5] |

- **Added request latency.** Serving the snapshot from memory adds about 4 ns to completion or hover. Against today's
  1.8–9.1 ms that is about 0.0002 %.
- **No I/O or evaluation on the request path.** This holds by construction in the prototype: the request reads one
  `Arc` and a stale flag. The spec's I/O-free request-path guarantee for the real DMLS must be proved by the
  implementation's tests. I timed today's request path but did not audit it for I/O.
- **Refresh cost.** A refresh costs 0.1 % of a cold DMLS validation (45–66 ms logged). A refresh could run on every
  save without being noticed.
- **Granularity caveat.** DMLS's timing logs report whole milliseconds, so sub-millisecond resolution cost will show as
  `0`. Add a slot-resolution stage (and work counters) to the validation event when this lands.

## S6. Lazy and partial resolution

**Setup.** The real provider `_schema.yaml` was made dynamic at three enum positions (`support`, `status`,
`enforcement`), each loading its own data file. Three strategies were compared:

- **(a)** resolve all three slots, then lower and compile;
- **(b)** resolve only the one slot a document reaches, then lower and compile the whole schema;
- **(c)** compile the static parts once, and compile only the dynamic enum as a separate sub-validator per
  resolution.

n = 7, load 3.58.

| Strategy | Slot eval µs | Lower µs | Compile µs | Total µs |
|---|---|---|---|---|
| (a) resolve all 3 | 49.9 | 1252.5 | 544.0 | ~1846 |
| (b) resolve reached 1 | 18.5 | 1247.2 | 546.4 | ~1812 (−1.7 %) |
| (c) dynamic sub-validator only | 18.1 (slot eval + compile of `{"enum": [...]}`) | — | — | 18.1 per re-resolution, plus a one-time ~1.8 ms static compile |

- **(b) is not a material win.** It saves 31 µs (1.7 %), because lowering and compile of the whole schema dominate
  either way, and it applies only to cold or invalidated resolutions. It also conflicts with "one schema resolves once
  for every document": per-document slot sets would fragment the cache key and the validator sharing.
- **(c) is the only notable saving, about 1.8 ms per re-resolution, and only on the invalidation path.** Warm hits
  already skip lowering and compile entirely. Getting it means changing a single compiled `jsonschema` document into a
  composite validator, and a separate structural pass for each slot's property. That has implications for positions,
  diagnostics, and `$defs`, and touches the extraction boundary S7 is drawing.
- **Recommendation: resolve the whole schema up front.** Do not pursue lazy or partial resolution. The saving is not
  worth the cache and diagnostic complexity at these magnitudes. If a future schema with heavy lowering is edited
  constantly in DMLS, revisit (c) as a refresh-path optimization only. The S2 rule that the validator is keyed on
  concrete content already removes recompiles when data changes don't change members.

## Does any slot position fail the budget?

**No.** Every measured cold cost is ≤ 1.4 % of its surface's cold time, even at the evaluator upper bound with a
500-member data file. Every warm cost is ≤ 1.8 % with the recommended `stat` re-check, or about 0 when pinned. The
DMLS request path adds nanoseconds.

The slot cost depends on the data the slot loads and on lowering the resulting values, not on where the slot sits.
The `literal`, `default`, `min`/`max`, and `pattern` positions substitute one scalar, which is cheaper than an enum
list. A whole-type slot pays one passive type-definition parse plus lowering, which is the same work as a static type
of that size.

The performance spikes therefore give **no grounds for narrowing** any S8 position. Any narrowing would have to rest on
implementation sequencing or safety, which belong to S7/S8 and S5, not on performance.

## Caveats

- One host, one quick sample. CLI numbers were taken at load 4.1–4.3, slightly above the gate; the git-index numbers
  at 5.5.
- Dynamic resolution is simulated; the evaluator figure is a compose-based upper bound.
- In the provider schema, the 12 cold validator compiles include baseline and trigger schemas, which a slot does not
  affect.
- DMLS completion inside the sandbox enum list returned no items at the positions tried (an empty list item, or after
  `- `). The sandbox session timed completion of a boolean value instead; the provider session timed a 5-item enum
  completion. This looks like an existing completion gap for array items of a named `enum` type, worth checking
  separately.
