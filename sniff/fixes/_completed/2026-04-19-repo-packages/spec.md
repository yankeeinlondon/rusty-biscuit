---
fixed: 2026-04-20
agent: claude
---

# Fix `sniff repo packages`

There are a few problems we need to fix when running the `sniff repo packages` command:

1. performance is noticeably slow. this should be almost instantanious.
2. when we add the `--verbose` flag we get raw debugging messages, this is a code smell. Use 'cli' skill and review the best practices.

    ```sh
    2026-04-19T22:27:56.878570Z  INFO detect_filesystem: sniff::performance: performance stage complete stage=detect.filesystem duration_ms=1350.294458
    2026-04-19T22:27:56.878647Z  INFO sniff:detect_with_plan: sniff::performance: performance stage complete stage=detect.total duration_ms=1350.422708 command=Some("Repo { latest_versions: false, filter: [], repo_subcommand: Some(Packages { filter: [] }) }") json=false plain=false perf=false os=false hw=false net=false fs=true perf=false
    agent-sandbox-cli, biscuit-file-cli, biscuit-file, biscuit-hash-cli, biscuit-hash, biscuit-location-cli, biscuit-location, biscuit-speaks-cli, biscuit-speaks, biscuit-terminal-cli, biscuit-terminal, biscuit-visualized, claudine-cli, claudine, darkmatter-cli, darkmatter, arcam-amp-integration, homelab-cli, eversolo-integration, homelab, homelab-server, homelab-frontend, sony-receiver-integration, unfolded-integration-helper, messenger-cli, messenger, model-citizen-cli, model-citizen, playa-cli, playa, queue-cli, queue, research-cli, research, schematic-define, schematic-definitions, schematic-gen, schematic-oauth, schematic-schema, sniff-cli, sniff, tabby, ui, tree-hugger-cli, tree-hugger, tui, unchained-ai-cli, unchained-ai-gen, unchained-ai, model_id, worktree-cli, worktree
    ```

3. we need to add a `--package-area <area>` flag which will return just the packages in the specified package area
4. we need a few variant output types added as switches:
    - `--md`

        rather than returning elements as a CSV string, we return them as a Markdown UnorderedList

    - `--list`

        this just returns raw list with each entry getting it's own line

5. Let's change the output of the `--verbose` / `-v` flag to show the package's root dir:
    - CSV format:
        - `agent-sandbox-cli(<dim><i>./agent-sandbox/cli</i></dim>), ...`
    - MD format:
        - `- agent-sandbox-cli(<dim><i>./agent-sandbox/cli</i></dim>)`
    - LIST format:
        - `agent-sandbox-cli(<dim><i>./agent-sandbox/cli</i></dim>)\n`
