#!/usr/bin/env bash
# Sync a just recipe from one justfile to all others in the monorepo.
#
# Usage: sync-recipe.sh <recipe-name> <source-justfile>
#
# Extracts the named recipe (including preceding comment lines) from the
# source justfile and replaces that same recipe in every other justfile
# that already contains it. Justfiles that don't have the recipe are skipped.
set -euo pipefail

BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'
RED='\033[31m'
GREEN='\033[32m'

RECIPE="${1:-}"
SOURCE="${2:-}"

if [ -z "$RECIPE" ] || [ -z "$SOURCE" ]; then
    echo -e "${RED}${BOLD}Usage:${RESET} sync-recipe.sh ${DIM}<recipe-name> <source-justfile>${RESET}"
    echo
    echo "  Examples:"
    echo "    sync-recipe.sh commit homelab/justfile"
    echo "    sync-recipe.sh lint   sniff/justfile"
    exit 1
fi

if [ ! -f "$SOURCE" ]; then
    echo -e "${RED}Error:${RESET} source file '${BOLD}${SOURCE}${RESET}' not found"
    exit 1
fi

# ---------------------------------------------------------------------------
# extract_recipe FILE NAME
#   Prints the recipe block: preceding comment lines + definition + body.
#   Body = all indented lines after the definition, up to the next
#   non-indented non-blank line (or EOF). Trailing blank lines are trimmed.
# ---------------------------------------------------------------------------
extract_recipe() {
    local file="$1" name="$2"
    awk -v name="$name" '
        BEGIN { found=0; comments=""; blanks="" }

        # --- before the recipe ---
        !found && /^#/ { comments = comments $0 "\n"; next }
        !found && /^[[:space:]]*$/ { comments = ""; next }
        !found && $0 ~ "^" name "([[:space:]:(]|$)" {
            found = 1
            printf "%s%s\n", comments, $0
            next
        }
        !found { comments = ""; next }

        # --- inside the recipe body ---
        found && /^[[:space:]]*$/ { blanks = blanks $0 "\n"; next }
        found && /^[[:space:]]/ {
            if (blanks != "") { printf "%s", blanks; blanks = "" }
            print
            next
        }
        # non-indented non-blank line = start of next recipe → stop
        found { exit }
    ' "$file"
}

# ---------------------------------------------------------------------------
# replace_recipe FILE NAME REPLACEMENT_FILE
#   Rewrites FILE to stdout, swapping the old recipe block with the contents
#   of REPLACEMENT_FILE.
# ---------------------------------------------------------------------------
replace_recipe() {
    local file="$1" name="$2" newfile="$3"
    awk -v name="$name" -v newfile="$newfile" '
        BEGIN { skip=0; comments=""; done_replacing=0 }

        # Already past the replaced recipe — pass through
        done_replacing { print; next }

        # Accumulate comment lines (might belong to our target recipe)
        !skip && /^#/ { comments = comments $0 "\n"; next }

        # Blank line while scanning — flush any accumulated comments
        !skip && /^[[:space:]]*$/ {
            if (comments != "") { printf "%s", comments; comments = "" }
            print; next
        }

        # Found the target recipe definition
        !skip && $0 ~ "^" name "([[:space:]:(]|$)" {
            skip = 1
            # Discard accumulated comments (old recipe header)
            # Print new recipe from file
            while ((getline line < newfile) > 0) print line
            close(newfile)
            next
        }

        # Some other definition — flush comments and print
        !skip {
            if (comments != "") { printf "%s", comments; comments = "" }
            print; next
        }

        # Skipping old recipe body (indented or blank lines)
        skip && /^[[:space:]]/ { next }
        skip && /^[[:space:]]*$/ { next }

        # First non-indented non-blank line after skip = next recipe
        skip {
            skip = 0
            done_replacing = 1
            print ""   # blank separator before next recipe
            print
        }

        END {
            # Flush any trailing comments that were not consumed
            if (!done_replacing && !skip && comments != "") {
                printf "%s", comments
            }
        }
    ' "$file"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
recipe_content=$(extract_recipe "$SOURCE" "$RECIPE")

if [ -z "$recipe_content" ]; then
    echo -e "${RED}Error:${RESET} recipe '${BOLD}${RECIPE}${RESET}' not found in ${SOURCE}"
    exit 1
fi

# Write extracted recipe to a temp file (avoids awk -v escaping issues)
tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT
echo "$recipe_content" > "$tmpfile"

echo -e "Syncing recipe ${BOLD}${RECIPE}${RESET} from ${DIM}${SOURCE}${RESET}"
echo

source_real=$(realpath "$SOURCE")
updated=0
skipped=0
missing=0

for justfile in $(find . -name justfile -not -path '*/target/*' | sort); do
    # Skip source file
    if [ "$(realpath "$justfile")" = "$source_real" ]; then
        continue
    fi

    # Skip justfiles that don't have this recipe
    if ! grep -q "^${RECIPE}[[:space:]:(]" "$justfile" 2>/dev/null; then
        missing=$((missing + 1))
        continue
    fi

    # Check if already identical
    existing=$(extract_recipe "$justfile" "$RECIPE")
    if [ "$existing" = "$recipe_content" ]; then
        echo -e "  ${DIM}skip ${justfile} (already up to date)${RESET}"
        skipped=$((skipped + 1))
        continue
    fi

    # Replace
    new_content=$(replace_recipe "$justfile" "$RECIPE" "$tmpfile")
    echo "$new_content" > "$justfile"
    echo -e "  ${GREEN}done${RESET} ${justfile}"
    updated=$((updated + 1))
done

echo
echo -e "Updated ${BOLD}${updated}${RESET}, skipped ${DIM}${skipped} (current)${RESET}, ${DIM}${missing} (no recipe)${RESET}"
