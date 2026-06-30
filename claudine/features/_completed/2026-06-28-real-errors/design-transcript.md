# Design Transcript — Reviewing Design 1 & Design 2

> My honest read of the two architects' designs: what each got right, where each
> fell short, what I took from each, and the few places I overrode both. The
> resulting synthesis is [`integrated-design.md`](./integrated-design.md).

---

## The headline reaction

Both designs reached the same load-bearing conclusion independently, which gave me
high confidence it's correct: **the cause-driven renderer already exists; the real
problem is upstream string-flattening, so the work is to let typed causes survive, not
to build a renderer.** When two independent architects converge on the same diagnosis
from the same evidence, that's the part you build on, not the part you second-guess.

So my job wasn't to pick a winner. It was to take the sharper *framing* from one, the
better *type factoring* from the other, verify both against the code (they cite slightly
different line numbers, which made me want to check), and then fix the two things
**neither** treated with enough rigor.

---

## Design 1 — "Cause-First Structured Diagnostics"

**What I admired.**

- **The reusable `FileReferenceDiagnostic` struct.** This is Design 1's best idea and
  I kept it wholesale. When I verified the code, the *same* `"invalid file path"`
  `format!` appears in `frontmatter()`, `absolute()`, `relative()`, and `load_markdown()`
  (`functions.rs:931/958/1024/1481`). A reusable struct means fixing one site fixes all of
  them and any future file-ref function. Design 2 inlined those fields into a single enum
  variant and quietly lost that leverage.
- **The migration catch-all (`ExpressionCause::Message`).** Design 1 understood that the
  expression catalog is huge and that a typed enum needs an escape hatch to migrate
  incrementally. I verified ~80 functions return `Result<Value, String>` — without a
  catch-all, L0 is a giant big-bang diff. Design 2's enum had no such variant; that's a
  real gap I corrected by importing this idea (as `Other { function, message }`).
- **The crisp principle:** *"Wrapping an error may add fields, but it must not rewrite the
  cause into prose."* That's the whole feature in one sentence. I lifted it into my
  principles section.

**Where it fell short.**

- **It missed the correctness landmine entirely.** Design 1 treats this as a
  display-quality project. It is not. (See "what I overrode" below.)
- **Over-layered types.** Three enums — `ExpressionError` (where) *and* `ExpressionCause`
  (what) *and* `InterpolationError` — plus `FileReferenceDiagnostic` plus `MarkdownError`
  changes. The `Error`-wraps-`Cause` split adds a `Box` and an indirection that carries no
  information the flat enum can't. Rule 2 (Simplicity First) says collapse it.
- **Softer code-grounding.** Some claims read as plausible rather than verified; line refs
  were close but not always exact. It felt like a design written *near* the code rather than
  *from* it.

---

## Design 2 — "Real Errors — Design 2"

**What I admired.**

- **It found the correctness landmine, and that's the single most valuable contribution
  across both docs.** `is_fatal_eval_error` (`rewrite.rs:20-21`) decides abort-vs-warn by
  `message.starts_with(UNKNOWN_FUNCTION_PREFIX)`. I verified it — it's exactly that. This
  reframes the whole effort: the string boundary isn't just ugly, it's load-bearing
  *control flow*. That elevates the work from cosmetics to correctness and changes how you
  have to test it. Respect.
- **Discipline of grounding.** Every claim tied to a file:line, and a stage-by-stage worked
  trace table that I found so clear I kept its shape in the integrated design. This is a
  design written *from* the code.
- **Sharp, justified scoping.** "Type to the function-dispatch boundary; keep the parser
  string-typed behind `Parse(String)`." It answered an open question from the brainstorm
  with a reason, not a shrug.
- **Honesty about the hardest corner.** It flagged that late-binding lifecycle interpolation
  (DM2 event-time) breaks the "excerpt a file on disk" assumption. Design 1 didn't notice
  this at all. That intellectual honesty is exactly what you want in a design doc.
- **`focused_yaml_excerpt` with non-contiguous union + elision** and **`suggest_strings`**
  (recognizing `Described::key()` is `&'static str`, verified at `catalog/mod.rs:42`) were
  both more concrete and more correct than Design 1's equivalents.
- **Lint-enforced boundary rule** — turning "treat as a code smell" into something CI checks.

**Where it fell short.**

- **No migration catch-all.** As above — its enum can't absorb the pure-function tail, so
  L0 risks being all-or-nothing. I patched this with Design 1's idea.
- **Lost the reusable file-ref struct.** Inlining the file-ref fields into one enum variant
  means `absolute()`/`relative()` don't automatically inherit the rich diagnostic.
- **Flagged the late-binding corner but didn't design it.** "May need a fallback" is a
  to-do, not a design. I promoted it to an actual type (`SourceRef::{OnDisk, Effective}`).
- **Left the delegation contract ambiguous.** Its own challenge #6 ("merge cause headline
  with wrapper excerpt") is a real open question it raised but didn't resolve. I specified
  the cause+scope composition rule explicitly.

---

## What I took from each

| From Design 1 | From Design 2 |
|---------------|---------------|
| Reusable `FileReferenceDiagnostic` struct | The framing: "let causes survive," not "build a renderer" |
| Migration catch-all variant (`Other`) | **The control-flow/fatality insight** (its crown jewel) |
| "Wrapping adds fields, never prose" principle | Code-grounded rigor + the worked-trace table |
| `FrontmatterFocus`-style focused excerpt shape | `focused_yaml_excerpt` union/elision concreteness |
| Clean success criteria | `suggest_strings` (the `Described` bound is real) |
| | Engine-typing scope decision + `Parse(String)` last-mile |
| | Lint-enforced transport rule |
| | Honesty about the late-binding corner |

---

## Where I overrode both (my own contributions)

1. **Two type layers, not three.** Neither got the factoring quite right: Design 1
   over-nested (three enums), Design 2 under-factored (lost the reusable struct). I merged
   the best of each — flat `ExpressionError` *with* a reusable `FileReferenceDiagnostic`
   and an `Other` catch-all — which is strictly simpler than Design 1 and strictly richer
   than Design 2.

2. **Made fatality a *gated* change, not a refactor.** This is the most important thing I
   added. Both designs would have changed `is_fatal_eval_error` → `is_authoring_fatal()`;
   only Design 2 even noticed it was dangerous. Neither made it a *hard gate*. I require a
   characterization-test matrix — `{cause} × {fail_fast/lenient} × {frontmatter/body}` —
   pinned green *before* typing, so the refactor is provably behavior-neutral, and I split
   out "should missing-file become fatal?" as a deliberate product decision for Ken rather
   than a silent side effect of typing. This is the difference between "good" and
   "won't quietly break composition semantics six months from now."

3. **Designed the late-binding fallback as a type.** `SourceRef::{OnDisk(SourceContext),
   Effective{rendered, origin_key}}` turns Design 2's hand-wave into a total function. The
   focused-excerpt renderer branches on it instead of fabricating line numbers for
   event-time-resolved lifecycle strings.

4. **Specified the cause+scope render-merge contract.** Resolved Design 2's own open
   challenge #6: cause owns headline+hint+suggestions; wrapper contributes the focused
   excerpt and the "while evaluating `iteration`" scope line. Not pure delegation, not a
   guess — a stated rule with its own test.

---

## How I feel about the result

Confident, and a little energized — this is the rare case where the two inputs were
genuinely complementary rather than redundant. Design 2 is the better *engineering*
document (grounded, found the landmine, honest about corners); Design 1 is the better
*type-modeling* document (reusable struct, migration pragmatism). Neither alone would
ship cleanly: Design 2 as-is risks an all-or-nothing engine diff and silently leaves the
delegation and late-binding corners open; Design 1 as-is would over-engineer the types
and walk straight into the fatality landmine it never saw.

The integrated design's spine is Design 2's framing and rigor, its type model is Design
1's factoring trimmed to two layers, and its distinguishing rigor — the characterization
gate and the `SourceRef` fallback — is mine. The thing I'm most sure of: treating this as
a *correctness* fix with a behavior-preservation gate, not a cosmetics pass, is what makes
it world-class rather than merely better-looking.
