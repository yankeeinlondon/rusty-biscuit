# Frozen Recursive Schema Measurement Fixtures

These fixtures compare an unrolled schema grammar with the same grammar written
as one recursive type. They hold the size baseline fixed: a later edit to
Darkmatter's effect catalog or expression functions cannot change what is
measured. The test is `darkmatter/lib/tests/l1/recursive_schema_frozen_fixture.rs`.

| Path | Holds |
| --- | --- |
| `action.yaml` | The 143-verb `action` and `bundle` tables |
| `lifecycle.yaml` | The lifecycle stack grammar unrolled to depth 5 (`item-0` … `item-5`, `stack-0` … `stack-5`), plus `loop` |
| `lifecycle-recursive.yaml` | Hand-written: the same grammar over the same `action.yaml`, with one recursive `item` type |

`action.yaml` and `lifecycle.yaml` are byte-for-byte copies of the committed
`spike/schemas/` files of `2026-10-20-lifecycle-grammar`, produced there by
`spike/gen-action-schema.py`. They are frozen: never regenerate them, and never
bring them in step with the current catalogs. The test fails if a copy stops
matching its committed original.

## `lifecycle-recursive.yaml`

The file keeps the unrolled grammar's approximations, including the one merged
mapping item (verbs, `when`, `then`, `else`, `no_error`), because the
comparison measures recursion, not a better grammar. Only the unroll changes:

```yaml
# lifecycle.yaml: one type per level; level 0 cannot nest
item-1:
  then: [item-0[]@this, bundle@./action.yaml]
stack: stack-5@this

# lifecycle-recursive.yaml: one type at every level
item:
  then: [item[]@this, bundle@./action.yaml]
stack: [item[]@this, bundle@./action.yaml]
```

Up to five nested `then`/`else` levels the two files accept and reject the
same documents. Deeper nesting is valid only in the recursive file.
