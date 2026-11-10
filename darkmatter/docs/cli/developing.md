---
kind: canonical-documentation
---

# Developing the CLI

This page is for changing `md` itself: where each part of the command line lives, and
how a test launches the binary. To use `md`, start at [the CLI index](./index.md).

## Where the code lives

The user-facing surface is defined under `darkmatter/cli/src/args/`:

| File | Holds |
|---|---|
| `cli.rs` | global flags |
| `command.rs` | subcommands |
| `target.rs` | command targets |
| `render.rs` | the `RenderOptions` shared by default and explicit rendering |
| `output.rs` | the `--output` and `--width` arguments, and `COMMAND_LEAVES`, the record of what each command declares |
| `policy.rs` | the `--allow`, `--deny`, and `--disable` identifiers |
| `enums.rs`, `wrappers.rs`, `parsers.rs` | value enums, CLI conversion types, and value parsers |
| `completion.rs` | shell completion helpers |

Runtime dispatch is `src/commands/mod.rs`, with one implementation per command beside
it. Shared input loading is in `src/io/`, output artifact handling for `--show` is in
`src/artifact.rs`, terminal rendering setup is in `src/render.rs`, and command-line style
(`--style` files and `style…=VALUE` setters) is prepared in `src/render_style.rs`.

When a command's data output changes, its schema file changes with it; see
[How the shapes are enforced](./data-contracts.md#how-the-shapes-are-enforced).

## Tests that launch `md`

L1 tests that launch the shipped `md` binary must construct a `CliProcessFixture` from
`cli/tests/common/fixture.rs` and build the command through its `command()` or
`command_builder()` surface. The fixture owns the child's disposable working
directory, home, config, cache, temporary directory, Git plumbing, rendering inputs,
and PATH through process completion, so a test cannot read the state of the machine it
runs on.

```rust
let fixture = CliProcessFixture::new();
let output = fixture.command().args(["toc", "doc.md", "--output", "json"]).output().expect("md runs");
```

Use `host_path()`, `fake_only_path()`, `ambient_context()`, or `inherit_no_env()` only
when the behavior under test requires that named policy, with a call-site comment
stating the reason.

The fixture also pins deterministic rendering and Darkmatter application inputs. A test
whose subject is one of them declares it on the builder rather than setting it on the
built command: `rendering_input(key, value)`, `rendering_input_removed(key)`,
`application_input(key, value)`, `application_input_removed(key)`, or
`plain_terminal(columns, lines)` for the whole fixed-size, `TERM=dumb`, no-color frame.
The home, config, cache, temp, and Git plumbing variables are a different class: they
exist to keep host state out of the child, so they have no declared override; use
`ambient_context()` when the launch context itself is the subject.
`cli/tests/common/protected_env.rs` holds the classification.

Do not modify the working directory, PATH, or environment clearing after `build()`, and
do not `.env`/`.env_remove` a protected key there. The L1 spawn and isolation guard
rejects raw spawns, all of those overrides, and stale exemptions.
