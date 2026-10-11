---
deferred_perf_measurement: false
implementation_1: "2026-10-11T00:05:19-07:00"
---

# Filesystem watchers — review/implementation log

## Implementation of Review Findings #1

> **started at:** 2026-10-11T00:05:19-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-10-05-filesystem-watchers/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Valid Linux watch IDs are dropped or misreported' at 00:06:00
        - defect class: a native numeric field is decoded using the wrong base, rejecting valid observations or changing their identifiers
        - confirmed: `parse_registration` decoded `wd` with `parse_decimal`; the kernel's `fs/notify/fdinfo.c` prints `wd:%x`
        - fixed: registration `wd` now parsed with `parse_hex` (still rejects absent/empty/signed/`0x`/duplicate/overflow; siblings preserved); `parse_fdinfo` doc corrected
        - added full-query regression `hexadecimal_watch_ids_past_nine_are_reported_with_their_kernel_values` (wd a/f/10/1a -> 10/15/16/26, complete inotify coverage); confirmed red against the decimal parser
        - extended the malformed matrix (negative, `0x`-prefixed, beyond-i64, wider-than-64-bit `wd`) and renamed "wd not decimal" to "wd not hex"; test helper `registration` now prints `wd` in hex like the kernel
        - extended the live child test to 23 registrations (21 in scope, IDs past 16) and asserted each path's reported ID equals the child's own `inotify_add_watch` result
        - docs: `sniff/docs/topics/filesystem-query.md` now states every registration field is hexadecimal; departure recorded in implementation-log.md
        - verification: `just test` 3347/3347 passed; `just lint` clean; `cargo clippy -p sniff --all-targets -D warnings` clean; `just cross-check sniff --os linux` pass (2352/2352, live tests included); macOS builds needed the documented `LIBRARY_PATH=/opt/homebrew/opt/libgit2/lib` workaround
- class sweep: a native numeric field is decoded using the wrong base, rejecting valid observations or changing their identifiers; sites checked: inotify registration wd, ino, sdev, mask; fdinfo header flags, ino (pos and mnt_id are not read); /proc/<pid>/stat starttime; /proc PID directory names; /proc/<pid>/fd descriptor names; macOS and Windows backends (no kernel text format); fixed here: inotify registration wd (decimal -> hexadecimal); clean: registration ino/sdev/mask (hex, %lx/%x), fdinfo flags (octal, 0%o), fdinfo ino (decimal, %lu), stat starttime (decimal, %llu), PID and fd names (decimal)
- work completed for 'Valid Linux watch IDs are dropped or misreported' at 00:14:07
- starting the work on 'Unix backends start cwd inspection after the budget expires' at 00:17:08
        - defect class: a transition between usage mechanisms fails to check the shared budget before starting the next mechanism
        - confirmed: `linux.rs` and `macos.rs` `inspect` called the cwd read unconditionally after `inspect_descriptors`, even when the descriptor loop had just returned on budget expiry
        - fixed: both backends check `context.budget.expired()` before the cwd read; a skipped read returns the new `InspectionResult::NotAttempted` (`Tally::not_attempted`); descriptor evidence is kept and the start-time revalidation still runs
        - shared query: `NotAttempted` is counted per mechanism apart from attempts; `finish` adds a `budget_exhausted` limitation with the skipped count and reports `not_attempted` (no process read) or `partial` (some read)
        - test seam: `Budget::expire_after_further_checks` (test-only) and a `tests::ExpiringDuringInspection` wrapper backend arm expiry inside the first real inspection of a full query; test-only cwd call counters on `ProcBackend` and the scripted macOS source
        - added full-query tests `linux_tests::a_budget_expiring_during_descriptors_never_starts_the_cwd_read` and `macos_tests::a_budget_expiring_during_descriptors_never_starts_the_cwd_read` (both portable to every Unix host; confirmed red with the gate removed), plus shared `tests::a_mechanism_the_budget_stopped_for_one_process_is_partial` and `tests::a_mechanism_the_budget_stopped_for_every_process_is_not_attempted`
        - docs: `sniff/docs/topics/filesystem-query.md` now describes the between-mechanism budget check; entry added to implementation-log.md
        - verification: `just test` 3351/3351 passed; `just lint` clean; `cargo clippy -p sniff --all-targets -D warnings` and `-p sniff-cli` clean; `just cross-check sniff --os linux` pass (2356/2356); `just cross-check sniff --os windows` pass (2314/2314, shared query code changed)
- class sweep: a transition between usage mechanisms fails to check the shared budget before starting the next mechanism; sites checked: Linux descriptors -> cwd, Linux descriptor stat -> readlink -> inotify fdinfo (per descriptor), Linux matched descriptor -> access fdinfo, macOS vnode descriptors -> cwd, Windows file-side scan workers, Windows file-side holders -> new-holder confirmation, Windows file-side -> per-process modules, Windows module -> next module, shared enumeration -> inspection, process -> next process, inspection -> root recheck, root recheck -> enrichment, enrichment process -> next process; fixed here: Linux descriptors -> cwd, macOS descriptors -> cwd; clean: Linux per-descriptor fdinfo reads (inside the descriptor unit the loop gates; not a separate mechanism start), Windows scan workers / confirmation / module loop (each checks before the native call), Windows file-side -> modules and every shared transition (process loop, `root::recheck`, `process::enrich` each check first)
- work completed for 'Unix backends start cwd inspection after the budget expires' at 00:31:31
- starting the work on 'Windows loses collected module paths when the budget expires' at 00:32:23
        - defect class: matching is deferred until after an inspection batch, so expiry discards already acquired path observations rather than retaining validated matches as they arrive
        - confirmed: `win32.rs` module loop pushed raw paths after a text pre-filter; `mod.rs` `inspect` looked each path up afterwards through `match_object`, which refuses once the budget has expired, so the first in-scope module and its process were lost when the budget expired before the second
        - fixed: `RawEvidence` gained `matched: Option<ObjectMatch>`; the module loop validates each candidate with `context.match_object` as it reads it and keeps the match; a lookup the budget refuses (`Budget::exhausted`) is a `budget_exhausted` problem on `loaded_modules`, never evidence; the matcher still honors the budget
        - fixed: the report builder never performs a path lookup: it uses the backend's match or an identity match against the captured tree (budget-free); a `debug_assert!` enforces that path-only evidence arrives validated
        - fixed: the shared test fake now validates its path-only evidence during inspection like a real backend; `tests::ExpiringDuringInspection` ungated from `cfg(unix)` so the portable win32 tests can use it
        - tests: `win32_tests::a_budget_expiring_between_modules_keeps_what_was_read` now runs the full query and asserts the first module's evidence, matched path, and process identity survive with `partial` coverage, a `budget_exhausted` limitation, and `budget_exhausted: true`; added `win32_tests::a_module_whose_lookup_the_budget_refuses_is_a_gap_not_evidence` and `tests::a_path_match_the_backend_validated_survives_a_budget_that_expires_before_projection`; all three confirmed red against the old deferred lookup
        - docs: `sniff/docs/topics/filesystem-query.md` matching rules and Windows section state when the lookup runs and what an expired budget keeps; entry added to implementation-log.md
        - verification: `just test` 3353/3353 passed; `just lint` clean; `cargo clippy -p sniff --all-targets -D warnings` and `-p sniff-cli` clean; `just cross-check sniff --os windows` pass (2316/2316, new tests plus win32 spellings/live included); `just cross-check sniff --os linux` pass (2358/2358)
- class sweep: matching is deferred until after an inspection batch, so expiry discards already acquired path observations rather than retaining validated matches as they arrive; sites checked: Windows loaded modules, Windows file-side holders, Linux descriptors, Linux cwd, Linux inotify registrations, macOS vnode descriptors, macOS cwd, shared fake backend, shared report projection (`mod.rs` `inspect`); fixed here: Windows loaded modules, shared report projection (no longer looks paths up), shared fake backend (validates path-only evidence during inspection); clean: Linux descriptors/cwd/inotify and macOS vnodes/cwd (native identity; identity matching never consults the budget), Windows file-side holders (projected to `Evidence` from tree entries inside the backend)
- work completed for 'Windows loses collected module paths when the budget expires' at 00:42:20

### Successful Completion

The implementation of review cycle 1 has completed successfully in 38 minutes (00:05:19 → 00:43). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- deferred: none
- disputed: none
- the files changed in this cycle are:
        - `sniff/lib/src/filesystem/query/{backend,budget,linux,macos,mod,win32}.rs`
        - `sniff/lib/src/filesystem/query/{linux_tests,macos_tests,tests,win32_tests}.rs`
        - `sniff/docs/topics/filesystem-query.md`
        - `sniff/features/2026-10-05-filesystem-watchers/implementation-log.md` (new `## Review cycle 1 fixes` section, which records the departure from the spec's "decimal watch descriptor" sentence)
- final verification: sniff `just test` 3353/3353 passed, `just lint` clean, `cargo clippy -D warnings` clean for `sniff` and `sniff-cli`, `just cross-check sniff --os linux` 2358/2358, `just cross-check sniff --os windows` 2316/2316
- nothing was committed; `spec.md` was not edited by this cycle
