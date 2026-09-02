---
status: draft
created: 2026-09-02
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-02
review_iterations: 1
area: claudine
packages:
    - darkmatter
    - claudine
depends-on: ../2026-09-01-file-param-anchoring/spec.md
related:
    - ../../features/2026-07-13-proxy-with/spec.md
    - ../../features/2026-08-26-finalized-references/spec.md
    - ../_completed/2026-06-27-path-resolution/spec.md
---

# Proxy targets lose caller file-parameter provenance

## Summary

A file parameter supplied by the caller can resolve successfully in a routing
document and then fail in the proxied target. The proxy target receives the
caller's raw relative spelling, but not the caller origin or the materialized
file identity that made the same value valid in the router. When the target
uses that value in `frontmatter()`, `file_exists()`, or another read-side file
function, Darkmatter reinterprets it as target-document-authored text and
anchors it to the target's source context.

The shipped implementation workflow exposes the defect:

```console
$ compose prompts/implement.md \
    spec='fixes/2026-09-01-file-param-anchoring/spec.md' \
    -y --codex
```

`prompts/implement.md` accepts the file, reads its `implemented` property, and
routes to `prompts/_implement/implement-suggestions.md`. The target then fails
while evaluating:

```yaml
iteration: "{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') || 1 : 1 }}"
```

The resulting diagnostic states that the same `spec` path which selected the
route cannot be resolved from the target document.

This fix makes caller file-parameter identity and origin explicit invocation
input. Canonical preparation must materialize a schema-selected caller file
parameter before frontmatter expressions consume it, and every proxy, retry,
resume, loop, and sequence re-entry must retain enough raw input and origin
information to reproduce that same materialization. A target document must not
re-anchor a caller-owned file value merely because the target declares the
property differently from the router.

> **Reader's note (review 2026-09-02):** This specification depends on the
> eager caller-projection fix rather than defining a parallel projection path.
> The review narrowed the new provenance record to immutable caller overrides,
> aligned lazy local, recursive, and remote behavior with the ratified
> finalized-reference contract, and made cache isolation explicit without
> changing the invocation-wide exact-command approval cache. Other input
> layers keep their established ownership and precedence.

## Classification and regression history

This is a regression against the completed
`2026-06-27-path-resolution` contract. That fix required caller-supplied file
references used by `frontmatter()` and `file_exists()` to retain a stable
launch-area fallback while document-authored references remained
document-first. The current read-side resolver instead deliberately excludes
the launch-area fallback because it cannot distinguish a caller-owned string
from a document-authored string.

Restoring the fallback indiscriminately would revive the original behavior but
break the newer document-ownership rule. The durable repair is to preserve
provenance until schema-selected file parameters are materialized, not to make
all strings search the caller's launch area.

The completed `2026-09-01-path-ref-fallback` fix is not the same surface. It
governs the operation-file positional passed to
`compose|inline-compose|sequence` and whether an unresolved positional enters
autocomplete. This failure occurs after the operation document resolved, in a
frontmatter expression inside a proxy target.

The implemented `2026-09-01-file-param-anchoring` fix is also narrower. It
projects eager caller file overrides before the first frontmatter interpolation pass,
but explicitly leaves lazy caller `file` values unchanged. The shipped router
declares `spec` as eager while the target declares it as lazy, so target
preparation falls back to the raw spelling and reproduces the failure. This
specification extends that projection artifact to caller-owned,
schema-selected lazy files; document-authored lazy files retain their existing
semantics. The earlier fix is therefore a direct dependency, including its
fail-closed schema-classification rule and pass-2 artifact handoff.

The active `2026-08-26-finalized-references` feature defines the broader target
model for parameter materialization, the finalized sigil catalog, `ctx.cwd`,
and `AGENT_CWD`. This fix extracts only the blocking provenance/materialization
subset. It must remain forward-compatible with that feature and must not
implement or preempt its unrelated grammar, sigil, completion, or environment
changes.

## Observed behavior

The failure was reproduced on 2026-09-02 with both the installed `claudine`
binary and the worktree's `target/debug/claudine`. Directly composing the proxy
target with the raw area-relative value fails with the same diagnostic:

```console
$ claudine compose prompts/_implement/implement-suggestions.md \
    spec='fixes/2026-09-01-file-param-anchoring/spec.md' \
    --dry-run --codex

MarkdownError: invalid file path
The iteration frontmatter property references the file
fixes/2026-09-01-file-param-anchoring/spec.md, which could not be resolved.
```

Supplying the repository-qualified spelling succeeds:

```console
$ claudine compose prompts/_implement/implement-suggestions.md \
    spec='claudine/fixes/2026-09-01-file-param-anchoring/spec.md' \
    --dry-run --codex
```

The second command is a diagnostic workaround, not the contract. Callers
should not need to rewrite a parameter after Claudine has already resolved it,
and a router must not change the meaning of immutable CLI input.

## Root cause

The failure crosses three ownership boundaries.

### 1. The router and target select different schema modes

`prompts/implement.md` declares `spec` as `file(required;eager;...)`. The eager
router value resolves successfully from the caller's captured file-resolution
context and is sufficient for the router's lifecycle condition to read the
spec frontmatter.

`prompts/_implement/implement-suggestions.md` declares `spec` as lazy
`file(required;...)`. Canonical target preparation reapplies the invocation's
raw `set_overrides` value at highest precedence. Because the target's property
is lazy, the eager-only projection path does not install the resolved semantic
value before the target's frontmatter dependency graph evaluates `iteration`.

### 2. The proxy retains the raw setter but loses its file origin

Invocation input correctly keeps caller overrides immutable so a router cannot
rewrite explicit caller intent. Immutability, however, is not the same as
discarding origin. The raw value and its authoring file-resolution context are
both invocation inputs. Retaining only the JSON value makes independently
authored layers indistinguishable after they are folded into one
`set_overrides` object.

By the time the target sees `spec`, it knows that the value outranks target
frontmatter but cannot know that the caller authored it relative to the launch
context. The target therefore treats the string as if it were authored in
`prompts/_implement/`.

### 3. Read-side functions intentionally use document context

Darkmatter's `resolve_arg` path for `frontmatter()`, `file_exists()`,
`absolute()`, `relative()`, and Markdown-loading functions resolves through
the active document's file-resolution context. It intentionally does not use
the legacy launch-area fallback for a nested-document reference. That is
correct for strings genuinely authored by the target and wrong for an
unmaterialized caller file parameter.

The expression layer cannot reconstruct provenance from a plain string. The
value must be materialized before expression evaluation, while canonical
preparation still has the target schema and the invocation's typed input
layers.

## Normative terminology

- **Raw parameter value** — the exact JSON/YAML value supplied by an input
  layer before schema-selected file materialization.
- **Parameter origin** — the immutable `FileResolutionContext` or trusted
  source context in which that input layer authored the raw file reference.
- **Materialized identity** — the absolute native path or typed remote identity
  produced by applying the selected file schema to the raw value and its
  origin.
- **Presentation value** — the portable text projection used in Markdown
  output while preserving the same materialized identity.
- **Document-authored value** — a value originating in the active document's
  frontmatter, schema default, or another document-owned source.
- **Caller-owned value** — a value supplied through immutable invocation input,
  including CLI shorthand setters and `--set`.

## Design decisions

### D1 — Raw value and origin are one caller-input record

Every immutable caller override must retain both its raw value and its
authoring origin until canonical preparation selects the effective target
schema. Origin is per winning property, not one directory attached to the
final merged map, because input assembly may combine properties authored by
different layers. This fix does not require document-owned values to be copied
into the caller-input record merely because their eventual schema contains a
file arm.

For this fix:

- CLI shorthand setters and `--set` use the invocation's captured launch
  `FileResolutionContext`.
- Target document frontmatter and schema defaults remain document-owned and
  use the target's `SourceContext`; they are not caller records.
- `proxy.with` and sequence/task values retain their established ownership,
  lifetime, and origin rules. They must never be mislabeled with the CLI launch
  origin merely because they share the input assembly path. The broader
  `2026-08-26-finalized-references` feature may route those established origins
  through the same general-purpose record later.
- An absolute materialized value retains its identity and is never re-anchored.

Layer precedence does not change. Target-authored frontmatter remains below
`proxy.with`, and explicit caller overrides remain highest. Provenance records
where a winning value came from; it does not alter which value wins.

### D2 — Materialize schema-selected caller files before interpolation

Canonical preparation must apply the target's effective schema to caller-owned
values before frontmatter interpolation pass 1.

For a selected local `file(eager)` value:

- resolve from the parameter origin;
- require the existing eager-file success condition; and
- install the winning absolute native identity plus its portable presentation
  value.

For a selected non-recursive local lazy `file` value:

- construct the ordered candidate plan from the parameter origin;
- materialize the first candidate as a normalized absolute path without
  requiring the target to exist; and
- install the native identity plus its portable presentation value.

Lazy materialization does not make lazy files eager. It binds an authoring
anchor; it does not add an existence check. If a later operation such as
`frontmatter()` requires the file to exist, that operation retains ownership of
the typed missing/read/parse failure.

Materialization must delegate parsing, typed local-versus-remote
classification, candidate construction, lexical normalization, and any
reference-kind containment checks to `FileReference`. It must not duplicate
sigil parsing or normalize by string manipulation. A
recursive lazy reference has no single unprobed identity and fails with the
typed parameter-binding guidance to use `file(eager)`, matching
`2026-08-26-finalized-references`. A lazy HTTP(S) value remains a typed remote
identity and is never converted into a local absolute path or probed as a local
file; eager remote behavior remains governed by the existing eager-file
contract.

Absent properties and explicit `null` values are not materialized. Schema
requiredness, nullability, defaults, and invalid-optional handling retain their
normal authority. A schema default remains document-owned even when a caller
record for another property exists.

Arrays and unions must use the same exactly-one-applicable-arm semantics as
normal schema coercion and file normalization. The pre-interpolation seam must
not guess an eager or lazy file arm merely because one arm mentions a file
format.

Ordinary strings are never parsed, probed, or rewritten. A value becomes a
file parameter only when the effective schema selects a file arm for that
caller-owned property.

### D3 — A proxy target reevaluates type, not origin

A proxy activates a fresh document and therefore applies the target's schema,
but it does not become the author of immutable caller input. Changing the
schema declaration from eager in a router to lazy in a target, or from lazy to
eager, may change validation requirements; it must not change the directory
against which the raw caller reference is interpreted.

This origin invariant does not assert that eager first-existing selection and
lazy first-candidate selection always choose the same identity. Direct/proxy
equivalence is evaluated after applying the same target schema to the same raw
caller record; the router's own schema verdict is not transported as target
state.

Given identical immutable invocation inputs, these two routes must produce the
same target semantic value:

```console
claudine compose target.md spec=relative/spec.md
claudine compose router.md spec=relative/spec.md  # router proxies to target.md
```

The equality is identity-based across platforms. Native and portable text may
differ on Windows while resolving to the same file.

### D4 — Preserve the record across every fresh preparation

Proxy, retry, resume, and any other fresh-read preparation must retain the raw
caller value and origin record, then reproduce materialization against the
current target schema. They must not use the prior document's effective
frontmatter as the new raw input, and they must not recapture ambient CWD.

Loop iterations or other paths that reuse a prepared structural plan must
retain the already materialized identity without re-anchoring it. Sequence
steps and tasks must follow their established fresh-preparation policy while
preserving per-layer origins. CLI caller overrides remain invocation-wide;
sequence-document and task-authored values do not become CLI caller overrides
when they enter the same preparation service.

The materialization operation must be idempotent. Applying canonical
preparation repeatedly to the same raw value, origin, schema, and captured
resolution context produces the same semantic and presentation values.

### D5 — Do not restore a global launch fallback in expression functions

This fix must not make `resolve_arg` unconditionally search the launch
directory. The expression evaluator receives strings from both caller-owned
and document-authored sources and cannot infer ownership after interpolation.

Document-authored references continue to resolve from the active document's
`SourceContext` according to the current grammar and candidate ordering.
Caller-owned file parameters reach expression functions as materialized
identities, so the expression layer needs no provenance heuristic or fallback
exception.

### D6 — Raw, semantic, and presentation projections stay distinct

Canonical preparation must retain:

1. the raw value and origin needed for future fresh preparation;
2. the native semantic identity consumed by frontmatter expressions, schema
   operations, lifecycle, and path functions; and
3. the portable presentation value used for Markdown rendering.

No stage may overwrite the raw invocation record with a normalized or
presentation value. No later stage may independently resolve the raw value a
second time using a different context.

### D7 — Direct and proxy diagnostics remain equivalent

When materialization or a later file operation fails, the typed diagnostic
must retain the same identity on direct and proxied routes. It must identify
the caller-authored reference and the origin/base used to construct candidates;
when a concrete candidate was selected, it must also identify that candidate.
If the current diagnostic payload cannot carry this evidence, extend its
structured detail without replacing its established diagnostic code.

The proxy coordinator must not wrap the target's typed file-reference or
schema diagnostic in a generic proxy bootstrap error that becomes the
effective diagnostic. Existing transparent/semantic diagnostic-selection
rules remain authoritative.

No new diagnostic code is required unless implementation reveals a genuinely
new authoring error. A missing eager caller file remains an invalid file
reference/schema failure; a lazy file that materializes successfully but later
fails `frontmatter()` remains the read-side invalid-file-reference failure.

## Implementation surface

Point-in-time file references below identify responsibilities, not mandatory
line-level edits. Implementation must follow the owning symbols if code moves.

### Darkmatter

1. `markdown::compose::ComposeOptions` and its request hashing/identity logic:
   retain per-property caller file-reference origin records alongside raw
   `set_overrides`; do not infer the origin later from the active source.
2. `markdown::compose::schema_validation`: provide one schema-aware caller-file
   materialization seam that runs before frontmatter interpolation and handles
   selected eager and lazy file arms without guessing unions.
3. `markdown::compose::pipeline`: carry one materialization artifact through
   schema validation, shell/pass-2 revalidation, and `EffectiveState` building.
4. `markdown::compose::expression`: consume materialized identities through
   existing document resolution; do not add a raw-string launch fallback.
5. Any cache, graph, or request fingerprint that can reuse composed or
   prepared parameter-derived state: include raw values and stable origin
   identity so two otherwise identical runs from different caller origins
   cannot share an invalid projection. The exact-command shell approval cache
   remains keyed by its established command/policy identity and is not split
   merely because a caller file origin differs.

### Claudine

6. Immutable invocation and caller-input layers: capture the launch
   file-resolution origin once and retain it per applicable caller property.
7. Canonical document preparation: pass raw values and origins through direct,
   proxy, retry, resume, loop, inline-compose, and sequence/task entry paths.
8. Proxy adoption: replace only document identity and the immediate proxy
   overlay; do not discard or retarget caller origins.
9. Documentation: update the authoritative composition/file-reference topic
   and portable skill snapshot to distinguish immutable caller origin from
   active-document origin. Keep both documents synchronized.

No terminal output changes are required. If implementation changes a
user-facing diagnostic, it must continue through Claudine's typed diagnostic
and `TerminalRenderable` path.

## Error behavior

- A malformed caller file reference fails through the existing typed
  file-reference diagnostic and names the raw caller spelling.
- A missing eager caller file fails before frontmatter expressions, using the
  captured caller origin and existing eager validation contract.
- A lazy caller file materializes without an existence check. If
  `frontmatter()`, `markdown_title()`, or another read later requires a missing
  file, that function reports the existing typed read-side failure against the
  materialized candidate while retaining the required raw/origin detail.
- A zero-match or ambiguous schema union does not guess a file arm. Normal
  schema validation owns the verdict.
- An absent or explicit-null caller property is not file-materialized; normal
  schema requiredness, nullability, and default behavior owns the verdict.
- A recursive lazy caller file fails as a typed parameter-binding error, while
  a lazy HTTP(S) caller file remains remote and causes no local filesystem
  probe.
- Dynamic schema classification that changes after pass 1 follows the
  fail-closed behavior defined by `2026-09-01-file-param-anchoring`; this fix
  extends that stability check to lazy-versus-eager file selection where the
  caller property has already been consumed.
- No failure may leak a raw whole-value `{{ ... }}` expression into the prompt
  or launch a provider with partially prepared frontmatter.

## Verification

### Darkmatter Level 1

Add focused tests proving:

1. a caller-owned lazy `file` override is materialized from its captured
   origin before `frontmatter(spec, ...)` runs;
2. the same raw value from two distinct origins produces two distinct
   materialized identities and cache identities;
3. eager and lazy declarations may differ between two preparations without
   changing origin;
4. direct preparation and proxy-target-equivalent preparation produce the same
   native semantic and portable presentation values;
5. arrays and exactly-one-applicable property/root unions materialize the
   selected file arm, while ambiguous and zero-match unions do not guess;
6. ordinary strings, document-authored lazy files, absent optionals, and
   excluded DM1/lifecycle keys are not caller-materialized;
7. explicit `null`, schema defaults, `proxy.with`, and sequence/task-authored
   values retain their established schema and origin behavior and are never
   mislabeled with the CLI launch origin;
8. a non-recursive lazy local value chooses the first unprobed candidate, a
   recursive lazy value fails with eager guidance, and a lazy HTTP(S) value
   remains remote without local probing;
9. pass 2 and post-shell schema revalidation retain the same artifact and fail
   closed on classification drift;
10. retry/resume-style fresh preparation reproduces the projection from raw
   input and origin, while reuse-style preparation does not re-anchor it;
11. missing eager and later-read missing lazy files preserve their respective
   typed diagnostics and origin evidence; and
12. Windows native absolute paths and portable `/` presentation identify the
    same file without manual separator replacement.

### Claudine Level 1

Add process-level coverage with a fake provider for the exact shipped
workflow:

1. launch from a package area containing
   `fixes/<case>/spec.md`;
2. invoke the repository-shared `prompts/implement.md` with
   `spec=fixes/<case>/spec.md`;
3. allow the router's `initialize` stack to select
   `prompts/_implement/implement-suggestions.md`;
4. prove the target evaluates `review_iterations`, derives `review`, `log`, and
   `design` beside the same specification, and reaches the fake provider; and
5. compare the prepared target with direct invocation of
   `implement-suggestions.md` using the same immutable caller inputs.

Additional focused cases must cover:

- retry and resume of the proxied target;
- a second proxy hop without explicit forwarding, proving caller inputs remain
  invocation-wide while `proxy.with` remains immediate-target-only;
- caller override precedence over a conflicting `proxy.with` value;
- inline-compose and a sequence task that route to a target using a caller file
  parameter;
- a sequence/task-authored file value and a CLI caller file value present in
  the same preparation, proving their origins remain distinct; and
- a post-launch process-CWD change, proving no ambient recapture participates.

No Level 2 or Level 3 coverage is required for the semantic contract. The
process tests must be non-interactive, use fake providers, and must not focus a
terminal or browser window. Existing proxy terminal-rendering tests remain the
authority for presentation behavior.

Use the package-area gates:

```console
$ cd darkmatter && just test && just lint
$ cd claudine && just test && just lint
```

Before completion, run `just test darkmatter claudine` and
`just ci-local darkmatter claudine` from the repository root. These root gates
are in addition to, not replacements for, the focused package-area commands.

## Acceptance criteria

1. The exact `implement.md` → `implement-suggestions.md` command succeeds when
   the caller supplies an area-relative `spec` that resolved in the router.
2. `frontmatter(spec, 'review_iterations')` reads the same file in the router,
   direct target, and proxied target.
3. The target derives `review`, `log`, and optional `design` beside the input
   specification; no derived path is retargeted beneath `prompts/` or
   `prompts/_implement/`.
4. A caller-owned file parameter retains its origin across direct, proxy,
   retry, resume, loop, inline-compose, and sequence/task preparation routes.
5. A target may redeclare the caller property as lazy or eager. The declaration
   changes existence/validation behavior but never the caller origin.
6. Caller-owned lazy local files materialize an absolute candidate before
   frontmatter interpolation without requiring existence. Caller-owned eager
   files retain the existing existence requirement.
7. Document-authored references remain scoped to the active document and never
   gain an unconditional launch-area fallback.
8. Ordinary string overrides remain byte-for-byte unchanged and cause no file
   parsing or filesystem probes.
9. Scalar, array, and exactly-one-applicable union shapes materialize through
   the same schema-selection semantics; ambiguous and zero-match unions never
   guess.
10. Absent and explicit-null caller properties are not materialized; schema
    defaults remain document-owned. Recursive lazy local references fail with
    eager guidance, and lazy HTTP(S) references retain remote identity without
    local probing.
11. Raw value, origin, native semantic identity, and portable presentation are
    retained as distinct projections and remain stable through pass 2 and
    fresh re-entry.
12. Direct and proxied failures retain the same typed diagnostic identity,
    raw reference, origin/base, and candidate evidence.
13. Cache and request identities that reuse prepared/composed state distinguish
    equal raw values authored from different origins; exact-command approval
    cache semantics do not change.
14. macOS, Linux, and Windows use platform-native semantic paths and portable
    presentation without changing identity.
15. Darkmatter and Claudine Level 1 and lint gates pass, including the shipped
    router regression and direct/proxy equivalence assertions.

## Non-goals

- Changing operation-file autocomplete or the
  `2026-09-01-path-ref-fallback` diagnostic contract.
- Restoring launch-area fallback for every string passed to a read-side file
  function.
- Changing document-authored file-reference candidate ordering.
- Implementing the complete finalized sigil catalog (`&`, `^`, `!` removal),
  completion changes, `ctx.cwd`, or `AGENT_CWD` from
  `2026-08-26-finalized-references`.
- Persisting proxy inputs into target Markdown or changing Darkmatter Markdown
  content hashes.
- Making lazy files require existence or adding speculative filesystem probes
  to choose among lazy candidates.
- Adding general expression-value taint tracking. Provenance is retained on
  typed input-layer records until file materialization; arbitrary derived
  strings remain ordinary values.
- Reclassifying `proxy.with`, schema defaults, document frontmatter, or
  sequence/task-authored values as CLI caller input, or implementing the
  finalized-reference feature's future origin-policy changes for those layers.
- Changing caller override, `proxy.with`, runtime mutation, or sequence overlay
  precedence.

## Open questions

None. Review ratifies a per-property caller record, schema-selected eager/lazy
materialization, recursive-lazy rejection, lazy-remote preservation, and
origin-sensitive prepared-state identity without changing other layers'
ownership or the shell approval cache.

## Definition of done

This fix is complete when every acceptance criterion is covered at the stated
test level, the exact shipped implementation router reaches its target from the
same caller-relative input that selected the route, direct/proxy preparation is
semantically equivalent for caller file parameters, affected documentation is
updated without drift, and the Darkmatter and Claudine package gates pass.
