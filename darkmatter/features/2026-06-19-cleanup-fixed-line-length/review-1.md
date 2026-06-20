---
ready: false
agent: codex
model: ""
created: 2026-06-20T00:42:34
---

# Review 1 - Cleanup Fixed Line Length

## Findings

### High - Structurally significant newlines are still destroyed

The implementation still collapses CommonMark hard breaks and `===` setext headings, even though the reconciled spec marks both as required fixes. In `darkmatter/lib/src/markdown/cleanup.rs`, collapsed boundaries only choose whether to insert a space by checking `line.ends_with(char::is_whitespace)` (`strip_incidental_newlines`, lines 409-416). That deletes the newline after a two-space hard break, and a trailing backslash hard break is merged with the next line. The structural classifier also protects `---`, `***`, `___`, `#`, tables, and directives, but not `===` (`is_structural_line`, lines 619-625), so:

```markdown
Heading
===
```

is treated as ordinary prose and becomes `Heading ===`.

Required verification level: Level 1 is appropriate because this is byte-for-byte Markdown transformation, not terminal rendering. Current strongest verification: Level 1 coverage exists for ordinary trailing whitespace collapse (`strip_incidental_newlines_drops_after_trailing_whitespace`, lines 2018-2023), but there are no fail-first tests for hard breaks or `===` setext headings. Add L1 tests for two trailing spaces, trailing backslash, `===`, and `---` setext heading preservation.

### High - Unicode-script join rules are not implemented

Decision 2 requires separator selection by Unicode Script, including no separator for Han/Hiragana/Katakana/Bopomofo/Thai/Lao/Khmer/Myanmar/Tibetan letter boundaries, no separator at spaceless-script transitions such as Han-to-Latin, no separator around ZWSP, and Hangul remaining space-delimited. The implementation still uses the earlier literal-whitespace rule only (`strip_incidental_newlines`, lines 409-416), and there is no `unicode-script` dependency or script lookup in the library. This means inputs such as `漢\n字`, Thai text, Han-to-Latin boundaries, and ZWSP boundaries are currently rewritten incorrectly.

Required verification level: Level 1 is appropriate. Current strongest verification: absent. The cleanup tests cover ASCII whitespace joins and Unicode display width for reflow, but not Unicode-script join decisions. Add L1 table tests for Han-Han, Thai-Thai, Hangul-Hangul, Han-Latin, emoji-emoji, CJK punctuation, and ZWSP boundaries.

### High - Fixed-width compose can reflow source wrapping instead of canonical unwrapped prose

The spec requires fixed-width reflow to strip incidental newlines first, so the requested width is applied to canonical paragraph/block form rather than to the input's existing wrapping. The CLI prevents the contradictory flag pair, but the public library API does not. `ComposeOptions` exposes both `with_incidental_newline_mode(...)` and `with_fixed_width(...)` independently (`darkmatter/lib/src/markdown/compose/context/options.rs`, lines 500-514). In the compose cleanup phase, stripping is conditional on `IncidentalNewlineMode::Strip`, while reflow runs afterward whenever `fixed_width` is set (`darkmatter/lib/src/markdown/compose/pipeline/phases.rs`, lines 79-105). A caller can therefore use:

```rust
ComposeOptions::new()
    .with_incidental_newline_mode(IncidentalNewlineMode::Preserve)
    .with_fixed_width(80)
```

and get fixed-width reflow over the original source wrapping, violating the contract. The existing test `test_compose_cleanup_preserve_and_fixed_width_is_noop_when_prose_fits` codifies the contradictory combination instead of rejecting or overriding it.

Required verification level: Level 1 is appropriate. Current strongest verification: Level 1, but it verifies the wrong behavior for the conflicting API combination. Either make `with_fixed_width` force strip semantics, make the compose phase strip when `fixed_width` is set regardless of incidental mode, or reject the conflicting library configuration before compose runs. Add a Level 1 compose regression where source-wrapped prose is first unwrapped and then reflowed.

### Medium - The v1 atomic spaceless-run fixed-width contract is untested and currently dependent on whitespace tokenization

The required criteria say CJK and other spaceless runs under `--fixed-width` must be treated as a single atomic token and allowed to overflow rather than being split between ideographs. The current tokenizer only splits on `char::is_whitespace()` (`reflow_tokens`, lines 922-949), so a contiguous CJK run is probably atomic today, but this is accidental relative to the new Unicode-script contract and has no focused regression. Once Unicode-script join handling is added, this needs to be pinned so future "better wrapping" work does not accidentally introduce intra-run breaks in v1.

Required verification level: Level 1 is appropriate. Current strongest verification: absent for CJK/spaceless fixed-width overflow. Add L1 tests using a wide CJK run whose `UnicodeWidthStr::width` exceeds the requested width and assert it remains on one line.

## Verification Level Matrix

| Requirement | Appropriate level | Current strongest level | Status |
|---|---:|---:|---|
| Default `md clean` strips incidental single newlines | L1 | L1 CLI + library | Adequate for shipped ASCII behavior |
| Preserve blank lines, code, tables, HTML, blockquotes, list markers, directives | L1 | L1 library | Broadly covered, except hard breaks and setext gaps above |
| `--fixed-width` wraps by display columns and preserves protected blocks | L1 | L1 CLI + library | Adequate for ASCII/non-ASCII width basics |
| `--ignore-incidental-newlines` preserves source single newlines | L1 | L1 CLI + compose | Adequate for CLI, but conflicts with fixed-width through library API |
| CLI conflict between `--fixed-width` and `--ignore-incidental-newlines` | L1 | L1 clap/CLI | Adequate |
| Hard line breaks and setext headings preserved | L1 | Missing | Gap |
| Unicode-script separator selection and ZWSP behavior | L1 | Missing | Gap |
| Spaceless fixed-width runs stay atomic and may overflow | L1 | Missing | Gap |

No Level 2 or Level 3 tests are required for this feature as specified: the behavior under review is Markdown text transformation and CLI argument handling, not terminal emulator rendering, terminal input encoding, keyboard behavior, mouse/paste/IME handling, or SGR/glyph rendering.

## Test Run

Attempted `just test` from `darkmatter/`. The run was interrupted at about 69 seconds under the non-interactive time budget; nextest had completed 2203 of 4632 tests, all passed so far, with 110 skipped and 2429 not run due to interrupt. No Level 2 or Level 3 run was attempted because this feature does not require real-terminal or OS-keyboard verification.

## Recommendation

Not ready for production. Fix the three high-severity findings first, then add the missing Level 1 regressions listed above. The implementation is close for the originally shipped ASCII workflow, but the reconciled spec now requires structural-safety and Unicode-script behavior that is not present.
