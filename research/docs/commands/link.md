# `link` command

The **research** CLI has a `link` command which is intended to create symbolic links to all of our skills so that Claude Code and OpenCode can use them.

- use symbolic link's from the known skills to the appropriate location for [Claude Code](https://code.claude.com/docs/en/overview) and [OpenCode CLI](https://opencode.ai/docs/)'s expected location for **user scoped** skills.
    - **Claude Code** expects skills to reside in `~/.claude/skills/{{NAME}}`
    - **OpenCode CLI** expects skills to reside in `~/.config/opencode/skill/{{NAME}}`

## Syntax

> **research link** \<filter\> [FLAGS]

- when the user types `research link` then all the skills known about (see [research filesystem](../research-filesystem.md)) will be iterated over (after filter is applied) and create a symbolic link for CLI tools which don't already have a symbolic link or locally defined skill.

**Note:** independent checks for each CLI tool must be made to check for both tools

**Note:** the filter parameters in `research link` work the same as they do in `research list`.

### Switches

- `--type`, `-t` this switch provides a filter on a particular _type_ of research topic:
    - `research link -t library` will link all topics which are of the type "library"
    - more than one type can be achieved by using this switch more than once. For instance, `research link -t library -t software` will link all topics which are either of the type "library" or "software".

By default the diagnostic output will be displayed to the screen in a "terminal friendly manner":
    
- `- {{SKILL}}: added {{BOLD}}link{{RESET}} to both {{YELLOW}}Claude Code{{RESET}} {{ITALIC}}and{{RESET}}`
- `- {{DIM}}{{SKILL}}: {{ITALIC}}already linked{{RESET}}`
- `- {{SKILL}}: already had a local definition for this skill`
- `- {{SKILL}}: {{YELLOW}}Claude Code{{RESET}} already had a local definition for this skill, created link for {{YELLOW}}OpenCode{{RESET}}`
- etc.

If the user uses the `--json` switch then the diagnostics will be reported as a JSON array of the type SkillLink:

```rust
enum SkillAction {
    created_link,
    none_link_existed,
    none_local_definition
}


struct SkillLink {
    name: string;
    claude_action: SkillAction;
    opencode_action: SkillAction;
}
```

