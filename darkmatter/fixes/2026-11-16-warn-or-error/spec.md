---
kind: fix
created: 2026-11-16
status: draft-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-07-31-error-handling-transclusions
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
implemented: false
---

# Warn or error: one default, one verb, one report

## Why this exists

Composing a document with an empty link target failed with an error that said
nothing useful:

```text
❯ md compose docs/topics/config-files.md
 ReferenceError: file reference failure
┃ file reference syntax is invalid: empty reference string
┃ failure: invalid-reference
┃ Check sigil usage: @ magic, & repository root, ^ repository-scoped.
```

The cause was one `[Expression Engine]()` on line 49. The reference check
passed every link target to `FileReference::new`, which rejects an empty
string, and the `?` in
[resolve_transclusion_target](../../lib/src/markdown/reference/mod.rs)
propagated the bare error. The error named no file, line, or link, gave a hint
about sigils that had nothing to do with the problem, and stopped the whole
compose.

While fixing that, four broader problems surfaced:

1. **The wrong defaults.** A hyperlink, image, or transclusion whose target is
   missing fails `md compose` by default. A broken link should not stop a
   document from composing; it should warn, and fail only when the author asks.
2. **Both verbs on one identifier.** Each recovery in the
   [policy registry](../../cli/src/args/policy.rs) (`PolicyKind::Recovery`)
   accepts both `--allow` and `--deny`. The verb a user needs depends on a
   default they cannot see, and `--allow X --deny X` needs a conflict rule
   to exist at all.
3. **The same finding reported twice.** A missing transclusion is reported
   once by the passive reference check (located, naming the construct) and
   again by the transclusion engine when it removes the directive (unlocated,
   `I/O error: File not found: ./absent.md`). This happens for `::file` and
   `::code`, in the root and in transcluded children. It predates this fix:
   the installed `md` does it under `--allow missing-transclusions`.
4. **A recovery that does not recover.** A missing `prologue:` (and presumably
   `epilogue:`) target fails with `TransclusionError: I/O failure` even with
   `--allow missing-transclusions`, although the policy docs say that recovery
   covers transclusions and the reference model classifies prologues as
   `ReferenceKind::Transclusion`.

## Supporting material

These sidecars hold the detail; this spec holds the decisions.

- [inventory.md](./inventory.md): every identifier and code that `--allow` and
  `--deny` offer (the same list as `--help` and the shell completions),
  whether it does anything, its default, and a proposed disposition. Includes
  the duplicates that sit on both lists.
- [naming.md](./naming.md): the two naming schemes in use, where the `dm.`
  codes travel (data envelopes, DMLS, `simplified-schema`, Claudine), and
  options for dropping `dm.` and for namespacing the identifiers.
- [audit/registry.md](./audit/registry.md) and [audit/codes.md](./audit/codes.md):
  the evidence for the above, with file and line for each claim and probes run
  against the built `md` at HEAD `baee016e5`.

## Decisions

### D1. A missing or empty reference target warns by default

For hyperlinks, images, and transclusions (`::file`, `::code`, `::url`,
`prologue:`, `epilogue:`):

- a target that does not resolve, or is empty (`[text]()`, `![alt]()`), is a
  **warning**; composition continues;
- a link or image keeps its text; a transclusion directive is removed;
- the warning names the construct and its authored location. An empty target
  quotes the construct, because the target itself is empty:

  ```text
  hyperlink `[Expression Engine]()` has no target (line 49, column 42)
  ```

- `md validate refs` has no policy and keeps reporting a missing target as an
  **error**, so it still exits 1 on broken references.

Targets of any other kind (CSS, script, and font imports, `<video>`,
`<audio>`, `<source>`, `<iframe>`) keep failing composition, as today.

### D2. Each identifier takes exactly one verb, fixed by its default

- A problem that **errors by default** is relaxed with `--allow <id>`. Its
  identifier is never accepted by `--deny`.
- A problem that **warns by default** is made blocking with `--deny <id>`. Its
  identifier is never accepted by `--allow`.
- Using the other verb is a usage error (exit 2) that names the verb the
  identifier takes, for example:
  `` `missing-hyperlinks` warns by default; use `--deny missing-hyperlinks` to make it fail ``.

Expected identifier inventory after this fix (`md compose`):

| Identifier | Default | Verb | Code |
|---|---|---|---|
| `missing-hyperlinks` | warn | `--deny` | `dm.reference.missing_hyperlink` |
| `missing-image-refs` | warn | `--deny` | `dm.reference.missing_image` |
| `missing-transclusions` | warn | `--deny` | `dm.reference.missing_transclusion` |
| `ctx-override` | error | `--allow` | `dm.context.ctx_override` |
| `invalid-frontmatter-assignment` | error | `--allow` | `dm.transclusion.invalid_frontmatter_assignment` |
| `reassigned-frontmatter-property` | error | `--allow` | `dm.transclusion.reassigned_frontmatter_property` |
| `shell-timeout` | error | `--allow` | `dm.shell.timeout_recovered` |
| `untyped-map-variables` | error | `--allow` | `dm.collection.untyped_map_source` |
| `untyped-filter-variables` | error | `--allow` | `dm.collection.untyped_filter_source` |

`host=HOST` and the `--disable` features already follow the rule and do not
change. Each recovery's code is also accepted by `--deny` today, as a second
spelling of its identifier; [naming.md](./naming.md) covers whether the
identifier or the code survives. [inventory.md](./inventory.md#duplicates-on-both---allow-and---deny)
shows that the second verb on each recovery does work only as a carve-out from
`all`.

### D5. `--deny` offers only what it can change

An entry stays deny-able only if `md` reports it as a **warning** that the
denial can block. Per [inventory.md](./inventory.md):

- remove the 11 codes that are always errors (`dm.expression.incompatible_call`,
  `parse_failure`, `evaluation_failure`, `invalid_callback`, the five
  `dm.collection.*` error codes, and the two `dm.iteration.*` limits);
- remove the 2 codes `md` never emits (`dm.expression.incomplete_analysis`,
  DMLS-only; `dm.schema.missing_simplified_envelope`, dead, so delete its code
  path too);
- remove `style` from `md compose` (never consulted there), unless compose is
  given style warnings to judge;
- add a registry guard test: every deny-able entry has a test showing it as a
  warning that the denial blocks, and every `--allow` recovery has a test
  showing the default failing and the allowance recovering it.

### D6. Naming

Pending the author's answers to the questions in [naming.md](./naming.md):
drop `dm.` from every code (recommended), whether each problem keeps one name
(the code) or two, the case of the final segment, and whether
`--deny <namespace>` becomes a general rule replacing the `style` family.

### D3. One finding, one report

A missing target that both the reference check and composition find is
delivered **once**, in the reference check's located form. A finding that only
composition can see (a target computed while composing, which the passive
check defers) is still reported by composition.

This must be structural, not a text match. The codebase already has the
pattern: `PendingTriggerWarnings` in
[compose.rs](../../cli/src/commands/compose.rs) holds skipped-trigger warnings
found by an earlier pass and merges each one once, and `ComposeWarning` has an
`identity` field (`WarningIdentity`) for declaring which finding a warning
reports. The recovered-transclusion warning should carry an identity
(document, authored directive position, target) that the reference check's
issue for the same directive shares, and delivery should merge by identity.

Before designing, audit the other warning families for the same overlap
(reference check versus composition); the transclusion case may not be the
only one.

### D4. Prologue and epilogue follow `missing-transclusions`

A missing `prologue:` or `epilogue:` target is a missing transclusion: it
warns by default, the reference is dropped, and `--deny missing-transclusions`
makes it fail.

## State of the branch (2026-11-16)

Work started on branch `fix/sequences` before this spec was written, in the
Claude Code session that also wrote this spec. It was committed as `11a909a82`
("fix(darkmatter): report empty reference targets through policy"), together
with unrelated editor-test changes from other work
(`darkmatter/cli/tests/l1/editor_*.rs`, `compose_edit_action.rs`). Everything
below is in that commit.

### Changes made

**Library (`darkmatter/lib`)**

- [validate.rs](../../lib/src/markdown/reference/validate.rs):
    - new `ReferenceIssueCode::EmptyTarget` (reason phrase "no target");
    - new `empty_target()`: an empty hyperlink or image target (`[text]()`,
      `![alt]()`, `<a href="">`) is a `Warning` issue whose message and
      `reference_display` quote the construct; an empty target of any other
      kind is an `Error`. The validation loop calls it before
      `validate_local_path`, so an empty target no longer reaches
      `FileReference::new("")`;
    - new `ReferenceIssue::is_reported_warning()` (a missing or empty target at
      warning severity);
    - `ReferenceIssue::describes` matches an `EmptyTarget` issue to its record
      even though `reference_display` is the construct, not the raw target;
    - unit test `empty_link_and_image_targets_warn_and_quote_the_construct`.
- [report_diagnostics.rs](../../lib/src/markdown/compose/report_diagnostics.rs):
  `reference_issue_diagnostic` gives `EmptyTarget` the kind's missing-reference
  code (`dm.reference.missing_hyperlink`, `…missing_image`).
- [file_tree/model.rs](../../lib/src/markdown/reference/file_tree/model.rs):
  labels an `EmptyTarget` issue `[no target]`.

**CLI (`darkmatter/cli`)**

- [policy.rs](../../cli/src/args/policy.rs):
    - new `DEFAULT_RECOVERIES` (`missing-hyperlinks`, `missing-image-refs`,
      `missing-transclusions`);
    - `ResolvedPolicy::recovers` now returns true for a default recovery unless
      it is denied by name or code, or `--deny all` is given. Before, a
      recovery applied only when allowed by name or by `--allow all`;
    - the three entries' help summaries now say "(the default; `--deny` blocks
      it)", which reads oddly under `--allow`.
- [compose.rs](../../cli/src/commands/compose.rs):
    - `check_references` downgrades an error-severity `MissingLocalTarget` whose
      kind the policy recovers to a warning, and uses `is_reported_warning`
      when deciding what to report and what the blocking policy judges;
    - `render_reference_warnings` prints the authored line and column, says
      "inside inserted text" only when the reference is, and names the file of
      a transcluded document;
    - new `reference_check_reported`, called after composition, drops a
      compose warning with code `dm.reference.missing_transclusion` whose
      message contains a target the reference check already reported (also
      called from [compose/lifecycle.rs](../../cli/src/commands/compose/lifecycle.rs)).
- [output/report.rs](../../cli/src/output/report.rs): the data-output sink
  records reference warnings through `is_reported_warning`.
- Tests:
    - new `policy_verbs::an_empty_link_target_warns_with_the_link_and_blocks_only_when_denied`;
    - `output_envelope.rs`: the `BROKEN` fixture and
      `the_problem_report_is_plain_and_unwrapped_on_a_pipe` switched from a
      missing link to a missing image, and the expected code to
      `dm.reference.missing_image`.

**Docs (`darkmatter/docs`)**: `cli/policy.md` (empty targets carry the
missing code and need no allowance), `cli/compose.md` (an "empty link target"
bullet), `cli/validate.md` (the `no target` reason), `inline/link-validation.md`
(the `EmptyTarget` row), `structs/Markdown.md` (the issue-code list).

### Regressions and breakage introduced by `11a909a82`

1. **Malformed references compose silently.** `check_references` refuses a
   document only for an error the policy does not recover, and it judges that
   by the reference's *kind* (`recovers_missing(policy, kind)`), not its
   *code*. With the missing-reference recoveries now on by default, every
   error on a hyperlink, image, or transclusion passes: `InvalidUrl`, a
   missing fragment, `RemoteUnreachable`. `[a](http://[::1)` composes with
   exit 0; `md validate refs` fails on it. Before the commit this leak existed
   only under an explicit `--allow missing-*` or `--allow all`. Fix: only a
   missing or empty target may be recovered.
2. **48 CLI L1 tests fail.** The commit changed the default but did not
   migrate the tests that use a missing link, image, or include as their
   example of a failing compose (see [Tests](#tests)).
3. **The `output_envelope.rs` image swap is wrong.** It replaced a missing
   link with a missing image to keep a failing fixture, but a missing image
   also warns by default now, so those tests still fail.
4. **`--allow missing-*` became a no-op** except next to `--deny all`, while
   help still lists them under `--allow`.
5. **`--deny missing-*` changed meaning.** Before, it blocked only the
   warning-severity variants (a missing link inside inserted text). Now it also
   turns the default recovery off, so a missing target fails again.
6. **Docs now contradict the code.** `docs/cli/policy.md` still says a missing
   reference fails without the allowance and shows `--allow missing-hyperlinks`
   as needed; `docs/cli/compose.md` says compose exits 1 on a missing target;
   the `use-darkmatter` caller skill lists the `missing-*` entries as `--allow`
   recoveries; the precedence flowchart has no default-recovery branch.

### Backed out on `feat/state-schema` (2026-11-17)

To keep that branch's pull request green, the "Replace under D2" parts of
`11a909a82` were reverted there: `DEFAULT_RECOVERIES`, the `recovers`
change, the severity downgrade in `check_references`, and the three help
summaries. A missing target fails by default again, as before the commit,
which clears regressions 1–5 above. The "Keep (D1)" parts stay. D2 starts
from that state.

### What to do with each change

- **Keep (D1, empty targets):** `EmptyTarget`, `empty_target()`,
  `is_reported_warning()`, the `describes` change, the diagnostic-code
  mapping, the `[no target]` label, the `render_reference_warnings`
  location changes, and both new tests.
- **Replace under D2:** `DEFAULT_RECOVERIES`, the `recovers` change, the
  severity downgrade in `check_references`, and the three help summaries. D2
  should give each registry entry a declared default, and with it a single
  verb.
- **Remove (superseded by D3):** `reference_check_reported` and its two call
  sites. It matches on message text and masks the duplication instead of
  fixing it.
- **Revert:** the `output_envelope.rs` image swap; migrate those tests as
  described under [Tests](#tests).
- **Revise:** the five docs pages above, for D1 through D6.

## Defects found by the audit

Fix these in this change; each is cited in [audit/registry.md](./audit/registry.md)
or [audit/codes.md](./audit/codes.md).

1. **Malformed references compose silently.** The regression from
   `11a909a82` described in
   [Regressions and breakage](#regressions-and-breakage-introduced-by-11a909a82),
   item 1.
2. **Hyperlink and image policy apply to file input only.** The reference check
   runs only for a file argument, so on stdin `--deny missing-hyperlinks` and
   `--deny missing-image-refs` are accepted and do nothing.
3. **A denial can fail under another code.** With its recovery off,
   `ctx-override` fails as `dm.compose.failure` or `dm.schema.type_mismatch`,
   and the two `set=` recoveries fail as `dm.transclusion.failure`. A failure
   should carry the code of the problem the user named.
4. **A vacuous test.** The compose leg of
   `policy_verbs::deny_style_and_deny_all_block_a_style_issue_on_every_render_path`
   passes only because `md compose --output html` is itself a usage error.
5. **A wrong flag in an error message.** `lib/src/markdown/compose/context/merge.rs:36`
   tells the user to pass `--allow-override`, which never existed. The flag is
   `--allow ctx-override`.
6. **Docs gap.** `--no-trigger-schemas` is in `REMOVED_FLAGS` but missing from
   the removed-flags table in `docs/cli/policy.md`.

## Tests

With the interim defaults, 48 CLI L1 tests fail (run with
`cargo nextest run -p darkmatter-cli --test l1 -E 'not test(/editor|compose_edit/)'`).
Almost all use a missing hyperlink or include as their example of a compose
that fails:

- `output_envelope`: `BROKEN` fixture, the `broken_reference_*` and
  `streams_compose*::failure` families, the `parity_compose_failure_*` and
  `conforms_*` tests, `the_problem_report_is_plain_and_unwrapped_on_a_pipe`;
- `compose_file_interpolate`: the reference-gate and inherited-value tests;
- `compose_lifecycle`: `a_missing_include_runs_failed_then_finalize` and two
  others that need a failing include;
- `compose_data_output`, `compose_report_diagnostics`,
  `compose_value_provenance`, `invalid_trigger_warnings`, `literal_glob_hint`,
  `magic_root`, `reference_report`, and
  `policy_verbs::each_missing_reference_allowance_recovers_its_own_kind_only`.

Sort each test by what it is about:

- **Any compose failure:** use a problem that still errors by default (a
  missing CSS import or `<video>` target has no recovery), or add the matching
  `--deny`.
- **Missing-reference reporting:** keep the fixture, and assert the warning by
  default plus the failure under `--deny`.
- **Policy verbs:** rewrite for D2 (one verb per identifier, the usage error
  naming the right verb).

Add coverage for D3 (each missing `::file` and `::code`, root and child,
reported exactly once, in both the formatted and data outputs, with and
without a lifecycle) and D4 (prologue and epilogue warn by default, and fail
under `--deny`).

## Documentation

- `docs/cli/policy.md`: the verb table, the recoveries table, the precedence
  flowchart and conflict examples (most conflicts become impossible under
  D2), and `--allow all`/`--deny all`.
- `docs/cli/compose.md`: the "Validation and Errors" section and its
  `--allow missing-*` examples.
- `docs/topics/hyperlinking.md`: the `--allow missing-hyperlinks` example.
- `docs/use-skills/use-darkmatter/cli.md`: the `--allow` row lists the
  missing-reference recoveries; republish with `just publish-skills`.
- `md compose --help` and completions come from the registry and follow it.

## Open questions

1. **`--deny all` exemptions.** Today `--deny all --allow missing-hyperlinks`
   exempts hyperlinks from `--deny all`. Under D2 `--allow` cannot name a
   warn-by-default identifier. Does a default-warn identifier need an
   exemption from `--deny all`, and if so, how is it spelled?
2. **`--allow all`.** Under D2 it would only cover default-error recoveries.
   Confirm.
3. **Message wording.** A missing target's warning reads
   `Missing local target: ./gone.md`. Should it quote the construct the way an
   empty target does (``hyperlink `[gone](./gone.md)`: file not found``)?
4. **Summary counts.** `ReferenceSummary.broken` counts error-severity issues
   only, so a run whose broken links all warn reports `0 broken`. Should
   warnings count as broken?
5. **Naming.** The four questions at the end of [naming.md](./naming.md).
6. **`style` on compose.** Remove it from compose, or give compose style
   warnings for it to judge (D5)?

## Success criteria

- `md compose` on a document with empty or missing hyperlink, image, and
  transclusion targets (including `prologue:` and `epilogue:`) exits 0, writes
  the document, and reports each problem once, located, naming the construct.
- `--deny missing-hyperlinks`, `--deny missing-image-refs`, and
  `--deny missing-transclusions` each make their kind fail with exit 1 and
  write no document.
- Every policy identifier accepts exactly one verb; the other is a usage error
  naming the right one.
- `md validate refs` still exits 1 on a missing target.
- The docs pages above and the `use-darkmatter` caller skill describe the new
  behavior.
