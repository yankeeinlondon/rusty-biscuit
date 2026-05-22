---
name: biscuit-clipboard
description: Expert knowledge for the biscuit-clipboard Rust library and CLI for cross-platform clipboard access. Use when working in the biscuit-clipboard package area, reading or writing the system clipboard, adding the biscuit-clipboard dependency, or using its CLI.
---
# Biscuit Clipboard

The **Biscuit Clipboard** package area hosts both a CLI and a Library with the shared goal of providing interactions with the underlying host's clipboard.

- the primary crate we're using to get at this functionality is [clipboard.rs](clipboard-rs.md) which provides:
    - multi-format read/write
    - clipboard change listener
    - thumbnail generation (for images)
    - custom format access (via format identifier such as UTI types in macOS or custom CF formats in Windows)
