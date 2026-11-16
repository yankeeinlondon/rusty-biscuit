---
kind: plan
created: 2026-11-14
total_phases: 6
phase: 2
agent: opencode/zai-coding-plan/glm-5.3
yolo: 'true'
source_files_during_phase_1:
- darkmatter/lib/src/markdown/compose/directives_api.rs
- darkmatter/lib/src/markdown/compose/parse_utils.rs
- darkmatter/lib/src/markdown/compose/transclusion/types.rs
- darkmatter/lib/src/markdown/compose/transclusion/parser.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/preflight/mod.rs
- darkmatter/lib/src/markdown/compose/tests/transclusion.rs
- darkmatter/dmls/src/overlay/directives.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/darkmatter/compose.md
packages:
- darkmatter
- darkmatter-lsp
---

# Plan: `::file interpolate=false` — transclude prose without evaluating it

## Summary and Definition of Done

### The work

Add an `interpolate` option to the `::file` and `::url` directives
(`interpolate=false` disables prose interpolation for the transcluded child),
plumb the resulting effective setting through every path that examines a
child, and keep executable directive inputs evaluating. The work spans:

- **Grammar** (`darkmatter/lib/src/markdown/compose/transclusion/parser.rs`,
  `types.rs`): parse and validate the option with the spec's value table
  (boolean words via SimplifiedSchema's shared conversion, unquoted `null` as
  the default, quoted values keep quoting visible so `"null"` is text),
  last-valid-wins with every occurrence validated, located directive errors,
  and `::code` excluded (unknown option there, unchanged).
- **Options model** (`compose/context/options.rs`): a new
  `ProseInterpolation` setting on `ComposeOptions`, classified into both
  identity products (compose-cache fingerprint and reference-graph identity)
  by the exhaustive `classify_options` destructure, plus the option in
  preflight directive parameter identity and in the composed-child cache key.
- **Execution semantics** (`compose/inline/interpolation.rs`,
  `compose/transclusion/engine.rs`, a new shared active-input classifier):
  suppress only the child's **prose** evaluation — directive targets, option
  values, `when=` conditions, collection gates/callbacks, and `::shell` /
  `::shell-block` command inputs keep evaluating; `{{{ … }}}` prose literals
  keep all three braces; `interpolate_code_blocks: true` on the child cannot
  re-enable code interpolation.
- **Inheritance**: once disabled, every Markdown descendant of the inclusion
  (nested `::file`/`::url`, prologue/epilogue includes, nested composition
  from an active expression) stays disabled; a nested `interpolate=true` is
  accepted silently; the setting never escapes to the parent or siblings.
- **Passive surfaces** (`call_precheck.rs`, `context/capture/groups.rs`,
  `context/capture/capabilities.rs`, `remote.rs`, `preflight/collect.rs`,
  `markdown/reference/graph.rs`): the same prose/active classification gates
  the call precheck, context-demand discovery, deferred-capability (ICMP)
  planning, remote-expression prefetch discovery, the preflight approval walk,
  and reference-graph preparation — suppressed prose causes no read, fetch,
  context demand, probe, or sample-call rejection under `--deny all`.
- **Editor** (`darkmatter/dmls`): `interpolate=` completions with
  `true`/`false` values, hover on the option explaining defaulting and
  inheritance (with the author-ruled sentence), a diagnostic for malformed
  values at the option, `::code` not offering it, and an open included file
  keeping its normal expression treatment.
- **Docs and skills**: `block-transclusion.md`, `inline/interpolation.md`,
  the `darkmatter` repo skill, the `use-darkmatter` caller skill, the
  `use-dmls` feature list, `dmls/docs/diagnostics.md` (new diagnostic code),
  and the `CLAUDE.md` Skill Audiences rule (replacing the "planned"
  workaround with `interpolate=false` guidance).
- **Verification**: L1 tests across the lib, CLI, and DMLS through the
  package-area `just` recipes; a frozen caller-skill fixture composed with
  `md compose --frontmatter --deny all` whose own `name` collides with a docs
  example; a passive shipped-artifact check. No real terminal or browser.

### Definition of done

- Every row of the spec's Verification table has an L1 test that passes, and
  the default (option absent) output is byte-identical to today's behavior
  for the existing test corpus (`just test` in `darkmatter/` passes with no
  pre-existing test modified except where the feature intentionally changes
  behavior — none is expected outside new tests).
- Invalid option values produce located directive errors from the library,
  the CLI, and a DMLS diagnostic, and composition blocks (nothing composed,
  nothing executed) per the existing blocking-policy behavior.
- `md compose --frontmatter --deny all` over a fixture that reproduces the
  caller-skill name-collision publishes `{{ name }}` and `{{{ x }}}`
  verbatim when the inclusion carries `interpolate=false`, and still fails
  publication for an undeclared name without the option.
- Two uses of one file — one normal, one `interpolate=false`, in both
  directive orders, with the compose cache enabled and disabled — produce
  independent, correct outputs and warnings.
- `just publish-skills` in `darkmatter/` succeeds under `--deny all` and the
  published `use-darkmatter` text describes the option.
- `just test` and `just lint` pass for the darkmatter package area; the spec
  frontmatter records `implemented: true`.

### Non-goals (from the spec)

- No frontmatter property, no CLI flag, no `::code` support, no new file
  types or remote permissions, no frontmatter opt-out for the child, no
  inclusion-specific "view as included by…" editor mode, no per-file
  persistent marking (a saved output recomposes under normal rules), no
  sentinel rewriting or rescanning of assembled output.
- No performance spike or benchmark expansion (the spec waives it).

### Spike decision

No spike. Every open question was answered by reading code: the value reader
drops quotes (so `parse_set_option`'s `was_quoted` capture pattern is the
model), the shared boolean converter already exists
(`simplified_schema::numeric::parse_boolean_word`), the body rewrite already
supports span selection (`SpanSelection` in `interpolation/rewrite.rs`), the
child options derivation is single-point
(`inherited_child_options`), and the masking technique for passive scanners
already exists (`scan_executable_body_groups` in
`context/capture/groups.rs`). Nothing needs a runtime measurement.

---

## Phase 1: Grammar, Types, and Identity

Foundation only: parse the option, model the setting, and make every identity
product aware of it. No behavior change ships from this phase beyond the
parser accepting the option (and `::file x.md interpolate=false` composing
exactly as `::file x.md` does until Phase 2 lands).

### Necessary Rules

1. **Value grammar — one shared validator.** A function
   `parse_interpolate_value(decoded: &str, was_quoted: bool) -> Result<Option<bool>, String>`
   lives in `darkmatter/lib/src/markdown/compose/directives_api.rs` (public,
   because DMLS reuses it) and implements exactly: unquoted `null` →
   `Ok(None)` (the null value, meaning default `true`); `decoded` empty →
   error; otherwise `simplified_schema::numeric::parse_boolean_word(decoded)`
   (`true/false/yes/no/on/off/1/0`, case-insensitive, trimmed) →
   `Ok(Some(b))`; anything else (including quoted `"null"`, objects, arrays,
   arbitrary text) → error. Evidence: spec "Option values and placement"
   table; `parse_boolean_word` at `simplified-schema/lib/src/numeric.rs:179`;
   the `was_quoted` capture pattern at
   `darkmatter/lib/src/markdown/compose/transclusion/parser.rs:265`
   (`parse_set_option`), needed because `Cursor::read_value` strips quotes
   (`parse_utils.rs:234-248`).
2. **`BlockOptions.interpolate: bool`, default `true`.** The parsed directive
   stores the resolved local value (absent/null/`true` all become `true`),
   so `::file a.md` and `::file a.md interpolate=true` compare equal and
   share caches. The derived `Default` for `BlockOptions`
   (`transclusion/types.rs:70`) is replaced with a hand-written one that
   keeps today's values and sets `interpolate: true` — required because the
   prologue/epilogue path builds `BlockOptions::default()` (engine.rs:984)
   and must inherit, not disable. Last-valid-wins: each valid occurrence
   assigns; an earlier invalid occurrence errors before a later one is read.
3. **Parser interception for `::file`/`::url` only.** In
   `parse_directive_line` (parser.rs:170), when the key read is
   `interpolate` and the directive kind is `File` or `Url`, consume `=`,
   capture `was_quoted` (current char is `'` or `"`), `read_value`, and apply
   the validator; `Ok(None)` or `Ok(Some(true))` sets
   `options.interpolate = true`, `Ok(Some(false))` sets `false`; `Err(reason)`
   returns `TransclusionError::ParseDirective { ctx, line, message, caret_col: Some(column) }`
   — the same located-error shape as `set`/`replace` (parser.rs:270-280,
   403-416). For `DirectiveKind::Code`, `interpolate` is **not** intercepted
   and falls through to `apply_option`'s unknown-option arm (parser.rs:444),
   preserving today's warning at engine.rs:579. Evidence: spec
   "`::file` and `::url` accept the option; `::code` does not".
4. **`ProseInterpolation` on `ComposeOptions`.** New crate-private enum in
   `compose/context/options.rs`: `Enabled` (default) and
   `DisabledByInclusion`, field `pub(crate) prose_interpolation` with
   builder `pub(crate) fn with_prose_interpolation`, mirroring
   `interpolate_code_blocks`
   (field at options.rs:451, builder at options.rs:1514). Only the fold in
   Rules 5-6 and tests ever set `DisabledByInclusion`; root requests and
   public callers always see `Enabled`. Evidence: spec "Inheritance is
   confined to the inclusion branch" and "no frontmatter opt-out".
5. **Identity classification in both products.** Add the field to
   `classify_options`'s exhaustive destructure (options.rs:3174 forces the
   decision at compile time): encode it in the graph encoder beside
   `interpolate_code_blocks` (options.rs:3582) **and** in the cache encoder's
   output-affecting subset. Evidence: spec "Include the effective setting in
   composed-child cache identity and reference-graph preparation identity";
   `options_hash` delegates to `compose_cache_fingerprint`
   (`cache/hashing.rs:175`).
6. **Single fold point.** `effective_prose_interpolation(parent: ProseInterpolation, directive: &BlockOptions) -> ProseInterpolation`
   (in `transclusion`, pub(crate), unit-tested in Phase 1): `DisabledByInclusion`
   iff the parent is disabled or `directive.interpolate == false`. Applied in
   `inherited_child_options` (engine.rs:1899 — the documented single
   derivation shared by execution, the preflight walk, and nested
   composition) so local-file children, the preflight walk
   (`preflight/collect.rs:987,1123,1189`), and `as_markdown` children
   (`nested.rs:259`) all inherit it. Two paths bypass that function and fold
   manually: the remote `PreparedTransclusion::RemoteFile` child
   (engine.rs:1263, `options.clone().derive(...)` gains
   `.with_prose_interpolation(...)`) and the reference-graph child
   (Phase 3). Prologue/epilogue children need no change:
   `BlockOptions::default()` (Rule 2) plus the parent-field clone yields
   inheritance. Evidence: engine.rs:1898-1929 doc comment; spec "Inheritance
   is confined to the inclusion branch".
7. **Directive parameter identity.** `parameters_identity`
   (`preflight/mod.rs:104`) folds `options.interpolate` in with the set
   overlay and origin so a repeated site with a different effective option is
   a different child (checked at engine.rs:507). The composed-child cache key
   in `render_markdown_transclusion` (engine.rs:1633) gains one final
   component for the child's folded setting (`:0` enabled / `:1` disabled) —
   the key already hashes parent options, so without this, a cached
   `interpolate=false` child could be reused for a normal inclusion.
   Evidence: spec "Two uses of one file must not share prepared content,
   warnings, or composed results across different effective settings. Raw
   file and HTTP-body caches can still be shared" — only the `compose:` key
   and parameter identity change.
8. **DMLS static surface.** `option_keys`
   (`dmls/src/overlay/directives.rs:177`): the
   `File | Code | Url` arm splits — `File | Url` gain `"interpolate"`,
   `Code` keeps today's six keys, so the editor warns on `::code
   interpolate=…` exactly as the library does. `option_values` gains
   `"interpolate" => Some(&["true", "false"])`. Evidence: spec "Editor
   behavior" and "its existing handling of unknown options is unchanged".
9. **No other grammar surface changes.** No frontmatter key, no schema
   change, no CLI flag, no `::toc-linking`/`::file-links` change (they have
   their own parsers); `directives_api::ParsedDirective` needs no shape
   change (option key/value spans already exposed, directives_api.rs:88-100).

### Input robustness matrix

The `interpolate` option value is the one load-bearing field this feature
adds to the directive grammar (a text-token config reader). Default outcome,
stated once: **every invalid value shape produces a located directive error
naming the including file and the authored option position, blocks the
composition, and runs nothing** — through the existing
`TransclusionError::ParseDirective` + blocking-policy path. Exceptions and
the full field-shape walk, tested once against the shared validator through
the parser's public result (`parse_directives` → `BlockOptions` or
`TransclusionError`):

| Shape (spelling on the directive) | Required outcome |
|---|---|
| absent (option omitted) | local `true`; today's behavior, byte-identical |
| `interpolate=null` (unquoted) | local `true` (the null value; per `2026-11-14-absent-equals-null`, absent and null mean the default) |
| `true`, `yes`, `on`, `1` — any case, trimmed | local `true` |
| `false`, `no`, `off`, `0` — any case, trimmed | local `false` |
| quoted boolean word (`"false"`, `'No'`) | same converted boolean (quoting is not semantics for words) |
| quoted `"null"` | located error (text, not the null value) |
| empty (`interpolate=""`, or `interpolate=` at end of line) | located error (`read_value` already errors for the EOL form; the validator errors for the quoted-empty form) |
| object or array (`={a:1}`, `=[1]` — `read_balanced_value` text) | located error |
| any other text (`banana`, `True-ish`) | located error |
| repeated `interpolate=` assignments | last valid wins; every occurrence validated (an invalid earlier one errors) |
| on `::code` | unknown option → existing warning, value never validated |

Document shapes are unchanged (no new loader): the directive-line parser is
existing and covered by its own tests. Wiring is proven with control cases
only: one engine compose control (Phase 2), one DMLS diagnostic control
(Phase 4), and the CLI spawns only for the control plus two invalid shapes
asserting the located error in rendered output and that nothing was composed
(Phase 4). Code-smell pass: the field is a plain `bool` (no `Option<T>`
absent/null conflation; the null distinction lives in
`parse_interpolate_value` via `was_quoted`), no `unwrap_or_default()`/
`.ok()`/`filter_map(.. as_str())` on the load-bearing parse.

### Tasks

**Wave 1** (two tasks, independent files, parallelizable):

- [x] Option parsing
    - Add `parse_interpolate_value` to `directives_api.rs` (public) per Rule 1;
      re-export nothing else
    - Add `BlockOptions.interpolate: bool` with the hand-written `Default`
      (Rule 2) in `transclusion/types.rs`
    - Intercept the key in `parse_directive_line` for File/Url with the
      `was_quoted` capture (Rule 3); `::code` untouched
    - Parser unit tests in `transclusion/parser.rs` `mod tests` covering every
      row of the matrix above, asserting the `ctx`/`line`/`caret_col` of one
      representative error
- [x] Options and identity plumbing
    - Add `ProseInterpolation` + field + builder to `compose/context/options.rs`
      (Rule 4); classify in both encoders (Rule 5)
    - Add `effective_prose_interpolation` in `transclusion` with unit tests
      (Rule 6)
    - Fold the option into `parameters_identity` (Rule 7)
    - Add the cache-key component in `render_markdown_transclusion`
      (engine.rs:1633) and the manual fold on the `RemoteFile` child path
      (engine.rs:1263) per Rules 6-7
    - DMLS static catalog split: `option_keys`/`option_values`
      (`dmls/src/overlay/directives.rs:177-195`) per Rule 8, with the
      catalog/allow-list unit tests updated

### Validation checkpoint (Phase 1)

- `cargo nextest run -p darkmatter transclusion::` and the DMLS overlay tests
  pass; `cargo check --workspace` clean (the exhaustive destructure proves
  classification is complete).
- `::file ./a.md interpolate=false` parses, and — until Phase 2 — composes
  byte-identically to `::file ./a.md` (assert in one engine-level test so the
  intermediate state is green).

---

## Phase 2: Execution Semantics

Suppress the child's prose evaluation while every executable input keeps
evaluating, and hold the spec's "no diagnostics, no reads, no fetches from
suppressed prose" line in the executing pipeline. Passive consumers that
share the classification land in this phase too, because they read the same
classifier.

### Necessary Rules (Phase 2)

10. **Shared active-input classifier.** New module
    `darkmatter/lib/src/markdown/compose/interpolation/active_inputs.rs`
    exposing `pub(crate) fn active_input_ranges(content: &str, data: Option<&DataRanges>) -> Vec<std::ops::Range<usize>>`
    and `pub(crate) fn mask_prose(content: &str, data: Option<&DataRanges>) -> String`
    (prose bytes blanked to spaces, offsets preserved — the technique of
    `scan_executable_body_groups`, groups.rs:223-237). Active ranges are,
    for the content as the scanning stage sees it: every
    `scan_darkmatter_directives_in` directive's `option.value` spans and
    `callback` span; the `target` span of `::shell` (the command) and of
    `::file`/`::code`/`::url`/`::map`/`::filter` (targets are executable
    inputs; the dedicated target pass handles whole-value targets first and
    the body pass owns the rest, as today); and every `::shell-block` body
    span from `block_pairs::scan_block_pairs_in(content, data)` filtered to
    Shell kind. Everything else — headings, paragraphs, lists, tables,
    inline code, fenced/indented code, block and collection bodies — is
    prose. Evidence: spec "Which text remains literal" and "Directives still
    execute"; body-pass ownership of option values follows from
    `interpolate_text_selected` scanning authored text
    (interpolation/rewrite.rs:293-347) and `::shell` argument expressions
    being body-evaluated (`docs/inline/interpolation.md:305`).
11. **Body-pass suppression via span selection.** In
    `inline/interpolation::run_stage` (inline/interpolation.rs:51): when
    `options.prose_interpolation == DisabledByInclusion`, the target pass
    (`rewrite_targets_guarded`) runs unchanged; the body pass
    (`rewrite_body`) computes `active_input_ranges` over its own `input` and
    passes a `select` closure that declines spans not intersecting an active
    range **and** composes with the existing `defer` closure for accepted
    spans (accepted spans keep the late-read deferral;
    `inline/interpolation.rs:125-136`). Declined expression spans are never
    parsed, evaluated, or warned about; declined `{{{ … }}}` literals keep
    all three braces (a declined `Found::Literal` is skipped verbatim,
    rewrite.rs:372-377). `resolve_pending` needs no new prose logic: plain
    pending reads can only originate from accepted spans. The
    `interpolate_code_blocks` resolution (inline/interpolation.rs:62, 278) is
    **not** consulted for suppression — suppression wins regardless of
    `ScanMode`. Evidence: spec rulings "triple braces stay triple; the option
    overrides the child's `interpolate_code_blocks: true`" and
    implementation requirement "Merely skipping the entire body interpolation
    stage is insufficient".
12. **Literal tokens respect selection.** The unconditional embedded-token
    failure at rewrite.rs:326 fires before spans; change
    `interpolate_text_selected` so `select`, when present, is consulted for
    tokens (called with `source: None`) and only tokens the select accepts
    fail. The pending-read `chosen` closures (inline/interpolation.rs:291,
    323) are left as-is: a token is never a pending span, and the full pass
    that created the pendings already failed on any authored token, so no
    reachable behavior changes for them. Evidence: spec "Body `{{ … }}` spans
    … produce no expression diagnostics … including malformed expression
    examples" — a literal token in suppressed prose is interpolation syntax,
    not a pending read.
13. **Passive consumers mask, they don't evaluate.**
    - `call_precheck` body walk (call_precheck.rs:753-778): when the request's
      prose interpolation is disabled, filter `scan.expressions` to spans
      intersecting `active_input_ranges` over the same body text (the walk
      reads expressions only — its module table lists literal tokens under
      "Not checked"). Frontmatter, `when=` conditions, and collection
      callbacks are stage-owned inputs and stay checked.
    - `ContextRequirements` gains
      `for_document_with(document: &Markdown, prose_enabled: bool)`
      (groups.rs:128); `for_document` delegates with `true`. Disabled prose
      scans `mask_prose(document.content(), None)` instead of the raw body.
      Call sites with options in scope switch to the new variant:
      `extend_context_for`/`needs_extension_for` (options.rs:861-883),
      the remote-child context (engine.rs:1259-1261), and `nested.rs:295`.
      `ContextCapture::capture_for_document` (context/runtime.rs:214) is
      root-only context seeding with no request in scope and a root that is
      always `Enabled` (Rule 4), so it keeps calling `for_document`.
    - `DeferredCapabilities::for_document` (capabilities.rs:53) gains the
      same prose flag (masked body) — its `current.*`/function scan is the
      ICMP probe planner consumed by the preflight walk
      (preflight/collect.rs:455).
    - `remote::discover_remote_urls_from_expressions` call sites gate and
      mask on the effective setting: the root path
      (pipeline/mod.rs:127-133 — the root is always `Enabled`, but
      `prepare_content` nodes reach here with a disabled setting through
      `run_root_pipeline_collecting`) and the fetched-remote-child
      registration (engine.rs:1239-1246, gated on the folded child setting).
      Directive-URL discovery (engine.rs:1225-1237) is untouched.
    Evidence: spec "Share the classification of inactive prose and active
    inputs between execution, preflight walks … and passive reference-graph
    preparation. Discovery must approve exactly the command bytes execution
    will use; it must not execute a sample expression …" and "Suppressed
    prose must not cause prefetches or rejection of sample calls under
    `--deny all`".
14. **Frontmatter untouched.** The frontmatter stages
    (`prepare_document`, pipeline/mod.rs:406-797) never read the setting:
    the child's frontmatter interpolation, schema, coercion, and shell
    expansion keep today's behavior, and `set=` overlays still apply
    (engine.rs:1592). Evidence: spec "The child's frontmatter still follows
    its normal interpolation … The option introduces no frontmatter
    opt-out".
15. **Transformations untouched.** Text replacement, section exclusion,
    link resolution/normalization, heading re-leveling, quotation/disclosure
    wrapping, cleanup, and indentation apply as today, including to
    preserved expression syntax. Evidence: spec Reader's note "'as written'
    concerns interpolation syntax, not identical file bytes".
16. **Origin protection untouched.** Produced text stays data (existing
    `DataRanges`/`BodyOrigin` machinery); no sentinel rewriting, no rescan
    of assembled output. The classifier reads authored text only through the
    same `data` masking the scanners use
    (`scan_darkmatter_directives_in`, directives_api.rs:187). Evidence: spec
    "Use the existing origin tracking … do not rewrite examples into sentinel
    text or rescan the assembled parent output".
17. **Scanner-count and work-counter guards.** The classifier performs its
    own directive scan (and code-region scan) per suppressed body pass —
    one extra scan over the body, taken only when suppression is active.
    Tests that assert scanner parse counts (`parse_utils.rs`
    `scanner_parse_count`) may need deliberate updates — update them in the
    same change and say so in the test. Evidence: spec
    "Reuse existing caching and work-count checks where applicable".

### Tasks

**Wave 2** (two tasks, parallelizable — different modules):

- [ ] Active-input classifier
    - Create `interpolation/active_inputs.rs` with `active_input_ranges` and
      `mask_prose` per Rule 10; unit tests over: option values (incl. `set.x=`,
      `when=`, `exclude=`), `::map`/`::filter` callback and source, `::shell`
      command, `::shell-block` body, `::file` target, prose/inline
      code/fenced code (not active), directives inside repeated bodies, data
      regions excluded via the `data` mask
- [ ] Child-setting derivation
    - Apply the fold in `inherited_child_options` (Rule 6) and verify the
      `RemoteFile` manual fold from Phase 1 composes the fetched body under
      the folded setting; prologue/epilogue inheritance test (a disabled
      parent's prologue child keeps its prose literal)

**Wave 3** (depends on Wave 2):

- [ ] Prose suppression
    - `run_stage` body-pass selection per Rule 11; token selection per
      Rule 12; `interpolate_code_blocks` override; `resolve_pending` audit
    - Engine-level behavior tests in a new
      `lib/src/markdown/compose/tests/file_interpolate.rs` covering the spec
      table rows: literal prose with known and unknown names (and no
      warnings under a `--deny all`-equivalent blocking policy), malformed
      expression examples, triple braces kept, code blocks literal despite
      `interpolate_code_blocks: true` (extend the case at
      `compose/tests/rendering.rs:1000-1012`), child frontmatter still
      interpolates plus `set=` override precedence, `when=` conditions and
      collection gates/callbacks execute while selected/repeated prose
      stays literal, nested `::file {{ target }}` and option-value
      expressions evaluate, shell inputs evaluate, replacement/exclusion/
      wrappers still transform the literal prose, parent and sibling prose
      keeps interpolating normally
    - Inheritance tests: nested omitted/`false`/`true`/`null` under a
      disabled ancestor (descendant prose literal; explicit `true` adds no
      warning even under `--deny all`)

**Wave 4** (depends on Wave 2; may run concurrently with Wave 3 — its files
(`call_precheck.rs`, `context/capture/*`, `remote.rs`, `pipeline/mod.rs`)
are disjoint from Wave 3's `inline/interpolation.rs` and the new test
file):

- [ ] Passive-consumer gating
    - call_precheck filtering, `ContextRequirements::for_document_with` and
      call-site switches, `DeferredCapabilities` prose flag,
      remote-expression discovery gating/masking, per Rule 13
    - Tests: a read-side function (`frontmatter(…)`) or `ctx.*` sample in
      suppressed prose causes no file read (assert the report's `read_set`
      empty for it), no remote fetch registration, no context-group demand
      (context capture test in `lib/tests/l1/ambient_ctx_capture.rs` style),
      no ICMP probe planned, and no call-check rejection under a blocking
      policy; the same sample in an active option value keeps today's
      behavior

### Validation checkpoint (Phase 2)

- `just test` (lib L1) in `darkmatter/` green; targeted filters:
  `file_interpolate`, `interpolation`, `transclusion`, `rendering`.
- Byte-identity guard: the full existing corpus composes unchanged with the
  option absent (no test regressions).
- Under a deny-all blocking policy, a child with unknown-name prose examples
  plus `interpolate=false` composes clean; the same child without the option
  still warns/blocks (both asserted).

---

## Phase 3: Preflight and Reference Graph

Prove the approval walk and the reference graph agree with execution, and
that identity separation holds.

### Necessary Rules (Phase 3)

18. **Preflight walk inherits by construction.** The condition-blind walk
    derives child options through `markdown_child_options` /
    `inherited_child_options` (preflight/collect.rs:987, 1123, 1189), which
    now fold the setting (Rule 6): discovery composes each child under the
    setting execution will use, so the approved command bytes are identical.
    `parameters_identity` (Rule 7) makes a repeated site whose copy differs
    in `interpolate` a distinct child. No preflight code change beyond
    tests. Evidence: engine.rs:1892-1898 ("Pre-flight discovery walks the
    graph with it, and composition executes with it").
19. **Reference-graph fold.** `build_node`'s local-file recursion
    (markdown/reference/graph.rs:605-666) folds the parsed directive's
    option: `accepted_child_options` (graph.rs:417) gains a
    `prose: ProseInterpolation` parameter — the caller at graph.rs:631
    passes `effective_prose_interpolation(options.compose.prose_interpolation,
    &directive.options)` (the walk parses directives with the transclusion
    parser at graph.rs:557, so `directive.options.interpolate` is the
    Phase 1 field) — and applies `with_prose_interpolation(prose)` before
    recursing, so `prepare_content`'s InlinePre run (graph.rs:1154-1204)
    suppresses the child's prose and reports no warnings from it. The graph
    identity picks the setting up from the options encoder (Rule 5);
    per-directive values are document content, which the graph's document
    identity already covers. URL nodes are only ever the root source in the
    graph (graph.rs:1126), so no URL-child fold exists there. Evidence:
    spec "Carry the setting through child-request derivation and every
    recursive composition path" and "reference-graph preparation identity".
20. **`md validate refs`/`md graph` behavior.** Prepared child content keeps
    its literal examples (they are still reference-bearing text — links in
    suppressed prose are still validated, since link extraction is not
    expression evaluation); warnings from suppressed prose expressions do
    not appear. Evidence: spec Verification rows "Replacement, exclusion,
    link handling, and wrappers under false" and the passive-preparation
    requirement.

### Tasks

**Wave 5** (two tasks, parallelizable):

- [ ] Preflight agreement
    - Tests in `compose/tests/preflight.rs` (and `compose/tests/caching.rs`
      for identity): a child under `interpolate=false` whose prose shows a
      `::shell`-looking sample or command-like text yields the same approval
      set with and without the option when no active inputs differ; a
      repeated site differing only in `interpolate` is treated as distinct
      parameters; shell discovery (`md compose --shell`'s library path)
      approves exactly the bytes execution uses for active inputs under a
      disabled-prose child
    - Cache-identity tests: same child included normally and under `false`,
      both directive orders, cache enabled and disabled — independent
      outputs and warnings; a cache entry from one setting is never served
      to the other (assert via `cache_stats` reuse counters)
- [ ] Reference-graph propagation
    - Implement Rule 19 in `markdown/reference/graph.rs`
    - Tests (in `markdown/reference/` test module or
      `lib/tests/l1/…` where graph tests live): a child under `false`
      prepares literal prose (assert prepared content / no expression
      warnings); two option sets differing only in the setting produce
      different graph options identity; links inside suppressed prose are
      still reported as references

### Validation checkpoint (Phase 3)

- `just test` green; targeted: `preflight`, `caching`, graph/reference
  filters.
- One end-to-end agreement test composes the same tree through execution,
  the preflight walk, and reference-graph preparation and asserts the
  prose-literal output and warning sets agree.

---

## Phase 4: Editor and CLI Verification

DMLS support and the CLI-level proof, including the frozen caller-skill
fixture and the shipped-artifact passive check.

### Necessary Rules (Phase 4)

21. **Completions.** Key completion offers `interpolate=` on `::file`/`::url`
    (Rule 8's `option_keys` drives `directive_option_completions`,
    dmls/src/providers/dsl.rs:168-230); value completion offers `true` and
    `false` via `option_values`. `::code` offers neither.
22. **Hover.** Inside `directive_hover` (dsl.rs:474), as an early branch
    before the target-resolution lines: when the hover offset falls within
    an `interpolate` option's key or value span on a File/Url directive,
    return the option hover instead of the generic directive hover. It
    shows: the exact author-ruled sentence — "interpolation in this file
    will not be interpolated but instead kept as literal text" — plus
    defaulting (omitted or `null` means `true`) and inheritance
    (descendants inherit; a nested `true` cannot re-enable under a disabled
    ancestor and warns nothing). Evidence: spec "Editor behavior".
23. **Malformed-value diagnostic.** New code
    `dm.directive.invalid_option_value` (ERROR) in
    `dmls/src/diagnostics/codes.rs`, documented in
    `dmls/docs/diagnostics.md`, reported at the option's value span by a new
    check in `malformed_option_diagnostics` (dsl.rs:895): for
    File/Url directives, every `interpolate` option is validated with the
    shared `parse_interpolate_value` (Rule 1), with `was_quoted` derived
    from the raw span (`&text[value.span]` starts with `'` or `"`), and a
    missing value (`interpolate=` with `value: None`) reports the same code
    at the key. Library and editor agree because they share the validator.
    A repeated valid assignment is not a diagnostic (last wins).
24. **Included files keep their own treatment.** No DMLS surface changes how
    an included file is analyzed when open: its expressions keep normal
    highlighting, completion, diagnostics (a regression test asserts an
    included file's diagnostics are identical whether or not any parent
    uses `interpolate=false`). Evidence: spec "An included file's
    expressions keep their normal highlighting … because the setting
    belongs to one inclusion, not to the file."
25. **CLI proof, no CLI changes.** The CLI binary needs no code change
    (it delegates to the library); the phase adds
    `darkmatter/cli/tests/l1/` tests:
    - **Publication fixture** ("frozen caller-skill fixture"): a fixture
      directory shaped like a caller skill — a `SKILL.md` whose frontmatter
      declares a colliding `name`, a `topics/` doc whose prose shows
      `{{ name }}`, `{{ undeclared }}`, and `{{{ x }}}` — composed with the
      publication arguments `compose --frontmatter --deny all` (the exact
      arguments of `just publish-skills`, `just/skills.just:49); with
      `interpolate=false` on the inclusion all three survive verbatim and
      the run succeeds; without the option the undeclared name fails the
      run (the existing guard stays).
    - **Located errors**: the control case plus two invalid shapes (quoted
      `"null"`, and a missing/empty value) assert the rendered error names
      the including file and directive line and that nothing was composed.
    - **Passive shipped-artifact check**: extend the shipped-artifact test
      pattern (`cli/tests/l1/schema_validate_shipped_artifacts.rs`) with a
      passive assertion over the real `darkmatter/docs/use-skills/` sources:
      for every `::file`/`::url` directive the library scanner finds there,
      `parse_directives` succeeds and every `interpolate` option value is
      valid or absent (the grammar recognizes the option on shipped
      artifacts; no fixture reads internal `prompts/`).

### Tasks

**Wave 6** (two tasks, parallelizable):

- [ ] DMLS option support
    - Completions (Rule 21), hover (Rule 22), diagnostic + code + doc row
      (Rule 23), included-file parity test (Rule 24); in-process tests
      following `dmls/tests/l1/value_hovers.rs` conventions; update the
      semantic-tokens test (`dmls/src/providers/semantic_tokens.rs:1261`
      pattern) with an `interpolate=` option token case
- [ ] CLI verification tests
    - The three Rule 25 tests, using `CliProcessFixture` and the frozen
      fixture under `cli/tests/fixtures/` (frozen files only; no reads of
      internal `prompts/`)

### Validation checkpoint (Phase 4)

- `cargo nextest run -p darkmatter-lsp` and `-p darkmatter-cli` green.
- The publication fixture test demonstrates the spec's final table row:
  `{{ name }}` and `{{{ x }}}` survive unchanged through the ordinary
  publication compose arguments.

---

## Phase 5: Documentation, Skills, and Publication

Every current-behavior document that this feature changes is updated in the
same change; the caller-skill transclusion audit is resolved (not deferred).

### Necessary Rules (Phase 5)

26. **Docs pages.**
    - `darkmatter/docs/transclusion/block-transclusion.md` "Options and
      Conditionals" (line 128): a new numbered entry after `set` for
      `interpolate` with the value table, the literal-prose example from the
      spec, inheritance (confined to the inclusion branch; nested `true`
      accepted silently), active directive inputs (the spec's `::block`
      example), frontmatter behavior, and the fresh-composition limit
      (saving and recomposing uses normal rules). Written for a developer
      new to the repo, with the example fenced.
    - `darkmatter/docs/inline/interpolation.md`: a short section presenting
      `interpolate=false` as the inclusion-side alternative to triple-brace
      escaping when reusing documentation, and its precedence over the
      child's `interpolate_code_blocks: true`.
27. **Skills.**
    - Repo skill `.claude/skills/darkmatter/compose.md`: the interpolation
      stage description and the "body follows body interpolation's
      executable-span rule" passage (compose.md:219-222) gain the
      inclusion-setting exception and a pointer to the option.
      `.claude/skills/darkmatter/expressions.md` needs no change: its
      `{{{ }}}` discussion (expressions.md:190-192) is about produced-value
      data scanning, which suppression does not alter.
    - Caller skill `darkmatter/docs/use-skills/use-darkmatter/authoring.md`:
      the transclusion section (line 137) shows `::file ./x.md
      interpolate=false` in its fenced example list with one line of
      guidance; `SKILL.md`'s description unchanged (the option is detail,
      not a new trigger).
    - `darkmatter/docs/use-skills/use-dmls/features.md`: one line under its
      Directives/Transclusion feature lists (DMLS completes/validates
      `interpolate=`).
    - Never edit installed skill copies; canonical sources only.
28. **CLAUDE.md.** The Skill Audiences bullet (CLAUDE.md:239-242) replaces
    "(a `::file` option to leave a passage's prose uninterpolated,
    `interpolate=false`, is **planned**)" with guidance to use
    `interpolate=false` when including documentation whose expression
    examples must survive publication, keeping the separate escaping
    guidance for expressions authored directly in the caller skill and the
    `--deny all` publication guard. Evidence: spec "Documentation and
    caller-skill adoption".
29. **Caller-skill adoption audit — resolved at planning time.** Every
    `::file`/`::code` line under a `docs/use-skills/` tree today is a fenced
    example, not a live transclusion (verified:
    `darkmatter/docs/use-skills/use-darkmatter/authoring.md:140-147`,
    `claudine/docs/use-skills/use-claudine/composition.md:60-62`,
    `claudine/docs/use-skills/use-claudine/SKILL.md:94` — all inside code
    fences). No adoption edits are needed. The implementer re-runs the
    audit grep (`rg -n "^::file |^::code "` under `*/docs/use-skills/`,
    excluding fenced blocks) as a freshness check: a live transclusion that
    has appeared since this plan was written gets the option added and its
    published text checked in Wave 8; the audit finding none confirms this
    rule's recorded outcome.
30. **Publication.** Only darkmatter's skill sources change (Rule 27), so
    `just publish-skills` runs from `darkmatter/`; composition under
    `--deny all` must succeed and the installed `use-darkmatter` text must
    contain the new guidance.

### Tasks

**Wave 7** (three tasks, parallelizable — separate files):

- [ ] Transclusion and interpolation docs
    - Rule 26's two pages
- [ ] Skill sources
    - Rule 27's repo-skill and caller-skill edits, plus the use-dmls line
- [ ] CLAUDE.md rule
    - Rule 28's replacement, wording kept minimal

**Wave 8** (depends on Wave 7):

- [ ] Publication and audit
    - Re-run the Rule 29 audit grep; `just publish-skills` in `darkmatter/`;
      verify the installed `~/.claude/skills/use-darkmatter/` text mentions
      `interpolate=false` and that its examples survived publication

### Validation checkpoint (Phase 5)

- `just publish-skills` exits 0; the composed caller-skill output shows
  literal `{{ … }}` examples only where authored as examples.
- Docs drift pass: each edited page states current behavior; no page
  references a feature/fix by name.

---

## Phase 6: Closing Verification and Handoff

### Tasks

**Wave 9**:

- [ ] Full area verification
    - `just test`, `just lint`, and a clean typecheck for the darkmatter
      package area (lib, cli, dmls); the spec scopes verification to L1 —
      no terminal, focused window, or browser is used
    - Comment-discipline pass over every symbol changed in Phases 1-4
      (`BlockOptions.interpolate`, `ProseInterpolation`,
      `parse_interpolate_value`, `active_input_ranges`, `mask_prose`,
      `effective_prose_interpolation`, the classifier module docs): `///`
      docs state the contract, no HOW-narration; drift in touched inline
      comments fixed in the same change
- [ ] Handoff artifacts
    - Create `darkmatter/features/2026-11-14-file-interpolate-option/implementation-log.md`
      recording what was built, any departure from the spec (none expected)
      and the Phase 5 audit outcome
    - Update the spec frontmatter: `implemented: true`, `implemented_by:
      opencode/zai-coding-plan/glm-5.3`, `status: implemented` (the status
      move to `completed` stays with the author)
    - Tick every `- [ ]` in this plan that is done

### Validation checkpoint (final)

- All phases' checkpoints re-checked green after the last edit; `git status`
  shows only files named by this plan (plus `Cargo.lock` if touched); the
  terminal state is "implementation complete, ready for review".

---

## Verification traceability

| Spec verification row | Where proven |
|---|---|
| Value table, defaults, aliases, quoted words | Phase 1 matrix tests (parser), Phase 4 CLI control |
| Literal prose under `false`, no warnings under `--deny all`, triple braces | Phase 2 Wave 3 |
| `interpolate_code_blocks: true` overridden | Phase 2 Wave 3 |
| Frontmatter + `set` override under `false` | Phase 2 Wave 3 |
| Conditional/collection blocks under `false` | Phase 2 Wave 3 |
| Nested targets, option values, shell inputs evaluate | Phase 2 Waves 2-3 |
| Inheritance: nested omitted/false/true/null | Phase 2 Waves 2-3 |
| Prologue/epilogue, remote Markdown, nested composition | Phase 2 Wave 2 (+ remote fixture via the local `127.0.0.1` server pattern of `remote_fetch.rs` tests) |
| Same child both ways, both orders, caching on/off | Phase 3 Wave 5 |
| Inactive prose: no read/fetch/demand/approval | Phase 2 Wave 4 |
| Replacement/exclusion/links/wrappers under `false` | Phase 2 Wave 3, Phase 3 Wave 5 |
| Invalid values, `"null"`, missing value, repeats; library and editor agree | Phase 1 matrix, Phase 4 Wave 6 |
| Published fixture with colliding `name` | Phase 4 Wave 6 |
| Locations in nested files after block expansion | Phase 2 Wave 3 (site-scope assertions), Phase 3 Wave 5 |
| Editor hover/completion/diagnostic; open child unchanged | Phase 4 Wave 6 |
| Reference-graph and shell preflight alongside composition | Phase 3 Wave 5 |