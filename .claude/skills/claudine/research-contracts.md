# Research Contracts and Fleets

How Claudine's provider research is specified, run, and checked. Read this
before writing or changing a research contract, a fleet prompt, or the
`research` recipe.

Reasoning level (`claudine/docs/research/reasoning-level/`) is the reference
topic. Copy its files when starting or narrowing another topic.

## What a topic consists of

| File | Holds |
| --- | --- |
| `docs/research/<topic>/_schema.yaml` | The properties of a research document |
| `docs/research/<topic>/_types.yaml` | The topic's named types |
| `docs/research/_types.yaml` | Types every topic shares: identifier, version, pointer, evidence, the researchers |
| `docs/research/<topic>/_relations.py` | The relations gate, until one configurable checker replaces it |
| `docs/research/<topic>/_fleet.md` | The prompt a researcher receives, with the lifecycle events that check its work |
| `docs/research/<topic>/<slug>.md` | One research document per provider |

## Running research

```sh
cd claudine
just research <topic>              # every provider in the roster
just research <topic> only=kilo    # one provider, for a pilot or a repair
```

The recipe starts one run per researcher, in parallel. When a researcher
reaches its usage limit, the others cover for it in rotation order. It ends
by checking that every rostered provider has a document that carries the
contract's revision and the fleet's stamp.

Logs are under `<repo>/.claudine/tmp/research-runs/<topic>/<time>/`.

## The contract standard

1. **Narrow types.** A closed set is an `enum`. An identifier, version, field
   path, flag, or token has a `pattern`. A plain `string` is for text a
   person reads, and is `not-empty`. `unknown` is a member of a set wherever
   research may fail to establish the fact.
2. **Named types, nested notation.** Every object is a named type in
   `_types.yaml`, one property per line. Never a quoted `{ ... }` string.
3. **A description on every property** that says something the name and type
   do not. Descriptions appear in validation errors, so the researcher reads
   them when a document is rejected.
4. **Lifecycle events communicate and validate.** See the reference prompt.

Check a contract with the lint:

```sh
python3 features/2026-09-29-update-research/prototypes/contract_description_lint.py \
    docs/research/_types.yaml docs/research/<topic>/_schema.yaml docs/research/<topic>/_types.yaml
```

## Limits of the schema grammar

Found with `md` 0.1.0. Several differ from the Darkmatter documentation.

| Limit | Do this |
| --- | --- |
| A `pattern(...)` with parentheses or a space does not parse | Use character classes: `^[a-z][a-z0-9-]*$`, `[.]` for a dot |
| A multi-line inline object does not parse | Use a named type |
| A file with `kind: schema` rejects `$schema` | Keep the contract and its types in two files |
| A mapping nested directly in another accepts unknown keys | Use a named type, which is closed |
| A description containing a colon followed by a space breaks the file | Reword it |
| `json` reads a bare string as encoded JSON | For a scalar, use a union of `string`, `number`, `boolean` |
| An array of unions is not supported | Give the object optional properties instead |
| `yes`, `no`, `on`, `off` are truth values to a YAML 1.1 reader | Avoid them in a closed set; read documents as text in Python with `yaml.BaseLoader` |

Reference a type with `name(required)@this`, `name[](required)@this`, or
`name@../_types.yaml`.

## The two gates

| Gate | Command | Checks |
| --- | --- | --- |
| Shape | `md schema validate <doc> --no-trigger-schemas` | Types, closed sets, patterns, unknown keys |
| Relations | `python3 docs/research/<topic>/_relations.py <doc>` | An identifier names an existing entry; a property required only in some cases; every `unknown` has a gap |

Both run in the prompt's `success` event. A failure raises an error, the
run becomes a failure, and `finalize` retries once with the findings shown
to the researcher.

## Researchers

The rotation is written once, in the prompt's `assigned` property, by
position in the roster. It is ordered so that no agent researches its own
provider. The recipe and `docs/research/_types.yaml` repeat the models and
efforts; change all three together.

A document records the researcher that wrote it. A model that has left the
rotation stays in the contract while a document it wrote remains.

## Traps

Each of these cost a failed run.

| Trap | Rule |
| --- | --- |
| A dry run composes prompts and fires no lifecycle event | Prove a prompt with a real run. For events alone, use a copy of the prompt with a one-line task |
| A lifecycle command may not take its executable from interpolation | Use `has_binary(name)`, or a literal executable with interpolated arguments |
| `err.message` is not a field | Use `err.msg` |
| A lifecycle write outside the repository is dropped without a message, with every later action in that stack item | Keep test documents inside the repository, such as under `target/` |
| `validate_schema()` in a lifecycle event cannot resolve a relative `$schema` | Run `md schema validate` through `set` with `$( … )::result` |
| Runs work in parallel and may read a document another is writing | Guard file reads with `env.AGENT == agent &&` |
| A usage limit is not cured by retrying | Test `err.category == 'cap'` before `retry` |
| A sequence plans one agent before any step exists | Name the agent on the command line; the prompt skips providers assigned to others |
| A contract changed without a revision change leaves documents invalid and looking current | Change `schema_revision` with every contract change |
| `cargo run -p claudine-cli` fails: the package has four binaries | Add `--bin claudine` |
| A build replaces the binary a running fleet is using | Do not build while a fleet runs |

## Design and history

The decisions, the order in which topics are narrowed, and the open
questions are in the feature `2026-09-29-update-research`.
