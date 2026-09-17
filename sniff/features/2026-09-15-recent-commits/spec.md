---
clarified: true
needs_rulings: false
clarified_by: opencode/zai-coding-plan/glm-5.3
references:
  spike-linking-cost.md: Benchmark of the per-remote containment engine reused by Decision 9, measuring pushed/unpushed/old-window scenarios across monorepo, fork-style, and 165k-commit checkouts; establishes the priority-ordered visit budget that bounds the walk.
implemented: true
implemented_by: "claude/opus"
---

# Recent Commits — Redesign Spec

The current "recent commits" functionality is limited and the CLI has taken over responsibilities that belong in the library (in particular linking). Two future-state documents describe the intended behavior and payload schema, and one current-state document records shipped CLI behavior:

- @sniff/docs/topics/repo/recent-commits.md — future-state behavior (topic doc)
- @sniff/docs/topics/repo/recent-commits-schema.md — future per-commit payload schema
- @sniff/docs/cli/repo_recent-commits.md — current shipped CLI behavior

A structured design review has concluded. The fifteen rulings below are the authoritative decision record for this feature. **Where a ruling contradicts either future-state document or the current code, this spec wins**; each decision states explicitly what it overrides so a reader knows exactly which statements in the referenced documents to distrust. The referenced documents themselves are not modified by this spec.

## Decisions

### Decision 1 — JSON contract: clean break

The library's `to_json()` returns a **bare JSON array** of commit objects. This overrides the current `CommitDescSet` object envelope documented in the current CLI doc. The envelope fields `period_label` and `repo_root` are dropped from JSON entirely; human output formats may still render equivalent context.

Field renames and re-semantics from the future schema are adopted as-is:

- `heading` — the first sentence of the commit message, terminating on `.` or newline
- `description` — the prose after the first sentence, before bullet points
- file kinds `modified`/`added`/`deleted`/`moved`

The file-kind vocabulary change (`renamed`/`copied` → `moved`) requires rename/copy detection to be **enabled** in the diff layer. Today it is deliberately disabled and rewrites map to `modified`; the implementation must turn on rename tracking. The schema's `original_path` and per-file `added`/`removed` line counts ride along with that change.

The payload schema gains an **author** field carrying name and email (see Decision 4).

### Decision 2 — Default scope and empty-result behavior: future docs wholesale

Adopted from the future topic doc without modification, overriding the current CLI:

- No scope argument = the last **10** commits (changes the current default of `3d`).
- A valid query with zero matching commits is a **success**: exit 0, empty report. A brief human-oriented "no commits matched" note is acceptable in text formats; JSON emits `[]`.
- `--no-error` and `--on-error` are **removed** from the recent-commits command (and from its siblings where the switches are shared — see Open Question 1).
- The repo-wide no-result convention used by other commands is untouched; only these subcommands change.
- Count selection filters **during** history traversal (walk until N matching commits are collected or history is exhausted), per the future doc.

### Decision 3 — Operation filter: free-form

`--action` (today's six-word enum: feat/chore/refactor/test/style/fix) is renamed to `--operation`, accepting **any** conventional-commit operation word: free-form, repeatable, OR'd. Completions may suggest common words but must not restrict input (precedent: the existing `--package` flag). Non-conventional commits are excluded when an operation filter is active. The schema's `operation` field is unchanged.

### Decision 4 — Author: split flags + schema field

The future doc's dual-use `--author` is resolved as **two** flags:

- `--author <name|email>` — filter
- `--show-author` — display toggle that adds the author to the commit header line

The payload schema gains an author field carrying **both** name and email. Author matching is a case-insensitive **substring** match against the author's name OR email. Co-author trailers are out of scope for v1 (future follow-up).

The subcommand's verbosity flags carry short aliases — `--verbose` (alias `-v`) and `--compact` (alias `-c`) — restoring the future doc's shortcuts. Both collide with the CLI's global repeatable `-v` verbosity counter and are resolved by subcommand-level shadowing (amended by Decision 13).

### Decision 5 — Calendar windows: local time, single-day semantics

All calendar-based scopes use the **viewer's local timezone**. This fixes a real defect: selection bounds are computed in UTC today while display labels are deliberately local, so a commit could be selected as "today" yet labeled as the previous day.

- Specific date `2026-04-01` = all commits whose local-time date is 2026-04-01 — a true single calendar day. This matches the current CLI **doc's** intent, not the current **code**, which implements since-midnight-UTC-forward; the drift was detected and the doc's semantics were chosen as the future behavior.
- `today` = since local midnight.
- `yesterday` = local midnight-to-midnight, that single day only. This **overrides** the future topic doc's "(commits starting on named day to latest)" phrasing — yesterday means yesterday only.
- Durations, hash, and count scopes are timezone-independent and unchanged.

Library selection builders accept an explicit timezone parameter, defaulting to host local time, so programmatic callers can pin a zone.

### Decision 6 — Merge commits, branch, and author posture: git-plausible defaults

- Merge commits are **included**; per-commit file lists remain diff-vs-first-parent ("what this commit brought into its mainline").
- The schema's `files: min(1)` constraint is **relaxed** to `min(0)` for degenerate no-change merges.
- `--branch <name>` is a selection **base**, not a filter: history walks from that branch's tip instead of HEAD. Local branch name first, remote-tracking name as fallback; local-only, no network.
- JSON datetime stays **UTC** (RFC 3339 `+00:00`) regardless of selection timezone.

### Decision 7 — Package/area attribution: structure-tier catalog, unified

Attribution and the `--package`/`--package-area` filters are powered by the manifest-only **structure tier** of repo detection — the same catalog `sniff repo packages` uses — not the full-tier detection the code calls today, which pays a whole-tree language/framework enrichment scan that attribution then discards; this is a cost fix. Filters reuse the same ownership index as attribution, deleting a duplicated linear rescan.

- Monorepo commits always carry `packages`/`package_areas` as **arrays** (empty when the commit touches no packaged file).
- Non-monorepo **omits** both fields entirely — a clean-break JSON change that also reconciles the schema's packages-optional / package_areas-required inconsistency.
- A file under a package area but outside any package (e.g. directly under `sniff/` but not under `sniff/lib` or `sniff/cli`) stays **unattributed** — no area-directory fallback.
- Unknown package/area names in filters remain typed errors listing valid names.

### Decision 8 — File-type classification: one classifier with precedence

A single canonical classifier: a new `ChangeCategory` enum (`source_code`, `web_assets`, `images`, `documentation`, `configuration`, `cicd`, `other`) plus a `classify_path` function in the existing path-only classification leaf module (`sniff/lib` `filesystem/path_kind.rs`).

Precedence, first match wins:

1. **CI/CD path rules** — a new small rule table (`.github/workflows/**`, `.gitlab-ci.yml`, `Jenkinsfile`, `.circleci/`, `azure-pipelines.yml`, `.drone.yml`, `buildspec.yml`, etc.). Nothing classifies CI/CD today.
2. Existing file-type registry associations: Font + Styling + `html`/`htm` → `web_assets`; Image (incl. SVG) → `images`; Documentation → `documentation`; Configuration → `configuration`; ProgrammingLanguage / FrameworkFile → `source_code`.
3. Miss → `other`.

Categories are **exclusive** — exactly one category per file — with the precedence documented. Per-commit `file_types` booleans are category-set membership and derive trivially. Existing predicates become wrappers over the enum; the CLI's duplicated filter collapses onto the enum. The sibling commands (`source-code-changes`, `documentation-changes`) keep their file-level pruning projection over the **same** categories.

Known behavior changes to document: `.html` leaves its documentation/source fallback and becomes `web_assets`; CSS leaves `source_code` and becomes `web_assets`; Angular `.component.html` templates stay `source_code` (registry framework file).

### Decision 9 — Library linking: containment engine reuse, existing preference order

The library's linking machinery reuses the existing hardened per-remote containment engine — currently used only by the deep request tier's "Synced to:" output — which walks ancestry from every remote-tracking branch tip, commit-graph accelerated, work-counter instrumented, and tested against clock skew. Among the remotes that **contain** the commit, the winning remote is chosen by the library's existing preferred-remote order (origin, then alphabetically-first non-upstream, then upstream): zero new preference policy.

A new commit-links module in the library (`sniff/lib` `filesystem/git/commit_links.rs`) consolidates the **three** existing divergent URL/parser implementations into one owner/repo + URL builder:

- The old library single-commit URL helper (origin-only, no containment check) becomes a thin wrapper; the `repo hash` command reuses it.
- The CLI's decoration-prefix "pushed" heuristic and its duplicate URL parser are **deleted** — the heuristic is provably wrong under merged branches on date-ordered walks.

Linking rides the RecentCommitsOptions/collect pipeline, computed once after the candidate set is fixed. Payload semantics: `remote` is three-valued — `true` means contained in at least one locally recorded remote-tracking ref (local knowledge, may lag true remote state), `false` means determined not contained in any walked tip, and `null` means undetermined after budget exhaustion (the authoritative statement lives in the refinement below); `commit_url` is present iff containment **and** the winning remote's provider yields a browser URL — so `remote: true` with no URL is a legitimate state for self-hosted providers. No network, no fetch. A pending spike measures the unpushed worst-case cost of the tip walks.

**Refinement (post-spike):** the containment walk is a **priority-ordered bounded walk**:

- Remote-tracking tips are walked in remote-preference order — origin first, then alphabetically-first non-upstream, then upstream; the same order this decision already uses to pick among containing remotes — stopping when all target commits are determined.
- A total budget expressed in **commit visits** bounds exhaustion. Visit counts, not wall time, are the budget unit — so the bound composes with the work-counter discipline — and it is sized with ~10× headroom over measured normal-state costs (the spike observed a transient ~13× per-visit cost degradation in one clone).
- When the budget exhausts, undetermined commits carry `remote: null` — a documented unknown state — and get no link. The payload's `remote` field is thereby three-valued (`true` / `false` / `null`), which respects the specification's principle that unavailable enrichment must not assert a negative fact.

Rationale, from the spike evidence (`spike-linking-cost.md`, same directory): unbounded containment costs 3–4 minutes on a 165k-commit clone with 5,197 remote-tracking tips (726M commit visits) while monorepo-scale costs run 34–76 ms; the existing deep-tier positional branch cap silently broke correctness there (alphabetical truncation dropped every relevant tip — 0/10 contained), so any bound must be priority-ordered, not positional; commit-graph acceleration measured neutral and is not relied upon.

### Decision 10 — Aggregate `sniff repo --json`: full alignment

The aggregate's three embedded commit families (`recent_commits`, `source_code_changes`, `documentation_changes`) embed the **same** bare JSON arrays the subcommand emits — one library collect path, no CLI-side envelope surgery (the surgery function is deleted). The window becomes last-**10**, matching the subcommand default; this overrides the current hardcoded 3-day window that embedded 84 commits ×3 families on this repo. Links are computed inline (local-only, so the aggregate's proven offline contract is intact); author and `file_types` fields flow through the same serialization.

Consequences:

- Aggregate consumers lose the constant period label from the payload (documented, not embedded).
- Quiet repos' recent activity reaches arbitrarily far back.
- ~8-10 test sites pinning the old shape are rewritten.
- The existing double history walk in the aggregate becomes consolidatable later (optional follow-up, not mandated).

### Decision 11 — Library API shape: non-generic runtime builder

The topic doc's typestate sketch is **superseded** — this spec wins. One non-generic `RecentCommitsOptions`, in the style of the library's existing builders:

- A **single** `selection: Selection` field defaulting to `Count(10)`; five selector methods set it (**last-wins**, documented — the same replacement semantics as `verbosity()`). "Only one selection at a time" is enforced structurally by the singular field rather than by compile-time rejection of a redundant second selector call.
- Additive AND-filter methods.
- `timezone(chrono::FixedOffset)`, defaulting to host local offset. **No chrono-tz dependency**: a fixed offset cannot express DST-observing named zones — accepted trade-off, recorded here.

Rationale: the CLI's scope argument is a runtime-parsed string, so compile-time enforcement cannot cover the primary caller; the library has zero typestate precedent; and the topic doc's own example passes an unselected options value to collect.

Legacy fate: the five `get_recent_commits_*` free functions, `CommitDescSet`, all post-hoc `filter_by_*` methods, and the aggregate's second-pass re-attribution entry point are **deleted** outright — a caller census verified zero uses outside the sniff package area; no deprecation wrappers. The ~1,745-line module splits into a directory: options / collect / render / links (links per Decision 9). `parse_period` (scope-string parsing) stays in the library beside `Selection`. Two-level re-exports (`filesystem::git` and `filesystem`) mirror today's pattern.

### Decision 12 — Rendering: library authors every text byte; CLI is a Prose passthrough

The library's render module contains **one** renderer emitting a prose-tagged document per options. The library's three text outputs are degradation modes over one layout walk:

- `to_prose()` — color/emphasis tags + links: the commit hash links to the remote URL when linking established one, else bare; file paths render as `file://` links.
- `to_markdown()` — links become `[text](url)`, emphasis degrades to `**`/`_`, color tags drop.
- `to_plain()` — bare text: no tags, no links.

The CLI **deletes** its parallel styled renderer (~350 duplicated lines across seven pieces of logic). Terminal mode becomes `Prose::new(report.to_prose(&opts)).render(&terminal)` via the existing biscuit-terminal Prose component; `--plain` becomes `to_plain()`.

Verbosity type: `RecentCommitsVerbosity { Compact, Normal, Verbose }` in the options module — settles the topic doc's `ReportVerbosity`-vs-`RecentCommitsVerbosity` naming inconsistency. `show_author` is an orthogonal toggle; `to_json()` ignores both.

The **normative** styling table (bold hash, blue operation, blue-dim scope, italic at, bold time, bold section labels, links) moves into the library topic doc expressed as prose-tag vocabulary; the CLI doc's table becomes a pointer. Doc fix: the topic doc's prose-grammar reference `@darkmatter/docs/topics/prose-grammar.md` is **dangling** — that document does not exist; the grammar is documented at `biscuit-terminal/docs/components/prose.md`, and the topic doc must repoint.

Known accepted behavior change: today's `--plain` keeps markdown bold markers (`**Description:**`); the future `to_plain()` strips everything — a documented behavior change for plain-mode consumers. Implementation note: verify in the first hour that a multi-line report survives a single Prose render without paragraph inflation (use the `bt prose` tool).

### Decision 13 — Verbosity flag shape (amends Decision 4)

Decision 4's `--verbose`/`--compact` ruling is **retained with short aliases**: the subcommand defines `--verbose` (alias `-v`) and `--compact` (alias `-c`). The CLI's global repeatable verbosity counter also owns `-v` and the `--verbose` long form, so the subcommand-level flags rely on clap's subcommand-overrides-global shadowing semantics. Recorded risks, accepted by the caller:

- The global `-v` count and the subcommand `-v`/`--verbose` mean different things in different argument positions.
- Shadowing behavior must be explicitly tested — add a test asserting the subcommand's `-v`/`--verbose` sets report verbosity and does not feed the global log-level counter.

This settles removed Question 4's "two flags or one value": **two flags**.

### Decision 14 — Sibling commands: kept as thin presets

`sniff repo source-code-changes` and `sniff repo documentation-changes` remain real subcommands, rebuilt as thin presets over the unified options → collect → render pipeline. The library's single renderer owns their file-level pruning projection and section headings (a consequence of Decisions 8 and 12 regardless of this choice). They inherit the shared behavior changes already decided: count-10 default, exit-0 on empty, `--no-error`/`--on-error` removed. Rationale: no script breakage beyond the already-decided changes; distinct, discoverable command names; reversible pre-release if consolidation is wanted later.

### Decision 15 — Heading/description parsing: schema-literal

Heading terminates at the FIRST `.` or newline, exactly as the schema states. Consequences, all deterministic and unit-testable: a subject without punctuation yields a heading equal to the whole first line; a commit without bullet points yields a present-but-empty description; multi-paragraph prose after the first sentence joins into the description until bullets begin; abbreviations such as "e.g." inside the first sentence truncate the heading early — recorded as a known limitation (a sentence-end heuristic can be adopted later if it annoys in practice). Conventional-commit operation/scope parse from the heading/subject line, not the whole message paragraph.

## Implementation Notes (no ruling required)

Former open questions closed as implementation-time details with documented defaults:

- **Work counters.** Linking reuses an engine whose counters already exist (commit visits, ref walks). Per-commit classification adds no I/O-shaped work and gets no new counter. The aggregate's every-invocation containment growth (Decision 10) is a verification step during implementation: compare counter totals before/after rather than a design ruling.
- **Count builder integer type.** The future doc's `count(count: int8)` is a doc typo; the count selection uses the platform word-size unsigned integer, matching the current scope parser's count payload.
- **Linking's ref snapshot.** The linking module must invoke the containment engine with its own locally observed ref snapshot — today the engine is only reachable through the fetch-coupled deep-tier path, which linking must not trigger.
- **Deep-tier positional branch cap.** The deep request tier's existing positional branch cap (the `max_remote_branches` truncation) demonstrably breaks containment correctness on tip-rich clones (0/10 contained in the spike); whether to migrate it to the same priority-ordered budget is adjacent scope this feature flags but does not own.

All open questions are now settled: Questions 2, 4, and 6 by Decisions 9, 12/13, and 12 respectively; Questions 1, 3, 5, and 7 by Decision 14, Decision 15, and the notes above.
