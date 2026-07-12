---
name: two-face
description: Expert knowledge for embedding bat-curated syntax definitions and themes into syntect-based Rust syntax highlighting. Use when adding highlighting for modern languages (TOML/TS/Dockerfile/etc.), generating HTML/ANSI output, or troubleshooting syntect regex-feature mismatches.
tools: [Read, Write, Edit, Grep, Glob, Bash]
---

# two-face

## What this skill is for
Use this skill when you need **syntect syntax highlighting with better language/theme coverage** via **two-face** (bat’s curated embedded syntaxes + themes). This skill helps you:
- Add two-face correctly (including **feature flags** that must match syntect)
- Initialize `SyntaxSet` / `ThemeSet`
- Highlight to **HTML** or **ANSI terminal**
- Pick syntaxes/themes safely and ergonomically
- Avoid common pitfalls (regex backend mismatch, newline mode, fancy-regex gaps, binary size concerns)

## Quick decision checklist (invoke this skill when…)
- You’re using `syntect` and missing syntaxes like **TOML, TypeScript, Dockerfile**
- You want **embedded themes** (Nord/Dracula/Monokai/Solarized, etc.) without shipping asset files
- You’re building a **CLI / SSG / web service / editor** that needs consistent highlighting
- You hit compile errors around **oniguruma vs fancy-regex** features

## Core entry points (memorize)
- `two_face::syntax::extra_newlines()` → `SyntaxSet` (recommended default)
- `two_face::syntax::extra_no_newlines()` → `SyntaxSet` (line-by-line / special cases)
- `two_face::theme::extra()` → `ThemeSet`
- `two_face::theme::EmbeddedThemeName` → enum for selecting embedded themes
- `two_face::acknowledgement::listing()` → bundled asset attributions/licenses
- `two_face::re_exports::syntect` → convenience re-export

## Key docs (open these first)
- [Setup & feature flags (critical)](setup-and-features.md)
- [Common recipes (HTML, ANSI, detection)](recipes.md)
- [Gotchas & troubleshooting](gotchas-troubleshooting.md)
- [Theme & syntax discovery + curation](themes-and-syntaxes.md)
- [Use cases + alternatives](use-cases-and-alternatives.md)

## Standard implementation pattern
1. Load sets:
   - `let syn_set = two_face::syntax::extra_newlines();`
   - `let theme_set = two_face::theme::extra();`
2. Pick syntax:
   - by extension: `find_syntax_by_extension("toml")`
   - optionally fallback by first line / plain text
3. Pick theme:
   - `&theme_set[two_face::theme::EmbeddedThemeName::Nord]`
4. Render:
   - HTML: `syntect::html::highlighted_html_for_string`
   - ANSI: `syntect::easy::HighlightLines` + `as_24_bit_terminal_escaped`

## Output expectations
When generating code, prefer:
- Small helper functions: `highlight_html(code, ext, theme)`
- Graceful fallback if syntax/theme isn’t found
- A curated list of supported themes (don’t expose everything by default)

## Guardrails (don’t skip)
- Ensure **syntect regex backend** matches two-face feature flags (see setup doc).
- Default to `extra_newlines()` unless you have a reason.
- If using `fancy-regex`, expect some syntaxes to be missing/excluded.