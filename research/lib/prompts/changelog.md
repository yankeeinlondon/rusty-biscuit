# Version History for {{topic}}

You are synthesizing changelog information for the **{{topic}}** library.

**Library Context:**
- Package Manager: {{package_manager}}
- Language: {{language}}
- URL: {{url}}

## Pre-gathered Version Data

The following version information was gathered from structured sources:

{{version_data}}

**Data Confidence:** {{confidence_level}}
**Sources Used:** {{sources_used}}

## Your Task

Based on the pre-gathered data above (if available) and your training knowledge:

1. **If structured data exists:** Synthesize it into a readable changelog format, enriching with context where helpful. Use the version dates, significance levels, and breaking changes/features provided. Add additional context from your knowledge where it enhances understanding.

2. **If no structured data exists:** Generate the best changelog you can from your knowledge, being explicit about uncertainty. Follow the format below but clearly indicate which information is based on your training data.

3. **If you cannot produce a confident changelog:** Generate a minimal timeline showing:
   - First known stable release date
   - Major version release dates (1.0, 2.0, etc.)
   - Current version information

## Required Output Format

Your response MUST begin with YAML frontmatter containing these required fields:

```yaml
---
created_at: {{current_date}}
updated_at: {{current_date}}
latest_version: "X.Y.Z"
confidence: high|medium|low
sources:
  - github_releases
  - changelog_file
  - registry_versions
  - llm_knowledge
---
```

**Frontmatter Field Descriptions:**
- `created_at`: ISO 8601 date when this changelog was generated (YYYY-MM-DD)
- `updated_at`: ISO 8601 date of last update (same as created_at for new changelogs)
- `latest_version`: The most recent version string (e.g., "2.5.3")
- `confidence`: high (multiple structured sources), medium (one structured source or LLM-enriched), low (LLM only)
- `sources`: List of data sources used (github_releases, changelog_file, registry_versions, llm_knowledge)

## Document Structure

After the frontmatter, structure your changelog with these sections:

### 1. Library Overview
- What the library does (1-3 sentences)
- Current major version
- Versioning scheme used (SemVer, CalVer, etc.)

### 2. Version Timeline

Create a table of significant versions:

| Version | Release Date | Significance |
|---------|--------------|--------------|
| 2.0.0 | 2024-01-15 | Major - Breaking API changes |
| 1.5.0 | 2023-08-20 | Minor - New async support |
| 1.0.0 | 2022-03-01 | Major - Initial stable release |

Focus on:
- Major version releases (1.0, 2.0, 3.0, etc.)
- Significant minor releases that introduced important features
- Initial stable release (1.0 or 0.1)

### 3. Major Version Details

For each major version, provide:

#### Version X.0 (Release Date)

**Key Changes:**
- 3-8 bullet points of what changed
- Focus on user-facing changes

**Breaking Changes:**
- Explicit list of breaking changes (if any)
- How they affect existing code

**New Features:**
- Major user-facing additions
- API enhancements

**Migration Notes:**
- What users needed to do to upgrade
- Links to migration guides if known

### 4. Notable Evolution

Describe the library's evolution over time:
- How has the API changed?
- What problems did different versions solve?
- Any significant rewrites or architectural changes?
- Community adoption milestones

### 5. Data Quality Notes

Be transparent about the data quality:

**Confidence Level:** [High/Medium/Low]

**Sources Used:**
- List which sources contributed to this changelog
- Indicate reliability of each source

**Knowledge Gaps:**
- Which versions have limited information
- Whether knowledge might be outdated
- Recommend checking official sources: {{url}}

## Style Guidelines

- **Dates:** Use ISO 8601 format (YYYY-MM-DD) for all dates in the body content
- **Versions:** Use exact version strings from structured data when available
- **Uncertainty:** Clearly mark uncertain information with "Based on available information..." or "As of my training data..."
- **Accuracy:** Do not invent version numbers or dates - if you don't know, say so
- **Context:** Enrich structured data with context from your knowledge, but don't contradict it
- **Audience:** Write for experienced developers who want to understand the library's evolution

## Example Minimal Timeline (Low Confidence)

If you cannot produce a detailed changelog, provide at minimum:

```markdown
### Version Timeline

Based on limited available information:

| Version | Approximate Date | Significance |
|---------|------------------|--------------|
| 2.0.0 | Unknown | Major version (current) |
| 1.0.0 | Unknown | Initial stable release |

**Note:** Detailed version history is not available. Please check the official repository for accurate changelog information: {{url}}
```
