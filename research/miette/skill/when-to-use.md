# When to use miette (and when not)

## Great fit

* Compilers, parsers, linters, DSLs: precise source spans + labels are core UX.
* Complex CLIs: validation failures benefit from codes/help/URLs.
* CI/build automation: JSON output enables tooling integration.
* DX-focused libraries (carefully): if you intentionally invest in user-facing diagnostics.

## Not a great fit

* Small/internal libraries where users won’t see formatted reports.
* Embedded/no-std or constrained environments (alloc/formatting heavy).
* Strict binary-size constraints (especially with `fancy`).
* Performance-critical paths with frequent errors.

## Alternatives (quick mapping)

* Want beautiful manual diagnostics only → `ariadne`
* Want rustc-style output with file DB → `codespan-reporting` / `annotate-snippets`
* Want app-level report + backtrace, not snippets → `eyre` / `color-eyre`
* Want lightweight custom error enums → `thiserror` alone