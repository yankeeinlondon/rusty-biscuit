---
area: claudine
status: ready for review
created: 2026-07-23
packages:
    - claudine-cli
review_iterations: 0
---

# Preserve Codex SQLite state when Claudine uses a shadow home

## Outcome

When Claudine launches Codex with a shadow home for repository resources or MCP
injection, Codex continues to use the same SQLite state directory it would have
used without Claudine.

The shadow home remains responsible for overlayable configuration and resources.
It does not create, copy, or symbolically link Codex databases, WAL files,
shared-memory files, journals, or database locks.

As a result:

- `codex` and `claudine codex` see the same sessions and state;
- a wrapped launch does not run an independent state-database backfill;
- plain and wrapped Codex processes coordinate through one SQLite database
  family and one set of WAL/SHM files;
- repository prompts and runtime MCP configuration remain isolated in
  Claudine's shadow home; and
- existing abandoned databases under `~/.claudine/.codex` are no longer opened,
  but are not deleted automatically.

## Problem

Claudine changes `HOME` when Codex needs a shadow home. Codex derives
`CODEX_HOME` from that environment and, unless configured otherwise, stores its
SQLite state in the same directory.

This redirects a wrapped launch from the normal state directory:

```text
/home/ken/.codex
```

to an independent shadow state directory:

```text
/home/ken/.claudine/.codex
```

The shadow home intentionally links stable Codex configuration and resource
entries back to the real Codex home while excluding volatile SQLite files.
Excluding those files avoids sharing a live database incorrectly, but it also
causes Codex to initialize a new database family in the shadow directory.

Codex now gates startup on completion of its rollout-to-SQLite backfill. If the
shadow backfill is interrupted after claiming its lease, later wrapped launches
observe `backfill_state.status = running`, wait up to 30 seconds, and fail
startup initialization while the abandoned lease remains active.

Plain `codex` continues to work because its real database is separate and has a
completed backfill.

## Observed failure

The failure was reproduced with:

```text
claudine 0.1.0
codex-cli 0.145.0
```

The wrapped launch reported:

```text
state db backfill is running at /home/ken/.claudine/.codex; waiting up to 30s before retrying startup initialization
```

Inspection showed:

| State | Real Codex home | Claudine shadow home |
|---|---:|---:|
| SQLite directory | `/home/ken/.codex` | `/home/ken/.claudine/.codex` |
| Backfill status | `complete` | `running` |
| Indexed threads | 2 | 0 |
| Backfill checkpoint | complete | none |

The session tree contained only two JSONL files totaling approximately 1.3 MiB.
The failure was therefore not caused by a large session history. It was caused
by redirecting Codex to a second state database whose startup lease was
abandoned.

## Safety invariant

A SQLite database in WAL mode is a coordinated file family, not one standalone
file:

```text
state_5.sqlite
state_5.sqlite-wal
state_5.sqlite-shm
```

The same rule applies to Codex's other versioned SQLite stores and their
sidecars.

SQLite creates, replaces, checkpoints, and removes WAL/SHM files dynamically.
Linking only the main database can make plain and wrapped Codex processes access
one main file through different sidecar and locking paths. Linking the sidecars
individually is also unsafe because SQLite may unlink and recreate them.

The invariant is:

> Every process accessing one Codex SQLite database must resolve the main
> database, WAL, shared-memory file, journals, and locks through the same SQLite
> storage directory.

Claudine must preserve that invariant by selecting the directory through
Codex's native SQLite-home mechanism. It must never approximate directory
identity with per-file links.

## Codex's native storage boundary

Codex supports a SQLite directory that is independent from `CODEX_HOME`.
Resolution follows Codex configuration, including:

1. an explicit `sqlite_home` configuration value;
2. an explicit `CODEX_SQLITE_HOME`; and
3. otherwise, the effective Codex home.

This is the supported boundary Claudine must use. `CODEX_HOME` may point at the
shadow overlay while the resolved SQLite home continues to point at the
pre-shadow state directory.

```text
                       claudine codex
                             |
              +--------------+--------------+
              |                             |
              v                             v
   HOME / CODEX_HOME overlay       CODEX_SQLITE_HOME
   ~/.claudine/.codex              pre-shadow SQLite home
              |                             |
   prompts, config, MCP             state_*.sqlite
   resource overlays               logs_*.sqlite
                                    memories_*.sqlite
                                    goals_*.sqlite
                                    WAL/SHM/journals
```

## Required behavior

### Capture the pre-shadow SQLite destination

Before applying any shadow-home environment changes, Claudine resolves the
SQLite directory Codex would use for the same invocation without the shadow.

The resolver must preserve user intent:

- an explicit Codex `sqlite_home` configuration remains authoritative;
- otherwise, an explicit `CODEX_SQLITE_HOME` remains authoritative;
- otherwise the destination is the effective pre-shadow Codex home, normally
  `~/.codex`; and
- paths are absolute before being placed in the child environment.

Claudine must not blindly force `~/.codex` when the user has selected another
Codex or SQLite home.

### Apply the override only with a shadow home

When no shadow home is needed, Claudine leaves Codex's SQLite resolution
unchanged.

When a shadow home is needed, the child launch receives the resolved
pre-shadow SQLite directory through Codex's native SQLite-home environment
contract. This applies consistently to:

- direct `claudine codex` wrappers;
- composition and sequence launches that select Codex;
- retries and resumes whose rebuilt launch plan selects Codex;
- repository-only resource isolation; and
- MCP-triggered shadow homes.

Every launch-plan rebuild must recompute or retain the same invocation-level
pre-shadow SQLite destination. A retry must not fall back to the shadow
directory because its provider or MCP facets changed.

### Keep volatile state out of shadow materialization

The current exclusion of live SQLite state from shadow-home linking remains in
place and expands only if Codex introduces additional recognized database
sidecars.

At minimum, Claudine must not copy or link:

- `*.sqlite`;
- `*.sqlite-wal`;
- `*.sqlite-shm`;
- `*.sqlite-journal`; or
- any future Codex state file explicitly classified as live SQLite state.

The native SQLite-home selection replaces shadow database materialization; it
does not supplement it.

### Preserve existing shadow state

Claudine must not automatically delete existing regular database files under
`~/.claudine/.codex`.

Those files may contain recoverable state from prior wrapped sessions. Once the
fix is active they are legacy, unused state. Cleanup may be offered later as an
explicit, inspected, recoverable maintenance action, but deletion is not part
of wrapper startup.

Legacy database symbolic links remain subject to the existing safety sweep
because they can violate the WAL-family invariant. Regular shadow-owned
databases are left untouched.

## Environment precedence

The launch environment must distinguish user intent from Claudine's derived
override.

| Input before shadowing | Wrapped result |
|---|---|
| Explicit Codex `sqlite_home` | Preserve that configured destination |
| No configured `sqlite_home`, explicit `CODEX_SQLITE_HOME` | Preserve the exact resolved environment destination |
| No SQLite override, explicit `CODEX_HOME` | Use the pre-shadow `CODEX_HOME` as SQLite home |
| No Codex-specific override | Use the normal pre-shadow Codex home |
| No shadow home required | Do not add a Claudine SQLite override |

The derived value is operational state, not a credential. It must survive the
wrapper environment sanitizer without requiring `--include`.

If the resolved destination is relative, invalid, or cannot be made absolute,
Claudine fails before spawning Codex with a diagnostic that identifies the
source of the value. A valid destination need not exist yet; Codex retains
responsibility for initializing its configured SQLite directory. Claudine must
not silently fall back to a shadow database.

## Scope

### In scope

- Codex launches that use Claudine's shadow home
- Pre-shadow SQLite-home resolution and child-environment application
- Direct wrappers, composition, sequence, retry, and resume launch plans
- Existing volatile-state exclusion and legacy-link cleanup
- Unit and integration coverage for environment precedence and launch rebuilds
- Documentation of the split between overlay home and SQLite home

### Out of scope

- Changing Codex's SQLite schema, migrations, backfill, or lease behavior
- Repairing or editing the user's real Codex databases
- Automatically deleting regular databases already present in the shadow home
- Replacing SQLite with another state backend
- Synchronizing database files by copying, hard-linking, or symbolic linking
- Changing when Claudine needs a shadow home
- Generalizing the mechanism to providers without a native separate-state
  directory

## Implementation constraints

- Use Codex's native SQLite-home contract; do not add a Claudine-specific
  database proxy.
- Resolve the destination once from the invocation's pre-shadow environment and
  thread it through the existing launch-home/launch-plan structures.
- Do not parse or mutate SQLite contents.
- Do not edit the user's Codex configuration to persist Claudine behavior.
- Do not introduce a second shadow-state migration system.
- Preserve non-Codex provider behavior.
- Match existing environment-overlay precedence and retry/resume reconstruction
  semantics.

## Test requirements

### Resolver tests

- No override resolves to the effective pre-shadow Codex home.
- Explicit `CODEX_HOME` changes the default SQLite destination.
- Explicit `CODEX_SQLITE_HOME` wins over the effective Codex home.
- Explicit `sqlite_home` configuration wins over `CODEX_SQLITE_HOME`.
- Relative or invalid destinations fail closed rather than selecting the shadow
  directory.

### Shadow-home tests

- A Codex shadow launch sets the child SQLite home to the pre-shadow
  destination.
- A non-shadow Codex launch does not add a derived SQLite override.
- Other providers receive no Codex SQLite environment changes.
- Shadow materialization does not create or link any SQLite database family.
- Existing regular shadow databases are preserved.
- Existing legacy SQLite symbolic links continue to be removed as one volatile
  family.

### Launch-path tests

- Direct wrapper and composition launches produce the same Codex SQLite
  destination.
- MCP-triggered and repository-resource-triggered shadow homes produce the same
  destination.
- A retry or resume rebuild that selects Codex preserves the invocation's
  pre-shadow destination.
- A rebuild that leaves Codex does not leak `CODEX_SQLITE_HOME` into another
  provider.
- A rebuild that returns to Codex restores the correct destination.

### Integration regression

A fake Codex executable captures its environment and filesystem view while both
real and shadow directories contain distinguishable marker files. The test must
prove:

1. `HOME` points at Claudine's shadow root;
2. Codex configuration and repository overlays are visible through the shadow;
3. `CODEX_SQLITE_HOME` points at the pre-shadow SQLite directory;
4. no database files are created in the shadow directory; and
5. an existing shadow database remains unchanged.

No test may open one database through two differently resolved WAL/SHM
directories.

## Documentation maintenance

Update the shadow-home and Codex wrapper documentation to state:

- why SQLite files are never linked into the shadow home;
- how Codex configuration/resources and SQLite state use separate roots;
- how explicit `CODEX_HOME`, `CODEX_SQLITE_HOME`, and `sqlite_home` choices are
  preserved; and
- that legacy regular shadow databases are retained but no longer used.

Update the Claudine skill documentation if the shadow-home architecture or
operator workflow described there changes.

## Verification scope

Before implementation gates:

1. run GitNexus impact analysis for every changed symbol in the shadow-home,
   environment, and launch-plan paths;
2. use `sniff repo packages` to record the affected packages and package areas;
3. include downstream consumers identified by impact analysis; and
4. run the `claudine` package area's narrow `just build`, `just test`, and
   `just lint` recipes or narrower supported selectors for that recorded scope.

Workspace-wide lifecycle commands are not required for this fix.

## Acceptance criteria

- [ ] `claudine codex` and plain `codex` resolve the same SQLite state directory
      unless the user explicitly configures them differently.
- [ ] A shadow-home launch uses Codex's native separate SQLite-home mechanism.
- [ ] Explicit `CODEX_SQLITE_HOME`, `sqlite_home`, and pre-shadow `CODEX_HOME`
      choices retain their documented precedence.
- [ ] No Codex SQLite database, WAL, SHM, journal, or lock is copied or linked
      into the shadow home.
- [ ] Plain and wrapped Codex processes access each database family through one
      SQLite storage directory.
- [ ] Wrapped startup does not create or backfill
      `~/.claudine/.codex/state_*.sqlite`.
- [ ] Existing regular shadow-owned databases are not deleted or modified.
- [ ] Direct, composition, sequence, retry, resume, repository-only, and MCP
      launch paths obey the same contract.
- [ ] Provider transitions do not leak `CODEX_SQLITE_HOME` to non-Codex
      processes.
- [ ] Unit and integration regressions cover precedence, shadow materialization,
      and launch-plan reconstruction.
- [ ] Relevant wrapper and shadow-home documentation is updated.
