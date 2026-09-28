---
created: 2026-09-27
total_phases: 6
phase: 1
agent: claude/opus
yolo: true
spec: 2026-09-27-agent-text-is-data
packages:
    - darkmatter
    - claudine
    - claudine-cli
---

# Plan: agent-produced text is data, never instructions

## Summary and Success Criteria

### The work

Darkmatter treats every string as a potential instruction. Text returned by an
expression, a file read, a shell command, or a `{{{ … }}}` escape is scanned
again by a later pass. Claudine then feeds agent output back through the same
channels that carry person-authored templates. The fix has five technical
tracks:

1. **Origin in Darkmatter (R1, R5).** Two places rescan today:
   - Frontmatter values are a bare `FrontmatterMap = IndexMap<String,
     serde_json::Value>` (`darkmatter/lib/src/markdown/types.rs:23`) with no
     origin. Pass 2 of frontmatter interpolation (`compose/pipeline/mod.rs:351-375`)
     re-partitions every key with `contains_interpolation`.
   - Body interpolation rescans to a fixed point (`interpolation/rewrite.rs:80`,
     `MAX_INTERPOLATION_DEPTH = 10`). `convert_literals` (`rewrite.rs:89`) turns
     `{{{x}}}` into a bare `{{x}}` that a later pass re-reads.

   Darkmatter must carry a **data** origin from the moment a value is produced
   until every instruction scan finishes. That covers frontmatter leaves, body
   text, and transcluded text. The shell-candidate decision is made on the
   **authored** value; today `parse_shell_value` decides on the resolved value
   (`frontmatter_shell_expansion.rs:539`). Errors on override keys must stop
   claiming "Defined in:" the document; `with_on_disk_source` rewrites
   `SourceRef::Effective` to `OnDisk` (`markdown/types.rs:382`).
2. **A literal token (R2).** Add a versioned `{{!data:v1:…}}` codec, lexer
   recognition, and a public decode API for consumers.
3. **Origin in Claudine's runtime (R3, runtime half).** `_loop_last_output`, loop
   ambient values, `outputs`, lifecycle `set:` mutations, and the reserved step
   overlay all reach Darkmatter through `layered_set_overrides`
   (`claudine/lib/src/composition/runtime_state.rs:235`). That function feeds
   `ComposeOptions::with_set_overrides`, the same channel as the person-typed
   `--set`. `proxy.with:` values are merged *into the authored frontmatter map*
   (`claudine/cli/src/commands/wrap/overlay.rs:17`). Two places re-evaluate a
   result that contains `{{`, which is itself a rescan:
   - lifecycle `render_message` (`lifecycle/executor.rs:1302`)
   - `resolve_typed_value` (`executor.rs:1862`)
4. **Inline persistence (R3 persistence half, R4).** The inline closure
   (`claudine/lib/src/composition/closure.rs:107`) today runs: read the file,
   check the body, `restore_properties_text`, plan and save the hash, then
   `atomic_write`. It gains **repair** before the restore and **encode** after
   it. The semantic `FrontmatterDelta` that `restore_properties_text` already
   returns (`darkmatter/lib/src/markdown/hash/write.rs:85`) is the starting
   point for deciding which values the agent owns.
5. **Docs and migration (R6, the fixed-point change).** Rewrite every authored
   template that relied on rescanning, and update Darkmatter and Claudine docs
   and skills.

### Packages touched

`darkmatter` (lib, and the CLI only through its tests), `claudine` (lib), and
`claudine-cli`. No other workspace crate calls `with_set_overrides` with
data-origin values. Phase 2 verifies this with a workspace-wide caller check.

### Definition of done

- [ ] `md compose esc.md` (spec reproduction) prints `Body: fixed {{ area }}`.
- [ ] A value injected with **data** origin that contains `{{ area }}`, `{{…}}`,
      `{{{ area }}}`, or whole-value `$(echo X)` composes to exact text in
      frontmatter, body, and transcluded text. There is no evaluation, no parse
      error, and no shell approval.
- [ ] `md compose repro.md --set '{"note":"see {{…}} siblings"}'` still
      fails, because command-line setters stay templates (ruling N1). The error
      now says the key came from `--set` and does not print the document as the
      definition site (R5).
- [ ] An authored whole-value shell candidate is found by preflight and handled
      by the normal approval policy. A command shape produced by interpolation
      is not. Shell output that looks like a command stays data.
- [ ] Property test: `decode(encode(s)) == s` for arbitrary Unicode, including
      strings that look like tokens. A malformed token fails with a source
      location.
- [ ] Claudine loop, sequence, inline, inline-unrepairable, and lifecycle
      acceptance tests from the spec exist and pass. So does the regression
      test that `--set '{"x":"{{ title }}"}'` still fills in.
- [ ] No authored prompt in the repository depends on fixed-point rescanning.
      Each migrated template is listed in the Phase 2 migration table.
- [ ] `just test` and `just lint` are green in `darkmatter/` and `claudine/`.
      No new `#[cfg]` beyond the existing unix-only fake-agent conventions.
- [ ] The Phase 6 docs and skills describe the single-pass rule, the token,
      raw runtime values, and the plain-scalar guardrail.

---

## Phase 1 — Rulings, Reproduction, Spikes, and Inventory

Goals: lock the open design points, turn every "where agent text re-enters"
row into a red test on today's code, and remove the three structural unknowns
before Darkmatter's core is changed.

### Necessary Rules

These rulings are proposed by the planner. Under `yolo: true`, implementation
proceeds on the stated default unless the author overrules one. **N9 is the one
the author should explicitly confirm**, because it narrows a sentence in R3.

- [ ] **N1 — The spec's `--set` reproductions stay failures.** The *Decisions*
      section keeps command-line setters as templates. So
      `--set '{"note":"see {{…}} siblings"}'` still fails after the fix, and
      `--set '{"note":"$(echo INJECTED)"}'` still asks for shell approval
      (both confirmed on today's `md`). Only the attribution changes (R5).
      Darkmatter's R1 acceptance tests inject data through the new library
      data-override API (N3). `md` gains **no** new CLI flag.
- [ ] **N2 — Origin travels out of band, never as in-band escaping.**
      - Frontmatter: each string leaf is in one of two states. *Authored and
        not yet scanned* means a template. *Data* means inert. Darkmatter
        records the data state as a set of JSON-pointer paths next to the
        `FrontmatterMap`, and that set survives merge, deferral, and pass 2.
      - Body and transcluded text: the representation is chosen by spike S1.
        The default is tracked data ranges extending `BodyOrigin`.
      - A leaf or span is scanned **at most once**. Its result is data.
- [ ] **N3 — Split the override channel by origin.** `ComposeOptions` gains
      layered overrides with an explicit `OverrideOrigin::{Authored, Data}`.
      `with_set_overrides` keeps its meaning (Authored), so every existing
      caller is unchanged.
      - Claudine's `layered_set_overrides` returns origin-tagged layers. User
        setters are Authored. Lifecycle mutations, `outputs`, the loop ambient
        values (`_loop_last_output` and the rest), and the reserved sequence
        overlay are Data.
      - `proxy.with:` moves from the authored-map merge into a Data layer.
      - Precedence between layers stays exactly as today, and a test pins it.
      - After a deep merge, a leaf takes the origin of the layer that supplied
        it.
- [ ] **N4 — Frontmatter pass 2 only scans deferred authored leaves.** Pass 2
      re-scans only the leaves deferred behind shell output, and only their
      authored text. Shell output is data. This also removes the mismatch
      between the untrimmed `starts_with("$(")` check
      (`frontmatter_interpolation.rs:714`) and the trimmed `parse_shell_value`:
      both use one shared predicate on the **authored source value**, and
      preflight and runtime call the same function (R1.2).
- [ ] **N5 — Token spelling and scope.**
      - Spelling: `{{!data:v1:<base64url, unpadded>}}`. The empty string is
        `{{!data:v1:}}`.
      - It is recognized **only as an entire frontmatter string leaf**: exact
        match, no surrounding whitespace.
      - `{{!data:` found anywhere else (mixed text, the body, inside an
        expression) is a located `MalformedLiteralToken` error. It never falls
        back to expression parsing. `!` is the unary operator today
        (`expression/lexer.rs:705`), so the lexer checks for the prefix before
        it parses an expression.
      - The encoder always writes a **double-quoted** YAML scalar, because a
        plain scalar starting with `{` would be read as a flow mapping.
      - Add `base64` as a direct `darkmatter` dependency. It is already in
        `Cargo.lock`. Update `docs/dependencies.md`.
- [ ] **N6 — Where tokens are decoded.**
      - Darkmatter decodes tokens during frontmatter pass 1 into Data leaves.
      - A public `decode_literal_tokens(&Value) -> Result<Value, …>` (plus a
        `Frontmatter` convenience method) serves every Claudine reader,
        including schema checks.
      - Nothing decodes tokens outside frontmatter.
- [ ] **N7 — Inline closure order.** Read, then the body checks, then
      **repair (R4)**, then `restore_properties_text`, then **encode (R3)**,
      then the hash plan and save, then `atomic_write`, then the completion
      instance (decoded values), then `evaluate_completion`.
      - The completion schema stays after the write. That is today's contract,
        and a failed schema verdict keeps the artifact.
      - This satisfies R3 ("encode before hashing, completion schema
        validation, and the write"). The schema sees decoded values through N6.
- [ ] **N8 — Which values the agent owns.**
      - The agent owns every **string leaf** that is new or changed compared
        with the pre-run parsed value tree. Comparison is by value, so
        whitespace-only rewrites do not count.
      - Inside a new container, every string leaf is owned. Non-string leaves,
        deleted keys, unchanged leaves, and the closure-owned properties
        (`prompt`, `hash`, `last_updated`) are never encoded.
      - Top-level scalars are replaced through their source spans. Nested
        leaves use the subset spike S2 proves works.
      - Anything outside that subset (anchors, aliases, flow collections, or
        anything S2 marks as not located) fails with a diagnostic attributed to
        the agent, and the existing rollback runs. The whole document is never
        encoded as a fallback.
      - An agent-changed block scalar is replaced by a token. Unchanged block
        scalars keep their bytes. Touched lines keep their line endings.
- [ ] **N9 — Encode only strings that could act as an instruction (author to
      confirm).** Encode an agent-owned string leaf only if it contains `{{`
      or `$(`. Every other string is written as a normal YAML string, quoted by
      the R4 repair when needed.
      - **Why:** Darkmatter treats frontmatter text as an instruction only
        through those two sequences. A value without them is inert with or
        without a token.
      - **The cost of encoding everything:** routine agent edits such as
        `status: completed` would turn into base64 that `sniff`, the kind
        catalog, `just complete`, and human readers cannot use.
      - **Consistency with R2:** the decision still follows origin. Only agent
        values are considered, and it never checks whether a value "looks
        encoded". A raw agent string that looks like a token contains `{{`, so
        it is encoded.
      - The gate is one predicate in one function, so switching to
        "encode every string" means changing one line.
      - All spec acceptance cases still hold. The `summary` and `cmd` values
        become tokens. `note` and `title` round-trip as quoted strings.
- [ ] **N10 — Lifecycle messages and `set:` results are data.**
      - Delete the re-resolution of `{{`-containing results in `render_message`
        (`executor.rs:1302`/`:926`) and in `resolve_typed_value` (`:1862-1875`).
      - Run `reject_surviving_spans(_deep)` on the **authored** template before
        evaluation and on the authored parts of mixed text, never on returned
        data.
      - The nested-span-in-literal guard and the strict whole-value checks stay
        as they are.
- [ ] **N11 — Loop action results are data.** A string produced by
      `render_string_with_lookup` (`looping/actions.rs:192`) enters the next
      iteration's frontmatter as Data.
- [ ] **N12 — Repair scope (R4).** Repair only touches a new or changed
      **top-level** key whose value is a single-line plain scalar. It wraps the
      complete source value in double quotes, escaping it, and keeps the line
      ending. It quotes only when the value:
      - contains `: ` or ` #`,
      - ends with `:`, or
      - starts with a YAML indicator (`{ [ & * ! | > ' " % @` or a backtick).

      YAML 1.2 core numbers, booleans, and nulls, quoted scalars, block
      scalars, collections, and unchanged keys are never touched. Owned
      properties are never repaired. A malformed owned property therefore fails
      inside `restore_properties_text` with an agent-attributed diagnostic, and
      rollback runs.
- [ ] **N13 — Authoring tools showing decoded text is out of scope.** DMLS
      (the Darkmatter language server) hover or display of decoded values is
      not part of this fix. Phase 6 documents how to hand-edit a token and adds
      one follow-up line to the Darkmatter docs. No new spec is created.

### Wave 1 (parallel; read-only apart from new test files)

- [ ] **Red reproduction tests** (`feature-tester-rust`)
      - Add failing L1 tests, marked `#[ignore = "red until phase N"]`, that
        reproduce each row of the spec's "Where agent text re-enters" table on
        today's code:
        - loop `_loop_last_output` with `{{…}}` and with `$(…)`
        - sequence `outputs` / `last(outputs)` containing `{{ ctx.repo }}`,
          including a nested entry from a parallel group
        - inline agent-added `summary: fixed {{…}} parsing` on the second run
        - lifecycle `message:` from `frontmatter(log, 'message_to_agent')`
        - `set:` and `proxy.with:` derived from agent data
        - the Darkmatter `esc.md` case
      - Use `InlineAgentStub` (`claudine/cli/tests/common/mod.rs:1182`) and the
        local `fake_goose` helpers. Spawn through `CliProcessFixture`.
      - Each later phase removes the `#[ignore]` for the rows it fixes.
- [ ] **Spike S1: body provenance** (`rust-architect`)
      - Compare two representations of data-origin text in the body across the
        whole operation list (`pipeline/operations.rs`):
        (a) byte ranges tracked through each rewrite, extending
        `body_origin.rs`;
        (b) opaque private-use placeholders into a data arena, flattened at
        Finalization.
      - The operations to cover are InlinePre (TextReplacement, PageBlocks,
        Interpolation, ShellExpansion, ShellBlocks), the Transclusion splice
        (`transclusion/engine.rs:1104/1482`), InlinePost, and Finalization,
        plus TOC, links, and Markdown-aware scanning.
      - Find out how frontmatter key-to-key references resolve today, for
        example `a: "{{ b }}"` and `b: "{{ c }}"`. Are they in dependency order,
        or do they rely on the rescan? Does N2 need a topological order?
      - **Output:** `spike-s1-body-provenance.md` in this fix directory with the
        chosen representation, every operation that must honor it, and a
        prototype diff summary. Default: (a), unless an operation cannot remap
        ranges.
- [ ] **Spike S2: nested YAML leaf spans** (`rust-developer`)
      - Establish which nested string leaves can be replaced exactly through
        source spans. Candidates are `parse_text_frontmatter` in `hash/write.rs`
        and the YAML focused-region locator recently added to `biscuit-terminal`
        (commit `4f3f601cf`).
      - Cover block maps, block sequences, block scalars, and CRLF.
      - **Output:** `spike-s2-yaml-leaf-spans.md` naming the supported subset
        for N8 and the API to use. The spike must not add a new YAML parser
        dependency.
- [ ] **Spike S3: token lexer surface** (`rust-developer`)
      - List every scanner that would meet `{{!data:`:
        - `ExpressionFinder::scan`
        - `expression/lint.rs` (`is_whole_value_span` / `whole_value_span`)
        - preflight `collect.rs`
        - the YAML fallback placeholder protections (`markdown/frontmatter.rs:634/695`)
        - `mask_interpolations` (`frontmatter_shell_expansion.rs:1040`)
        - DMLS diagnostics
      - Confirm that N5's prefix check is enough for none of them to treat a
        token as an expression or a shell candidate.
      - **Output:** a short table in `spike-s3-token-lexer.md`.
- [ ] **Inventory I1: fixed-point reliance** (`Explore`)
      - Find every authored template that depends on rescanning: in
        `prompts/**`, `claudine/docs/research/**` prompt documents, the
        `darkmatter/` and `claudine/` test fixtures, and any Markdown in the
        repository that uses `{{{` in frontmatter and references that key from
        the body.
      - Known starting points: `prompts/_prompt.md`, `prompts/_add/_workflow.md`,
        `prompts/_add/add-context-variables.md`,
        `prompts/_add/add-expressions.md`. Also list the tests that assert
        today's rescan behavior (the `MAX_INTERPOLATION_DEPTH` tests and the
        `rewrite.rs` / `frontmatter_interpolation.rs` suites).
      - **Output:** a migration table in `inventory.md` (file, construct, new
        form or new expected output).
- [ ] **Inventory I2: entry points and readers** (`Explore`)
      - Confirm each spec table row against the code, then look for other
        places where file or runtime data re-enters (R1 asks for this). Check
        `dispatch/template.rs`, system-prompt preparation
        (`system_prompt/prepare.rs:198`), the `claudine-contract` and Reaper
        consumers of `outputs`, and hook templates.
      - List every Claudine reader of inline frontmatter that must use N6's
        decoded API. Start from the survey list: `resolve.rs:174/367`,
        `prepare.rs:316/345/570`, `schema/mod.rs:419/502/527/763`,
        `looping/seed.rs`, `prepare/service.rs`, `file_detail.rs:52-104`,
        `sequence/preflight/mod.rs:778`, `sequence/task/mod.rs:508`,
        `cli/wrap/wrapper_stages.rs:370`, `cli/wrap/overlay.rs:48`,
        `completion.rs:319`.
      - **Output:** append these tables to `inventory.md`.

### Checkpoint 1

- [ ] Every red test fails for the documented reason (not a fixture error).
- [ ] S1, S2, and S3 outcomes are recorded. N2 and N8 defaults are confirmed or
      amended in this plan before Phase 2 starts.
- [ ] The I1 and I2 tables exist. Any new re-entry point from I2 has a red test
      and an owning task in Phase 2, 4, or 5.

---

## Phase 2 — Darkmatter: single-pass scanning and true origin (R1, R5)

Depends on Phase 1. All changes are in `darkmatter/lib`.

### Wave 1 (parallel)

- [ ] **Origin-tagged overrides** (`rust-developer`)
      - Add `OverrideOrigin`, the layered-override builder on `ComposeOptions`,
        and data-path tracking through `prepare_frontmatter_for_compose`
        (`compose/util.rs:216-240`). Keep `with_set_overrides` as Authored.
      - The pre-interpolation string snapshot (`util.rs:240`) records only
        authored top-level strings, so a data leaf can never be treated as
        "original" shell text.
      - Search the whole workspace for `with_set_overrides` callers and confirm
        none needs Data.
- [ ] **Frontmatter single pass** (`rust-developer`)
      - `interpolate_frontmatter_impl` / `rewrite_value`
        (`frontmatter_interpolation.rs:254/696`): an interpolated leaf becomes
        Data. Literal-escape results (`convert_frontmatter_literals`, `:476`)
        are Data. Pass 2 (`pipeline/mod.rs:351-375`) visits only deferred
        authored leaves (N4). If S1 says so, add dependency-ordered key
        resolution.
      - Keep typed whole-value results, null handling, and
        `is_whole_value_literal`.
- [ ] **Shell shape from authored source** (`rust-developer`)
      - Add one predicate, `authored_shell_candidate`, used by:
        - runtime `scan_frontmatter` / `parse_shell_value`
          (`frontmatter_shell_expansion.rs:523/1245`)
        - preflight `scan_one_frontmatter` / `detect_dynamic_frontmatter_command_shape`
          (`preflight/collect.rs:693/841`)
        - the deferral mark (`frontmatter_interpolation.rs:714`)
      - Data leaves and shell output are never candidates. The
        executable-token rule is unchanged.
      - `validate_no_whole_value_shell_leak` (`:1866`) inspects authored
        unresolved values only (R1.4). It stays a real guard, not a string
        check on flattened text. The decision is the same when shell expansion
        is disabled.

### Wave 2 (depends on Wave 1)

- [ ] **Body and transclusion single pass** (`rust-developer`)
      - Implement S1's chosen representation. `interpolate_text_located`
        (`interpolation/rewrite.rs:149`) scans authored text once; inserted
        text, literal-escape output, file reads, and shell output are Data.
        Remove the rescan loop and `MAX_INTERPOLATION_DEPTH`, and delete their
        documentation in the same change.
      - Every body operation that scans for instructions skips Data: inline
        ShellExpansion, ShellBlocks, directives, and transclusion discovery.
        The transclusion splice keeps the child's Data ranges.
      - Flatten to plain text only at Finalization.
- [ ] **R5 attribution** (`rust-developer`)
      - Add `SourceRef` variants for command-line overrides (Authored override)
        so `with_on_disk_source` (`markdown/types.rs:382`) no longer rewrites
        them to `OnDisk`.
      - `markdown/errors/blocks.rs:287-435` renders "came from `--set`" instead
        of "Defined in:" plus a document excerpt. Authored document errors
        keep their location.
      - Keep the variant generic, so Claudine can attribute a malformed token
        or a failed repair to the agent (Phase 5).

### Wave 3

- [ ] **Migration and tests** (`feature-tester-rust`)
      - Apply I1's migration table. Every authored template that relied on the
        rescan is rewritten into separate authored spans or expression
        concatenation. Update tests that asserted the rescan, recording each
        changed expectation and its reason in `inventory.md`.
      - Add the Darkmatter L1 acceptance tests for R1 and for R1.2/R1.4 across
        frontmatter, body, and transclusion:
        - data-origin inputs `{{ area }}`, `{{…}}`, `{{{ area }}}`, and
          `$(echo X)` stay exact
        - `esc.md` prints `Body: fixed {{ area }}`
        - adjacent authored spans still evaluate
        - authored shell goes through preflight and approval
        - an interpolated executable is rejected
        - shell output that looks like a command stays data
        - strict whole-value errors and lenient mixed-text warnings are
          unchanged
      - Add the R5 test: the `--set` repro error names the override.
      - Un-ignore the Darkmatter red test.

### Checkpoint 2

- [ ] `cd darkmatter && just test && just lint` is green, and `just test-l2`
      is green.
- [ ] `md compose` on every file changed in I1 produces the output the
      migration table expects.
- [ ] Claudine still compiles and its L1 suite is unchanged (`cd claudine &&
      just test`). Claudine has not opted into Data yet, so its behavior should
      not have changed.

---

## Phase 3 — Darkmatter: the literal token (R2)

Depends on Phase 2 Wave 1 (Data leaves exist). **The codec task has no
dependency and may start alongside Phase 2.**

### Wave 1 (parallel)

- [ ] **Codec** (`rust-developer`)
      - Add a `literal_token` module with `encode(&str) -> String` (the
        double-quoted YAML scalar form and the bare token), `decode`, and
        `TokenError`. Follow N5.
      - Add `base64` as a direct dependency (unpadded URL-safe alphabet) and
        update `docs/dependencies.md`.
- [ ] **Property tests** (`feature-tester-rust`)
      - Using `proptest` (already a dev-dependency), test that
        `decode(encode(s)) == s` for arbitrary Unicode. Cover the empty string,
        braces, `$(`, backslashes, newlines, quote characters, and strings that
        look like the token. Also test that the encoded output parses as YAML
        back to the same string.

### Wave 2

- [ ] **Scanner integration** (`rust-developer`)
      - Following S3, a whole-leaf token decodes to a Data leaf during
        frontmatter pass 1 (never re-scanned, never a shell candidate). A
        malformed or misplaced token fails with a located `MalformedLiteralToken`.
        An unchanged token is left as it is in the source.
      - Add the public `decode_literal_tokens` API (N6).
      - Add L1 tests: a decoded `$(echo X)` is not a shell candidate, a decoded
        `{{ area }}` is not evaluated, and malformed tokens report the right
        location.

### Checkpoint 3

- [ ] `cd darkmatter && just test && just lint` is green, including the
      proptest.

---

## Phase 4 — Claudine: runtime values keep data origin (R3 runtime, N10, N11)

Depends on Phases 2 and 3.

### Wave 1

- [ ] **Origin-tagged runtime layers** (`rust-developer`)
      - `layered_set_overrides`, `with_initialized_outputs`, and
        `merge_object_into` (`runtime_state.rs:235-270`) return origin-tagged
        layers. `canonical_compose_options` (`prepare.rs:268-300`) passes them
        to N3's builder.
      - Update the call sites: `cli/compose/prep.rs:1000`,
        `cli/wrap/harness_orch/prompt.rs:163/172`,
        `cli/wrap/sequence/jit.rs:89/105`, and `sequence/task/mod.rs:827/864`.
      - `LoopIterationContext::as_set_overrides` (`looping/types.rs:63/260`)
        and `SequenceStepOverlay::as_set_overrides` (`sequence/model.rs:309`)
        produce Data.
      - `RuntimeState` keeps typed raw values; no token appears at runtime.
      - Make this the single shared boundary API, and add a unit test that
        fails if a second escaping path appears.

### Wave 2 (parallel; depends on Wave 1)

- [ ] **`proxy.with:` as a Data layer** (`rust-developer`)
      - Move the overlay out of `merge_frontmatter_overlay`
        (`cli/wrap/overlay.rs:17`; call sites `compose/prep.rs:352`,
        `harness_orch/prompt.rs:131`, `sequence/iterate.rs:686`) into a Data
        layer with the same precedence. Add a precedence test.
- [ ] **Lifecycle messages and `set:`** (`rust-developer`)
      - Apply N10 in `lifecycle/executor.rs`: `render_message` (`:1302`),
        `resolve_string_value` (`:926`), `resolve_typed_value` (`:1862`),
        `dispatch_runtime_set` (`:1476`), and `resolve_proxy_with` /
        `walk_proxy_with` (`:1757/1787`).
      - The surviving-span checks (`:2050/2067`) inspect authored syntax only.
      - Update the tests in `lifecycle/executor/tests/*` and
        `lifecycle/tests/nested_span.rs`.
- [ ] **Loop actions** (`rust-developer`)
      - Apply N11 in `looping/actions.rs:192`, `looping/expression.rs:267`, and
        `engine.rs:1108`.

### Wave 3

- [ ] **Acceptance tests** (`feature-tester-rust`)
      - Un-ignore and complete these Claudine L1 tests:
        - **loop:** `see {{…}} and $(rm -rf x)` is raw to predicates, appears
          byte-for-byte in the prompt, and requests no approval.
        - **sequence:** `{{ ctx.repo }}` is raw in `outputs` and in
          `last(outputs)`, including a parallel group's nested entry.
        - **lifecycle:** the exact `message:` text is sent, and `set:` /
          `proxy.with:` values stay inert after the next preparation.
      - Add the regression tests: `--set '{"x":"{{ title }}"}'` fills in, and
        the nested-span and strict whole-value guards still fire.
      - Audio stays silent under the fixture defaults.

### Checkpoint 4

- [ ] `cd claudine && just test && just lint` is green.
- [ ] The loop, sequence, and lifecycle red tests pass.

---

## Phase 5 — Claudine: inline repair and persistence encoding (R3 persistence, R4)

Depends on Phase 3 (codec and decode API). It can overlap Phase 4 Waves 2–3,
because it touches `closure.rs`, `completion.rs`, and the readers, not the
runtime layers.

### Wave 1 (parallel)

- [ ] **Narrow YAML repair** (`rust-developer`)
      - Add `repair_agent_frontmatter(candidate, original) -> Result<String,
        RepairError>`. It works lexically on top-level keys only, following
        N12, and preserves CRLF.
      - After the repair, re-parse the whole frontmatter. Duplicate keys, bad
        nesting, missing delimiters, and anything outside the repair case fail
        with the candidate line, attributed to the agent when the pre-run
        comparison supports it. Nothing is ever partially written.
      - Unit tests: nested map, block scalar, duplicate keys, CRLF, a value
        that is already valid YAML, `title: Fix: colons`, and
        `note: see issue #42`.
- [ ] **Leaf encoder** (`rust-developer`)
      - Add `encode_agent_values(restored, original, delta) -> Result<String,
        EncodeError>`. Ownership follows N8 and the encoding gate follows N9.
        Nested leaves use the span API chosen in S2.
      - Unsupported shapes produce an attributed diagnostic.
      - An unchanged stored token, or an unchanged authored value, keeps its
        original bytes.
      - Encode from the raw agent value exactly once.
- [ ] **Decoded readers** (`rust-developer`)
      - Route every I2 reader through N6's `decode_literal_tokens`. This
        includes the schema pre-validation and `drop_invalid_optionals`, whose
        composition-tolerant rule must not treat a token as a pending `{{`
        value.
      - `file_detail.rs` shows decoded text.

### Wave 2 (depends on Wave 1)

- [ ] **Closure wiring** (`rust-developer`)
      - In `reconcile_inline_artifact_with_evidence` (`closure.rs:107-157`),
        apply N7's order (repair, restore, encode, hash, `atomic_write`).
      - `inline_completion_instance` (`completion.rs:300/319`) reads decoded
        values.
      - A repair or encode error goes to the existing rollback path
        (`loop_control.rs:288/2306`) with an agent-attributed diagnostic that
        uses the R5 source variant.
- [ ] **Guardrails** (`rust-developer`)
      - Add the plain-scalar rule to `DEFAULT_GUARDRAILS`
        (`claudine/lib/src/composition/guardrails.rs:25`): quote a value, or
        use a separate comment line, when `#` is meant as a comment.
      - Add the previous text to `HISTORICAL_SHIPPED_GUARDRAILS` (`:84`) so
        materialized copies upgrade.

### Wave 3

- [ ] **Inline acceptance tests** (`feature-tester-rust`)
      - **Inline:** the fake agent adds `summary: fixed {{…}} parsing`,
        `note: see issue #42`, `cmd: "$(echo X)"`, and `title: Fix: colons`.
        The run succeeds, and the file is valid YAML: `summary` and `cmd` are
        tokens, and `note` and `title` are quoted strings (N9).
      - A second run reads back the original strings. An unchanged author
        `{{ area }}` still fills in. The completion schema sees decoded values,
        and `md hash --diff` agrees with the stored hash.
      - **Unrepairable:** a duplicate key, or a malformed nested value, names
        the line and the agent origin. Rollback preserves the pre-run bytes.
        CRLF and block-scalar fixtures keep their formatting.

### Checkpoint 5

- [ ] `cd claudine && just test && just lint && just test-l2` is green.
- [ ] Every red test from Phase 1 now passes, and no `#[ignore = "red until`
      markers remain.

---

## Phase 6 — Documentation, skills, and final validation (R6)

Depends on Phases 2–5.

### Wave 1 (parallel)

- [ ] **Darkmatter docs** (`Documenter`)
      - Files: `darkmatter/docs/inline/interpolation.md` (literals `:187-271`;
        replace the fixed-point description), `inline/fm-interpolation.md`,
        `inline/fm-shell-expansion.md`, `inline/shell-expansion.md`,
        `topics/darkmatter-expressions.md`, `topics/frontmatter-recursion.md`,
        `composition/frontmatter-in-pipelining.md`, and
        `darkmatter-compose-pipeline.md`.
      - Cover the single-pass rule, the fixed-point behavior change and how to
        migrate, authored-source shell detection, the origin-aware guards, the
        token format, and how to hand-edit a token without creating a template.
        Add the N13 follow-up note.
- [ ] **Darkmatter skill** (`Documenter`)
      - `.claude/skills/darkmatter/SKILL.md` (`:261`, `:294`), `compose.md`
        (`:408-412`, `:671`), `frontmatter.md`, and `errors.md`.
- [ ] **Claudine docs and skill** (`Documenter`)
      - Docs: `claudine/docs/topics/flow-control/looping.md`,
        `topics/frontmatter-properties.md`, `topics/composition.md`
        (guardrails `:168-172`), `topics/lifecycle.md` (update the
        surviving-span section to R1.4), `flow-control/lifecycle/*`, and
        `flow-control/sequences.md`.
      - Skill: `.claude/skills/claudine/composition.md` and `lifecycle.md`, and
        the SKILL.md whole-value and lifecycle rows.
      - Cover raw runtime values, the on-disk token contract (including that
        tools reading the YAML directly see the token), N9's gate, and the
        plain-scalar guardrail.

### Wave 2

- [ ] **Final validation** (`rust-developer`)
      - Run `just test`, `just test-l2`, and `just lint` in `darkmatter/` and
        `claudine/`.
      - Re-run the spec's reproduction script and record the new outputs in
        `inventory.md`.
      - Confirm there are no Windows-incompatible constructs (CRLF handling is
        covered by tests; paths go through existing helpers). Load the `os`
        skill before reviewing any `#[cfg]` changes.
      - Walk through the Definition of Done and check off each item with its
        evidence.
      - Final state: **implementation complete, ready for review**. The spec is
        not moved to `_completed`.
