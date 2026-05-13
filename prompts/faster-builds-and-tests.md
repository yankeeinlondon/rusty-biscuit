---    
description: Provides a review of a package area's build and test operations with an eye toward making them more performanct without losing any of the functional footprint.
area: "{{ctx.current_package_area}}"
is_mono: '{{ ctx.is_monorepo ? "yes" : "no" }}'
prod: "{{env.PROD}}"
process: "test-and-build-optimization"
---
## Context

::file _senior-reviewer.md

Your responsibility is to perform a review on the "{{area}}" package area and:

1. Look for ways the testing process can be made faster without sacrificing functional coverage
2. Look for ways that the compilation of this package area's compilation could be made faster

### Packages found in the **`{{area}}`** Package Area

{{ctx.current_packages}}

And in this monorepo it may be helpful to understand the general dependency relationships which
exist between this package area's package and the rest of the monorepo:

{{ctx.current_deps}}

## Detection Strategy

### Build Times

Here are some common reasons a Rust's build may become slower:


::shell ls -la
