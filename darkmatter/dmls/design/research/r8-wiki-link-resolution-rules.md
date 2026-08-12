---
prompt: |-
  DMLS (the Darkmatter Language Server) ships wiki-style link support in v1:
  `[[target]]`, `[[target|alias]]`, `[[target#heading]]` resolved against
  the LSP workspace folder(s) with an optional `wiki_root` override. Ranking
  is: same directory → unique basename across the workspace → ambiguity
  (multiple locations + diagnostic). See
  @darkmatter/features/2026-07-04-dmls/spec.md (Layer 1) and the prior
  research in @darkmatter/dmls/design/wiki-style-links.md.

  Turn that outline into a precise, implementable rule set:

  1. Compare resolution algorithms in the wild at the mechanism level:
     Obsidian ("shortest path when possible" — find its documented/observed
     tie-breaking order), marksman, markdown-oxide, Foam, and IWE's
     key-based model. For each: how are basenames matched (case
     sensitivity? extension elision? spaces vs dashes?), how are folders
     and `index.md` handled, and how do they break ties?
  2. Propose DMLS's exact matching rules: case sensitivity policy on
     case-insensitive filesystems (macOS/Windows) vs case-sensitive
     (Linux) — the same workspace must resolve identically on all three;
     `.md` extension handling; subpath targets (`[[folder/note]]`);
     Unicode normalization (NFC/NFD — macOS filenames!); spaces in targets.
  3. Define heading-anchor semantics for `[[target#heading]]`: exact
     heading-text match vs slug match, and how that interacts with the
     GitHub-style slug algorithm used for Markdown `#anchors`.
  4. Define completion insertion policy: what text gets inserted when the
     user picks a file that is ambiguous by basename (full relative path?)
     and the config surface for path style (shortest / relative / absolute
     from wiki root).
  5. Define rename safety: when a file participating in wiki-links is
     renamed, which link spellings can be rewritten confidently, and which
     must be refused (ambiguity created or removed by the rename)?
  6. Enumerate the diagnostic cases: unresolved target, ambiguous target,
     ambiguous-after-rename, and heading-missing-in-target.

  Deliverables: a comparison table of existing tools, DMLS's rule set
  written as a decision list an implementer can code directly, a
  cross-platform gotcha list, and a fixture-workspace design (files +
  expected resolutions) for the test suite.
last_updated: 2026-07-06
hash: cbb0138477a4c0a7-73f496d54898e512
---
# DMLS Wiki-Style Link Resolution Rules

## Existing Tool Comparison

| Tool           | Resolution universe                                                               | File target forms                                                                                                                                                                       | Case policy                                                                                                                   | Extension handling                                                                                     | Spaces / dashes / encoding                                                                                                                        | Folder and `index.md` handling                                                                                                                                                                                                                              | Tie-breaking / ambiguity                                                                                                                                                                                                                                                                                   | Heading target behavior                                                                                                                                                                                                                                                                                                                                                |
|----------------|-----------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Obsidian       | One vault.                                                                        | `[[note]]`, `[[folder/note]]`, `[[/folder/note]]`, relative-style links for inserted links depending on settings.                                                                       | Not fully specified in public docs; behavior is platform/vault implementation dependent. Do not copy this as a DMLS contract. | `.md` is normally omitted in wikilinks.                                                                | Literal spaces are normal. Obsidian docs warn that `#`, \`                                                                                         | `, `^`, `:`, `%%`, `[\`, and `](<`, and `>)\` may break filenames or link destinations.                                                                                                                                                                         | Folder path components can be emitted when needed. Public docs do not define `index.md` semantics for wikilinks.                                                                                                                                                                                           | Documented insertion policy is "shortest path when possible": filename when unique; add enough path components when duplicate filenames exist. Public docs say resolution finds the "best match" but do not publish a complete deterministic tie-break order. Source: [Obsidian internal links](https://obsidianmd-obsidian-help.mintlify.app/linking/internal-links). |
| Marksman       | LSP project folder.                                                               | Wiki links can refer to document title or filename. Completion style controls preferred spelling: `title-slug`, `file-stem`, or `file-path-stem`.                                       | Not clearly documented as an OS-independent policy.                                                                           | File completion styles omit extension. Tests cover filenames with dots and `.md` forms.                | Supports URL-encoded spaces in links in tests; title slug style turns heading/title text into slug-like names.                                    | `file-path-stem` can include folders. No documented `index.md` rule.                                                                                                                                                                                        | Ambiguity is resolved according to configured completion style. A link like `[[foo]]` may bind to title `# Foo` under title style or file stem `foo.md` under file style; refactors follow that binding. Source: [Marksman features](https://github.com/artempyanykh/marksman/blob/main/docs/features.md). | Cross-document headings are referenceable when title-from-heading mode is active. Heading anchor tests include URL-encoded and plain heading forms.                                                                                                                                                                                                                    |
| markdown-oxide | Workspace/vault. Inspired by Obsidian.                                            | Wikilinks and Markdown links; supports files, headings, indexed blocks, tags, and unresolved placeholders.                                                                              | Implementation tests include case-insensitive wikilink resolution.                                                            | `.md` is stripped for reference text in several parsing paths.                                         | Literal spaces work. It also recognizes slugified heading variants by replacing spaces with `-`; source code also lowercases heading comparisons. | Recent tests cover directory links: prefer `bar.md`; otherwise `bar/index.md`; otherwise `bar/README.md`; trailing slash is equivalent. Directory-index resolution is configurable. Source: [markdown-oxide configuration](https://oxide.md/Configuration). | Older/current implementation behavior is resolution-oriented rather than ambiguity-diagnostic-first. Tests show ambiguous basename resolving to the first indexed match in some cases.                                                                                                                     | Heading matching accepts both heading text and a slugified form. The in-code slug helper replaces spaces with dashes, but is not a full GitHub slug implementation.                                                                                                                                                                                                    |
| Foam           | Workspace roots.                                                                  | Two modes: path links if target starts with `/` or `.`; otherwise identifier links. Identifier links are path suffixes such as `[[todo]]`, `[[house/todo]]`, `[[projects/house/todo]]`. | Docs imply identifier matching across the workspace; implementation tests include case-insensitive resolution.                | `.md` is omitted from identifiers; generated Markdown reference definitions include actual file paths. | Literal spaces are supported. Dashes are not aliases for spaces unless present in the filename.                                                   | Current implementation tests include directory index fallback: `bar.md` wins over `bar/index.md`, then `bar/README.md`. Public docs focus on identifiers and paths.                                                                                         | Public docs say Foam picks the shortest unambiguous identifier for writing. If `[[todo]]` is ambiguous, Foam resolves alphabetically for deterministic behavior and emits a warning diagnostic. Source: [Foam wikilinks](https://github.com/foambubble/foam/blob/main/docs/user/features/wikilinks.md).    | Supports section links with `[[resource#Section Title]]`, plus navigation, completion, embeds, and diagnostics for note sections.                                                                                                                                                                                                                                      |
| IWE / IWES     | Library document set keyed by path-like `Key` values without document extensions. | Wiki links resolve by path suffix across the whole document set. `[[topic]]` matches any key ending in `topic`; `[[shared/topic]]` matches any key ending in those segments.            | Key comparison is string/key based, not filesystem-case based.                                                                | `.md` is optional and stripped from wiki links.                                                        | Percent-decoding is applied before key matching. Spaces can exist in keys, but IWE’s note creation often uses slug templates.                     | No `index.md` special case in the key model; documents are keys.                                                                                                                                                                                            | Deterministic non-diagnostic tie-break: matching keys are sorted by fewest path segments, then lexicographic path. \`wiki_link_path = preserve                                                                                                                                                              | full                                                                                                                                                                                                                                                                                                                                                                   |

## DMLS Decision List

### Definitions

1. A DMLS wiki root is a logical root used for wiki-link indexing.
2. If `wiki_root` is configured, index only Markdown files under that root. `wiki_root` is resolved relative to the containing LSP workspace folder unless absolute.
3. If `wiki_root` is not configured, each LSP workspace folder is a wiki root.
4. In multi-root workspaces, every indexed document has:
    - `workspace_id`: stable LSP workspace folder identity.
    - `root_relative_path`: slash-separated path from the wiki root.
    - `canonical_logical_path`: `root_relative_path` after separator normalization, extension stripping, and Unicode normalization.

5. Only Markdown documents are wiki-link file targets in v1. Recognized extensions are `.md` and `.markdown` unless DMLS later exposes a config for Markdown extensions.
6. Paths inside wiki links always use `/` as the separator, including on Windows.

### Target Parsing

1. Parse only these v1 forms:
    - `[[target]]`
    - `[[target|alias]]`
    - `[[target#heading]]`
    - `[[target#heading|alias]]`
    - `[[#heading]]`
    - `[[#heading|alias]]`

2. Split alias at the first unescaped `|`.
3. Split heading at the first unescaped `#` in the target side.
4. Backslash escaping is recognized only for `\|`, `\#`, `\]`, and `\\` inside wiki links.
5. Empty target with no heading is invalid: `[[]]`, `[[|alias]]`.
6. Empty heading after `#` is invalid for diagnostics and completion should treat it as an incomplete heading query.
7. `![[...]]`, `[[...#^block]]`, aliases from frontmatter, block references, and interwiki prefixes are post-v1.

### Normalization

1. DMLS must not depend on host filesystem case behavior. macOS, Windows, and Linux must resolve the same workspace content identically.
2. Normalize all indexed logical paths and wiki-link file targets to Unicode NFC before matching.
3. Preserve the original spelling for edits, hover, completion display, and rename rewrites unless a rule explicitly says to insert a normalized spelling.
4. Case is significant everywhere. `[[Foo]]` matches `Foo.md`; it does not match `foo.md`.
5. Emit a workspace diagnostic for files whose NFC-normalized, extensionless logical paths collide exactly. These are not safely distinguishable by DMLS.
6. Emit a workspace diagnostic for files whose logical paths differ only by Unicode normalization or by ASCII/Unicode case fold. These are cross-platform portability hazards even though DMLS matching remains case-sensitive.
7. Literal spaces are significant and supported: `[[My Note]]` matches `My Note.md`.
8. Dashes and spaces are not interchangeable for file targets: `[[my-note]]` does not match `my note.md`.
9. Percent escapes are decoded once before file-target matching for URI compatibility. `[[My%20Note]]` and `[[My Note]]` therefore resolve to the same file. Invalid percent escapes are left literal and produce a low-severity diagnostic.

### Extension Handling

1. A wiki-link target may include or omit a final Markdown extension.
2. `[[note]]`, `[[note.md]]`, and `[[note.markdown]]` all query the same extensionless logical target `note`.
3. Extension elision applies only to the final path segment.
4. Non-Markdown extensions are not stripped. `[[image.png]]` is unresolved as a v1 wiki file target unless DMLS later supports attachment links.
5. A physical file named `note.md.md` has logical path `note.md`, so `[[note.md]]` can target it. This is legal but should receive a portability/info diagnostic because it is visually confusing.

### Path Matching

1. Convert backslashes in wiki-link targets to `/` only for diagnostics and completion suggestions; do not treat `\` as a path separator in accepted syntax. On Windows, users still write `[[folder/note]]`.
2. Reject targets containing empty path segments, `.` segments, or `..` segments for v1 wiki-links. `[[./note]]` and `[[../note]]` are Markdown-link concepts, not DMLS wiki-link concepts.
3. A target beginning with `/` is root-relative within the current wiki root. It must match exactly one indexed `canonical_logical_path` in that root after removing the leading `/`.
4. A target containing `/` but not beginning with `/` is a path-suffix target:
    - Match indexed documents whose `canonical_logical_path` ends with exactly those path segments.
    - If exactly one candidate exists, resolve to it.
    - If multiple candidates exist, apply same-directory rank only when one candidate is in the same directory as the source document and its suffix match is also valid.
    - If same-directory rank does not produce exactly one winner, report ambiguity.

5. A bare target with no `/` is a basename target:
    - Candidate set is every indexed document whose final logical path segment equals the target.
    - If the source document's directory contains a candidate with that basename, resolve to that candidate.
    - Otherwise, if exactly one candidate exists in the active wiki roots, resolve to it.
    - Otherwise, report ambiguity.

6. Same-directory rank means the target file's parent directory equals the source document's parent directory within the same wiki root.
7. Same-directory rank intentionally wins over a unique-looking global basename because it preserves the outline from the existing Layer 1 spec: same directory, then unique basename, then ambiguity.
8. DMLS does not use lexicographic fallback for real resolution. Lexicographic order is only for stable display of ambiguous candidate lists.
9. DMLS does not implement implicit `index.md` or `README.md` folder targets in v1. `[[folder]]` targets `folder.md`, not `folder/index.md`. This avoids silently importing Foam/website semantics into Markdown notes.

### Multi-Root Matching

1. If the source document belongs to a wiki root, same-directory rank is evaluated only inside that wiki root first.
2. If no same-directory candidate exists, all configured wiki roots are searched.
3. A bare or suffix target that matches one candidate in each of two roots is ambiguous unless the target is root-relative and the current root yields exactly one match.
4. Completion and diagnostics must display root labels for candidates outside the source root.

## Heading Anchor Semantics

DMLS must distinguish author-facing wiki heading text from rendered Markdown anchor slugs.

1. For `[[target#heading]]`, first resolve `target` to a document using the file rules above.
2. If `target` is empty, the target document is the source document.
3. Build two heading indexes for each document:
    - `heading_text_index`: NFC-normalized exact visible heading text.
    - `github_slug_index`: GitHub-style generated anchor IDs for rendered Markdown links.

4. Exact heading-text match is the primary wiki-link rule. `[[note#My Heading]]` resolves to a heading whose visible text is exactly `My Heading` after NFC normalization.
5. Case is significant for heading-text matching. `# Details` is not `# details`.
6. If no exact heading-text match exists, try a GitHub-style slug match. `[[note#my-heading]]` can resolve to `# My Heading` because `my-heading` is its rendered Markdown anchor.
7. If both an exact heading text and a slug match exist but they point to different headings, exact heading text wins and DMLS emits an informational diagnostic suggesting a less ambiguous spelling.
8. If multiple headings have the same exact visible text, `[[note#Heading]]` is ambiguous unless the duplicate headings also have distinct GitHub slugs and the link used one of those distinct slug forms.
9. GitHub-style slug generation follows GitHub’s documented behavior:
    - Use rendered inline text, not raw Markdown marker text.
    - Lowercase.
    - Remove characters not allowed in GitHub heading fragments.
    - Convert whitespace runs to single hyphens.
    - Preserve allowed non-ASCII characters where GitHub would preserve them.
    - For duplicates in one document, append `-1`, `-2`, and so on in document order.    Source: [GitHub writing docs](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax).

10. Completion after `#` inserts visible heading text by default, not the slug, because wiki links are author-facing and Obsidian-compatible.
11. Add config `wiki.heading_completion_style = "text" | "slug"` with default `"text"`.
12. Markdown links continue to use slug anchors. Wiki links accept both exact heading text and slug forms for navigation, diagnostics, and rename.

## Completion Insertion Policy

Add config:

```toml
[wiki]
path_style = "shortest" # "shortest" | "relative" | "root-relative"
heading_completion_style = "text" # "text" | "slug"
```

1. `shortest` is the default.
2. `shortest` inserts the shortest suffix that resolves uniquely under DMLS rules from the source document.
3. If the basename is unique after same-directory ranking, insert `[[note]]`.
4. If a basename is ambiguous, insert the shortest suffix that uniquely identifies the selected target: `[[folder/note]]`, then `[[parent/folder/note]]`, up to the full root-relative logical path without a leading slash.
5. If even the full root-relative path is ambiguous because multiple wiki roots contain the same path, insert a root-labeled edit only if the client supports additional text edits or detail labels; otherwise insert the full root-relative path and keep ambiguity diagnostics active. v1 should prefer diagnostics over inventing root-prefix syntax.
6. `relative` inserts a POSIX-style relative path from the source document's directory to the target document without extension, but only if the resulting path does not contain `.` or `..`. If it would require `..`, fall back to `shortest` and include completion detail explaining the fallback.
7. `root-relative` inserts `/` plus the target's root-relative logical path without extension.
8. Completion never inserts `.md` by default.
9. Completion preserves literal spaces. It does not slugify file paths.
10. When completing a heading:
- `wiki.heading_completion_style = "text"` inserts `#Visible Heading`.
- `wiki.heading_completion_style = "slug"` inserts `#github-style-slug`.
11. When completing a file plus heading, use the selected file path style first, then append the heading fragment.

## Rename Safety

### File Rename

For a file rename from old target `A` to new target `B`:

1. Build the pre-rename index and resolve every wiki-link occurrence.
2. A link participates in the rename only if it resolves uniquely to `A` before the rename.
3. Links that are unresolved before the rename are not rewritten.
4. Links that are ambiguous before the rename are not rewritten and receive `ambiguous-target`.
5. Simulate the post-rename index.
6. For each participating link, compute the preferred replacement spelling using `wiki.path_style`.
7. Accept the rewrite only if the replacement spelling resolves uniquely to `B` in the post-rename index.
8. If the old spelling would still resolve uniquely to `B`, DMLS may leave it unchanged unless the rename changed a path segment present in the link text.
9. If the old spelling would resolve to a different document after the rename, rewrite if the new spelling is unique; otherwise refuse.
10. If the rename creates a new ambiguity for a previously safe short spelling, rewrite to the shortest unique suffix.
11. If no unique suffix exists within the wiki root because of cross-root duplication, refuse and emit `ambiguous-after-rename`.
12. Preserve aliases: `[[old|Alias]]` becomes `[[new|Alias]]`.
13. Preserve heading fragments if the heading still exists in the renamed file.
14. If the heading fragment no longer resolves after the file rename, rewrite the file portion only and emit `heading-missing-in-target`.
15. Do not rewrite a link whose target text uses unsupported v1 syntax.

### Heading Rename

For a heading rename in target document `D`:

1. Resolve all `[[target#heading]]` and `[[#heading]]` links.
2. Rewrite only links whose file portion resolves uniquely to `D` and whose heading portion resolves uniquely to the renamed heading before the edit.
3. If the link used exact heading text, replace with the new visible heading text.
4. If the link used a GitHub slug, replace with the new GitHub slug.
5. If config `wiki.heading_completion_style` is `"text"`, new completions use text, but rename preserves the existing spelling class where possible.
6. If duplicate headings make the old heading reference ambiguous, refuse the rewrite and report ambiguity.

## Diagnostic Cases

| Code                             | Severity                                                                         | Trigger                                                                                                                | Message shape                                                                                                       | Related information                                                                       |
|----------------------------------|----------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| `wiki.unresolved-target`         | Warning                                                                          | File target candidate set is empty.                                                                                    | `Unresolved wiki target: [[target]]`                                                                                | Suggested create-file action path using configured new-note location or source directory. |
| `wiki.ambiguous-target`          | Warning                                                                          | File target candidate set has multiple candidates after same-directory and uniqueness rules.                           | `Ambiguous wiki target: [[target]] resolves to multiple files`                                                      | Related locations for every candidate, sorted by root label then path.                    |
| `wiki.ambiguous-after-rename`    | Error for rename prepare/apply; warning if detected during ordinary diagnostics. | A rename would make a rewritten or preserved link resolve to multiple files, or no unique replacement spelling exists. | `Cannot safely update wiki link because the rename would make it ambiguous`                                         | Old target, new target, candidate locations.                                              |
| `wiki.heading-missing-in-target` | Warning                                                                          | File target resolves but heading fragment does not match any exact heading text or GitHub slug.                        | `Heading not found in wiki target: #heading`                                                                        | Related location for target file and suggested heading completions.                       |
| `wiki.empty-target`              | Warning                                                                          | `[[]]` or \`[\[                                                                                                          | alias]\]\`.                                                                                                           | `Wiki link target is empty`                                                               |
| `wiki.empty-heading`             | Hint while editing; warning on save if persisted.                                | `[[target#]]`.                                                                                                         | `Wiki heading target is empty`                                                                                      | Heading completions for target.                                                           |
| `wiki.unsupported-syntax`        | Information                                                                      | v1 sees embed, block ref, interwiki prefix, or other unsupported extension.                                            | `This wiki-link form is not supported by DMLS v1`                                                                   | None.                                                                                     |
| `wiki.portability-collision`     | Warning at workspace scope.                                                      | Indexed paths collide under NFC normalization or case fold.                                                            | `Wiki target differs only by case or Unicode normalization and may not round-trip across macOS, Windows, and Linux` | Related locations for colliding files.                                                    |
| `wiki.invalid-percent-escape`    | Information                                                                      | Target contains malformed `%` escape.                                                                                  | `Invalid percent escape in wiki target; treating it literally`                                                      | None.                                                                                     |

## Cross-Platform Gotchas

1. macOS often stores or exposes filenames in decomposed Unicode forms; DMLS indexes and matches NFC to avoid `é` vs `e + combining acute` surprises.
2. Windows and default macOS filesystems are case-insensitive; Linux usually is not. DMLS resolution is case-sensitive on all platforms to keep results identical.
3. A Linux workspace can contain both `Note.md` and `note.md`; that workspace cannot be cloned faithfully to many macOS/Windows filesystems. DMLS must flag this.
4. Windows disallows several filename characters that POSIX filesystems allow. DMLS should not create notes with `<>:"\|?*` or trailing spaces/dots when offering create-file actions.
5. `:` is common in prose but problematic in Windows filenames and Obsidian docs warn it may break link destinations. DMLS should allow existing files but avoid generating new ones with `:`.
6. Backslash is not a wiki path separator. Always insert `/`.
7. Percent decoding must be performed before Unicode normalization, exactly once.
8. Symlink traversal must be explicit. v1 should either ignore symlinked Markdown files or canonicalize them with loop detection; do not index the same physical file twice under two logical paths without a diagnostic.
9. Multi-root workspaces can contain the same root-relative path in more than one root. DMLS must not silently pick the first root.
10. Git case-only renames are hard on case-insensitive filesystems. Rename support must detect old and new logical paths that differ only by case and still simulate the post-rename index.

## Fixture Workspace Design

Use one fixture tree with two wiki roots to exercise all rules:

```text
workspace/
  root-a/
    index.md
    notes/
      Source.md
      Target.md
      Target Duplicate.md
      My Note.md
      my-note.md
      Case.md
      case.md
      café.md              # NFD spelling on disk if the test harness can create it
      café.md               # NFC spelling; skip physical dual-file creation on filesystems that cannot represent both
      folder/
        Nested.md
        Target.md
      other/
        Target.md
      same/
        Local.md
      ambiguous/
        Same.md
      also-ambiguous/
        Same.md
      headings/
        Headed.md
      literal/
        note.md.md
  root-b/
    notes/
      Target.md
      ambiguous/
        Same.md
```

`root-a/notes/Source.md` should contain:

```markdown
# Source

[[Target]]
[[folder/Target]]
[[/notes/folder/Target]]
[[My Note]]
[[My%20Note]]
[[my-note]]
[[Case]]
[[case]]
[[Same]]
[[ambiguous/Same]]
[[Headed#Exact Heading]]
[[Headed#exact-heading]]
[[Headed#Missing Heading]]
[[#Local Heading]]
[[literal/note.md]]
[[No Such Note]]
[[Target|Alias]]
[[Target#Exact Heading|Alias]]
```

`root-a/notes/headings/Headed.md` should contain:

```markdown
# Headed

## Exact Heading

## Duplicate

## Duplicate

## Symbols & Greek Θ!

## exact-heading
```

`root-a/notes/Source.md` should also contain:

```markdown
## Local Heading
```

Expected resolutions from `root-a/notes/Source.md`:

| Link                         | Expected result                                                                                                                                                |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `[[Target]]`                 | `root-a/notes/Target.md` because same-directory rank wins.                                                                                                     |
| `[[folder/Target]]`          | `root-a/notes/folder/Target.md` because suffix is unique within `root-a` unless `root-b` creates the same suffix; if duplicated, ambiguous.                    |
| `[[/notes/folder/Target]]`   | `root-a/notes/folder/Target.md` root-relative to current wiki root.                                                                                            |
| `[[My Note]]`                | `root-a/notes/My Note.md`.                                                                                                                                     |
| `[[My%20Note]]`              | `root-a/notes/My Note.md`.                                                                                                                                     |
| `[[my-note]]`                | `root-a/notes/my-note.md`; no space/dash equivalence.                                                                                                          |
| `[[Case]]`                   | `root-a/notes/Case.md`.                                                                                                                                        |
| `[[case]]`                   | `root-a/notes/case.md`; also emit portability collision for `Case.md` / `case.md`.                                                                             |
| `[[Same]]`                   | Ambiguous between `root-a/notes/ambiguous/Same.md`, `root-a/notes/also-ambiguous/Same.md`, and `root-b/notes/ambiguous/Same.md`.                               |
| `[[ambiguous/Same]]`         | Ambiguous if both roots contain `notes/ambiguous/Same.md`; otherwise resolves to the single suffix match.                                                      |
| `[[Headed#Exact Heading]]`   | `root-a/notes/headings/Headed.md`, heading `Exact Heading`, exact-text match.                                                                                  |
| `[[Headed#exact-heading]]`   | Exact-text match to heading `exact-heading`, not slug match to `Exact Heading`, because exact text wins. Emit informational ambiguous-spelling note if useful. |
| `[[Headed#Missing Heading]]` | File resolves; emit `wiki.heading-missing-in-target`.                                                                                                          |
| `[[#Local Heading]]`         | Heading `Local Heading` in `Source.md`.                                                                                                                        |
| `[[literal/note.md]]`        | `root-a/notes/literal/note.md.md` because only the final `.md` is stripped from the target. Emit confusing-extension info diagnostic.                          |
| `[[No Such Note]]`           | `wiki.unresolved-target` with create-file action.                                                                                                              |
| \`[\[Target                    | Alias]\]\`                                                                                                                                                       |
| \`[\[Target#Exact Heading      | Alias]\]\`                                                                                                                                                       |

Rename fixtures:

1. Rename `root-a/notes/Target.md` to `Renamed.md`.
    - `[[Target]]` in same directory rewrites to `[[Renamed]]`.
    - `[[Target|Alias]]` rewrites to `[[Renamed|Alias]]`.
    - Links to `folder/Target` and `other/Target` are unchanged.

2. Rename `root-a/notes/folder/Target.md` to `root-a/notes/other/Target.md` when `other/Target.md` already exists.
    - Refuse with `wiki.ambiguous-after-rename` or filesystem conflict before edits.

3. Rename `root-a/notes/ambiguous/Same.md` to `Unique.md`.
    - Do not rewrite ambiguous `[[Same]]` occurrences because they did not uniquely resolve before rename.
    - Rewrite `[[ambiguous/Same]]` only if it uniquely resolved before the rename and `[[Unique]]` or the configured replacement path uniquely resolves after.

4. Rename heading `Exact Heading` in `Headed.md` to `Renamed Heading`.
    - `[[Headed#Exact Heading]]` rewrites to `[[Headed#Renamed Heading]]`.
    - A slug-form link to the same heading rewrites to the new slug only if it resolved uniquely before the rename.
