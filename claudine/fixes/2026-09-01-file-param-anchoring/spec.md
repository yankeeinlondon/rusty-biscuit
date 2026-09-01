---
status: draft
created: 2026-09-01
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-01
area: claudine
packages:
    - darkmatter
    - claudine
---

# Project caller file overrides before frontmatter expression evaluation

## Summary

An eager `file(eager)` parameter supplied by a caller currently has two semantic
values during one composition:

- body interpolation receives the caller-resolved value; and
- frontmatter whole-value expressions receive the raw override string because
  they run before schema processing resolves eager caller file overrides.

A derived path such as `{{ dirname(spec) + '/plan.md' }}` therefore starts from
the caller's launch-relative spelling instead of the resolved file. When that
derived property is a lazy `file()`, its relative text is subsequently treated
as document-authored and shaped from the prompt document. The result can point
under the prompt directory rather than beside the caller's input file.

This is an ordering defect, not a new lazy-file anchoring case. Resolve and
install caller-originated eager-file projections before frontmatter
interpolation pass 1. Every expression surface then consumes the same resolved
semantic value, and existing path functions produce their established portable
display projection. Do not add provenance tracking to strings derived from
file parameters and do not change the source-relative contract for genuinely
document-authored lazy file references.

Observed incident (2026-09-01): running `prompts/plan.md` from the `claudine/`
package area against
`claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md` produced
`prompts/fixes/2026-09-01-inline-compose-frontmatter/plan.md` instead of a plan
beside the specification. The plan was moved by hand; the defect remains.

> **Reader's note:** The draft proposed teaching lazy `file()` normalization
> that an arbitrary derived string was "override-derived." Review rejected that
> shape because the provenance is lost through general expression operations
> and preserving it would require a new taint-like value system. Correcting the
> value before expression evaluation fixes the cause while retaining the
> established eager/lazy and caller/document ownership boundaries.

## Reproduction

Verified on 2026-09-01 with the Claudine build installed on 2026-08-28.

From the repository root, the result is correct only by coincidence because
the raw override already uses the repository-root-relative spelling:

```console
$ claudine compose prompts/plan.md \
    spec=claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md --dry-run
…
- Save the plan as "claudine/fixes/2026-09-01-inline-compose-frontmatter/plan.md"
```

From the `claudine/` package area, using the same prompt and target:

```console
$ cd claudine
$ claudine compose ../prompts/plan.md \
    spec=fixes/2026-09-01-inline-compose-frontmatter/spec.md --dry-run
…
- Functional Specification: /…/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
- Save the plan as "prompts/fixes/2026-09-01-inline-compose-frontmatter/plan.md"
```

The body's `{{spec}}` resolves correctly from both directories. Only the path
derived from `spec` in frontmatter is retargeted.

## Root cause

The failure has two stages.

### Stage 1 — frontmatter expressions see the raw override

A probe with `x: "{{ spec }}"`,
`y: "{{ dirname(spec) + '/plan.md' }}"`, and `{{spec}}` in the body, launched
from `claudine/`, produces:

```text
SPEC-BODY = /…/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
X-FM      = fixes/2026-09-01-inline-compose-frontmatter/spec.md
Y-FM      = fixes/2026-09-01-inline-compose-frontmatter/plan.md
```

`prepare_frontmatter_for_compose` applies `ComposeOptions::set_overrides`
before the pipeline begins, but the first frontmatter interpolation pass runs
before `schema_validation::run_with_registry`. The latter currently calls
`resolve_eager_caller_file_overrides`, installs absolute native values, and
records portable presentation values. The eager projection therefore arrives
after the frontmatter dependency graph has already consumed and replaced the
raw value.

### Stage 2 — the derived relative string has document provenance

With `plan` schema-typed as lazy
`file(required;match(**/*plan*.md))` and the prompt stored in `prompts/`, the
same probe produces:

```text
PLAN-FM = prompts/fixes/2026-09-01-inline-compose-frontmatter/plan.md
```

At this point `plan` is an ordinary document-authored string. The schema and
file-reference layers have no sound way to infer that it was derived from a
caller value, so they correctly apply the document-side rules to the relative
text they receive. Stage 2 exposes the defect but does not cause it.

## Established contracts

This fix repairs accidental pipeline drift and preserves these existing
contracts:

- Caller-supplied inputs anchor at the invocation's captured launch context;
  document-authored references anchor at their source context. The
  `2026-08-12-ctx-launch-anchor` fix keeps those ownership domains distinct.
- `ComposeOptions` owns the request-scoped `FileResolutionContext`. No stage
  may recapture ambient CWD, repository, environment, or host state.
- `file(eager)` is the only schema file type that resolves and requires
  existence. Bare `file` remains lazy and syntax-only.
- Document-owned eager values normalize after successful validation to the
  portable git-root-relative projection. Validation-only APIs remain passive
  and do not mutate their input.
- Caller-owned eager overrides remain absolute native paths in semantic
  frontmatter so later path operations cannot reinterpret them as
  document-authored. Their presentation projection uses `/` separators for
  portable Markdown output.
- `dirname()`, `relative()`, and the other path functions resolve through the
  request context and return the established portable display shape. This is
  how an absolute caller-owned `spec` naturally derives the repo-relative
  `claudine/fixes/…/plan.md` result.

“One parameter, one value” means one resolved **semantic** value across
frontmatter expressions, schema evaluation, path functions, and body effective
state. It does not collapse the existing semantic/presentation distinction:
native path text may differ from its `/`-normalized Markdown presentation on
Windows while identifying the same file.

## Design decisions

### D1 — Project eager caller file overrides before frontmatter interpolation

Add a schema-aware caller-override projection seam before frontmatter
interpolation pass 1. It must:

1. use the already captured launch `FileResolutionContext` and
   `file_ref_fallback_dir`;
2. identify only caller-supplied top-level values whose effective schema
   fragment is eager `format: darkmatter-file`;
3. resolve those values with `FileReference` from the trusted caller base;
4. install the absolute native value into the working frontmatter before the
   interpolation dependency graph is built; and
5. retain the portable presentation value for the later `EffectiveState` build.

The seam must support the schema shapes already handled by
`resolve_eager_caller_value`: scalars, arrays, property/root unions, and
baseline plus document schemas. Ordinary strings, lazy `file` values,
document-authored values, absent optionals, and excluded DM1 keys are not
projected.

This is not full schema validation and must not coerce values, materialize
optional document bindings, emit a verdict, or normalize document-owned eager
files. Those operations keep their current validation-stage ownership.

### D2 — Preserve one projection artifact through the pipeline

The pre-interpolation seam returns one caller-projection artifact containing
the native and presentation maps. Pass that artifact into normal schema
processing and the final `EffectiveState` build rather than resolving the raw
override independently a second time.

This prevents three representations from drifting:

- the value frontmatter expressions read;
- the value schema/path operations validate; and
- the value body interpolation presents.

Post-shell schema revalidation must reuse or deterministically reproduce the
same artifact. It must never fall back to the already-mutated process CWD or
treat the installed absolute value as document-authored.

### D3 — Dynamic schema typing is fail-closed

Trigger matching is re-evaluated after interpolation and shell expansion. A
trigger can therefore make a caller property eager, or stop making it eager,
after pass 1 has already consumed that property. Silently accepting a changed
classification would recreate the same split-value defect through a less
common route.

Compare the eager caller-property classification at each effective-schema
assembly with the pre-interpolation classification. If it changes for a
present caller override, abort with a typed composition diagnostic that names
the property and explains that caller file-parameter typing must be stable
before frontmatter interpolation. The author can make the eager declaration
unconditional in the baseline/document schema or remove the frontmatter
dependency.

Do not rerun interpolation from partially composed state. Read-side functions
and warnings would otherwise execute more than once, and retaining enough
pristine state for a speculative fixed-point pipeline is disproportionate to
this defect.

The diagnostic must participate in Claudine's established diagnostic discovery
and effective-selection seam; do not flatten it into a generic string error.

### D4 — Lazy file semantics do not change

Once frontmatter expressions receive the resolved eager input,
`dirname(spec)` uses the shared path resolver and portable projection. The
derived `plan` value is therefore already correctly shaped for the existing
lazy-file rules.

Do not:

- add an “already anchored” flag to `file()`;
- preserve expression provenance through concatenation;
- make lazy `file()` resolve or require existence;
- rewrite arbitrary path-shaped strings; or
- alter document-first/repository-aware resolution for document-authored
  references.

### D5 — Keep the compose pipeline's validation boundary intact

The public stage order remains frontmatter interpolation → schema validation →
shell expansion → frontmatter interpolation pass 2. The new work is a narrow
input-projection prelude required to establish the values consumed by pass 1;
it does not move schema validation or its verdict ahead of interpolation.

Factor effective-schema construction and trigger-registry discovery so the
prelude and normal validation can share the same request-scoped registry and
schema-resolution logic. Do not introduce a second filesystem discovery walk
or a second owner for schema assembly.

## Implementation surface

File references below are point-in-time anchors; implementation should follow
the named symbols if lines move.

### Darkmatter

1. In `darkmatter/lib/src/markdown/compose/schema_validation.rs`, extract the
   reusable effective-schema/registry assembly currently embedded in
   `run_with_registry`.
2. Expose a crate-private pre-interpolation operation that builds the stable
   eager caller-property classification and calls the existing
   `resolve_eager_caller_file_overrides` / `resolve_eager_caller_value` logic.
   Preserve the `FileReference` typed failures and caller-base resolution.
3. Introduce a small pipeline-owned caller projection artifact with native and
   presentation maps plus the eager-property classification needed for D3.
   This is transient request state, not serialized frontmatter metadata.
4. In `pipeline::run_compose_pipeline_internal`, create and apply the artifact
   after `prepare_frontmatter_for_compose` applies overrides and before
   `frontmatter_interpolation::interpolate_frontmatter` pass 1.
5. Change `run_with_registry` to consume the prepared artifact. Retain its
   validation, coercion, optional-binding, problem filtering, and
   document-owned eager normalization responsibilities, but remove independent
   raw-override projection that could disagree with the prelude.
6. Recheck the eager-property classification whenever triggers are matched
   again, including the post-shell pass, and emit the D3 typed diagnostic on
   drift.
7. Feed the artifact's presentation map into `EffectiveStateBuilder` exactly
   once. Arrays and static member/index interpolation must retain current
   presentation behavior.
8. Update pipeline and schema documentation/comments whose current stage
   narrative says frontmatter interpolation sees raw `--set` values without
   mentioning eager caller projection.

### Claudine

9. Add an end-to-end regression through Claudine's normal compose preparation
   path using the actual `prompts/plan.md` shape. Claudine should not add a
   second path resolver or patch the composed prompt after Darkmatter returns.
10. Update the Claudine composition topic and its skill-local mirror to state
    that caller eager-file parameters are launch-resolved before frontmatter
    expressions, while document-authored file references remain source-relative.

No terminal rendering changes are required. Any new user-facing diagnostic
must use the existing typed diagnostic renderer; do not add direct ANSI or
ad-hoc terminal output.

## Error behavior

- A malformed or missing eager caller override fails with the existing typed
  file-reference diagnostic and launch-base provenance.
- A relative eager caller override never falls back to the prompt directory.
- A phase-unstable eager classification fails with the D3 typed diagnostic
  before provider launch or shell execution.
- Lazy/missing output paths remain valid and do not touch the filesystem.
- No failure path may leak the raw `{{ ... }}` whole-value expression into the
  composed document.

## Verification

### Darkmatter Level 1

Add focused unit/integration coverage for:

1. a prompt under `prompts/`, launch base under `claudine/`, and an eager
   `spec` override spelled `fixes/<case>/spec.md`; `x: "{{ spec }}"` observes
   the resolved semantic value and
   `plan: "{{ dirname(spec) + '/plan.md' }}"` becomes
   `claudine/fixes/<case>/plan.md`;
2. the same fixture launched from the repository root produces the identical
   derived `plan` value;
3. body interpolation still uses the portable presentation value and resolves
   to the same file as the frontmatter semantic value;
4. eager arrays and applicable union arms are projected before expressions;
5. lazy `file`, ordinary string, absent optional, document-authored eager file,
   and DM1-excluded values are not caller-projected;
6. malformed and missing eager overrides retain their typed diagnostic and
   launch anchor;
7. a trigger that changes eager classification after interpolation fails
   closed with the D3 diagnostic before shell execution;
8. pass 2 and post-shell schema revalidation retain the same projection; and
9. projection is idempotent and performs no second registry/discovery walk.

Include Windows coverage for native absolute semantic paths and `/`-normalized
presentation/derived paths. Where the host cannot execute Windows behavior,
add platform-gated tests suitable for Windows CI plus platform-neutral tests of
portable projection. macOS and Linux behavior must remain identical.

### Claudine Level 1

Run the actual planning workflow from both the repository root and the
`claudine/` package area with `--dry-run`. Both outputs must instruct the agent
to save beside the same specification. Assert the complete target path, not
merely the absence of a `prompts/` prefix.

Use the package-area gates:

```console
$ cd darkmatter && just test && just lint
$ cd claudine && just test && just lint
```

No Level 2 or Level 3 test is required because the changed behavior is
composition semantics, not terminal rendering, browser rendering, focus, or
input encoding.

## Acceptance criteria

1. Frontmatter expressions, schema/path operations, and body effective state
   consume one caller-resolved semantic value for every eager caller file
   override.
2. The `prompts/plan.md` workflow derives the plan beside the input spec from
   both repository-root and package-area launch directories.
3. Caller overrides resolve only from the captured launch context; no ambient
   CWD or prompt-directory fallback participates.
4. Caller native semantic values and portable presentation values retain their
   existing cross-platform distinction without identifying different files.
5. Document-owned eager normalization and lazy file semantics are unchanged.
6. Dynamic eager classification cannot silently change after frontmatter
   interpolation; it fails with a typed diagnostic.
7. Scalar, array, union, shell-pass, and DM1 behavior is covered at Level 1.
8. Darkmatter and Claudine package-area Level 1 and lint gates pass.

## Impact

Any prompt that derives a path from an eager caller-supplied `file()` parameter
can silently target the wrong directory when launched outside the repository
root. The planning workflow is the known live example, but the defect is in the
shared Darkmatter pipeline and therefore affects direct compose, inline
compose, sequence/task preparation, proxy targets, retries, and any downstream
caller using the same `ComposeOptions::set_overrides` path.

The fix intentionally changes only when an already-required caller projection
becomes visible. It does not expand filesystem access, alter lazy-file
validation, or create a new path-provenance system.

## Open questions

None. Review ratifies early caller projection, stable eager typing, and
unchanged lazy-file semantics.
