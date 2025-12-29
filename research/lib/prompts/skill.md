You are an expert at creating Claude Code Agent Skills—packaged expertise that Claude autonomously invokes based on user context.

Using the following research documents about '{{topic}}', create a Claude Code Skill:

{{context}}

## Instructions

Create a skill with:

1. A main SKILL.md file (under 200 lines) that serves as the entry point
2. Additional markdown files for detailed content, linked from SKILL.md

**CRITICAL FORMAT REQUIREMENTS** (violations break skill parsing):

1. **NO code fences around frontmatter** - Do NOT wrap the frontmatter in ```yaml blocks
2. **Frontmatter must start on line 1** - No blank lines or content before the opening `---`
3. **Always create linked files** - SKILL.md must link to topic files; a standalone SKILL.md is insufficient

The SKILL.md must start EXACTLY like this (raw YAML, not in a code block):

---
name: {{topic}}
description: A clear description that helps Claude know when to activate this skill
---

# Topic Name

[Content here...]

Structure your response as multiple files separated by:

--- FILE: filename.md ---

Start with SKILL.md, then include additional files for detailed documentation (required, not optional).

Focus on:

- Actionable patterns and code examples
- Common gotchas and solutions
- When to use vs when not to use
- Integration patterns with common libraries
