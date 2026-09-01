---
status: draft
created: 2026-09-01
area: claudine
packages:
    - darkmatter
    - claudine
---

# Frontmatter expressions see a `file()` parameter's raw override string, so derived paths re-anchor at the prompt document

## Summary

An eager `file()` schema parameter presents two different values inside one
composition: the **body** interpolates the resolved absolute path, while
**frontmatter whole-value expressions** receive the raw user-typed override
string. A path derived from that raw string (e.g.
`{{ dirname(spec) + '/plan.md' }}`) is therefore launch-CWD-relative, and when
the derived property is itself schema-typed as a lazy `file()`, the schema
pass re-anchors the relative string at the **prompt document's directory**
and re-spells it git-root-relative. The composed prompt then instructs the
agent to write its output into a directory that mirrors the source path under
the prompt document's home.

Observed incident (2026-09-01): running `prompts/plan.md` from the
`claudine/` package area against
`claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md` produced the
plan at `prompts/fixes/2026-09-01-inline-compose-frontmatter/plan.md` instead
of beside the spec. The plan was moved by hand; the defect remains.

## Reproduction (verified 2026-09-01, installed claudine of 2026-08-28)

From the repository root the derived path is correct only by coincidence (the
raw override string happens to equal the git-root-relative spelling):

```console
$ claudine compose prompts/plan.md \
    spec=claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md --dry-run
…
- Save the plan as "claudine/fixes/2026-09-01-inline-compose-frontmatter/plan.md"
```

From the `claudine/` package area, same document, same target file:

```console
$ cd claudine && claudine compose ../prompts/plan.md \
    spec=fixes/2026-09-01-inline-compose-frontmatter/spec.md --dry-run
…
- Functional Specification: /…/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
- Save the plan as "prompts/fixes/2026-09-01-inline-compose-frontmatter/plan.md"
```

The body's `{{spec}}` resolves correctly from both directories; only the
frontmatter-derived `{{plan}}` retargets.

## Two-stage mechanism (each stage isolated by probe)

**Stage 1 — raw string in expressions.** A probe with
`x: "{{ spec }}"` and `y: "{{ dirname(spec) + '/plan.md' }}"` in frontmatter
and `{{spec}}` in the body, launched from `claudine/`:

```text
SPEC-BODY = /…/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
X-FM      = fixes/2026-09-01-inline-compose-frontmatter/spec.md   (raw override)
Y-FM      = fixes/2026-09-01-inline-compose-frontmatter/plan.md
```

The expression environment binds `spec` to the raw override string even
though the eager resolution succeeded (the body proves it). Eager `file()`
values are supposed to be rewritten to git-root-relative form
(`rewrite_eager_file_values`, `darkmatter/lib/src/markdown/schemas/rewrite.rs`);
that rewritten (or resolved) value is not what expressions see.

**Stage 2 — lazy `file()` re-anchors at the document.** The same probe with
`plan` additionally schema-typed `file(required;match(**/*plan*.md))` and the
probe document placed in `prompts/` (mirroring `prompts/plan.md`):

```text
PLAN-FM = prompts/fixes/2026-09-01-inline-compose-frontmatter/plan.md
```

The lazy `file()` schema pass takes the stage-1 relative string, anchors it
at the source document's directory, and re-spells it git-root-relative. The
`prompts/` prefix in the incident comes from this stage, not from the agent.

## Expected contract

One parameter, one value. Frontmatter expressions must see the same resolved
projection of a `file()` parameter that body interpolation sees, so
`dirname()`-style derivations compose against the real location. This is the
launch-vs-source anchoring family: the 2026-08-12 ctx-launch-anchor fix
established that caller-supplied inputs anchor at the launch context while
document-authored references stay source-relative. A CLI override is a
caller-supplied input; re-anchoring it (or a string derived from it) at the
prompt document's directory silently retargets output paths whenever the
launch CWD is not the repository root.

Fix shape (to be ratified when scheduled): bind the post-resolution value
(git-root-relative or absolute — pick one and state it) into the expression
environment for eager parameters, and make the lazy `file()` normalization
treat an override-derived value as already anchored rather than
document-relative. Add a regression test mirroring the `prompts/plan.md`
shape: prompt document in one directory, spec override launch-relative from a
second, derived output path must land beside the spec.

## Impact

Any prompt that derives a path from a `file()` parameter in frontmatter —
the planning workflow (`prompts/plan.md`) being the live example — writes its
artifact to a wrong, silently created directory unless launched from the
repository root. No error is raised; the misplacement is only discovered when
the artifact goes missing from its expected location.
