# Phase 1: Darkmatter path inventory

Read-only inventory for spec section 10 ("Path functions use the shared
resolver"), acceptance criterion 10, plan rule 7, and the Wave 1 "Path
inventory" task. Line numbers are as of this checkout
(`feat/osc-9-4-protocol`, 2026-10-16). Paths are relative to the repository
root.

Abbreviations:

- `F` = `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`
- `R` = `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`
- `P` = `darkmatter/lib/src/markdown/compose/expression/path_projection.rs`
- `BF` = `biscuit-file/lib/src/file_reference/`

## 1. The shared resolution machinery

| Helper | Location | What it does |
| --- | --- | --- |
| `normalize_path_arg` | R:342 | Strips `file://`, collapses repeated `/`. Applied before every `FileReference::new`. |
| `is_remote_url` | R:330 | Case-insensitive `http`/`https` scheme test. |
| `resolve_document_file_ref` | R:394 | `FileReference::resolve_in_context(document_file_context(cwd, request))`. `Ok(Some)` = regular file matched, `Ok(None)` = clean miss, `Err` = typed `FileReferenceError` (probe I/O, missing home/vault/env, tree escape, `RemoteNotLocal`, …). |
| `document_file_context` | R:413 | Reuses the captured request `FileResolutionContext` when `cwd` is the same directory (identity via `canonicalize_simplified`, R:430); otherwise `for_cwd`, or `for_trusted_external_cwd` when only that validates. Never re-reads CWD/home/git. |
| `resolve_document_file_ref_shape` | R:462 | Matched file, else the **first** entry of `FileReference::candidate_plan(ctx)`; `Err(InvalidSyntax)` when there is no local candidate. |
| `resolve_arg` | F:1536 | `normalize_path_arg` + `FileReference::new` + `resolve_document_file_ref`. Returns `Option<PathBuf>`. No URL guard of its own. |
| `resolve_path_arg` / `resolve_path_shape` | F:1690 / F:1707 | Rejects HTTP(S) with `"{name}() does not accept HTTP(S) URLs"` (`ExpressionError::Other`), then `resolve_document_file_ref_shape`. Typed errors become `ExpressionError::FileReference` via `file_reference_error` (F:1505). |
| `path_display_components` | F:1759 | `make_portable_relative_in_context(path, ctx.cwd, request)` split on `/` into `(dirs, basename)`. |
| `make_portable_relative_in_context` | P:142 (ladder `project_in_context` P:61) | First match wins: strip repository root (`strip_prefix_any_spelling`, P:89, retries canonical spellings), strip `cwd`, `~/`-alias under home, else absolute verbatim. Rendered with `biscuit_file::to_portable_string`. |
| `FileRefFailure::classify` | `expression/error.rs:74` | `InvalidSyntax`/`UnsupportedScheme`/`UnsupportedUserHome`/`ForeignAbsolutePath` → `malformed`; `RemoteNotLocal` → `remote_not_enabled`; **everything else, including `Io` probe failures, → `not_found`** (typed cause kept in `source`). |

Candidate plan (biscuit-file, `BF/resolve.rs`):

- Absolute payload → one `RootProvenance::Absolute` candidate, verbatim
  (`build_anchoring_candidates`, BF/resolve.rs:1043). A drive path on POSIX or
  a drive-less `/x` on Windows is `ForeignAbsolutePath` (BF/resolve.rs:183).
- Explicit `./`/`../` → one `Source` candidate `simplify_root(cwd).join(raw)`;
  the `./` is **not** normalized away (`candidate_plan` does not normalize;
  `candidate_plan_with_order` does, BF/mod.rs:852).
- Implicit bare → `cwd` (`Source`) then repository root (`Repository`), deduped
  (`implicit_relative_roots`, BF/resolve.rs:954). Outside a repository: `cwd`
  only. No root is invented.
- `@`, `&`, `^`, `~`, `vault:`, `%` → their own root lists; `&`/`^` without a
  repository and `~` without home are typed errors; `%` (recursive) yields
  **search-root directories**, not file paths.
- URL → empty plan; `resolve_in_context` returns `Err(RemoteNotLocal)`.
- Probe (`probe_candidate`, BF/resolve.rs:362): regular file = match;
  directory/other = `NonFile` (continue); `NotFound`/`InvalidFilename` = miss;
  any other I/O error (permission) **stops the search** with
  `FileReferenceError::Io` (BF/resolve.rs:490).

Comment drift found: R:443-448 (doc on `resolve_document_file_ref_shape`) says
"An implicit bare miss therefore yields the repository-root candidate". The
code and the test `missing_implicit_path_shape_is_document_cwd_first`
(F:4763) show the miss shape is the **document-cwd** candidate. The code is
correct; Phase 2 should fix the comment when it touches this helper.

## 2. Function inventory

"Shape" = `resolve_path_shape` (shared resolver, missing target becomes first
candidate). "Resolve" = `resolve_arg` (shared resolver, miss is `None`). "Display"
= result projected through `make_portable_relative_in_context`.

| Function | Def (F unless noted) | Return | Input resolution | Missing target | HTTP(S) input | Probe/permission, invalid reference | Existing tests |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `absolute` | 1566 | `file` (native `to_string_lossy`, not portable) | Resolve | **Error**: `FileReference` diagnostic, kind `not_found`, no source (F:1574-1581) | Error via `RemoteNotLocal` → kind `remote_not_enabled` (no explicit guard) | Typed `FileReference` error (`Io` → `not_found` kind with source) | F:3808 `absolute_and_file_exists_resolve_relative_to_base_dir`; F:3832 `absolute_repository_scoped_reference_uses_package_before_repository` |
| `relative` | 1657 | `file` (display, portable) | Resolve | Error `not_found` | Error `remote_not_enabled` | Typed | F:3904 |
| `file_exists` | 1587 | `bool` | Resolve, then `p.exists()` | `false` | runtime attached: fetched → `true`, **any fetch failure/denial → `false`**; no runtime → `Other` error | **Swallowed**: `Ok(Some(p)) => p.exists(), _ => false` (F:1619-1622). Invalid reference, permission probe error, missing home/vault, tree escape all read `false`. Non-string arg → `false` | F:3808 (incl. `"\0bad"` → false), F:3861, F:3886, F:5493, F:5521 `file_exists_false_for_policy_denied_remote`, F:5531 |
| `frontmatter` | 2430 | object / value / null | `load_markdown` (F:2381) = Resolve then `read_to_string` | Error `not_found` | runtime: read from fetch cache; no runtime → `remote_not_enabled`; fetch failure → `Other` | Resolve errors typed; read failure → `FileReference` kind `not_found` with typed `Io` source (F:2410-2419) | F:3914; F:3938 `load_markdown_unreadable_file_yields_typed_file_reference` (unix); F:5493, F:5542 |
| `try_frontmatter` | 2461 | `{ok,value,error}` | `load_markdown` | `{ok:false,error}` | as `frontmatter`, folded into `error` | folded into `error` (a value, not success) | `darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs` |
| `markdown_body_empty`, `markdown_title`, `validate_schema` | 2538, 2550, 2584 | bool / string-or-null / bool | `load_markdown` | Error | as `frontmatter` | as `frontmatter` | F:~3975 `markdown_body_empty_and_title`, F:5493 |
| `find_files` | 2504 | `file[]` (absolute, portable) | `GlobReference::list_files(ctx.file_context())` | `[]` | `GlobReference` error | `GlobReference` error | `darkmatter/lib/tests/l1/glob_consumers.rs` (incl. permission at :495), `find_files_and_try_frontmatter.rs` |
| `is_indexed_file` | 1782 | `bool` | Shape; basename from display | works (pure name) | `Other` "does not accept HTTP(S) URLs" | Typed | F:4587, F:4654 |
| `file_index` | 1794 | number | Shape | works | `Other` | Typed | F:4602 |
| `increment_file_index` | 1810 | string (display) | Shape; keeps **resolved** `path.parent()`; output re-projected | works; implicit miss is shaped under the **document dir** | `Other` | Typed | F:4612, F:4781; `compose/tests/schema.rs:166` (`decrement_file_index(review)`) |
| `decrement_file_index` | 1843 | string (display) | same as increment | same | `Other` | Typed | F:4633, schema.rs:166 |
| `find_first_index` / `find_last_index` | 1962 / 1970 (body `find_index_endpoint` 1880) | string (display) | Shape; scans resolved parent | returns input shape; parent `NotFound` → input | `Other` | Typed for resolution; `read_dir` errors other than `NotFound` → `Other` "failed to scan" | `darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs:68` |
| `basename` | 1978 | string | Shape; display basename | works | `Other` | Typed | F:4679, F:4709, F:4993 |
| `basename_without_index` | 1990 | string | Shape | works | `Other` | Typed | F:4679 |
| `dirname` | 2015 | string | Shape; **display** dirs unless the display form is absolute | works (shape) | `Other` | Typed | F:3794, F:4679, F:4709, F:4781; compose tests listed in section 4 |
| `ext` | 2046 | string | Shape | works | `Other` | Typed | F:4679 |
| `parent_dir` | 2058 | string | Shape; last display dir | works | `Other` | Typed | F:4679 |
| `file_trailing` | 2070 | string | Shape | works | `Other` | Typed | F:4679 |
| `dir_leading` | 2085 | string | Shape; display dirs minus last | works | `Other` | Typed | F:4679 |
| `join` | 2102 | string (display) | Left: Shape; right: leading separators trimmed, raw-joined; joined string re-shaped | works | `Other` (either arg) | Typed | F:4721, F:4741, F:4752, F:5005 |
| `link` / `code_link` | 2291 / 2302 (`resolve_link_parts` 2229) | Markdown string | Shape; label = display, destination = `try_portable_string` | works | 1-arg: `Other`; 2-arg: URL accepted as destination | Typed; unportable Windows path → `Other` | F:4832-5128 |
| `package` / `package_area` | `functions/repository.rs:27,31` (`lookup` :39) | string | Shape | lexical lookup works | `ContractViolation` | Typed | `functions/repository.rs` tests; `darkmatter/lib/tests/l1/context_functions.rs` |

Pure filename helpers, unchanged by this feature: `parse_indexed_stem` (F:379),
`file_extension` (F:407), `file_stem` (F:416), `indexed_stem_info` (F:1731),
`format_indexed_stem` (F:1750). `has_command`/`has_binary` (F:1629) never
resolve through the context and are out of scope.

## 3. Findings with code evidence

### 3.1 `absolute` errors on a missing target today (confirmed)

```rust
// F:1566-1582
match resolve_arg("absolute", raw, ctx)? {
    Some(p) => Ok(Value::String(p.to_string_lossy().to_string())),
    None => Err(file_reference_error("absolute", raw, ctx, FileRefFailure::NotFound, None)),
}
```

`resolve_arg` returns `None` for any clean miss, and also for an **existing
directory** (the probe classifies a directory as `NonFile` and keeps
searching). So `absolute("sub/")` errors today even when `sub/` exists. HTTP(S)
inputs reach `resolve_in_context` and fail as `RemoteNotLocal` (kind
`remote_not_enabled`), not with the explicit "does not accept HTTP(S) URLs"
message the path-component functions use.

"The resolver's first candidate" is exactly what
`resolve_document_file_ref_shape` (R:462) already returns:

| Input (cwd = `<repo>/prompts`, repo = `<repo>`) | First candidate |
| --- | --- |
| `sub/missing.md` (implicit) | `<repo>/prompts/sub/missing.md` (document dir, `Source`) |
| `./sub/missing.md` | `<repo>/prompts/./sub/missing.md` (the `./` is kept; F:4778 asserts this) |
| `../x.md` | `<repo>/prompts/../x.md` (tree-escape check applies) |
| `&docs/x.md` | `<repo>/docs/x.md` |
| `^x.md` | package root first, then package area, then repository |
| `@x.md` | first magic root (local prepends, package, area, local root, …) |
| `~/x.md` | `<home>/x.md`; missing home → `MissingHomeContext` |
| `/abs/x.md` | `/abs/x.md` verbatim |
| `%x.md` (recursive) | the first **search-root directory**, not a file path |
| `https://…` | no candidate → `InvalidSyntax("has no local filesystem candidate")`; `resolve_path_shape` rejects it before that |

### 3.2 `dirname` loses an absolute anchor inside a projection root

```rust
// F:2015-2039
let path = resolve_path_arg("dirname", &args[0], ctx)?;
let projected = make_portable_relative_in_context(&path, &ctx.cwd, &ctx.file_resolution_context);
if Path::new(&projected).is_absolute() {
    return Ok(Value::String(path.parent().map(biscuit_file::to_portable_string).unwrap_or_default()));
}
let (dirs, _) = path_display_components(&path, &ctx.cwd, &ctx.file_resolution_context);
Ok(Value::String(if dirs.is_empty() { String::new() } else { dirs.join("/") }))
```

The absolute branch only fires when the projection ladder (P:61) found **no**
root: not under the repository, not under the document `cwd`, not under home.
An absolute input under any of those roots is returned in display form:

- under the repository → repository-relative (`claudine/features/X`), an
  implicit reference that a later reader resolves document-dir first;
- under the document dir (outside a repository) → cwd-relative;
- under home → `~/…` (this one survives, because `~` is itself an anchor).

The function's own doc comment (F:2012-2013, "An absolute input retains its
absolute anchor") overstates the behavior; it holds only outside every
projection root. `parent_dir`, `dir_leading`, `join`, the file-index family,
and `link`'s label share the same display projection.

Worked example: `dirname(absolute(spec)) + '/review-1.md'` (spec section 11,
`review-loop.md` line 664 of the spec). Repository `<repo>`, sequence document
`<repo>/prompts/review-loop.md`, caller spec
`<repo>/claudine/features/X/spec.md` (outside the prompt dir).

1. `spec` reaches the expression as the projected caller value, an absolute
   native path (`resolve_caller_file_value`,
   `darkmatter/lib/src/markdown/compose/schema_validation.rs:1174`).
2. `absolute(spec)` → absolute anchoring, file exists →
   `<repo>/claudine/features/X/spec.md` (native spelling; backslashes on
   Windows).
3. `dirname(...)` → projection strips the repository root →
   `claudine/features/X/spec.md` is relative → returns `claudine/features/X`.
4. `+ '/review-1.md'` → **`claudine/features/X/review-1.md`**, a bare implicit
   reference with no provenance.
5. `file_exists(latest_review)` evaluated in the group's `start:` (cwd
   `<repo>/prompts`) probes `<repo>/prompts/claudine/features/X/review-1.md`
   first, then `<repo>/claudine/features/X/review-1.md`. Existence is right
   only by repository fallback. It is wrong when:
   - a same-named path exists under the document dir (collision);
   - the sequence document sits outside the repository (user-scope or embedded
     prompt). `document_file_context` derives a trusted-external context, and
     `FileResolutionContext::derive` (`BF/context.rs:875-884`) drops
     `repository_root` when no scope catalog selects one, so only the
     prompt-dir candidate is probed;
   - any path-building consumer shapes the missing `review-1.md` (the new
     `absolute`, `increment_file_index`, `join`). The first candidate is the
     **document dir**: `<repo>/prompts/claudine/features/X/review-1.md`;
   - the string goes to another process or task with a different cwd (agent
     prompt, `ensure_file`, `shell:`).

The shipped prompt already works around this:
`prompts/_reviews/feature-review.md:36-40` explains that
"`decrement_file_index(review)` is not used because it re-anchors a relative
`review` on this document's directory, and a sequence step passes a
repo-relative `dir`".

### 3.3 File-index operations: resolve first, keep resolved parent, then lose it in display

`increment_file_index` (F:1810) and `decrement_file_index` (F:1843):

1. resolve the input with `resolve_path_arg`, which uses the shared shaper and
   so resolves before editing (satisfies "resolves its input first");
2. compute the new name from the display basename;
3. join it onto the **resolved** `path.parent()` (satisfies "keeping the
   resolved parent" internally);
4. return `make_portable_relative_in_context(&out, …)`, so an in-repo absolute
   input comes back repository-relative. This is the same anchor loss as
   `dirname`.

"Provenance" is only what the input string carries. A relative string derived
from `dirname` (3.2) has none: if the target is missing, it is shaped under the
document dir and the incremented path lands there too. `find_first_index` and
`find_last_index` (F:1880) scan the resolved parent, which is correct, but
return display text as well.

### 3.4 How caller file provenance travels

- **As the value.** Schema-typed `file` caller values are bound before
  frontmatter interpolation
  (`schema_validation.rs:1174 resolve_caller_file_value`). Lazy values take
  `candidate_plan_with_order(context, AuthoringBaseFirst).first()` against the
  **caller's** `FileResolutionContext`. Eager values take the matched path.
  Either way the expression sees an absolute native string, so every
  downstream function resolves it with `Absolute` anchoring. The anchor lives
  in the string, and only a projection can lose it.
- **As a side record, for diagnostics only.** `CallerFileProvenance` (R:16:
  property, raw reference, origin context, candidate, candidate provenance) is
  stored in `ComposeOptions.caller_file_provenance`
  (`context/options.rs:243`, copied into the `ResolutionContext` at
  `options.rs:1634,1681`) and keyed by occurrence (`/prop`, `/prop/0`). When
  an fs function's **first argument** is a direct identifier or index of such
  a value (`evaluate_caller_file_argument`, `expression/mod.rs:899`;
  `caller_file_occurrence` :939), dispatch sets
  `active_caller_file_provenance` (`expression/mod.rs:858-864`).
  `file_reference_error` (F:1505) is its only reader: it reports the caller's
  raw reference and cwd. **It never changes resolution.**
- A nested call (`dirname(absolute(spec))`) or a concatenation carries no
  occurrence. Claudine lifecycle and loop contexts are built by
  `document_expression_resolution_context`
  (`claudine/lib/src/composition/mod.rs:235`) from fresh `ComposeOptions`, so
  their `caller_file_provenance` map is empty. Request anchors come from the
  captured `FileResolutionContext`, and nothing recaptures CWD, home, or the
  repository.

### 3.5 Existence and read contracts compared with the spec's error rule

Spec section 10 says "Permission errors, invalid references, and remote-fetch
failures are never turned into missing-file success". The current code
conflicts with that in two places:

- `file_exists` maps **every** resolution error to `false` (F:1619-1622),
  including a permission probe error, an invalid reference (`"\0bad"`, asserted
  `false` at F:3825-3828), a missing home or vault, and a tree escape. With a
  runtime attached, a denied or failed remote fetch also reads `false`
  (F:1609-1612; asserted by F:5521). The function doc (F:1584-1586) documents
  "never error".
- `load_markdown` read failures and `Io` probe failures are reported with kind
  `not_found` (typed source attached). This is an error, not a success, so it
  is compliant, but the kind label is misleading.

Phase 2 needs an explicit decision on this. Either `file_exists` starts
returning typed errors for probe, invalid-reference, and fetch failures (and
the three tests above change), or the spec's "keep missing-file behavior" is
read as covering them. The plan says "Keep read/existence contracts and typed
probe errors", which supports the first reading; record the choice in the
implementation log.

## 4. Existing tests that pin the current display behavior (at risk in Phase 2)

These assert repository-relative output from `dirname(<absolute caller value>)`.
They follow 2026-09-01-file-param-anchoring, whose D4 relied on the display
projection. Preserving the absolute anchor changes them, and each needs a
deliberate expectation update:

- `darkmatter/lib/src/markdown/compose/tests/schema.rs:565`
  `eager_caller_file_projection_is_identical_from_root_and_package_launches`
  expects `claudine/fixes/file-param-anchoring/plan.md`.
- `schema.rs:795` `baseline_file_projection_layers_with_document_schema_and_skips_absent_optional`
  expects `claudine/fixes/layered/plan.md`.
- `schema.rs:874` `file_resolution_context_alone_anchors_raw_caller_overrides`
  expects `claudine/fixes/context-only/plan.md`.
- `claudine/cli/tests/l1/compose_caller_file_provenance.rs:86` expects
  `**Log File:** packages/example/fixes/case/log.md` and `@packages/...`
  mentions.
- `darkmatter/cli/tests/l1/compose_schema_file_rewrite.rs:146` expects
  `review_file == "area/review-1.md"` (document-authored eager spec; check
  whether the expression sees an absolute value there).
- Probably unaffected (substring or relative input), still to re-run:
  `claudine/cli/tests/l1/compose_initialize_acceptance.rs:611`,
  `claudine/cli/tests/l1/ctx_launch_anchor.rs:273`,
  `darkmatter/lib/tests/l1/frontmatter_surface_projection.rs`,
  `schema.rs:166` (interpolation only; spec stays relative),
  `claudine/cli/tests/prompts/shipped_prompt_contract.rs:361` (opt-in; relies
  on the `~/` arm).

No existing test covers: `absolute` on a missing target, `absolute` on a URL
with the explicit HTTP(S) message, a dirname or file-index result for an
absolute input **inside** the repository, a document/repository collision for
path functions, paths with spaces, or probe errors for path-shaping functions.
F:4763 is the only test of missing-shape order.

## 5. Path contract and fixture table for Phase 2

Columns state the expected result per the spec. "abs" means absolute portable
or native spelling, compared by identity with `canonicalize_simplified`.
Fixture: scratch repo `<r>` with `.git`, document `<r>/prompts/doc.md`
(cwd `<r>/prompts`), private home `<h>`.

| # | Case | `absolute` | `dirname` | file-index ops (`increment`/`decrement`/`find_*`) | existence/read (`file_exists`, `frontmatter`, …) |
| --- | --- | --- | --- | --- | --- |
| 1 | Collision: `notes.md` in both `<r>/prompts` and `<r>` | `<r>/prompts/notes.md` (document first) | `""` (unchanged) | `notes-2.md` beside `<r>/prompts/notes.md` | reads the document copy |
| 1b | Only `<r>/sub/x.md` exists; input `sub/x.md` | `<r>/sub/x.md` | `sub` (relative input: display unchanged) | sibling of `<r>/sub/x.md` | `true` / reads repo copy |
| 2 | Explicit `./sub/x.md`, missing | `<r>/prompts/sub/x.md` (first candidate; normalize `./`) | `sub` (display) | `sub/x-2.md` under `<r>/prompts` | `false` / `not_found` error |
| 2b | Explicit `../x.md` (inside tree) | `<r>/x.md` | display of `<r>` (`""`) | under `<r>` | per existence |
| 2c | `../../escape.md` leaving the tree | typed `RelativeTreeEscape` error | same error | same error | error, **not** `false` (needs the 3.5 ruling) |
| 3 | Caller provenance: lazy `spec` from a caller launched in `<r>/pkg`, value `fixes/c/spec.md` | `<r>/pkg/fixes/c/spec.md` | **`<r>/pkg/fixes/c`** (absolute, portable) | `<r>/pkg/fixes/c/spec-2.md` (absolute) | resolves the caller file; errors cite the caller reference and cwd |
| 4a | `@x.md` (magic) missing | first magic root + `x.md` | display, or absolute if Phase 2 extends the rule to resolved prefixes (decide) | first magic root | `false` / error |
| 4b | `~/x.md` | `<h>/x.md`; no home → `MissingHomeContext` | `~` | `~/x-2.md` | per existence; missing home is an error |
| 4c | `&docs/x.md` | `<r>/docs/x.md`; outside repo → typed error | `docs` | `docs/x-2.md` | per existence; no repo → error |
| 4d | `^x.md` with package catalog | package-root candidate (F:3832 pattern) | display | package root | per existence |
| 4e | `vault:x.md` unconfigured | `VaultNotConfigured` error | error | error | error, not `false` |
| 5 | Absolute reference outside every root (`<other>/a/b.md`) | `<other>/a/b.md` | `<other>/a` (current absolute branch) | `<other>/a/b-2.md` | per existence |
| 6 | Missing target, implicit `sub/new.md` | `<r>/prompts/sub/new.md` (**changed**: was error) | `sub` | `sub/new-2.md` under `<r>/prompts` | `false`; `frontmatter` → `not_found` error |
| 6b | Missing `%x.md` (recursive) | reject (the first candidate is a directory, not a file shape) | reject or as today (decide) | same | per glob semantics |
| 7 | Outside a repository (cwd `<t>/docs`, no `.git`) | implicit → `<t>/docs/x.md` only; never invents a root | display is cwd-relative unless input absolute | under `<t>/docs` | `&`/`^` → typed error |
| 8 | Path with spaces (`my dir/a b.md`) | `<r>/prompts/my dir/a b.md` | `my dir` | `my dir/a b-2.md` | works |
| 9 | HTTP(S) URL | `Other` "absolute() does not accept HTTP(S) URLs" (**changed**: was `remote_not_enabled`) | same message (today) | same message (today) | runtime: fetched content / `true`; failure → error (not `false`), per 3.5 ruling |
| 10 | Probe/permission error (parent dir mode `0o000`, unix) | typed `FileReference` with `Io` source | same | same; `find_*` `read_dir` failure → error | `frontmatter`: typed `Io`; `file_exists`: today `false`, target is an error |
| 11 | Absolute caller inside the projection root: `dirname(absolute(spec)) + '/review-1.md'`, spec `<r>/claudine/features/X/spec.md`, doc `<r>/prompts/review-loop.md`, `review-1.md` missing | `<r>/claudine/features/X/spec.md`; `absolute(latest_review)` = `<r>/claudine/features/X/review-1.md` | **`<r>/claudine/features/X`** (today `claudine/features/X`) | `increment_file_index(latest_review)` = `<r>/claudine/features/X/review-2.md` (today `prompts/claudine/...` when missing) | `file_exists` → `false`, then `true` after the review writes; same result when the document is outside the repo |
| 11b | Same as 11 with the sequence document outside the repository | as 11 | as 11 | as 11 | as 11 (today wrong: repository root dropped) |

Fixture notes:

- Use real temporary directories. Build expected values with
  `biscuit_file::canonicalize_simplified` and compare portably
  (`to_portable_string`) or by identity. Never compare raw `fs::canonicalize`
  output (`.claude/skills/os/windows-paths.md` items 1 and 17; macOS
  `/var` vs `/private/var` in `os/macos.md`).
- Construct Windows-native spellings (`C:\…`, `\\server\share`) only under
  `#[cfg(windows)]`. On POSIX, `C:/x` is `ForeignAbsolutePath` (malformed); on
  Windows, a rootless `/x` is `ForeignAbsolutePath` (windows-paths item 15).
- The permission-error row is `#[cfg(unix)]` with a root-bypass skip (pattern
  at F:3938 and `darkmatter/lib/tests/l1/reference_integration.rs:200`).
  Windows needs an ACL-based fixture or a documented gap.
- Public-API pattern for table tests: `ResolutionContext::new(FileResolutionContext::…)`
  plus `evaluate(&parse(src)?, &lookup)`, as in
  `darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs:68`. Caller
  provenance needs a compose-level fixture (`compose/tests/schema.rs:565`
  pattern) or `CliProcessFixture` (`compose_caller_file_provenance.rs`).

## 6. Smallest change sites

1. **`absolute_fn` (F:1566).** Add an explicit `is_remote_url` rejection (or
   call `resolve_path_shape`, which already has one), then return
   `resolve_path_shape("absolute", raw, ctx)` instead of
   `resolve_arg` + `NotFound`. Decide whether to lexically normalize the
   candidate (`./` and `..`). Options are
   `candidate_plan_with_order(ctx, CandidatePlanOrder::Resolution)`, which
   normalizes, or normalizing in `absolute` only. Changing
   `resolve_document_file_ref_shape` itself breaks the F:4778 assertion. Keep
   the native `to_string_lossy` return so existing callers are unaffected.
   Reject `%` recursive inputs (their first candidate is a directory). Update
   the function doc and the catalog entry
   (`darkmatter/docs/schemas/expression-functions.yaml:1026`,
   "Resolves a file path to an absolute path", `fallible: true`) and
   `darkmatter/docs/topics/darkmatter-expressions.md:597,724`.
2. **`dirname_fn` (F:2015).** When the normalized raw input is absolute
   (`Path::new(&normalize_path_arg(raw)).is_absolute()`, equivalently
   `FileReference::class().kind` = absolute), return
   `to_portable_string(path.parent())` regardless of projection. Otherwise
   keep the display path. That is a three-line condition change ahead of the
   existing `projected` check. Fix the doc comment at F:2012-2013 and the
   catalog/docs rows (`expression-functions.yaml:1204`,
   `darkmatter-expressions.md:614,832`, "directory portion of the display
   path").
3. **File-index outputs (F:1834-1838, F:1871-1875, and `find_index_endpoint`
   returns at F:1890, F:1910, F:1955).** Apply the same "absolute input →
   absolute portable output" rule. A shared private helper such as
   `render_path_result(raw, &path, ctx)` would serve these and `dirname`, and
   `join` (F:2137) if it is in scope. Name parsing stays as is.
4. **`file_exists_fn` (F:1587).** Only if the 3.5 ruling makes probe,
   invalid-reference, and fetch failures errors: replace `_ => false` with
   `Ok(None) => false` plus error propagation. Update F:3825, F:5521, and the
   docs (`darkmatter-expressions.md:717`).
5. **Comment drift at R:443-448** (repository-root claim). Fix it while in the
   file.

## 7. Windows and portability notes

- `absolute` returns `to_string_lossy` (native `C:\…`), while `dirname`'s
  absolute branch returns portable `C:/…`. Concatenating `dirname(...) + '/x'`
  gives `C:/r/…/x`, which `FileReference` accepts on Windows. Keep `dirname`
  portable so composed Markdown matches across OSes.
- `strip_prefix_any_spelling` (P:89) already retries through
  `canonicalize_simplified`, so a verbatim `\\?\C:` candidate or a macOS
  `/private` spelling still projects. Any new comparison must use the same
  helper, never raw `canonicalize`.
- `to_portable_string` keeps a native spelling for UNC and device paths, so
  `dirname` of a UNC caller file returns backslashes. Separately,
  `normalize_path_arg` (R:342) collapses `//server/share` to
  `/server/share`, which is drive-less and therefore `ForeignAbsolutePath` on
  Windows. A portable UNC spelling cannot round-trip through path functions
  today. Out of scope; listed as a known limitation.
- Drive-qualified fixtures on Windows: never use rootless `/w/x` paths (they
  are relative there; windows-paths item 14).
- `InvalidFilename` (`*`, `?` on Windows) is treated as a miss by the probe
  (windows-paths item 16), so `absolute("a*.md")` on a miss would shape a
  candidate on Unix but must still not be read as a glob. Do not add a
  wildcard row to the fixture table.
