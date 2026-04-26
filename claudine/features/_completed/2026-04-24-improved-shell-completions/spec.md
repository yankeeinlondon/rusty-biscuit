We had improved shell completions fairly recently for claudine's CLI but the result has been subpar and so we're going to
further adjust how we're handling this.

Here are the business rules we should use for claudine CLI's shell completions:

## Root Level Autocomplete

- when at the `claudine <tab>` level we should offer the following auto-completions:
    - composition subcommands
        - "compose" (compose subcommand)
        - "inline-compose" (inline-compose subcommand)
        - "sequence" (sequence subcommand)
    - "claude", "codex", "gemini", "opencode", "goose", "qwen", "kimi"... (aka, all of the wrapper subcommands)
    - "skills", "commands", "agents", "mcp" (aka, all of the **shared resources** subcommands)
    - "hooks" and "events" (subcommands for hooks and actions)
    - "sync", "uninstall", "providers", "logs", "completions"
    - "config" (config subcommand)
    - "init" (init subcommand; only show if there is NO configuration for current repo (if in repo), or if there is no user based configuration)
    - the only CLI switch at this point should be `--help`

## Composition Commands

When a user has now committed to a _composition_ command (compose, inline-compose, sequence) then the autocompletion changes:

- for `compose`
    - at the `claudine compose <tab>` point we will:
        - we are looking for high profile "prompt files"
        - "high profile" in this case just means that it is in one of the following directories:
            - "{repo root}/prompts"
            - "{package-area}/prompts" (if this is a monorepo and CWD is in a package area)
            - "{package}/prompts" (if this is a monorepo and CWD is in a discrete package directory)
            - "{repo root}/.claudine/prompts"
            - "~/.claudine/prompts"
        - > **Note:** Monorepo structure detection is delegated to the existing `sniff` tool/crate. The concepts of "package area" and "discrete package" map directly to `sniff`'s workspace and package detection logic. The completion engine does not implement its own monorepo heuristics.
        - so long as magic paths aren't used, we will support fuzzy matching for high profile prompt files
            - that means that if a user types `claudine compose plan<tab>` and there is a valid prompt file located at `prompts/plan.md` then this will match even though the directory name was not referenced by the caller.
        - we will support magic paths starting with `@` but when the user accepts a match then the magic path will be "resolved" to a relative path.
            - the resolution follows this explicit priority order:
                1. **Repo-local directories first** (repo root `prompts/`)
                2. **Repo `.claudine/` directory second** (repo root `.claudine/prompts/`)
                3. **User-global directory last** (`~/.claudine/prompts/`)
            - this means project-specific prompts take precedence over user-global ones
            - examples of resolved paths:
                - `@prompts/plan.md` if found in the repo's prompts directory would resolve to `prompts/plan.md`
                - `@prompts/plan.md` if found in the repo's ".claudine" directory would resolve to `.claudine/prompts/plan.md`
                - `@prompts/plan.md` if found first in the user's ~/.claudine/prompts directory will resolve to `~/.claudine/prompts/plan.md`
        - to be a **prompt** it must be a Markdown file that _does not_ have the 'prompt' property set
        - > **Note:** Frontmatter property matching is case-sensitive. Only exact lowercase keys (`prompt`, `sequence`) are recognized.
    - after a caller has typed 1, 2, or 3 characters (e.g., `claudine {ch}{optional(ch)}{optional(ch)}<tab>`) we will still only look in the high profile places for a full prompt file match, but we will start to allow "directory" matches where:
        - we will only look within the current repo (or CWD if not in a repo)
        - we are not matching on fuzzy criteria for directories, must be a starting substring match
    - when a match on a directory is selected we will move forward to include that directory and now autocomplete will operate exclusively in the directory that was selected
    - whenever a user has typed MORE than three characters but NOT selected a directory then:
        - we will show the "high profile" prompt file matches plus all directories which match but we will use fuzzy matching for directories now too
        - NOTE: we will NEVER show prompt files or directories which:
            - have a directory starting with a `_` character or a filename starting with a `_`.
            - match the `.gitignore` glob pattern
- for `sequence`:
    - follows almost exactly the same rules as `compose` except that:
        - we will match on YAML files which have a "sequence" property as a root property of the document
        - we will add the `docs` directory to the "high profile" area
        - we will add "agent skill" directories in the local repo:
            - `.claude/skills/**/*.md`
            - `.codex/skills/**/*.md`
            - `.gemini/skills/**/*.md`
            - `.opencode/skills/**/*.md`
            - `.goose/skills/**/*.md`
            - `.qwen/skills/**/*.md`
            - `.kimi/skills/**/*.md`
        - NOTE: we will NOT follow symbolic links for skill directories (this is to prevent finding duplicates when Claudine has symlinked to provide consistent skills across Agent CLI's)
- for `inline-compose`:
    - follows similar rules as `compose` except:
        - instead of filtering out markdown documents with a `prompt` frontmatter property set, we will do the opposite (aka, we ONLY show markdown files with a "prompt" file)
        - similar to "sequence" we will add `docs` and agent skill directories local to the repo:
            - `.claude/skills/**/*.md`
            - `.codex/skills/**/*.md`
            - `.gemini/skills/**/*.md`
            - `.opencode/skills/**/*.md`
            - `.goose/skills/**/*.md`
            - `.qwen/skills/**/*.md`
            - `.kimi/skills/**/*.md`

### Directory Traversal

The following rules govern how the completion engine traverses directories when scanning for files:

- **Recursion depth**: All listed directories are searched **recursively to unlimited depth**.
- **`.gitignore` rules**: `.gitignore` rules are applied at **every** directory level, consistent with standard Git behavior.
- **Symlink behavior**:
    - For non-skill directories, symbolic links **are followed** (standard filesystem behavior).
    - For agent skill directories (e.g., `.claude/skills/`, `.codex/skills/`), symbolic links are **not followed** (to prevent finding duplicates when Claudine has symlinked skills across Agent CLI providers).

### After the File Reference

With all three composition commands we start with the subcommand, the we add the file-reference to valid Markdown (or YAML) document, and then ... this section describes how shell completions should work after that.

- to a large degree the auto completion will revert back to the normal defaults at this point
- however, we will make an exception when a user types `claudine {compose-command} {file-reference} {variable}=<tab>`:
    - The `@` symbol is the **sole** signal that a variable value is a file-path reference. The completion engine treats **all** variable values as opaque strings unless the user explicitly types `@` at the beginning of the value.
    - No pre-knowledge of variable types is required by the engine. Schema-based typing may be added later but is **out of scope** for this implementation.
    - When a user starts the variable assignment with a `@` character we will assume the user is specifying a file path reference
        - we will now fuzzy search for any Markdown file which is in one of the following directories:
            - `{repo root}/docs`
            - `{repo root}/features`
            - `{repo root}/fixes`
            - `{repo root}/reviews`
            - if in a monorepo and CWD is in a discrete "package area":
                - `{package-area}/{docs|features|fixes|reviews}`
            - if in a monoprepo and CWD is in a discrete "package":
                - `{package}/{docs|features|fixes|reviews}`
        - we will not require that the user starts out by "quoting" the value but when we "resolve" a file path we will always wrap it in single quotes so that any potential spaces in the file path are handled gracefully:
            - both `claudine compose foobar.md spec=@spec<tab>` ad `claudine compose foobar.md spec="@spec.md<tab>` will both suggest the same way and both will resolve to a file in single quotes
            - that means if a user starts his tab completion with a double quote that double quote will be replaced with a single quote to match the closing single quote

## Other Commands

All commands outside of the _composition_ commands should behave in a more typical behavior to clap's normal provided behavior.

## Performance

- **Target**: "snappy" means completion resolution should complete in **under 100ms**.
- **Initial strategy — no caching**: Start with **no caching**, relying on aggressive short-circuiting instead:
    - Optimize traversal order (high-profile directories first).
    - Skip expensive YAML/Markdown parsing when possible by using file extensions and fast heuristics.
    - Short-circuit on prefix length:
        - **0–2 characters**: only search curated roots (high-profile directories).
        - **3+ characters**: perform a broad scan.
- **Fallback caching strategy**: If profiling reveals latency exceeding **150ms** in real-world usage, implement a stale-while-revalidate cache:
    - Cache location: `~/.cache/claudine/completions/<repo-hash>.json`
    - Cache metadata must include:
        - `repo_git_head` — the Git HEAD commit hash at scan time
        - `youngest_mtime` — the youngest mtime observed across all scanned directories
        - `scanned_at` — the timestamp when the scan completed
    - Invalidation on read:
        - If `repo_git_head` does **not** match the current HEAD, the cache is stale.
        - If any scanned directory has an mtime newer than `scanned_at`, the cache is stale.
    - Stale-while-revalidate behavior:
        - On cache read, if the cache is stale, **return the cached results immediately** while performing a full synchronous scan in the background to refresh the cache.
    - Atomic write:
        - Write the cache to a temporary file, then rename it to the final path to avoid readers observing a partial write.

## Documentation

The rules for how shell completions are setup needs to be fully documented. The plan and all reviews should ensure that a document claudine/docs/topics/shell-completions.md exists and adequately describes all of the expected business logic along with examples of how this business logic will behave. Where ever possible the documentation should also describe "why" the rules have been setup this way.

This document should also have a `## Performance Optimization` section which describes what we've done to optimize the performance of the shell completions.

## Help System Defect

Currently the claudine CLI's help system does NOT show `sequence` as a composition function! This needs to be fixed!
