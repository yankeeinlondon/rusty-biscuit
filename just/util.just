
# Emit an OSC8 hyperlink: `$(just _link "text" "https://example.com")`
# Uses %b for display so embedded `\033[..m` SGR codes are interpreted.
# Uses BEL (\a) as the OSC string terminator instead of ESC+backslash so the
# captured output can be safely interpolated into later `echo -e` calls —
# a trailing backslash would otherwise merge with a following `\033...` and
# be consumed by echo's backslash interpretation.
_link display url:
    @printf '\033]8;;%s\a%b\033]8;;\a' '{{url}}' '{{display}}'

# Resolve a path to an absolute path; works for non-existent paths.
# `$(just _abs_path ./foo)`
_abs_path rel_path:
    #!/usr/bin/env bash
    set -euo pipefail
    path="{{rel_path}}"
    if [[ "${path}" = /* ]]; then
        printf '%s\n' "${path}"
    else
        printf '%s/%s\n' "$(pwd)" "${path#./}"
    fi


_title_case str:
    #!/usr/bin/env bash
    set -euo pipefail
    s="{{str}}"
    s="${s//-/ }"
    echo "${s}" | awk '{for(i=1;i<=NF;i++) $i=toupper(substr($i,1,1)) tolower(substr($i,2))}1'


# check whether file passed in has non-whitespace content in it
_file_has_content file:
    #!/usr/bin/env bash
    set -euo pipefail
    grep -q '[^[:space:]]' "{{file}}"

# Prompt the user with a yes/no question. Prints `y` or `n` to stdout and
# always exits 0 so callers can branch with `[[ $(just _confirm "...") == y ]]`
# without triggering just's "Recipe failed" error banner on a "no" answer.
_confirm question:
    #!/usr/bin/env bash
    set -euo pipefail
    printf '%b [y/N] ' "{{question}}" >&2
    read -n 1 ans; echo >&2
    [[ $ans =~ ^[Yy]$ ]] && echo y || echo n
