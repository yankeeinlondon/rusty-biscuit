# Phase 1: Input robustness matrix contracts

These are the expanded robustness tables from the plan's "Input robustness
matrix", fixed against the code as of 2026-10-16. Each cell has an outcome and
a state:

- **covered**: a passing public-result test exists now. It is named here and
  lives in `claudine/cli/tests/l1/loop_config_matrix.rs`.
- **Phase N**: current behavior differs from the contract. The change that
  makes it pass belongs to that phase's task. The cell is added as a row in
  the same `matrix!` table once that change lands.
- **N/A**: the shape cannot occur for that field's type. A whole-field type
  error covers it.

Evidence for current behavior comes from three places: the probe run of the
cells below (one `claudine compose` per cell, with the `launch_recorder` stub
provider), `phase-1/dependency-audit.md` §6, and `phase-1/coverage-inventory.md`
§5 and §7.

## Formats and where each field can appear

Duplicate-key and trailing-content behavior is set per format by the loader
(coverage inventory §5.3):

| Format | Fields it can carry | Duplicate key today | Trailing content today | Contract |
|---|---|---|---|---|
| YAML frontmatter (`.md`) | everything, including document `loop:` and the review metadata | rejected (serde_yaml_ng mapping visitor) | a second fence closes; an **unclosed opening fence reads as empty frontmatter** | duplicate → Error (**covered**). Unclosed fence: the existing Darkmatter rule is kept. The document then has no `loop:` and runs once as body text; this feature does not change the Markdown grammar |
| Standalone YAML (`claudine sequence x.yaml`, `kind: group`/`task` files) | sequence/group/task fields, container `loop:` | rejected | second document rejected | Error for both (rows added in Phase 2 with the container grammar) |
| JSON / JSON5 (`sequence: "@x.json"`, `group: "@x.json"`, `kind:` files) | sequence/group/task fields, container `loop:` | **last-wins** | rejected | duplicate → Error (**Phase 2**: needs a duplicate-detecting read before the `serde_json::Value` conversion; the `MappingOrders` pass in `sequence/data.rs:127` already traverses the raw text). JSON5 comments are allowed content |
| JSONL / NDJSON (`sequence: "@x.jsonl::op(..)"` only) | task records only; no `kind:` files, no offsets | **last-wins** within a record | rejected with a line number | duplicate in a record → Error (**Phase 2**). A valid later record stays valid. Offsets and `kind:` files: unsupported, N/A |
| Direct `claudine sequence x.json` / compose of non-Markdown | none | — | — | unsupported (`NotMarkdown`); recorded, not added |

## Document `loop:` (YAML frontmatter, `claudine compose`)

Control: `count.md` with `n: 1`, `until: "n >= 3"`, `action: "increment(n)"`,
and `max: 4` gives three launches (`Iteration 1..3`) and exit 0.

| Shape | `loop` | `loop.until` / `loop.while` | `loop.max` | `loop.action` / `loop.actions` |
|---|---|---|---|---|
| Control | 3 launches **covered** (`control_condition_ends_the_loop`) | `while` variant 3 launches **covered** (`while_replaces_until`) | condition beats cap **covered** | list form, alias **covered** (`action_list_form_advances`, `actions_alias_advances`) |
| Absent | 1 launch **covered** (`loop_absent_runs_once`) | both absent → Error. Today rejected without the document location → **Phase 2** (located `LoopInvalid`) | default 100 **covered** (`max_absent_uses_the_default`) | no mutation; exhausts at cap. Exhaustion with **exit 1 covered** (`action_absent_never_advances`) |
| Explicit null | Error; today unlocated → **Phase 2** | Error; today unlocated → **Phase 2** (both) | Error; today unlocated → **Phase 2** | Error; today unlocated → **Phase 2** (both) |
| Wrong whole type | scalar, list **covered** | list, map **covered** (`until_*`, `while_list_*`) | text, list, fraction, negative, zero, overflow **covered** | number **covered**; empty map **covered** |
| One wrong element/member | **covered** via members below | N/A → Error | N/A → Error | `["increment(n)", 5]` **covered**; staging is atomic, so no partial mutation |
| Every element/member wrong | **covered** (`loop_every_member_wrong_is_rejected`) | N/A | N/A | `[5, 6]` **covered** |
| Empty | `{}` Error; today unlocated → **Phase 2** | `""` Error **covered** (`until_empty_is_rejected`) | `""` Error **covered** | `[]` no-op **covered** (`action_empty_list_is_a_no_op`). `""` Error; today unlocated → **Phase 2**. `{}` Error **covered** |
| Duplicate key | **covered** | **covered** | **covered** | **covered**. Both aliases together → Error; today unlocated → **Phase 2** |
| Trailing/invalid content | invalid YAML **covered** (`loop_invalid_yaml_is_rejected`) | same | same | same |

Additional loop rows:

| Case | Contract | State |
|---|---|---|
| `until` and `while` together | Error, located | today unlocated → **Phase 2** |
| templated `max: "{{ cap }}"` with `cap: 4` | resolved once; 3 launches | today rejected ("must be a positive integer, got string") → **Phase 2** |
| `max: 2` (condition unmet on the last permitted pass) | exhaustion: exit 1, 2 launches, no action after pass 2 | today **exit 0**: the action runs after the final pass and the second condition check passes. The defect is removed in **Phase 3** (Loop state machine) |
| `loop.fail_fast` in any shape (`true`, `null`, …) | typed removed-key Error naming the replacement | today `true` is accepted (3 launches); `null` is rejected but unlocated → **Phase 2** |
| `loop.when` | Error | **covered** (`loop_when_is_rejected`) |
| unknown key `colour` / typo `untill` | Error; a typo gets a did-you-mean hint | rejection **covered** (`loop_unknown_key_is_rejected`, `until_misspelled_is_rejected`). Edit-distance hints: **Phase 2** (the current hint map is hard-coded) |
| `CLAUDINE_MAX_ITERATIONS` that does not parse | Error, not silently ignored | today ignored (`looping/config.rs:41`) → **Phase 2** |
| runtime cap override on nested loops | applies to each loop independently | **Phase 3** (needs container loops) |
| `loop.on_rate_limit` | depends on the author ruling (implementation log, Phase 1). Today `null` is rejected but unlocated | **Phase 2**, after the ruling |

## Loop gate and loose verbs (YAML frontmatter; container placements in Phase 2)

`gate:` and loose verbs are two forms of one stack. Today mixing them is
rejected even when `gate` is empty (**covered**:
`gate_mixed_with_loose_verbs_is_rejected`).

| Shape | `loop.gate` | Contract source |
|---|---|---|
| Control | actions run after `finalize` | plan table |
| Absent | no explicit gate | — |
| Explicit null | **Error**. Today accepted as empty → **Phase 2** | ruling below |
| Wrong whole type | Error (scalar rejected today) | — |
| One / every wrong member | Error; nothing executes | lifecycle parser (pre-scan before any event) |
| Empty | `[]` explicit no-op **covered** (`gate_empty_list_is_a_no_op`). `{}` **Error**; today accepted → **Phase 2**. `""` Error | ruling below |
| Duplicate key | Error (YAML; JSON family Phase 2) | — |
| Trailing/invalid | Error | — |

Loose-verb columns are expanded from the lifecycle grammar's actual
signatures (dependency audit §6). The generic short-form rule
(`action_shape.rs:20-61`) is: null or `[]` means zero arguments, a scalar
means one argument, an array means one argument per element, and an object
means long form. So "null" is accepted exactly where a verb's signature
allows zero arguments, and that existing verb contract is kept:

| Verb under `loop:` | null / `[]` | Wrong whole type | Gate contract |
|---|---|---|---|
| `stop` | accepted (0 args) | object other than `{no_error}` → Error | ends the gate; the condition still runs |
| `error` | accepted (0..1) | — | fails the loop owner with the iteration and location |
| `skip` | — | — | Error: `initialize`-only (`is_valid_for`) |
| `proxy`, `retry`, `resume`, `defer` | Error / per arity | — | parse, then refused at run time in the gate. Kept as is; `defer` stays planned |
| `say`, `speak`, `effect`, `message`, `notify`, `stderr`, `stdout`, `info`, `warn`, `success` | Error (exactly 1) | — | run |
| `shell` | Error | — | runs (the gate is post-`start`) |
| `set` | Error (`set: null`) | non-mapping → Error | **Phase 3**: the gate's `runtime_state` is `None` today, so `set` does not persist. The contract is that it persists into the owner's runtime layer |
| side-effect verbs (`set_frontmatter`, `append_line`, …) | Error (each needs at least 1 arg) | — | run |
| expression functions | only where the signature has 0 required args | — | read-only |
| `break` (new) | accepted (0 args, `reason` optional) | see `break` table | **Phase 2** grammar, Phase 4 behavior |
| `prep` (new) | Error (target required) | see `prep` table | **Phase 2** grammar, Phase 4 behavior |
| `no_error` | — | non-boolean → Error | needs a sibling verb |

## Task `when:` and task `loop` placement (all formats that carry tasks)

None of these exist yet. Today `when:` is not reserved and falls into step
data, so a `when: false` task runs. They are owned by Phase 2 (grammar) and
Phase 4 (evaluation); every cell is **Phase 2/4**.

| Shape | Task `when` | Task `loop` placement |
|---|---|---|
| Control | truthy runs; falsey records `skipped` with no launch | an inline nested sequence's loop repeats it |
| Absent | runs normally | no task-owned repetition |
| Explicit null | Error | Error |
| Wrong whole type | Error (list or map) | Error |
| One / every wrong member | N/A → Error | nested loop field errors (document table) |
| Empty | `""` Error | `{}` Error |
| Duplicate key | Error (JSON family needs the Phase 2 detector) | Error |
| Trailing/invalid | Error | Error |
| Placement | sequence-root `when` → typed Error | `loop` on a prompt/shell/side-effect/group-reference task → Error; a group's loop goes on the group object |

Format fixtures (coverage inventory §7.1): YAML (`seq.md` root `loop:` and a
`group:` leg with `loop:`/`when:`), standalone YAML (`steps.yaml` and
`kind: group` YAML), JSON and JSON5 (formal document and `kind: group`), and
JSONL/NDJSON (two records; edit record 1 only).

## `break`, `prep`, and caller bindings

All cells are **Phase 2** (grammar and passive validation) or **Phase 4**
(runtime), as in the plan's table. These rulings are fixed now:

- `break` with no argument and `break: ""` are both accepted. `""` is an
  explicit empty reason; a collection reason is an Error.
- `break.code` must be an integer in `0..=255`; absent means 0. Null, a
  fraction, a string, a negative, and 256 are Errors.
- `prep.with` absent means no overlay. `with: {}` is an explicit empty
  overlay. `with: null` and a string or list are Errors, matching
  `proxy.with` today (`parse.rs:1078-1110`).
- An empty `prep.target` (or `""`) is an Error. Unknown arguments are Errors.
- Task `params.<key>` keeps its declared schema. A value the schema permits,
  including null, stays data. For the load-bearing keys `agent` and `spec`,
  absent means a required-input Error before launch.
- `caller_setters` that is not an object becomes an empty map silently today
  (`preflight/mod.rs:455`). That is an internal value, not authored
  configuration, so it needs no matrix cell. A characterization test for
  AC8 belongs to Phase 4.

## Review metadata (YAML frontmatter only)

Measured with `md compose` on this tree. The table shows each value of
`implemented` in the spec, then `frontmatter(spec,'implemented') != true`,
`as_json([frontmatter(spec,'implemented')])`, and
`has_key(frontmatter(spec),'implemented')`:

| `implemented:` | `!= true` (implementation runs?) | `as_json([...])` | `has_key` |
|---|---|---|---|
| `true` | false | `[true]` | true |
| `false` | true | `[false]` | true |
| `null` / empty value | true | `[null]` | true |
| `'true'` (quoted) | **false**: a quoted string skips implementation | `["true"]` | true |
| `1`, `0` | true | `[1]`, `[0]` | true |
| `[true]`, `[]`, `{}` | true | `[[true]]`, `[[]]`, `[{}]` | true |
| absent | true | `[null]` | **false** |

Decisions recorded:

- The `implemented` guard keeps the spec's `!= true` expression semantics,
  including the quoted-`'true'` row. This is the existing comparison, not a
  new validator, and the rehearsal (Phase 5) asserts it.
- `ready` and `recurrence` must be actual booleans, checked with
  `as_json([...]) == '[true]'` / `'[false]'` as the spec shows. A quoted
  `"true"` is an Error.
- `review_iterations` before a review must distinguish absent (zero) from
  null (Error). `frontmatter()` returns null for both, so the migrated prompt
  uses `has_key(frontmatter(spec), 'review_iterations')` to tell them apart.
  This is the smallest presence-aware read, and it already exists.

## Code smells checked (load-bearing parses)

From the coverage inventory §7.2. To fix in the phase that owns the field:

- `looping/config.rs:24,31,41`: `CLAUDINE_FAIL_FAST`, `FAIL_FAST`, and
  `CLAUDINE_MAX_ITERATIONS` use `.ok()`, so a bad value is silently ignored.
  The fail-fast variables go away (Phase 3); the max-iterations override must
  error (Phase 2).
- `looping/engine.rs:96-100`: `fail_fast` defaults to true via
  `unwrap_or_default`-style reads. Removed in Phase 3.
- `sequence/grammar.rs:414`: non-object `group.tasks` legs are skipped by
  the removed-key walk. Phase 2 confirms they are rejected elsewhere and adds
  a matrix row.
- No `#[serde(default)]` on loop, task, or lifecycle configuration: it is
  hand-parsed from `serde_json::Value`.
