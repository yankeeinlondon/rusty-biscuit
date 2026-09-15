---
created: 2026-07-22
status: superseded
superseded_by: ../2026-09-16-expression-type-system/spec.md
---

# Explicit `null`

**Superseded 2026-09-16** by
[2026-09-16-expression-type-system](../2026-09-16-expression-type-system/spec.md):
the `null` keyword's specced core and the XOR authoring idiom below are
folded into that spec's Phase A (the idiom as a Phase A test case), and the
draft is retired in place. The consolidation is recorded as Resolved
Decision 17 in
[2026-09-15-dasherized-identifiers](../2026-09-15-dasherized-identifiers/spec.md).

Currently we can only partially type a Frontmatter property in Darkmatter as `null` and that is done by declaring a property to _not_ be required:

- a property which is _not_ required then defines a _union type_ where the two arms are the type when it does exist or `null`
- what we do NOT have, however, is an explicit `null` property that can define the null type more explicitly in SimplifiedSchema

This feature will introduce the explicit `null` type definition which will allow for schemas like the following to be expressed:

```yaml
$schema:
    - spec: file(required)
      review: null
    - review: file(required)
      spec: null
```

This type definition type -- and one's like it -- are often desirable as they clearly express that a caller should pass in either a `spec` file 
or a `review` file but NOT both.
