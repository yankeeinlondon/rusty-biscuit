//! The sourceable shell integration printed by `wt --completions <shell>`: a
//! `wt` wrapper function plus clap's dynamic-completion registration.
//!
//! The wrapper is the only thing that can move the caller's shell. `wt`
//! prints protocol lines on stdout, and the wrapper acts on them after `wt`
//! exits:
//!
//! - `cd:<path>`: change to `<path>`; a failed change stops the wrapper.
//! - `remove-handoff:<token>`: after a successful change, run the fixed command
//!   `wt remove --handoff <token>`, passing the token as one argument.
//!
//! Every wrapper announces itself with `WT_SHELL_WRAPPER=1` on exactly the
//! invocations it makes, and never evaluates `wt`'s output as shell code.

use std::io::Write as _;

use clap_complete::Shell;
use clap_complete::env::Shells;

/// The shells `wt --completions` supports.
pub const SUPPORTED_SHELLS: [&str; 4] = ["bash", "zsh", "fish", "powershell"];

/// Placeholder in [`POWERSHELL_WRAPPER`] replaced by the `wt` executable path.
const EXE_PLACEHOLDER: &str = "__WT_EXE__";

// bash 3.2 (macOS's system bash) compatible. The loop only collects the
// protocol values; acting after it keeps the handoff run on the terminal's
// stdin rather than the here-document's.
const BASH_WRAPPER: &str = r#"wt() {
    local out line rc dest="" token=""
    out="$(WT_SHELL_WRAPPER=1 command wt "$@")"
    rc=$?
    if [ -n "$out" ]; then
        while IFS= read -r line; do
            case "$line" in
                cd:*) dest="${line#cd:}" ;;
                remove-handoff:*) token="${line#remove-handoff:}" ;;
                *) printf '%s\n' "$line" ;;
            esac
        done <<WT_OUTPUT
$out
WT_OUTPUT
    fi
    [ "$rc" -eq 0 ] || return "$rc"
    if [ -n "$dest" ]; then
        builtin cd -- "$dest" || return 1
    fi
    if [ -n "$token" ]; then
        WT_SHELL_WRAPPER=1 command wt remove --handoff "$token"
        return
    fi
    return 0
}
"#;

const FISH_WRAPPER: &str = r#"function wt
    set -l out (WT_SHELL_WRAPPER=1 command wt $argv)
    set -l rc $status
    set -l dest
    set -l token
    for line in $out
        switch $line
            case 'cd:*'
                set dest (string replace -r '^cd:' '' -- $line)
            case 'remove-handoff:*'
                set token (string replace -r '^remove-handoff:' '' -- $line)
            case '*'
                printf '%s\n' $line
        end
    end
    test $rc -eq 0; or return $rc
    if test -n "$dest"
        builtin cd -- $dest; or return 1
    end
    if test -n "$token"
        WT_SHELL_WRAPPER=1 command wt remove --handoff $token
        return
    end
    return 0
end
"#;

// The executable path is fixed at generation time because Windows Terminal
// installs its own `wt.exe` app alias, which a bare `wt` could resolve to.
// Windows PowerShell 5.1 decodes captured native output with the console code
// page, so the encoding is switched to UTF-8 for the call or non-ASCII `cd:`
// paths arrive corrupted. Moving only the location would leave the process's
// Win32 current directory, which is what holds a Windows directory open,
// inside the old worktree.
const POWERSHELL_WRAPPER: &str = r#"function wt {
    $wtExe = '__WT_EXE__'
    $previousWrapper = $env:WT_SHELL_WRAPPER
    $previousEncoding = [Console]::OutputEncoding
    $env:WT_SHELL_WRAPPER = '1'
    [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    try {
        $out = & $wtExe @args
        $rc = $LASTEXITCODE
    } finally {
        [Console]::OutputEncoding = $previousEncoding
        if ($null -eq $previousWrapper) {
            Remove-Item Env:WT_SHELL_WRAPPER -ErrorAction SilentlyContinue
        } else {
            $env:WT_SHELL_WRAPPER = $previousWrapper
        }
    }
    $dest = $null
    $token = $null
    foreach ($line in @($out)) {
        $text = [string]$line
        if ($text.StartsWith('cd:')) {
            $dest = $text.Substring(3)
        } elseif ($text.StartsWith('remove-handoff:')) {
            $token = $text.Substring(15)
        } else {
            Write-Output $text
        }
    }
    if ($rc -ne 0) {
        $global:LASTEXITCODE = $rc
        return
    }
    if ($dest) {
        try {
            Set-Location -LiteralPath $dest -ErrorAction Stop
        } catch {
            Write-Error $_
            $global:LASTEXITCODE = 1
            return
        }
        [Environment]::CurrentDirectory = (Get-Location -PSProvider FileSystem).ProviderPath
    }
    if ($token) {
        $env:WT_SHELL_WRAPPER = '1'
        try {
            & $wtExe remove --handoff $token
        } finally {
            if ($null -eq $previousWrapper) {
                Remove-Item Env:WT_SHELL_WRAPPER -ErrorAction SilentlyContinue
            } else {
                $env:WT_SHELL_WRAPPER = $previousWrapper
            }
        }
    }
}
"#;

/// The `wt` wrapper function for `shell`, or `None` for an unsupported shell.
///
/// `exe` is the `wt` executable the PowerShell wrapper calls; the POSIX
/// wrappers resolve `wt` through `PATH` with `command`.
pub fn wrapper(shell: Shell, exe: &str) -> Option<String> {
    match shell {
        Shell::Bash | Shell::Zsh => Some(BASH_WRAPPER.to_string()),
        Shell::Fish => Some(FISH_WRAPPER.to_string()),
        Shell::PowerShell => Some(
            POWERSHELL_WRAPPER.replace(EXE_PLACEHOLDER, &powershell_single_quoted(exe)),
        ),
        _ => None,
    }
}

/// The full sourceable script: an install comment, the wrapper, and the
/// dynamic-completion registration that calls back into `exe`.
pub fn script(shell: Shell, exe: &str) -> Option<String> {
    let (install, completer_name) = match shell {
        Shell::Bash => ("# add to ~/.bashrc:\n#   source <(wt --completions bash)", "bash"),
        Shell::Zsh => ("# add to ~/.zshrc:\n#   source <(wt --completions zsh)", "zsh"),
        Shell::Fish => (
            "# add to ~/.config/fish/config.fish:\n#   wt --completions fish | source",
            "fish",
        ),
        Shell::PowerShell => (
            "# add to $PROFILE:\n#   wt --completions powershell | Out-String | Invoke-Expression",
            "powershell",
        ),
        _ => return None,
    };
    let mut out = format!("# wt shell integration (cd wrapper + completions)\n{install}\n\n");
    out.push_str(&wrapper(shell, exe)?);
    out.push('\n');
    let mut registration = Vec::new();
    Shells::builtins()
        .completer(completer_name)?
        .write_registration("COMPLETE", "wt", "wt", exe, &mut registration)
        .ok()?;
    out.push_str(&String::from_utf8_lossy(&registration));
    Some(out)
}

/// Prints [`script`] for `shell` to stdout.
pub fn print(shell: Shell) {
    let exe = std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "wt".to_string());
    if let Some(text) = script(shell, &exe) {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(text.as_bytes());
        let _ = stdout.flush();
    }
}

/// The body of a PowerShell single-quoted string, where only `'` is special.
fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = "/opt/bin/wt";

    fn all_wrappers() -> Vec<(Shell, String)> {
        [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell]
            .into_iter()
            .map(|shell| (shell, wrapper(shell, EXE).expect("supported shell")))
            .collect()
    }

    fn words(text: &str) -> impl Iterator<Item = String> + '_ {
        text.split(|c: char| !(c.is_alphanumeric() || c == '-'))
            .filter(|word| !word.is_empty())
            .map(str::to_ascii_lowercase)
    }

    #[test]
    fn every_wrapper_sets_the_variable_for_each_wt_call() {
        for (shell, text) in all_wrappers() {
            let announcements = match shell {
                Shell::PowerShell => text.matches("$env:WT_SHELL_WRAPPER = '1'").count(),
                _ => text.matches("WT_SHELL_WRAPPER=1 command wt").count(),
            };
            assert_eq!(announcements, 2, "{shell:?} must announce both wt calls:\n{text}");
        }
    }

    #[test]
    fn every_wrapper_handles_both_protocol_lines() {
        for (shell, text) in all_wrappers() {
            assert!(text.contains("cd:"), "{shell:?}");
            assert!(text.contains("remove-handoff:"), "{shell:?}");
        }
    }

    #[test]
    fn every_wrapper_stops_when_the_directory_change_fails() {
        let checks = [
            (Shell::Bash, "builtin cd -- \"$dest\" || return 1"),
            (Shell::Zsh, "builtin cd -- \"$dest\" || return 1"),
            (Shell::Fish, "builtin cd -- $dest; or return 1"),
            (Shell::PowerShell, "Set-Location -LiteralPath $dest -ErrorAction Stop"),
        ];
        for (shell, check) in checks {
            let text = wrapper(shell, EXE).unwrap();
            assert!(text.contains(check), "{shell:?} lacks `{check}`");
            let cd_at = text.find(check).unwrap();
            let handoff_at = text.find("remove --handoff").unwrap();
            assert!(cd_at < handoff_at, "{shell:?} must change directory before the handoff");
        }
    }

    #[test]
    fn every_wrapper_runs_the_fixed_handoff_command_with_a_quoted_token() {
        let commands = [
            (Shell::Bash, "command wt remove --handoff \"$token\""),
            (Shell::Zsh, "command wt remove --handoff \"$token\""),
            // fish never word-splits a variable, so `$token` is one argument.
            (Shell::Fish, "command wt remove --handoff $token"),
            (Shell::PowerShell, "& $wtExe remove --handoff $token"),
        ];
        for (shell, command) in commands {
            assert!(wrapper(shell, EXE).unwrap().contains(command), "{shell:?}");
        }
    }

    #[test]
    fn no_wrapper_evaluates_output() {
        for (shell, text) in all_wrappers() {
            for word in words(&text) {
                assert!(
                    !matches!(word.as_str(), "eval" | "invoke-expression" | "iex" | "source"),
                    "{shell:?} wrapper uses `{word}`"
                );
            }
        }
    }

    #[test]
    fn powershell_wrapper_embeds_the_executable_and_sets_both_directories() {
        let text = wrapper(Shell::PowerShell, r"C:\Users\o'neil\bin\wt.exe").unwrap();
        assert!(text.contains(r"$wtExe = 'C:\Users\o''neil\bin\wt.exe'"));
        assert!(!text.contains(EXE_PLACEHOLDER));
        assert!(text.contains("[Environment]::CurrentDirectory ="));
        assert!(text.contains("[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)"));
    }

    #[test]
    fn script_appends_completion_registration() {
        let expected = [
            (Shell::Bash, "complete -o nospace"),
            (Shell::Zsh, "compdef _clap_dynamic_completer_wt wt"),
            (Shell::Fish, "complete --keep-order --exclusive --command wt"),
            (Shell::PowerShell, "Register-ArgumentCompleter -Native -CommandName wt"),
        ];
        for (shell, registration) in expected {
            let text = script(shell, EXE).unwrap();
            assert!(text.contains(registration), "{shell:?}:\n{text}");
            assert!(text.contains(EXE), "{shell:?} registration must call back into the exe");
            let wrapper_at = text.find(&wrapper(shell, EXE).unwrap()).unwrap();
            assert!(wrapper_at < text.find(registration).unwrap());
        }
    }

    #[test]
    fn unsupported_shells_produce_nothing() {
        assert!(script(Shell::Elvish, EXE).is_none());
    }
}
