use super::*;

#[test]
fn bash_script_registers_complete_callback() {
    let out = render(Shell::Bash);
    assert!(out.contains("_claudine_complete()"));
    assert!(out.contains("claudine __complete --current"));
    assert!(out.contains("complete"));
    assert!(out.contains("-F _claudine_complete claudine"));
    assert!(out.ends_with('\n'));
}

#[test]
fn bash_script_forwards_full_argv_to_engine() {
    let out = render(Shell::Bash);
    // The engine needs the full argv (binary + subcommand + flags +
    // partial token) to classify; the bash adapter must pass all of
    // `COMP_WORDS` as the trailing positional.
    assert!(out.contains("\"${COMP_WORDS[@]}\""));
    assert!(out.contains("$COMP_CWORD"));
}

#[test]
fn bash_script_falls_back_to_default_completion() {
    let out = render(Shell::Bash);
    // `-o bashdefault -o default` lets bash attempt its native
    // completion (usually filenames) when our function leaves
    // COMPREPLY empty. This is how non-targeted argument positions
    // keep behaving as if completion were unset.
    assert!(out.contains("-o bashdefault"));
    assert!(out.contains("-o default"));
}

#[test]
fn zsh_script_registers_compdef_directive() {
    let out = render(Shell::Zsh);
    assert!(out.starts_with("#compdef claudine"));
    assert!(out.contains("_claudine()"));
    assert!(out.contains("claudine __complete --current"));
    assert!(out.contains("compdef _claudine claudine"));
    assert!(out.ends_with('\n'));
}

#[test]
fn zsh_script_forwards_words_and_current_to_engine() {
    let out = render(Shell::Zsh);
    assert!(out.contains("\"${words[@]}\""));
    assert!(out.contains("$((CURRENT - 1))"));
}

#[test]
fn zsh_script_suppresses_compadd_prefix_filter() {
    let out = render(Shell::Zsh);
    // The engine returns fuzzy/substring matches (e.g. `plan` matches
    // `prompts/plan.md`). Without `-U`, compadd drops any candidate
    // whose string doesn't start with what the user typed, silently
    // erasing the whole point of substring completion.
    assert!(
        out.contains("compadd -U"),
        "zsh script must pass `-U` to compadd so fuzzy matches survive: {out}"
    );
}

#[test]
fn zsh_script_forces_menu_mode_for_substring_matches() {
    let out = render(Shell::Zsh);
    // When substring candidates have no common prefix with what the
    // user typed, zsh's default behavior is to insert the unambiguous
    // common prefix — which is empty — effectively erasing the typed
    // text. Forcing menu mode shows the candidates as a selectable
    // list instead.
    assert!(
        out.contains("compstate[insert]=menu"),
        "zsh script must force menu completion so the typed text is not \
         erased by an empty common-prefix insert: {out}"
    );
}

#[test]
fn zsh_script_autoloads_compinit_when_compdef_missing() {
    let out = render(Shell::Zsh);
    // Users frequently `source <(claudine completions zsh)` from rc
    // files that run before `compinit`. The script must cope with that
    // by autoloading the completion system on demand; without the
    // guard, `compdef _claudine claudine` silently fails and the
    // user's TAB presses fall through to generic command-name /
    // filename completion.
    assert!(
        out.contains("$+functions[compdef]"),
        "zsh script must guard `compdef` availability before calling it: {out}"
    );
    assert!(
        out.contains("autoload -Uz compinit"),
        "zsh script must autoload compinit when compdef is missing: {out}"
    );
}

#[test]
fn zsh_script_falls_back_to_files_on_empty_candidates() {
    let out = render(Shell::Zsh);
    assert!(
        out.contains("_files"),
        "zsh script must fall back to `_files` when the engine emits \
         no candidates: {out}"
    );
}

#[test]
fn fish_script_registers_complete_function() {
    let out = render(Shell::Fish);
    assert!(out.contains("function __claudine_complete"));
    assert!(out.contains("claudine __complete --current"));
    assert!(out.contains("complete -c claudine"));
    assert!(out.contains("-a '(__claudine_complete)'"));
    assert!(out.ends_with('\n'));
}

#[test]
fn fish_script_allows_file_completion_fallback() {
    let out = render(Shell::Fish);
    // The `-f` registration prevents fish from mixing native files into
    // Claudine-owned slots. The function restores native file completion
    // only when the engine emits no candidates.
    assert!(out.contains("set -l candidates"));
    assert!(out.contains("if test (count $candidates) -gt 0"));
    assert!(out.contains("printf '%s\\n' $candidates"));
    assert!(out.contains("__fish_complete_path $current_partial"));
    assert!(
        out.contains("complete -c claudine -f -a '(__claudine_complete)'"),
        "fish script must disable automatic file completion and provide \
         its own fallback: {out}"
    );
}

#[test]
fn fish_script_computes_argv_from_commandline() {
    let out = render(Shell::Fish);
    // commandline -opc gives prior tokens, -ct gives the current
    // partial — the adapter has to reassemble a full argv from both.
    assert!(out.contains("commandline -opc"));
    assert!(out.contains("commandline -ct"));
}

#[test]
fn powershell_script_registers_native_completer_for_claudine() {
    let out = render(Shell::PowerShell);
    assert!(
        out.contains("Register-ArgumentCompleter -Native -CommandName 'claudine', 'claudine.exe'"),
        "powershell script must register for both spellings of the binary: {out}"
    );
    assert!(
        !out.contains("COMPLETE"),
        "powershell script must not route through the legacy CompleteEnv bootstrap: {out}"
    );
}

#[test]
fn powershell_script_passes_engine_args_in_one_splatted_array() {
    let out = render(Shell::PowerShell);
    // A bare `--` written on the command line would be PowerShell's
    // end-of-parameters token; inside a splatted array it reaches the
    // engine verbatim.
    assert!(
        out.contains("@('__complete', '--current', ($argv.Count - 1), '--') + $argv.ToArray()"),
        "powershell script must build the engine argv as one array: {out}"
    );
    assert!(out.contains("& $claudine.Source @engineArgs"));
}

#[test]
fn powershell_script_swallows_stderr_unless_debugging() {
    let out = render(Shell::PowerShell);
    assert!(out.contains("if ($env:CLAUDINE_COMPLETION_DEBUG)"));
    assert!(out.contains("& $claudine.Source @engineArgs 2>$null"));
}

#[test]
fn powershell_script_offers_shortcut_registration() {
    let out = render(Shell::PowerShell);
    // Shortcut functions like `compose` complete as if the user typed
    // `claudine compose`, so the prefix must be spliced in after argv[0].
    assert!(out.contains("function Register-ClaudineShortcutCompletion"));
    assert!(out.contains("foreach ($word in $Prefix) { $argv.Add($word) }"));
    assert!(out.contains(".GetNewClosure()"));
}

#[test]
fn elvish_script_registers_arg_completer_for_claudine() {
    let out = render(Shell::Elvish);
    assert!(out.contains("set edit:completion:arg-completer[claudine] = {|@words| complete [] $words }"));
    assert!(out.contains("e:claudine __complete --current $current -- $@argv"));
    assert!(
        !out.contains("COMPLETE="),
        "elvish script must not route through the legacy CompleteEnv bootstrap: {out}"
    );
}

#[test]
fn elvish_script_explodes_the_sliced_word_list() {
    let out = render(Shell::Elvish);
    // `$@words[1..]` slices every word instead of the list, turning
    // `compose` into `ompose` before the engine sees it.
    assert!(!out.contains("$@prefix $@words[1..]"));
    assert!(out.contains("var typed = $words[1..]"));
}

#[test]
fn elvish_script_bypasses_prefix_matcher_only_for_engine_candidates() {
    let out = render(Shell::Elvish);
    // Elvish's default prefix matcher would drop fuzzy candidates such
    // as `slow` -> `prompts/slow.md`; every other completion must keep
    // the matcher that was installed before the script loaded.
    assert!(out.contains("set edit:completion:matcher[argument] = {|seed|"));
    assert!(out.contains("var fallback-matcher = $edit:match-prefix~"));
    assert!(out.contains("$fallback-matcher $seed"));
    assert!(out.contains("set engine-matched = $true"));
}

#[test]
fn elvish_script_falls_back_to_filename_completion() {
    let out = render(Shell::Elvish);
    assert!(out.contains("edit:complete-filename $words[-1]"));
    assert!(out.contains("2>$os:dev-null"));
    assert!(out.contains("has-env CLAUDINE_COMPLETION_DEBUG"));
}

#[test]
fn elvish_script_offers_shortcut_registration() {
    let out = render(Shell::Elvish);
    assert!(out.contains("fn shortcut {|name @prefix|"));
    assert!(out.contains("var argv = [claudine $@prefix $@typed]"));
}

#[test]
fn every_script_ends_with_a_newline() {
    for shell in [
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ] {
        assert!(
            render(shell).ends_with('\n'),
            "script for {shell:?} missing trailing newline",
        );
    }
}

#[test]
fn primary_shells_invoke_the_hidden_complete_subcommand() {
    // bash/zsh/fish must not rely on `COMPLETE=<shell> claudine`;
    // they shell out to `claudine __complete` directly so every
    // `<TAB>` reaches the current engine.
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        let out = render(shell);
        assert!(
            out.contains("claudine __complete --current"),
            "{shell:?} script must invoke the hidden `__complete` \
             subcommand; output was: {out}"
        );
        assert!(
            !out.contains("COMPLETE="),
            "{shell:?} script must not use the legacy `COMPLETE=<shell>` \
             bootstrap path; output was: {out}"
        );
    }
}

#[test]
fn bash_script_swallows_stderr_by_default() {
    let out = render(Shell::Bash);
    // Default branch (no CLAUDINE_COMPLETION_DEBUG set) must redirect
    // engine stderr to /dev/null so a transient panic does not corrupt
    // the user's prompt.
    assert!(
        out.contains("2>/dev/null"),
        "bash script must redirect engine stderr by default: {out}"
    );
    // The branch is gated on the debug env-var so users can opt in to
    // raw stderr without re-rendering the script.
    assert!(
        out.contains("CLAUDINE_COMPLETION_DEBUG"),
        "bash script must check CLAUDINE_COMPLETION_DEBUG so the \
         swallow-stderr default is overridable: {out}"
    );
}

#[test]
fn bash_script_exposes_stderr_under_debug_env() {
    let out = render(Shell::Bash);
    // Assert the conditional structure: an `if`/`else` branch keyed on
    // CLAUDINE_COMPLETION_DEBUG with a debug arm that does NOT redirect
    // stderr. The substring-level check on the env-var test is the
    // anchor; pair it with `else` so we know both branches exist.
    assert!(
        out.contains("if [ -n \"${CLAUDINE_COMPLETION_DEBUG:-}\" ]"),
        "bash script must branch on CLAUDINE_COMPLETION_DEBUG using \
         POSIX `[ ... ]` so a strict shell config cannot break it: {out}"
    );
    assert!(
        out.contains("else"),
        "bash script must keep the silent default arm alongside the \
         debug branch: {out}"
    );
    // The debug arm must invoke `__complete` without redirecting stderr.
    assert!(
        out.contains(
            "candidates=( $(command claudine __complete --current \
             \"$COMP_CWORD\" -- \"${COMP_WORDS[@]}\") )"
        ),
        "bash debug branch must invoke __complete without stderr \
         redirection: {out}"
    );
}

#[test]
fn zsh_script_swallows_stderr_by_default() {
    let out = render(Shell::Zsh);
    // Default branch (no CLAUDINE_COMPLETION_DEBUG set) must redirect
    // engine stderr to /dev/null so a transient panic does not corrupt
    // the user's prompt.
    assert!(
        out.contains("2>/dev/null"),
        "zsh script must redirect engine stderr by default: {out}"
    );
    assert!(
        out.contains("CLAUDINE_COMPLETION_DEBUG"),
        "zsh script must check CLAUDINE_COMPLETION_DEBUG so the \
         swallow-stderr default is overridable: {out}"
    );
}

#[test]
fn zsh_script_exposes_stderr_under_debug_env() {
    let out = render(Shell::Zsh);
    // Use POSIX `[ ... ]` not `[[ ... ]]` so a strict ZSH_NULLCMD
    // configuration cannot break the conditional.
    assert!(
        out.contains("if [ -n \"${CLAUDINE_COMPLETION_DEBUG:-}\" ]"),
        "zsh script must gate stderr exposure on \
         CLAUDINE_COMPLETION_DEBUG via POSIX `[ ... ]`: {out}"
    );
    assert!(
        out.contains("else"),
        "zsh script must keep the silent default arm alongside the \
         debug branch: {out}"
    );
    // The debug arm must capture candidates without redirecting stderr.
    assert!(
        out.contains(
            "candidates=(\"${(@f)$(command claudine __complete --current \
             $current -- \"${words[@]}\")}\")"
        ),
        "zsh debug branch must invoke __complete without stderr \
         redirection: {out}"
    );
}

#[test]
fn fish_script_swallows_stderr_by_default() {
    let out = render(Shell::Fish);
    // Default branch (no CLAUDINE_COMPLETION_DEBUG set) must redirect
    // engine stderr to /dev/null so a transient panic does not corrupt
    // the user's prompt.
    assert!(
        out.contains("2>/dev/null"),
        "fish script must redirect engine stderr by default: {out}"
    );
    assert!(
        out.contains("CLAUDINE_COMPLETION_DEBUG"),
        "fish script must check CLAUDINE_COMPLETION_DEBUG so the \
         swallow-stderr default is overridable: {out}"
    );
}

#[test]
fn fish_script_exposes_stderr_under_debug_env() {
    let out = render(Shell::Fish);
    // Fish uses `set -q VAR` to test for a defined variable.
    assert!(
        out.contains("if set -q CLAUDINE_COMPLETION_DEBUG"),
        "fish script must gate stderr exposure on \
         CLAUDINE_COMPLETION_DEBUG via `set -q`: {out}"
    );
    assert!(
        out.contains("else"),
        "fish script must keep the silent default arm alongside the \
         debug branch: {out}"
    );
    // The debug arm must invoke __complete without redirecting stderr.
    assert!(
        out.contains(
            "set candidates (command claudine __complete --current \
             $idx -- $argv_all)"
        ),
        "fish debug branch must invoke __complete without stderr \
         redirection: {out}"
    );
}
