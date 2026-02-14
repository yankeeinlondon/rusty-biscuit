# CLAUDE.md Drift Review

You are performing the third step of drift orchestration:
review `@CLAUDE.md` and update it only if recent docs/skill changes imply it should change.

Package area context: `{{LIB_NAME}}` (`{{LIBRARY}}`, `{{CLI}}`)
Target docs reviewed earlier: `$(just readme_files) {{DOCS}} {{ARGS}}`

## Inputs

- Change summary from the docs refresh phase:

{{SUMMARY}}

## Rules

1. Use path-based inspection (`@CLAUDE.md`, affected package docs, related code) and keep reasoning concise.
2. Only modify `@CLAUDE.md` when there is a clear workflow/convention mismatch or missing guidance.
3. Prioritize semantic updates; avoid style-only edits.
4. Keep diffs minimal and specific.

## Task

1. Check whether updates made to the package docs/skill imply that `@CLAUDE.md` is outdated.
2. If yes, edit `@CLAUDE.md` with focused updates.
3. If no changes are needed, leave it untouched.

## Final Output

Report:
1. `CLAUDE.md` - `changed` or `unchanged`
2. Why
3. Evidence references (`path:line` where relevant)
