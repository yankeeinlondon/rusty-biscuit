# Spike S1: Embedded Schema Reference

Date: 2026-09-29. Run on macOS against `6eed776cb` (the `feat/osc-9-4-protocol`
HEAD) in a throwaway worktree. The code changes were left uncommitted there and
are discarded. Nothing here was added to the branch.

## Question

Darkmatter embeds its base schema with `include_str!` in
`darkmatter_base_schema_ref`
(`darkmatter/lib/src/markdown/schemas/mod.rs:142-151`, reading
`darkmatter/docs/schemas/darkmatter.yaml`). Can that embedded baseline resolve a
relative named-type import such as
`policy[]@../../../content-policy/schemas/content-policy.yaml` in
`md schema validate` and in DMLS? If it cannot, what is the smallest change
that makes it resolve, including for an installed binary on a machine with no
repository checkout?

## Answer (short)

- **No.** The embedded baseline never runs import expansion. With the line
  added, the first use of the baseline fails: `md compose` reports "baseline
  could not be converted to JSON Schema", and DMLS and every other caller of
  the cached JSON baseline **panic** at `mod.rs:157`
  (`expect("baseline schema must convert")`).
- **`md schema validate` never uses the embedded baseline.** This holds with
  or without the reference. When a document has no `$schema` and neither
  `--schema` nor `BASELINE_SCHEMA` is set, the command reports it "valid by
  default". This is the documented contract
  (`darkmatter/docs/topics/schemas/definition.md`, "Compose and Validate
  Defaults"). The second half of AC 21 cannot pass through the embedded
  baseline in `md schema validate` unless the author rules on it (see
  "Implications for Task 6.1").
- **A baseline loaded from disk already resolves the reference.**
  `md schema validate --schema <absolute path>/darkmatter/docs/schemas/darkmatter.yaml`
  applies the `content_policy` typing today, with no code change.
- **Smallest fix for the embedded baseline:** embed the referenced file too,
  and give the import engine a small table of embedded files. Each file is
  keyed by its virtual path relative to the repository root. The baseline is
  expanded against that table once, where it is loaded. A prototype of about
  110 lines in two files (`resolve.rs`, `mod.rs`) made `md compose` and DMLS
  (validation and completion) apply the typing. It worked from a copied binary
  with the repository's `content-policy/` directory moved away. A disk-resolved
  load of `darkmatter.yaml` produced exactly the same JSON Schema as the
  embedded one.

## Method

1. Detached the worktree at `6eed776cb`. It had been created from an older
   `main` commit, whose Darkmatter differs from the branch.
2. Created a **non-union** placeholder
   `content-policy/schemas/content-policy.yaml`:

   ```yaml
   kind: schema
   types:
     action_kind: "enum(refresh, archive, remove) -> Spike S1 placeholder action"
     policy:
       rule: "string(required) -> Spike S1 placeholder rule"
       action: "action_kind@./content-policy.yaml"
   ```

   The `action_kind@./content-policy.yaml` self-reference is there on purpose.
   The spec's excerpt uses the same same-file form
   (`short_form@./content-policy.yaml`), so any fix must resolve nested imports
   inside the embedded file as well. The first runs used a single `policy` type
   whose `action` was an inline `enum(...)`. The results were the same.
3. Added one line beside `last_updated` in `darkmatter/docs/schemas/darkmatter.yaml`:

   ```yaml
   content_policy: "policy[]@../../../content-policy/schemas/content-policy.yaml"
   ```

   `darkmatter/docs/schemas/` is three levels below the repository root, so the
   path is correct. `darkmatter/schemas/partials/doc.yaml` is not loaded by any
   runtime code at this commit. The only `include_str!` sites are `mod.rs:144`
   and three tests in `simplified/mod.rs` (lines 800, 851, 909).
4. Built `md` (`cargo build -p darkmatter-cli --bin md`). Ran it against
   Markdown documents with no `$schema`. Exercised DMLS through the existing
   unit test `overlay::schema::tests::test_base_baseline_applies_without_schema`
   (which calls `assemble` with `DmlsConfig::default()`, the path a document
   without `$schema` takes) and through one added test harness in the same
   module.
5. Prototyped the fix, then re-ran the same commands and the full L1 suites of
   `darkmatter`, `dmls`, and `darkmatter-cli`.

Test documents (all with no `$schema`):

- `doc-bad.md`: `last_updated: not-a-date`, plus a policy entry.
- `doc-good.md`: `content_policy: [{rule: TimeSensitive}]`.
- `doc-policy-bad.md`: `[{rule: 42, action: explode}, {action: refresh}]`.

## Observed Results

### Before any change: `md schema validate` ignores the built-in baseline

```text
$ md schema validate --format json spike-s1/doc-bad.md
{"file":"spike-s1/doc-bad.md","problems":[],"schema":null,"valid":true}   # exit 0

$ md compose spike-s1/doc-bad.md
MarkdownError: schema validation failed
  invalid last_updated: "not-a-date" is not a "date" at 3:1                # exit 1
```

`load_api` (`darkmatter/cli/src/commands/schema/validate.rs:113-123`) builds
`DarkmatterSchemas::new()` and adds a baseline only from `--schema` or
`BASELINE_SCHEMA`. `md compose` injects the embedded baseline
(`cli/src/commands/compose.rs:760` →
`ComposeOptions::with_darkmatter_baseline_schema`, `options.rs:1611`).

### With the reference line only (no fix)

`md schema validate` without `--schema` is unchanged, because it never reads
the baseline. The other commands:

```text
$ md compose spike-s1/doc-good.md
MarkdownError: schema validation failed
  schema could not be prepared: baseline schema is invalid: baseline could
  not be converted to JSON Schema
```

Library and DMLS unit tests:

```text
$ cargo nextest run -p dmls --lib -E 'test(test_base_baseline_applies_without_schema)'
FAIL dmls overlay::schema::tests::test_base_baseline_applies_without_schema
panicked at darkmatter/lib/src/markdown/schemas/mod.rs:157:58:
baseline schema must convert: Convert { property: "content_policy", message:
"imported type `policy@../../../content-policy/schemas/content-policy.yaml`
must be resolved before conversion (handled by the resolution layer)" }

$ cargo nextest run -p darkmatter --lib -E 'test(darkmatter_base_json_schema)'
6 failed (same panic at mod.rs:157)
```

The cause is that `darkmatter_base_schema_ref` calls only
`parse_yaml_schema`, which leaves `TypeExpr::Imported` atoms in place, and the
converter rejects them (`simplified/convert.rs:387-397`). Import expansion
(`expand_document_imports`, `resolve.rs:966`) runs only on the `$schema`
resolution paths, which need a `base_dir` on disk. An embedded string has no
such directory.

Every consumer of the embedded baseline is affected, because they all go
through `darkmatter_base_schema_ref`:

| Consumer | Site | Failure mode |
| --- | --- | --- |
| `md compose` (library `ComposeOptions`) | `options.rs:1611` → `DarkmatterSchemas::with_baseline` (`mod.rs:327`) | `SchemaError::Baseline` |
| Cached JSON baseline (compose fast path, DMLS validation) | `mod.rs:157`; DMLS `overlay/schema.rs:908`, `:985` | panic |
| DMLS completion and hover shape | `overlay/schema.rs:743`, `providers/frontmatter.rs:1516` | an unexpanded `Imported` atom, so no completion for `content_policy` |
| Frontmatter shape helpers | `frontmatter_shape.rs:47` | same as DMLS completion |

### A baseline loaded from disk already resolves the reference

```text
$ md schema validate --format json \
    --schema /…/darkmatter/docs/schemas/darkmatter.yaml spike-s1/doc-policy-bad.md spike-s1/doc-good.md
{"file":"spike-s1/doc-policy-bad.md","problems":[
  {"kind":"invalid","message":"\"explode\" is not one of \"refresh\", \"archive\" or \"remove\"","path":"/content_policy/0/action",…},
  {"kind":"missing","message":"\"rule\" is a required property","path":"/content_policy/1","property":"rule",…}],"valid":false}
{"file":"spike-s1/doc-good.md","problems":[],"valid":true}                  # exit 1
```

`with_baseline_from_file` (`mod.rs:343`) goes through `resolve_yaml_schema`
with the file's parent directory as `base_dir`, so the relative import resolves
exactly as in any schema file. (`rule: 42` is not flagged. Darkmatter's scalar
coercion accepts it as a string. This does not affect the question.)

**A separate existing defect:** `--schema` with a *relative* path that
contains a directory fails:

```text
$ md schema validate --schema spike-s1/sub/base.yaml spike-s1/doc-good.md
SchemaError: baseline schema invalid
  Cause: could not resolve $schema reference `spike-s1/sub/base.yaml`        # exit 2
$ md schema validate --schema /…/spike-s1/sub/base.yaml spike-s1/doc-good.md
… "\"Spike doc\" is not of type \"number\"" …                               # works
```

`with_baseline_from_file` passes the whole path as the reference and also uses
its parent as `base_dir` (`mod.rs:345-348`). The directory is therefore applied
twice (`spike-s1/sub/spike-s1/sub/base.yaml`). This affects any acceptance
command that spells `--schema darkmatter/docs/schemas/darkmatter.yaml`.

### With the prototype fix

The prototype diff, excluding tests, touches two files:

- `resolve.rs` (+~95 lines):
  - A `pub(crate) struct EmbeddedSchemaFile { path: &'static str, text: &'static str }`.
  - An `embedded: &'static [EmbeddedSchemaFile]` field on `ImportEngine`
    (`resolve.rs:938`). `expand_document_imports` sets it to `&[]`, so on-disk
    behavior is unchanged.
  - A lexical `lexical_join(base, reference)` with no filesystem access, where
    `..` above the virtual root means no match.
  - A branch in `resolve_namespace` (`resolve.rs:1175`): when the table is
    non-empty, join the reference onto the namespace's virtual `base_dir`, look
    the result up in the table, and parse the text. Otherwise return
    `SchemaError::Unresolved`. The filesystem is never consulted.
  - `load_named_types` (`resolve.rs:1296`) split into a read step and a
    `named_types_from_text(text, path)` parse step, which the new branch shares.
  - `pub(crate) fn expand_embedded_imports(schema, virtual_base_dir, table)`,
    which builds the engine with the table and no schema roots or request
    context.
- `mod.rs` (+~9 lines in `darkmatter_base_schema_ref`):

  ```rust
  static EMBEDDED: &[resolve::EmbeddedSchemaFile] = &[resolve::EmbeddedSchemaFile {
      path: "content-policy/schemas/content-policy.yaml",
      text: include_str!("../../../../../content-policy/schemas/content-policy.yaml"),
  }];
  resolve::expand_embedded_imports(schema, Path::new("darkmatter/docs/schemas"), EMBEDDED)
      .expect("baseline imports must resolve from embedded schemas")
  ```

The YAML line is the same relative reference an on-disk load resolves. The
table key is that reference resolved from the base schema's own virtual
location. Because the expansion happens once in `darkmatter_base_schema_ref`,
every consumer in the table above receives an import-free schema.

Results:

```text
$ md compose spike-s1/doc-good.md            # exit 0, renders
$ md compose spike-s1/doc-policy-bad.md
  invalid content_policy: "explode" is not one of "refresh", "archive" or "remove" at 3:1
  missing rule: required but not provided

# Installed-binary simulation: binary copied to /tmp, the repo's content-policy/ moved away
$ /tmp/s1-install/md compose /tmp/s1-install/doc-policy-bad.md
  invalid content_policy: "explode" is not one of …
  missing rule: required but not provided

$ cargo nextest run -p dmls --lib -E 'test(spike_s1) | test(test_base_baseline_applies_without_schema)'
PASS … test_base_baseline_applies_without_schema
PASS … spike_s1_content_policy_typing_applies_without_schema
  problems: /content_policy/0/action (enum), /content_policy/1 missing `rule`
  completion shape: content_policy = InlineObject{rule: String(required), action: Enum[refresh, archive, remove]}, is_array: true

$ cargo nextest run -p darkmatter --lib -E 'test(spike_s1)'
PASS … spike_s1_disk_resolution_matches_embedded
  imports: [".../content-policy/schemas/content-policy.yaml"]
  disk content_policy == embedded content_policy (assert_eq on the whole `properties` map)
```

The last test resolves `darkmatter.yaml` from disk through
`resolve_yaml_schema`, where the real relative path resolves. It asserts that
the resulting `properties` equal the embedded baseline's. This is the guard
that shows the relative reference and the embedded table agree.

Full L1 run with the fix
(`cargo nextest run -p darkmatter -p dmls -p darkmatter-cli --lib --test l1`):
8,620 run, 8,616 passed, 4 failed.

- Three tests convert the raw `darkmatter.yaml` directly with
  `parse_yaml_schema` + `to_json_schema`, bypassing the loader. They fail on
  the unexpanded import and must switch to the expanded schema:
  `simplified::tests::darkmatter_baseline_schema_converts_to_json_schema`
  (`simplified/mod.rs:851`),
  `simplified::tests::darkmatter_baseline_schema_ctx_reflects_runtime_semantics`
  (`:909`), and
  `l1 base_schema_end_to_end::schema_document_transcludes_same_file_as_library_source`
  (`lib/tests/l1/base_schema_end_to_end.rs:318-324`). The third test's intent
  ("the file on disk is the library's source") is best kept by comparing a
  disk `resolve_yaml_schema` load with `darkmatter_base_json_schema()`, which
  is the spike test above.
- `file_match::tests::conversion_emits_every_root_union_glob` also failed. It
  builds its own inline schema and never reads `darkmatter.yaml`, so it is
  unrelated to this change. It was not investigated further.

## Alternatives Considered

| Option | In-tree | Installed binary, no repo | Verdict |
| --- | --- | --- | --- |
| Resolve against a compile-time base (`concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/schemas")`) | works | **breaks**: the path is on the build host. This repo builds and installs from `wt` worktrees that are later deleted, and the failure lands in an `expect`, so it panics | rejected |
| Registry of embedded schemas keyed by name (for example `policy@content-policy`) | works | works | rejected: the base schema would no longer use the relative reference the spec decided on, and a disk load of `darkmatter.yaml` would disagree with the embedded one |
| **Embed the referenced file and key it by virtual repo-relative path** (the prototype) | works | works | **recommended** |
| Inline the content-policy types into `darkmatter.yaml` | works | works | rejected: contradicts the spec's editor-schema decision and duplicates the types |
| `build.rs` that pre-expands the baseline | works | works | rejected: more machinery than the table for one file |

For the recommended option, packaging and CI are not blockers:

- Darkmatter is not published (`release-plz.toml`: `publish = false`,
  `git_only = true`). It already embeds `darkmatter/docs/schemas/darkmatter.yaml`,
  which is outside the `darkmatter/lib` package directory, so a
  cross-directory `include_str!` has precedent.
- `scripts/ci/affected_scope.py:447` treats an `include_str!` target outside
  tests as that package's source. An edit to
  `content-policy/schemas/content-policy.yaml` then schedules `darkmatter` and
  runs the baseline unit tests (`darkmatter_base_json_schema_*`). This matters
  because a broken `content-policy.yaml` would otherwise surface as a runtime
  panic in `md compose` and DMLS.
- No Rust dependency is added. `cargo tree` is unchanged.

## Smallest Change (for Task 6.1)

1. `darkmatter/lib/src/markdown/schemas/resolve.rs`: add the embedded-file
   table to `ImportEngine` (`:938`), a lexical join, a branch at the top of
   `resolve_namespace` (`:1175`), split `load_named_types` (`:1296`) into read
   and parse, and add `expand_embedded_imports`. About 70–95 lines. The
   prototype repeated the namespace-cache insert, and folding that into a
   helper brings it to the lower end of the range.
2. `darkmatter/lib/src/markdown/schemas/mod.rs:142-151`: declare the one-entry
   table and call `expand_embedded_imports` in `darkmatter_base_schema_ref`.
   About 10 lines. Update the rustdoc of `darkmatter_base_schema`
   (`mod.rs:163-181`), which says the schema is "loaded from
   `darkmatter/docs/schemas/darkmatter.yaml`", to say that its imports are
   resolved from embedded files.
3. Tests, about 60–100 lines:
   - Fix the three raw-conversion tests listed above.
   - Add a lib test showing that disk resolution of `darkmatter.yaml` equals
     the embedded baseline.
   - Add a lib test for an embedded-table miss (`Unresolved`, not a
     filesystem read).
   - Add the AC 21 behavior tests from plan 6.1 (Darkmatter and DMLS).
4. Docs: a short note in `darkmatter/docs/topics/schemas/definition.md` that
   the built-in baseline's `Name@file` imports are resolved from files compiled
   into the binary. Update `docs/dependencies.md` only if the reviewer treats
   the embedded file as a dependency; it is not a crate.

Estimate: about 0.5 day including tests. It does not depend on
`2026-09-28-recursive-schema-types`, because union named types flow through
the same `expand_import` path that the dependency changes. The prototype's
`expect` keeps the existing "library bug, not an author error" contract of
`darkmatter_base_schema`.

## Implications for Task 6.1

- **Split the task.** The embedded-resolution change above can land before
  the gate, with a non-union placeholder or with the Phase 4 schema, because
  it does not wait on union `[]`. Only the `policy[]` line over the
  union-typed `policy` has to wait. Landing it early removes the "discovered
  at the end" risk that S1 was meant to size.
- **`md schema validate` needs a ruling before 6.1 can meet AC 21 as
  written.** AC 21 says "`md schema validate` … apply the `content_policy`
  typing to an ordinary Markdown document with no `$schema`". By its
  documented contract, that command applies no baseline unless one is given.
  The choices are:
  - (a) Accept `md schema validate --schema <path to darkmatter.yaml>` (or
    `BASELINE_SCHEMA`) as the `md schema validate` evidence. This works today
    through disk resolution. With an absolute path it needs no code change.
    With a relative path it first needs the `with_baseline_from_file` fix
    (`mod.rs:345-348`, pass the file name or an absolutized path, about 3
    lines plus a test).
  - (b) Change `md schema validate` to default to the embedded Darkmatter
    baseline, as `md compose` does. This changes Darkmatter behavior, and the
    "valid by default" output and its docs would change with it. It belongs
    to Darkmatter's own schema work, not this feature.
  - (c) Reword AC 21 to "`md compose` and DMLS" for the no-`$schema` half,
    and keep `md schema validate` for loading `content-policy.yaml` itself.

  Recommendation: (a) with the relative-path fix, and state it in the
  implementation log. It keeps the Darkmatter contract, and the fix is small.
- **Add the behavior tests where the baseline is used.** For a document with
  no `$schema`, the Darkmatter test should go through
  `ComposeOptions::with_darkmatter_baseline_schema` or
  `DarkmatterSchemas::with_darkmatter_baseline_json_schema`, not
  `md schema validate`. The DMLS test fits beside
  `overlay::schema::tests::test_base_baseline_applies_without_schema`.
- **Self-references inside `content-policy.yaml` need the embedded table's
  nested lookup.** The spec's `short_form@./content-policy.yaml` form was
  exercised (as `action_kind@./content-policy.yaml`) and resolves through the
  table, because each embedded namespace keeps its own virtual `base_dir`.
  `@this` also works, because it never reaches the table.
- **The placeholder path is correct.** The relative path is right for
  `darkmatter/docs/schemas/darkmatter.yaml`.
  `darkmatter/schemas/partials/doc.yaml` is not loaded by any runtime code at
  `6eed776cb`. If Darkmatter moves the baseline there, the table's virtual
  `base_dir` (`darkmatter/docs/schemas`) must change with the `include_str!`
  path.

## Prototype Diff

The prototype's uncommitted changes (including the spike-only tests) are kept
in `prototype.diff` beside this file, as a starting point for task 6.1. It
applies against `6eed776cb`.
