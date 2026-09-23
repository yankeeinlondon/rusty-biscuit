# Phase 1 Impact Analysis

Captured on 2026-07-18 before any production-code edit. The local GitNexus
index was refreshed at commit `d672388`. The MCP daemon did not register the
`more-is-more` worktree, so MCP impact queries used the indexed `darkmatter`
worktree at the same commit. The freshly indexed local worktree remains the
authority for later change detection.

`sniff repo packages`, `sniff repo package-areas`, and
`sniff repo package-dependencies` identify `darkmatter`, `darkmatter-cli`, and
`dmls` as the affected packages in the Darkmatter area. GitNexus additionally
identifies Claudine schema discovery as a downstream consumer of the public
schema parser. Verification must therefore include the Darkmatter library and
CLI plus DMLS; any later public parser behavior change must also exercise the
identified Claudine consumer.

## Upstream blast radius

| Slated symbol | Risk | Direct | Total (depth 3) | Principal consumers |
|---|---:|---:|---:|---|
| `parse_property_def` | CRITICAL | 1 | 58 | `parse_schema_shape`, simplified grammar tests |
| `parse_yaml_schema` | CRITICAL | 56 | 256 | base schema, resolver, compose/catalog parsing, source parser, CLI, Claudine discovery |
| `parse_type_expr` | CRITICAL | 47 | 196 | simplified grammar, serialization, about/catalog tests |
| `resolve_reference` | HIGH | 3 | 45 | recursive schema resolution and resolver tests |
| `is_bare_name` | HIGH | 3 | 25 | reference classification and schema-root lookup |
| `SimplifiedType` enum/impl | LOW | 0 | 0 | GitNexus did not resolve enum-variant matches; textual consumers remain in scope |
| `type_fragment` | LOW | 1 | 22 | simplified-to-JSON-Schema lowering |
| `register_darkmatter_formats` | HIGH | 3 | 82 | custom validation registration and compilation tests |
| `build_validator` | CRITICAL | 55 | 147 | all compiled-schema validation paths |
| `DocumentOverlay` | HIGH | 1 | 32 | `OverlayState::for_document`, hover/completion/diagnostic session flows |

The only indexed execution flow attached to `parse_yaml_schema` was the CLI
render path: `run_subcommand` → `run_render` → `load_markdown` →
`read_from_stdin` → `Markdown::try_from_content` → `Markdown::from` →
`Markdown::new`. This is not exhaustive because direct library consumers are
represented as call relationships rather than named processes.

## Risk handling and verification scope

The HIGH/CRITICAL risks were reported before test edits. Phase 1 changes only
test and feature-documentation files; no impacted production symbol is edited.
Later phases must run a fresh upstream impact query immediately before each
production symbol edit and preserve these gates:

- `darkmatter`: parser, compilation, validation, resolver, baseline schema,
  serialization/property tests, and CLI schema command tests.
- `dmls`: LSP session tests covering hover, completion, diagnostics,
  standalone activation, and last-good retention.
- `darkmatter-cli`: `md schema about`, validate, compose, and detect behavior.
- downstream Claudine schema discovery when the public parser contract changes.

Cross-platform design review must keep path classification lexical and based on
`biscuit_file::FileReference`; semantic validation must not depend on whether a
path exists on macOS, Windows, or Linux.
