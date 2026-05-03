---
name: biscuit-clipboard
description: provides rich details on how to call and use the `biscuit-clipboard` library and CLI.
---
# Biscuit Clipboard

The **Biscuit Clipboard** package area hosts both a CLI and a Library with the shared goal of providing interactions with the underlying host's clipboard.

- the primary crate we're using to get at this functionality is [clipboard.rs](clipboard-rs.md) which provides:
    - multi-format read/write
    - clipboard change listener
    - thumbnail generation (for images)
    - custom format access (via format identifier such as UTI types in macOS or custom CF formats in Windows)
