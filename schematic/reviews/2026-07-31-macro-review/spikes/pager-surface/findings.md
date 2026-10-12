# Pager-surface spike findings

**Date:** 2026-10-11
**Spike code:** this directory (`Cargo.toml`, `src/`, `tests/`, `bloat/`). A throwaway prototype crate, not part of the workspace, not shippable.
**Question:** D14 says the generated client must expose only authored pagination knobs, and a missing knob must be a compile error. Can one generic pager do that with readable errors, and do the runtime rules (deadline, cancel, host policy, loop detection) hold?

**Reading guide.** A "pager" fetches a list endpoint page by page. A "knob" is a configuration method such as `.page_size(..)`, `.after(..)`, `.page(..)`. The earlier spike (`../pagination/`) proved the data model; this one proves the pager API surface and some runtime behavior.

**Status labels.** *Verified here*: checked in this directory on 2026-10-11 (counts only). *Reported, not re-run*: from the spike report. *Proposed, not measured* and *Unverified* as stated.

## Result

All nine checks done; eight with pass criteria passed, one (early drop of a session pager) was measured only. Reported, not re-run.

| Suite | Count | Status |
|---|---|---|
| Runtime tests (`tests/runtime.rs`) | 31 | Count verified; 8 consecutive passing runs reported |
| trybuild (`tests/surface.rs`) | 1 test, 21 cases (19 compile-fail, 2 pass) | Case counts verified (`ui/fail_*.rs` = 19, `pass_*.rs` = 2); pass reported |
| clippy | 1 warning, in the rejected encoding | Reported |

## Recommended encoding (recommended by the spike; not ratified)

One generic `Pager<E>` (configuration) and `Pages<E>` (iteration) for all four strategy kinds (page number, cursor, next-link, session). Each knob is a method with its own trait bound, e.g. `fn page_size(..) where E: PageSizeKnob`, so a missing knob is E0277 showing the trait's `#[diagnostic::on_unimplemented]` text. Kind-only knobs such as `.page(n)` use blanket-implemented capability traits (e.g. `PageNumbered`) with `#[diagnostic::do_not_recommend]`, so the error names the capability. The engine is not generic; per endpoint the generator emits about 20 lines of constants and empty marker impls with no method bodies.

Rejected: impl-level bounds (E0599 ignores the custom message); type-level flag parameters `TPager<Item, CursorK, No, No, No>` (flags and per-endpoint config can drift; errors name type parameters).

## The nine checks

- (a) PASS. Absent `.page_size/.page/.after/.before/.prev/.keep_alive/.close`, a pager for an unpaginated endpoint, and a knob after `.pages()` all fail to compile (19 compile-fail + 2 pass).
- (b) PASS deadline: 150 ms pages vs 400 ms budget; third request cut off; error reports 2 completed pages and elapsed; requests got the remaining budget (~400, 250, <110 ms); nothing sent afterwards.
- (c) PASS, HTTP/1.1 only: dropping cancels; the server sees the connection close; live tasks 1 -> 0.
- (d) PASS host policy: a link differing in port, scheme, or host rejected with zero requests to the other server; a declared extra origin followed.
- (e) PASS: out-of-range page size errors in the builder before any request.
- (f) MEASURED early drop of a session pager: `with_session` closes on return, error, and panic; the drop-warning option leaks unless the caller calls `close()`; NEITHER closes when the caller's future is cancelled (only a detached spawn would, which D13 forbids). `#[must_use]` does not help.
- (g) PASS: repeated cursor, cycle a->b->a, repeated link, missing/invalid continuation each return explicit errors; a failed pager stays failed.
- (h) PASS with finding: at 157 endpoints the pager adds about 1.77 MB to the release binary; its own generic code is ~253 IR lines per endpoint; 74% of added IR is a second copy of each item's deserializer because pages decode via `serde_json::from_value`. Decoding through the same `from_slice` path as unpaginated calls would remove most (proposed, NOT measured). Scripts in `bloat/`.
- (i) PASS: messages like "`Ep1` has no authored page size"; implementor list cut to 8 at 157 endpoints.

## Findings that surface rulings (none decided here)

- Generate BOTH the drop warning and `with_session`; document `with_session` as the main path. The leak on cancellation remains, bounded by the server lease.
- Widen the stall rule to any repeated continuation.
- ALWAYS origin-check followed links, even with the D12 policy off (today they are followed unchecked).
- Accept "stream reset" as the HTTP/2 form of connection close in D13's cancel criterion (HTTP/2 untested).
- Per-request time: remaining vs min(remaining, 30 s). Whether session close gets time after the deadline.
- Split `PageStrategy` variants into named structs and emit a constant projection.
- Use the unpaginated `from_slice` decode path in pages to recover most of the binary-size cost.

## Not tested

Real providers and generator; the offset strategy; partitions and advanced dimensions; HTTP/2; rust-analyzer completion; Linux/Windows/WSL2 (macOS only). The `.stderr` snapshots contain `kache`-remapped paths; regenerate with `TRYBUILD=overwrite`.
