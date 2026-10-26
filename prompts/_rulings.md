## Rulings, Defaults, and Pre-authorizations

This work runs unattended through every phase. A decision the specification already
anticipated never pauses it. Sort every open choice into one of three kinds:

1. **Decided.** The specification, design, or an author ruling settles it. Follow it.
2. **Defaulted.** The specification leaves it open but gives a recommendation, a
   planning default, or a preferred option. Adopt that recommendation, record it in
   the implementation log as "adopted default: {choice} (source: {section})", and
   continue. Wording such as "recommendations are proposals for the author" or "the
   author must confirm" does not turn a recommendation into a blocker; the author
   reviews adopted defaults afterwards.
3. **Blocking.** Only when all of the following hold:
    - the specification offers no recommendation or default for it
    - no option is clearly consistent with the specification's goals and constraints
    - a wrong guess would be costly to undo (for example a public API or serialized
      shape that later phases build on), not merely a diff to revise in review

A finding made during the work counts as blocking under the same test, for example
evidence that the specification's chosen design cannot work as written.

**Pre-authorizations.** Starting this run is the author's confirmation of every
pre-authorization the specification lists, including those it describes as "must be
confirmed before the unattended run". Do not wait for a separate record of that
confirmation. Actions the specification places outside its authorizations stay
outside them: commits, pushes, CI changes, live provider calls, lifecycle moves,
and anything else it explicitly excludes.

Scope the specification does not mention is new scope. Log it and leave it undone.
If the plan's goals cannot be met without it, it is blocking.
