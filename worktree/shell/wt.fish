# Worktree shell wrapper function for Fish
# Add this to ~/.config/fish/config.fish:
#   source /path/to/wt.fish

function wt
    set -l output (command wt $argv 2>&1)
    set -l status_code $status

    if test $status_code -ne 0
        echo $output
        return $status_code
    end

    if string match -q 'cd:*' -- $output
        cd (string replace 'cd:' '' -- $output)
    else if test -n "$output"
        echo $output
    end
end
