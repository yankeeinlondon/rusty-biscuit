---
kind: fix
created: 2026-11-08
status: finalized-spec
area: claudine
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
human_review: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-10
review_iterations: 0
needs_rulings: false
implemented: false
---

# Per-command help for `compose`, `inline-compose`, and `sequence`

## The defect

`claudine compose -h`, `claudine inline-compose --help`, and
`claudine sequence -h` print the **top-level** grouped help screen. That
screen lists the commands and five global options. None of the switches the
three commands accept appears on that screen:

- the provider switches (`--provider`, `--claude`, …, `--exclude`);
- `--set`;
- `--dry-run` and `--dry-run-shell`;
- `--max-iterations` and `--on-rate-limit`;
- the wrapper-grade switches they share.

Provider wrappers and administrative subcommands already show their own
help: `claudine claude -h`, `claudine mcp -h`, and
`claudine actions --help`. The current [CLI guide](../../docs/cli/index.md)
accurately describes composition help as the root overview and marks
per-command help as planned. Implementation must remove that planned marker.

```text
$ claudine compose -h
Claudine
cross-agent abstraction and meta-agent harness

Usage: claudine <command> [options]
Shared Resources:
  skills   …
```

Users currently have to read the docs or the claudine-cli package's
[SharedComposeArgs](../../cli/src/commands/compose/mod.rs), which defines
the shared command options, to find a switch.

## Why it happens

The behavior is deliberate, from argv normalization rule 4
in the claudine-cli package's [help-hoisting pass](../../cli/src/argv/rule4_help_hoist.rs),
which moves a help token ahead of the command before clap parses it:

- The three composition subcommands collect their file reference, setters,
  and agent arguments with a greedy positional. A trailing `--help` would be
  swallowed by it.
- So rule 4 moves `-h`/`--help` to argv position 1.
- The claudine-cli package's [dispatch and parsing code](../../cli/src/main.rs)
  routes that root help flag to the grouped screen. On a parse failure,
  `root_help_request` can discard the command entirely so help does not
  depend on supplying a required file. Merely inspecting the parsed command
  in dispatch would therefore fail for `claudine compose --help`.

There is a second boundary before clap: the claudine-cli package's
[argument partition](../../cli/src/argv/partition.rs) separates Claudine's
options from arguments intended for the provider. It can reject an invocation
before the parser sees help. Help selection must precede that validation.

The hoist fixed a real bug: a misleading clap tip, and later a "second file
reference" error. The root screen it lands on is the part to correct.
The claudine-cli package's [argument-normalization regression tests](../../cli/tests/l1/argv_normalization.rs)
currently pin the root-screen outcome by asserting the `Composition` group
heading appears. Their comments also claim command-specific help despite
asserting the grouped screen; correct that drift with the assertions.

## What should happen

- `claudine compose -h` (and `--help`, `inline-compose`, `sequence`, with or
  without a file, setters, or provider switches before the help token) prints
  that command's own help: its usage line, positional arguments, and every
  supported public Claudine option with a concise description. Include the
  inherited presentation options and the help option itself.
- Exit status `0`, help on stdout, nothing on stderr. This matches the
  other subcommands.
- A help token after `--` is still forwarded to the agent untouched. Rule
  4's existing `--` boundary is kept.
- `claudine --help` and bare `claudine` still print the grouped screen.

## Design decisions

### Select help before validating a composition invocation

Recognize the help destination from the original arguments, before the
help-hoisting pass loses the token's position and before the argument
partition can reject the invocation. Keep that destination independently of
successfully parsed command arguments; do not fabricate a file or parse a
composition request merely to render help. Reuse the existing command-name
recognition, including its handling of root options with values, rather than
searching for a command name anywhere in the token list.

The selection rules are:

| Input | Destination |
| --- | --- |
| `claudine`, `claudine -h`, `claudine --help` | Grouped root overview |
| `claudine --help compose x.md` | Grouped root overview; explicitly requested before the command |
| `claudine compose --help`, `claudine compose x.md -h` | Compose help |
| `claudine --plain inline-compose x.md --help` | Inline-compose help without ANSI escapes |
| `claudine sequence --help -- --help` | Sequence help; the earlier help token wins |
| `claudine compose x.md -- --help` | Ordinary invocation; forward the final token to the provider |

Only exact standalone `-h` and `--help` tokens before the first literal `--`
request composition help. Preserve the existing token-based recognition:
`name=--help` and `--set='{"name":"--help"}'` are data, while an exact
standalone help token requests help even beside an incomplete option.
Repeated help tokens select one screen and exit once. Both spellings show
the same compact command reference; a separate expanded manual is not needed.

A recognized help request succeeds without validating a missing file,
unknown provider, malformed setter, conflicting options, or an unknown switch
before the file. For example, `claudine compose --provider unknown --help`
and `claudine sequence --dry-run-shell --help` both show help. Without help,
the existing validation errors still apply. Do not change version routing or
the help behavior of wrappers, administrative commands, or completion calls.

Help must not resolve or read the composition file, evaluate templates or
shells, run lifecycle actions, initialize configuration, prompt the user, or
discover or launch a provider. It must work from an isolated empty directory
with an isolated home and no installed provider. Honor `--plain` before
rendering; `--quiet`, `--silent`, and `--perf` must not suppress help or add
stderr output to a help request.

> **Reader's note:** Moving the token to the root currently solves the greedy
> positional problem but erases the distinction between root and command
> help. Preserve the destination before that rewrite rather than inferring it
> from a parsed command that may be absent. This intentionally changes the
> documented root-screen result for trailing composition help, while retaining
> the explicit `--` forwarding boundary.

### Render the existing command model

Use the selected subcommand from the same fully built clap command model
used by normal parsing, including injected help and inherited options. Ensure
the usage line includes `claudine` and the selected command name. Render that
model without parsing the user's remaining arguments. Clap's standard help
rendering is the existing convention for non-wrapper command help; do not add
a custom terminal layout or another option list.

The claudine-cli package's [print_wrapper_help](../../cli/src/commands/wrap/flags.rs)
is actually a manually maintained string, not clap-generated help. It is not
the implementation model for this fix. Leave wrapper help outside this scope.

The usage and argument descriptions must explain one required file reference,
optional `key=value` setters, and provider arguments after the file, including
the explicit `--` boundary. Preserve the existing positional parser rather
than splitting its greedy collector solely to improve the display. A compact
generated-help footer may illustrate the syntax with a command-specific
example; it must not duplicate the option reference.

### Make the exposed reference complete and accurate

- **Doc comments become user text.** Clap renders the `///` comments on
  the shared options. Give each rendered field a concise user-facing
  description. Remove maintainer history from rendered text, including the
  retired `YOLO` environment-variable discussion. Keep useful user warnings:
  dry run can still execute composition shell expansions, and
  `--dry-run-shell` executes shell tasks for real and requires `--dry-run`.
  Internal skipped fields remain omitted. Correct comments alongside any
  behavior change; avoid unrelated comment cleanup.
- **Provider shortcuts must match accepted input.** The help fields currently
  omit `--kilo`, `--pi`, and `--antigravity`, although the normalizer accepts
  them from the provider catalog. Add their help declarations using the
  existing shared-option pattern. Check that every catalog-derived provider
  shortcut has a visible help entry. This documents existing accepted input;
  it does not introduce new providers or flags.
- **No provider-native option inventory.** List Claudine's options and explain
  forwarding; do not merge the changing native provider help screens into
  this reference. Preserve visible aliases such as `--asp`, `--rsp`, and
  `--op`, and the canonical values for enum options.
- **Shell completion shares the model.** Preserve existing candidates and
  ownership of arguments after the file. Adding the missing provider help
  declarations can legitimately expose those already-supported shortcuts to
  completion; this is the only intended candidate expansion. Verify help
  routing does not execute when the shell is requesting completions.

## Implementation and verification

### Phase 1: Route and render help

Implement destination selection and clap-generated command help, then update
the rendered option and positional descriptions and missing provider entries.
Keep normal execution and the provider forwarding boundary intact.

### Phase 2: Verify the public behavior

Use ordinary L1 tests; a real terminal, real provider, network service, and
performance spike are unnecessary. Apply the repository's `rust-testing`
skill and `CliProcessFixture` isolation. Put the selection edge cases and
catalog/help consistency checks in-process; use a small set of real CLI
invocations to prove rendering, channels, and routing are wired together.

- Exercise all three commands with both help spellings, with and without a
  file. Assert the correct command-specific usage, positional description,
  `--help`, `--provider`, `--set`, `--dry-run`, `--dry-run-shell`,
  `--max-iterations`, and `--on-rate-limit`; root command-group headings must
  be absent. Retain the original interleaved-provider/setter regression input
  and its assertions against the clap tip and second-file error.
- Cover the destination table, repeated tokens, embedded help text in setter
  data, incomplete/invalid command arguments, and `--plain` before and after
  the command. Assert exit 0, nonempty stdout, and empty stderr for help,
  including when combined with `--silent` or `--perf`. Use the shared ANSI
  stripping helper to verify `--plain` emits no escape sequences.
- Use a nonexistent file for ordinary help. Add one portable trap fixture
  with a document lifecycle action and a fake provider launch marker; assert
  help changes neither marker nor input file and creates no user config.
  Keep audio disabled using the existing fixture policy.
- For help after `--`, prove forwarding with the existing portable fake
  provider fixture and assert the received argument. Do not spawn a real
  provider merely to test forwarding. Keep root help and representative
  wrapper/administrative help regression coverage.
- Strengthen the existing removed-sequence-flag help test so it first proves
  it received sequence-specific help, then checks that `--fail-fast` is absent.
- Extend existing completion checks for the missing provider shortcuts and
  retain representative checks for normal completion and provider-owned tails.

During implementation, run `just test-cli` and `just lint` from `claudine/`
using the canonical recipes. No additional CI environments or gates are
needed for this fix; keep the implementation and fixtures portable across
macOS, Linux, native Windows, and WSL2.

### Phase 3: Update the current documentation

Update the [CLI guide](../../docs/cli/index.md) and
[CLI README](../../cli/README.md) with the resulting help behavior, keeping
the README to an overview. Correct the detailed help-routing explanations in
[argument normalization](../../docs/topics/argv-normalization.md) and
[CLI pre-parsing](../../docs/topics/cli-pre-parsing.md), including examples
and diagrams that currently say composition help always reaches the root.
Update the claudine skill's affected parsing guidance alongside the code.
The docs must explain root help, command help, and forwarding with `--`
directly, without referring readers to this specification.

## Acceptance

1. Each of these prints the compose (or inline-compose, or sequence)
   screen, exits 0, and lists `--dry-run`, `--set`, and `--provider`:
   - `claudine compose -h`
   - `claudine compose x.md --gemini name=Ken --help`
   - `claudine inline-compose --help`
   - `claudine sequence -h`
2. `claudine compose x.md -- --help` forwards `--help` to the agent, as today.
3. `claudine --help` is unchanged.
4. No rendered help line is a maintainer note: no history, no type paths,
   no references to internal modules.
5. Missing or invalid command arguments do not prevent a recognized help
   request from displaying the selected screen. Explicit root help before a
   command still displays the root overview.
6. Help has no composition or provider side effects, requires no configured
   provider or existing file, and respects `--plain`.
7. All accepted provider shortcuts appear in help, and completion changes are
   limited to exposing the previously missing shortcuts.
8. The CLI guide, README overview, parsing topic pages, skill guidance, and
   touched source/test comments describe the resulting behavior accurately.

## Open Questions

None. The decisions above preserve the existing forwarding and root-help
contracts while making the intended command-help change explicit.
