---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-06-dotted-paths/spec.md
plan: darkmatter/features/2026-11-06-dotted-paths/plan.md
implemented_by: claude/opus
started_phase: "1"
decisions_for_review:
    - |-
        Compose setters keep reaching transcluded child documents, and each
        document reads a setter through its own schema. The plan (rule 16) chose
        this; the spec does not cover the case. `md compose p.md v=7` already
        changes a child's `v` today, because options flow down to children, so
        keeping it preserves current behavior. One new consequence: a nested
        setter that meets a scalar parent or an out-of-range index in a child
        fails the whole run and names that child.
        Alternatives: apply setters to the root document only, which makes
        children predictable but silently changes what `v=7` does today; or skip
        a child where the path cannot apply, which never fails on a child but
        hides a setter that did nothing.
        To choose another option, change the setter plan's application step in
        the pipeline and preflight callers, plus the child-document tests. The
        path parser, value parser, and text editor would not change.
    - |-
        The spec's phrase "recursive merge behavior of those older options" is
        wrong about `--set`, and later phases follow the code. `--set` replaces a
        top-level key whole and only `--state` deep-merges, as
        `compose/util.rs` shows. The plan called for this correction. Neither
        option changes, and the spec stays as written because a spec is a
        snapshot; current docs will describe the real behavior.
        The alternative was to make `--set` merge recursively so the spec's
        wording became true, but that changes the behavior of an existing option
        and the spec puts both options out of scope.
        If the reviewer wants `--set` to merge, the change is confined to the
        `--set` block of `prepare_frontmatter_for_compose` and its tests, and the
        documented precedence examples would need rewriting.
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
message_to_agent: |-
    Phase 1 changed no code, and the baselines are green on macOS. One existing test
    runs close to the 5 s L1 budget: compose_shell_template::every_suggested_rule_approves_the_command
    took 2.8 s alone and 4.3 to 5.5 s in the full darkmatter suite while the host was
    busy. If it goes over again, rerun it alone before suspecting your change, but do
    look at it if your compose changes make it slower when run alone. The shell here is
    zsh, so PIPESTATUS is empty: capture `just` exit codes without a pipe. biscuit-file
    `just test` runs two suites, so read every Summary line rather than the last one.
---

# Implementation Log for 2026-11-06-dotted-paths (10 phases)

## Phase 1

Phase 1 changes no code. It records the contract the later phases implement and
establishes the test baseline they are measured against.

### Contract recorded from the plan

`plan.md` is the authority for the detail of each rule; this digest is for
orientation.

| Rule | Contract | Phase |
|---|---|---|
| 1 | `PropertyPath` in `simplified-schema`: `\.` and `\\` are the two escapes; the whole argument `''` is one empty segment; other empty segments are an error; printing round-trips | 2 |
| 2 | `PropertyPath::resolve` walks a value; a canonical decimal is an index only on an array; overflow is out of range; a `null` or scalar cannot be traversed; conversion to `PathStep` and `JsonPointer` needs the walk | 2 |
| 3 | Pure `set_at` and `RemovalPlan`/`remove_at` with `PathEditError`; mapping parents are created; a digit under a missing parent is an error; emptied parents stay | 2 |
| 4 | A path syntax failure is a usage error, exit 2; an edit failure or rejected value is a run failure, exit 1 | 4 |
| 5 | `SetterValue::parse`: JSON5, then YAML flow for a value starting with `{`, then text | 2 |
| 6 | Parsed containers are admitted at the token level through `json5_scan` and `numeric::admit_json5_text_numbers`; a bad container fails and never becomes text | 2 |
| 7 | `SetterValue::bind` converts through the declared `Shape`; with no declaration it keeps today's JSON5 guess | 2 |
| 8 | `hash/path_edit.rs` edits nested nodes and refuses forms it cannot edit safely | 3 |
| 9 | Every edit is reparsed and compared with the expected values before any write; `WouldChangeUnnamed` has no fallback; `UnsupportedForm` falls back to verified serialization | 3, 4 |
| 10 | `md get` stays schema-free and flat; paths are parsed before the document loads | 4 |
| 11 | `md set` blocks on a problem at or under the changed path, or on any new problem; `$schema` only checks its reference | 4 |
| 12 | `md rm` plans against the original; conflicts and missing targets cancel the command; array removals use original indexes | 4 |
| 13 | `compose::setters::classify_token` is the one setter/path classifier, shared by execution and completion | 5 |
| 14 | An ordered `SetterPlan` replaces the setter map; overlaps resolve by specificity, before any document is read | 5 |
| 15 | `apply_setter_plan` runs per document right after schemas are prepared; an empty plan costs nothing; no retry | 5 |
| 16 | Setters keep reaching transcluded children, bound per document (see below) | 5 |
| 17 | Provenance and caller origins cover only a setter's own path | 5 |
| 18 | `MarkdownError::Setter` is reported as `dm.setter.invalid` with `source: null` | 5 |
| 19 | One neutral `path_candidates` function serves completion; no baseline union over a valid effective schema | 6 |
| 20 | The CLI adapter alone reads process arguments; every failure means fewer candidates and no output | 7 |
| 21 | Custom `EnvCompleter`s only where the built-in shell scripts fail | 7 |
| 22 | Tests are tiered and hermetic; an absent shell is recorded as unproven, never as passing | 8 |

### Planning decision: setters and transcluded children (rule 16)

Setters keep reaching transcluded children, and each document binds a setter
through its own schema. The spec does not cover this case. A nested setter that
hits a scalar parent or an out-of-range index in a child fails the run and names
that child. This is listed in `decisions_for_review`.

### Correction to the spec

The spec's "recursive merge behavior of those older options" describes `--state`
only. In `prepare_frontmatter_for_compose` (`darkmatter/lib/src/markdown/compose/util.rs`),
`--state` goes through `deep_merge` (line 177), while `--set` overwrites each
top-level key with `fm.insert` (line 191). I confirmed this by reading the code
on 2026-11-06 and did not rerun the commands; the plan records the runtime
observation. Neither option changes. The spec is left as written.

### Baseline results (macOS, 2026-11-06)

`just test` in each area, before any code change:

| Area | Result | Exit |
|---|---|---|
| `biscuit-file` | 1187 passed, 0 skipped; then `test-minimal`: 6 passed, 67 filtered out by that recipe | 0 |
| `simplified-schema` | 1401 passed, 0 skipped; then a second run: 1213 passed, 0 skipped | 0 |
| `darkmatter` | 13979 passed, 10 skipped | 0 |

The 10 darkmatter skips are excluded by the recipe's selection, in both runs. I did
not enumerate them.

### Gates

The scope check does not allow skipping the gates. Its output for the phase's plan
and log edits was `packages: [darkmatter, darkmatter-cli]`, `change_class: package`:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

`just lint` and `just test` in `darkmatter/` were therefore run after the plan and
log edits. Lint exited 0. Tests exited 0 with 13979 passed and 10 skipped.

### Load-sensitive test

`darkmatter-cli::l1 compose_shell_template::every_suggested_rule_approves_the_command`
passed in all three runs but went over the 5 s L1 budget in one of them:

| Run | Duration |
|---|---|
| first full suite | 4.341 s |
| second full suite, host load about 37 | 5.522 s |
| alone, host load about 32 | 2.819 s |

No code changed in this phase, so this phase did not cause it. Other work on the
host, including Blender, accounted for part of the load. I left the test as it is.

### Test design and other checks

- **Requirement-to-test mapping:** none. This phase changes no behavior and adds no
  tests; later phases carry the mapping.
- **Input Robustness Matrix:** not applicable to this phase, since it adds no reader.
- **Cross-OS check:** not run; a Markdown-only change carries no OS risk.
- **Spec:** unchanged. This phase names no spec frontmatter to set.
- **Skill:** the `darkmatter` skill is unchanged; nothing it records has changed.
