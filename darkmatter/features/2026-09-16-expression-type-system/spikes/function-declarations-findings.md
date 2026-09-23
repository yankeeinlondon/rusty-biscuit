---
created: 2026-09-18
status: prototype
---

# Shared Function Declaration Representation

One candidate can separate ordered call arguments, schema value types, and
function behavior metadata without a special case keyed to a function name.
The existing passive parser accepts the candidate's metadata envelope and
nine representative value definitions. It rejects three vocabulary entries:
`unknown`, `null`, and the existing catalog's `ip-address`. This is evidence
for a feasible representation direction, not validation of a future checker.

The user authorized this bounded representation prototype. Its syntax is a
proposal, not an additional ratified language or implementation API.

## Artifacts and scope

- [Candidate declarations](function-declarations.yaml): one representation,
  with representative existing functions and explicitly marked hypothetical
  functions. Examples are retained as data and are never executed.
- [Passive parser checks](check-function-declarations.py): reproducible local
  probes using an installed `md`; temporary Markdown envelopes are removed
  when the process exits. It creates no production files or dependencies.

The existing catalog at
[expression-functions.yaml](../../../docs/schemas/expression-functions.yaml)
supplies the representative shapes. This is not a complete catalog migration.

## Representation findings

| Requirement | Candidate representation | Evidence or limitation |
|---|---|---|
| Ordered arguments and omission | Ordered `parameters` with independent `presence` | A required argument remains mandatory even when its value type is `unknown` or `null`; an explicit null is a supplied argument |
| Optional argument | `round` has an optional second argument | Omission does not imply that the accepted supplied value is nullable |
| Variadic argument | `and` has a variadic parameter and short-circuit evaluation metadata | Minimum count and lazy evaluation must preserve existing dispatcher behavior; the marker alone does not prove runtime zero-argument support |
| Overloads | `frontmatter` has different counts; `pr_list` has same-count object/number alternatives | Arity alone cannot choose an overload; existing dispatch and conversions must be audited |
| Arrays and refinements | Required file items in a required array; integer-constrained numbers | Actual parser accepts representative definitions; no files are resolved by the type-definition probe |
| Mixed return values | `ping_under` returns boolean, literal `unstable`, or null | Literal and nullable alternatives are value types; fallibility is separate outcome metadata |
| Conversions | Per-parameter conversion policy identifiers | `round`'s numeric catalog type must not become new rejection of runtime-supported strings or fallback behavior |
| Advisory mismatch | `file_exists` declares warning plus false result for unsupported input | The fixture stores the policy; no new checker interprets it in this spike |
| Guard facts | `is_file_type` and hypothetical host `is_project_file` declare an argument suitable for `file` after true | Same shape for built-in and host; a boolean return alone conveys no fact |
| Documentation and examples | Category, description, order, example expression/result/verification/reason | These fields survive the representation; representative fixtures do not copy every catalog entry's documentation |

The call-presence layer must reject an absent required argument before value
checking. It must not use SimplifiedSchema property omission acceptance to
decide function arity. Conversely, it must not change the agreed schema
meaning of `null(required)` or `unknown(required)` merely to enforce arity.

Guard suitability is local analysis information. It neither rewrites stored
strings nor proves existence, readability, consent, or future read success.
Hosts remain responsible for honoring their declared facts; DMLS does not
execute their implementations to establish those facts.

## Executed checks

Command, run from the repository root:

```sh
python3 darkmatter/features/2026-09-16-expression-type-system/spikes/check-function-declarations.py
```

The executable found on this host was `/Users/ken/.cargo/bin/md`, reporting
`md 0.1.0`. Set `DARKMATTER_SPIKE_MD` to select another binary. No provenance
check establishes that this installed binary was built from the current
worktree, and the version string alone cannot establish that correspondence.

For each temporary document the checker runs:

```sh
md schema validate --no-trigger-schemas --format json <temporary-document.md>
```

The candidate is wrapped as Markdown frontmatter with an inline schema for
its function metadata. Each value-definition probe uses a separate inline
schema declaring `definition: type-definition(required)`. The probe reads
the definition as text; it does not resolve the file, contact a service, or
invoke the represented function. Local fixture/configuration reads by the
CLI are not a claim of zero process-wide filesystem activity.

Final checker exit status: **0**. Results:

| Case | Validator exit | Result |
|---|---|---|
| Candidate metadata envelope | 0 | Valid |
| `number(required)` | 0 | Valid |
| `number(integer; required)` | 0 | Valid |
| `file(required)` | 0 | Valid |
| `file(required)[](required)` | 0 | Valid |
| `string(required)[](required)` | 0 | Valid |
| `object(required)` | 0 | Valid |
| `boolean(required)` | 0 | Valid |
| `literal(unstable; required)` | 0 | Valid |
| `any(required)` | 0 | Valid |
| `unknown(required)` | 1 | Unknown type `unknown` |
| `null(required)` | 1 | Unknown type `null` |
| `ip-address(required)` | 1 | Unknown type `ip-address` |

The last three rejections are expected by this snapshot checker. A future
binary accepting these types will require updating those expectations; a
checker failure then does not by itself indicate a product regression.

The first envelope attempt intentionally described only its essential
fields, but the parser rejected undeclared metadata as additional
properties. The final envelope explicitly declares those metadata fields.
The initial value-probe run also exposed the unexpected `ip-address`
rejection; it was recorded rather than weakened to plain string.

The metadata envelope uses `any` for proposed value-definition payloads so
it can carry strings or union lists containing future types. Envelope
success therefore does **not** validate every nested type definition,
conversion policy, referenced argument name, guard fact, or union payload.
Individual parser probes provide only the more limited evidence above.

## Gaps and recommended next step

1. Phase A must supply `unknown` and `null` before complete candidate type
   definitions can be checked by the production parser.
2. Phase B must map every existing catalog refinement without losing its
   constraints. `ip-address` is a concrete mismatch in the installed parser;
   confirm current source support during planning and choose a shared
   representation. Replacing it with unrestricted string is not parity.
3. Conversion policies need explicit, reusable semantics. Catalog types do
   not alone describe existing conversion/fallback behavior. Migration must
   inspect runtime contracts instead of enforcing the displayed type as a
   new rejection rule.
4. Metadata validation must reject invalid argument references, impossible
   overload declarations, unsupported conversion policies, and malformed
   guard facts. This spike neither implements nor proves those checks.
5. Full catalog parity, expression/editor execution, source spans,
   cross-platform operation, and performance remain implementation evidence.

Recommend carrying forward the separation of call presence, schema value
definitions, and shared behavioral metadata. Do not ratify the fixture's
exact field names or policy identifier strings on the strength of parser
acceptance. The findings require implementation planning, not a new user
decision or additional prototype.

No production Rust changed; no functions represented here were executed;
no new dependencies, builds, network probes, commits, or formatting runs
were performed. GitNexus impact lookup for the new artifact name returned
`UNKNOWN`/not found, and a confirming text search found no existing
references before creation. That result is not a claim that future
implementation has no callers or risk.
