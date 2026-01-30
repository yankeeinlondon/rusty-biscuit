# Incremental Research

The research system uses a DRY (Don't Repeat Yourself) approach to avoid redundant work.

## Existence Check

Before running research, the library checks for existing metadata:

```rust
let metadata_path = output_dir.join("metadata.json");
if metadata_path.exists() {
    // Incremental mode: check for overlaps
} else {
    // Full research pipeline
}
```

## When Incremental Mode Activates

1. `metadata.json` exists in the output directory
2. User provides additional research questions
3. System compares new questions against existing documents

## Overlap Detection

Uses Gemini Flash for semantic comparison of new prompts against existing documents.

### PromptOverlap Structure

```rust
struct PromptOverlap {
    prompt: String,
    filename: String,
    verdict: OverlapVerdict,
    conflict: Option<String>,  // Conflicting file if overlap
}

enum OverlapVerdict {
    New,      // No overlap, proceed
    Conflict, // Potential overlap with existing doc
}
```

### Detection Flow

1. Load existing `metadata.json`
2. Extract all existing document content
3. For each new prompt:
   - Send prompt + existing docs to Gemini Flash
   - Get semantic overlap assessment
   - Return verdict (New or Conflict)

## Interactive Selection

When overlaps are detected:

### Single Prompt with Conflict
```
The prompt "How does it compare to structopt?" overlaps with "similar_libraries.md".
Include anyway? [y/N]
```

### Multiple Prompts
```
Select prompts to include:
> [x] How does the derive macro work? (NEW)
  [ ] How does it compare to structopt? (conflicts with similar_libraries.md)
> [x] What are the performance characteristics? (NEW)
```

Defaults:
- **New prompts**: selected by default
- **Conflicting prompts**: unselected by default

## Re-Synthesis After Adding Documents

When new underlying documents are added:

1. New files written to research directory
2. `metadata.json` updated with new `additional_files` entries
3. Phase 2 synthesis re-runs with expanded corpus
4. `updated_at` timestamp updated

## Force Mode

The `--force` flag bypasses incremental mode entirely:

```bash
research library clap --force
```

This regenerates ALL documents, including:
- All Phase 1 underlying research
- All Phase 2 synthesis outputs
- Updated timestamps

## Skill-Only Regeneration

The `--skill` flag regenerates only the skill output:

```bash
research library clap --skill
```

Requirements:
- All underlying research documents must exist
- Removes existing `skill/*` contents
- Regenerates `SKILL.md` and supporting files

Useful when:
- Skill template has been updated
- Want to refresh skill format without re-researching

## Missing Standard Prompts

The system can detect missing standard prompts:

```rust
const STANDARD_PROMPTS: [(&str, &str, &str); 5] = [
    ("overview", "overview.md", prompts::OVERVIEW),
    ("similar_libraries", "similar_libraries.md", prompts::SIMILAR_LIBRARIES),
    ("integration_partners", "integration_partners.md", prompts::INTEGRATION_PARTNERS),
    ("use_cases", "use_cases.md", prompts::USE_CASES),
    ("changelog", "changelog.md", prompts::CHANGELOG),
];
```

Use `research list --verbose` to see which documents are missing for each topic.

## Best Practices

1. **Start fresh for major updates**: Use `--force` when the library has significant changes
2. **Use incremental for questions**: Add specific questions without re-running all prompts
3. **Regenerate skills periodically**: Use `--skill` when skill template improves
4. **Review overlaps carefully**: Conflicting prompts might still add value
5. **Check completion status**: Use `research list --verbose` to find incomplete research
