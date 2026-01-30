-- Research Inventory Schema
-- Migrates from research-inventory.json to SQLite
-- Schema version: 2 (matching JSON schema_version)

-- Topics table: stores all research topics
-- Uses adjacency list pattern for recursive parent-child relationships
CREATE TABLE IF NOT EXISTS topics (
    -- Primary key: topic name (unique identifier)
    name TEXT PRIMARY KEY NOT NULL,

    -- Kind discriminator: stores the KindCategory variant name
    -- Values: 'Library', 'Software', 'Person', 'SolutionArea', 'ProgrammingLanguage'
    kind TEXT NOT NULL CHECK (kind IN ('Library', 'Software', 'Person', 'SolutionArea', 'ProgrammingLanguage')),

    -- Adjacency list: nullable FK for recursive parent-child relationship
    -- NULL means this is a root topic
    parent_topic_name TEXT REFERENCES topics(name) ON DELETE CASCADE,

    -- Timestamps (ISO8601 TEXT format for chrono compatibility)
    created TEXT NOT NULL,
    last_updated TEXT NOT NULL,

    -- Text content fields
    brief TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    when_to_use TEXT NOT NULL DEFAULT ''
);

-- Index for parent lookups (critical for recursive CTE performance)
CREATE INDEX IF NOT EXISTS idx_topics_parent ON topics(parent_topic_name);

-- Index for kind-based queries
CREATE INDEX IF NOT EXISTS idx_topics_kind ON topics(kind);

-- Library details table: 1:0-1 relationship with topics where kind='Library'
CREATE TABLE IF NOT EXISTS library_details (
    -- FK to topics table
    topic_name TEXT PRIMARY KEY REFERENCES topics(name) ON DELETE CASCADE,

    -- Package manager: stores LanguagePackageManager as TEXT
    -- Values: 'Cargo', 'Npm', 'PyPI', 'Packagist', 'LuaRocks', 'Go', etc.
    package_manager TEXT NOT NULL,

    -- Package name on the registry
    package_name TEXT NOT NULL,

    -- Optional feature flags (stored as JSON array)
    features TEXT, -- JSON array: ["feature1", "feature2"]

    -- Programming language
    language TEXT NOT NULL,

    -- URLs
    url TEXT NOT NULL,
    repo TEXT,
    docs TEXT,

    -- Licenses (stored as JSON array of License enum values)
    -- Each entry is either a simple string like "Mit" or {"Other": "custom"}
    licenses TEXT NOT NULL DEFAULT '[]' -- JSON array
);

-- Software details table: 1:0-1 relationship with topics where kind='Software'
CREATE TABLE IF NOT EXISTS software_details (
    -- FK to topics table
    topic_name TEXT PRIMARY KEY REFERENCES topics(name) ON DELETE CASCADE,

    -- Software name
    name TEXT NOT NULL,

    -- Optional company name
    company TEXT
);

-- Documents table: stores all documents associated with topics
-- Composite primary key: (topic_name, filename)
CREATE TABLE IF NOT EXISTS documents (
    -- FK to topics table
    topic_name TEXT NOT NULL REFERENCES topics(name) ON DELETE CASCADE,

    -- Filename relative to research root
    filename TEXT NOT NULL,

    -- Content type: describes how the document was created
    -- Values: 'Static', 'Template', 'Prompt', 'KindDerived', 'Skill', 'DeepDive'
    content_type TEXT NOT NULL CHECK (content_type IN ('Static', 'Template', 'Prompt', 'KindDerived', 'Skill', 'DeepDive')),

    -- Optional prompt used to generate the document
    prompt TEXT,

    -- AI workflow used to generate the document
    -- Values: 'Research', 'Synthesis', 'Manual' or NULL
    flow TEXT CHECK (flow IS NULL OR flow IN ('Research', 'Synthesis', 'Manual')),

    -- Timestamps (ISO8601 TEXT format)
    created TEXT NOT NULL,
    last_updated TEXT NOT NULL,

    -- Optional model information
    model TEXT,
    model_capability TEXT, -- Stored as TEXT for external enum

    -- Content hashes stored as INTEGER (i64 bit reinterpretation of u64)
    -- Use bit cast: u64 as i64 for storage, i64 as u64 for retrieval
    content_hash INTEGER NOT NULL DEFAULT 0,
    interpolated_hash INTEGER NOT NULL DEFAULT 0,

    -- Composite primary key
    PRIMARY KEY (topic_name, filename)
);

-- Index for document lookups by topic
CREATE INDEX IF NOT EXISTS idx_documents_topic ON documents(topic_name);

-- Index for content type queries
CREATE INDEX IF NOT EXISTS idx_documents_content_type ON documents(content_type);

-- Schema metadata table for tracking version
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

-- Insert schema version
INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('schema_version', '2');
