# Code Review: Type Safety, DRY, and Test Coverage

**Date**: 2026-04-03
**Scope**: `research/research/` (lib + CLI crates)
**Focus**: Type safety improvements, DRY violations, test coverage gaps

---

## Summary

The research library is a ~5,200-line Rust codebase with a well-structured module system. However, there are significant opportunities for improvement in three areas: (1) replacing stringly-typed fields with proper enums, (2) eliminating large blocks of duplicated code, and (3) filling test gaps for core orchestration functions.

**Severity ratings**: High = should fix soon (correctness or significant maintainability risk), Medium = worth fixing in next iteration, Low = nice to have.

---

## 1. Type Safety Issues

### 1.1 `ChangelogFrontmatter.confidence` is a `String` (High)

**File**: `lib/src/validation/frontmatter.rs:82`

```rust
pub struct ChangelogFrontmatter {
    pub confidence: String,  // should be ConfidenceLevel enum
    ...
}
```

The `confidence` field is validated against `"high"`, `"medium"`, `"low"` at runtime (lines 610-614) but stored as a bare `String`. The `changelog::types::ConfidenceLevel` enum already exists with exactly these variants (`High`, `Medium`, `Low`). Using the enum would:

- Eliminate the runtime validation check entirely (serde handles it)
- Make invalid states unrepresentable
- Remove the `confidence_lower` normalization hack (line 610)

**Recommendation**: Change to `pub confidence: ConfidenceLevel` with `#[serde(rename_all = "lowercase")]`. Add a serde deserializer if backward compatibility with existing YAML frontmatter is needed.

### 1.2 `ChangelogFrontmatter` date fields are `String` (High)

**File**: `lib/src/validation/frontmatter.rs:73-76`

```rust
pub struct ChangelogFrontmatter {
    pub created_at: String,   // "YYYY-MM-DD"
    pub updated_at: String,   // "YYYY-MM-DD"
    ...
}
```

These store ISO 8601 dates as strings. The codebase already depends on `chrono` and has `parse_flexible_date()` in `changelog::types.rs`. Using `NaiveDate` would:

- Catch malformed dates at deserialization time
- Enable date arithmetic without parsing
- Align with `VersionInfo.release_date` which already uses `DateTime<Utc>`

**Recommendation**: Change to `pub created_at: NaiveDate` with a custom serde deserializer that handles both `NaiveDate` and the existing string format.

### 1.3 `TopicInfo.topic_type` is a `String` (High)

**File**: `lib/src/list/types.rs:68`

```rust
pub struct TopicInfo {
    pub topic_type: String,  // "library", "framework", "software", etc.
}
```

This field holds one of a known set of values (`"library"`, `"framework"`, `"software"`, `"person"`, `"solution_area"`, `"language"`) but is a `String`. Meanwhile, `ResearchType` in `validation/health.rs` defines 4 of these as an enum, and `ResearchKind` in `lib.rs` defines 2. The values are compared case-insensitively via `to_lowercase()` in `filter.rs:87`, which is error-prone.

**Recommendation**: Create a single `TopicType` enum covering all 6+ variants. Replace both `ResearchType` and the string-based `topic_type` with this unified enum. See issue 1.5 below for the overlap problem.

### 1.4 `VersionSignificance` logic bug in `from_version_str` (High)

**File**: `lib/src/changelog/types.rs:127-135`

```rust
let significance = if !semver.pre.is_empty() {
    VersionSignificance::Prerelease
} else if semver.major > 0 {
    VersionSignificance::Major    // <-- BUG
} else if semver.minor > 0 {
    VersionSignificance::Minor
} else {
    VersionSignificance::Patch
};
```

Any version with `major > 0` is classified as `Major`, regardless of whether it's a patch release. For example, `2.0.1` (a patch on 2.0.0) gets `Major` significance, and `1.2.3` (a patch) also gets `Major`. This defeats the purpose of having `Minor` and `Patch` significance levels.

The existing test at line 326-327 confirms this behavior is intentional but incorrect:
```rust
let v1 = VersionInfo::from_version_str("2.5.0").unwrap();
assert_eq!(v1.significance, VersionSignificance::Major); // "2.5.0" is Minor, not Major
```

**Recommendation**: This function should compare against the *previous* version to determine significance (major change = Major, etc.), or the significance should be determined by the caller who has context about the version range. At minimum, the function should be renamed to `from_version_str_unranked` and default to a neutral significance, with the actual classification happening at a higher level.

### 1.5 Overlapping type hierarchies for research kinds/types (Medium)

The codebase has three overlapping enums/types that describe "what kind of research this is":

| Type | Location | Variants |
|------|----------|----------|
| `ResearchKind` | `lib/src/lib.rs:275` | `Library`, `Api` |
| `ResearchType` | `lib/src/validation/health.rs:20` | `Library`, `Tool`, `Software`, `Framework` |
| `ResearchDetails` | `lib/src/metadata/types.rs:39` | 15 variants (LibraryDetails, ApiDetails, etc.) |
| `topic_type` (String) | `lib/src/list/types.rs:68` | `"library"`, `"framework"`, `"software"`, `"person"`, `"solution_area"`, `"language"` |

These overlap significantly but none is a superset of the others. `ResearchKind` has `Api` but no `Tool`. `ResearchType` has `Tool` but no `Api`. `topic_type` has `person` and `solution_area` but these aren't represented anywhere else. `ResearchDetails` is a detail-level enum with 15 variants that somewhat corresponds to `ResearchKind` but has its own taxonomy.

**Recommendation**: Unify into a single type hierarchy:
- `ResearchCategory` enum: the top-level distinction (Library, Api, Tool, Software, Framework, Person, SolutionArea, Language)
- `ResearchDetails` remains as the detail-level enum, but with a clear mapping from each variant to its `ResearchCategory`

### 1.6 `LibraryInfo` vs `LibraryInfoMetadata` near-duplicate structs (Medium)

**File**: `lib/src/lib.rs:248-254` and `lib/src/lib.rs:317-323`

```rust
pub struct LibraryInfo {
    pub package_manager: String,
    pub language: String,
    pub url: String,
    pub repository: Option<String>,
    pub description: Option<String>,      // only difference
}

pub struct LibraryInfoMetadata {
    pub package_manager: String,
    pub language: String,
    pub url: String,
    pub repository: Option<String>,
    // no description field
}
```

These are the same struct except `LibraryInfo` has a `description` field. The `From<&LibraryInfo> for LibraryInfoMetadata` impl at line 325 just drops `description`.

**Recommendation**: Use a single `LibraryInfo` struct with `#[serde(skip_serializing_if = "Option::is_none")]` on `description`. Remove `LibraryInfoMetadata` entirely.

### 1.7 `LinkError::HomeDirectory` carries no context (Low)

**File**: `lib/src/link/types.rs` (referenced at `link/mod.rs:104-111`)

```rust
detection::get_claude_skills_dir().map_err(|_| LinkError::HomeDirectory)?;
```

The `HomeDirectory` error variant discards the original error message, making debugging difficult. If `get_claude_skills_dir()` fails, you get a generic "home directory" error with no indication of *which* directory or *why* it failed.

**Recommendation**: Change to `LinkError::HomeDirectory { path: PathBuf, source: std::io::Error }` or at minimum `LinkError::HomeDirectory(String)`.

---

## 2. DRY Violations

### 2.1 Agent preamble string repeated 12+ times (High)

**File**: `lib/src/lib.rs` (lines 2364, 2382, 2402, 2421, 2444, 3695, 3713, 3733, 3752, 3771, 3790, 3812)

The string `"You are a research assistant with web search and scraping tools..."` appears at least 12 times with minor variations. Three distinct variants are used:

1. **Standard preamble** (~10 occurrences): `"You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer."`
2. **Changelog variant** (~1 occurrence): `"You are a research assistant with web search and scraping tools. Search for recent releases, changelogs, and version history..."`
3. **Synthesis variant** (~1 occurrence): `"You are a research assistant with web search and scraping tools. You have been provided with pre-gathered version data..."`

**Recommendation**: Define constants or a helper function:
```rust
const AGENT_PREAMBLE_STANDARD: &str = "You are a research assistant...";
const AGENT_PREAMBLE_CHANGELOG: &str = "You are a research assistant... changelogs...";
const AGENT_PREAMBLE_SYNTHESIS: &str = "You are a research assistant... pre-gathered...";
```

### 2.2 `link/mod.rs` skill symlink creation tripled (High)

**File**: `lib/src/link/mod.rs:198-295`

The skill symlink creation logic (determine action -> attempt creation -> handle errors) is copy-pasted verbatim for Claude Code (lines 198-226), OpenCode (lines 228-256), and Roo Code (lines 258-286). Each block is ~28 lines of identical pattern matching on `SkillAction::CreatedLink`, `CreationError::InvalidSource`, `PermissionDenied`, etc. — only the target directory and error label differ.

The deep-dive doc linking at lines 303-400 has the same triplication problem with a slightly different logic branch (check_is_symlink first, then check_local_definition_exists).

**Recommendation**: Extract a helper:
```rust
fn create_skill_link(
    source: &Path,
    target: &Path,
    service_name: &str,
    topic_name: &str,
    errors: &mut Vec<(String, String)>,
) -> SkillAction { ... }
```
Then call it 3 times in a loop:
```rust
let actions: Vec<SkillAction> = [
    (&claude_skills_dir, "Claude Code"),
    (&opencode_skills_dir, "OpenCode"),
    (&roo_skills_dir, "Roo Code"),
].iter().map(|(dir, name)| create_skill_link(source, &dir.join(&topic.name), *name, &topic.name, &mut errors))
.collect();
```
Same pattern for deep-dive docs. This would reduce ~200 lines to ~60.

### 2.3 `run_changelog_agent_task` vs `run_changelog_completion_task` near-duplicate (Medium)

**File**: `lib/src/lib.rs`

These two functions share ~80% identical code: version history aggregation, prompt building, result writing, and span recording. The only meaningful difference is the LLM call (one uses an agent with tools, one uses a plain completion).

**Recommendation**: Extract shared logic into a `ChangelogContext` builder that handles the version aggregation and prompt building, then call the appropriate LLM method at the end.

### 2.4 Phase 2 brief generation duplicated (Medium)

The brief generation code (reading deep_dive, calling Gemini, parsing BRIEF/SUMMARY markers, writing brief.md) is copy-pasted between `research()` and `run_incremental_research()`. The combined context building (reading all phase 1 docs, building `combined_context` from template) is also duplicated.

**Recommendation**: Extract `generate_brief()` and `build_combined_context()` as standalone functions.

### 2.5 `AssistantContent::Text` extraction repeated 7 times (Medium)

**File**: `lib/src/lib.rs` (lines 1261, 1531, 1694, 2017, 2751, 4133, 5098)

The pattern of extracting text from completion responses appears in slightly different forms:
```rust
// Pattern A (filter_map):
content.iter().filter_map(|c| match c {
    AssistantContent::Text(text) => Some(text.text.clone()),
    _ => None,
}).collect::<Vec<String>>().join("")

// Pattern B (if let):
if let AssistantContent::Text(text) = c {
    // use text.text
}
```

**Recommendation**: Add a helper:
```rust
fn extract_text_content(content: &[AssistantContent]) -> String {
    content.iter().filter_map(|c| match c {
        AssistantContent::Text(text) => Some(text.text.as_str()),
        _ => None,
    }).collect()
}
```

---

## 3. Test Coverage Gaps

### 3.1 Core orchestration functions untested (High)

These functions form the primary public API and have zero test coverage:

| Function | Location | Description |
|----------|----------|-------------|
| `research()` | `lib.rs` (~780 lines) | Main research orchestration |
| `research_api()` | `lib.rs` | API-specific research |
| `run_prompt_task()` | `lib.rs` | Core prompt execution |
| `run_agent_prompt_task()` | `lib.rs` | Agent-based prompt execution |
| `run_question_task()` | `lib.rs` | Question-answering |
| `run_changelog_agent_task()` | `lib.rs` | Changelog with agent |
| `run_changelog_completion_task()` | `lib.rs` | Changelog with completion |

These are large, complex functions that orchestrate multi-phase LLM interactions. The lack of tests means regressions in prompt construction, context building, and result parsing go undetected.

**Recommendation**: At minimum, add integration tests with mocked LLM responses (the codebase already uses `wiremock` for other tests) that verify:
- Prompt templates are correctly assembled
- Phase transitions produce expected artifacts
- Error paths are handled gracefully
- Brief/summary parsing works correctly

### 3.2 Skill generation untested (Medium)

| Function | Description |
|----------|-------------|
| `generate_skill_files()` | Creates SKILL.md from research output |
| `regenerate_skill_from_existing_research()` | `--skill` flag path |
| `delete_research_output_documents()` | `--force` flag path |

`generate_skill_files()` is particularly important — it's the function that produces the primary output artifact (SKILL.md files) consumed by agentic CLIs. A bug here could silently corrupt skill content.

### 3.3 `parse_brief_response` untested (Medium)

The brief parser extracts `BRIEF:` and `SUMMARY:` markers from LLM output. This is fragile string parsing with no tests for edge cases: missing markers, malformed output, markers in wrong order, extra whitespace, multi-line values.

### 3.4 Library discovery functions untested (Medium)

| Function | Description |
|----------|-------------|
| `find_library()` | Main library discovery entry point |
| `check_crates_io()` | crates.io API check |
| `check_npm()` | npm registry check |
| `check_pypi()` | PyPI API check |

These hit external APIs, making them hard to unit test, but they could be tested with `wiremock` (already a dev dependency).

### 3.5 `link/mod.rs` tests are superficial (Medium)

The three existing tests (lines 460-486) only verify the function "runs without crashing" — they don't assert any specific behavior about symlink creation, stale link removal, or error handling. The tests accept both `Ok` and `Err(Discovery(_))` as passing.

**Recommendation**: Use a temp directory structure with mock skill directories to verify actual symlink creation and stale link cleanup behavior.

### 3.6 No property-based testing for key invariants (Low)

Several types have mathematical invariants that would benefit from property-based testing (e.g., `proptest`):

- `VersionInfo` ordering transitivity and consistency with semver
- `resolve_collision` determinism
- Filter matching correctness across all `TopicType` variants
- `parse_flexible_date` round-tripping through ISO 8601 format

### 3.7 `get_researchrary_path()` has a typo (Low)

**File**: `lib/src/pull.rs`

The function name `get_researchrary_path()` appears to be a typo of "research library". This should be `get_research_library_path()`.

---

## 4. Priority Matrix

| # | Issue | Severity | Effort | Impact |
|---|-------|----------|--------|--------|
| 1.4 | `from_version_str` significance bug | High | Small | Correctness |
| 1.1 | `ChangelogFrontmatter.confidence` as String | High | Small | Type safety |
| 1.2 | Date fields as String | High | Small | Type safety |
| 2.1 | Agent preamble 12x duplication | High | Small | Maintainability |
| 2.2 | Link symlink creation 3x duplication | High | Medium | Maintainability |
| 3.1 | Core orchestration untested | High | Large | Reliability |
| 1.3 | `TopicInfo.topic_type` as String | High | Medium | Type safety |
| 1.5 | Overlapping type hierarchies | Medium | Large | Clarity |
| 1.6 | `LibraryInfo` / `LibraryInfoMetadata` duplication | Medium | Small | DRY |
| 2.3 | Changelog task duplication | Medium | Medium | DRY |
| 2.4 | Brief generation duplication | Medium | Medium | DRY |
| 2.5 | Text extraction duplication | Medium | Small | DRY |
| 3.2 | Skill generation untested | Medium | Medium | Reliability |
| 3.3 | `parse_brief_response` untested | Medium | Small | Reliability |
| 3.4 | Library discovery untested | Medium | Medium | Reliability |
| 3.5 | Link tests superficial | Medium | Medium | Reliability |
| 1.7 | `LinkError::HomeDirectory` no context | Low | Small | Debuggability |
| 3.6 | No property-based tests | Low | Medium | Reliability |
| 3.7 | `get_researchrary_path` typo | Low | Small | Naming |

---

## 5. Suggested Implementation Order

1. **Quick wins** (type safety, low effort):
   - Fix `from_version_str` significance logic (1.4)
   - Change `confidence` to `ConfidenceLevel` enum (1.1)
   - Extract agent preamble constants (2.1)
   - Extract `extract_text_content` helper (2.5)

2. **Medium refactorings**:
   - Consolidate `LibraryInfo` structs (1.6)
   - Extract link helper to deduplicate `link/mod.rs` (2.2)
   - Create `TopicType` enum to replace string (1.3)
   - Add tests for `parse_brief_response` (3.3)

3. **Larger efforts**:
   - Unify type hierarchies (1.5)
   - Extract shared changelog logic (2.3)
   - Extract shared brief generation (2.4)
   - Add integration tests for core orchestration (3.1)
