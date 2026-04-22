**Important:** use the 'just' skill.

The 'status' recipe in @just/devops.just does what I want it to when no parameters are passed (e.g., it give you an overview of what is in the features and fixes directories of the given package area. However, I'd like you to add a parameter called "subject":

- the variable should be optional (aka, set to "" by default)
- when this variable is NOT set then the current behavior is run
- when a subject is provided then we will try to "match" that subject with one of the directories in fixes or featues.
- note: fixes and features have subdirectories `_completed` and `_unscheduled`:
    - `_completed` folder are tasks or features that have been completed and should be fully ignored for the 'status' recipe
    - `_unscheduled` folder are things which have been identified as needing be done but have not yet been
- a "match" will be achieved by:
    1. if a single _scheduled_ fix or feature matches as a substring (e.g., resides directly in the fixes or features directory but not in `_completed` or `_unscheduled`) then we will match immediately
    2. if more than one fix or feature matches then we'll use `fzf` to present the choices to the user; consider creating a helper recipe in `utils.just` for this as there are others like that in utils.just but it gives us another base for reuse.
    3. the choices given to `fzf` should add in a "NONE" option which will quit out of the process if chosen

## Output Behavior After a Match

When a subject matches a single directory, display only that matched item's directory listing using `eza` output for that single feature/fix directory. Do not show a filtered full status—show only the contents of the matched directory.

## Matching Algorithm

### Scheduled vs Unscheduled Handling

1. **First pass - Scheduled items only**: Match the subject against scheduled items (directories that reside directly in `fixes/` or `features/`, excluding `_completed` and `_unscheduled` subdirectories).
2. **Fallback - Include unscheduled**: If no match is found among scheduled items, or if multiple matches are found, include unscheduled items (from `_unscheduled` subdirectories) in the fzf options as a fallback.

### Match Resolution

- **Single scheduled match**: Subject matches exactly one scheduled item → display that directory's contents via `eza`.
- **Multiple matches or no scheduled match**: Present all matching candidates (including unscheduled as fallback) via `fzf` for user selection.
- **"NONE" option**: Always include a "NONE" option in fzf choices that exits the process when selected.

## Error/Failure Handling

- **fzf not installed**: If `fzf` is not available on the system, exit with error code 1 only when fzf would be invoked (i.e., when multiple matches require disambiguation). If a single scheduled match succeeds without needing fzf, the recipe succeeds.
- **fzf returns empty**: If the user presses Escape or closes fzf without making a selection, exit with error code 1.

## Helper Recipe

A helper recipe should be created in `utils.just` to encapsulate the fzf selection logic (including the "NONE" option). This provides a reusable base for other recipes that may need similar functionality.
