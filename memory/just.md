# Just Monorepo Learnings

## Using `mod` instead of `import` to avoid variable conflicts

When shared `.just` files (like `util.just`) define their own variables (e.g., `bold`, `reset`), importing them into another shared file with `import` causes duplicate-variable errors. Using `mod` creates a proper submodule namespace, avoiding the conflict entirely.

Example:

```just
# devops.just
mod util

# Recipes in devops.just can call util::_fzf_select
```

This is especially useful in a monorepo where multiple shared recipe files in `just/*.just` may define the same ANSI color variables or other helpers.

Note: `mod` was stabilized in just 1.31.0.

## Unscheduled Feature Priority Prefixes

Unscheduled features and fixes in `features/_unscheduled/` and `fixes/_unscheduled/` may be prefixed with a single digit priority: `{#}-{name}` where `1` is highest priority. The `just schedule` recipe will:
- Sort the fzf picker by priority (lowest number first)
- Strip the numeric prefix when moving to a scheduled `YYYY-MM-DD-{name}` directory

The `just status` recipe lists unscheduled features sorted by name, which respects the priority prefix order.
