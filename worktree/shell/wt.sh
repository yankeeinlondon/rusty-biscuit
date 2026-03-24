# wt shell integration — cd wrapper + completions
#
# Source this once in your shell config, OR use the generated version:
#
#   Bash:  source <(wt --completions bash)   # add to ~/.bashrc
#   Zsh:   source <(wt --completions zsh)    # add to ~/.zshrc
#   Fish:  source (wt --completions fish | psub)  # add to config.fish
#
# The wrapper intercepts the "cd:" protocol from the wt binary so that
# "wt go <name>" can actually change the parent shell's working directory.

wt() {
    local output exit_code
    output="$(command wt "$@")"
    exit_code=$?
    (( exit_code != 0 )) && return $exit_code
    if [[ "$output" == cd:* ]]; then
        builtin cd -- "${output#cd:}"
    else
        [[ -n "$output" ]] && print -- "$output"
    fi
}
