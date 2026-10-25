---
kind: implementation-log
---

# Test audit: 2026-10-21-claudine-schemas

Added **6 test identities**, all net additions: **2 genuine coverage/guard
identities** and **4 scheduling identities**. Modified **2 original slow L1
tests** without removing cases or assertions. No production behavior changed.

Final verification passed **889 distinct L1 tests**: **848 Claudine** tests
with a maximum of **4.870 seconds**, and **41 Darkmatter** tests with a
maximum of **2.691 seconds**. No final test exceeded five seconds.

## Coverage changes

The new CLI regression writes a malformed on-disk `claudine.yaml` into an
isolated repository and invokes `inline-compose` with a valid prompt and a
compiled marker provider. It requires exit 1, the failing schema filename and
`match` grammar in the diagnostic, and no provider marker. Existing public
library tests already proved the no-fallback rule; this test proves the real
command surfaces the load failure instead of silently launching with the
embedded copy.

Two existing matrix tests exceeded five seconds in the logged parallel run:

- `ac5_trigger_selection`: **5.070 seconds**. Its three malformed identifying
  values remain in that identity, and its six control cases now run in
  `ac5_trigger_selection_controls`. Every trigger, schema, parser, application
  data, and linking assertion is preserved. Existing group and passive-parity
  roster guards cover the split; the parity macro order was updated to match
  the regrouped table. Final durations were **1.223** and **2.572 seconds**.
- `embedded_sequence_types_exports_resolve_without_disk_schemas`: **6.045
  seconds**. Four independently scheduled tests now judge one export's good
  and bad values each. Every test still resolves **all four exports together**
  through the same embedded trigger, preserving combined import resolution,
  exact origins, absent file dependencies, and the assertion that no files
  were extracted. A new roster guard requires exactly one scheduled test for
  every probe. This adds three scheduling identities and one guard identity.
  Final durations were **4.581–4.870 seconds**, including parallel contention.

The two retained original names plus four additional scheduling identities
preserve the previous coverage. The CLI rejection test and export roster guard
are the two genuinely new coverage identities. No identities were removed.

## Requirement coverage

| Requirements | Evidence |
| --- | --- |
| AC1–4: files, kinds, exports, action/catalog parity, bundles, closed action maps and open application data | `claudine_schemas`, its YAML/JSON robustness matrices, `claudine_schema_parity`, and the embedded byte-identity/export tests. Later recursive and generated-operand typing is preserved rather than restoring the snapshot's interim unroll/untyped forms. |
| AC5: schema/parser verdicts and passive fixture parity | AC5 fixture groups, one passive-parity test per fixture, roster guards, exact problem paths and key text, required parameter checks, all supported linking forms, and arbitrary application data. |
| AC6 and AC13: checkout-free embedded imports, origin isolation, nearest-name shadowing, invalid source behavior | `claudine_schemas_embedded`, both embedded/disk input matrices, and Darkmatter's public embedded-trigger suite. Tests cover all five roots, file symlinks on supported Unix hosts, nonmatching/malformed disk shadows, absent disk imports, imported library edits, authored replacement and ownership. New real CLI test crosses the load-failure boundary. |
| AC7: DMLS discovery and shared inventory | In-memory LSP tests validate diagnostics, completion, hover, source dependencies, authored override, no Claudine crate dependency, and the moved lifecycle inventory check. |
| AC9: resolved size | Existing integrated payload-size guard passes. Historical one-time timing is retained in the implementation log; this audit adds no timing assertion or benchmark gate. |
| AC10: launch verdict, phase/pending values, cleanup, transitions and completion freezing | `launch_trigger_schemas` invokes real binaries with hermetic fake providers: both entry modes, missing eager/non-eager authored values, strict controls versus authored optional cleanup, initialize-before-verdict, no start/provider after rejection, retry/proxy/sequence rereads, literal tokens, deferred operands, and launch-frozen completion schema. |
| AC14: old Darkmatter-owned copies retired | Current shipped-artifact tests resolve the Claudine-owned bundle through real `md`; source search finds no live implementation/test dependency on the retired Darkmatter schema paths. Frozen provenance remains historical fixture data. |
| AC8 and AC11: canonical behavior descriptions | Concurrent documentation agent owns current docs and canonical references. Source/test comments were corrected to match effective-schema validation, recursive stack families, and content-based memoization; an obsolete spec-plan link was replaced with the canonical composition guide. |

Direct parser/resolver assertions are sufficient for transformation contracts.
The real `md` and Claudine binaries prove command wiring, source discovery,
diagnostic delivery, and provider nonlaunch. The in-memory LSP session crosses
the actual protocol handler boundary for diagnostics and editor requests.
There is no terminal-rendering, terminal-input, browser, live-provider, or
audio requirement in this spec, so **L2 is not applicable**. No windows were
opened and no live provider was launched.

## Verification

From `claudine/`, parallel L1:

```sh
BISCUIT_TEST_FILTER='test(claudine_schemas) | test(claudine_schema_parity) | test(launch_trigger_schemas)' just test
```

From `darkmatter/`, parallel L1:

```sh
BISCUIT_TEST_FILTER='test(claudine_schema_discovery) | test(schema_embedded_trigger_bundles) | test(schema_validate_shipped_claudine) | test(lifecycle_inventory_matches_the_authored_claudine_schema)' just test
```

The final Claudine slice passed **848 tests**, maximum **4.870 seconds**,
with zero slow tests. The Darkmatter slice passed **41 tests**, maximum
**2.691 seconds**. The new
CLI regression passed in **0.283 seconds** in its first 52-test CLI run.
The final Claudine matrix run is recorded in
`/tmp/pipeline-s11-claudine-final.log`; Darkmatter evidence is in
`/tmp/pipeline-s11-darkmatter.log`.

An initial 842-test run passed. A subsequent logged run passed all 843 tests
functionally but exposed the two slow identities above. The first regrouped
run caught a passive-parity roster ordering mismatch, which was corrected
without changing assertions or cases. Only the final passing run counts as
post-fix timing evidence. `git diff --check` passed. No full-area, cross-OS,
or lint run is claimed; the coordinator owns final package gates.
