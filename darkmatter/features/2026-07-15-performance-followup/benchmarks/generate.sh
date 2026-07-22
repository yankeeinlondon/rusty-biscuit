#!/usr/bin/env bash
# Deterministic fixture generator for the 2026-07-15 performance follow-up.
#
# Every fixture consumed by a measured checkpoint is produced here so the exact
# bytes are reproducible: no dates, no randomness, LF line endings, stable
# ordering. Re-running this script MUST reproduce the committed fixtures
# byte-for-byte; `manifest.yaml` records the resulting size/hash identities and
# `benchmark_fixtures.rs` proves the recorded identities still match.
#
# Generator version: bump on any change to emitted bytes.
#   version: 1.3.0
#   command: bash generate.sh
#
# Usage:
#   bash generate.sh            # (re)write fixtures/ under this directory
set -euo pipefail

GENERATOR_VERSION="1.3.0"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/fixtures"
mkdir -p "$out"

# LF-only writer: prints its stdin verbatim to $1.
write() { cat >"$out/$1"; }

# --- Authored fixtures (literal bytes; quoted heredocs, no expansion) --------

# render_basic.md — prose + heading hierarchy + list + one code block.
write render_basic.md <<'EOF'
# Rendering Basics

A short paragraph of prose that exercises ordinary inline rendering, including
*emphasis*, **strong emphasis**, and `inline code`.

## A List

- First item
- Second item with a [local link](./other.md)
- Third item

## A Code Block

```rust
fn main() {
    println!("hello, darkmatter");
}
```

Closing prose after the code block.
EOF

# hash_basic.md — frontmatter + body, for the `md hash` case.
write hash_basic.md <<'EOF'
---
title: Hash Fixture
tags:
  - alpha
  - beta
version: 1
---

# Hash Fixture Body

Stable body content whose Markdown-aware frontmatter/body hash identity is
recorded in the manifest and re-verified by the fixture test.

## Section Two

More prose so the body hash covers multiple blocks.
EOF

# compose_trivial.md — trivial compose: frontmatter vars + body interpolation.
write compose_trivial.md <<'EOF'
---
name: Darkmatter
role: renderer
greeting: "Hello from {{ name }}"
---

# {{ greeting }}

The {{ name }} project acts as a {{ role }}. No transclusion, schema, or shell
directives are present, so this is the trivial-compose control case.
EOF

# compose_child.md — transclusion target for compose_schema_transclusion.md.
write compose_child.md <<'EOF'
## Included Section

This paragraph lives in a child document and is pulled in by a `::file`
transclusion so the schema/transclusion compose case exercises a real cross-file
read.
EOF

# compose_schema_transclusion.md — inline $schema + a `::file` transclusion.
write compose_schema_transclusion.md <<'EOF'
---
$schema:
  title: string
  status: enum(draft, ready)
title: Schema and Transclusion
status: draft
---

# {{ title }}

An inline `$schema` validates and coerces this frontmatter, then a transclusion
directive pulls in a child document:

::file ./compose_child.md

Closing prose after the transclusion.
EOF

# compose_transclusion_heavy.md — many `::file` directives resolved in one
# transclusion phase. Every directive targets the same child, so the phase
# builds N per-directive compose cache keys while single-flight composes the
# child once. Exercises Finding 35.1 (the effective-state/context hash is
# hoisted to once-per-phase instead of recomputed per directive). Added in
# perf-followup Phase 5.
{
  printf '# Transclusion Heavy\n\nIntro prose before the transclusions.\n'
  i=0
  while [ "$i" -lt 40 ]; do
    printf '\n::file ./compose_child.md\n'
    i=$((i + 1))
  done
  printf '\nClosing prose after the transclusions.\n'
} | write compose_transclusion_heavy.md

# render_code_heavy.md — many fenced code blocks across languages.
{
  printf '# Code-Heavy Render\n\nIntro prose before the code blocks.\n'
  langs=(rust python json yaml toml bash typescript go)
  i=0
  while [ "$i" -lt 40 ]; do
    lang="${langs[$((i % ${#langs[@]}))]}"
    printf '\n## Block %02d (%s)\n\n' "$i" "$lang"
    printf '```%s\n' "$lang"
    printf 'let block_%02d = "sample line one";\n' "$i"
    printf 'let block_%02d = "sample line two";\n' "$i"
    printf '```\n'
    i=$((i + 1))
  done
} | write render_code_heavy.md

# compose_interpolation_heavy.md — a clean-composing document exercising the
# Phase-6 frontmatter + body interpolation paths (Findings 11, 12, 14): a wide
# frontmatter dependency graph (30 keys fanning out from `base`/`proj`), a deep
# dependency chain (15 links), nested body interpolation, a `{{{ ... }}}`
# literal escape, multiline-indented interpolation, Unicode prose, a fenced
# code block whose `{{ ... }}` is skipped (MarkdownAware), and a `replace:` map.
# The pathological interpolation cases the plan also enumerates — dependency
# cycles, shell-pending `$(...)` keys, and best-effort per-key errors — are
# exercised by the frontmatter_interpolation.rs unit suite rather than a
# clean-composing committed fixture (they resolve to warnings/raw text, not a
# byte-stable composed artifact). Added in perf-followup Phase 6.
{
  printf -- '---\n'
  printf 'base: /root/project\n'
  printf 'proj: Darkmatter\n'
  i=1
  while [ "$i" -le 30 ]; do
    printf 'wide_%02d: "{{ base }}/section-%02d/{{ proj }}"\n' "$i" "$i"
    i=$((i + 1))
  done
  printf 'chain_00: "{{ base }}"\n'
  i=1
  while [ "$i" -le 15 ]; do
    prev=$((i - 1))
    printf 'chain_%02d: "{{ chain_%02d }}/level-%02d"\n' "$i" "$prev" "$i"
    i=$((i + 1))
  done
  printf 'title: "{{ proj }} Interpolation Fixture"\n'
  printf 'replace:\n'
  printf '  ACME: Darkmatter\n'
  printf '  PLACEHOLDER_VERSION: 1.0.0\n'
  printf '  TBD: resolved\n'
  printf -- '---\n'
  printf '\n# {{ title }}\n\n'
  printf 'Project {{ proj }} rooted at {{ base }}.\n\n'
  printf "Nested: {{ proj ? 'inside {{proj}} now' : 'none' }}\n\n"
  printf 'Literal escape stays raw: {{{ not_interpolated }}}\n\n'
  printf '  indented: {{ chain_15 }}\n\n'
  printf 'Unicode prose: café {{ proj }} 日本語 🎉\n\n'
  printf '```rust\n'
  printf '// {{ not_touched }} stays literal inside a fence\n'
  printf 'let x = 1;\n'
  printf '```\n\n'
  printf 'ACME ships PLACEHOLDER_VERSION; status TBD.\n\n'
  i=1
  while [ "$i" -le 30 ]; do
    printf -- '- {{ wide_%02d }}\n' "$i"
    i=$((i + 1))
  done
} | write compose_interpolation_heavy.md

# replace_heavy.md — a `replace:` map with 40 same-length rules plus three
# overlapping-prefix rules (`TOKEN`, `TOKEN_01`, `TOKEN_01_EXTRA`) so the
# longest-match-at-a-position precedence is exercised, over a body with ~1600
# replacement occurrences. Drives the Finding 13 multi-pattern-matcher
# comparison. Added in perf-followup Phase 6.
{
  printf -- '---\n'
  printf 'title: Replacement Heavy\n'
  printf 'replace:\n'
  i=1
  while [ "$i" -le 40 ]; do
    printf '  TOKEN_%02d: value-%02d\n' "$i" "$i"
    i=$((i + 1))
  done
  printf '  TOKEN: short\n'
  printf '  TOKEN_01_EXTRA: longer\n'
  printf -- '---\n'
  printf '\n# Replacement Heavy\n\n'
  j=1
  while [ "$j" -le 20 ]; do
    i=1
    while [ "$i" -le 40 ]; do
      printf 'Line refers to TOKEN_%02d and café TOKEN_%02d again.\n' "$i" "$i"
      i=$((i + 1))
    done
    j=$((j + 1))
  done
} | write replace_heavy.md

# remote_heavy.md — 300 read-side remote expressions spread through a large
# body, so each successive `{{ frontmatter("https://…") }}` starts further into
# the document than the last. Drives the Finding 33 comparison: the per-
# expression `content[..start]` prefix rescan is quadratic in that growing
# offset, while one newline-offset table is linear. The `café` padding keeps
# multi-byte characters ahead of the expressions so byte offsets diverge from
# character offsets. Added in perf-followup Phase 9.
{
  printf -- '---\n'
  printf 'title: Remote Heavy\n'
  printf -- '---\n'
  printf '\n# Remote Heavy\n'
  i=1
  while [ "$i" -le 300 ]; do
    printf '\n## Section %d\n\n' "$i"
    printf 'Prose for section %d with *emphasis*, `inline code`, and a café of\n' "$i"
    printf 'ordinary words padding the line offsets so that each expression\n'
    printf 'starts further into the document than the one before it did.\n\n'
    printf '{{ frontmatter("https://example.com/api/%03d.md") }}\n' "$i"
    i=$((i + 1))
  done
} | write remote_heavy.md

# --- Procedural fixtures: three TOC size tiers ------------------------------
#
# Each section emits an `## Heading N`, a paragraph containing a mid-line
# internal link, and (every 10th section) a fenced code block. Mid-line link
# offsets and code-block boundaries at scale exercise the non-quadratic
# `line_at_offset` path guarded by the fixture test.
emit_toc() {
  local sections="$1"
  printf '# Table of Contents Tier\n\nRoot prose before the sections.\n'
  local n=1
  while [ "$n" -le "$sections" ]; do
    printf '\n## Heading %d\n\nProse for section %d, see [heading one](#heading-1) above.\n' "$n" "$n"
    if [ $((n % 10)) -eq 0 ]; then
      printf '\n```text\ncode sample for section %d\n```\n' "$n"
    fi
    n=$((n + 1))
  done
}

emit_toc 12   | write toc_small.md
emit_toc 120  | write toc_medium.md
emit_toc 1000 | write toc_large.md

echo "generator $GENERATOR_VERSION wrote $(find "$out" -type f | wc -l | tr -d ' ') fixtures to $out"
