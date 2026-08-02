# Online resources

Consult these when this skill is insufficient — particularly for config keys, which are the least
well covered by the public docs and are best read from the tool itself.

## Primary sources

| Resource | URL | Use for |
| --- | --- | --- |
| Product page | https://kunobi.ninja/product/kache | Overview, positioning, headline claims |
| Docs (index) | https://kunobi.ninja/docs/kache | Navigation to all doc sections |
| Installation | https://kunobi.ninja/docs/kache/getting-started/installation | Per-OS install commands |
| GitHub repo | https://github.com/kunobi-ninja/kache | Source of truth; Apache-2.0 |
| README | https://github.com/kunobi-ninja/kache/blob/main/README.md | What is/isn't cached, key composition |
| Releases | https://github.com/kunobi-ninja/kache/releases | Version history, key-version bumps |
| CI action | https://github.com/kunobi-ninja/kache-action | Full input reference for GitHub Actions |

## Blog posts worth reading in full

| Post | URL | Why |
| --- | --- | --- |
| What kache actually caches | https://kunobi.ninja/blog/what-kache-actually-caches | Definitive on cached vs skipped, incremental, key composition |
| kache storage, measured | https://kunobi.ninja/blog/kache-storage-worktrees | The Firefox benchmark, reflink vs hardlink storage numbers, and the authors' own caveats |
| Open-sourcing kache | https://kunobi.ninja/blog/open-sourcing-kache | Positioning vs sccache, design rationale |

The storage post is unusually candid about measurement limits — it flags which figures are directly
measured versus noisy, and that it's a single sample. Cite it accordingly rather than quoting the
headline numbers as universal.

## The tool is the best reference

The public docs don't publish a complete config-key table. The binary does:

```bash
kache --help                 # command surface for the installed version
kache <cmd> --help           # exact flags and defaults
kache doctor                 # resolved paths, wiring, restore mode, health
kache config                 # interactive config editor
```

Prefer this over web docs when versions might differ — flags and defaults change between releases,
and `--help` is always true for the binary in front of you.

## Diagnosing with the built-in tools

| Question | Command |
| --- | --- |
| Is it even wired up? | `kache doctor` |
| Why didn't this crate hit? | `kache why-miss <crate>` |
| What's consuming the store? | `kache list --sort size` |
| Is it actually helping? | `kache stats --since 24h`, `kache monitor` |
| Where did build time go? | `kache report --format perfetto -o trace.json` |

## Related tooling

- **cargo-sweep** — https://github.com/holmgr/cargo-sweep — stale-artifact pruning inside `target/`.
  Still relevant alongside kache for keying speed; see [when-not-to-use.md](when-not-to-use.md).
- **sccache** — https://github.com/mozilla/sccache — the incumbent kache positions against.

## Facts worth re-verifying over time

These were true as of **July 2026, kache 0.7.0** and are the most likely to drift:

- Default `local_max_size` (50 GiB) and the CI action's `max-size` default
- Whether proc-macros/executables remain excluded by default
- Whether C/C++ artifacts are still local-only
- Windows daemon service mechanism, and whether ReFS block cloning is used for restores
- The planner service's status (preview at time of writing)
- Whether incremental compilation remains disabled — the single fact most worth re-checking, since
  it drives the main adoption tradeoff
