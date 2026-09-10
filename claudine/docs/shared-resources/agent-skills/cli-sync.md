```sh
# synchronizes Agent Skills (not other shared resources)
claudine skills sync
# synchronizes all shared resources (skills, prompts, agent definitions, mcp)
claudine sync
```

## CLI Switches

- `--fix` will try to fix malformed agent skills

## Frontmatter Props

- if the `sync` Frontmatter property is set to `false` on an Agent Skill then this skill will _not_ be synced, however, non-synced skills will be called out in the CLI reporting (as having not been synced)

## User and Repo scoping

- running these commands will _always_ effect user-scoped skills
- it will also effect repo scoped skills when called from within a repo
