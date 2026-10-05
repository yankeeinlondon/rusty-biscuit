# Composition Requests and File-Resolution Contexts

Detail behind the "Composition authority" section of [SKILL.md](SKILL.md).

Every entry point that resolves file references takes a `ComposeRequest`:
`compose_with`, `compose_preflight*`, the `collect_*` functions,
`transclusions_with_options`, `ReferenceGraphOptions::with_compose`,
`execute_directive`, and `execute_resolved_shell_values`. A request is
`ComposeRequest::prepare(options, &RequestSnapshot)` (which builds the context
through `build_resolution_context`) or `ComposeRequest::with_context(options,
built_or_derived_context)`. Preparation fixes the repository observation and
reuses it for the builder (one discovery per request), re-anchors a
`ComposeOptions::new()` context on the request directory, and makes `ctx.env`
the context's environment. Magic roots enter only through
`RequestSnapshot::with_magic_root*` (for `md`, the top-level repeatable
`--magic-root <DIR>`, applied in `main` by `request::with_magic_roots`); `ComposeOptions` has no magic paths and no
public context setter. Only binaries call `RequestSnapshot::from_process()`. DMLS builds one
context per repository (`dmls/src/context.rs`, see
[dmls.md](dmls.md#file-resolution-contexts)).
A file source (root or child) outside the request's tree is admitted
trusted-external when only that derivation validates (`source_derivation_for`,
applied to the root in `ComposeRequest::assemble`). The schema stage validates
in the source's derived context (`source_file_resolution_context`), and
`resolve_ctx::document_file_context` recognizes that context by canonical
directory, not spelling (`/var` vs `/private/var`); re-deriving it with
`for_cwd` would drop a `~` tree root.
Unit tests use `crate::markdown::compose::test_request(options)` /
`test_request_in(options, context)`; `lib/tests/l1` uses
`crate::request_support::{request, request_at, context_at, cwd_context}`.

**The context is required, never `Option`.** `ComposeOptions` is only the
settings builder and holds no context. The pipeline runs on `ComposeRequest`,
which `Deref`s to `ComposeOptions`: stages take `&ComposeRequest`, context
views live on the request (`transclusion_options`,
`source_file_resolution_context`, `*_resolution_context`,
`with_accepted_source_file`, `extended_for`), and in-pipeline builder chains
use `request.derive(|o| …)`. `request.resolution_context()` is the
file-resolution context; `request.context()` (through `Deref`) is the
captured `ctx.*` `ComposeContext`, so never name a new request method
`context`. Internal inline passes use `Markdown::compose_with_options(request)`.
`DarkmatterSchemas::new(ctx)`, `CleanSchemaConfig::new(ctx)`,
`ResolutionContext::new(ctx)`, `ReferenceGraphOptions::with_compose(&request)`,
`FileTree::new(path, &request)`, `evaluate_condition_against(expr, data, &ctx)`,
`detect_schema(.., &ctx)`, `resolve_schema*(.., &ctx)`, and
`triggers::scan(&ctx)` all take one; none has a context-free form.
Validators are either context-bound (`ValidatorCache::validator_for(schema,
base, &ctx)`) or structural (`structural_validator_for`, `build_structural_validator`) for
callers with no request (coercion probes, examples, lint): an absolute path is
judged as resolved (it must exist; `match()` judges its full path through
`GlobReference::matches_without_context`, bare and absolute patterns only), and
every value that needs a context is judged by syntax only. Root-union coercion
relies on the absolute-path half to pick an arm by glob, so a `./`/`&`/`^`/`@`
pattern cannot steer coercion.

`match()` patterns are `GlobReference`s (`FileMatchGlobs` wraps one with the
file-name view); the grammar rejects a non-glob-reference pattern at definition
time (`file_match::definition_error`). A context-bound validator judges each
value from its own `cwd`: the document's folder for frontmatter, the caller's
origin for a caller-supplied top-level property (`validate::CallerOrigins`,
from the schema stage's `caller_input_records`; the keyword finds its property
in its schema location). Never judge `match()` from the process directory or
from "any containing root". `find_files()` and `::file-links <glob>` call
`list_files` in the document's context and report skipped out-of-tree file
symlinks as `dm.glob.skipped_symlink` (`compose/glob_listing.rs`).
