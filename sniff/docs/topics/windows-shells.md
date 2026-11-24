# Windows shells and their `PATH`

On native Windows, a program the current process can find may still be
missing in the shell a user actually opens. PowerShell, Git Bash, and Cygwin
each build `PATH` their own way when they start. `sniff::programs::windows_shells`
finds which of those shells are installed and asks each one, without its
startup files, whether it can find a program.

```rust
use sniff::programs::windows_shells::{self, ShellLookup};
use std::path::Path;

for shell in windows_shells::detect() {
    let name = shell.kind.display_name();
    match windows_shells::probe(&shell, "claudine", Path::new(".")) {
        Ok(ShellLookup::Found) => println!("{name} finds it"),
        Ok(ShellLookup::Missing) => println!("{name} cannot find it"),
        Ok(other) => println!("{name} gave no answer: {other:?}"),
        Err(invalid) => eprintln!("{invalid}"),
    }
}
```

`detect()` returns an empty list on macOS, Linux, and WSL2: there the
current process's own `PATH` is the only one to check.

## Finding installed shells

`detect()` reads file metadata, the process environment, and two read-only
registry keys. It starts nothing. It reports at most one shell for each of
three families, in this order:

| Family | Chosen shell, first match wins |
| --- | --- |
| PowerShell | `pwsh` (PowerShell 7) from Sniff's executable index, then `powershell`, then `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe` |
| MSYS | Git Bash, then a standalone MSYS2 (see below) |
| Cygwin | The indexed `bash`, then each `rootdir` of `SOFTWARE\Cygwin\setup` (current user, then machine; 64-bit view, then 32-bit), then `%SystemDrive%\cygwin64` and `%SystemDrive%\cygwin` |

A `bash.exe` belongs to a family only when its runtime DLL sits beside it:
`msys-2.0.dll` for MSYS, `cygwin1.dll` for Cygwin. That is how the WSL
launcher `C:\Windows\System32\bash.exe`, which has neither, is left out.

The MSYS family checks these places in order:

1. The indexed `bash`, if it is Git Bash: its installation also holds
   `cmd\git.exe`.
2. `usr\bin\bash.exe` under the Git for Windows installation that `git`
   resolves to, using the same launcher layouts Sniff uses for Git itself.
3. `%ProgramFiles%\Git\usr\bin\bash.exe`.
4. The indexed `bash`, if it is a standalone MSYS2.
5. `%SystemDrive%\msys64\usr\bin\bash.exe`, then `%SystemDrive%\msys32\usr\bin\bash.exe`.

`%SystemDrive%` holds a drive such as `C:`. Sniff adds the root separator
(`C:\`) before joining, because `C:msys64` would mean "relative to the
current directory on drive C".

The executable index lists `PATH` first, so a shell on `PATH` wins over an
install layout of the same kind. A layout that is also the indexed `bash`
is still one shell.

## Asking a shell

`probe(shell, program, working_dir)` starts the shell's absolute executable
in `working_dir` and returns one of:

| Outcome | Meaning |
| --- | --- |
| `Found` | The lookup ran and found `program` |
| `Missing` | The lookup ran and found no such program |
| `TimedOut` | The shell did not answer within 2 seconds; its process tree was terminated |
| `SpawnFailed { kind }` | The shell could not be started |
| `Failed { exit_code }` | The shell ended without the lookup's answer; `None` means it was terminated |

The probe never runs `program` and never reads the user's startup code:

- PowerShell runs as `-NoLogo -NoProfile -NonInteractive -Command`, with
  module auto-loading off and a module-qualified
  `Get-Command -CommandType Application`.
- Bash runs as `--noprofile --norc -p -c 'type -P -- "$1" …' sniff-shell-lookup <program>`,
  with `BASH_ENV` and `ENV` removed from its environment. The program name
  travels as an argument, never as script text.
- Each script exits `0` for found and `3` for missing; any other status is
  `Failed`.
- Stdin is null, both output pipes are drained, and on Windows the child gets
  no console window. The shell otherwise inherits this process's
  environment, including `PATH`.

`program` must be a plain name of ASCII letters, digits, `.`, `_`, and `-`,
not starting with `-` or `.`. Anything else is `InvalidProgramName`, and no
shell is started.

The probe uses the same bounded runner as every other Sniff subprocess, so
the deadline, pipe draining, and tree cleanup follow
[process trees](process-trees.md).
