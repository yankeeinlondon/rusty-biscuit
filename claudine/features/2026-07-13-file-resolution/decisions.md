---
created: 2026-07-17
phase: 1
feature: 2026-07-13-file-resolution
spec: ./spec.md
plan: ./plan.md
---

# Decision Gates — Unified File-Reference Resolution

Phase 1 output. This file records the rulings for the four decision gates
(G1–G4) that block Phase 2, each with rationale grounded against HEAD on branch
`error-prop-and-file-resolution`. The call-site audit, fixture-collision
inventory, extended migration sweep, and recorded baselines live in
[`inventory.md`](./inventory.md).

No production code is changed in Phase 1.

## Grounding Re-Verification (against HEAD)

Every grounding fact the plan asserted was re-checked before ruling:

| Fact | Verified location | Status |
|---|---|---|
| `ReferenceKind` is `pub(crate)`; no public classification exists | `biscuit-file/lib/src/file_reference/mod.rs:403` | ✅ confirmed |
| Implicit order today is **base/CWD first, then git root** | `biscuit-file/lib/src/file_reference/resolve.rs:190-198` (`roots = vec![ctx.cwd]`, then `git_root`) | ✅ confirmed |
| No `Home` (`~`) kind in the grammar; `/` is the **only** absolute check | `parse.rs:64-88` (`/`→Absolute, `./`\|`../`→Relative, else→ImplicitRelative; no `~`, no `C:\`, no UNC) | ✅ confirmed |
| `home_dir()` reads `$HOME` only — `None` on native Windows | `context.rs:157-159` (`std::env::var_os("HOME")`) | ✅ confirmed |
| `prefers_cwd_over_git_root_on_name_collision` ratifies CWD-first | `lib/tests/implicit_relative.rs:49-72` | ✅ confirmed — Phase 4 inverts this |
| `biscuit-file` does **not** depend on `sniff` (cycle risk `sniff`→`biscuit-file` is real) | `biscuit-file/lib/Cargo.toml` (no `sniff` entry) | ✅ confirmed |

Internal enum naming note for Phase 2: the private `ReferenceKind::Relative`
variant **is** the explicit-relative (`./`, `../`) kind; `ImplicitRelative` is
the bare kind. The public surface must name these `ExplicitRelative` /
`ImplicitRelative` to avoid the ambiguous "Relative".

---

## G1 — Ordering against error-propagation

**Ruling: SPLIT THE SEAM (plan's recommended default), and treat
error-propagation's outputs as ALREADY AVAILABLE.**

### Rationale

The execution plan (`plan.md:56-70`) was written on the premise that
`../2026-07-13-error-propogation/` is a `status: draft` spec with **no
`plan.md`** that "must land first," which would block this feature entirely.
That premise is **stale**. Re-verification against HEAD shows error-propagation
is fully executed:

- `claudine/features/2026-07-13-error-propogation/plan.md` exists with
  `phase: 8, total_phases: 8` — all eight phases were planned and run.
- The file-resolution spec's own "Upstream Dependency Status — typed transport
  has LANDED" section (`spec.md:13-73`) states the dependency is **complete**
  and enumerates its outputs as this feature's inputs.
- The concrete outputs are present in the tree:
  - the `composition.invalid_file_reference` wrapper and the whole
    `claudine/lib/src/diagnostics/` subsystem (registry, discovery, snapshot);
  - `err.code` / `err.detail.*` projection plumbing
    (`diagnostics/mod.rs:113,155`, `composition/lifecycle/context.rs`);
  - the AC5 pinning test
    `level2_proxy_routes_share_a_typed_surface_but_diverge_on_identity_in_tmux`
    at `claudine/cli/tests/level2_typed_error_render_capture.rs:580`.
- The dependency spec's frontmatter still reads `status: draft`
  (`error-propogation/spec.md:3`); this is a stale frontmatter value, **not**
  evidence the work is unfinished. The authoritative signal is the executed
  8-phase plan plus the present code.

Because the transport is done, the plan's "split the seam" default is not just
viable — it is the low-risk path with the blocker already removed:

1. **`biscuit-file` owns the typed detailed outcome** (classification, ordered
   provenance-carrying candidates, root provenance, per-candidate probe
   disposition, typed failure). It has **zero** dependency on error-propagation.
   Built here in Phases 2–4. This unblocks Phases 2–5 and 7 entirely.
2. **The Claudine semantic wrapper and `err.detail.*` projection already
   exist** from error-propagation. Phase 6 lands a narrow typed adapter that
   fills the fields error-propagation reserved as `null` by ruling
   (`spec.md:21-37`): `failure`, `candidates`, `repository_root`. Per that
   spec's null contract, `base_dir`/`fallback_dir` remain compatibility
   projections and must not be removed or re-typed.

### Consequences / guardrails

- **Do not derive `failure` from `kind`** (`spec.md:29`): Darkmatter's
  `FileRefFailure::classify` folds I/O, permission, and missing-context
  failures into `NotFound`, so `NotFound → no_match` would mislabel a
  permission error that never probed a candidate. The typed `failure`
  classification is produced by `biscuit-file`'s detailed outcome (Phase 3, D8).
- **The AC5 pinning test will fail when this feature lands — by design.** When
  it fails, do **not** weaken it: promote its assertions to full AC5 parity
  (identical code, headline, hint, and typed detail across both proxy routes),
  keeping the event/property context assertions separate so route-specific
  detail is not mistaken for drift (`spec.md:59-66`). This is a Phase 6/Phase 8
  action, recorded here so it is not forgotten.
- Inheritable debt: `error_guards/transport-allow.toml` carries 71
  `error-propagation-followup` entries (`spec.md:68-73`). Any sitting on a
  file-resolution path are fair game to close in this feature; the rest are out
  of scope.

---

## G2 — `@` semantic collision

**Ruling: ADOPT `FileReference` MAGIC SEMANTICS OUTRIGHT. No compatibility
shim. Recorded as an intentional behavior change.**

### Rationale

`resolve_harness_path` defines `@foo` as **repo-root-relative**
(`harness/resolve.rs:46-53`); `FileReference` defines `@` as **magic-root
search**. Different contracts on the same sigil, so a naïve migration would
silently change meaning.

Re-verified at execution time (Phase 1 required this):

```
grep -rn --include='*.md' -E '(proxy|sequence):[[:space:]]*@' prompts .claudine
→ (empty)
```

A repo-wide sweep for `(proxy|sequence):\s*@` across all `*.md` also returned
empty. **Zero** authored `@`-prefixed `proxy:`/`sequence:` values exist, so no
in-tree document changes meaning under the switch.

The `@` in the harness proxy therefore changes from repo-root join to magic
search. Record in Phase 8 timeline/release notes as an intentional behavior
change (`plan.md:280`, spec D-list). Risk downgraded Medium → **Low**.

---

## G3 — Coordination with `biscuit-file/features/2026-06-13-resolve-tuple/`

**Ruling: BUILD D3's CANDIDATE/ROOT PLAN AS A REUSABLE PUBLIC SURFACE so
resolve-tuple consumes it. Do NOT implement resolve-tuple in this feature.**

### Rationale

`biscuit-file/features/2026-06-13-resolve-tuple/spec.md` exists (17 KB, spec
only — no `plan.md`). It adds `resolve_tuple()` and states existing methods are
unchanged: additive and non-conflicting. It will, however, want the same
root/provenance data this feature builds in D3, and it centralizes
path-abbreviation across seven copies.

To avoid growing an eighth copy, D3's candidate/root **builder** (Phase 3) is
designed as a separable, inspectable public surface — candidate generation
split from matching, provenance carried as data. resolve-tuple later consumes
that builder rather than reimplementing it. This feature does **not** implement
resolve-tuple. Risk: Low.

---

## G4 — Acceptance criterion 4 vs. the already-applied workaround

**Ruling: PROVE AC4 WITH A DEDICATED L2 FIXTURE (Phase 8). The `prompts/`
revert to bare spelling is OPTIONAL and is NOT a gate.**

### Rationale

Commit `2d7c847d4` ("fix(prompts): make proxy paths relative and route review
input") already rewrote authored proxy values to `./` form. Verified current
state:

- `prompts/implement.md:24` → `proxy: ./_implement/implement-suggestions.md`
- `prompts/implement.md:28,32`, `prompts/review.md:30-42` → all `./`-prefixed.

The bare form from the motivating incident
(`prompts/_implement/implement-suggestions.md`) **no longer exists** in the
tree, so AC4 ("the motivating router reference resolves successfully without
rewriting it to `./`") cannot be proven against live prompts as they stand.

AC4 is therefore proven with a dedicated L2 fixture in Phase 8: a router at
`<repo>/prompts/` proxying the **bare** `prompts/_implement/...` and asserting
it resolves to `<repo>/prompts/_implement/...`, not the doubled
`<repo>/prompts/prompts/...`. A paired fixture proves the `./` spelling stays
source-relative and fails when the source-local path is absent (`plan.md:271-272`).

Whether to revert `prompts/` to the bare spelling once repository-first lands
is deferred to a Phase 8 decision task and is **optional** — the `./` spelling
remains correct and pins source-local intent. It is **not** treated as a gate
(`plan.md:96-106`).

---

## Summary of Rulings

| Gate | Ruling | Risk after ruling |
|---|---|---|
| G1 | Split the seam; error-propagation outputs already landed — Phase 6 fills reserved nulls | High → **Low** (blocker already removed) |
| G2 | Adopt `FileReference` magic `@` outright; intentional behavior change; zero in-tree usage | Medium → **Low** |
| G3 | Expose D3 builder as reusable public surface; do not implement resolve-tuple | **Low** |
| G4 | Prove AC4 via dedicated L2 fixture; `prompts/` revert optional, not a gate | **Low** |
