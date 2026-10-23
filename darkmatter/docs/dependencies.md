# Darkmatter Dependencies

## `serde_json` Features

`darkmatter` declares three `serde_json` features itself rather than relying
on feature unification from other crates:

- `preserve_order` keeps every JSON map in written (insertion) order, so
  frontmatter values, `with:` overlays, expression results such as
  `as_json5`, and SimplifiedSchema → JSON Schema output keep the order the
  author wrote. Cargo unifies the feature into every package that links
  `darkmatter`. Outputs that publish alphabetical keys sort explicitly:
  `md schema validate --format json`, the non-strict frontmatter hash used by
  `md hash`, Claudine's OpenCode/Kilo inline config, and `sniff`'s JSON
  output. Insertion order is not always source order: a map a function
  builds keeps the order it was built in. A new consumer that needs a stable
  order sorts explicitly rather than relying on the map.
  - Written order is intentional where a reader sees it:
    `md schema detect --format json` lists properties in the order the
    documents wrote them (with `$schema` first), a mapping converted to text
    for a `json`/`yaml` property or a `json` parameter keeps its key order
    (`{ b: 1, a: 2 }` becomes `{"b":1,"a":2}`), and `keys()`/`values()`
    return a map's entries in that order.
  - Interoperability: the compiled JSON Schema is standard Draft 2020-12
    plus Darkmatter's custom `format`s (`darkmatter-numberlike`,
    `darkmatter-boolish`, `darkmatter-json`, `darkmatter-yaml`, and others)
    and the `x-darkmatter-integer` keyword. Key order does not change what a
    schema means, so another validator reads it the same way, but a
    validator that ignores unknown formats and keywords accepts more than
    Darkmatter does.
- `raw_value` and `float_roundtrip` let the JSON loaders read each number's
  written text and parse it exactly, which numeric admission needs (see
  [Numbers](./topics/numbers.md)).

## Remote URL Referencing

- `biscuit-file` with the `fetch` feature supplies the shared HTTP fetch
  primitive and host allowlist policy used by compose remote reads and
  side-effect network writes.
- `reqwest` provides the shared HTTP client for compose remote fetches and the
  side-effect `http_post` verb.
- `tokio` runs remote fetch tasks and the blocking wrapper used by synchronous
  callers.
- `url` parses and normalizes HTTP(S) references before policy checks.

These dependencies are required so every network egress path goes through the
same scheme validation and deny-all-by-default host policy.

## Tier-only Test Dependencies

- `browser-tests` enables `biscuit-browser-harness`, `chromiumoxide`, and
  `futures-util` for the headless-browser and OS-input browser targets.
- `terminal-tests` enables `image` for the real-terminal image fixtures. The
  shared terminal harness remains available to L1 because `render_invariants`
  uses its pure layout-invariant helpers without opening a terminal.
- The CLI's separate `terminal-tests` feature enables its terminal harness.
- The CLI's `test-fixtures` feature enables `test-toolkit` as an optional
  dependency for one target, the `darkmatter-argv-recorder` fixture binary
  (`test_toolkit::argv_recorder`), which the shell template tests run as a
  native program. `just test` and CI enable it; `md`'s install never builds
  it. It adds no crate: `test-toolkit` is already a dev dependency.

The corresponding integration targets declare `required-features`, so an L1
filter does not first compile binaries and native/browser dependencies that it
will never execute. CI enables the tier features for its reusable all-tier
build; `just test-l2`, `just test-l3`, and `just test-browser` enable only the
features their tier requires.

## Source-guard Test Dependencies

- `toml` (test-only, `darkmatter` and `darkmatter-cli`) — the shared
  `path_lookup_guard` engine reads each package's `[lib]`, `[[bin]]`, and
  `[package].build` with a real TOML parser, so every spelling Cargo accepts
  selects the same sources and a malformed manifest fails the guard. Every
  other package that includes the engine already depends on `toml`.

## Content Policy Schema Dependency

- `content-policy` (`darkmatter`, `default-features = false`) — supplies
  `EDITOR_SCHEMA`, the Content Policy editor schema that the runtime baseline
  imports to type `content_policy`. The library compiles it in beside the
  baseline, so an installed binary prepares the baseline without schema files
  on disk. Tests also run the shared Content Policy declaration cases through
  `Policy::from_declaration` and the baseline validator, so each case states
  where the parser and the editor schema agree and where they differ. The
  dependency runs one way: Content Policy never depends on Darkmatter, and
  `just deps-check` in `content-policy/` fails if it ever does. The
  `file-adapter` feature stays off.

## Execution Identity and Network Context

- `biscuit-hash` with the `blake3` feature computes `ctx.sid`, the full BLAKE3
  digest of the same length-prefixed inputs `ctx.id` hashes with xxHash.
- `getrandom` draws the 16-byte per-execution nonce shared by `ctx.id` and
  `ctx.sid` from the OS CSPRNG. It reports failure as a typed error, so an
  entropy failure becomes a compose error instead of a weaker nonce.
- `ipnet` parses and matches CIDRs for the `ipv4`/`ipv6` filters and the ICMP
  address grants. It is already in the tree through `sniff`.

## Login-shell Probes

- `windows` (Windows only, `Win32_Foundation` and `Win32_System_JobObjects`)
  places each `has_alias`/`has_builtin_function`/`has_user_function`/
  `can_execute` shell query in a kill-on-close Job Object, so a PowerShell
  profile cannot leave processes running after the query ends. Unix uses the
  existing `libc` dependency for the equivalent `setsid` session and
  process-group kill.

## Compose Text Replacement

- `aho-corasick` powers the `replace:` map matcher in
  `markdown/compose/replacement.rs`: a `MatchKind::LeftmostLongest` automaton
  gives a single linear pass with the same leftmost / non-overlapping /
  longest-key-wins / no-rescan contract the previous hand-rolled per-character
  scanner had, but without its `O(content × rules × keylen)` cost. Already
  compiled transitively via `regex`, so it adds no build cost.

## Literal Tokens

- `base64` encodes the payload of the `{{!data:v1:…}}` literal token in
  `markdown/literal_token.rs`. The token stores one frontmatter string as data,
  so a later compose never scans it. The unpadded URL-safe alphabet holds no
  brace, quote, `$`, `:`, or whitespace, so the token cannot close early, open
  `$(`, or need YAML escaping. Canonical decoding rejects padding and stray
  trailing bits, so each string has exactly one token. Already in the tree
  through several workspace crates.

## Expression List Serialization

- `biscuit-file` with the `json5` feature supplies `json5::to_json5_compact`,
  which backs the `as_json5(…)` expression function. The feature is named
  explicitly in `lib/Cargo.toml` rather than relied on through
  `biscuit-file`'s defaults, so the call site survives a change to that
  default set. JSON output uses `serde_json` directly; no second JSON5
  implementation is carried here.

## Shell Directive Execution

- `shared_child` backs the child-process wait in
  `markdown/compose/shell_expansion/executor.rs`. Both executor variants block
  on an OS wait event (a helper thread calls `SharedChild::wait`, the caller
  consumes it with `recv_timeout`) instead of the previous 10 ms
  `try_wait`/`sleep` poll loop, so a fast command's completion is observed
  immediately rather than up to one poll interval later. `SharedChild` is what
  makes the timeout arm safe: it can kill and reap through the same handle the
  waiter thread is blocked on, which `std::process::Child` cannot express.
  Depended on with `default-features = false` — the default `timeout` feature's
  `wait_timeout` is built on a process-wide SIGCHLD handler (pulling
  `sigchld`/`signal-hook`), and Darkmatter is a library that must not hijack a
  host application's signal disposition. The ungated `wait`/`kill` core we use
  is `waitid`/`WaitForSingleObject`-based and installs nothing, so Darkmatter
  carries no platform split of its own.
- `libc` (Unix only) supplies `setsid` for the alias-lookup child in
  `markdown/compose/shell_expansion/alias.rs`. That lookup must run the shell
  interactively to see rc-file aliases, and an interactive shell left on the
  caller's controlling terminal stops itself with `SIGTTIN` whenever it is not
  the terminal's foreground process group — a hang, not an error. A new session
  is the only thing that detaches the controlling terminal; a new process group
  alone does not. One syscall, so the raw binding rather than a wrapper crate.

## DMLS (`darkmatter/dmls`)

The Darkmatter Language Server keeps its dependency surface small and
protocol-focused:

- `lsp-server` — stdio framing, the synchronous `Connection` loop, and
  `Connection::memory()` for in-memory L2 session tests.
- `lsp-types` — LSP 3.17 protocol types (0.97; its `Uri` carries no
  file-path conversion).
- `crossbeam-channel` — the channel family `lsp-server`'s `Connection`
  exposes; also the worker-pool channel from Phase 3 on.
- `line-index` — rust-analyzer's line/offset index behind the `source_map`
  module; its types never leak past that module.
- `rlsp-yaml-parser` — lossless, byte-span YAML 1.2 parse behind the Phase-7
  `overlay::frontmatter::FrontmatterAst` facade (AD-4 / R-3); its types never
  leak past that module.
- `url` — battle-tested `file://` URI ↔ path conversion (percent decoding,
  Windows drive letters) that `lsp-types` 0.97 lacks.
- `toml` — `.dmls.toml` config parsing.
- `serde` / `serde_json` — LSP payloads and the config overlay merge.
- `thiserror` — error types.
- `tracing` / `tracing-subscriber` — logging to stderr or `--log-file`
  only; stdout is reserved for LSP framing.
- `ignore` — gitignore-aware workspace discovery walk (Phase 3); symlinks
  are not followed (the R-8 v1 policy).
- `globset` — include/exclude glob matching over discovered paths (Phase 3).
- `unicode-normalization` — NFC normalization of wiki-link targets and logical
  paths (Phase 5 / R-8), so a vault resolves identically on macOS, Windows, and
  Linux. Already in the tree transitively via `url`/`idna`.
- `rusqlite` (`bundled`) — the persistent per-file facts store that
  references, rename, file rename, and workspace symbols narrow their
  candidates with. Bundled SQLite gives every OS the same 3.45 build, which
  supports `STRICT` tables; the same version and feature `claudine/lib` uses.
- `darkmatter` (lib) — the semantic authority (span vocabulary,
  frontmatter block extraction, single-document reference extraction, and the
  later-phase parsing surfaces).
- `biscuit-file` (`file-reference` feature, default features off) —
  file-reference resolution conventions, and the Git discovery that keys the
  per-repository context cache.
- `biscuit-hash` — xxHash content-hash identity for the Phase 3 invalidation
  engine (`WorkspaceIndex`) and the rescan's package-manifest fingerprints.
- `sniff` — `PACKAGE_MANIFEST_FILE_NAMES`, the manifest names DMLS watches so
  adding a package rebuilds the repository's context. Already compiled through
  `darkmatter`.
- `wait-timeout` (test-only) — bounded, cross-platform reaping for the real
  `dmls` stdio subprocess lifecycle test.
- `tracing-test` (test-only) — scoped log capture for the context-build
  failure log.

## DMLS Zed CLI (`darkmatter/dmls/zed-dmls-cli`)

- `sniff` captures the host OS and the `dmls` executable lookup once per
  invocation.
- `dirs` supplies cross-platform per-user data and log directory roots.
- `biscuit-terminal` supplies `TerminalRenderable` output for staging and
  diagnostics.
- `clap` defines the `stage` and `doctor` command surface.
- `toml` validates the staged and registered extension manifests and reads the
  DMLS package version for the helper's build-time compatibility contract.
- `wait-timeout` bounds the `dmls --version` probe.
- `thiserror` and `color-eyre` provide typed library and CLI error reporting.

## CLI (`darkmatter/cli`)

- `serde` (`derive`) — the `md clean --json` diagnostic envelope. The
  per-diagnostic body is `biscuit_file::YamlDiagnostic`'s own `Serialize`, so
  the wire format stays pinned to the shared diagnostic vocabulary; the derive
  is only needed for the CLI-local envelope and stage tag that wrap it.
- `ctrlc` — the Ctrl-C handler of a lifecycle `md compose` run: the first
  interrupt cancels the run so its `cancelled` and `finalize` stacks still run
  and it exits 130, and a second one exits at once. Installed only for a
  document that authors a lifecycle event.
