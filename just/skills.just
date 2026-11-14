# Caller skills ("use-*") are written in each package area's
# `docs/use-skills/<skill>/` and installed into this host's user skill
# directory, where agents working in other repositories load them. Markdown
# files are composed with Darkmatter first, so a skill may interpolate or
# transclude from its package's docs; any other file is copied as written.

# Build this repository's `md` and print its path. Publishing composes with
# it rather than whatever `md` is on PATH, so the output matches this tree.
_repo_md:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --quiet -p darkmatter-cli --bin md 2>/dev/null
    echo "$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)/debug/md"

# Compose and install this area's caller skills. `dest` defaults to
# `~/.claude/skills`. Each skill directory there is replaced whole, so a file
# removed from the source is removed from the host as well.
_publish_use_skills dest="":
    #!/usr/bin/env bash
    set -euo pipefail
    src="docs/use-skills"
    if [ ! -d "${src}" ]; then
        echo "no ${src}/ in $(pwd); nothing to publish" >&2
        exit 1
    fi
    dest="${1:-}"
    if [ -z "${dest}" ]; then
        dest="${HOME:-${USERPROFILE:-}}/.claude/skills"
    fi
    md="$(just _repo_md)"
    mkdir -p "${dest}"
    for skill_dir in "${src}"/*/; do
        skill="$(basename "${skill_dir}")"
        if [ ! -f "${skill_dir}SKILL.md" ]; then
            echo "skipped ${skill}: no SKILL.md" >&2
            continue
        fi
        # Build the whole skill beside the destination, then swap it in, so
        # a failed composition leaves the installed copy untouched.
        staging="$(mktemp -d "${dest}/.${skill}.XXXXXX")"
        trap 'rm -rf "${staging}"' EXIT
        while IFS= read -r -d '' file; do
            rel="${file#"${skill_dir}"}"
            mkdir -p "${staging}/$(dirname "${rel}")"
            if [[ "${file}" == *.md ]]; then
                # `--deny all`: a warning (such as an undeclared name in a
                # transcluded docs example) fails publication instead of
                # silently publishing an evaluated example.
                "${md}" compose --frontmatter --deny all "${file}" > "${staging}/${rel}"
            else
                cp "${file}" "${staging}/${rel}"
            fi
        done < <(find "${skill_dir}" -type f -print0)
        rm -rf "${dest:?}/${skill}"
        mv "${staging}" "${dest}/${skill}"
        trap - EXIT
        echo -e "{{ GREEN }}published{{ RESET }} ${skill} → ${dest}/${skill}"
    done
