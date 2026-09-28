planning(claudine): record lifecycle-ergonomics schema spike findings

- add spike-results.md with measurements, findings, and rulings from the schema feasibility spike called for by the spec's Schemas section
- add spike/README.md describing the spike layout, the throwaway generator, the discovered traps (T1-T5), and the reproduction recipe
- add spike/schema-harness.rs, the scratch crate's main.rs, kept here so the timings in the results can be re-derived without checking out the harness repo
- extend spike/gen-action-schema.py with the trap workarounds (no-alias dumper, indentless=False, width=inf) and emit the lifecycle-collapsed-d3 variant plus an inline `$schema:` fixture set
- regenerate the spike's schemas/ tree: action.yaml and action-collapsed.yaml drop the bare-arm indexing bug, lifecycle.yaml / lifecycle-d{1..4}.yaml / lifecycle-collapsed.yaml carry the depth curve and depth-5 unrolled stack grammar, and the new lifecycle-collapsed-d3.yaml sits alongside
- add spike/fixtures/variants/lifecycle.md, lifecycle-d{1..4}.md, lifecycle-collapsed.md, and lifecycle-collapsed-d3.md so the inline-`$schema:` probe set covers every depth variant
