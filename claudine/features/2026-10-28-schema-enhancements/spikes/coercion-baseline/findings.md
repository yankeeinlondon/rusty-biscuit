# Coercion-Baseline Spike: Findings

_Throwaway spike, 2026-09-26, for [2026-09-21-schema-enhancements](../../spec.md)._

## What was measured

Darkmatter documents call about 110 built-in functions, such as `upper(title)`.
Today each function checks and converts its own arguments. The spec replaces
that with one shared **coercion engine** that keeps, converts, or rejects each
argument according to the function's declared signature, and it migrates every
function at once. The question here: how much observable behavior would change?

## Method

1. **Recorder.** The spike called every registered function through the real
   evaluator. For each parameter in turn, it tried 25 sample inputs (`null`,
   `true`, `2.5`, `"4"`, `" 4 "`, `"1_000"`, `"pear"`, `[]`, `{"a":1}`, and
   others) and recorded 3,575 outcomes. The calls ran in a temporary directory
   that is not a repository, with deny-all network, ICMP, and shell
   authorities, so no network request was possible.
2. **Stand-in binder.** A small parser for the draft `function(...)` signatures
   applied the spec's rules, then ran the unchanged handler on the converted
   values to predict the new result.
3. **Diff.** Each call was classified as preserved or changed, with the cause of
   each change.

## Key numbers

- 110 functions; 96 have a draft signature, and **14 are missing** from the
  draft, as the spec already notes.
- **26** functions are fully preserved. All of them have `any` parameters: the
  type predicates, the date predicates, `and`, `or`, `contains`, `pr`, and `cicd`.
- **84** functions change somewhere. Most changes are benign: 421 failing
  probes would succeed through number/boolean → text or text → number.
- **71** functions have a call that works today and would fail or return a
  different value. **59 of those 71 change only because of `null`.**
- Only **12** functions regress for a non-null reason: `number`, `round`,
  `length`, `has_key`, `validate_schema`, `file_exists`, `has_command`,
  `has_binary`, `can_execute`, and three `has_*` shell probes.

## Biggest risks

1. **Null.** Sixty functions return `null` for `null` today. The draft declares
   `| null` nowhere, so any such call, including one reading a missing optional
   frontmatter property, would fail composition.
2. **The conversion functions.** `number()` and `round()` use their own number
   grammar (`" 4 "` → `0`, `true` → `1`, `"pear"` → `0`). The draft's
   `round(x: number, default?)` makes the fallback unreachable.
3. **Draft signatures are wrong in places.** `file_exists` lacks the spec's own
   `| null`; `length` drops `object`; `is_positive`, `ensure_leading`, `pr`, and
   `cicd` use accommodating `any`; `pr_list` lacks `min(1)`.
4. **Exact integers are lost.** The engine binds `"9007199254740993"` exactly;
   handlers compute in floating point and return `9007199254740992`.
5. **Boolean → text surprises.** `has_command(true)` finds a program named
   `true`; today it returns `false`.

## Recommended spec changes

- Add one default null policy, preferably a function-level `propagates-null`
  flag: the engine returns `null` without calling the function. One word covers
  each of 60 functions, and functions still never see `null`.
- Settle `number()` and `round()` fully: grammar, booleans, fallback, name.
- Require refined types (`date`, `ip-address`, `enum`) and constraints wherever
  a handler validates content; otherwise 93 probes turn type errors into domain
  errors.
- Scope or strengthen the exact-integer guarantee.
- Fix the draft signatures listed in `signature-decisions.md`.

## Limits

- One argument varies at a time; interactions between arguments are untested.
- Errors are classified by message heuristics. Provider, repository, ICMP,
  shell, and capture-dependent functions are compared only up to the point
  where they need I/O.
- The binder implements the spec text, not a real engine. The draft barely
  uses unions or overloads.
- Call-site counts (`callsites.json`) are rough regex matches, mostly in
  documentation and tests; `~/.claude` has 6.

## Reproduce

The throwaway code lives in worktree
`/Volumes/coding/personal/rusty-biscuit/.claude/worktrees/agent-a27591efd1c6158e9`
at `darkmatter/lib/examples/spike_coercion_baseline/` (`main.rs`, `binder.rs`,
`analyze.py`, `callsites.py`). Copies are in `src/`. From the worktree root:

```sh
cargo nextest run -p darkmatter --example spike_coercion_baseline   # binder unit test
bash src/run.sh <out-dir>              # writes <out-dir>/baseline.json (about 1 s after build)
python3 src/analyze.py <out-dir>/baseline.json <out-dir>   # ledger-draft.md, stats.json
python3 src/callsites.py <out-dir>/baseline.json <out-dir>/callsites.json <root>...
```

`run.sh` hard-codes the worktree path; edit its `cd` line to run elsewhere.
