//! Shell-completion generation with choice-option hotkey-prefix support.
//!
//! The standard `clap_complete` script is emitted first, then shell-
//! specific post-processing and suffixes add the `[CTRL+`, `[ALT+`,
//! and `[OPT+` hotkey-prefix candidates for supported shells (`bash`,
//! `zsh`).
//!
//! Shells we don't have special hotkey-prefix handling for (fish,
//! powershell, elvish) get the plain `clap_complete` script.
//!
//! Spec reference: `2026-04-28-choose-one-improvements/spec.md` →
//! "Completions" section.

use std::io::{self, Write};

use clap::CommandFactory;
use clap_complete::Shell;

/// Writes a shell completion script to `writer`, augmented with the
/// hotkey-prefix handling for supported shells (`bash`, `zsh`).
pub fn write_completions<C: CommandFactory, W: Write>(
    shell: Shell,
    bin: &str,
    writer: &mut W,
) -> io::Result<()> {
    let mut cmd = C::command();
    let mut buf: Vec<u8> = Vec::new();
    clap_complete::generate(shell, &mut cmd, bin, &mut buf);

    let mut script = String::from_utf8(buf).expect("clap_complete produces valid UTF-8");

    match shell {
        Shell::Zsh => {
            strip_hidden_legacy_zsh(&mut script);
            zsh_post_process(&mut script);
            writer.write_all(script.as_bytes())?;
            writer.write_all(zsh_positional_completer().as_bytes())?;
        }
        Shell::Bash => {
            strip_hidden_legacy_bash(&mut script);
            writer.write_all(script.as_bytes())?;
            writer.write_all(bash_wrapper(bin).as_bytes())?;
        }
        Shell::Fish => {
            strip_hidden_legacy_fish(&mut script);
            writer.write_all(script.as_bytes())?;
        }
        _ => {
            strip_hidden_legacy_generic(&mut script);
            writer.write_all(script.as_bytes())?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// zsh
// ---------------------------------------------------------------------------

fn zsh_post_process(script: &mut String) {
    // Replace the catch-all :_default for choose-one/choose-many
    // positionals with our dedicated completer.  The exact descriptive
    // text only appears for those two subcommands, so a global replace
    // is safe.
    *script = script.replace(
        "'*::positional -- Option strings. Trailing positional arguments become the list of options when no explicit source flag is set:_default' \\",
        "'*::positional -- Option strings. Trailing positional arguments become the list of options when no explicit source flag is set:_question_choice_positional' \\",
    );

    // Remove `-S` from `_arguments_options` so that `-- <TAB>` after
    // positional arguments continues to suggest remaining flags.
    *script = script.replace(
        "_arguments_options=(-s -S -C)",
        "_arguments_options=(-s -C)",
    );

    // The two-value `--md PATH PROP` flag is rendered by clap_complete
    // with `_default` for both arg positions.  Replace the path arg's
    // action with `_files` so file completion fires; leave the property
    // name as a free-text message.
    *script = script.replace(
        "'*--md=[Path to a markdown file and frontmatter property name containing an array of options]:PATH:_default:PATH:_default'",
        "'*--md=[Path to a markdown file and frontmatter property name containing an array of options]:PATH:_files:PROP: '",
    );
}

fn strip_hidden_legacy_zsh(script: &mut String) {
    // zsh option specs are one line each, so a simple line filter
    // is sufficient.  bash specs span multiple lines per case clause
    // and need the dedicated [`strip_hidden_legacy_bash`] state machine.
    *script = script
        .lines()
        .filter(|line| {
            !line.contains("--options-from-file") && !line.contains("--options-from-dictionary")
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !script.ends_with('\n') {
        script.push('\n');
    }
}

fn zsh_positional_completer() -> &'static str {
    r#"
# Positional completer for choose-one/choose-many option arguments.
#
# This function is invoked by `_arguments` for every word that lands in
# the `*::positional` rest-pattern slot.  Because that rule consumes
# everything after the first positional argument, it also swallows
# option-value positions (e.g. the value following `--active-color` once
# any positional option has been typed).  Without intervention, those
# positions fall through to `_default` and complete file paths instead
# of the option's enumerated values.
#
# To restore correct behaviour, dispatch to the per-flag value action
# whenever the previous word is a known value-taking flag, and only
# then apply the existing `--*` flag overlay and `[` hotkey prefix
# checks.
_question_choice_positional() {
    local prev="${words[CURRENT-1]}"
    case "$prev" in
        --active-color)
            _values 'colour' grey green yellow red
            return
            ;;
        --sort)
            _values 'sort order' natural inverse asc desc
            return
            ;;
        --border-style)
            _values 'border style' \
                none rounded sharp bold double block thin-block \
                horizontal vertical line top bottom left right
            return
            ;;
        --label-position)
            _values 'label position' above below left right
            return
            ;;
        --label-convention|--value-convention)
            _values 'naming convention' \
                none camel-case pascal-case kebab-case snake-case \
                title-case caps lowercase
            return
            ;;
        --hotkey-badges)
            _values 'badge mode' auto always never ctrl alt
            return
            ;;
        --orientation)
            _values 'orientation' vertical horizontal
            return
            ;;
        --output)
            _values 'output mode' raw json null
            return
            ;;
        --file)
            _files
            return
            ;;
        --md)
            _files
            return
            ;;
        --csv|--list|--rows|--delimiter|--selected|--initial|--label|--border-label)
            _message 'value'
            return
            ;;
        --margin|--mt|--mb|--ml|--mr|--padding|-p|--pt|--pb|--pl|--pr|--max-selections|--min-selections|--height)
            _message 'integer'
            return
            ;;
    esac
    if [[ "$PREFIX" == --* ]]; then
        local -a option_candidates
        option_candidates=(
            --csv --list --rows --file --md
            --selected --required --min-selections --max-selections
            --delimiter --sort --no-filter
            --border --border-label --border-style
            --margin --mt --mb --ml --mr
            --padding --pt --pb --pl --pr
            --numeric-hot-keys
            --label --label-position --label-convention --value-convention
            --active-color --hotkey-badges --orientation
            --output --height --help
        )
        _describe -t option-flags 'option flag' option_candidates
        return
    fi
    if [[ "$PREFIX" == \[* ]]; then
        local -a candidates
        candidates=( '[CTRL+' '[ALT+' '[OPT+' )
        _describe -t hotkey-prefix 'hotkey prefix' candidates
        return
    fi
    _default
}
"#
}

// ---------------------------------------------------------------------------
// bash
// ---------------------------------------------------------------------------

fn strip_hidden_legacy_bash(script: &mut String) {
    // The legacy flags appear in the script in two forms:
    //
    // 1. As tokens inside the per-subcommand `opts="..."` string; we
    //    must remove those tokens *in place* without dropping the line.
    //
    // 2. As four-line `case "${prev}"` clauses:
    //
    //        --options-from-file)
    //            COMPREPLY=($(compgen -f "${cur}"))
    //            return 0
    //            ;;
    //
    //    Dropping only the label leaves an orphan body that `bash -n`
    //    flags as a syntax error.  Skip the whole clause by consuming
    //    lines through the closing `;;`.
    let mut out = String::with_capacity(script.len());
    let mut skip_until_terminator = false;
    for line in script.lines() {
        if skip_until_terminator {
            if line.trim_end().ends_with(";;") {
                skip_until_terminator = false;
            }
            continue;
        }
        let trimmed = line.trim_start();
        let is_case_label = trimmed.starts_with("--options-from-file)")
            || trimmed.starts_with("--options-from-dictionary)");
        if is_case_label {
            skip_until_terminator = true;
            continue;
        }
        let cleaned = scrub_legacy_tokens(line);
        out.push_str(&cleaned);
        out.push('\n');
    }
    *script = out;
}

fn scrub_legacy_tokens(line: &str) -> String {
    line.replace(" --options-from-file", "")
        .replace(" --options-from-dictionary", "")
}

fn bash_wrapper(bin: &str) -> String {
    format!(
        r#"
# Hotkey-prefix wrapper (added by `{bin} completions bash`).
# When the current word begins with `[`, return the hotkey prefixes
# directly without running the standard completion (which would add
# path/command fallback pollution).  Otherwise delegate to `_{bin}`.
_{bin}_choice_command_seen() {{
    local i
    for (( i=1; i<COMP_CWORD; i++ )); do
        case "${{COMP_WORDS[i]}}" in
            choose-one|choose-many)
                return 0
                ;;
        esac
    done
    return 1
}}
_{bin}_complete() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    if _{bin}_choice_command_seen; then
        if [[ "$cur" == \\[* ]]; then
            COMPREPLY=( "\\[CTRL+" "\\[ALT+" "\\[OPT+" )
            return 0
        fi
        if [[ "$cur" == \[* ]]; then
            COMPREPLY=( $(compgen -W "[CTRL+ [ALT+ [OPT+" -- "$cur") )
            return 0
        fi
    fi
    _{bin} "$@"
}}
if [[ "${{BASH_VERSINFO[0]}}" -eq 4 && "${{BASH_VERSINFO[1]}}" -ge 4 || "${{BASH_VERSINFO[0]}}" -gt 4 ]]; then
    complete -F _{bin}_complete -o nosort -o bashdefault -o default {bin}
else
    complete -F _{bin}_complete -o bashdefault -o default {bin}
fi
"#
    )
}

fn strip_hidden_legacy_fish(script: &mut String) {
    strip_hidden_legacy_generic(script);
}

fn strip_hidden_legacy_generic(script: &mut String) {
    *script = script
        .lines()
        .filter(|line| {
            !line.contains("options-from-file") && !line.contains("options-from-dictionary")
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !script.ends_with('\n') {
        script.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Minimal command surface used to exercise `write_completions`
    /// without depending on the full `Cli` struct from `main.rs`.
    #[derive(Parser)]
    #[command(name = "question")]
    struct TestCli {
        /// Option strings. Trailing positional arguments become the list
        /// of options when no explicit source flag is set.
        #[arg(value_name = "OPTIONS")]
        positional: Vec<String>,
    }

    // -----------------------------------------------------------------------
    // Smoke tests — string existence in generated scripts
    // -----------------------------------------------------------------------

    #[test]
    fn bash_output_contains_hotkey_wrapper() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Bash, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("_question_complete"),
            "bash wrapper function should be defined",
        );
        assert!(
            text.contains("[CTRL+ [ALT+ [OPT+"),
            "bash wrapper should advertise the hotkey prefixes",
        );
        assert!(
            text.contains("_question_choice_command_seen"),
            "bash wrapper should inspect completion words for the choice subcommands",
        );
        assert!(
            text.contains("complete -F _question_complete"),
            "bash wrapper must bind `complete` to the wrapper",
        );
    }

    #[test]
    fn zsh_output_contains_positional_completer() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Zsh, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("_question_choice_positional"),
            "zsh positional completer function should be defined",
        );
        assert!(
            text.contains("_describe -t hotkey-prefix 'hotkey prefix' candidates"),
            "zsh positional completer should _describe the hotkey prefixes",
        );
        assert!(
            text.contains("'[CTRL+' '[ALT+' '[OPT+'"),
            "zsh positional completer should advertise the hotkey prefixes",
        );
        assert!(
            text.contains("--numeric-hot-keys")
                && text.contains("--no-filter")
                && text.contains("--required"),
            "zsh positional completer should advertise option flags for -- prefixes",
        );
    }

    #[test]
    fn zsh_choice_positional_completer_does_not_offer_hotkeys_for_empty_prefix() {
        let script = zsh_positional_completer();
        assert!(
            !script.contains("|| -z \"$PREFIX\""),
            "empty option positions should not be treated as hotkey-prefix completions",
        );
    }

    #[test]
    fn zsh_post_separator_flag_overlay_lists_full_choose_flag_set() {
        let script = zsh_positional_completer();
        for flag in [
            "--csv",
            "--list",
            "--rows",
            "--file",
            "--md",
            "--selected",
            "--required",
            "--min-selections",
            "--max-selections",
            "--delimiter",
            "--sort",
            "--no-filter",
            "--border",
            "--border-label",
            "--border-style",
            "--margin",
            "--mt",
            "--mb",
            "--ml",
            "--mr",
            "--padding",
            "--pt",
            "--pb",
            "--pl",
            "--pr",
            "--numeric-hot-keys",
            "--label",
            "--label-position",
            "--label-convention",
            "--value-convention",
            "--active-color",
            "--hotkey-badges",
            "--orientation",
            "--output",
            "--height",
            "--help",
        ] {
            assert!(
                script.contains(flag),
                "zsh post-separator overlay should include {flag}",
            );
        }
    }

    /// Locate a `bash` that can actually run scripts.
    ///
    /// On Windows, plain `bash` on `PATH` resolves to the WSL stub in
    /// `System32` (which precedes Git's directories), and the stub *spawns*
    /// successfully even with no WSL distribution installed — it then fails
    /// every invocation, printing its error as UTF-16 to stdout. A
    /// spawn-error check therefore passes while `bash -n` fails with empty
    /// stderr. On Windows, probe only Git Bash's well-known install
    /// locations (present on GitHub's Windows runners): a real WSL bash on
    /// `PATH` would pass a version probe yet still fail `bash -n C:\...`
    /// because it needs `/mnt/c`-translated paths. Every candidate must
    /// print the `GNU bash` banner and exit 0.
    fn find_bash() -> Option<std::path::PathBuf> {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
        if cfg!(windows) {
            for var in ["ProgramFiles", "ProgramFiles(x86)"] {
                if let Some(root) = std::env::var_os(var) {
                    let git = std::path::PathBuf::from(root).join("Git");
                    candidates.push(git.join("bin").join("bash.exe"));
                    candidates.push(git.join("usr").join("bin").join("bash.exe"));
                }
            }
        } else {
            candidates.push(std::path::PathBuf::from("bash"));
        }
        candidates.into_iter().find(|bash| {
            std::process::Command::new(bash)
                .arg("--version")
                .output()
                .is_ok_and(|probe| {
                    probe.status.success()
                        && String::from_utf8_lossy(&probe.stdout).contains("GNU bash")
                })
        })
    }

    #[test]
    fn bash_script_passes_syntax_check() {
        // Regression guard: an earlier line-only filter for the hidden
        // legacy flags left orphan case bodies that `bash -n` rejected
        // with "syntax error near unexpected token `('".  The current
        // strip routine drops the entire case clause and scrubs the
        // legacy tokens out of the per-subcommand `opts` string, so the
        // resulting script must parse cleanly.
        let Some(bash) = find_bash() else {
            eprintln!("SKIP: no usable bash found (all candidates failed the GNU bash probe)");
            return;
        };
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Bash, "question", &mut buf).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "bash_syntax_check_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("question.bash");
        std::fs::write(&path, &buf).unwrap();
        let output = std::process::Command::new(&bash)
            .arg("-n")
            .arg(&path)
            .output()
            .expect("invoke bash -n");
        assert!(
            output.status.success(),
            "bash -n rejected the generated script:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn zsh_positional_completer_dispatches_enum_value_actions() {
        // Regression guard for the bug fixed alongside this test:
        // the zsh `*::positional` rule swallows option-value positions
        // after the first positional argument, so `--active-color <TAB>`
        // (post-positional) used to fall through to file completion.
        // The dispatcher embedded in `_question_choice_positional` must
        // route each enum-typed flag to its value action.
        let script = zsh_positional_completer();
        for flag_and_values in [
            ("--active-color)", "grey green yellow red"),
            ("--sort)", "natural inverse asc desc"),
            ("--label-position)", "above below left right"),
            ("--hotkey-badges)", "auto always never ctrl alt"),
            ("--orientation)", "vertical horizontal"),
            ("--output)", "raw json null"),
            ("--label-convention|--value-convention)", "camel-case"),
            ("--border-style)", "rounded"),
        ] {
            assert!(
                script.contains(flag_and_values.0),
                "positional completer must include case for {}",
                flag_and_values.0
            );
            assert!(
                script.contains(flag_and_values.1),
                "case for {} should advertise {}",
                flag_and_values.0,
                flag_and_values.1
            );
        }
    }

    #[test]
    fn generated_scripts_do_not_present_hidden_legacy_flags() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            let mut buf = Vec::new();
            write_completions::<TestCli, _>(shell, "question", &mut buf).unwrap();
            let text = String::from_utf8(buf).unwrap();
            assert!(
                !text.contains("options-from-file") && !text.contains("options-from-dictionary"),
                "{shell:?} completions must not present hidden legacy flags: {text}",
            );
        }
    }

    #[test]
    fn fish_output_skips_overlay_but_still_completes() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Fish, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(!text.contains("_hotkey_overlay"), "fish has no overlay");
        assert!(
            text.contains("complete -c question"),
            "fish should still emit the standard clap_complete script",
        );
    }

    // -----------------------------------------------------------------------
    // Phase 4: structural assertions on post-processed zsh script
    // -----------------------------------------------------------------------

    #[test]
    fn zsh_arguments_options_does_not_contain_dash_s() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Zsh, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        // The `-S` flag disables option suggestions after a literal `--`.
        // We remove it so that `-- <TAB>` after positional args still works.
        assert!(
            !text.contains("_arguments_options=(-s -S -C)"),
            "post-processed zsh script must not contain -S in _arguments_options",
        );
        assert!(
            text.contains("_arguments_options=(-s -C)"),
            "post-processed zsh script must contain the corrected _arguments_options",
        );
    }

    #[test]
    fn zsh_choose_positional_rules_use_choice_completer() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Zsh, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let old = "'*::positional -- Option strings. Trailing positional arguments become the list of options when no explicit source flag is set:_default' \\";
        assert!(
            !text.contains(old),
            "choose-one/choose-many positional rules must not reference _default",
        );
        let new = "'*::positional -- Option strings. Trailing positional arguments become the list of options when no explicit source flag is set:_question_choice_positional' \\";
        // The real CLI generates this line twice (choose-one + choose-many).
        // TestCli has a single positional arg, so we expect one occurrence
        // as a smoke test that the replacement is wired correctly.
        let count = text.matches(new).count();
        assert!(
            count >= 1,
            "positional rules must reference _question_choice_positional",
        );
    }
}
