# Version History for {{topic}}

You are researching the **{{topic}}** library.

**Library Context:**
- Package Manager: {{package_manager}}
- Language: {{language}}
- URL: {{url}}

Your task is to document this library's version history, focusing on major versions and significant releases. Use your training knowledge about this library - you do not have live access to repositories or APIs, so work from what you know.

## Output Requirements

Produce a Version History document with these sections:

### 1. Library Overview
- What the library does (1-3 sentences)
- Current major version (if known)
- Versioning scheme used (SemVer, CalVer, etc.)

### 2. Version Timeline

Create a table of significant versions:

| Version | Approximate Date | Significance |
|---------|------------------|--------------|
| ... | ... | ... |

Focus on:
- Major version releases (1.0, 2.0, 3.0, etc.)
- Significant minor releases that introduced important features
- Initial stable release (1.0 or 0.1)

### 3. Major Version Details

For each major version you know about, provide:

#### Version X.0
- **Key Changes**: 3-8 bullet points of what changed
- **Breaking Changes**: Explicit list of breaking changes (if any)
- **New Features**: Major user-facing additions
- **Migration Notes**: What users needed to do to upgrade

### 4. Notable Evolution

Describe the library's evolution over time:
- How has the API changed?
- What problems did different versions solve?
- Any significant rewrites or architectural changes?

### 5. Knowledge Gaps

Be explicit about what you don't know:
- Which versions you have limited information about
- Whether your knowledge might be outdated
- Suggest checking the official changelog/releases for current information

## Style Guidelines

- Be honest about uncertainty - prefix uncertain information with "Based on available information..." or "As of my training data..."
- Focus on what you actually know rather than speculating
- If you have very limited knowledge of this library's history, say so clearly and provide what you can
- Include the package manager URL for users to check current releases
- Write for experienced developers who want to understand the library's evolution
