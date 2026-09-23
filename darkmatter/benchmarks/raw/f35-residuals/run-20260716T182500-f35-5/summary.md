# Finding 35.5 — `md hash --diff` / `--save` shared artifact: retained evidence

Rebuilt for review-2's findings that (a) the checkpoint had no retained raw
samples and (b) **"the newly changed F35.5 implementation has no reproducible
measurement against its current code."** Both are confirmed and closed here.

**Headline:** the recorded **-18.0 % CLI** figure is *reproducible* but
*superseded* — it measured an implementation that no longer exists. The shipped
code is **-37.6 %** against pre-F35.5, not -18.0 %.

## The three states (why the recorded number measured the wrong code)

| State | Commit | `--diff` artifact computations | Status |
|---|---|---|---|
| **S0** | `8f604c5a3` | 2, or **3** for `detailed` | pre-F35.5 |
| **S1** | `540262812` | **2** (`compare_hash` + `explain_hash_diff`) | what the original `run-20260716T160000` record measured -> **-18.0 %** |
| **S2** | `b8ecb88cb` | **1** (`diff_hash`) | **current HEAD** — never measured until this run |

The original record's own *"Honest residual (NOT fixed, and why)"* section states
that at S1 `--diff` **still computed the artifact twice**, and that this is why
its `simple` / `structured` rows were unchanged. Commit `b8ecb88cb` then closed
exactly that residual. So the retained -18.0 % is a genuine **S0->S1**
measurement that no longer describes the shipped code.

## Run record

| Field | Value |
|---|---|
| Run id | `run-20260716T182500-f35-5` |
| Candidate commit | `a80e032c3` (HEAD, contains S2 = `b8ecb88cb`) |
| Baseline commits | `8f604c5a3` (S0), `7e7d6a61d` (S1 = `b8ecb88cb^`) |
| Library harness | `darkmatter/lib/benches/phase11_evidence.rs` -> `bench_f35_5_diff_hash` — **RETAINED** |
| Library command | `cargo bench -p darkmatter --bench phase11_evidence` |
| CLI harness | `hyperfine` against three release `md` binaries (the README's "release CLI runner" contract; CLI evidence is deliberately not forced through `just bench`) |
| Profile | release |
| Host | macOS Darwin 25.5.0, arm64, Apple M4 Max |
| TTY mode | piped (non-interactive) |
| Tools | rustc 1.96.0, cargo 1.96.0, criterion 0.5.1, hyperfine 1.20.0, bun 1.3.3 |
| Fixtures | `fixtures/toc_large.md` (80936 B) and `fixtures/hash_basic.md` (292 B) — manifest identities unchanged, **no new fixture registered** |

### Predeclared threshold (carried verbatim from the original record)

> Target operation: one `md hash --diff` / `--save` invocation.
> Minimum repeatable win: >= 5% on the large-document `detailed` target.
> Maximum permitted control regression: 0% on the `simple` and small-document
> shapes.

### Recomputation

```
bun recompute.ts raw/f35-residuals/run-20260716T182500-f35-5
```

Reads both retained formats in this record: the Criterion `*-sample.json` vectors
(library) and the hyperfine `cli-*.json` exports (CLI).

## Library evidence — current code (S1 -> S2), Criterion, n=100, means

Baseline and candidate are **both current public API**, so this needs no pinned
copy and no cross-build comparison: the baseline *is* the two-call path
`run_hash_diff` used at S1.

- baseline  = `compare_hash` + `explain_hash_diff`
- candidate = `diff_hash`

Every pair is **equivalence-gated before timing** on identical
`frontmatter_changed` / `body_changed` **and** an identical rendered explanation.

| Fixture / kind | Baseline | Candidate | Delta | CIs |
|---|---|---|---|---|
| `toc_large` / Simple | 427.39 us | **213.13 us** | **-50.1 %** | disjoint |
| `toc_large` / Structured | 4.2690 ms | **2.1247 ms** | **-50.2 %** | disjoint |
| `toc_large` / Detailed | 5.5128 ms | **3.1952 ms** | **-42.0 %** | disjoint |

The mechanism predicts the number: S1 computed the artifact exactly twice, so
removing one compute halves the sequence — **-50 % is what 2->1 must produce**,
and it is what was measured. `Detailed` lands at -42 % rather than -50 % because
its `detailed_body` alignment is not part of the duplicated artifact.

**Cross-validation:** the `toc_large`/Simple baseline re-measured at 427.39 us
against the original record's 426.3 us — 0.3 % agreement across two independent
runs, which isolates the delta to the code change rather than the host.

### `hash_basic` shapes — retained, not relied on

Vectors are committed, but dispersion swamps the effect (detailed baseline
40.70 us median with 31.35 us sigma and a 182 us max, captured under concurrent
load). Direction matches the large-document result; **no claim is made from
them.**

## CLI evidence — bracketed hyperfine, 100 runs/arm, means

Cross-build comparison, so the README requires a **drift bracket**: the candidate
is run on **each side** of the baselines and the observed drift bounds what
counts as a result.

Arm order: `candidate_A(S2)` -> `baseline_S0` -> `baseline_S1` -> `candidate_B(S2)`.
Host load held at **5.85-6.29** for all four arms (`cli-load.log`).

Equivalence gate: S0, S1 and S2 produce **byte-identical** `--diff` stdout and
exit status on all three inputs, verified before timing.

| Case | S0 (pre) | S1 (recorded) | S2 = HEAD | drift | S0->S2 | S1->S2 |
|---|---|---|---|---|---|---|
| `large_fm_detailed` **[TARGET]** | 17.61 ms | 14.67 ms | **10.99 ms** | 3.4 % | **-37.6 %** | **-25.1 %** |
| `large_fm_simple` [control] | 5.64 ms | 5.75 ms | 5.20 ms | 4.7 % | -7.8 % | -9.6 % |
| `small_detailed` [control] | 4.88 ms | 5.33 ms | 4.91 ms | 3.8 % | +0.6 % (< drift — parity) | -7.9 % |

Inputs per the original record's recipe: `toc_large.md` prefixed with a 3-line
frontmatter block so a stored hash property can live in it (a CLI-runner input,
**not** a new manifest fixture — the immutable fixture bytes are unchanged and
still hashed), each seeded with `md hash <f> --kind <simple|detailed> --save`.

### Findings

1. **The recorded -18.0 % REPRODUCES as an S0->S1 measurement.** Measured here:
   17.61 -> 14.67 ms = **-16.7 %**, inside the original's stated `17.2 +/- 0.5`
   -> `14.1 +/- 0.7` sigma. The original was honest work; it simply describes a
   superseded implementation.
2. **It materially UNDERSTATES the shipped code.** The current implementation is
   **-37.6 %** against pre-F35.5 (17.61 -> 10.99 ms), because `b8ecb88cb` closed
   the residual the original recorded as unfixable. Quoting -18.0 % for the
   shipped code is the honest correction this run exists to make.
3. **No control regressed.** `small_detailed` is at parity (+0.6 %, inside the
   3.8 % drift). `large_fm_simple` *improves* (-7.8 %); a control that gets
   faster cannot breach a 0 %-regression budget. Its improvement is expected —
   `b8ecb88cb` halves the `simple` library sequence too (-50.1 % of 427 us
   ~= 214 us saved against a ~5.2 ms command ~= 4 %).
4. **The >= 5 % target floor is met** at -37.6 % (or -25.1 % counting only the
   currently-unmeasured commit), against 3.4 % drift.

## Open — NOT resolved by this run

**The S2 mechanism adds public API.** `Markdown::diff_hash` and
`Markdown::plan_hash_save_explained` are new public inherent methods, which
review-2 flags as violating compatibility invariant 2 ("no new public Rust API
shape"). The measurement above is real and the mechanism is sound, but the
*shape* awaits an owner ruling — keep the operation crate-private behind a
non-public seam, or record an explicit exception. This run is **evidence only**
and changes no production code.

## Rejected / superseded runs retained beside this one

- `invalid-run-load46-69/` — rejected by its own drift bracket at host load
  46->69. Retained as the negative control proving the quiet-host run was needed.
- `superseded-3arm-load11/` — an earlier 3-arm attempt whose controls failed the
  drift gate.

## Cross-platform

OS-identical, confirmed from the diff: pure call-graph restructuring in
`compare.rs` / `explain.rs` / `save.rs` — no `cfg`, no filesystem access (the CLI
still owns `fs::write`), no path handling, no clock. Windows compile + this macOS
behavioral run + ordinary Linux CI are sufficient per the Verification Matrix.
