# Worktree shell wrapper function
# Add this to ~/.bashrc or ~/.zshrc:
#   source /path/to/wt.sh
#
# This wrapper intercepts the "cd:" protocol from the wt binary
# to change the parent shell's working directory.

wt() {
    local output
    output="$(command wt "$@")" || { echo "$output"; return $?; }
    if [[ "$output" == "cd:"* ]]; then
        builtin cd "${output#cd:}" || return $?
    else
        [[ -n "$output" ]] && echo "$output"
    fi
}
