#!/usr/bin/env bash
# Regenerates this folder from the fixtures and a built `md`.
# Usage: generate.sh <path-to-md>   (run from the repository root)
# Only the fixture root and timing values are normalized; positions, trails,
# query parameters, control bytes, and stream contents are left as produced.
set -uo pipefail
MD=$1
HERE=$(CDPATH= cd "$(dirname "$0")" && pwd)
ROOT=$(pwd)
WORK=$(CDPATH= cd "$(mktemp -d)" && pwd -P)
trap 'rm -rf "$WORK"' EXIT
normalize() { sed -e "s#${WORK}#<root>#g" -e 's/"\(total_\)\{0,1\}ms": *[0-9.]*/"ms": <t>/g'; }
CDPATH= cd "$WORK" >/dev/null || exit 1
git init -q .
cp "$ROOT"/darkmatter/lib/tests/fixtures/composition_errors/{root,child}.md .

for target in plain markdown markdown-plus html json yaml toml; do
    case $target in plain) ext=txt ;; markdown|markdown-plus) ext=md ;; *) ext=$target ;; esac
    NO_COLOR=1 "$MD" validate refs root.md --output "$target" 2>&1 | normalize > "$HERE/report.$target.$ext"
done
# A real pseudo-terminal, no window: the styled report with its OSC 8 bytes.
script -q /dev/null sh -c "stty cols 120 rows 40; COLORTERM=truecolor '$MD' validate refs root.md" </dev/null 2>&1 | normalize | LC_ALL=C sed $'s/\x1b/\\\\e/g' > "$HERE/report.styled.escaped.txt"

cat > corpus.md <<'DOC'
# Plain Corpus

A soft-broken paragraph
that continues here
and ends here.

A long paragraph that keeps going well past any narrow width, so that it has to wrap more than once at forty columns and at eighty columns too, while under an unlimited width it stays on a single line from its first word all the way through to its very last word.

Run `md compose` with **bold words** and *italic words*.

```yaml
name: string(required)
```

| Name | Type |
| --- | --- |
| id | ulid |

---

> Quoted text keeps a visible marker.

- Item one
    - Nested item
- Item two

1. First
2. Second

See [the docs](https://example.com/docs) and <https://example.com/same>.

![A diagram](./diagram.png)

::disclosure
Disclosure summary
::details
Disclosure body.
::end-disclosure

```mermaid
flowchart LR
    A --> B
```
DOC
for width in 40 80 unlimited; do
    "$MD" render corpus.md --output plain --width "$width" 2>&1 | normalize > "$HERE/plain-corpus.$width.txt"
done

# status, stdout value count, and stderr bytes for the contract's cases.
{
    printf '%-58s %6s %12s\n' invocation status stderr-bytes
    probe() {
        "$MD" "$@" >"$WORK/.out" 2>"$WORK/.err" </dev/null
        status=$?
        printf '%-58s %6s %12s\n' "md $*" "$status" "$(wc -c <"$WORK/.err" | tr -d ' ')"
    }
    printf '# OK\n' > ok.md
    probe compose ok.md --output json
    probe compose root.md --output json
    probe compose root.md --output json -v --perf
    probe compose root.md --output toml
    probe compose root.md
    probe compose ok.md --output plain
    probe render ok.md --output json
    probe validate refs root.md --output json
    probe graph root.md --validate --output json
    probe hash ok.md --diff --output json
    probe get ok.md --raw
    probe --bogus-flag
} > "$HERE/streams-and-status.txt"
