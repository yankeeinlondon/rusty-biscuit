# Dependency planning evidence

`prompts/dependency-upgrade.md` invokes `collect.py` during composition. Run it
from the workspace root with Python 3.11+:

```sh
python3 scripts/dependency-report/collect.py
python3 -m unittest discover -s scripts/dependency-report -p 'test_*.py'
```

The collector uses only Python's standard library. Cargo, Git, Rust, and
cargo-outdated are probed independently. It writes a new
`target/dependency-report/run-*/` directory and prints links to its index and
manifest snapshot. It does not update dependencies. Cargo's registry/cache
activity still requires the normal filesystem and network permissions.

The index is the authority for successful, failed, and skipped evidence. A
partial collection exits successfully so the planning agent can analyze blockers;
failure to initialize or write the report is fatal. Raw stdout/stderr are kept
separately; only successful, valid JSON becomes a JSON artifact. The index remains
`collecting` if the process is interrupted. Do not reuse an incomplete run.

Default-feature and all-feature graphs remain separate. Name groups with multiple
package identities report both source and version counts; they are candidates for
investigation, not proof that the workspace can consolidate them. Requirements
are retained even for a single resolved version. Raw manifests preserve workspace
inheritance and feature declarations; raw metadata preserves package traits and
enabled feature sets. Reverse edges retain IDs, aliases, kinds, and targets.

If all metadata commands fail, only the root manifest is snapshotted. The prompt
requires manual member discovery before claiming full coverage. A failed report
does not establish its cause; read its diagnostics. Tool outputs and manifest
snapshots are local evidence and should be reviewed before sharing externally.

Claudine `compose --dry-run` executes the collector too. For a composition smoke
test, use a disposable Git workspace with a minimal Cargo package and copy the
prompt and helper into their normal relative locations. Use isolated application
configuration and tool stubs when testing failure paths; never launch a provider
just to verify evidence collection.
