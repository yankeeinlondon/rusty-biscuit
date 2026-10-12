---
name: content-policy
description: Expert knowledge for the content-policy Rust library and `policy` CLI, which evaluate and renew freshness rules (`content_policy:`) declared in Markdown frontmatter. Use when working in the content-policy/ package area, adding the content-policy dependency, declaring or debugging a document's `content_policy` rules (Evergreen, TimeSensitive, ValidFor, ValidUntil, FileChanged), evaluating documents or cache manifests for staleness, planning or applying renewal edits, implementing a FileProvider, or touching code that stamps `last_updated`.
---

# Content Policy

`content-policy/lib` (crate `content-policy`) and `content-policy/cli` (crate
`content-policy-cli`, binary `policy`). Markdown documents declare rules under
a frontmatter key; the library validates them, evaluates them at an explicit
instant, resolves one action, and plans byte-exact renewal edits.

The behavior record is `content-policy/docs/topics/policy-lifecycle.md`; the
user-facing overview is `content-policy/README.md`. Read the topic page before
changing semantics: this skill only orients.

## Rule grammar

```yaml
last_updated: 2026-09-28
config_fingerprint: blake3-lf:9f2c…e7        # 64 lowercase hex digits
content_policy:
  - ValidFor(3mo)                            # compact form, action refresh
  - rule: ValidUntil(2027-01-01)             # long form: rule + action, both required
    action: archive
  - FileChanged(src/config.rs, @config_fingerprint)
```

| Rule | Triggers | Renewal |
| --- | --- | --- |
| `Evergreen` | never | nothing to renew |
| `TimeSensitive` | always | nothing to renew |
| `ValidFor(<n><d\|wk\|mo\|yr>[, @prop \| YYYY-MM-DD])` | interval elapsed; bare form reads the date property (`last_updated`) | replace the date |
| `ValidUntil(YYYY-MM-DD \| @prop)` | fixed deadline | nonrenewable |
| `FileChanged(<relative path>, @prop)` | file content differs from the fingerprint | replace the fingerprint |

- Dates take effect at **00:00 UTC**; a baseline later than today's UTC date
  is `unknown` (inconsistent), which is why every writer stamps UTC.
- Actions: `remove` > `archive` > `refresh`. Rules are an OR of triggers.
- Status: `fresh`, `stale` (refresh), `expired` (archive/remove), `unknown`.
  An unknown rule never nominates an action; the report marks the resolution
  incomplete instead.
- No declaration means the caller's default policy (`ValidFor(6mo)` built in),
  reported as `defaulted`. A default can be replaced, never removed.
- Invalid declarations are validation errors with **no verdict**, never a
  filtered or partial policy. Missing evidence is `unknown`, never `fresh`.
- Flow-list comma trap: `[ValidFor(3mo, @x)]` splits at the comma; quote rules
  that contain one. The parser names the trap in its diagnostic.
- `FileChanged` paths are relative, stay inside the repository (or, outside
  one, the starting directory), and reject absolute paths, drive letters, `\`,
  `~ @ % & ^` prefixes, `vault:`, `{{`, URLs, `,`, `)`, and surrounding
  whitespace. The lexical check runs in core without any feature.
- `blake3-lf:` normalizes only CRLF to LF (a lone CR is data); `blake3:` hashes
  raw bytes. A well-formed unknown scheme (`sha256:…`) is `unknown`, not an
  error; uppercase hex is an error.

## Library entry points

| Need | API |
| --- | --- |
| Evaluate Markdown bytes | `evaluate_document(bytes, &EvaluationContext)` → `Report` or `DocumentError` |
| Evaluate a map the caller already parsed (Darkmatter, cache manifests) | `evaluate_record(&EvidenceRecord, &ctx)` |
| Evaluate an explicit policy | `evaluate_policy(&Policy, &record, &ctx)` |
| Parse a policy | `Policy::from_declaration(&Value)` (frontmatter value), `Policy::from_text` (CLI text: one compact rule, or a flow list starting `[`) |
| Store a policy on its own | `Policy::to_json` / `Policy::from_json` (strict: `grammar_version` + `entries`, unknown or newer fails closed); `Policy::identity` |
| Configure key, default policy, date property | `PolicyOptions` via `ctx.with_options(..)` |
| Read frontmatter the same way the CLI does | `reader::read_frontmatter` |
| Plan / apply renewal | `plan_renewal(bytes, &RenewalContext)`, `apply_renewal(path, &plan)`, `RenewalPlan::apply_to(bytes)` |
| Watch files | `ctx.with_files(Arc<dyn FileProvider>, base_dir)` on either context |
| Compute a fingerprint yourself | `FingerprintScheme::Blake3Lf.fingerprint(bytes)` |

```rust
use content_policy::{EvaluationContext, evaluate_document};

let context = EvaluationContext::new(chrono::Utc::now()).with_document("notes.md");
let report = evaluate_document(&std::fs::read("notes.md")?, &context)?;
println!("{:?} {:?}", report.status, report.action);
```

- Always pass the instant; tests use a fixed `DateTime<Utc>` and
  `RenewalContext::new(date)` rather than `now()`.
- `Report::to_json` and `RenewalPlan::to_json` field names are frozen
  (`evaluation::report_json_field_names_are_frozen`); changing one is a
  breaking change to the CLI's `--json` contract.
- Policy identity (`xxh64:` + 16 hex) hashes sorted entries without baseline
  values, so renewal never changes it and reordering never does.

### Providers and the `file-adapter` feature

- `FileProvider::observe(&FileRequest) -> FileObservation` is synchronous and
  reports facts only: `Present(bytes)`, `Missing`, `NotAFile`,
  `Unreadable(reason)`, `OutsideBoundary(reason)`. Core hashes and decides, so
  every provider is judged identically. `OutsideBoundary` becomes a validation
  error (no verdict).
- Each path is observed once per evaluation or renewal run.
- Without a provider a file rule is `unknown` (`missing_provider`).
- `FileAdapter` (local disk through Biscuit File's `FileReference`) is behind
  the **off-by-default** `file-adapter` feature, because it pulls `gix`. The
  CLI enables it. Use `FileAdapter::new().with_tree_root(dir)` in tests so
  nothing depends on the process's current directory.

## Renewal limits

Evaluation never renews; renewal is an explicit content-update assertion.
`plan_renewal` writes nothing, and records missing baselines as
`new_baseline` (first capture). Planning fails, listing every problem, for:

- a future update date, unreadable or invalid frontmatter, a non-date baseline;
- a conflict: a renewed property that is also a `ValidUntil` deadline, or one
  property asked to take two values;
- a `FileChanged` fingerprint it cannot capture (`MissingEvidence`);
- a refused YAML shape for a value it must change: inside a flow list or a
  `- {rule: …}` entry, a block or multi-line scalar, a double-quoted string
  with escapes, an anchor/alias/tag, or a frontmatter block without a proper
  closing `---`.

`apply_renewal` re-reads the file, refuses if its `xxh64` plan fingerprint
changed (`ModifiedSincePlan`), re-parses the edited text as a safety net, and
writes atomically. Edits keep each line's own terminator (mixed CRLF/LF safe).

## Dependency rule

The library must **never** depend on Darkmatter or any PDF crate, with default
features or `--all-features`. Darkmatter is a caller (it passes parsed
frontmatter as an `EvidenceRecord`), not a dependency. `just deps-check`
enforces this and runs inside `just lint`. Biscuit File is used with
`default-features = false, features = ["yaml"]`.

## Writers of `last_updated`

`md hash --save` and Claudine's `inline-compose` write-back stamp
`last_updated` with the UTC date via `darkmatter::markdown::hash::last_updated_stamp`,
which renews every `@last_updated` rule. Darkmatter's effects auto-rehash
writes it too. Only `policy renew` also updates inline dates and fingerprints.
New writers must call `Markdown::stamp_baseline` with an injected
`DateTime<Utc>`, never `Local::now()`.

## CLI

```sh
policy check notes.md                   # terminal report; --plain / --json
policy check --needs-action notes.md    # prints true | false | unknown
policy check --at 2027-01-01 notes.md   # evaluate as of a date
policy renew notes.md                   # preview the plan
policy renew notes.md --write           # apply; --on YYYY-MM-DD for an earlier date
```

Exit `0` when output was produced (stale included), `1` on error (diagnostics
on stderr even with `--json`), `2` on usage. `--key`, `--default-policy`, and
`--date-property` fall back to `CONTENT_POLICY_KEY`, `CONTENT_POLICY_DEFAULT`,
`CONTENT_POLICY_DATE_PROPERTY`. `renew --today` is hidden and exists for tests.

## Editor schema

`content-policy/schemas/content-policy.yaml` declares `short_form`,
`long_form`, and `policy` for DMLS; the library exports its text as
`content_policy::EDITOR_SCHEMA` for callers to compile into their own schema.
`policy[]` (a list of the union type)
cannot be loaded: Darkmatter rejects `[]` on a union-typed named type
("cannot apply `[]`/constraints to the union-typed named type"), so its base
schema does not type `content_policy` yet, and `rule` is not `(required)`. Its tests live
in `darkmatter/lib/tests/l1/content_policy_editor_schema.rs`.

## Recipes (run in `content-policy/`)

| Recipe | Does |
| --- | --- |
| `just test` | L1 for `content-policy --all-features` and `content-policy-cli` |
| `just test-l2` | the styled `policy check` / `policy renew` tables in a real tmux pane |
| `just lint` | clippy for both crates, then `deps-check` |
| `just deps-check` | the dependency rule above |
| `just doctest` | library doc tests |
| `just install` | installs `policy` |

The only real-terminal tests are `cli/tests/level2_terminal_tables.rs` (tmux,
80 columns, one owned session per test). They assert what the pane shows: every
table row keeps its right border, each row is as wide in display cells as the
top border (`unicode-width`, so double-width characters count twice), a long
`FileChanged` rule and fingerprint rejoin after wrapping, and the status word
carries its SGR color. `--plain`, `--json`, and `--needs-action` stay Level 1.
Run them only through `just test-l2`. There are no L3/browser/real tiers; those
recipes are "not applicable" stubs, so never give a test their tier-marker
name here. Test binaries use
per-file discovery; shared helpers are `tests/common/` and the scripted file
provider is `lib/tests/fake/`. Keep large `include_bytes!` fixtures out of
`common/`, because every binary that declares it is then scheduled by them.
