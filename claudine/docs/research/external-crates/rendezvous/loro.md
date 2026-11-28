---
prompt: |-
    The 'loro' crate in Rust provides a modern and highly performant CRTD library which
    we will be using in the "rendezvous" daemon.

    Your task is to do a deep dive on the 'loro' crate and answer the following questions through thorough research:

    - What is the functional footprint of the 'loro' crate?
    - What features does loro expose and what functionality do these features map to? When should you use each feature? When should you avoid?
    - What are the key URLs for this crate? Repo? Website? Docs?
    - What are 2-3 common use cases that this crate would be used for? For each use case, describe the use case and provide Rust code examples of how this use case might be implemented.
    - What do developers say about using this crate? What "gotchas" are there and how can they be worked around?
last_updated: 2026-05-24
---
## Functional Footprint

The `loro` crate provides a complete CRDT implementation with the following core capabilities:

- **Document-centric API**: All operations center around `LoroDoc`, which serves as the container for all collaborative state
- **Rich CRDT types**: Text (with rich text support), Lists, Movable Lists, Maps, Trees, and Counters
- **Automatic conflict resolution**: Concurrent edits merge automatically without user intervention
- **Version control**: Full history tracking with time travel, branching (fork), and merging
- **Incremental synchronization**: Export/import only the changes (deltas) between versions
- **Event system**: Subscribe to changes at the document or container level
- **Undo/Redo**: Built-in `UndoManager` for local undo operations
- **Persistence**: Export to binary snapshots or incremental updates
- **JSON compatibility**: Documents can be serialized to/from JSON
- **Cross-platform**: Available in Rust, JavaScript (WASM), Swift, and Python

## Features

The crate exposes three feature flags:

### `counter` (enabled by default)

- **What it does**: Enables the `LoroCounter` type for distributed counter operations
- **When to use**: When you need increment/decrement operations that converge correctly across peers (e.g., vote counts, statistics)
- **When to avoid**: If you need exact arithmetic precision or negative balances; counters use CRDT semantics, not ACID transactions

### `jsonpath`

- **What it does**: Enables JSONPath queries against the document state
- **When to use**: When you need to query nested document structures using JSONPath expressions
- **When to avoid**: If you don't need querying capabilities, as it adds compilation overhead

### `logging`

- **What it does**: Enables internal logging via the `tracing` crate
- **When to use**: During development/debugging to trace internal operations
- **When to avoid**: In production builds where you want to minimize dependencies and overhead

## Key URLs

| Resource          | URL                                       |
|-------------------|-------------------------------------------|
| **Repository**    | https://github.com/loro-dev/loro          |
| **Website**       | https://loro.dev                          |
| **Documentation** | https://docs.rs/loro                      |
| **Examples**      | https://github.com/loro-dev/loro-examples |
| **Discord**       | https://discord.gg/tUsBSVfqzf             |
| **Crates.io**     | https://crates.io/crates/loro             |

## Common Use Cases

### 1. Real-time Collaborative Text Editor

Loro's rich text CRDT makes it ideal for building collaborative editors where multiple users can edit simultaneously.

```rust
use loro::{LoroDoc, ExportMode};
use std::sync::Arc;

fn main() {
    // Create two documents simulating two users
    let doc_a = LoroDoc::new();
    let doc_b = LoroDoc::new();
    
    // User A inserts text
    let text_a = doc_a.get_text("content");
    text_a.insert(0, "Hello ").unwrap();
    text_a.insert(6, "world!").unwrap();
    
    // Apply formatting
    text_a.mark(0..5, "bold", true).unwrap();
    
    // Export changes from A
    let updates = doc_a.export(ExportMode::all_updates()).unwrap();
    
    // User B receives and applies changes
    doc_b.import(&updates).unwrap();
    
    // Both documents now have the same state
    assert_eq!(doc_a.get_deep_value(), doc_b.get_deep_value());
    
    // User B makes concurrent edits
    let text_b = doc_b.get_text("content");
    text_b.insert(6, "beautiful ").unwrap();
    
    // Export B's changes and send back to A
    let b_updates = doc_b.export(ExportMode::all_updates()).unwrap();
    doc_a.import(&b_updates).unwrap();
    
    // Changes are merged automatically
    println!("Merged text: {}", text_a.to_string());
    // Output: "Hello beautiful world!"
}
```

### 2. Collaborative Task Management with Lists and Maps

Using Loro's Map and List containers to build a shared task board.

```rust
use loro::{LoroDoc, LoroValue};
use std::collections::HashMap;

fn main() {
    let doc = LoroDoc::new();
    
    // Create a tasks list
    let tasks = doc.get_list("tasks");
    
    // Add a task as a map
    let task1 = tasks.insert_container(0, doc.get_map("")).unwrap();
    task1.insert("id", "task-1").unwrap();
    task1.insert("title", "Design API").unwrap();
    task1.insert("status", "in-progress").unwrap();
    
    // Add another task
    let task2 = tasks.insert_container(1, doc.get_map("")).unwrap();
    task2.insert("id", "task-2").unwrap();
    task2.insert("title", "Write tests").unwrap();
    task2.insert("status", "todo").unwrap();
    
    // Query the document state
    let state = doc.get_deep_value();
    println!("Tasks: {:?}", state);
    
    // Export snapshot for persistence
    let snapshot = doc.export(ExportMode::Snapshot).unwrap();
    
    // Later, restore from snapshot
    let restored = LoroDoc::from_snapshot(&snapshot).unwrap();
    assert_eq!(doc.get_deep_value(), restored.get_deep_value());
}
```

### 3. Version Control and Time Travel

Loro's built-in versioning enables Git-like operations for document state.

```rust
use loro::LoroDoc;

fn main() {
    let doc = LoroDoc::new();
    let text = doc.get_text("doc");
    
    // Make some edits
    text.insert(0, "Version 1").unwrap();
    doc.commit();
    let v1 = doc.state_frontiers();
    
    text.insert(9, " - added feature A").unwrap();
    doc.commit();
    let v2 = doc.state_frontiers();
    
    text.insert(27, " - added feature B").unwrap();
    doc.commit();
    
    println!("Current: {}", text.to_string());
    // "Version 1 - added feature A - added feature B"
    
    // Time travel back to v1 (read-only)
    doc.checkout(&v1).unwrap();
    println!("At v1: {}", text.to_string());
    // "Version 1"
    
    // Return to latest
    doc.checkout_to_latest();
    
    // Create a branch from v1
    let branch = doc.fork_at(&v1).unwrap();
    let branch_text = branch.get_text("doc");
    branch_text.insert(9, " - experimental").unwrap();
    branch.commit();
    
    // Merge branch back
    let branch_updates = branch.export(ExportMode::all_updates()).unwrap();
    doc.import(&branch_updates).unwrap();
    
    println!("After merge: {}", text.to_string());
}
```

## Developer Feedback and Gotchas

### What Developers Say

- **Performance**: Loro is significantly faster than alternatives like Automerge, with decode times under 1ms for typical documents
- **Ease of use**: The API is intuitive and follows Rust conventions well
- **Rich text support**: One of the few CRDT libraries with robust rich text formatting support
- **Version control**: The built-in time travel and branching capabilities are unique and powerful

### Common Gotchas and Workarounds

1. **PeerID uniqueness is critical**

    - **Gotcha**: Reusing the same PeerID across concurrent writers (tabs, devices) causes document corruption
    - **Workaround**: Use the default random PeerID per session, or implement strict locking if reusing IDs

2. **Transactions are not ACID**

    - **Gotcha**: Loro transactions group operations for events/history but don't provide isolation or rollback
    - **Workaround**: For operations requiring atomicity, validate before applying changes

3. **Detached mode editing**

    - **Gotcha**: After `checkout`, the document enters detached mode where edits aren't applied to the current state
    - **Workaround**: Call `attach()` or `checkout_to_latest()` to reattach, or enable detached editing with `set_detached_editing(true)`

4. **Container initialization conflicts**

    - **Gotcha**: Concurrently creating child containers in a Map with the same key results in overwrites, not merges
    - **Workaround**: Initialize all child containers upfront, or use root-level containers instead of nested ones

5. **WASM bundle size**

    - **Gotcha**: The JavaScript/WASM build is ~970KB gzipped
    - **Workaround**: Use the Rust native build for size-sensitive applications

6. **Not suitable for all use cases**

    - **Gotcha**: CRDTs are poor fits for financial transactions, exclusive resource booking, or strong consistency requirements
    - **Workaround**: Use traditional databases with ACID properties for these scenarios, or hybrid approaches

7. **Import dependencies**

    - **Gotcha**: Importing updates may fail if prerequisite operations are missing
    - **Workaround**: Always check `ImportStatus.pending` and fetch missing ranges before retrying

8. **Timestamp ordering**

    - **Gotcha**: Timestamps are forced to be monotonically increasing; earlier timestamps get clamped
    - **Workaround**: Don't rely on custom timestamps for ordering; use the CRDT's built-in causality tracking
