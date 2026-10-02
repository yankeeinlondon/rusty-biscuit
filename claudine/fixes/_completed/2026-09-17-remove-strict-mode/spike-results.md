# Provenance transfer and editor recovery spikes

Run on 2026-09-19 against `a0eedb5cbedf058a075c5b3b6fab2dcf2ff5628f`.
Status: experiments complete; production implementation and design approval
remain outstanding. The human requested these two spikes after the specification
review. They supersede the earlier assessment that no separate spike was needed.

## Findings that should inform the design

1. D5's sparse carrier is viable at the tested batch boundary, provided values
   and metadata share one commit and snapshot. Existing `RuntimeState` is a
   plain-JSON boundary; keeping an independently mutable metadata map outside
   its mutex would not establish the required atomicity.
2. Completion state must survive transfers independently of shell policy.
   Darkmatter converts a triple-brace literal to double-brace text. A second
   ordinary interpolation really evaluates that output. Preserving only
   `no-shell-expansion` would not prevent this semantic change.
3. R12 requires a common effective semantic schema for diagnostics, hover, and
   completion, in addition to new failure states. Current trigger-only schema
   properties affect validation but are absent from hover/completion even when
   their source is healthy. Explicit document schemas take a different route.
4. Refresh result identity needs to follow the work being refreshed. A single
   global epoch failed the independent-source test: refreshing B invalidated
   A's still-valid result and left A pending. Source-specific tickets passed
   both the independent-source and stale-result experiments. Combined results
   will additionally need to check all relevant dependency versions; this
   prototype is not a complete dependency-graph implementation.

These findings support the agreed sparse carrier and shared semantic authority.
They do not resolve the remaining binding grammar, root-union assembly, or
activation-policy rulings, and do not authorize implementation from the stale
`plan.md`.

## Spike 1: provenance through real batch validation

Fixture: [strict_mode_provenance_spike.rs](../../lib/tests/strict_mode_provenance_spike.rs).

The test-only carrier holds ordinary JSON and sparse records addressed by typed
object-key/array-index segments. It supports selection, array reconstruction,
replacement, and a minimal remaining-stage marker. It invokes the real
`RuntimeState::set_batch` on a candidate snapshot before publishing the value
and metadata together. Real `SubtreeCompose` performs interpolation.

| Experiment | Observed result |
| --- | --- |
| Select a nested deferred value whose restriction exists only on its ancestor | The selected root retains the inherited shell prohibition. |
| Commit a restricted value, then reject a batch with a late reserved `outputs` key | Real batch validation rejects it; both candidate value and metadata remain unchanged. |
| Prepare twice from the retained snapshot after prohibited execution | Both attempts fail before the shell sentinel; the retained stage is not consumed. |
| Move restricted data to an unrestricted destination, and unrestricted data to a restricted destination | Both prohibit the pending shell stage; an unrestricted positive control reaches the sentinel once. |
| Reorder, remove, and insert selected array elements | Metadata moves with values; source restrictions do not contaminate unrestricted siblings. Destination restrictions still cover every child. |
| Address `a.b`, string key `0`, and array index `0` | Structural paths keep these identities distinct. |
| Replace restricted source content | Old provenance is discarded; the new destination restriction still applies. |
| Carry completed literals through real batch storage and repeated snapshot preparation | All three escape forms retain their exact ordinary subtree result without an extra evaluation pass. |

The initial negative-control assertion incorrectly expected every escape form's
output to become executable on a second pass. Execution disproved that
assumption. At the `SubtreeCompose` boundary:

| Authored source | First result | Second ordinary interpolation |
| --- | --- | --- |
| `{{{ secret }}}` | `{{ secret }}` | Reads the supplied `secret` value. |
| `\{{ secret }}` | `\{{ secret }}` | Retains the escaped text. |
| `\{\{ secret }}` | `\{\{ secret }}` | Retains the escaped text. |

The retained fixture pins first-result bytes for all three forms and uses the
triple-brace case as the real negative control. It does not normalize escapes
or claim that another composition stage has the same representation.

### Limits and implementation consequences

This is a single-threaded candidate/commit prototype, not an external sidecar
recommended for production. Production must place value and metadata inside
the same `RuntimeState` lock and carry them through its snapshots, returned
prior values, and layering APIs. The current raw JSON types cannot carry this
information by themselves.

The shell operation is a counter sentinel, not a real shell command. The spike
proves the proposed dispatch guard's ordering, not an implemented
`no-shell-expansion` grammar or enforcement in the full composition pipeline.
No provider, audio, shell action, or UI is launched. Retry means rebuilding
from the retained runtime snapshot; the full coordinator retry path, proxy
handoff, sequence merging, serialization, and arbitrary mixed-stage subtrees
remain integration work. Array mutations are modeled by reconstruction from
selected envelopes rather than a proposed final array-edit API.

The design should name which existing stages remain on each deferred unit and
which real APIs can export terminal plain data. Keep the complete transfer
inventory as an implementation prerequisite.

## Spike 2: DMLS protocol failure and recovery

Fixture: [strict_mode_recovery_spike.rs](../../../darkmatter/dmls/tests/strict_mode_recovery_spike.rs).

The real DMLS server runs through the existing in-memory LSP fixture with
bounded cleanup. The workspace contains two `payments` documents consuming one
named-type import, plus an independent `shipping` document. One payments
document uses only an automatically discovered trigger; the other also names
the payload through document `$schema`. The test opens all three, deletes and
repairs the shared import, then deletes and recreates `payments/schemas`.
Every change is sent through `workspace/didChangeWatchedFiles`.

| Phase | Trigger-only consumer | Explicit-schema consumer | Independent sibling |
| --- | --- | --- | --- |
| Healthy | Required-property diagnostic appears; imported hover and schema key completion are absent. | Diagnostic, imported hover, and schema completion appear. | No payments schema leaks into assistance. |
| Shared import deleted | Old required-property diagnostic remains; trigger source receives a preparation error. | Old required-property diagnostic and dependent hover/completion disappear. | Independent diagnostic codes remain unchanged. |
| Shared import repaired with new documentation | Validation resumes; trigger-only assistance remains absent. | Hover shows the new description, not the old one; completion and validation return. | Remains independent. |
| Optional schema directory removed | Trigger-derived requirement disappears. | Missing explicitly required schema does not retain old assistance. | Remains independent. |
| Schema directory recreated | Trigger validation returns. | Validation and imported assistance return. | No schema completion leaks across scope. |

The server builds no Darkmatter effect engine and attempts no network operation
throughout this protocol test, checked with `effects-instrumentation` counters.
Startup readiness and request barriers are used rather than fixed sleeps.
Independent diagnostic codes and baseline key completion also remain available
inside the affected payments documents while their import is missing.

Source inspection explains the healthy-state discrepancy:

- `DarkmatterSchemas::effective_for_with_override` merges trigger JSON Schema
  into validation, but `EffectiveSchema.simplified` comes only from the
  document's resolved `$schema`.
- DMLS `known_shape` uses the base, extension shapes, and that simplified
  document schema for hover/completion.
- `OverlayCache::trigger_registry` retains its last-good registry after a
  scan failure. Direct document-schema resolution can instead fail normally.

This is not evidence that stale hover/completion already occurs on every
trigger failure: those properties were never offered through that route. The
design must first make all three consumers use the same semantic contributions
and then apply dependency-aware suppression to each surface.

### Controlled publication experiment

Current DMLS schema refresh and diagnostic publication are synchronous; an
older schema-refresh worker cannot be released late through the existing
protocol seam. A separate test-only model therefore controls completion order
without adding an asynchronous production worker or artificial sleeps.

The model marks dependent contributions unavailable at invalidation, retains
independent assistance, removes failed definitions, records source/cause, and
publishes diagnostics/hover/completion from one current view. It rejects an old
successful result after a newer failure and an old failure after successful
repair. A second test interleaves two independent source refreshes.

The first global-epoch model failed the second test at the explicit oracle
`an independent source refresh must not be discarded by a global epoch`.
Per-source generations corrected that bounded model. A production alternative
can use one immutable generation per complete refresh pass, but it must
reschedule superseded work and never strand pending dependencies. Neither
alternative should use document version alone: schema files can change while
the document's version stays constant.

This model does not discover dependencies, resolve precedence, handle unknown
activation applicability, or wire suppression into the server. Those remain
design work. For multi-source results, capture and verify their dependency
version set and relevant document/configuration identity together.

The protocol fixture supplies watcher notifications; it does not prove that
each editor/OS will register or deliver newly created-directory events.
Native watcher registration, `SCHEMA_DIR`, time-based activation, malformed
activation rules, and cross-platform execution are outside this spike.

## Verification and retention

Executed on macOS with Nextest:

```sh
cd claudine
BISCUIT_TEST_FILTER='test(spike_provenance)' just test-library --test strict_mode_provenance_spike

cd ../darkmatter
BISCUIT_TEST_FILTER='test(spike_recovery)' just _test dmls --features effects-instrumentation --test strict_mode_recovery_spike
```

- Provenance: 4 tests passed, 0 skipped; final run ID
  `18e3910e-75ca-41b7-b3d8-15465647da10`.
- Recovery: 3 tests passed, 0 skipped; final run ID
  `09812f0c-0626-4f81-8ce4-ca73c72c0f15`.
- Representative execution times: provenance binary 0.016 s; recovery binary
  0.161 s, including the real protocol fixture. These are observations, not
  a performance budget or benchmark. Initial dependency builds took minutes.
- The root `just test dmls --test ...` route failed before compilation with an
  empty `--package` argument. The existing shared `_test` recipe above ran the
  actual DMLS tests; no runner configuration was changed.
- Targeted Clippy passed with warnings denied:
  `cargo clippy -p claudine -p dmls --test strict_mode_provenance_spike --test strict_mode_recovery_spike --features dmls/effects-instrumentation -- -D warnings`.

The experiments introduce no production behavior, dependencies, CI cells, or
format changes. Their Rust fixtures are ordinary L1 tests with no terminal or
browser focus. Full package acceptance gates and other-OS runs are not claimed.

When implementation lands, promote the real transfer tests onto the production
carrier and replace the DMLS characterization assertions with R12's expected
behavior. The assertions identifying today's trigger defects are evidence of
the baseline, not requirements to preserve those defects. Delete redundant
test-only models once production contract coverage supersedes them.

GitNexus was bound to this exact worktree as `rusty`, at the starting commit.
Upstream checks for `set_batch` and `OverlayCache::trigger_registry` returned
LOW risk, respectively 3 and 1 direct callers and no indexed processes. The
fixture symbol was unindexed and was confirmed by text inspection. These
checks guide the experiment boundaries; they are not a complete impact audit
of the future fix. Production functions were not edited.
