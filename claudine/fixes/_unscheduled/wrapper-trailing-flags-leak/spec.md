# Wrapper flags after the positional prompt leak to the provider

A direct provider wrapper recognizes some of its own flags on either side of
the first positional argument, but not all of them. The flags it lifts back out
of the passthrough bucket after a positional are `-y`, `-i`, `--edit`,
`--repo`, `-q`, `--silent`, `-v`, `--perf`, and `--operation`/`--op`
(`extract_wrapper_flags_from_passthrough_with_boundary` in
`cli/src/commands/wrap/flags.rs`). `--dry-run`, `--timeout`, and
`--step-timeout` are not lifted. Written after the positional, they are passed
to the provider verbatim, and Claudine acts as if they were never given.

## Reproduction

```sh
claudine codex "hello" -i --dry-run
```

Expected: a dry-run preview; nothing launched. Actual: the real `codex` is
launched with `--dry-run` on its argv (observed 2026-09-28 while planning
`2026-09-18-edit-integration`). The same happens with:

```sh
claudine codex "hello" --timeout 5m     # no wall-clock timeout; codex gets --timeout 5m
claudine codex "hello" --step-timeout 5m
```

`--dry-run` is the dangerous one: a user asking for a preview gets a real,
possibly autonomous, provider run.

## Direction

Treat every wrapper-owned flag the same way on both sides of the positional,
or refuse a wrapper-owned flag that appears after it. Arguments after an
explicit `--` must stay opaque (see `wrapper-user-separator-forwarded`, which
is the neighboring defect for `--` itself).

## Not fixed in `2026-09-18-edit-integration`

That fix only needed `-i` and `--edit` recovered after a seed prompt, which
already works. Changing which flags are lifted is outside its scope (Rule 3).
