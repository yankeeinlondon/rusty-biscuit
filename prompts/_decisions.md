## Design Decisions

You have the authority to make design decisions. A gap or an ambiguity is
yours to close, not a question to hand back. Exercise that authority and
write each decision down.

### Record every decision

Record each decision you make where the work says to record it (the spec's
`## Design Decisions` section, the plan's `### Decisions`, or the
implementation log's `### Design Decisions` under the current phase). Each
entry states:

- **decision**: what you chose, in one or two sentences
- **alternatives considered**: the other options, one line each
- **why**: the evidence and reasoning that made this option the best fit
  (the spec, the area's `docs/`, existing behavior, simplicity)
- **downsides**: what this choice costs the solution, or what it rules out

A decision recorded this way is settled. Later phases, reviews, and repair
passes build on it; they do not ask anyone to confirm it.

### Settled is settled

- A recommendation already written into the spec, the plan, or a file the
  plan points to is **decided**. Implement it. Never turn it back into a
  "ruling", an "open question", or an item that needs approval, and never
  write that a recommendation "is not an approval".
- A spec or plan that reads as if the author still has to choose between
  options it already recommends is a drafting defect: adopt the
  recommendation, record it as a decision, and move on.
- A criterion that cannot be met literally (for example, a test case that
  cannot exist for some inputs) is a decision, not a blocker: choose the
  reading that keeps the criterion's intent, record it, and test it.

### Never defer requested scope

You may not narrow, postpone, or drop scope the spec asks for. That includes
marking a requirement "blocked on a ruling", moving it to a later spec or a
backlog, replacing it with a weaker acceptance test, or leaving it for "after
review". Where the requested scope forces a choice, make the choice and
deliver the scope.

### When a human is needed

Human review is the exception. Raise it only when **both** of these are true:

1. the decision changes what the spec promises (a public contract, user-visible
   behavior, data format, or ownership between packages), and
2. you cannot recommend an option on the evidence, **or** every option has
   very large consequences that are hard to undo.

Raise it at the earliest stage that can see it: while the spec is reviewed or
the plan is written, never after implementation has started. When you raise
one, still give the options, their pros and cons, and your best
recommendation, written for a reader who does not know this repository.
