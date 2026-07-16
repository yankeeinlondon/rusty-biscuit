# Cumulative closeout — run summary

Contract declared **before** capture in [`declared-contract.md`](./declared-contract.md);
read it first — it fixes the case matrix, commands, thresholds, and controls.

- run id: `run-20260716T050518`
- host: Apple M4 Max (`Mac16,5`), macOS 26.5.2, Darwin 25.5.0 arm64
- toolchain: rustc 1.96.0 (ac68faa20 2026-05-25), release profile, isolated
  `CARGO_TARGET_DIR` per pin, all pins built + measured in one session
- harness: `hyperfine 1.20.0 --warmup 3 --runs 20 --shell=none`, `NO_COLOR=1`,
  stdout+stderr piped (non-TTY)
- load average during capture: 4.99–7.75 (1 min) — a deliberate low-load window;
  Phase 10 recorded that this host reaches ~29–30 under concurrent indexing,
  where cross-run comparison becomes unsound. All pins are interleaved *within*
  each `hyperfine` invocation, so they share thermal/scheduling conditions.
- statistic: mean ± stddev (matching the Phase-2 F4 record so both tables read together)
- raw per-case JSON retained beside this file; attribution JSON under `attribution/`

## Result — complete manifest, pre-optimization → final feature head

| case | pre `83aaecc8f` | audit `51c1f16e1` | head | pre→head | audit→head |
|---|---|---|---|---|---|
| `help` | 4.65 ± 0.14 | 5.06 ± 0.50 | 4.59 ± 0.30 | **-1.2%** | -9.3% *(noise)* |
| `render_basic` | 4.69 ± 0.52 | 4.36 ± 0.20 | 4.28 ± 0.19 | **-8.7%** | -1.8% *(noise)* |
| `render_code_heavy` | 5.02 ± 0.56 | 5.04 ± 0.27 | 4.76 ± 0.20 | **-5.0%** | -5.5% *(noise)* |
| `hash_basic` | 4.77 ± 0.49 | 4.40 ± 0.32 | 4.25 ± 0.16 | **-10.7%** | -3.3% *(noise)* |
| `compose_trivial` | 86.17 ± 6.26 | 10.16 ± 0.48 | 13.69 ± 0.91 | **-84.1%** | +34.8% |
| `compose_schema_transclusion` | 106.35 ± 6.64 | 15.88 ± 0.86 | 19.56 ± 0.63 | **-81.6%** | +23.1% |
| `compose_transclusion_heavy` | 236.23 ± 5.43 | 51.37 ± 1.79 | 58.57 ± 1.44 | **-75.2%** | +14.0% |
| `compose_interpolation_heavy` | 93.75 ± 9.16 | 14.60 ± 1.12 | 17.27 ± 0.36 | **-81.6%** | +18.3% |
| `replace_heavy` | 149.58 ± 5.27 | 67.91 ± 0.72 | 61.50 ± 0.83 | **-58.9%** | -9.4% |
| `remote_heavy` | 307.48 ± 7.63 | 182.36 ± 3.37 | 177.82 ± 4.06 | **-42.2%** | -2.5% *(noise)* |
| `toc_small` | 10.29 ± 0.61 | 4.97 ± 0.21 | 4.93 ± 0.37 | **-52.1%** | -0.7% *(noise)* |
| `toc_medium` | 12.14 ± 0.47 | 5.76 ± 0.88 | 5.42 ± 0.18 | **-55.4%** | -5.9% *(noise)* |
| `toc_large` | 148.30 ± 3.93 | 9.36 ± 0.38 | 8.80 ± 0.50 | **-94.1%** | -6.0% *(noise)* |

## Disposition

**Cumulative claim (threshold 2) — PASS.** `toc_large` retains **-94.1%**
(148.30 ± 3.93 → 8.80 ± 0.50 ms) against a declared ≥90% floor, so the
non-quadratic `line_at_offset` result that Findings 4 and 35.2 both rest on
holds at the final head. Every one of the 13 cases is at or better than the
pre-optimization baseline; the compose family lands **-75% to -84%** and
`remote_heavy` **-42.2%**.

**Control honesty (threshold 3) — controls are flat.** `help` (-1.2% pre→head,
and within noise audit→head) and `render_basic` (-8.7% pre→head, within noise
audit→head) did not move materially, so — unlike the Phase-9 checkpoint — there
is **no build-drift caveat to discount** from this table. The deltas below are
attributable to code.

**Regression gate (threshold 1) — FAILED, and the failure is real.** Four
compose cases regressed **out of noise** against the audit commit:
`compose_trivial` +34.8%, `compose_schema_transclusion` +23.1%,
`compose_interpolation_heavy` +18.3%, `compose_transclusion_heavy` +14.0%
(dispersion is tight — e.g. 10.16 ± 0.48 → 13.69 ± 0.91 ms). This was
investigated rather than narrated, per the declared contract.

### Attribution — the regression is NOT this feature's

`audit → head` is **not** this follow-up's diff. Only two *code* commits landed
between them (`git log 51c1f16e1..HEAD` — everything else is documentation), and
**both belong to the linked Opaque Reference Graph feature**:

- `a8e5e98d9` — *refactor(darkmatter): coordinate graph and cache identity*
- `16ed1e57a` — *feat(darkmatter): wire prebuilt-graph guard and reference-graph benches*

Splitting the interval isolates it (`attribution/`, same harness, interleaved):

| case | audit `51c1f16e1` | clean head `b425fb466` | working tree (this feature) | audit→clean | clean→worktree |
|---|---|---|---|---|---|
| `compose_trivial` | 10.83 ± 0.35 | 13.72 ± 0.41 | 13.75 ± 1.10 | **+26.7%** | +0.2% |
| `compose_schema_transclusion` | 16.64 ± 0.44 | 20.33 ± 1.02 | 19.99 ± 0.43 | **+22.2%** | -1.7% |
| `compose_transclusion_heavy` | 51.60 ± 1.61 | 58.50 ± 1.40 | 57.94 ± 0.99 | **+13.4%** | -1.0% |
| `compose_interpolation_heavy` | 14.80 ± 0.61 | 18.15 ± 1.13 | 17.24 ± 0.32 | **+22.6%** | -5.0% |
| `render_basic` (control) | 5.09 ± 0.61 | 4.70 ± 0.22 | 4.81 ± 0.28 | -7.7% | +2.4% |

`clean head → working tree` **is** this feature's Phase 1–10 diff. It is **flat
or improving on every case** (-5.0% to +0.2%); `compose_interpolation_heavy`
-5.0% is the F11/F14 interpolation work. **This follow-up introduces no compose
regression.** The entire +13–27% arrived with the two reference-graph commits.

A finer bisect on `compose_trivial` shows both reference-graph commits
contribute roughly equally (`attribution/bisect_compose_trivial.json`):

| pin | mean |
|---|---|
| audit `51c1f16e1` | 10.49 ms ± 0.30 |
| `a8e5e98d9` (identity coordination) | 11.97 ms ± 0.44 (**+14.1%**) |
| `b425fb466` (guard wired) | 13.67 ms ± 0.46 (**+14.2%** on top) |

### Where the cost lands

`md compose --perf` on `compose_trivial`, audit vs head, localizes it precisely
— the **compose pipeline itself is unchanged** (807 µs → 833 µs). The whole
delta is in **Command Setup** (5.6 ms → 9.0 ms):

| segment | audit | head |
|---|---|---|
| `validate references` | 3.6 ms | **6.9 ms** |
| `build options` | 4.0 ms | **7.4 ms** |
| `capture context` | 147 µs | 148 µs |
| `compose pipeline` | 1.2 ms | 1.3 ms |

Note `compose_trivial` has **no** transclusion descendants, so
`verify_descendants`' per-child disk re-read cannot explain its +2.9 ms — the
cost is in graph/identity construction on the setup path, not the manifest walk.
The descendant re-read is an additional, separate cost that would scale with
child count (consistent with `compose_transclusion_heavy`'s +7.2 ms absolute).

### Ownership — reported, deliberately not fixed here

Finding 18 / `ReferenceGraph` correctness is **out of scope** for this plan by
its own charter, and "no Finding 18 correctness work landed here" is one of this
feature's acceptance criteria. Fixing or tuning the guard would violate that
boundary and the one-owner rule. It is therefore **reported to the owner** as a
cross-feature regression for the Opaque Reference Graph feature to disposition,
with the bisect and segment evidence above. Nothing was changed in response.

The guard is a correctness mechanism (it re-reads descendants to refuse a stale
prebuilt graph), so its cost may be a deliberate, accepted trade — that call
belongs to that feature's owner, not to this one.

## Correctness gate — byte-identical output at the final head

`clean head b425fb466` vs `working tree` over the complete manifest, all exit 0:

- `compose`: `compose_trivial`, `compose_schema_transclusion`,
  `compose_transclusion_heavy`, `compose_interpolation_heavy`, `replace_heavy`,
  `remote_heavy` — **all identical**
- `render` × `terminal`/`html`/`markdown`: `render_basic`, `render_code_heavy` —
  **all identical**
- `toc`: `toc_small`, `toc_medium`, `toc_large` — **all identical**
- `hash`: `hash_basic` — **identical**

16/16 identical. This is the cumulative form of the per-phase byte-identity
gates: the whole Phase 1–10 diff changes no user-visible output on any shipped
fixture, on any target.
