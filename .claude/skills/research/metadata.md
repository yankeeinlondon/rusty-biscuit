# Metadata System

The research package uses a versioned metadata schema for tracking research topics.

## Per-Topic Metadata (v1)

Each research topic has a `metadata.json` file:

```json
{
  "schema_version": 1,
  "kind": "Library",
  "details": {
    "type": "Library",
    "package_manager": "crates.io",
    "language": "Rust",
    "url": "https://crates.io/crates/clap",
    "repository": "https://github.com/clap-rs/clap"
  },
  "additional_files": {
    "question_1.md": "How does it compare to structopt?"
  },
  "created_at": "2025-12-28T10:00:00Z",
  "updated_at": "2025-12-28T10:00:00Z",
  "brief": "A full-featured command-line argument parser.",
  "summary": "clap is a fast, ergonomic CLI parser...",
  "when_to_use": "Use when building CLI applications..."
}
```

## Schema Fields

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | `u32` | Always `1` for current format |
| `kind` | `enum` | Research type (see below) |
| `details` | `ResearchDetails` | Type-specific metadata |
| `additional_files` | `HashMap<String, String>` | Filename → prompt text |
| `created_at` | `DateTime<Utc>` | ISO 8601 creation timestamp |
| `updated_at` | `DateTime<Utc>` | ISO 8601 last update timestamp |
| `brief` | `Option<String>` | Single-sentence summary |
| `summary` | `Option<String>` | Paragraph-length summary |
| `when_to_use` | `Option<String>` | Usage guidance from SKILL.md |

## Research Types (Kind)

The `kind` field uses the `ResearchDetails` enum with these variants:

| Type | Description | Type-Specific Fields |
|------|-------------|---------------------|
| `Library` | Package/library research | `package_manager`, `language`, `url`, `repository` |
| `Api` | Public API research | (empty) |
| `Cli` | Command-line tool | (empty) |
| `App` | Application research | (empty) |
| `CloudProvider` | Cloud service research | (empty) |
| `Standard` | Technical standard/spec | (empty) |
| `SolutionSpace` | Problem space comparison | (empty) |
| `Person` | Individual person | (empty) |
| `People` | Group/team research | (empty) |
| `Place` | Location research | (empty) |
| `Product` | Commercial product | (empty) |
| `Company` | Company research | (empty) |
| `CompanyCategory` | Industry sector | (empty) |
| `News` | Current events | (empty) |
| `SkillSet` | Collection of skills | (empty) |

## LibraryDetails Fields

For `kind: Library`:

```rust
pub struct LibraryDetails {
    pub package_manager: Option<String>,  // e.g., "crates.io", "npm"
    pub language: Option<String>,         // e.g., "Rust", "TypeScript"
    pub url: Option<String>,              // Package manager URL
    pub repository: Option<String>,       // Source repository URL
}
```

## Schema Migration (v0 → v1)

Legacy v0 metadata used `library_info` at top level:

```json
{
  "library_info": {
    "name": "clap",
    "package_manager": "crates.io"
  }
}
```

Migration to v1:
1. Automatic on `ResearchMetadata::load()`
2. Original file backed up as `metadata.v0.json.backup`
3. Batch migration via `research list --migrate`

## Centralized Inventory (v2)

Recent addition: centralized `research-inventory.json` at `$RESEARCH_DIR/.research/`:

```json
{
  "schema_version": 2,
  "topics": {
    "clap": { /* Topic struct */ },
    "tokio": { /* Topic struct */ }
  }
}
```

### Inventory Features

- **O(1) lookup** by topic name
- **Lazy migration** from per-topic metadata.json files
- **Atomic writes** via temp file + rename
- **Content hashing** via xxHash for change detection

### Topic Struct

```rust
pub struct Topic {
    name: String,
    kind: KindCategory,
    documents: Vec<Document>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub enum KindCategory {
    Software(Software),
    Library(Library),
    // ... other variants
}
```

### ResearchInventory API

```rust
use research_lib::metadata::ResearchInventory;

// Load or create
let mut inventory = ResearchInventory::load()?;

// CRUD operations
inventory.insert("clap".to_string(), topic);
let topic = inventory.get("clap");
inventory.remove("clap");

// Save atomically
inventory.save()?;
```

## SQLite Database Layer

For advanced querying, there's also `ResearchInventoryDb`:

```rust
use research_lib::metadata::{init_pool, ResearchInventoryDb};

let pool = init_pool("path/to/research.db").await?;
let db = ResearchInventoryDb::new(pool);

// Query operations
db.get_topic("clap").await?;
db.list_topics().await?;
```

Features:
- WAL mode for concurrent access
- 5-second busy timeout
- Foreign key constraints enabled
- Connection pooling via sqlx

## Content Policy

The `ContentPolicy` module controls document expiry:

```rust
pub struct ContentPolicy {
    pub expiry: ContentExpiry,
}

pub enum ContentExpiry {
    Never,
    Days(u32),
    Date(DateTime<Utc>),
}
```

Used for deciding when to refresh research content.
