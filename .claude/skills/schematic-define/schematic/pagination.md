# Pagination authoring

Canonical behavior: `schematic/docs/pagination.md`; validation rules:
`schematic/docs/generation/validation.md`. Types live in
`schematic_define::pagination` (most in the prelude; `Relay` is not). GraphQL
Relay connections are in [`responses.md`](./responses.md#relay-pagination).

Use API `.default_pagination(strategy)` and endpoint `.pagination(Collection)`.
There are no API exception tables or parameter-level legacy pagination fields.

- `pagination::resolve(api, endpoint)` uses the endpoint collection first;
  otherwise a GET whose only success shape is JSON typed `Vec<T>` inherits the
  API default. Wrappers, opt-outs, overrides, multi-shape lists, and non-GET
  lists need an explicit collection:
  `.pagination(Collection::at("data").item_type("Item"))` or
  `.pagination(Collection::root().unpaginated())`. `at("")` is invalid.
  Explicit collections keep the serialized origin `ResolutionOrigin::Exception`.
- `caller_knobs` is exactly what the generated call exposes; alternates
  (`or_paginate`) are modeled and validated but add no knob.
- `param_adoptions`/`adopted_params`: a strategy input with the same location
  and name as a declared param adopts it (removed from the request struct)
  only when the strategy authors the matching knob (a cursor part needs
  `.resumable()`); otherwise `adoption-without-knob`. A required adopted size
  needs `PageSize::sent_default(n)` (type-valid, else
  `pager-managed-default-invalid`); `server_default` is documentation only.
  The generator writes `ManagedRole::sent_default` into `into_parts` for a
  required adopted param, so an ordinary send still carries it; the pager
  replaces it.
- The adopted page-size param's effective bounds come from the strategy
  (`page_size_constraints(&PageSize)`, `adopted_page_size(api, ep)`); a
  declared param must repeat exactly `minimum = min`/`maximum = max` or
  declare none, and no other value rule (`format` allowed), else
  `page-size-constraint-conflict` (`PaginationError::PageSizeConstraintConflict`).
  The runtime pager keeps its own page-size check.
- `validate_pagination` feeds `schematic-gen`'s `validate_rust_client` as the
  `pagination-strategy` / `pagination-exception` / `pagination-parameter`
  features.
- Definitions keep one API-local strategy helper (`github_page_number()`,
  `bitbucket_next_link()`, ...) and attach collections to endpoint builders.
  Authored: GitHub, GitLab, Gitea, Bitbucket, OpenAI (through the
  `openai_pagination` overlay), and ElevenLabs `GetHistory`/`ListSharedVoices`
  (93 pagers; adoption must preserve their controls). Unverified providers
  (Anthropic, EMQX, Hugging Face, GitLab participants, Gitea trees) stay
  unpaginated: a GET `Vec<T>` under a default needs
  `Collection::root().unpaginated()`, and a wrapper gets an `.unpaginated()`
  collection so it reads as a list.
- `schematic/definitions/tests/pagination_coverage/` is exhaustive: every
  endpoint resolving as a list needs a row (origin, items, item type,
  strategy description, exact knobs, adopted fields).

## Generated pager surface

A paginated endpoint generates `impl schematic_runtime::pager::Paginated`
(`&'static` strategy constants) plus one capability marker per knob. Its call
type forwards `pager()`, each authored knob (`page`, `offset`, `after`,
`before`, `page_size`, `keep_alive`), and `pages()` / `fetch_page()` /
`collect_all()`, or `with_session(|scope| Box::pin(async move { .. }))` for a
strategy with a `Session` (pages, `partitioned`, and `renew` only inside the
scope, which closes with the latest handle under its own grace,
`close_grace(..)`, default 5 s). A partition without a session forwards
`partitioned(n)`. Knobs return the runtime `Pager`, so endpoint setters come
first. A remaining setter named like a forwarded knob becomes `<name>_param`.
Traversal and session logic live only in `schematic-runtime`'s `pager` module.
`Paginated::page_root` (default: the whole 2xx body) picks the value item and
continuation paths read; GraphQL operations override it.

A session with partitions must use `Session::opened_by(..)` (a separate open
request). `Session::opened_by_first_page()` is valid only without partitions;
validation reports `partitioned-first-page-session` on defaults, overrides,
and alternates. Shared partition sessions retain the latest observed handle,
including rotating handles, and close once.

## Committed-crate evidence

Run only by `just test-schema` (CI does not build `schematic-schema`):
`schematic/schema/tests/pagination.rs` pages GitHub `ListReleases`, OpenAI
`ListBatches`, and Bitbucket `ListTags` against wiremock, and
`compile_fail.rs` runs trybuild cases on real endpoints (`trybuild` is a
generated dev-dependency in `gen/src/cargo_gen.rs`). Some `.stderr` files list
rustc's first eight implementors of a capability trait, so adding a paginated
endpoint can change them: refresh with `TRYBUILD=overwrite cargo nextest run
--manifest-path schematic/schema/Cargo.toml --test compile_fail` and review the
diff.
