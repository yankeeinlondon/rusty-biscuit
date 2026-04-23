# DarkMatter Error Types

## Top-level (markdown module)

| Error | File | Variants |
|-------|------|----------|
| **`MarkdownError`** | `darkmatter/lib/src/markdown/types.rs:15` | `FrontmatterParse`, `FrontmatterMerge`, `FileLoad`, `UrlFetch`, `ThemeLoad`, `AstParse`, `InvalidLineRange`, `Serialization`, `Transform`, `Transclusion`, `TocLinking`, `ShellExpansion`, `PageBlock`, `Reference`, `CtxMerge` |

## Compose Pipeline

| Error | File | Variants |
|-------|------|----------|
| **`TransclusionError`** | `darkmatter/lib/src/markdown/compose/transclusion/types.rs:241` | `ParseDirective`, `InvalidReference`, `MissingSourceContext`, `UnsupportedReferenceType`, `UnsupportedFileType`, `NonTextCodeSource`, `CycleDetected`, `MaxDepthExceeded`, `ConditionEval`, `ConditionParse`, `Relevel`, `UrlExecutionDisabled`, `InvalidFrontmatterAssignment`, `InvalidReassignedFrontmatterProperty`, `Io`, `UrlParse`, `FileReference`, `Json` |
| **`DeferredSetError`** | `darkmatter/lib/src/markdown/compose/transclusion/types.rs:96` | `InvalidAssignment`, `ReassignedProperty` |
| **`ConditionError`** | `darkmatter/lib/src/markdown/compose/conditions.rs:14` | `Parse`, `Eval` |
| **`ShellExpansionError`** | `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:297` | `ParseDirective`, `CommandNotFound`, `Blacklisted`, `ApprovalRequired`, `Denied`, `NotPreApproved`, `Timeout`, `ExecutionFailed`, `PolicyIo` |
| **`TocLinkingError`** | `darkmatter/lib/src/markdown/compose/toc_linking/types.rs:11` | `ParseDirective`, `InvalidCleanupService`, `InvalidLevel`, `FileNotFound`, `InvalidGlob`, `Io` |
| **`PageBlockError`** | `darkmatter/lib/src/markdown/compose/page_blocks/types.rs:7` | `ParseDirective`, `UnmatchedEnd`, `UnterminatedBlock`, `Condition` |
| **`CtxMergeError`** | `darkmatter/lib/src/markdown/compose/context/merge.rs:10` | `InvalidUserCtx` |

## Normalize

| Error | File | Variants |
|-------|------|----------|
| **`NormalizationError`** | `darkmatter/lib/src/markdown/normalize/types.rs:399` | `LevelOverflow`, `ValidationFailed` |

## Reference Analysis

| Error | File | Variants |
|-------|------|----------|
| **`ReferenceError`** | `darkmatter/lib/src/markdown/reference/errors.rs:7` | `ParseDirective`, `MissingSourceContext`, `Validation`, `Compose`, `FileReference`, `Io`, `Url` |
| **`FileTreeError`** | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:44` | `PathNotFound`, `NotAFile`, `Markdown`, `Reference` |

## Render

| Error | File | Variants |
|-------|------|----------|
| **`StylesheetError`** | `darkmatter/lib/src/render/stylesheet.rs:18` | `InvalidDeclaration`, `InvalidPropertyName`, `PropertyValueTypeMismatch`, `InvalidSizing`, `InvalidSizingMulti`, `InvalidColor`, `InvalidInteger` |
| **`LinkError`** | `darkmatter/lib/src/render/link.rs:32` | `EmptyHref`, `UnrecognizedFormat`, `MalformedHtml`, `MalformedMarkdown`, `MissingHref`, `InvalidStyle`, `InvalidTarget` |
| **`ImageRefError`** | `darkmatter/lib/src/render/image_ref.rs:28` | `EmptySource`, `MissingSource`, `UnrecognizedFormat`, `MalformedHtml`, `MalformedMarkdown`, `InvalidStyle`, `InvalidDecoding`, `InvalidFetchPriority`, `InvalidLoading`, `InvalidReferrerPolicy` |

## Mermaid

| Error | File | Variants |
|-------|------|----------|
| **`MermaidThemeError`** | `darkmatter/lib/src/mermaid/theme.rs:14` | `InvalidJson`, `InvalidColor` |

## Editor

| Error | File | Variants |
|-------|------|----------|
| **`EditorError`** | `darkmatter/lib/src/editor/mod.rs:45` | `NoEditorFound`, `NonZeroExit`, `Missing`, `LaunchFailed`, `Io` |

## Summary

- **16 error enums** total across the darkmatter library
- 15 with `#[derive(thiserror::Error)]`
- 1 non-error enum (`DeferredSetError`) using `Debug + Clone + PartialEq + Eq` only
