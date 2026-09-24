---
kind: baseline
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
rev: 208051f75
---

# Skip baseline

`.github/ci/ci-baseline.toml` at `208051f75` holds **no entries at all**. Its
only non-comment line is `schema_version = 3`, and its header states it is
"EMPTY ON PURPOSE". The one mention of an in-scope package
(`biscuit-terminal-cli`, `:74` and `:79`) is inside the commented `Shape:`
example. It is not a live `[[skip]]`.

Consequences:

- No approved exact-test skip names a test in any of the ten packages. No
  skip identity has to be rewritten when test paths gain a module prefix.
- If an entry for one of these packages is added before its migration
  commit, that commit must rewrite the entry's `tests` identities to the new
  module-qualified paths. Re-check this file at each package's structural
  move.

Checked with:

```sh
grep -v '^\s*#\|^\s*$' .github/ci/ci-baseline.toml   # → schema_version = 3
```
