# Interim technical design review: D3–D7

Reviewed 2026-09-18. This review covers the confirmed decisions through D7 and
their source support. It is not approval for implementation planning. Pending
design subjects remain pending; their absence is not treated as a defect in
this batch. Only this review artifact was written.

## Result

D3–D7 faithfully record the choices reported by the design orchestrator. D3's
dated specification clarification preserves current snapshot behavior and
assigns the future coherent live-global migration to more-context. D4 preserves
the distinction between authored executable input and completed literal output.
D5 addresses real plain-JSON transfer boundaries without introducing universal
operand taint. D6 keeps lifecycle policy in authoritative schema data and its
interpretation in Darkmatter. D7 makes missing registration a configuration
error without requiring providers during passive validation.

No blocking contradiction with the functional specification was found in these
choices. The following clarifications are needed as their concrete contracts
are completed, before planning.

## D5: sparse metadata invariants

The draft correctly assigns atomic selection, replacement, merge, and move to
Darkmatter. State the selection rule more explicitly: **the selected subtree
must retain its effective inherited policy and stage state at its new root**.
Copying only sparse records whose paths fall inside the selection loses an
ancestor restriction when no child record exists. Rebase descendant records
and materialize the inherited root context together.

The eventual operation contract should cover:

- Replacement takes the incoming value's provenance, drops discarded content's
  records, and retains the destination's applicable schema restrictions.
- Array insertions, removals, and reordering shift metadata paths with values;
  object keys containing dots or numeric strings remain distinct from array
  indices.
- Value and metadata commit together for batched mutations, proxy handoff, and
  sequence merges. A failure leaves both unchanged; returned prior values
  carry their prior provenance if they can resume evaluation later.
- Retried preparation derives from the retained input envelope, not from a
  partially consumed result envelope. Define where consumed stage state lives
  so a failed attempt neither skips a required stage on retry nor adds an extra
  pass to completed output.

These are details of the confirmed sparse carrier, not arguments for replacing
it with a recursive value type. Current source reinforces the need:
`lifecycle/executor.rs:1739` returns a plain evaluated overlay;
`coordinator/document.rs:23` retains overlay JSON across refresh;
`runtime_state.rs:143` validates a whole batch before committing a cloned
mutation map. The new carrier must preserve those existing atomic boundaries.

D5's no-new-passes wording is essential. Policy metadata authorizes no scanning
of successful output merely because braces or shell-looking text remain.
Only already-defined executable stages consume pending provenance.

## D6: imported types and scope identity

The draft correctly leaves event scope on the use site rather than on a shared
`lifecycle-event` type. Existing named import expansion resolves the definition,
strips defining-site presence constraints, and applies use-site postfix data
(`darkmatter/lib/src/markdown/schemas/resolve.rs:1102`, `:1336`). New binding
metadata needs its own explicit rules; it must not accidentally inherit the
presence-constraint treatment or disappear during type expansion.

The remaining syntax/merge ruling should settle:

- Catalog identity and resolution of an opaque scope name: two active schemas
  may use the same short name without meaning the same policy.
- Scope attachment after imported nested types, arrays, and pattern-key
  containers expand; the attachment follows the use-site subtree rather than
  the imported file's root path.
- Whether nested scopes override or conflict, and how an explicit runtime
  selection interacts with schema-selected scopes. Shell-preflight selection
  must not silently relax definite unavailable bindings or shell restrictions.
- Referenced binding metadata as a dependency of the effective schema. Failed
  metadata must suspend the diagnostics and assistance that depend on it.

An illustrative acceptance fixture should import one action type at
`initialize` and `failure`, then prove that identical bare `err` source has
different declared availability at the two use sites. A second fixture should
import that containing type elsewhere, proving that the association follows
the intended nested location without binding all uses to one event.

## D7: clarify the loop row and complete association invariants

The draft's unavailable row says “loop setup, task setup.” Name the literal
`loop` lifecycle event explicitly to avoid implying that only setup is covered.
The current validator's contract excludes `initialize`, `start`, `success`, and
`loop`; `LifecycleSignal::can_carry_error` allows only blocked, failure, and
finalize (`lifecycle/validate.rs:680-748`, `lifecycle/audio.rs:95`).

The confirmed null-valued finalize/teardown case is correctly distinguished
from missing registration and unavailable scope. Future concrete association
rules must also reject contradictory registrations, not merely omitted ones:
an eager/lazy value for a definitely unavailable declaration, an unavailable
entry for a definitely available declaration, duplicate roots, and undeclared
entries need explicit dispositions. Execution-dependent declarations need
the corresponding allowed runtime choices. These decisions belong in the
remaining binding contract, not in an implementation plan.

## Evidence limits

Graph-first queries used the exact absolute worktree path. Context for
`validate_no_err_in_no_error_events` and `can_carry_error` located the expected
source. The schema query included useful named-import functions alongside
corrupted property/name associations. Source verification supports the findings
above, but this review makes no clean impact-count or risk claim. No prototype,
provider, audio, browser, implementation tests, or commits were run.
