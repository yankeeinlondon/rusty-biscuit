# Testing Tree Hugger

This page is for contributors: how to run Tree Hugger's tests, what each test
level covers, where the fixtures live, and what a change to queries or
extraction is expected to test.

## Running the tests

Run every command from `tree-hugger/`. The recipes cover both packages, the
`tree-hugger` library and the `tree-hugger-cli` binary (`hug`), and run
through [cargo-nextest](https://nexte.st):

```bash
just test       # L1: library and CLI unit and integration tests
just test-l2    # L2: the CLI's styled output in a real terminal
just lint       # Clippy over both packages
```

Extra arguments are passed on to nextest, so you can narrow a run with a
[filter expression](https://nexte.st/docs/filtersets/):

```bash
just test -E 'binary(symbols)'    # only the CLI's `symbols` test binary
```

A few more recipes are useful before pushing:

| Recipe | What it runs |
|---|---|
| `just sanity` | A fast subset of the library and binary tests |
| `just doctest` | The examples in the Rust API docs |
| `just bench` | Criterion benchmarks for symbol extraction |
| `just all` | `sanity`, `lint`, `doctest`, `test` and `test-l2`, in that order |

## Test levels

The monorepo sorts tests into levels by what they need from the machine:

| Level | Needs | In Tree Hugger |
|---|---|---|
| **L1** | Nothing beyond the source tree | Almost every test: extraction per language, cache behavior, CLI output against golden files |
| **L2** | A real terminal (tmux, WezTerm or Kitty) | The CLI's colored, wrapped and table output for the declaration commands, `refs` and `god-files`, read back from a live terminal pane |
| **L3** | OS keyboard injection | Not applicable |

L1 tests can only check the bytes `hug` writes to a pipe. L2 tests run `hug`
attached to a terminal, which is the only way to check what a user actually
sees: that `hug` detects the terminal and colors its output, and that wrapping
and tables fit the pane. The L2 targets are behind the CLI's internal
`terminal-tests` feature, which `just test-l2` (and CI) enables; an ordinary
`just test` leaves them out.

## Where the tests and fixtures live

```txt
tree-hugger/
├── lib/tests/
│   ├── l1/                       # library integration tests
│   └── fixtures/
│       ├── sample.*              # basic syntax per language (functions, classes)
│       ├── types.*               # type-system constructs (struct, enum, interface, trait)
│       ├── generics.*            # generic and parameterized types
│       ├── imports.*             # import forms per language
│       ├── signatures.*          # signature rendering
│       ├── symbols/              # one primary file per language, plus extras
│       │   ├── multi/            # a 16-language tree for multi-file and golden tests
│       │   └── test_files/       # one file per "is this a test file?" naming rule
│       └── lint/                 # lint fixtures
└── cli/tests/
    ├── cli.rs                    # command-line behavior
    ├── symbols/                  # declaration commands, cache, exports, goldens
    ├── changed/                  # `--changed` file selection
    ├── level2_*.rs               # L2 real-terminal tests
    └── fixtures/
        ├── golden/               # expected `hug` output, one file per format
        └── declarations/         # input for the L2 declaration tests
```

`lib/tests/fixtures/symbols/<language>.<ext>` is the main fixture for each
language: kind, visibility and `static` expectations are written against it,
and a test fails when a language has no primary fixture or its fixture stops
parsing cleanly.

`hug` never scans files under a `fixtures/` directory, so CLI tests copy the
fixtures they need into a scratch Git repository first and run `hug` there
with an isolated home and temporary directory (which is where the persistent
cache lives).

### Golden files

The files in `cli/tests/fixtures/golden/` pin `hug`'s output over the
`symbols/multi/` tree in several formats (`plain`, `json`, `jsonl`, `table`,
`markdown`, `ctags` and more). When you change output on purpose, regenerate
them and review the diff:

```bash
HUG_UPDATE_GOLDEN=1 just test -E 'binary(symbols)'
git diff cli/tests/fixtures/golden/
```

Never regenerate goldens to make an unexplained failure go away; a golden diff
is a change users will see.

## What a change should test

When you change a [query](./queries.md) or the extraction logic, cover:

1. **Type distinction.** Different type constructs (struct vs. enum, class vs.
   interface) must come out with distinct kinds. Every language with type
   constructs has a `types.*` fixture that exercises all of them.
2. **Cross-language parity.** A typed language gets fixtures for its type
   system, like the others.
3. **Regressions.** A bug fix comes with a test that fails without the fix.
4. **Diagnostics.** Lint and syntax diagnostics have regression tests for
   representative languages.
5. **Re-exports.** A change to forwarding extraction keeps the report equal
   across the direct call, the complete index, a selective query and the
   cache (cold, hit, narrowed, widened, another scan root), and the work
   counters show one extraction and no reference or diagnostic work for a
   report-only request. The per-language rules are in
   [Re-exports](./reexports.md).

Every query for every language is compiled by the L1 tests, so a broken
pattern fails `just test` rather than a user's run.

## Related pages

- [Tree-sitter Queries](./queries.md): the query files these tests exercise
- [Symbol Kinds, Visibility and `static`](./symbol-kinds-and-visibility.md):
  the per-language results the fixtures pin
- [Tree Hugger's Persistent Symbol Cache](./cache-design.md): the cache
  behavior the cache tests check
- [Using the `hug` CLI](./cli.md)
