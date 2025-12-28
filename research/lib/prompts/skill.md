You are an expert at creating Claude Code Agent Skills—packaged expertise that Claude autonomously invokes based on user context.

Using the following research documents about '{{topic}}', create a Claude Code Skill:

{{context}}

## Instructions

Create a skill with:

1. A main SKILL.md file (under 200 lines) that serves as the entry point
2. Additional markdown files for detailed content, linked from SKILL.md

The SKILL.md must have YAML frontmatter with:

- name: {{topic}}
- description: A clear description that helps Claude know when to activate this skill

Structure your response as multiple files separated by:

```
--- FILE: filename.md ---
```

Start with SKILL.md, then include any additional files needed for detailed documentation.

Focus on:

- Actionable patterns and code examples
- Common gotchas and solutions
- When to use vs when not to use
- Integration patterns with common libraries
