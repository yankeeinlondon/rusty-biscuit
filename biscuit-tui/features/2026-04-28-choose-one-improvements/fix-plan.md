---
phases: 6
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - spec.md
  - tech-design.md
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - cli/tests/completions_shell.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - lib/src/core/standalone.rs
  - cli/tests/keyboard_protocol.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
packages:
  - biscuit-tui
---
# Fix Plan — `question` CLI choose-one Regressions

## Context

Despite five rounds of review marking this feature "production ready", four
defects shipped:

1. **Hotkey-prefix completion is broken.** `question choose-one "[<TAB>` returns
   `[` and `[[` (zsh's command-name fallback) instead of `[CTRL+`, `[ALT+`,
   `[OPT+`. Unquoted `[<TAB>` is even worse — zsh treats `[` as a glob bracket
   and offers a flood of bad candidates.
2. **No flag completion after positional args.** Once 4+ positional options
   plus `--border`/`--border-label "..."` are on the line, `--<TAB>` produces
   nothing. Many real flags (`--csv`, `--list`, `--numeric-hot-keys`, …) remain
   undiscoverable.
3. **Hotkey badges never appear on bare modifier press.** Holding `Ctrl` or
   `Alt` alone does nothing. Badges only flash on a chord (e.g. `Ctrl+f`)
   because the bare-modifier path is dead code in this runner.

## Why the reviews missed these

- **Completions tests are string-grep on the generated script** —
  `cli/src/completions.rs:104-141` only asserts that `_question_hotkey_overlay`
  and `[CTRL+ [ALT+ [OPT+` appear *somewhere in the file*. No real shell ever
  ran the script.
- **Hotkey-display tests synthesize `KeyEvent { code: Modifier(LeftControl), kind: Press }` directly**,
  bypassing the runner's terminal setup. The library code path is unit-tested,
  but the runner path that would have surfaced the missing
  `KeyboardEnhancementFlags` push was never integration-tested.
- **No PTY/expectrl harness exists** for either layer.

## Root Causes

### Cause A — `prepare_terminal` never enables the kitty keyboard protocol

`lib/src/core/standalone.rs:412-420`:

```rust
fn prepare_terminal(fullscreen: bool) -> io::Result<()> {
    enable_raw_mode()?;
    if fullscreen {
        let mut out: Stdout = io::stdout();
        execute!(out, EnterAlternateScreen)?;
        out.flush().ok();
    }
    Ok(())
}
```

Without `PushKeyboardEnhancementFlags(REPORT_EVENT_TYPES | DISAMBIGUATE_ESCAPE_CODES)`,
crossterm cannot emit `KeyCode::Modifier(...)` press/release events. The
modifier-only branch in `choose_one.rs:502-515` (and the equivalent in
`choose_many.rs`) is therefore unreachable in production. The chord-fallback
deadline (`choose_one.rs:520-526`) only arms when a *chord* is pressed
(`Ctrl+letter`), so bare `Ctrl`/`Alt` produces no event whatsoever.

### Cause B — clap_complete's `_arguments -s -S -C` swallows post-`--` flags

The generated zsh script (visible via `question completions zsh`) declares:

```zsh
_arguments_options=(-s -S -C)
```

The `-S` flag tells `_arguments` to stop offering options after a literal `--`
appears on the line. Combined with the `'*::positional ... :_default'` catch-all,
zsh's `_arguments` has no way to know that more option flags remain. Result:
`-- <TAB>` after positional input returns nothing.

### Cause C — zsh hotkey-prefix overlay is fragile and incorrectly anchored

The overlay in `cli/src/completions.rs:73-88`:

```zsh
_question_hotkey_overlay() {
    _question "$@"
    if [[ "$PREFIX" == \[* ]]; then
        compadd -- '[CTRL+' '[ALT+' '[OPT+'
    fi
}
compdef _question_hotkey_overlay question
```

Multiple problems:

1. The standard clap_complete trailer ends with
   `if [ "$funcstack[1]" = "_question" ]; then _question "$@"; else compdef _question question; fi`.
   When the file is autoloaded via `fpath`/`compinit`, only the function body
   executes; trailing top-level statements run **at file-source time, not
   autoload time**. Depending on installation order our `compdef` may be
   overwritten by zsh's autoload re-binding.
2. Even when the wrapper fires, `_question "$@"` already invoked `_default` for
   the positional, which scheduled command/file fallbacks. By the time we call
   `compadd`, our candidates compete with `_default`'s pollution — and since
   `[`/`[[` are valid commands in PATH, they win the display.
3. The match `[[ "$PREFIX" == \[* ]]` correctly tests "starts with `[`" in
   shell, but `$PREFIX` may not contain the literal bracket the user typed in
   all contexts (notably when zsh strips a leading quote). The behaviour is
   shell- and version-dependent and was never verified.
4. Bash overlay has analogous problems but is less affected because bash
   completion is more forgiving.

## Remediation Phases

### Phase 1 — Spec amendments (no code)

Update `spec.md` and `tech-design.md` to make the following requirements
explicit:

1.1 **Modifier-only badge visibility.** Bare `Ctrl` / `Alt` press MUST surface
hotkey badges on terminals that support the kitty keyboard protocol. On
terminals that don't, the chord-fallback path covers chord presses; bare
modifiers may legitimately do nothing on those terminals. The runner MUST
attempt to enable the protocol and silently fall back if rejected.

1.2 **Required keyboard protocol flags.** `REPORT_EVENT_TYPES` is required for
modifier-only press/release. `DISAMBIGUATE_ESCAPE_CODES` is desirable so that
`Esc` can be distinguished from CSI sequence prefixes. Both must be popped on
restore.

1.3 **Completion contract.** State explicitly that:
- Typing `[` followed by `<TAB>` (quoted or unquoted) in any positional
  argument position MUST offer `[CTRL+`, `[ALT+`, `[OPT+` as the *only*
  completion candidates (no command/file fallback pollution).
- Tab completion MUST continue to suggest remaining option flags after a
  literal `--` separator, for the lifetime of the command line.

1.4 **Verification gates.** Mark all completion claims as requiring
PTY-driven shell tests (zsh + bash). Mark all keyboard-modifier claims as
requiring an integration test that exercises the real `prepare_terminal`
sequence under a PTY.

### Phase 2 — Real shell-integration test harness (write tests first)

Create `cli/tests/completions_shell.rs` (gated on a `shell-integration` feature
or `RUN_SHELL_TESTS=1` env so default `cargo test` stays fast).

Use the `expectrl` crate (already in the workspace per `Cargo.lock`) to:

2.1 Write the generated completion script to a temp `fpath` directory.
2.2 Spawn a real `zsh -i` with `FPATH` pointing at the temp dir, source
    `compinit -u`, then drive the prompt.
2.3 Assert candidates for each scenario:
    - `question choose-one "[<TAB>` → exactly `[CTRL+`, `[ALT+`, `[OPT+`.
    - `question choose-one [<TAB>` → same.
    - `question choose-one a b c d --border --border-label X --<TAB>` →
      candidate set must include `--csv`, `--list`, `--numeric-hot-keys`,
      `--no-filter`, `--required` etc. Not empty.
    - `question <TAB>` → subcommand list.
2.4 Repeat the relevant subset for `bash --norc -i` with `bash-completion`
    sourced.

The tests MUST fail against `main` today; they are the regression gate.

### Phase 3 — Runner fix: enable kitty keyboard protocol

3.1 Update `lib/src/core/standalone.rs::prepare_terminal`:

```rust
use crossterm::event::{
    KeyboardEnhancementFlags, PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
};

fn prepare_terminal(fullscreen: bool) -> io::Result<bool> {
    enable_raw_mode()?;
    let mut out: Stdout = io::stdout();
    if fullscreen {
        execute!(out, EnterAlternateScreen)?;
    }
    let kbd_pushed = execute!(
        out,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
        )
    )
    .is_ok();
    out.flush().ok();
    Ok(kbd_pushed)
}
```

3.2 Update `restore_terminal` to take the `kbd_pushed: bool` and pop the
    flags only if they were pushed.

3.3 Thread the bool through `run_standalone` / `run_standalone_with_chrome` /
    `drive_event_loop` so the restore path is symmetric. Use `defer!`-style
    cleanup (or a `Drop` guard) so a panic in the body still pops the flags.

3.4 Add a runner test that enables the flags under a PTY, sends a bare
    `Ctrl` press via the kitty protocol bytes, and asserts the choose state's
    `current_hotkey_display` advances to `CtrlHeld`. Use `expectrl` + raw
    bytes; do not stub crossterm.

3.5 Add a degraded-terminal test (PTY that rejects the push) asserting we
    don't panic and that chord fallback still works.

### Phase 4 — Completion fix: hotkey-prefix overlay rewrite

The current overlay is layered on top of `_question`. We replace it with a
dedicated *positional context* completer so candidates aren't competing with
`_default`.

4.1 **zsh:** Generate two functions:

```zsh
# Replaces the catch-all `:_default` for choose-one/choose-many positionals.
_question_choice_positional() {
    if [[ "$PREFIX" == \[* || -z "$PREFIX" ]]; then
        local -a candidates
        candidates=( '[CTRL+' '[ALT+' '[OPT+' )
        _describe -t hotkey-prefix 'hotkey prefix' candidates
        return
    fi
    _default
}
```

Then post-process the generated `_question` script (in
`completions::write_completions`) to:

- Replace `'*::positional ...:_default'` → `'*::positional ...:_question_choice_positional'`
  inside the `(choose-one)` and `(choose-many)` cases only.
- Remove `-S` from `_arguments_options=(-s -S -C)` → `_arguments_options=(-s -C)`
  so the post-`--` flag list keeps working.

Both edits are simple `str::replace` calls, keyed off the surrounding context
to avoid global rewrites.

4.2 **bash:** Hook into the existing `_question` function via
`compopt -o nospace` and a wrapper that pre-empts `[*` words *before* delegating
to clap's machinery. Concretely:

```bash
_question_complete() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    if [[ "$cur" == \[* ]]; then
        COMPREPLY=( $(compgen -W "[CTRL+ [ALT+ [OPT+" -- "$cur") )
        return 0
    fi
    _question "$@"
}
complete -F _question_complete -o nosort -o bashdefault -o default question
```

This stops bash's path/command fallback from contributing `[`/`[[`.

4.3 **fish/powershell/elvish:** Document as unsupported for the hotkey-prefix
overlay; fall back to plain clap_complete output.

4.4 Drop the `compdef _question_hotkey_overlay question` line — the fix is
inside the standard `_question` flow now, so no second registration is needed.

4.5 Update `completions.rs` unit tests:
- Keep the string-existence asserts as smoke tests.
- Add asserts that `-S` is *not* in the post-processed `_arguments_options`.
- Add asserts that the choose-one/choose-many positional rules reference
  `_question_choice_positional`, not `_default`.
- Mark these as insufficient on their own; the Phase 2 PTY tests are the real
  gate.

### Phase 5 — Documentation & install instructions

5.1 Update `cli/README.md` (or equivalent) with explicit shell install steps:

```sh
# zsh
question completions zsh > "${fpath[1]}/_question"
# Restart shell or: autoload -U compinit && compinit

# bash
question completions bash > /usr/local/etc/bash_completion.d/question
```

Note that the `question` binary must be installed *before* sourcing the
completion script (clap_complete calls `_question` against the binary's
`--help`).

5.2 Document the keyboard-protocol behaviour in `docs/components/choose_one.md`
and `choose_many.md`: bare modifier display works on Kitty, WezTerm, Ghostty,
foot, Alacritty (≥ 0.13), and modern iTerm2; on others, badges flash only on
chord press.

### Phase 6 — Verification before claiming "fixed"

Per the spec's verification gate (Phase 1.4):

6.1 Run the new PTY tests (`RUN_SHELL_TESTS=1 cargo test -p tui-chrome-cli`)
    and capture transcripts in the PR.
6.2 Manually verify in iTerm2 (user's environment): all six bug scenarios
    from the report, plus a regression sweep against the existing spec.
6.3 Capture a fresh asciicast or Snagit screenshot showing badges appearing
    when bare `Ctrl`/`Alt` is held, and another showing post-`--` completion.

## Out of Scope

- Adding new completion candidates for option *values* (e.g. `--border-style`
  glyphs are already enum-completed by clap; no new work needed).
- Rewriting the entire chord/keybinding model. We only wire the missing
  protocol push and verify the existing modifier-only path now reaches its
  receiver.

## Risks

- **Kitty protocol push may print stray bytes on terminals that ignore CSI
  `>1u` instead of rejecting it cleanly.** Mitigation: gate the push behind a
  capability check via `biscuit-terminal`'s detector when available, otherwise
  push optimistically and rely on the `is_ok()` flag-pushed return to decide
  whether to pop.
- **`_arguments` post-processing is fragile across clap_complete versions.**
  Mitigation: pin the regex to anchored substrings (`'*::positional ... :_default'`)
  and add a runtime assertion that the pre-processed script contained the
  expected anchor; fail loudly on mismatch.
- **expectrl PTY tests are slow and can flake.** Mitigation: `cargo-nextest`
  with retries (already in workspace), generous timeouts, and gate behind an
  env var so default `cargo test` stays green.

## Acceptance Criteria

A reviewer may mark this remediation complete only when:

1. PTY tests for all four reported bugs pass on zsh and bash.
2. Manual iTerm2 verification (with screenshots or asciicast) confirms bare
   `Ctrl`/`Alt` shows badges and `-- <TAB>` lists remaining flags.
3. `spec.md` and `tech-design.md` reflect the new requirements (Phase 1).
4. No existing test regresses.
5. README/install docs explain the shell-completion install steps.

## Suggested Execution Order

```
Phase 1 (spec)          ──┐
                          ├─→ Phase 2 (PTY tests, RED)
                          │
Phase 3 (runner) ─────────┤
                          ├─→ Phase 6 (verification)
Phase 4 (completions) ────┤
                          │
Phase 5 (docs) ───────────┘
```

Phases 3 and 4 are independent and parallelisable. Phase 2 must land first so
the fixes have a regression gate.
