# MCP Mode Spec Review

**Date:** 2026-03-10
**Scope:** Compare implementation against `docs/cli/mcp-mode.md` and `docs/cli/mcp-catalog.md`
**Status:** Post-refactor review (plan: `plans/2026-03-10.refactor-mcp.md`)

## Summary

The refactor plan has been substantially executed. The implementation now covers the core spec surface: tag lexing, tiered resolution, session composition, reactive bootstrap, the full CLI command set, validation, import/export, and runtime injection. The architecture is clean and well-decomposed across 10 library modules and a comprehensive CLI layer.

This review identifies remaining gaps, implementation issues, ergonomic improvements, and areas needing further research.

---

## 1. Functionality Gaps

### 1.1 `effective_defaults` replaces rather than merges

**Spec (mcp-catalog.md, line 19-21):** User-scoped and repo-scoped defaults are described as two independent lists. The init flow at lines 46-50 says "any options chosen at User scope" should be visible when choosing repo defaults, implying layering.

**Implementation (`defaults.rs:68-80`):** `effective_defaults()` returns repo defaults **instead of** user defaults when repo defaults exist. This means a user who sets user-scope defaults and then initializes repo defaults loses their user defaults entirely for that repo context.

**Impact:** Medium. A user with `brave-search` as a user default and `github` as a repo default would lose `brave-search` in that repo.

**Recommendation:** Consider whether `effective_defaults` should return `user_defaults ∪ repo_defaults` (union) or whether the current replacement semantics are intentional. The spec is ambiguous here — it says "repo defaults override user defaults" is not explicitly stated, but the init re-entry flow (line 46-50) implies awareness of both scopes. If replacement is intentional, document it clearly.

### 1.2 Missing `--strict` handling for ambiguous tags in non-interactive mode

**Spec (mcp-mode.md, lines 77-79):** "in non-interactive runs, ambiguity is treated as a hard error because there is no safe prompt path."

**Implementation (`wrap/mod.rs:250-261`):** The `resolve_ambiguous` closure returns `None` when `args.strict || non_interactive_requested || !prompt_is_interactive`, which correctly refuses to select. However, later at lines 284-292, ALL remaining ambiguous tags are treated as a hard error regardless of mode. This means ambiguous tags always fail — there's no path where they're silently dropped.

**Impact:** Low — the behavior is stricter than the spec, which is safer. But the spec says default interactive mode should prompt, and if the user dismisses the prompt (Esc/Cancel), the `Select::prompt().ok()` returns `None`, which becomes an ambiguous tag, which is then a hard error. This means the user can't cancel an ambiguous prompt without aborting the entire run.

**Recommendation:** In interactive non-strict mode, if the user cancels the disambiguation prompt, consider warning and dropping the tag rather than hard-erroring.

### 1.3 `claudine mcp` without subcommand calls `list` (spec-compliant)

**Spec (mcp-catalog.md, line 99):** Confirmed. Implementation at `mcp.rs:179` routes `None` to `run_list`. This is correct.

### 1.4 Init re-entry does not show user-level defaults when prompting for repo defaults

**Spec (mcp-catalog.md, lines 49-50):** "remind them what MCP servers are _always_ included at the user level" before repo-default selection.

**Implementation (`mcp.rs:293-309`):** The re-entry path prompts for repo defaults but does not display the current user defaults first. The `prompt_for_defaults` function at line 694 receives a `current` parameter but ignores it (`let _ = current;` at line 710).

**Impact:** Medium — UX gap. Users won't see their user-scope context when choosing repo defaults.

**Recommendation:** Use the `current` parameter to pre-select or at least display user defaults in the `MultiSelect` widget (via `with_default` or a header message).

### 1.5 `remove` command does not report remaining aliases when removing an alias

**Spec (mcp-catalog.md, lines 75-76):** "we will report that the referenced MCP server _still_ exists and any other aliases it may still have."

**Implementation (`mcp.rs:509-519`):** When an alias is removed, it reports the owner but does not list remaining aliases.

**Impact:** Low — minor UX gap.

### 1.6 `remove` without arguments doesn't list valid names

**Spec (mcp-catalog.md, lines 77-79):** When no name or alias is provided, "we will provide a list of valid names (with any aliases they have in parenthesis)."

**Implementation (`mcp.rs:504-507`):** Uses `prompt_for_server_query` which shows an interactive Select widget. This is functionally equivalent and arguably better UX, so this is fine.

### 1.7 Tag extraction only works for Codex, Gemini, and OpenCode

**Implementation (`wrap/mod.rs:571-578`):** `find_prompt_location` returns `None` for Claude and all other providers. This means `#tag` syntax in prompts won't work for those providers even if they support runtime injection in the future.

**Impact:** Low for now — Claude doesn't support runtime injection anyway. But this should be extended as provider support grows.

---

## 2. Implementation Issues

### 2.1 `current_repo_root` always returns `Some`

**File:** `mcp.rs:1027-1030`

```rust
fn current_repo_root() -> Result<Option<PathBuf>> {
    let cwd = std::env::current_dir()?;
    Ok(Some(resolve_repo_root(&cwd)))
}
```

This always returns `Some(...)` — `resolve_repo_root` presumably returns the cwd itself if no git root is found. The `Option` wrapper is misleading. Several callers check `repo_root.is_some()` to decide behavior, but that check will always be true.

**Impact:** Medium — could cause repo-scope defaults to be created/loaded in non-repo directories.

**Recommendation:** Check whether `resolve_repo_root` actually returns `None` for non-repo dirs. If not, this function should detect the git root properly (e.g., `git rev-parse --show-toplevel`) and return `None` when outside a repo.

### 2.2 Fingerprint uses BLAKE3 but spec mentions xxHash

**Spec (mcp-mode.md, line 61):** "hash the entire configuration JSON with xxHash (biscuit-hash library) and give it a hash name."

**Implementation:** `McpServer::fingerprint()` uses `biscuit_hash::blake3_hash` (types.rs:136), while `derive_server_name` uses `biscuit_hash::xx_hash` (types.rs:383) for the fallback name.

**Impact:** None functionally — BLAKE3 is fine for fingerprinting. But the spec and implementation disagree on which hash is used where. The spec talks about xxHash for the _name_ derivation case, and the implementation correctly uses xxHash there. The fingerprint (for dedup) using BLAKE3 is a reasonable implementation choice.

**Recommendation:** No code change needed, but the spec could clarify that fingerprinting and name-derivation use different hash functions.

### 2.3 `prompt_for_defaults` ignores `current` parameter

**File:** `mcp.rs:710`

```rust
let _ = current;
```

This is dead code. The `current` parameter is accepted but suppressed.

**Impact:** See gap 1.4 above.

### 2.4 `base_table` creates an unused `Alignment::Right` binding

**File:** `mcp.rs:1134`

```rust
let _ = Alignment::Right;
```

This looks like leftover development code.

**Impact:** None — but should be cleaned up.

### 2.5 `McpServer` test helpers are duplicated across 4 files

`make_server` / `test_server` functions with identical bodies exist in:
- `catalog.rs` tests
- `session.rs` tests
- `validation.rs` tests
- `wrap/mod.rs` tests

**Impact:** Maintenance burden. Any field added to `McpServer` requires updating all four.

**Recommendation:** Create a `#[cfg(test)] pub mod test_fixtures` in the mcp module with a shared `make_test_server` helper.

---

## 3. Ergonomic / Idiomatic Improvements

### 3.1 `Resolution` enum should impl `Display`

`Resolution` is central to the user-facing matching logic but has no `Display` impl. Both the CLI and wrapper manually format resolution outcomes with ad-hoc string building.

### 3.2 `MatchTier` should impl `Display`

Used in wrapper output (`{:?}` formatting at `wrap/mod.rs:306`), which produces `ExactId`, `ExactAlias`, etc. A proper `Display` impl could produce user-friendly labels like "exact match", "alias match", "case-insensitive", "substring".

### 3.3 `SessionSet` could track the cleaned prompt internally

Currently, `compute_session_set` returns a `SessionSet` but the `cleaned_prompt` field is set _after_ the call at `wrap/mod.rs:263`. The session computation should accept the raw prompt, call `lex_tags` internally, and return the cleaned prompt as part of its result.

### 3.4 `resolve_match_list` could use `match` on slice patterns

```rust
fn resolve_match_list<'a>(matches: Vec<&'a McpServer>) -> ... {
    match matches.as_slice() {
        [single] => Ok(Some(*single)),
        [_, _, ..] => Err(matches),
        [] => Ok(None),
    }
}
```

More idiomatic than the current `if matches.len() == 1` / `if matches.len() > 1` pattern.

### 3.5 `lex_tags` could return `(String, Vec<Tag>)` with span info

Currently returns bare `Vec<String>`. If it returned tag objects with byte offsets, downstream code could produce better error messages pointing to the exact position of problematic tags.

### 3.6 Import report could use a builder pattern

`ImportReport` has 5 `Vec` fields that are built up during import. A builder with `report.imported(entry)`, `report.merged(entry)`, etc. would be cleaner than direct field pushes.

---

## 4. Research Gaps

### 4.1 Claude runtime MCP injection — **Medium urgency**

**Current state:** Claude is import/export only. The `injector_for_provider` function returns `None` for Claude, and the wrapper produces a hard error directing users to `claudine mcp export claude --apply`.

**Research needed:**
- Can Claude Code accept MCP servers via environment variables or runtime config files?
- Does `claude --mcp-config <file>` or similar exist?
- Is there a shadow-home approach that works for Claude's settings?

This is the most popular provider, so any injection support would have outsized impact.

### 4.2 Goose, Kimi, Qwen runtime injection — **Low urgency**

These providers are listed in Claudine's provider registry but have no import, export, or injection support. Research should determine:
- What config format each uses for MCP
- Whether runtime injection is feasible
- Whether import/export is feasible

Not urgent — the current provider set (Claude, Codex, Gemini, OpenCode, RooCode) covers the primary use cases.

### 4.3 MCP server health checking — **Low urgency**

The `claudine mcp check` command validates configuration structure but does not verify that configured servers are actually reachable or functional. Research topics:
- Can stdio servers be probed with `initialize` without side effects?
- Can HTTP/SSE servers be health-checked with a lightweight request?
- What timeout/retry semantics make sense?

This could power a `claudine mcp check --live` command.

### 4.4 MCP tool discovery for better `enabled_tools`/`disabled_tools` UX — **Low urgency**

The `McpServer` type supports `enabled_tools` and `disabled_tools` but there's no way to discover what tools a server offers. Research:
- Can tool lists be fetched from the MCP `tools/list` method?
- Should `claudine mcp config <server> --tools` show available tools?
- Could this power an interactive tool selection flow?

### 4.5 Provider config file watching / auto-sync — **Very low urgency**

Currently, catalog sync is manual (`claudine mcp sync`). Research whether filesystem watching (via `notify` crate) could auto-detect provider config changes and update the catalog. This is a convenience feature, not a correctness requirement.

---

## 5. Spec Inconsistencies

### 5.1 Storage layout

**Spec (mcp-catalog.md, line 3):** `~/.claudine/mcp/catalog.json`
**Implementation:** Same — `~/.claudine/mcp/catalog.json`

This is consistent. The plan noted older doc versions had flat paths like `~/.claudine/mcp-catalog.json` — those appear to have been resolved.

### 5.2 Defaults replacement vs merge

**Spec:** Does not explicitly state whether repo defaults replace or merge with user defaults. The plan (line 68-80 of defaults.rs) implements replacement.

**Recommendation:** The spec should explicitly state the merge/replace policy.

### 5.3 Tag terminal condition

**Spec (mcp-mode.md, lines 36-41):** Tags terminate on whitespace or end-of-line.

**Implementation (`session.rs:196`):** Also rejects tags that are followed by non-whitespace, non-EOF characters (e.g., `#calendar,`). This is stricter than the spec, which only defines whitespace and EOL as terminal conditions without explicitly addressing punctuation.

The implementation's behavior is more useful (prevents `#tag,` from matching), but the spec should document this explicitly.

---

## 6. Test Coverage Assessment

### Well-covered areas:
- Tag lexing edge cases (whitespace, numeric prefix, punctuation termination)
- Catalog CRUD and resolution tiers
- Session set deduplication and source tracking
- Validation rules
- Serde round-trips
- Wrapper flag extraction

### Under-covered areas:
- **No integration tests for reactive bootstrap path** — the `bootstrap_mcp_state` function in `wrap/mod.rs` is not directly tested
- **No tests for the `init` re-entry flow** — the branching logic in `run_init` based on existing state
- **No tests for `export` dry-run vs apply** — the `run_export` function
- **No tests for ambiguous tag cancellation** — what happens when `Select::prompt()` returns `Err`
- **Import module tests** — the exploration agent reported 1170 lines in `import.rs` but inline tests weren't shown; should verify coverage of per-provider parsers
- **Inject module tests** — 641 lines with provider-specific injection logic; test coverage should be verified

---

## 7. Priority Summary

| Item | Priority | Type |
|------|----------|------|
| 2.1 `current_repo_root` always returns `Some` | High | Bug |
| 1.1 Defaults replacement vs merge semantics | Medium | Design decision |
| 1.4 Init re-entry ignores user defaults context | Medium | UX gap |
| 1.2 Ambiguous tag cancellation is a hard error | Medium | UX gap |
| 2.3 `current` parameter ignored in defaults prompt | Medium | Dead code |
| 4.1 Claude runtime injection research | Medium | Research |
| 2.5 Duplicated test helpers | Low | Maintenance |
| 3.2 `MatchTier` Display impl | Low | Ergonomics |
| 3.3 SessionSet should own cleaned_prompt lifecycle | Low | Ergonomics |
| 1.5 Remove alias doesn't report remaining aliases | Low | UX gap |
| 2.4 Unused `Alignment::Right` binding | Low | Cleanup |
| 4.2-4.5 Provider/health/tools/watch research | Low-Very Low | Research |
