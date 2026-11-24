---
kind: feature
created: 2026-11-22
status: finalized-spec
area: claudine
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
    - 2026-11-22-expression-and-mutation-schemas
depends-on:
    - 2026-11-22-host-registration-v2
next-spec: 2026-11-22-expression-and-mutation-schemas
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed-by: string -> the agent and model used in the spec review
    reviewed-on: date -> the date the spec was reviewed
    review-iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented-by: string -> the agent who implemented the plan
reviewed: true
reviewed-by: codex/gpt-6.2-sol
reviewed-on: "2026-11-22"
review-iterations: 5
needs-rulings: false
implemented: true
implemented-by: "claude/opus"
---

# `claudine doctor`

## Why this exists

Claudine can start even when Darkmatter has lost the entry that identifies
Claudine documents. Other problems, such as a misplaced configuration key,
can stop ordinary commands before they explain what else needs attention.
`claudine doctor` checks configuration, registration, permissions, speech
availability, and executable discovery in one run. It gives a remedy for
each problem it finds.

## Command and report

```text
claudine doctor [--fix] [--json]
```

Show `doctor` in grouped Administration help and shell completion. Run it
non-interactively, including with `--fix`. Use one invocation context for
the home and the repository containing the launch directory. Dispatch
before normal initialization so a broken or missing configuration cannot
prevent diagnosis or cause a setup prompt, a legacy conversion, a default
file write, or a registration refresh. The default run does no writes,
network requests, or speech synthesis. `--fix` authorizes only the
repairs described below.

Keep one ordered list of built-in checks. Each entry supplies an ID, a title,
a check function, and an optional repair. An unavailable prerequisite
produces an `info` row that names what blocked the check. Continue independent
checks after a failure.

- The exit status is `0` when the final report has no `error` rows and `1`
  when any error remains. Ordinary CLI usage errors exit `2`.
- Print a single JSON array on stdout for `--json`. Each object has `id`
  (stable string), `severity` (`ok`, `info`, `warn`, or `error`),
  `summary` (one line), `detail` (string, empty if unused), and `fixable`
  (boolean). Emit one row per check in the order below, even when blocked
  or when multiple files fail the same check. Use `detail` for per-file
  findings and remedies. `fixable` means that an available automatic
  repair applies to a finding in that row.
- Use the same rows for the human report on stdout. Render severity text and
  findings with biscuit-terminal list and prose components, honoring
  `--plain` and redirected output. Keep diagnostics about running the command
  on stderr. Keep raw configuration values, permission contents, credentials,
  and file contents out of both reports.
- With `--fix`, inspect first, attempt eligible repairs, and check the
  affected rows again from disk. Report repair outcomes in `detail` and use
  the verified state for severity and exit status. A failed repair is an
  `error`, with the reason and any changes already made. Unrelated findings
  do not block a repair.

## Checks

| ID | What it reports | Automatic repair |
|---|---|---|
| `config` | Missing required user configuration, unreadable files, invalid YAML, and violations of the applicable schema or runtime configuration rules are errors. | None. |
| `registration` | Missing, unreadable, or unregistered files are errors. Changed or extra managed files and legacy registration leftovers are warnings. | Publish the expected files and update the user registration safely. |
| `darkmatter-permissions` | Assess how Darkmatter gets its permissions at each applicable scope and whether setup can complete the handoff. | Apply the same handoff as setup where safe. |
| `repo-config` | In a repository, report whether its optional Claudine file exists. An absent file, or a run outside a repository, is informational. | None. |
| `stray-config-keys` | List unknown Claudine keys and their source locations as errors. | None. |
| `tts` | Report provider availability and quality under the configured speech settings. | None. |
| `path` | Report whether the current process can find `claudine` on its executable path. A miss is an error. | None. |

### Configuration files and unknown keys

Check `~/.claudine/config.yaml` and `~/.darkmatter.yaml`. Inside a repository,
also validate any existing `{repo-root}/.claudine/config.yaml` and
`{repo-root}/.darkmatter.yaml` as working-copy files, each under its
repository schema. A missing repository file is optional. When a
repository file resolves to the same file as the user configuration,
validate it once as a user file. If the home cannot be located, report that
under `config` and continue checks that can run without it.

Reuse the passive readers and the same scope schemas, defaults, and
configuration validation used for normal loads. Apply Claudine's
post-merge semantic checks when the needed files are valid. A legacy
`config.json` without YAML is reported with the supported migration or
setup remedy and left in place. Keep the checks focused on the files
under inspection. The permissions check describes their handoff on disk,
while actual repository grants still depend on Darkmatter's user,
committed, and working-copy policy.
[Claudine configuration](../../docs/topics/configuration.md) ·
[Darkmatter permissions](../../../darkmatter/docs/topics/permissions.md)

The Claudine schemas reject unknown keys, including known user settings
in the wrong scope, such as `tts` in a repository configuration. Use the
validator's unknown-key findings to populate `stray-config-keys`, with
file, line, and column when available. Keep `config` at error for the
rejected file and refer to the key findings there without repeating them.
Use the same validator for nested keys. The imported `permissions`
setting is valid in either scope. When a parse failure prevents key
validation, say which file could not be inspected.

### Darkmatter registration

This check depends on the Host registration v2 specification
(`2026-11-22-host-registration-v2`). Ship the full command after that
dependency lands. This command inspects the published files on demand,
and `doctor --fix` is an explicit caller of the shared publisher. Ordinary
commands keep the dependency's no-refresh startup behavior. Share the
inventory behind the hidden `claudine registrations` command in-process.
Compare the manifest and
every published artifact against the inventory using the hash of the
exact bytes from biscuit-hash. Inspect the managed directory
`~/.claudine/registrations/darkmatter/` for extras. Validate the
`register` entry in `~/.darkmatter.yaml` using Darkmatter's manifest
identity rules so equivalent reference spellings pass. A changed
manifest that Darkmatter cannot load as Claudine's registration is an
error. Preserve entries for other hosts.
[How Darkmatter uses host registrations](../../../darkmatter/docs/topics/host-registration.md)

`--fix` uses the shared publisher and Darkmatter's locked configuration
writer. It may overwrite modified published files and delete extras
inside Claudine's managed registration directory, as a normal publish
does. Report which paths are replaced or removed. Keep cleanup confined to
that directory and the two known legacy paths, `~/.claudine/darkmatter-host.yaml`
and `~/.claudine/darkmatter-host/`. Never follow symlinks or junctions for
repair or cleanup. Report an obstructing link and let the user resolve it.

Publish and verify the current inventory, update and verify the user
`register` entry, then clean up the legacy layout. Use the shared
publisher and coordinate this ordering with setup so both keep the
legacy files until the user configuration points to the replacement.
Serialize repairs with other callers of the publisher. If publication
fails partway through, report the incomplete inventory and leave legacy
paths available for a retry. A missing or invalid user Darkmatter file
needs separate setup or correction before this repair can update its
entry.

### Darkmatter permissions handoff

Check user scope and, inside a repository, repository scope when both
configuration files exist as a distinct pair. Report a skipped scope
when a required file is unavailable. Resolve references with Darkmatter's
normal scope-aware file resolution. A valid reference to Claudine's file
passes, even when its spelling differs from the setup example. A valid
third-party reference is informational. A broken or invalid reference
is an error and requires manual correction.

An omitted permissions setting or an inline block that setup could safely
transfer is an error with an available repair. When an inline block
conflicts with Claudine's authored permissions setting, report an error
and tell the user to choose which setting to keep. Use setup's
effective-value comparison so a retry after a partial write can finish an
already copied block. Reuse setup's
handoff logic and locked writer. Validate both candidates, preserve
unrelated settings and comments, and publish Claudine's block before
Darkmatter's reference. A failed second write leaves the original inline
block in force. The repair operates on existing valid files and does not
launch setup or convert legacy configuration.
[How Claudine setup moves permissions](../../docs/topics/configuration.md#darkmatters-permissions)

### Text-to-speech

Use the user configuration's effective `tts` setting. An absent, null,
or false setting disables hook speech. Lifecycle `say` can still use
automatic speech. If the user configuration cannot be read or validated,
report that speech settings could not be determined.

Use the detected candidates and base quality ratings from
[biscuit-speaks provider detection](../../../biscuit-speaks/lib/src/detection.rs).
For automatic selection, report the highest-rated available candidate and
identify the first candidate in runtime order if it differs. For a
recognized `tts.provider`, assess only that provider because runtime
selection is pinned to it. An unrecognized name produces at least a
warning and uses the automatic candidates, as runtime does. Report
missing candidates as an error when hook speech is enabled, even when
the name is unrecognized.

| Hook speech | No candidate | Highest available quality below `Good` | Available quality `Good` or higher |
|---|---|---|---|
| Enabled | `error` | `warn` | `ok` |
| Disabled | `info` | `info` | `ok` |

Detection measures local prerequisites, such as an executable or the
presence of a credential. A passing finding does not verify a working
voice, network access, credentials, or audio playback.

### Executable path

On macOS, Linux, WSL2, and native Windows, check the invoking process's
`PATH` using Sniff's
[`ExecutableIndex::build_path_only`](../../../sniff/lib/src/executable_index.rs),
which looks up the executable without consulting other install locations.
Do not launch the executable. A WSL2 run checks its own Linux environment.

On native Windows, also check each of these shells **when it is installed**:
PowerShell (`pwsh`, otherwise Windows PowerShell), Git Bash / MSYS2, and
Cygwin. Each can derive a different `PATH` when it starts. A shell that is
not installed is skipped and not reported.

- Find installed shells with Sniff.
- Ask each shell for `claudine` in a bounded subprocess with a timeout,
  no prompts, and no visible window.
- Keep three outcomes distinct in the report: the shell is absent, the
  probe failed (timeout, crash), or the lookup completed and found no
  executable.
- Run each probe **without** the shell's startup files (`pwsh -NoProfile`,
  `bash --norc --noprofile`), so a probe is fast, has no side effects, and
  never runs the user's own profile code. Say so in the finding, for
  example: "a fresh Git Bash without your startup files cannot find
  `claudine`."
- A miss in the invoking process is an `error` on every OS. A miss or a
  failed probe in another installed shell is a `warn`: it is listed with
  its remedy but cannot fail the run, so a shell the user never opens
  cannot fail `doctor` or a CI step that runs it.

## Documentation to update

- Add a `claudine/docs/cli/doctor.md` command reference and link it from the
  CLI index. Include the JSON contract, exit statuses, and repair scope.
- Add a troubleshooting link in `claudine/README.md` and describe the repair
  behavior in the configuration and registration topic pages.
- Update the Claudine repo skill and `use-claudine` caller skill. Publish
  the caller skill with `just publish-skills`.

## Tests

- Use L1 tests with temporary homes and repositories, fake speech
  detection, and controlled executable lookup. Cover passing, failing, and
  blocked findings. Use the Claudine CLI process fixture to verify that
  help and diagnostics work with missing or invalid configuration, and
  that an inspection creates no files, migrates no legacy files, and
  leaves registration untouched.
- Verify scope-specific validation, unknown-key locations, optional and
  aliased repository files, and the permissions check for equivalent,
  third-party, broken, and conflicting settings.
- Exercise repairs with stale or missing registration files, extra
  files, old layout remnants, and simulated write failures. Verify that
  repair preserves unrelated settings, comments, and other hosts'
  registrations, never writes through escaping links, and is a no-op on a
  second run after success. Verify that an interrupted permissions handoff
  can be retried and leaves the original policy in effect until the
  reference is recorded.
- Exercise terminal and JSON reports for warnings alone, unresolved
  errors, repaired errors, and failed repairs. Parse JSON from stdout and
  verify the stable row order and the exit status.
- Cover automatic and pinned speech, disabled hook speech, unrecognized
  provider names, and enabled speech with no candidate. Verify current-path
  lookup with present and absent executables, including platform-specific
  executable suffixes. Use L1 subprocess fixtures for installed, absent,
  timed-out, and failing Windows shell probes. These
  behaviors need no real terminal or focus changes.
