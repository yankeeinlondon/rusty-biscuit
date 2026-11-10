---
$schema: feature-review.yaml
ready: false
findings:
    - 'R1-F1 (high): Setter binding and completion use the wrong baseline selection'
    - 'R1-F2 (high): Declared values bypass the shared conversion rules'
    - 'R1-F3 (high): Union selection loses the origin of nested file setters'
    - 'R1-F4 (high): Elvish and PowerShell lack the required shell verification paths'
human_review: true
human_review_items:
    - Confirm retaining setters on transcluded children, with each child using its own schema and incompatible paths failing the run.
    - Confirm retaining existing whole-key replacement for --set and recursive merging for --state; the spec described the former incorrectly.
    - Review the decision to guess declared unknown values; R1-F2 recommends restoring the spec's text-first conversion.
    - Confirm treating tagged, anchored, and aliased YAML flow arguments as text without executing them.
    - Confirm retaining inadmissible bare numbers as text instead of admitting an inexact number.
    - Review bypassing conversion for quoted strings; R1-F2 recommends converting the decoded value through its declared type.
    - Confirm accepting YAML flow shorthand such as {foo} as a mapping whose value is null.
    - Confirm the added conflict error, JSON5 grammar helper, map-rooted edit APIs, and reuse of the existing typed path steps.
    - Confirm keeping serialization fallback in the library and preserving original fences and body bytes.
    - Confirm allowing whole-flow-container replacement and insertion into empty mappings without serialization fallback.
    - Confirm recognizing orphaned aliases by the YAML parser's unknown-anchor message, with a regression test guarding that dependency.
    - Confirm refusing every unnamed-value change even when the source contains no alias.
    - Confirm the documented choices for retaining preceding comments and moving comments when a replacement becomes multiline.
    - Confirm allowing valid pending expressions in md set without evaluating them.
    - Confirm preserving native true and false under boolish while keeping On and yes as text.
    - Review binding through the compiled baseline despite a custom or disabled baseline; R1-F1 recommends honoring the selected baseline.
    - Confirm accepting valid inline schemas as well as schema-reference strings without opening the referenced file during that edit.
    - Confirm retaining the existing missing-property wording and aggregate missing-path report for md rm.
    - Confirm the shared schema/diagnostic helpers and launch-directory trigger context for stdin edits.
    - Confirm retaining schema diagnostic codes while naming the setter and removing the old document position.
    - Confirm keeping source null on setter application errors and naming affected documents in the message.
    - Review the deferred nested-file origin and portable-output work; R1-F3 requires repairing the demonstrated origin failure.
    - Confirm reporting every offending unknown key in one validation problem rather than splitting it into several diagnostics.
    - Confirm returning setter provenance from application, retaining the first duplicate's position and the last duplicate's value.
    - Confirm using merged schema layers for completion rather than the document-only helper named in the plan.
    - Confirm the additional completion query for md set values and the separate indication that a value accepts files.
    - Confirm returning whole-word candidates and removing the key prefix only in Bash's adapter.
    - Confirm suppressing completion paths that the current scalar, null, or array bounds make impossible to execute.
    - Confirm offering both descent and whole-container replacement for object and container-item setters.
    - Confirm ignoring a tag's current value when suggesting replacements for that same tag.
    - Confirm quoting completion values only when plain insertion would change the intended scalar.
    - Confirm generating Bash and Zsh scripts from upstream scripts using edits guarded by a test.
    - Confirm the extended Bash word joining and its documented ambiguity after a deliberately empty earlier setter.
    - Confirm shell-unquoting Bash/Zsh words and retaining Fish's documented limitation inside an open quote.
    - Confirm adding a trailing space directly to finished Bash 3.2 candidates.
    - Review ignoring baseline and trigger-disable flags during completion; R1-F1 recommends honoring those flags.
    - Confirm the separately guarded request snapshot at completion's early-exit entry point.
    - Confirm completing file arguments containing equals when the shared classifier identifies them as paths.
    - Confirm landing completion subprocess tests one phase earlier than planned.
    - Confirm combining counters, observed resolver reads, and code inspection for the read-once proof.
    - Review optional-shell prerequisites and omitted Elvish script tests; R1-F4 recommends adding the missing executable verification paths.
    - Confirm explicitly typing the dot after Tab reaches the common prefix shared by descent and replacement candidates.
    - Confirm leaving md schema validate on its separate, out-of-scope assignment grammar.
    - Confirm keeping migration examples on the individual command pages.
    - Review documenting untested shell behavior as likely; R1-F4 requires a measured limitation or the appropriate test path.
    - Review the implementation log's accidental git init in another repository and determine whether its overwritten Git configuration needs restoration separately.
    - 'Iteration 1 repair (R1-F1): md compose completion follows md compose and ignores BASELINE_SCHEMA; the environment-selected custom baseline applies to get/set/rm completion only.'
    - 'Iteration 1 repair (R1-F1): with no document or a failing document schema, completion answers from the selected baseline (none when disabled); the compiled baseline remains only for the built-in selection or a custom baseline that cannot load.'
    - 'Iteration 1 repair (R1-F1): raw JSON Schema baseline, document, and trigger layers now declare types through the existing RawDeclarations projection; PathQuery.effective became PathQuery.schema (PathSchema).'
    - 'Iteration 1 repair (R1-F2): container syntax under unknown is parsed, so {a:1} is an object in md set and md compose; quote it to keep text.'
    - 'Iteration 1 repair (R1-F2): a key matched by "<string>": unknown keeps text, so author.mistake=1 is now the text "1".'
    - 'Iteration 1 repair (R1-F2): a raw JSON Schema unknown keeps unquoted text only when the validator accepts that exact text there; a quoted number behind raw allOf stays refused, as an authored one is.'
    - 'Iteration 1 repair (R1-F2.16/F2.22): simplified-schema partial_copy no longer sets the root aside, removing a false type mismatch beside a missing undeclared required key in md schema validate as well as md set.'
    - 'Iteration 1 repair (R1-F3): a caller-supplied file that does not resolve from its launch origin is refused even when the document folder holds that name, at the top level too; templates keep the document-origin rule after rendering.'
    - 'Iteration 1 repair (R1-F3.20): an authored copy of a supplied absolute path gets a second validation without supplied values, so match() still judges it from the document.'
    - 'Iteration 1 repair (R1-F3.21): nested caller files render in the portable spelling on native Windows, as top-level ones do; four tests that expected the native spelling changed.'
    - 'Iteration 1 repair (R1-F3.22): Claudine pre-validation leaves every nested caller-file verdict to composition instead of dropping the object, so such a launch can fail after pre-flight.'
    - 'Iteration 1 repair (R1-F4): the printed Elvish setup line and startup path changed (Elvish 0.21), and the printed PowerShell line removes the built-in md→mkdir alias; Elvish needs Enter per candidate (measured, documented).'
    - 'Iteration 1 repair (R1-F4): Elvish installed on the macOS dev host via Homebrew for measurement; the ConPTY PowerShell gate uses require_level! string labels, and xpty is a new Windows-only dev-dependency of darkmatter-cli.'
    - 'Iteration 1 repair (R1-F4): a host without fish or Elvish still reports those L1/L2 tests as nextest PASS while printing unproven, matching the earlier fish pattern; CI hosts have no Elvish.'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-06T21:54:30-08:00
spec: 2026-11-06-dotted-paths/spec.md
implemented: true
description: 'A **feature** review of `2026-11-06-dotted-paths/spec.md`'
feature: 2026-11-06-dotted-paths/review-1.md
next: 2026-11-06-dotted-paths/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-06-dotted-paths/log.md
implemented_by: claude/opus
---

# Review 1: Dotted Paths

The feature is not production ready. Four findings need repair. None requires an author decision before repair: the recommendations below stay within the spec. This is the first implementation review, so there are no previous findings to close or classify as partial repairs.

The review covers the implementation across its feature commits and current working-tree additions, including the shared parser and value adapter in simplified-schema, Darkmatter's editor and setter pipeline, and the CLI completion adapters. Two independent agents read the spec and implementation log in parallel to extract decisions and deferred work before the code review.

## Findings

### R1-F1 (high): Setter binding and completion use the wrong baseline selection

**Authority: spec.** A declared key must pass its raw text through its declared type, and completion must use the effective schema's “runtime baseline, schema triggers, and its `$schema`.” The spec also says: “Never union baseline type/value suggestions back into a valid effective schema.” Custom and disabled baselines are existing supported inputs, not additional feature scope.

**Class:** schema consumers substitute compiled defaults for the schema actually selected by the request.

In the darkmatter package, [declared_shape](../../lib/src/markdown/schemas/completion.rs:421), which finds the type used to read a setter, calls a property-shape projection that always starts with the compiled baseline. Custom-baseline-only declarations disappear; disabled compiled declarations remain. The same projection supplies completion keys and values. In darkmatter-cli, [record_flag](../../cli/src/args/path_completion.rs:169) retains only magic roots and object overrides, and [resolve_schema](../../cli/src/args/path_completion.rs:272) does not receive baseline/trigger policy from the command line.

Reproduction: create `base.yaml` containing `$schema: {version: string, status: "enum(custom)"}` and a document containing only `title: T`. From that directory:

```sh
md compose doc.md version=1.10 --baseline-schema base.yaml --frontmatter
BASELINE_SCHEMA=./base.yaml md set doc.md version 1.10 --output markdown
COMPLETE=fish md -- md compose --baseline-schema base.yaml doc.md 'status='
```

The first writes `version: '1.1'` into the composed result; the second prints `version: 1.1`; the third offers no `status=custom` candidate. The intended string is `1.10`. Later validation cannot recover digits already discarded by guessing.

The bounded sweep followed the selected schema into both mutation consumers and both completion decisions, including policy selection:

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Compose binding | Custom baseline declares `version: string`; raw `1.10` | String `1.1` | String `1.10` |
| R1-F1.2 | Set binding | Same baseline via `BASELINE_SCHEMA` | Number `1.1` | String `1.10` |
| R1-F1.3 | Compose binding | Baseline disabled; document declares another key; `title=1.10` | Compiled `title` type still keeps a string | Undeclared numeric guess `1.1` |
| R1-F1.4 | Completion schema keys/value lookup | Custom baseline `status: enum(custom)`, selected by flag | No `status=custom`; built-in style values remain | Selected baseline's candidates |
| R1-F1.5 | Completion schema keys/value lookup | Same custom baseline via environment | Same missing custom value and retained built-in values | Selected baseline's candidates |
| R1-F1.6 | Completion policy | `--disable baseline-schema`, query `style.page.alignment=` | Offers built-in `left`, `center`, `right` | No candidates from the disabled baseline |
| R1-F1.7 | Completion policy | Trigger declares `edition: enum(stable)`; `--disable schema-triggers` | Still offers `edition=stable` | No candidates from the disabled trigger |
| R1-F1.8 | Binding control | Document `$schema` declares `version: string` | Keeps `1.10` in set and compose | Clean |
| R1-F1.9 | Completion control | Separate document schema declares `status: enum(custom)` | Offers `status=custom` | Clean |
| R1-F1.10 | No-schema completion fallback | No resolvable document/effective schema | Compiled baseline offered by explicit fallback branch | Clean; the spec authorizes this fallback |
| R1-F1.11 | Set binding | Baseline disabled through `DARKMATTER_NO_BASELINE_SCHEMA`; document declares another key; `title` set to `1.10` | Compiled declaration still produces a string | Undeclared numeric guess `1.1` |
| R1-F1.12 | Get/set completion consumers | Custom baseline via environment; get key prefix `sta` and set value for `status` | Custom declaration/value absent | Offer the custom schema key/value through the common schema query |
| R1-F1.13 | Remove completion consumer | Same custom baseline, absent document key `status` | No candidate | Clean; remove offers present keys only |

**Repair:** expose a conversion/completion view of the selected effective schema, using its actual baseline and preserving its declaration information. Pass the relevant command policy into completion assembly. Keep compiled fallback only where the spec requests fallback. Add focused public-API tests for custom and disabled baselines plus subprocess wiring tests for flags and the environment; keep the existing document-schema controls. Correct the comments and docs describing the limitation when repairing it. No separate baseline implementation or new syntax is needed.

### R1-F2 (high): Declared values bypass the shared conversion rules

**Authority: spec.** “A declared key passes its raw text through the conversion engine”; “Unquoted scalar text goes to the shared conversion engine as text”; and for `md set`, “The new value is converted and checked against its declared type.” The stated exception for quoted strings is decoding once, not skipping declared-type conversion.

**Class:** special branches bypass declared-type conversion, either losing scalar spelling or validating a converted copy while persisting the unconverted value.

In simplified-schema, [SetterValue::bind](../../../simplified-schema/lib/src/setter_value.rs:130), the shared CLI value adapter, filters `Shape::Any` out before binding and returns quoted strings unchanged. Consequently a declared `unknown` receives a guessed number, and `md set` can accept a numeric string after validation converts a temporary copy but write that string rather than the converted number.

Reproduction document:

```yaml
---
$schema:
  x: unknown
  count: number
---
Body
```

`md set doc.md x 1.10 --output markdown` and `md compose doc.md x=1.10 --frontmatter` both produce numeric `1.1`; the conversion engine's `unknown` shape would preserve text `1.10`. `md set doc.md count '"5"' --output markdown` accepts the edit but prints `count: '5'`, whereas composing the same supplied value ultimately yields numeric `5`.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Shared `Text` binding and set consumer | Declared `unknown`, `1.10` | Number `1.1` | Text `1.10` |
| R1-F2.2 | Shared `Text` binding and compose consumer | Same declaration and argument | Number `1.1` | Text `1.10` |
| R1-F2.3 | Shared `Quoted` binding and set persistence | Declared `number`, JSON5-quoted `"5"` | Accepts and writes string `5` | Decode once, convert, write number `5` |
| R1-F2.4 | Compose's later schema conversion | Same quoted number | Final numeric `5` | Clean final value; later conversion masks the adapter bypass |
| R1-F2.5 | Normal declared-text binding | `string`, `number`, scalar union, `numberlike`, and `boolish` rows in `text_goes_through_the_declared_type` | Expected conversion/spelling | Clean controls, apart from the separately recorded compatibility choice for boolean literals |
| R1-F2.6 | Container binding | Object child typed number receives string `5` | Recursive conversion yields number `5` | Clean (`control_a_conversion_inside_a_container_converts`) |
| R1-F2.7 | Null binding | Native null with a declaration | Null retained for validation | Clean; intentional contract |
| R1-F2.8 | Undeclared scalar/quoted binding | No declaration | Scalar guessed; quoted string retained | Clean; intentional contract |
| R1-F2.9 | Completion's scalar spelling decision | `spell_value` calls this same public parser/binder | Inherits the adapter's conversion policy | Recheck this caller with the repaired binder; no second conversion policy |

Existing tests `a_type_declaring_no_structure_guesses_too` and `a_quoted_string_is_not_converted_by_binding` explicitly preserve the departures; passing them does not establish spec compliance.

**Repair:** keep `unknown` as a declared shape and bind a decoded quoted string through the selected declared type. Preserve the quoted-string behavior when no type is declared, native null handling, and the existing union rules. Make `md set` persist the converted value it validated. Update the two contrary tests and add a small set/compose parity check. These are local conversion changes with a spec-defined default, so they need no human ruling.

### R1-F3 (high): Union selection loses the origin of nested file setters

**Authority: spec.** “Existing pending-value and caller-origin rules are retained.” A command-line file value is authored in the launch context; adding a dotted path must not make that same file resolve from the input document's folder instead.

**Class:** schema-arm selection judges caller-supplied descendants as document-authored values before their file projection can be installed.

In darkmatter, [resolve_nested_setter_files](../../lib/src/markdown/compose/schema_validation.rs:901) projects nested file values after collecting applicable schema fragments. That collection first calls [root_schema_arm_applies](../../lib/src/markdown/compose/schema_validation.rs:1350), whose caller records cover whole top-level values, not nested setter records. A union arm containing a named recursive type can therefore be rejected while resolving the unprojected relative value from the document's directory. The later nested projection never gets an applicable fragment.

Use this separate schema file `s.yaml`:

```yaml
kind: schema
types:
  node:
    avatar: file(eager; match(./assets/*.png))
    children: node[]
$schema:
  - kind: literal(image)
    avatar: file(eager; match(./assets/*.png))
    author: node
    authors: node[]
    byid:
      "<string>": node
  - kind: literal(other)
    title: string(required)
```

Place `assets/avatar.png` in the launch directory. Put `sub/doc.md` beneath it with `$schema: ../s.yaml`, `kind: image`, and existing container parents (`author.children: [{children: []}]`, `authors: [{children: []}]`, `byid.primary.children: []`). Run:

```sh
md compose sub/doc.md author.avatar=assets/avatar.png --output json
```

It exits 1 with `dm.schema.invalid_file_reference`, saying it resolved `assets/avatar.png` from `sub`. The file exists at the caller's location and the corresponding top-level setter succeeds.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | Root-arm selection → named object file | `author.avatar=assets/avatar.png` | Rejects, resolves from `sub` | Resolve from launch directory and succeed |
| R1-F3.2 | Root-arm selection → array item file | `authors.0.avatar=assets/avatar.png` | Same rejection | Same success |
| R1-F3.3 | Root-arm selection → dictionary value file | `byid.primary.avatar=assets/avatar.png` | Same rejection | Same success |
| R1-F3.4 | Root-arm selection → recursive child file | `author.children.0.avatar=assets/avatar.png` | Same rejection | Same success |
| R1-F3.5 | Root-arm selection → whole object setter | `author={avatar:"assets/avatar.png",children:[]}` | Same document-origin rejection | Preserve caller origin for the supplied object's file value |
| R1-F3.6 | Existing whole-object `--set` consumer | Equivalent `--set` object containing `author.avatar` | Same document-origin rejection | Same caller-origin resolution; an existing sibling defect |
| R1-F3.7 | Root-arm selection → top-level file | `avatar=assets/avatar.png` with the same schema | Succeeds | Clean control |
| R1-F3.8 | Single-schema inline nested file | The existing `author.avatar` fixture without the root union/named type | Succeeds from launch directory | Clean control; demonstrated by the focused tests and CLI probe |

The sweep follows the value through candidate arm selection, caller projection, and final validation rather than checking the file reader alone. The failure is reached by an ordinary nested image setting in a typed document; it does not require hostile input.

**Repair:** make nested supplied origins available during arm selection and final validation, as well as materialization. Preserve the origin of authored siblings; assigning the caller origin to the entire parent would incorrectly rebase those siblings. Reuse the existing typed setter paths and file-resolution machinery. Add one public-API matrix for these container shapes and a representative CLI test. Whole supplied objects should use the same recursive origin handling; do not add a separate file resolver or special-case avatar keys. The log's deferred origin work has a demonstrated correctness cost, so this part must be completed here.

### R1-F4 (high): Elvish and PowerShell lack the required shell verification paths

**Authority: spec.** The final completion criterion requires the Tab sequence in all six listed shells “or the documented limit for that shell.” The Tests section requires scripts to source in environments providing their shell and states: “A shell limit is a measured limitation with a documented fallback, not an excuse to count an unrun test as passing.”

**Class:** manufactured shell word shapes are used without an executable verification path for the actual shell behavior the requirement describes.

The darkmatter-cli [L1 completion tests](../../cli/tests/l1/path_completion.rs:535) contain no Elvish script-load test. The [L2 shell tests](../../cli/tests/level2/level2_path_completion.rs:1) provide real Tab paths for Bash, Bash 3.2, Zsh, and Fish, but none for Elvish or PowerShell. The docs accurately label those results untested, yet infer Elvish's likely trailing-space behavior instead of measuring a limitation. PowerShell script loading was measured on Windows; that does not exercise accepting and continuing a candidate.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F4.1 | Modern Bash adapter | Word-shape, script load, real Tab sequence | L1 and L2 pass | Clean |
| R1-F4.2 | Bash 3.2 adapter | Same, including its separate spacing branch | L1 and L2 pass | Clean |
| R1-F4.3 | Zsh adapter | Word-shape, script load, real Tab sequence | L1 and L2 pass | Clean |
| R1-F4.4 | Fish adapter | Word-shape, script load, real Tab sequence | L1 and L2 pass on this host | Clean; missing-shell runs still prove nothing |
| R1-F4.5 | Elvish adapter | Manufactured whole-word input only | L1 shape passes; no script-load or real-shell test exists | Executable script-load test and measured Tab behavior or measured/documented limitation |
| R1-F4.6 | PowerShell adapter | Manufactured words and native script-load test | L1 path exists; real-shell candidate acceptance path absent | Platform-appropriate real-shell test or measured/documented limitation |

**Repair:** add narrowly gated tests that run the generated script in Elvish where available and exercise candidate acceptance in Elvish and native PowerShell without taking focus. Use the existing terminal harness. If a shell cannot continue after a candidate, measure that behavior and document the specific manual fallback instead of implementing an unnecessary adapter. Do not add a broad CI matrix or require every shell on every host. The missing test paths are the finding; absence of fresh cross-OS results on this review host is not a separate blocker.

## Verification and input robustness

All review reproductions used the built `md` CLI with temporary documents, schemas, homes, and configuration directories. Findings concern public results, not assertions against private parser internals. The schema and file probes were checked against the implementation's contracts; advisory `match` behavior on a single schema is intentionally not a finding.

The following checks ran during this review:

| Check | Result |
|---|---|
| `just test property_paths::` in darkmatter | 16 passed |
| `just test compose_setters::` in darkmatter | 54 passed |
| `just test path_completion::` in darkmatter | 47 passed |
| `just test path_edit::` in darkmatter | 50 passed |
| `just test get_set_rm::` in darkmatter | 94 passed |
| `just test setter_values::` in simplified-schema | 38 passed with normal features; its recipe also ran 1,292 minimal-feature tests, all passed |
| `just test property_paths::` in simplified-schema | 40 passed with normal features; its recipe also ran 1,292 minimal-feature tests, all passed |
| `just test-l2 level2_path_completion` in darkmatter | Four real-shell tests passed in detached tmux; no window focus taken |
| `just check-tier-coverage darkmatter` | Zero stranded tests |

No selected L1 test reached five seconds. Builds emitted the existing large-unwind-table linker warning. Full-area lint and full-area L1 were recorded by the implementer but were not rerun for this review. No cross-OS proof is inferred from the local runs.

The new L1 and L2 files are declared by their consolidated target roots; the L2 module has its tier marker and an active recipe. The tests run with the relevant fixture, instrumentation, and terminal features supplied by the recipes.

### Requirement-to-verification mapping

| User-visible requirement | Appropriate level | Strongest verification reviewed |
|---|---|---|
| Path escaping, empty key, Unicode, malformed segments, numeric mapping keys, canonical/huge indexes | L1 | Public parser/walker tests plus CLI path tests; passing |
| Missing parents, scalar/null refusal, atomic multi-remove, original indexes | L1 | Public operations and CLI failure-without-write cases; passing |
| Preserved sibling bytes/comments, flow fallback, CRLF, anchors/aliases, body bytes | L1 | 50 editor cases and persisted command round trip; passing |
| Flat multi-get, top-level remaining keys, stdout/save/stdin contracts | L1 | Command tests and output checks; passing |
| Scalar/container parsing, specificity, duplicate setters, required-child combination | L1 | Public adapter and compose command cases; R1-F1 and R1-F2 identify missing/contrary cases |
| Trigger selection, schema validation, unknown keys, supplied-value diagnostics | L1 | Trigger and schema command tests; passing for tested paths; R1-F1 and R1-F3 cover the remaining failures |
| Pending expressions and passive set/completion | L1 | Pending setter tests, command recorder, mock network, no-write tests; passing |
| Completion key sources/order, spelling, values, arrays, unions, fallback, cursor position | L1 | Candidate tests and 47 focused completion tests; selection gap in R1-F1 |
| Read/effect limits and release responsiveness | L1 observations plus one release sample | Counters/read observation test passes; log records one release sample, about 40 ms outside a repo and 80 ms inside this monorepo |
| Shell word splitting and generated script loading | L1 with the actual shell for loading | Four local shell paths plus recorded native PowerShell script load; missing Elvish loader in R1-F4 |
| Accepted Tab candidates continue without stray spaces | L2, as explicitly required by the spec | Four passing detached-tmux tests; remaining shell paths in R1-F4 |

The file/config input matrix applies to the shared JSON5/YAML-flow value loaders, not to the dotted text grammar or source editor. Values are arbitrary user data rather than a fixed configuration record: absence stays absent; null stays null; invalid collection elements stay present for validation; empty values remain empty. Invalid admitted containers fail; syntactically rejected values may become text, as the spec explicitly allows.

The public `setter_values` suite covers the control conversion, absent/null/wrong whole field/mixed elements/all invalid elements/empty field shapes, both loaders' duplicate keys, trailing or malformed syntax, JSON5 comments/trailing commas, YAML extra documents/tags, numeric overflow/underflow, and mapping order. All 38 cases passed. The source-smell search found no newly defaulted load-bearing field or element-filtering parser. The YAML-number `.ok()` follows numeric admission and propagates failure for the whole conversion; it does not discard collection elements. The conversion exceptions in R1-F2 are reported despite those passing loader checks.

## Implementation decisions (informational)

The following items combine the two independent extractions. Each needs no pipeline stop: it either follows the spec, preserves existing behavior, is inexpensive to reverse, or has a recommended repair within the spec. Items requiring correction are attached to findings instead of being reserved for an author decision. The author can review or reverse the remaining choices after repair.

1. Keep applying setters to transcluded children through each child's schema; reject incompatible child paths. This preserves existing behavior.
2. Keep `--set` replacing whole top-level keys and `--state` recursively merging. The implementation found comment/spec drift and followed the existing code; current docs explain the distinction.
3. Guess declared `unknown` values. Correct this under R1-F2.
4. Treat tagged, anchored, or aliased YAML flow arguments as text. This is a safe passive syntax fallback.
5. Preserve inadmissible bare numbers as text. This avoids numerical rounding without inventing another numeric representation.
6. Skip conversion of quoted strings. Correct this under R1-F2.
7. Accept `{foo}` as `{foo: null}` through YAML flow fallback; preserve template syntax as text when it cannot become a JSON mapping.
8. Add a removal-conflict error, JSON5 well-formedness helper, map-rooted edit APIs, and existing typed path steps. These are bounded API choices supporting required behavior.
9. Put serialization fallback in `edited_document_text` in the library, preserving source fences and body bytes. Retain this improvement over the old serializer.
10. Edit a whole flow container and add into `{}` directly; serialize when traversal into a nonempty flow collection is unsupported. This preserves more source comments safely.
11. Identify an orphaned alias from the parser's `unknown anchor` wording, with a regression test. This is a parser-coupling tradeoff, not a required redesign.
12. Refuse any unnamed-value difference without fallback, even without aliases. Retain the conservative safety invariant.
13. Preserve comments preceding removed nodes; discard comments inside replaced containers except their key-line comment; move a trailing comment when needed for valid multiline syntax. These are reasonable source-edit boundaries.
14. Defer valid pending expressions in `md set` validation without executing them. This aligns with composition.
15. Preserve native boolean literals under `boolish`; keep `On` and `yes` as text. This retains existing behavior and the spec's illustrated cases.
16. Use the compiled baseline even when another baseline is selected. Correct this under R1-F1.
17. Accept schema-reference strings and valid inline schemas, judge the whole inline result on a nested edit, reject invalid scalar shapes, and do not open a referenced schema during that edit. This retains schema authoring/repair use.
18. Preserve `md rm`'s old missing-property wording and aggregate list. This is a small compatibility choice.
19. Add shared schema/conversion/diagnostic helpers and use the launch context for stdin trigger discovery. These support the required common behavior.
20. Preserve schema diagnostic codes for supplied-value failures, name the setter, and clear authored positions. Retain this attribution.
21. Keep `dm.setter.invalid` source null and identify affected documents in the message. This avoids attributing command-line text to an on-disk scalar.
22. Resolve nested files from the launch context where supported, but defer path-aware portable presentation and glob origins. R1-F3 demonstrates that origin work cannot all remain deferred; platform-native presentation alone is not reported as an additional defect without an affected output reproduction.
23. Add all offending unknown keys to one validation problem. This fixes lost diagnostics without changing the number of diagnostic records.
24. Return provenance from setter application, retain whole-key override ownership, and keep first position/last value for duplicates. These are internal choices consistent with the specified ordering.
25. Use merged schema helpers for completion. Retain the strategy, with selected-baseline correctness repaired under R1-F1.
26. Add a separate-value query position for `md set` and a pure indication that file candidates are needed. These keep file enumeration in the host adapter.
27. Return whole-word candidates and strip the key prefix in Bash. This matches the shells' different replacement units.
28. Suppress descendants impossible under current scalar/null parents or array bounds. Users can still replace the parent deliberately.
29. Offer both descent and whole replacement for objects and container items. This keeps both valid operations discoverable.
30. Do not let a tag narrow its own replacement values. Other properties still narrow through the tag, and users can change it.
31. Quote completion values only when necessary to preserve their intended scalar. Retain this use of the real parser; retest after R1-F2.
32. Adapt generated upstream Bash/Zsh scripts with guarded textual edits. The guard catches upstream drift without maintaining copied scripts.
33. Rejoin Bash's actual `=`/`:` word splits. Defer the ambiguous deliberately empty earlier setter rather than add another command-line parser; this only affects advisory completion state.
34. Shell-unquote Bash/Zsh inputs, escape Bash replies, and retain Fish's open-quote limitation. This is documented and does not justify a speculative new adapter.
35. Supply the finished-word space directly on Bash 3.2. The real-shell test proves its branch.
36. Ignore completion baseline/trigger policy flags. Correct this under R1-F1.
37. Capture one completion request snapshot at the early-exit path, with its guard entry. This does not recapture downstream state.
38. Complete file arguments containing `=` when the common classifier says they are paths. Leave the older schema-validation completer outside scope.
39. Land subprocess tests in Phase 7 instead of Phase 8. No behavior or verification obligation was lost.
40. Combine work counters, observed resolver files, and namespace-reuse inspection for read-once evidence. Retain this proportionate method; no syscall tracing or new counter is required by this review.
41. Require guaranteed shells and report optional missing shells as unproven. Add the missing executable test paths under R1-F4; absence of a shell must never be presented as a passed shell observation.
42. Type the dot after Tab reaches the common `style.page` prefix. Both `.` and `=` are offered, so this keeps the intended no-stray-space measurement.
43. Keep `md schema validate`'s older assignment grammar. The spec explicitly excludes unrelated schema-query grammar.
44. Put migration examples on each command page. This keeps them near the affected command.
45. Distinguish word-shape, script-load, and real-Tab evidence in the docs. Retain the distinction, but replace inferred shell limitations through R1-F4.

Deferred verification also includes Linux/WSL2 and some native-Windows package results, the full unrelated L2 suite, and dependent suites whose packages were only compiled. Those are not additional findings: cross-OS results belong to CI, and the focused checks above cover this review's boundaries. The supporting test-toolkit suite initially deferred in Phase 7 was run in Phase 10. The single release sample was not repeated after documentation/test-only changes, which meets the spec's bounded measurement requirement.

The log also records an accidental `git init` outside this worktree that overwrote another repository's Git configuration and added a sample hook. No restoration was attempted during this review. This is disclosed for the author's separate follow-up; none of this feature's repairs depends on a ruling about that repository.
