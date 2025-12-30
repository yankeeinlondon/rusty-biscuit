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
4. **NO file separator before first file** - Do NOT include "--- FILE: SKILL.md ---" at the start

SKILL.md MUST start with this EXACT format (no code fences):

---
name: {{library_name}}
description: Expert knowledge for [technology/library] - [key capabilities]. Use when [specific scenarios].
tools: [Read, Write, Edit, Grep, Glob, Bash]
---

# {{library_name}}

[Rest of content...]

**Description Field Guidance:**

The `description` field should:
- Start with action verb (e.g., "Build", "Expert knowledge for", "Use when")
- Describe WHAT the skill provides (capabilities, expertise)
- Describe WHEN to activate it (use cases, scenarios)
- Be 1-2 sentences, concise but informative
- Help Claude decide if this skill is relevant to the current task

GOOD: "Expert knowledge for building Rust CLI interfaces with clap v4, covering derive macros, subcommands, argument validation, and shell completions. Use when designing CLI argument parsing, implementing commands, or troubleshooting clap issues."

BAD: "A skill about clap" (too vague)
BAD: "This skill covers everything about clap including installation, configuration, usage, examples, best practices, troubleshooting, and more." (too verbose)

**Multi-File Structure:**

IMPORTANT: The file separator "--- FILE: filename.md ---" is ONLY used BETWEEN files, not before the first file.

If you're creating multiple files, structure like this:

[SKILL.md content starts here - no separator before it]
---
name: library-name
description: ...
tools: [Read, Write, Edit, Grep, Glob, Bash]
---

# Content...

--- FILE: advanced-usage.md ---
# Advanced Usage
...

--- FILE: examples.md ---
# Examples
...

Start with SKILL.md (no file separator), then include additional files for detailed documentation (required, not optional).

Focus on:

- Actionable patterns and code examples
- Common gotchas and solutions
- When to use vs when not to use
- Integration patterns with common libraries
