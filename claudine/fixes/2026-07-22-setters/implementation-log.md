# Implementation log: 2026-07-22-setters

## Phase 1 rulings (2026-10-01, Ken Snyder)

Dependency state at ruling time: `2026-07-13-cli-switches` has a plan but no
implementation log. Neither R8 (switch metadata) nor R9 (the shared ownership
function) is in `lib/` or `cli/`. `partition_composition_tail` still forwards
everything after the first unowned switch (`cli/src/argv/partition.rs`, the
`Ownership::Unowned` arm).

1. **Sequencing: wait for R9.** Phases 1 and 2 (rulings, spikes, red
   regression suite) may run now. Phase 3 onward is blocked until R9's shared
   ownership function is on the branch. No interim stopgap.
2. **Injectable switch metadata: required.** R9's ownership function takes the
   switch-metadata lookup as data, so unit fixtures use controlled switch
   types. Binary tests use real generated metadata, with fixtures chosen from
   it. This is a requirement on the dependency's R9 design.
3. **Reclaimed setters keep their argv positions.** Reclaimed setters return
   to the Claudine argv in their original relative order, so
   `parse_composition_positionals` collects them in one pass and last-wins
   comes from the existing `Map::insert` order. No separate merge path.
4. **Schema-claimed setter vs. missing value: accepted default (spec wins).**
5. **Dotted keys are not setters: accepted default (spec wins).**
6. **`argv=` after a provider switch: accepted default (dependency owns it).**
7. **Spec status:** set to `planned`.
8. **Measurement: accepted default.** No benchmark; any wider measurement is
   the author's call.
