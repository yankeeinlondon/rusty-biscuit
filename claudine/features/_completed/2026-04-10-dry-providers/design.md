# DRY Providers: Unified Prompt Pipeline

Status: Draft — design only, not yet approved for implementation.
Author: Claudine / feat-claudine-composition
Date: 2026-04-10

## Overview

This design collapses the current three-phase, per-provider prompt handling contract in `claudine/cli/src/commands/wrap/profile.rs` into a single canonical pipeline. The goal is to make it structurally impossible for a provider to drift out of agreement with the composition pipeline the way Gemini and Qwen just did (where `apply_non_interactive` bailed because it was inspecting args for a prompt that hadn't been delivered yet).

Option 1 (already implemented, see `commit feat/claudine-composition`) added a shared `require_prompt_or_stdin` helper so all five "prompt-required" providers validate through the same code path. That removes the immediate drift vector but leaves the underlying shape intact: a three-phase contract where `apply_non_interactive`, `prompt_delivery`, and `validate_final_args` each touch the prompt in subtly different ways at different points in the pipeline.

Option 2 removes the shape itself. The prompt becomes a first-class input to the wrap pipeline — extracted once at the entry point, carried through as a typed value, and placed into the child process by exactly one method (`prompt_delivery`). Providers never see the prompt as "an arg inside `args`" during flag assembly, and the post-hoc `validate_final_args` prompt check disappears entirely.

## Goals

- Exactly one method on `WrapperProfile` places the prompt into the child process.
- Providers never infer "is a prompt present" by scanning `args` — that question is answered once, upfront, by the caller.
- `apply_non_interactive` stops mutating or inspecting prompt-shaped arguments and becomes a pure entrypoint/flag injector.
- The direct wrap path (`claudine gemini "hello"`) and the composition path (`claudine sequence file.md --gemini`) share a single prompt-delivery implementation per provider.
- Adding a new provider requires implementing exactly one prompt-related method (`prompt_delivery`) instead of reasoning about three.
- Existing behavior is preserved byte-for-byte on the wire — same flags, same stdin seeding, same error wording for user-facing validation failures.

## Non-Goals

- Redesigning the broader `WrapperProfile` trait (system prompt, MCP, structured streaming, YOLO) — those stay as they are.
- Changing provider binary invocations, subcommand conventions, or user-visible flags.
- Collapsing interactive and non-interactive mode into a single abstraction — they remain distinct session modes.
- Unifying the harness retry (`build_harness_launch`) path — it already flows through `prompt_delivery`, so it gets to ride along unchanged.
- Revisiting the `PromptDelivery` enum variants (`Stdin`, `AppendArgs`, `InsertArgs`) — they remain the placement primitives.

## Current-State Analysis

The existing contract is split across three trait methods and two call sites.

### Trait methods that currently touch the prompt

| Method | Purpose | Actually touches prompt? |
|--------|---------|--------------------------|
| `apply_non_interactive(args)` | Inject entrypoint (`exec`, `run`, `--print`) and reject mode conflicts | **Yes** — Gemini/Qwen convert first positional to `--prompt <value>` here |
| `apply_non_interactive_defaults(args)` | Default model etc. | No |
| `prompt_delivery(args, prompt, non_interactive)` | Decide how to place a *separately supplied* prompt | **Yes** — this is the only method that takes `prompt: &str` as an input |
| `validate_final_args(args, non_interactive, has_stdin)` | Final post-hoc check | **Yes** — scans args to confirm a prompt reaches the child |

### Call sites

There are three places in `wrap/mod.rs` and `wrap/composition.rs` that drive the pipeline:

1. **Direct wrap path** (`wrap/mod.rs` around line 996): `child_args` is pre-populated from the user's passthrough, including any positional prompt. Calls `apply_non_interactive`, then calls `validate_final_args` at line 1108. **Never calls `prompt_delivery` on the main launch** — the user's positional is the prompt.

2. **Composition path** (`wrap/composition.rs` around line 344): `child_args` starts empty. Calls `apply_non_interactive` on an empty args vec, then calls `prompt_delivery(..., &effective_prompt, ...)` at line 487, then `validate_final_args`.

3. **Harness retry path** (`wrap/mod.rs::build_harness_launch` around line 1905): starts from `base_args`, strips any existing prompt via `strip_prompt_from_args`, then calls `prompt_delivery` with the new prompt, then `validate_final_args`.

### The structural problem

Each call site has a different idea of where the prompt lives at each pipeline stage:

| Stage | Direct path | Composition path | Harness retry |
|-------|-------------|------------------|---------------|
| Before `apply_non_interactive` | In `args` as positional | Not in `args` at all | Was in `args`, just stripped |
| After `apply_non_interactive` | In `args` as `--prompt x` (Gemini/Qwen) or still positional (others) | Still not in `args` | Still not in `args` |
| After `prompt_delivery` | N/A — not called | In `args` or stdin | In `args` or stdin |
| `validate_final_args` | Checks `args` | Checks `args` + stdin seed | Checks `args` + stdin seed |

A provider writing `apply_non_interactive` has no idea which of these three states it will be called in. Gemini and Qwen guessed wrong — they assumed the direct path (prompt in args) and bailed in the composition path (empty args). Codex, OpenCode, and Goose guessed right because their author left a comment saying "defer to `validate_final_args`."

The comment at `profile.rs:263-280` is load-bearing documentation. Load-bearing documentation is a smell: it means the trait's shape does not prevent the mistake it is documenting.

### Secondary evidence

- `strip_prompt_from_args` exists in `wrap/mod.rs:1613` as a separate helper with hand-rolled per-provider logic that duplicates the `prompt_delivery` providers' placement rules in reverse. Any provider whose `prompt_delivery` changes must also update `strip_prompt_from_args`. They are two sides of the same contract maintained in two places.
- `find_prompt_location` (`wrap/mod.rs:3423`) is a third hand-rolled per-provider reader of where the prompt lives inside args, used by `extract_user_prompt` for display purposes. Third place, same knowledge.
- In tests, Gemini's `apply_non_interactive` had four separate tests covering positional-to-flag conversion at different arg shapes (`gemini_non_interactive_converts_positional_to_prompt_flag`, `..._preserves_existing_prompt_flag`, `..._converts_positional_with_other_flags`, `..._skips_approval_mode_value`) — all of them exist because that one method is trying to do prompt parsing, which is not what a flag-injection method should be doing.

## Proposed Architecture

### Core idea

A prompt is a typed input to the wrap pipeline, not an argv position to be inferred. The pipeline looks like this:

```text
                               ┌──────────────────────────┐
                               │   Prompt source decision │
                               │  (direct | composition)  │
                               └─────────────┬────────────┘
                                             │
                               PromptSource enum
                                             │
                                             ▼
                      ┌────────────────────────────────────────┐
                      │  Extract prompt from passthrough args  │
                      │  (direct path only, strips from args)  │
                      └─────────────────────┬──────────────────┘
                                            │
               (clean args, Option<String> prompt)
                                            │
                                            ▼
                      ┌────────────────────────────────────────┐
                      │        Pipeline (same both paths)      │
                      │                                        │
                      │  profile.apply_entrypoint(&mut args)   │
                      │  profile.apply_non_interactive(..)     │
                      │  profile.apply_model(..) / yolo / ...  │
                      │  profile.apply_system_prompt(..)       │
                      │                                        │
                      │  if let Some(p) = prompt {             │
                      │    stdin = profile                     │
                      │      .prompt_delivery(&args, &p, ni)   │
                      │      .apply_to(&mut args);             │
                      │  } else if ni && !has_stdin_override { │
                      │    bail!("--non-interactive requires   │
                      │            a prompt");                 │
                      │  }                                     │
                      └─────────────────────┬──────────────────┘
                                            │
                                      launch child
```

### Key invariants

1. **Upfront extraction.** By the time any `profile.apply_*` method is called, `args` contains zero prompt characters. The prompt is held separately as `Option<String>`.
2. **Single delivery.** `prompt_delivery` is the only method that places the prompt. `apply_non_interactive` never reads or writes prompt text.
3. **Presence is a caller concern.** The caller knows whether it has a prompt. If non-interactive mode requires one and the caller has `None`, the caller bails. Providers do not check this.
4. **Validation is generic.** Any remaining `validate_final_args` checks are provider-blind — they check *execution environment* things (e.g. "is stdin both seeded and required?"), not prompt presence.

### New types

Add to `wrap/profile.rs`:

```rust
/// A prompt supplied to the wrap pipeline, already extracted from any
/// CLI passthrough or composition source.
#[derive(Debug, Clone)]
pub(crate) enum PromptSource {
    /// No prompt provided. Valid only for interactive sessions or
    /// when stdin will be piped from the parent.
    None,
    /// A text prompt to be placed by `prompt_delivery`.
    Inline(String),
    /// The caller is forwarding stdin from its own stdin (piped input).
    /// The pipeline should not seed stdin; the child inherits it.
    InheritStdin,
}

impl PromptSource {
    pub fn as_inline(&self) -> Option<&str> {
        match self {
            Self::Inline(s) => Some(s.as_str()),
            _ => None,
        }
    }
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}
```

### Trait changes

The `WrapperProfile` trait gets a smaller, sharper prompt-related surface.

**Removed:**

- `validate_final_args` — replaced by a generic pipeline-level check when there is no prompt and no stdin inheritance. Providers stop needing to reason about this.

**Added:**

- `fn apply_entrypoint(&self, args: &mut Vec<String>)` — the *non-prompt* part of current `apply_non_interactive`. Injects `exec`, `run`, `--print`, etc. Called in both interactive and non-interactive modes because entrypoint injection is mode-agnostic for most providers (e.g. `codex exec` is a non-interactive thing; `claude --print` is non-interactive only — this method gets `non_interactive: bool` so it can choose).

    ```rust
    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool);
    ```

**Narrowed:**

- `apply_non_interactive` becomes `apply_non_interactive_flags`, which only rejects mode conflicts (`-i` vs non-interactive) and injects non-prompt non-interactive flags. It is a `default` no-op on the trait; only Gemini/Qwen actually need it today (for the `-i` conflict check). The positional-to-flag conversion is **deleted entirely** because it is no longer possible for a positional prompt to be in `args` at this stage.

    ```rust
    fn apply_non_interactive_flags(&self, args: &mut [String]) -> Result<()> {
        let _ = args;
        Ok(())
    }
    ```

- `prompt_delivery` keeps its signature and becomes load-bearing. It is the *only* method that takes the prompt and returns a `PromptDelivery`.

### Call site changes

#### `wrap/mod.rs` — direct wrap path

Today (conceptually):

```rust
let mut child_args = args.passthrough.clone();  // includes prompt
profile.apply_non_interactive(&mut child_args)?; // may mutate prompt
// ... other apply_*
profile.validate_final_args(&child_args, ni, false)?;
```

Under the new design:

```rust
// Extract prompt once, upfront, from passthrough — using the same
// logic currently used by `extract_user_prompt` and `find_prompt_location`.
let (mut child_args, prompt_source) =
    extract_prompt_source_from_passthrough(provider, &args.passthrough, has_piped_stdin);

profile.apply_entrypoint(&mut child_args, non_interactive);
profile.apply_non_interactive_flags(&mut child_args)?;
// ... apply_model / apply_yolo / apply_output_format / apply_system_prompt / etc.

let stdin_seed = match &prompt_source {
    PromptSource::Inline(prompt) => {
        profile
            .prompt_delivery(&child_args, prompt, non_interactive)?
            .apply_to(&mut child_args)
    }
    PromptSource::InheritStdin | PromptSource::None => None,
};

require_prompt_unless_stdin(non_interactive, &prompt_source)?;  // generic
```

The function `extract_prompt_source_from_passthrough` is a single, centralized extractor that replaces:

- `extract_user_prompt` (display path)
- `find_prompt_location` (display path reader)
- `strip_prompt_from_args` (harness retry stripper)
- the per-provider prompt-hunting inside `apply_non_interactive` for Gemini/Qwen

All four call sites share one implementation that understands provider conventions once.

#### `wrap/composition.rs` — composition path

Today:

```rust
let mut child_args = Vec::new();
profile.apply_non_interactive(&mut child_args)?; // empty args; Gemini/Qwen bail here today
// ... apply_*
let stdin_seed = profile
    .prompt_delivery(&child_args, &effective_prompt, ni)?
    .apply_to(&mut child_args);
profile.validate_final_args(&child_args, ni, stdin_seed.is_some())?;
```

Under the new design:

```rust
let mut child_args: Vec<String> = Vec::new();
let prompt_source = PromptSource::Inline(effective_prompt.clone());

profile.apply_entrypoint(&mut child_args, non_interactive);
profile.apply_non_interactive_flags(&mut child_args)?;
// ... apply_*

let stdin_seed = profile
    .prompt_delivery(&child_args, &effective_prompt, non_interactive)?
    .apply_to(&mut child_args);
```

The composition path has a prompt by construction (the composed markdown body is the prompt), so there is no `None` branch and no post-hoc validation. The two paths converge on an identical pipeline shape.

#### `wrap/mod.rs::build_harness_launch` — harness retry path

Already uses `prompt_delivery`. The only change is that the caller (`build_harness_launch`) no longer needs `strip_prompt_from_args` because the harness launch is now built from a `PromptSource::Inline(state.base_prompt)` and clean `base_args`, not from args-containing-prompt.

### Provider-by-provider migration

| Provider | `apply_entrypoint` | `apply_non_interactive_flags` | `prompt_delivery` (mostly unchanged) |
|----------|--------------------|-------------------------------|--------------------------------------|
| Claude | add `--print` if `non_interactive && !has_flag("--print")` | no-op | `Stdin` on non-interactive, `AppendArgs` on interactive |
| Codex | ensure `exec` entrypoint | no-op | `Stdin` on non-interactive, `InsertArgs(after exec)` on interactive |
| Gemini | no-op (no entrypoint subcommand) | reject `-i`/`--prompt-interactive` conflict | `AppendArgs(["--prompt", p])` |
| Kimi | add `--print` | no-op | `AppendArgs` |
| OpenCode | ensure `run` entrypoint | no-op | `AppendArgs(positional)` on non-interactive, `AppendArgs(["--prompt", p])` on interactive |
| Qwen | no-op | reject `-i`/`--prompt-interactive` conflict | `AppendArgs(["--prompt", p])` |
| Goose | ensure `run` entrypoint | no-op | `InsertArgs(after run, ["-t", p])` |

Gemini and Qwen lose their `apply_non_interactive` positional-to-flag shuffling entirely. The direct-wrap extractor finds the positional and becomes `PromptSource::Inline(value)`; `prompt_delivery` then places it as `--prompt value`. The round-trip is cheap (one allocation) and eliminates an entire class of bugs.

### Validation: the end state

There is no longer a `validate_final_args` method on `WrapperProfile`. In its place:

```rust
// wrap/mod.rs and wrap/composition.rs
fn require_prompt_or_stdin(
    non_interactive: bool,
    source: &PromptSource,
) -> Result<()> {
    if !non_interactive {
        return Ok(());
    }
    match source {
        PromptSource::Inline(_) | PromptSource::InheritStdin => Ok(()),
        PromptSource::None => bail!("--non-interactive requires a prompt"),
    }
}
```

This is generic, not per-provider. Error wording is generic too (the provider name can be interpolated from `profile.provider()` if desired). Provider-specific error messages (`"requires a prompt (positional or --prompt/-p)"`) were only useful because the user might be typing the wrong flag — but under the new design, *any* positional on the passthrough becomes the prompt via extraction, so there is no "wrong flag" to warn about.

## Risks and Trade-offs

### Risk: error messages change

Existing tests assert substrings like `"requires a prompt (positional or --prompt/-p)"`. Under the new design, those messages disappear and are replaced by a single generic `"--non-interactive requires a prompt"` (or similar). Mitigation: keep the provider-flavored wording as an optional `fn required_prompt_hint(&self) -> Option<&str>` on the trait, invoked by the generic error. Tests update to match the new wording.

### Risk: Codex `exec` entrypoint is both interactive and non-interactive

Codex's `exec` entrypoint is only required in non-interactive mode. The new `apply_entrypoint` method must receive `non_interactive: bool` so it can decide. The proposed signature above already accounts for this.

### Risk: the direct-path prompt extractor has to know every provider's flag conventions

`extract_prompt_source_from_passthrough` centralizes knowledge that today is split across `extract_user_prompt`, `find_prompt_location`, and `strip_prompt_from_args`. That is a net DRY win (three places → one). It also means one place has to know that Gemini uses `-p/--prompt`, Goose uses `-t/--text`, Codex/OpenCode use a positional after the entrypoint, etc. Two options:

1. Hard-code per-provider rules in the extractor (simplest, mirrors today's status quo across three files).
2. Add an `fn prompt_arg_conventions(&self) -> PromptArgConventions` method to `WrapperProfile` that the extractor dispatches on. More typed, slightly more ceremony.

**Recommendation:** option 2. It moves per-provider flag knowledge back onto the provider impl, where `prompt_delivery` already lives, and the extractor becomes a provider-blind algorithm. Adding a new provider then requires implementing `prompt_arg_conventions` + `prompt_delivery` instead of also touching a central `match` statement.

### Risk: users of `claudine <provider> foo bar baz` with multiple positionals

Today, Gemini's `find_first_positional` takes the first positional and the remaining ones stay as bare args, which Gemini CLI then rejects with its own error. The new extractor will do the same thing by default. We can choose a clearer rule in a follow-up (concatenate? first-only + warning?) but that is out of scope for this refactor.

### Risk: harness retry prompt state

`HarnessPromptState::base_prompt` already carries the prompt as a typed `String`, and `build_harness_launch` already calls `prompt_delivery`. The retry path is actually *already* in the shape this refactor wants. The refactor cleans up the entry points to match it.

### Trade-off: one more trait method

The new trait has `apply_entrypoint` + `apply_non_interactive_flags` + `prompt_delivery` + `prompt_arg_conventions` (4 prompt-related methods) where today there are `apply_non_interactive` + `prompt_delivery` + `validate_final_args` (3). The count goes up by one, but each method does exactly one thing and each has a single call site in the generic pipeline — which is the DRY win.

### Trade-off: the extractor has to be thorough

The extractor must handle all the argv shapes today's `find_prompt_location` handles: `-p value`, `--prompt value`, `-p=value`, `--prompt=value`, positional after entrypoint, positional with known value-taking flags skipped, etc. Getting this wrong breaks direct-path invocations. Mitigation: the extractor is the consolidation of `extract_user_prompt` + `find_prompt_location` + `strip_prompt_from_args`, all of which already handle these cases today. The refactor is consolidation, not net-new argv parsing.

## Test Plan

### Unit tests

1. **`PromptSource` basic behavior** — `as_inline`, `is_none`, round-trip.
2. **`extract_prompt_source_from_passthrough`, per provider** — parameterized across all seven providers covering:
   - `claudine <provider>` (no prompt → `None`)
   - `claudine <provider> "hello"` (positional → `Inline("hello")`, args cleaned)
   - `claudine <provider> --prompt "hello"` (explicit flag → `Inline("hello")`, args cleaned)
   - `claudine <provider> --prompt=hello` (inline form)
   - `claudine <provider> -p hello`
   - `claudine <provider> --model gpt foo bar` (value-taking flag ahead of positional)
   - `claudine <provider>` with stdin piped → `InheritStdin`
3. **Pipeline assembly per provider, direct path** — verify that after running the full pipeline on a direct-path invocation, the resulting `child_args` matches a golden snapshot byte-for-byte (same as today's behavior).
4. **Pipeline assembly per provider, composition path** — verify that after running the full pipeline on a composition-path invocation (empty starting args + inline prompt), the resulting `child_args` matches the same golden snapshot as the direct path.
5. **Non-interactive + no prompt + no stdin** — generic bail. Parameterize across all providers; all should fail with the same error.
6. **Interactive + no prompt** — allowed. Generic pass.
7. **`prompt_arg_conventions`** — unit tests per provider confirming the conventions returned match the provider's native CLI.

### Integration tests

1. **`claudine sequence file.md --<provider>`** for all providers that support composition (Claude, Codex, Gemini, Kimi, OpenCode, Qwen, Goose). This is the test that would have caught the Gemini/Qwen drift originally. Use a mock binary so no real provider is invoked.
2. **`claudine <provider> "prompt text"`** direct path for all providers. Golden snapshot of child argv.
3. **`claudine <provider>` (interactive)** for all providers. Verify no prompt is injected.
4. **`claudine <provider> --prompt "text"` vs `claudine <provider> "text"`** — both should produce identical child argv after the pipeline, for every provider that supports `--prompt`.

### Regression tests to preserve

- All 35 `wrap::profile::tests` currently pass and cover the per-provider `apply_non_interactive` behavior. Rewrite them to target `apply_entrypoint`, `apply_non_interactive_flags`, and `prompt_delivery` separately. The *test count* goes up (more focused tests), but the *concepts tested* stay the same.
- The nine `require_prompt_or_stdin` helper tests from Option 1 are deleted (the helper is gone).
- `extract_user_prompt_finds_first_non_switch` moves to `extract_prompt_source_from_passthrough_finds_positional`.
- `strip_prompt_from_args_preserves_output_last_message_pair_for_codex` moves to a test on `extract_prompt_source_from_passthrough` for Codex.

## Rollout Plan

Because the refactor touches the `WrapperProfile` trait and every provider impl, it is not worth shipping behind a feature flag. Single-PR migration.

1. **Branch off current state** (post-Option-1).
2. **Add types and helper functions without changing trait** — introduce `PromptSource`, `extract_prompt_source_from_passthrough`, `require_prompt_or_stdin` (the new generic one). Unit-test them against the current behavior in isolation.
3. **Add new trait methods with default implementations** — `apply_entrypoint`, `apply_non_interactive_flags`, `prompt_arg_conventions`. Defaults forward to the old `apply_non_interactive` for one commit so nothing breaks.
4. **Migrate one provider at a time, smallest first** — Kimi → Claude → Codex → Goose → OpenCode → Gemini → Qwen. After each provider:
   - Run `cargo test -p claudine-cli --bin claudine wrap::` (should stay green).
   - Add a composition integration test for that provider.
   - Verify golden-snapshot for direct and composition paths match.
5. **Migrate call sites** — `wrap/mod.rs` direct path and `wrap/composition.rs` start using the new pipeline shape. This is the commit that actually changes behavior; previous commits are prep.
6. **Remove old trait methods and their defaults** — `apply_non_interactive` and `validate_final_args` are deleted. `strip_prompt_from_args`, `find_prompt_location`, `extract_user_prompt` are deleted (replaced by the new extractor).
7. **Update documentation** — `claudine/docs/topics/composition.md` and the trait doc comments. Delete the load-bearing "NOTE: prompt validation is deferred..." comments (they are no longer needed — the trait shape makes deferral automatic).

Approximate diff size, estimated from inventory:

| File | Lines added | Lines removed |
|------|-------------|---------------|
| `wrap/profile.rs` | ~400 | ~350 (per-provider `apply_non_interactive` prompt logic, `validate_final_args`, old tests) |
| `wrap/mod.rs` | ~100 (new extractor, call-site rewrite) | ~200 (`extract_user_prompt`, `find_prompt_location`, `strip_prompt_from_args`, old direct-path wiring) |
| `wrap/composition.rs` | ~20 | ~30 |
| `wrap/profile.rs` tests | ~300 (new focused tests + snapshots) | ~250 (old `apply_non_interactive` and helper tests) |

Net: roughly neutral in line count, meaningfully smaller in conceptual surface area.

## Open Questions

1. **Should `PromptSource::InheritStdin` exist, or should stdin-piped invocations just go through `prompt_delivery` with a placeholder?** Leaning toward keeping it explicit — a piped-stdin direct-wrap is semantically different from "composition pipeline supplied a prompt body," and conflating them makes the `prompt_delivery` contract murkier.
2. **Should `apply_entrypoint` be split into interactive/non-interactive variants, or should it take a `non_interactive: bool` and branch internally?** Leaning toward the flag, because Claude is the only provider where the decision is actually conditional — all others inject entrypoints that work in both modes.
3. **Should `prompt_arg_conventions` be a method or data?** A `const` associated data type would be even more DRY but Rust's trait associated consts for arrays of `&'static str` are awkward. Method returning a struct is fine.
4. **Interactive direct-wrap with positional — is the positional a prompt?** Today some providers (Claude, Codex) treat an interactive positional as "a first turn message" and some (Gemini, Qwen) use `--prompt-interactive`. The new extractor needs to decide per-provider via `prompt_arg_conventions`. This is already known ground — no new research needed.
5. **Should the refactor also collapse `strip_prompt_tags_for_provider` into `extract_prompt_source_from_passthrough`?** The tag stripping is already cleanly separated (it runs after MCP tag lexing) and is not per-provider in the same way. Leave it alone.

## Appendix: Why this is worth doing

Option 1 (the shared `require_prompt_or_stdin` helper, already shipped) fixes the immediate bug and removes the four-condition duplication across five providers. That is valuable in isolation.

Option 2 fixes the *shape* that made Option 1 necessary. The test that would have caught Gemini and Qwen at their original PR is "run a composition pipeline against every provider end-to-end" — under Option 2 that test becomes trivial because the composition pipeline and the direct pipeline are structurally identical and the prompt-vs-args confusion is impossible to express.

The cost is one focused refactor PR that touches seven provider implementations and two call sites. The benefit is that the load-bearing comment in the trait (`"prompt validation is deferred to validate_final_args() because the prompt may not be in args yet"`) becomes impossible to get wrong, because the prompt is never in args at that stage under any circumstances.

When the next provider is added (Kimi 2? Mistral-code? Something new?), the author will implement `prompt_delivery` and `prompt_arg_conventions` and that is all. There is no three-phase contract to read, no load-bearing comment to notice, no composition-vs-direct gotcha to internalize. The trait shape teaches the correct invariant directly.
