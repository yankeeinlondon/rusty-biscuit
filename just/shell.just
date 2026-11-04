# On native Windows, the bin directory of the bash that runs recipes (Cygwin's
# `bin`, or Git for Windows' / MSYS2's `usr\bin`) goes first on the PATH that
# `just` hands to every recipe. A non-login bash otherwise keeps the Windows
# PATH order, where WinGet links (uutils coreutils), scoop shims, and System32
# can sort earlier and answer for `uname`, `env`, `sort`, `find`, or `cat`
# (uutils `uname -s` prints `Windows_NT`, which no recipe recognizes).
#
# Done in `just` rather than in a BASH_ENV hook because Cygwin's bash skips
# BASH_ENV for a `bash -c` started under SSH (it sources the bashrc files
# instead), which is every linewise recipe on a remote rig.
#
# What this does not reach: `just` resolves the bare `env` at the front of
# `set shell` from the PATH it inherited, so a foreign `env.exe` may still
# launch the recipe shell. That is harmless: it only sets one variable and
# starts `bash`, which it finds through this PATH. Nor does `export` reach the
# shells that evaluate load-time backticks; those stay limited to the tools
# `check-canonical` allows.
#
# `/proc/$$/winexename` is the Windows path of that bash, read with a builtin
# (`read` fails at its unterminated last line but still assigns, hence
# `|| true`). Inside WSL (the "accidental WSL" case `_ensure-native-bash`
# rejects) the file does not exist, the value is empty, and PATH is left alone.
# The backtick holds no backslash because Cygwin re-parses the command line a
# native program such as `just` passes it, collapsing `\\`; `?` stands in for
# the separator before `bash.exe`.
_recipe_bash_bin := if os_family() == "windows" { `exe=""; if [[ -r /proc/$$/winexename ]]; then read -r exe < /proc/$$/winexename || true; fi; printf '%s' "${exe%?bash.exe}"` } else { "" }

export PATH := if _recipe_bash_bin == "" { env("PATH") } else { _recipe_bash_bin + ";" + trim_start_match(env("PATH"), _recipe_bash_bin + ";") }
