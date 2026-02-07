# Named Serial Groups

Organize tests by the shared resources they access for optimal parallelism.

## Basic Concept

Tests without a key block ALL other `#[serial]` tests. Named groups allow fine-grained control.

```rust
// These two tests serialize with each other
#[test]
#[serial(database)]
fn test_db_insert() { }

#[test]
#[serial(database)]
fn test_db_delete() { }

// This test can run in parallel with the database tests
#[test]
#[serial(cache)]
fn test_cache_invalidate() { }
```

## Multiple Keys

A test can belong to multiple groups. It will serialize with tests in ANY of its groups.

```rust
#[test]
#[serial(database)]
fn test_only_database() { }

#[test]
#[serial(cache)]
fn test_only_cache() { }

#[test]
#[serial(database, cache)]
fn test_both() { }  // Blocks both database AND cache tests
```

## Mixing Serial and Parallel

Use `#[parallel]` for tests that can run concurrently with other parallel tests, but must respect serial tests.

```rust
// Serial tests with database key
#[test]
#[serial(database)]
fn test_db_write() { }

// Parallel tests can run together, but not during serial(database)
#[test]
#[parallel(database)]
fn test_db_read_1() { }

#[test]
#[parallel(database)]
fn test_db_read_2() { }  // Can run with test_db_read_1
```

## Recommended Group Names

| Group Name | Use For |
|------------|---------|
| `env` | Environment variable manipulation |
| `database` or `db` | Database connections/writes |
| `cache` | Cache operations |
| `fs` or `filesystem` | Shared file system paths |
| `network` | Network ports or sockets |
| `singleton` | Global singleton state |
| `config` | Configuration file modifications |

## Module-Level Groups

Apply to all tests in a module.

```rust
#[cfg(test)]
mod database_tests {
    use serial_test::serial;

    // All tests in this module use the database key
    #[test]
    #[serial(database)]
    fn test_create_user() { }

    #[test]
    #[serial(database)]
    fn test_delete_user() { }
}

#[cfg(test)]
mod cache_tests {
    use serial_test::serial;

    #[test]
    #[serial(cache)]
    fn test_cache_hit() { }
}
```

## Group Hierarchy Pattern

Create hierarchical groups when some tests need broader exclusivity.

```rust
// Fine-grained: only blocks user table operations
#[test]
#[serial(db_users)]
fn test_user_insert() { }

// Fine-grained: only blocks order table operations
#[test]
#[serial(db_orders)]
fn test_order_insert() { }

// Coarse-grained: blocks ALL database operations
#[test]
#[serial(db_users, db_orders)]
fn test_full_transaction() { }
```

## Best Practices

1. **Use descriptive names** - `database` not `db1`
2. **Document shared resources** - Comment what each group protects
3. **Keep groups minimal** - Don't block more tests than necessary
4. **Prefer fine-grained groups** - Split by resource type, not test type
5. **Avoid orphan groups** - Groups with only one test provide no benefit
