# `library` command

The **research** CLI has a `library` command which generates comprehensive research documentation for software libraries and packages. This command uses AI agents to research a library and produce multiple artifacts including a SKILL.md file for Claude Code integration, a deep dive document, and a brief summary.

## Syntax

> **research library** \<library-name\> [FLAGS]

### Parameters

- `library-name` **(required)**: The name of the library to research
  - Can be a simple name like `axum` or `pulldown-cmark`
  - Can include language/ecosystem context like `rust/tokio` or `typescript/vitest`
  - The command will detect the programming language and ecosystem

### Switches

- `--output`, `-o` <path>: Specify output directory for research artifacts
  - Defaults to `~/.claude/research/` for global research
  - Can specify a local path like `./research/` for project-specific research

- `--skill`
    - Adding this flag indicates the user believes the topic is already finished (with both underlying and final outputs rendered/produced)
    - If the user is correct that the topic has all files produced (note: an invalid structure to the `SKILL.md` is fine and often the reason someone would use this command) then:
        - we will removed the files in the topic's `skill/*` directory (but not remove the directory itself as that would break symbolic links that point to this directory)
        - We then regenerate the skill files from the underlying research documents
- `--force`
    - This will force the recreation of all ResearchOutput documents even if the documents were already created.

## Output

The command generates several artifacts in the output directory under `library/<library-name>/`:

### Generated Files

1. **skill/SKILL.md** - Claude Code skill file with YAML frontmatter
   - Contains expert knowledge formatted for Claude Code
   - Includes frontmatter with metadata:

     ```yaml
     ---
     name: library-name
     description: Expert knowledge for [library] - [capabilities]. Use when [scenarios].
     tools: [Read, Write, Edit, Grep, Glob, Bash]
     ---
     ```

   - May include additional supporting documentation files
   - **Validation**: Frontmatter is validated for proper YAML structure and required fields

2. **deep_dive.md** - Comprehensive technical documentation
   - In-depth coverage of library features, architecture, and patterns
   - Code examples and advanced usage scenarios
   - Integration patterns and best practices

3. **brief.md** - Quick reference summary
   - One-page overview of the library
   - Key concepts and common patterns
   - Quick-start examples

4. **metadata.json** - Research metadata

   ```json
   {
     "schema_version": 0,
     "topic": "library-name",
     "type": "library",
     "library_info": {
       "name": "library-name",
       "language": "Rust|TypeScript|Python|etc",
       "ecosystem": "crates.io|npm|pypi|etc",
       "repository": "https://github.com/..."
     },
     "brief": "One sentence description",
     "summary": "Longer summary paragraph",
     "when_to_use": "Expert knowledge for... Use when...",
     "created_at": "2025-12-29T...",
     "updated_at": "2025-12-29T...",
     "additional_files": ["question_1.md", "question_2.md"],
     "research_questions": ["What is...?", "How does...?"]
   }
   ```

### Research Process

The command executes a multi-phase research workflow:

1. **Phase 0: Discovery** - Detect library language, ecosystem, and gather basic info
2. **Phase 1: Core Research** - Use LLM agents to research library documentation and sources
3. **Phase 2a: Generate Artifacts** - Create SKILL.md and deep_dive.md in parallel
4. **Phase 2b: Generate Brief** - Create brief summary from deep dive document
5. **Validation** - Validate SKILL.md frontmatter and extract metadata
6. **Finalization** - Write metadata.json and create symlinks if requested

### Frontmatter Validation

The command validates SKILL.md frontmatter and provides actionable error messages:

**Success:**

```
✓ SKILL.md frontmatter is valid
✓ Updated metadata.when_to_use
```

**Failure:**

```
⚠️  Warning: SKILL.md frontmatter is invalid
   Missing required field: description
   File: /path/to/skill/SKILL.md
   The skill may not activate correctly until this is fixed.
```

Validation checks for:

- Proper YAML structure with `---` delimiters
- Required fields: `name`, `description`
- Optional fields: `tools`, `last_updated`, `hash`
- No empty field values
- Valid YAML syntax

### Terminal Output

During execution, the command displays progress information:

```
🔍 Researching library: axum

Phase 1: Core Research
▸ Gathering library information...
▸ Researching documentation...
✓ Core research complete (45.2s)

Phase 2: Generating Artifacts
▸ Generating SKILL.md...
▸ Generating deep_dive.md...
✓ Artifacts generated (62.8s)

Phase 2b: Generating Brief
▸ Creating brief summary...
✓ Brief generated (12.3s)

Validation
✓ SKILL.md frontmatter is valid
✓ Updated metadata.when_to_use

📦 Research complete!
   Location: ~/.claude/research/library/axum/
   Files: SKILL.md, deep_dive.md, brief.md, metadata.json
```

## Examples

### Basic Usage

Research a Rust library:

```bash
research library axum
```

Research a TypeScript library:

```bash
research library vitest
```

### Incremental Update

Update existing research with new information:

```bash
research library axum --incremental
```

### Custom Output Location

Generate research in a project-specific directory:

```bash
research library serde --output ./docs/research
```

### Force Regeneration

Completely regenerate all artifacts:

```bash
research library tokio --force
```

### Verbose Output

See detailed progress and debugging information:

```bash
research library pulldown-cmark --verbose
```

## Integration with Claude Code

After generating research, use the `research link` command to create symbolic links so Claude Code can discover and use the skills:

```bash
research library axum
research link axum
```

Or link all library skills at once:

```bash
research link -t library
```

The skill will then be automatically activated by Claude Code when working on projects that use the library.

## Notes

- The command requires an OpenAI API key configured in your environment
- Research generation can take 1-3 minutes depending on library complexity
- The `when_to_use` field in metadata.json is automatically populated from the SKILL.md frontmatter description
- All generated markdown files are normalized for consistent formatting (except SKILL.md frontmatter which is preserved exactly)
- Multi-file skills are supported via `--- FILE: filename.md ---` separators in LLM output

## Error Handling

The command handles various error conditions gracefully:

- **Invalid frontmatter**: Warns but continues, allowing manual fixes
- **Missing library info**: Prompts for manual input or uses defaults
- **Network errors**: Retries with exponential backoff
- **Rate limiting**: Respects API rate limits and waits appropriately
- **Validation failures**: Reports errors with file paths and actionable guidance

All errors are logged and displayed with context to help debug issues.
