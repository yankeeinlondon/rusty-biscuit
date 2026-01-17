# Choosing prettyplease vs alternatives

## Use prettyplease when

* You’re formatting **generated Rust** (proc macros, bindgen-like output, schema codegen).
* You need a **library** formatter (no external binary).
* You want deterministic, fast formatting that **doesn’t bail out** on deeply nested code.

## Use rustfmt when

* The code is **human-maintained** and you want community-standard formatting.
* You need configuration (`rustfmt.toml`) and exact team consistency.
* IDE/editor integration is important.

Tradeoffs: rustfmt may be slower, harder to embed, and can bail out on pathological generated code.

## Consider prettier-please when

* You like prettyplease’s approach but need **custom printing hooks** (e.g., DSL-ish macro constructs).
* You want formatting closer to rustfmt while staying library-based.

## Consider rust-format when

* You want a unified interface that can try rustfmt and **fallback** to prettyplease.
* You want to expose formatter choice to your users.

## Consider “quote only” when

* Output is never read by humans and performance is paramount.
* You’re okay with long lines / minimal whitespace.

## Summary decision rule

* “Readable generated code, no external tooling” → **prettyplease**
* “Exact standard formatting for checked-in code” → **rustfmt**
* “Need customization hooks” → **prettier-please**
* “Need pluggable backends/fallback” → **rust-format**