# CI evidence schema

Use the [SimplifiedSchema catalog](../../schemas/ci-evidence.yaml) to inspect
the evidence CI currently records. It describes existing records, not a new
release-readiness format. The executable authority remains
[`scripts/ci/schema.py`](../../scripts/ci/schema.py), together with the reuse
verifier in [`local_evidence.py`](../../scripts/ci/local_evidence.py).

## Choose a record type

The catalog exposes two document types and their component types:

| Type | Version | What it records |
|---|---|---|
| `observation_collection` | 3 | One commit and environment's independently produced observations, stored in Git notes under `refs/notes/ci-local/<environment>` |
| `validation_receipt` | 2 | One host run containing measured results for one or more package/gate cells; also the input used to create hook observations |

An observation records one attempt at one **cell**: a Cargo package, an
environment, and a gate (`lint`, `check`, `L1`, `L2`, or `browser`). The
environment belongs to the collection; the package and gate belong to the
observation's cell. Package areas do not identify evidence.

For example, a collection for `macos-latest` can contain an observation for
`biscuit-hash` at `L1` and another for `biscuit-hash-cli` at `L1`. A retry adds
another observation and explicitly names the observation it retries.

Version-1 receipts remain readable under legacy rules but are not described
by this catalog. Plans, scope receipts, expected-test manifests, completion
records, and nightly coverage certifications are separate documents; their
owners and versions are listed in [CI schema versions](schema-versions.md).

## Read and use the grammar

The catalog uses [Darkmatter's SimplifiedSchema grammar](../../darkmatter/docs/topics/schemas/definition.md).
`required` marks a required property, `[]` denotes an array, `literal(3)` fixes
a version, and `->` explains a field. Named types keep the shared cell shape
identical in both document types.

To validate a sample with `md`, wrap its data in Markdown frontmatter and
reference the appropriate named type. For a sample saved at the repository
root as `evidence-example.md`:

```yaml
---
$schema:
  evidence: observation_collection(required)@./schemas/ci-evidence.yaml
evidence:
  schema_version: 3
  environment: macos-latest
  head: "0123456789abcdef0123456789abcdef01234567"
  tree: "89abcdef0123456789abcdef0123456789abcdef"
  observations: []
---
```

Run `md schema validate --disable baseline-schema --disable schema-triggers evidence-example.md`.
The empty collection is structurally valid and proves no cells. Use
`validation_receipt` instead for version-2 receipt data. The wrapper is for
schema inspection; persisted evidence remains JSON without a `$schema` field.

## Shape is not verified evidence

The catalog describes field shapes. It does not replace the current runtime
validators or prove that a record qualifies for reuse. In particular:

- SimplifiedSchema accepts explicit `null` for optional fields. Runtime
  validators reject explicit null for fields such as `retry_of`, `build`, and
  `features`; omit an unused field instead. JSON Schema numeric validation
  also does not enforce the runtime's exact integer representation for version
  numbers. Raw evidence loading rejects duplicate keys, nonfinite numbers,
  and trailing content.
- Counts must satisfy `total = passed + failed + skipped + errored`. A complete
  failing cell needs failing test names or a true `failure_detail_truncated`.
  `test_filter` is permitted only for an L1 cell and cannot be blank.
- A `hosted` producer must name `workflow`, `run_id`, and `attempt`; a `seed`
  producer must name `seed_id`. A hook observation must omit `proof`. A
  passing non-hook observation requires proof including a completion record
  digest; a failing non-hook observation may omit proof altogether.
- A complete cell must cite its report in `reports`. A `host-dir` reference
  requires a nonempty `producer.host.report_dir`. Report bytes, digests, and
  availability are verified separately. Paths are strings describing the
  producing host or artifact, not files that must exist on the reader's host.
- Observation IDs must equal their content identities. IDs must be unique
  and sorted; retry links must name observations of the same cell within the
  collection and cannot form cycles. Publication refuses collections larger
  than 256 KiB.
- Reuse requires qualifying passing evidence and complete execution, matching
  inputs, features, and applicable backend requirements. Input equivalence is
  recomputed rather than accepted from the stored identity. Narrowed
  test-input evidence requires the same filter and exact tree; it can qualify
  across hosts as described in [test inputs](test-inputs.md).
- Retry links and commit ancestry determine supersession. Timestamps do not.
  A newer qualifying failure or an unordered pass/fail conflict prevents an
  older passing result from silently satisfying coverage.

Some producer fields have intentionally broader runtime checks than their
usual values suggest: for example, proof digest fields are checked as
nonempty strings by the structural validator. The catalog preserves that
shape instead of imposing a new digest format. Conversely, counts and the
failure-detail flag describe the intended producer types; some legacy
validator checks are looser. The catalog is documentation, not a stricter
replacement validator wired into CI.

The existing [evidence contract](../../.github/ci/schemas/README.md#observation-collection)
describes storage and verification in more detail. Release readiness must use
those checks and the canonical coverage plan; schema validation alone is not
a release approval.
