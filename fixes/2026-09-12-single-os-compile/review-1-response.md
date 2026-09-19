# Review 1 response

The code corrections are implemented. Rollout acceptance remains incomplete;
this response does not supersede the original review or claim hosted success.

| Finding | Correction | Regression boundary |
|---|---|---|
| Hosted fan-out loses shared compilation | One matrix leg per producer; one `produce-owner.sh` invocation; programmatic per-package uploads | Real Cargo shared-dependency fixture calls the workflow script; publisher tests inject upload failure and verify sibling publication and package identities |
| Dirty/mismatched source accepted | Clean tracked checkout at the planned commit; actual tree checked before/after build and at consumer; supplied tree is only an assertion | Tracked modification, forged tree label, and edited manifest tree all refused |
| Runtime declarations treated as observations | Inspect shipped binaries and sidecars; compare actual external library identities and record/select the probed linker | A real external C dynamic dependency is discovered during production; removing it makes verification fail without editing the manifest |
| macOS Bash failure | Indexed arrays, Bash 3.2 empty-array handling, separate Unix/PowerShell quoting | Cross-check shipping suite runs under `/bin/bash` on macOS |
| Owner measurement unreachable | Workflow switch enables per-record counters through shared owner script; package artifacts carry owner totals and wall time | Shared fixture proves one identical dependency compile and two incompatible feature configurations |
| Python test requires newer interpreter | TemporaryDirectory uses `addCleanup` | Python 3.9 canonical suite |

## Deliberate compatibility limits

External native libraries must match their observed content hashes or Mach-O
UUIDs; macOS system libraries additionally require the same OS build. This is
conservative and can reject ABI-compatible upgrades. It does not broaden
Linux/WSL2 compatibility by assuming their declared environments are equivalent.

Archive mode requires clean tracked source. Uncommitted local work can use an
explicit native diagnostic invocation; it cannot be labeled with a committed
archive identity. Hosted archive consumers never rebuild on refusal.

## Verification

- macOS: archive/counter suite **114 passed**, workflow contracts **116 passed**.
- Linux: archive/counter suite **114 passed** in an isolated remote worktree;
  the worktree and its target directory were removed after verification.
- Publisher failure-isolation test: passed with the upload API stubbed.
- Clippy with warnings denied, actionlint, `git diff --check`, and
  `just ci-local --plan`: passed.
- Python 3.9 canonical suite: **590 passed**.

GitNexus's full working-tree analysis reports high aggregate risk: 467 changed
symbols in 66 files and 11 affected flows, including the implementation that
predated this correction pass. The response carried neither `partial` nor
`truncated`. This is a CI-wide change, not a low-risk merge clearance.

## Still required for acceptance

- Matched hosted pre/post cold and warm measurements, three consecutive green
  observations per environment, and the 15% architecture decision.
- A hosted run through the actual artifact service. Local publisher tests stub
  only the upload API; they cannot establish hosted transport availability.
- Current native Windows and WSL2 behavioral evidence. The declared Windows
  host had 36 GiB free on W:, below the 50 GiB repository preflight floor; WSL
  SSH timed out. No storage preflight was bypassed.

No repository commit or push was made.
