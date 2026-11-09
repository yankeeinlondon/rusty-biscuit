```sh
# synchronizes Agent Skills (not other shared resources)
claudine skills --fix
# slash commands and agent definitions have their own --fix
claudine commands --fix
claudine agents --fix
```

There is no `skills sync` subcommand: `claudine skills sync` lists the skills whose names contain "sync". `claudine sync` re-registers hooks only; it does not touch skills, commands, agents, or MCP servers.

## CLI Switches

- `--fix` (alias of `--apply`) links shared skills into providers that are missing them, and repairs fixable frontmatter problems (tab indentation, a missing `name`) first

## Frontmatter Props

- **planned:** setting the `sync` Frontmatter property to `false` on an Agent Skill will keep that skill from being synced, and the CLI report will call it out as not synced; today every skill is synced

## User and Repo scoping

- running these commands will _always_ effect user-scoped skills
- it will also effect repo scoped skills when called from within a repo
