//! Typed descriptor catalog for runtime context variables.
//!
//! Each [`ContextVariableDescriptor`] describes a single variable that may be
//! captured into `ComposeContext` at runtime. The catalog is a static,
//! compile-time constant — constructing or reading it performs no host probes,
//! no I/O, and no runtime context capture.
use crate::catalog::{Described, Example};


/// Display type for a context variable's runtime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextValueType {
    /// ISO-8601 date string (`YYYY-MM-DD`).
    Date,
    /// ISO-8601 datetime string (`YYYY-MM-DDTHH:MM:SS`).
    DateTime,
    /// Time string (`HH:MM` or `HH:MM AM/PM`).
    Time,
    /// Timezone abbreviation or IANA name.
    Timezone,
    /// Integer number.
    Integer,
    /// Floating-point number.
    Number,
    /// Boolean (`true`/`false`).
    Boolean,
    /// Plain string (possibly empty).
    String,
    /// Comma-separated values.
    Csv,
    /// Markdown bullet list.
    MarkdownList,
    /// Nested Markdown list (dependencies, etc.).
    NestedMarkdownList,
    /// Object/map shape.
    Object,
    /// Value of the inner type that may be `null` when unavailable.
    Nullable(&'static ContextValueType),
}

/// Descriptor for a single runtime context variable.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextVariableDescriptor {

    /// Canonical variable name (used in `ctx.NAME`).
    pub name: &'static str,
    /// Human-readable type for display.
    pub display_type: ContextValueType,
    /// Short description of the variable's meaning.
    pub description: &'static str,
    /// Top-level grouping category.
    pub category: &'static str,
    /// Sub-section within the category (may be empty).
    pub subsection: &'static str,
    /// Stable display order within the subsection.
    pub order: usize,
    /// Optional verified example.
    pub example: Option<Example>,
}
impl Described for ContextVariableDescriptor {
    fn key(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn category(&self) -> &'static str {
        self.category
    }
    fn order(&self) -> usize {
        self.order
    }
    fn example(&self) -> Option<&Example> {
        self.example.as_ref()
    }
}


/// All context variable descriptors, in display order.
pub const CONTEXT_VARIABLE_DESCRIPTORS: &[ContextVariableDescriptor] = &[
    // ── Date and Time ───────────────────────────────────────────────
    ContextVariableDescriptor {
        name: "now",
        display_type: ContextValueType::DateTime,
        description: "Local date and time in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.now", result: "2024-06-15T10:30:00" }),
 },
    ContextVariableDescriptor {

        name: "now_utc",
        display_type: ContextValueType::DateTime,
        description: "UTC date and time in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.now_utc", result: "2024-06-15T17:30:00Z" }),

    },
    ContextVariableDescriptor {

        name: "today",
        display_type: ContextValueType::Date,
        description: "Local date in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.today", result: "2024-06-15" }),

    },
    ContextVariableDescriptor {

        name: "today_utc",
        display_type: ContextValueType::Date,
        description: "UTC date in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 4,

        example: Some(Example { invocation: "ctx.today_utc", result: "2024-06-15" }),

    },
    ContextVariableDescriptor {

        name: "yesterday",
        display_type: ContextValueType::Date,
        description: "Local date for yesterday in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 5,

        example: Some(Example { invocation: "ctx.yesterday", result: "2024-06-14" }),

    },
    ContextVariableDescriptor {

        name: "yesterday_utc",
        display_type: ContextValueType::Date,
        description: "UTC date for yesterday in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 6,

        example: Some(Example { invocation: "ctx.yesterday_utc", result: "2024-06-14" }),

    },
    ContextVariableDescriptor {

        name: "tomorrow",
        display_type: ContextValueType::Date,
        description: "Local date for tomorrow in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 7,

        example: Some(Example { invocation: "ctx.tomorrow", result: "2024-06-16" }),

    },
    ContextVariableDescriptor {

        name: "tomorrow_utc",
        display_type: ContextValueType::Date,
        description: "UTC date for tomorrow in ISO-8601 format.",
        category: "Date and Time",
        subsection: "",
        order: 8,

        example: Some(Example { invocation: "ctx.tomorrow_utc", result: "2024-06-16" }),

    },
    ContextVariableDescriptor {

        name: "day",
        display_type: ContextValueType::String,
        description: "Full day of week name (local).",
        category: "Date and Time",
        subsection: "",
        order: 9,

        example: Some(Example { invocation: "ctx.day", result: "Saturday" }),

    },
    ContextVariableDescriptor {

        name: "day_utc",
        display_type: ContextValueType::String,
        description: "Full day of week name (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 10,

        example: Some(Example { invocation: "ctx.day_utc", result: "Saturday" }),

    },
    ContextVariableDescriptor {

        name: "day_abbr",
        display_type: ContextValueType::String,
        description: "Abbreviated day of week name (local).",
        category: "Date and Time",
        subsection: "",
        order: 11,

        example: Some(Example { invocation: "ctx.day_abbr", result: "Sat" }),

    },
    ContextVariableDescriptor {

        name: "day_abbr_utc",
        display_type: ContextValueType::String,
        description: "Abbreviated day of week name (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 12,

        example: Some(Example { invocation: "ctx.day_abbr_utc", result: "Sat" }),

    },
    ContextVariableDescriptor {
        name: "year",
        display_type: ContextValueType::String,
        description: "Current year (local).",
        category: "Date and Time",
        subsection: "",
        order: 13,


        example: Some(Example { invocation: "ctx.year", result: "2024" }),

    },
    ContextVariableDescriptor {
        name: "year_utc",
        display_type: ContextValueType::String,
        description: "Current year (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 14,


        example: Some(Example { invocation: "ctx.year_utc", result: "2024" }),

    },
    ContextVariableDescriptor {
        name: "month",
        display_type: ContextValueType::String,
        description: "Current month as zero-padded number (local).",
        category: "Date and Time",
        subsection: "",
        order: 15,


        example: Some(Example { invocation: "ctx.month", result: "06" }),

    },
    ContextVariableDescriptor {

        name: "month_name",
        display_type: ContextValueType::String,
        description: "Full month name (local).",
        category: "Date and Time",
        subsection: "",
        order: 16,

        example: Some(Example { invocation: "ctx.month_name", result: "June" }),

    },
    ContextVariableDescriptor {

        name: "month_name_abbr",
        display_type: ContextValueType::String,
        description: "Abbreviated month name (local).",
        category: "Date and Time",
        subsection: "",
        order: 17,

        example: Some(Example { invocation: "ctx.month_name_abbr", result: "Jun" }),

    },
    ContextVariableDescriptor {
        name: "day_of_month",
        display_type: ContextValueType::String,
        description: "Day of month as number (local).",
        category: "Date and Time",
        subsection: "",
        order: 18,


        example: Some(Example { invocation: "ctx.day_of_month", result: "15" }),

    },
    ContextVariableDescriptor {

        name: "day_of_month_suffixed",
        display_type: ContextValueType::String,
        description: "Day of month with English ordinal suffix (local).",
        category: "Date and Time",
        subsection: "",
        order: 19,

        example: Some(Example { invocation: "ctx.day_of_month_suffixed", result: "15th" }),

    },
    ContextVariableDescriptor {

        name: "time",
        display_type: ContextValueType::Time,
        description: "Current local time in 12-hour format with AM/PM.",
        category: "Date and Time",
        subsection: "",
        order: 20,

        example: Some(Example { invocation: "ctx.time", result: "10:30 AM" }),

    },
    ContextVariableDescriptor {

        name: "time_military",
        display_type: ContextValueType::Time,
        description: "Current local time in 24-hour format.",
        category: "Date and Time",
        subsection: "",
        order: 21,

        example: Some(Example { invocation: "ctx.time_military", result: "10:30" }),

    },
    ContextVariableDescriptor {

        name: "time_utc",
        display_type: ContextValueType::Time,
        description: "Current UTC time in 12-hour format with AM/PM.",
        category: "Date and Time",
        subsection: "",
        order: 22,

        example: Some(Example { invocation: "ctx.time_utc", result: "05:30 PM (UTC)" }),

    },
    ContextVariableDescriptor {

        name: "time_military_utc",
        display_type: ContextValueType::Time,
        description: "Current UTC time in 24-hour format.",
        category: "Date and Time",
        subsection: "",
        order: 23,

        example: Some(Example { invocation: "ctx.time_military_utc", result: "17:30 (UTC)" }),

    },
    ContextVariableDescriptor {

        name: "timezone",
        display_type: ContextValueType::Timezone,
        description: "Local timezone abbreviation (e.g., PST, EST).",
        category: "Date and Time",
        subsection: "",
        order: 24,

        example: Some(Example { invocation: "ctx.timezone", result: "PDT" }),

    },
    ContextVariableDescriptor {

        name: "timezone_offset",
        display_type: ContextValueType::String,
        description: "Local timezone offset (e.g., -0800).",
        category: "Date and Time",
        subsection: "",
        order: 25,

        example: Some(Example { invocation: "ctx.timezone_offset", result: "-0700" }),

    },
    ContextVariableDescriptor {

        name: "timezone_iana",
        display_type: ContextValueType::Timezone,
        description: "Local timezone IANA identifier (e.g., America/Los_Angeles).",
        category: "Date and Time",
        subsection: "",
        order: 26,

        example: Some(Example { invocation: "ctx.timezone_iana", result: "America/Los_Angeles" }),

    },
    ContextVariableDescriptor {

        name: "start_of_week_sun",
        display_type: ContextValueType::Date,
        description: "Date of Sunday-start week boundary (local).",
        category: "Date and Time",
        subsection: "",
        order: 27,

        example: Some(Example { invocation: "ctx.start_of_week_sun", result: "2024-06-09" }),

    },
    ContextVariableDescriptor {

        name: "end_of_week_sun",
        display_type: ContextValueType::Date,
        description: "Date of Sunday-start week end (local).",
        category: "Date and Time",
        subsection: "",
        order: 28,

        example: Some(Example { invocation: "ctx.end_of_week_sun", result: "2024-06-15" }),

    },
    ContextVariableDescriptor {

        name: "start_of_week_mon",
        display_type: ContextValueType::Date,
        description: "Date of Monday-start week boundary (local).",
        category: "Date and Time",
        subsection: "",
        order: 29,

        example: Some(Example { invocation: "ctx.start_of_week_mon", result: "2024-06-10" }),

    },
    ContextVariableDescriptor {

        name: "end_of_week_mon",
        display_type: ContextValueType::Date,
        description: "Date of Monday-start week end (local).",
        category: "Date and Time",
        subsection: "",
        order: 30,

        example: Some(Example { invocation: "ctx.end_of_week_mon", result: "2024-06-16" }),

    },
    ContextVariableDescriptor {

        name: "start_of_week_sun_utc",
        display_type: ContextValueType::Date,
        description: "Date of Sunday-start week boundary (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 31,

        example: Some(Example { invocation: "ctx.start_of_week_sun_utc", result: "2024-06-09" }),

    },
    ContextVariableDescriptor {

        name: "end_of_week_sun_utc",
        display_type: ContextValueType::Date,
        description: "Date of Sunday-start week end (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 32,

        example: Some(Example { invocation: "ctx.end_of_week_sun_utc", result: "2024-06-15" }),

    },
    ContextVariableDescriptor {

        name: "start_of_week_mon_utc",
        display_type: ContextValueType::Date,
        description: "Date of Monday-start week boundary (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 33,

        example: Some(Example { invocation: "ctx.start_of_week_mon_utc", result: "2024-06-10" }),

    },
    ContextVariableDescriptor {

        name: "end_of_week_mon_utc",
        display_type: ContextValueType::Date,
        description: "Date of Monday-start week end (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 34,

        example: Some(Example { invocation: "ctx.end_of_week_mon_utc", result: "2024-06-16" }),

    },
    ContextVariableDescriptor {

        name: "season",
        display_type: ContextValueType::String,
        description: "Current meteorological season.",
        category: "Date and Time",
        subsection: "",
        order: 35,

        example: Some(Example { invocation: "ctx.season", result: "Summer" }),

    },
    ContextVariableDescriptor {

        name: "timestamp",
        display_type: ContextValueType::Integer,
        description: "Unix timestamp in seconds (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 36,

        example: Some(Example { invocation: "ctx.timestamp", result: "1718458200" }),

    },
    ContextVariableDescriptor {

        name: "timestamp_ms",
        display_type: ContextValueType::Integer,
        description: "Unix timestamp in milliseconds (UTC).",
        category: "Date and Time",
        subsection: "",
        order: 37,

        example: Some(Example { invocation: "ctx.timestamp_ms", result: "1718458200000" }),

    },
    // ── Date and Time aliases ───────────────────────────────────────
    // Backward-compatible aliases inserted by `populate_datetime_aliases`.
    // Each mirrors a canonical key; kept so existing documents that use the
    // older spelling keep resolving. Grouped into their own subsection so the
    // report makes the redundancy explicit rather than scattering duplicates
    // among the canonical rows.
    ContextVariableDescriptor {

        name: "utc",
        display_type: ContextValueType::DateTime,
        description: "Alias of `now_utc`: UTC date and time in ISO-8601 format.",
        category: "Date and Time",
        subsection: "Aliases",
        order: 1,

        example: Some(Example { invocation: "ctx.utc", result: "2024-06-15T17:30:00Z" }),

    },
    ContextVariableDescriptor {

        name: "dow",
        display_type: ContextValueType::String,
        description: "Alias of `day`: full day of week name (local).",
        category: "Date and Time",
        subsection: "Aliases",
        order: 2,

        example: Some(Example { invocation: "ctx.dow", result: "Saturday" }),

    },
    ContextVariableDescriptor {

        name: "dow_abbr",
        display_type: ContextValueType::String,
        description: "Alias of `day_abbr`: abbreviated day of week name (local).",
        category: "Date and Time",
        subsection: "Aliases",
        order: 3,

        example: Some(Example { invocation: "ctx.dow_abbr", result: "Sat" }),

    },
    // ── Repository ──────────────────────────────────────────────────
    ContextVariableDescriptor {

        name: "repo",
        display_type: ContextValueType::String,
        description: "Repository name from preferred remote URL.",
        category: "Repository",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.repo", result: "rusty-biscuit" }),

    },
    ContextVariableDescriptor {

        name: "repo_root",
        display_type: ContextValueType::String,
        description: "Absolute path to repository root.",
        category: "Repository",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.repo_root", result: "/Users/ken/rusty-biscuit" }),

    },
    ContextVariableDescriptor {

        name: "is_monorepo",
        display_type: ContextValueType::Boolean,
        description: "Whether the repository is a monorepo.",
        category: "Repository",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.is_monorepo", result: "true" }),

    },
    ContextVariableDescriptor {

        name: "package_root",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Absolute path to current package root (monorepo only).",
        category: "Repository",
        subsection: "Packages",
        order: 1,

        example: Some(Example { invocation: "ctx.package_root", result: "claudine/lib" }),

    },
    ContextVariableDescriptor {

        name: "package_area_root",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Absolute path to current package area root (monorepo only).",
        category: "Repository",
        subsection: "Packages",
        order: 2,

        example: Some(Example { invocation: "ctx.package_area_root", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "packages",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of all packages in the monorepo.",
        category: "Repository",
        subsection: "Packages",
        order: 3,

        example: Some(Example { invocation: "ctx.packages", result: "biscuit-hash, claudine, darkmatter" }),

    },
    ContextVariableDescriptor {

        name: "packages_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of all packages in the monorepo.",
        category: "Repository",
        subsection: "Packages",
        order: 4,

        example: Some(Example { invocation: "ctx.packages_list", result: "- biscuit-hash\\n- claudine\\n- darkmatter" }),

    },
    ContextVariableDescriptor {

        name: "package_areas",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of all package areas.",
        category: "Repository",
        subsection: "Packages",
        order: 5,

        example: Some(Example { invocation: "ctx.package_areas", result: "biscuit, claudine" }),

    },
    ContextVariableDescriptor {

        name: "package_areas_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of all package areas.",
        category: "Repository",
        subsection: "Packages",
        order: 6,

        example: Some(Example { invocation: "ctx.package_areas_list", result: "- biscuit\\n- claudine" }),

    },
    ContextVariableDescriptor {

        name: "current_package",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Name of the package containing the current working directory.",
        category: "Repository",
        subsection: "Packages",
        order: 7,

        example: Some(Example { invocation: "ctx.current_package", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "current_package_area",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Name of the package area containing the current working directory.",
        category: "Repository",
        subsection: "Packages",
        order: 8,

        example: Some(Example { invocation: "ctx.current_package_area", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "area",
        display_type: ContextValueType::String,
        description: "Scoped area name: package name, area name, or empty.",
        category: "Repository",
        subsection: "Scope",
        order: 1,

        example: Some(Example { invocation: "ctx.area", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "area_description",
        display_type: ContextValueType::String,
        description: "Human-readable description of the scoped area.",
        category: "Repository",
        subsection: "Scope",
        order: 2,

        example: Some(Example { invocation: "ctx.area_description", result: "claudine package area" }),

    },
    ContextVariableDescriptor {

        name: "area_root",
        display_type: ContextValueType::String,
        description: "Absolute path to the scoped area root.",
        category: "Repository",
        subsection: "Scope",
        order: 3,

        example: Some(Example { invocation: "ctx.area_root", result: "/Users/ken/rusty-biscuit/claudine" }),

    },
    ContextVariableDescriptor {

        name: "current_packages",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown list of packages under the current working directory.",
        category: "Repository",
        subsection: "Scope",
        order: 4,

        example: Some(Example { invocation: "ctx.current_packages", result: "- claudine (claudine/lib)\\n- claudine-cli (claudine/cli)" }),

    },
    ContextVariableDescriptor {

        name: "depends_on",
        display_type: ContextValueType::NestedMarkdownList,
        description: "Nested Markdown list of dependencies for the scoped area.",
        category: "Repository",
        subsection: "Scope",
        order: 5,

        example: Some(Example { invocation: "ctx.depends_on", result: "- 'claudine' depends on:\\n    - biscuit-terminal" }),

    },
    ContextVariableDescriptor {

        name: "used_by",
        display_type: ContextValueType::NestedMarkdownList,
        description: "Nested Markdown list of packages that depend on the scoped area.",
        category: "Repository",
        subsection: "Scope",
        order: 6,

        example: Some(Example { invocation: "ctx.used_by", result: "- 'claudine' is used by:\\n    - claudine-cli" }),

    },
    // ── File Changes ────────────────────────────────────────────────
    ContextVariableDescriptor {

        name: "dirty_files",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of files with uncommitted changes.",
        category: "File Changes",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.dirty_files", result: "Cargo.toml, src/lib.rs" }),

    },
    ContextVariableDescriptor {

        name: "dirty_files_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of files with uncommitted changes.",
        category: "File Changes",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.dirty_files_list", result: "- Cargo.toml\\n- src/lib.rs" }),

    },
    ContextVariableDescriptor {

        name: "dirty_source_code_files",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of dirty source code files.",
        category: "File Changes",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.dirty_source_code_files", result: "src/lib.rs" }),

    },
    ContextVariableDescriptor {

        name: "dirty_source_code_files_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of dirty source code files.",
        category: "File Changes",
        subsection: "",
        order: 4,

        example: Some(Example { invocation: "ctx.dirty_source_code_files_list", result: "- src/lib.rs" }),

    },
    ContextVariableDescriptor {

        name: "staged_files",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of staged files.",
        category: "File Changes",
        subsection: "",
        order: 5,

        example: Some(Example { invocation: "ctx.staged_files", result: "src/lib.rs" }),

    },
    ContextVariableDescriptor {

        name: "staged_files_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of staged files.",
        category: "File Changes",
        subsection: "",
        order: 6,

        example: Some(Example { invocation: "ctx.staged_files_list", result: "- src/lib.rs" }),

    },
    ContextVariableDescriptor {

        name: "untracked_files",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of untracked files.",
        category: "File Changes",
        subsection: "",
        order: 7,

        example: Some(Example { invocation: "ctx.untracked_files", result: "new.md" }),

    },
    ContextVariableDescriptor {

        name: "untracked_files_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of untracked files.",
        category: "File Changes",
        subsection: "",
        order: 8,

        example: Some(Example { invocation: "ctx.untracked_files_list", result: "- new.md" }),

    },
    ContextVariableDescriptor {

        name: "dirty_packages",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of packages with dirty files.",
        category: "File Changes",
        subsection: "Packages",
        order: 1,

        example: Some(Example { invocation: "ctx.dirty_packages", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "dirty_packages_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of packages with dirty files.",
        category: "File Changes",
        subsection: "Packages",
        order: 2,

        example: Some(Example { invocation: "ctx.dirty_packages_list", result: "- claudine" }),

    },
    ContextVariableDescriptor {

        name: "dirty_package_areas",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of package areas with dirty files.",
        category: "File Changes",
        subsection: "Packages",
        order: 3,

        example: Some(Example { invocation: "ctx.dirty_package_areas", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "dirty_package_areas_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of package areas with dirty files.",
        category: "File Changes",
        subsection: "Packages",
        order: 4,

        example: Some(Example { invocation: "ctx.dirty_package_areas_list", result: "- claudine" }),

    },
    ContextVariableDescriptor {

        name: "staged_packages",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of packages with staged files.",
        category: "File Changes",
        subsection: "Packages",
        order: 5,

        example: Some(Example { invocation: "ctx.staged_packages", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "staged_packages_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of packages with staged files.",
        category: "File Changes",
        subsection: "Packages",
        order: 6,

        example: Some(Example { invocation: "ctx.staged_packages_list", result: "- claudine" }),

    },
    ContextVariableDescriptor {

        name: "staged_package_areas",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of package areas with staged files.",
        category: "File Changes",
        subsection: "Packages",
        order: 7,

        example: Some(Example { invocation: "ctx.staged_package_areas", result: "claudine" }),

    },
    ContextVariableDescriptor {

        name: "staged_package_areas_list",
        display_type: ContextValueType::MarkdownList,
        description: "Markdown bullet list of package areas with staged files.",
        category: "File Changes",
        subsection: "Packages",
        order: 8,

        example: Some(Example { invocation: "ctx.staged_package_areas_list", result: "- claudine" }),

    },
    ContextVariableDescriptor {

        name: "current_package_has_staged_files",
        display_type: ContextValueType::Boolean,
        description: "Whether the current package has staged files.",
        category: "File Changes",
        subsection: "Flags",
        order: 1,

        example: Some(Example { invocation: "ctx.current_package_has_staged_files", result: "false" }),

    },
    ContextVariableDescriptor {

        name: "current_package_area_has_staged_files",
        display_type: ContextValueType::Boolean,
        description: "Whether the current package area has staged files.",
        category: "File Changes",
        subsection: "Flags",
        order: 2,

        example: Some(Example { invocation: "ctx.current_package_area_has_staged_files", result: "false" }),

    },
    ContextVariableDescriptor {

        name: "current_package_has_dirty_files",
        display_type: ContextValueType::Boolean,
        description: "Whether the current package has dirty files.",
        category: "File Changes",
        subsection: "Flags",
        order: 3,

        example: Some(Example { invocation: "ctx.current_package_has_dirty_files", result: "true" }),

    },
    ContextVariableDescriptor {

        name: "current_package_area_has_dirty_files",
        display_type: ContextValueType::Boolean,
        description: "Whether the current package area has dirty files.",
        category: "File Changes",
        subsection: "Flags",
        order: 4,

        example: Some(Example { invocation: "ctx.current_package_area_has_dirty_files", result: "true" }),

    },
    // ── Languages ───────────────────────────────────────────────────
    ContextVariableDescriptor {
        name: "programming_languages_in_repo",
        display_type: ContextValueType::Nullable(&ContextValueType::Csv),
        description: "Comma-separated list of all programming languages in the repository.",
        category: "Languages",
        subsection: "",
        order: 1,


        example: Some(Example { invocation: "ctx.programming_languages_in_repo", result: "Rust, TypeScript" }),

    },
    ContextVariableDescriptor {

        name: "programming_language",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Primary programming language for the current scope.",
        category: "Languages",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.programming_language", result: "Rust" }),

    },
    ContextVariableDescriptor {

        name: "package_manager",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Primary package manager for the current scope.",
        category: "Languages",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.package_manager", result: "cargo" }),

    },
    // ── Documents ───────────────────────────────────────────────────
    ContextVariableDescriptor {

        name: "docs_readme",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of README files in scope.",
        category: "Documents",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.docs_readme", result: "README.md, claudine/README.md" }),

    },
    ContextVariableDescriptor {

        name: "docs_blast_radius",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of documents with blast_radius frontmatter.",
        category: "Documents",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.docs_blast_radius", result: "claudine/features/plan.md" }),

    },
    ContextVariableDescriptor {

        name: "docs_drift",
        display_type: ContextValueType::Csv,
        description: "Comma-separated list of documents whose blast_radius intersects dirty source files.",
        category: "Documents",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.docs_drift", result: "claudine/features/plan.md" }),

    },
    ContextVariableDescriptor {

        name: "docs_skill",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Path to the best-matching skill file for the current scope.",
        category: "Documents",
        subsection: "",
        order: 4,

        example: Some(Example { invocation: "ctx.docs_skill", result: ".claude/skills/claudine/SKILL.md" }),

    },
    // ── Operating System ────────────────────────────────────────────
    ContextVariableDescriptor {

        name: "os",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Operating system name (Windows, macOS, Linux).",
        category: "Operating System",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.os", result: "macOS" }),

    },
    ContextVariableDescriptor {

        name: "os_distro",
        display_type: ContextValueType::String,
        description: "OS distribution name.",
        category: "Operating System",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.os_distro", result: "macOS" }),

    },
    ContextVariableDescriptor {

        name: "os_package_manager",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Primary system package manager.",
        category: "Operating System",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.os_package_manager", result: "brew" }),

    },
    ContextVariableDescriptor {

        name: "os_version",
        display_type: ContextValueType::String,
        description: "OS version string.",
        category: "Operating System",
        subsection: "",
        order: 4,

        example: Some(Example { invocation: "ctx.os_version", result: "14.5" }),

    },
    // ── Hardware ────────────────────────────────────────────────────
    ContextVariableDescriptor {

        name: "memory_total",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Total system memory as human-readable string.",
        category: "Hardware",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.memory_total", result: "32 GB" }),

    },
    ContextVariableDescriptor {

        name: "memory_used",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Percentage of memory currently in use.",
        category: "Hardware",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.memory_used", result: "45%" }),

    },
    ContextVariableDescriptor {

        name: "memory_avail",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "Available memory as human-readable string.",
        category: "Hardware",
        subsection: "",
        order: 3,

        example: Some(Example { invocation: "ctx.memory_avail", result: "18 GB" }),

    },
    ContextVariableDescriptor {

        name: "cpu_cores",
        display_type: ContextValueType::Nullable(&ContextValueType::Integer),
        description: "Number of logical CPU cores.",
        category: "Hardware",
        subsection: "",
        order: 4,

        example: Some(Example { invocation: "ctx.cpu_cores", result: "12" }),

    },
    ContextVariableDescriptor {

        name: "cpu_arch",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "CPU architecture string.",
        category: "Hardware",
        subsection: "",
        order: 5,

        example: Some(Example { invocation: "ctx.cpu_arch", result: "arm64" }),

    },
    ContextVariableDescriptor {

        name: "gpu",
        display_type: ContextValueType::Nullable(&ContextValueType::String),
        description: "GPU name(s) or null when unavailable.",
        category: "Hardware",
        subsection: "",
        order: 6,

        example: Some(Example { invocation: "ctx.gpu", result: "Apple M3 Max" }),

    },
    // ── Agent ───────────────────────────────────────────────────────
    ContextVariableDescriptor {

        name: "agent",
        display_type: ContextValueType::String,
        description: "Name of the executing agentic CLI, trimmed from the AGENT env var; defaults to \"unknown\".",
        category: "Agent",
        subsection: "",
        order: 1,

        example: Some(Example { invocation: "ctx.agent", result: "opencode" }),

    },
    ContextVariableDescriptor {

        name: "model",
        display_type: ContextValueType::String,
        description: "Active model identifier, trimmed from the MODEL env var; defaults to \"default\".",
        category: "Agent",
        subsection: "",
        order: 2,

        example: Some(Example { invocation: "ctx.model", result: "kimi-for-coding/k2p7" }),

    },
];

/// Returns all context variable descriptors in display order.
pub fn context_variable_descriptors() -> &'static [ContextVariableDescriptor] {
    CONTEXT_VARIABLE_DESCRIPTORS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeContext;
    use std::collections::HashSet;

    /// The catalog must describe exactly the variables the runtime produces.
    ///
    /// The "runtime" side is a real [`ComposeContext::capture`] rather than a
    /// hand-maintained name list, so adding, removing, or renaming an inserted
    /// key (including the backward-compatible `utc`/`dow`/`dow_abbr` aliases)
    /// breaks this test until its descriptor is updated. Capture inserts a
    /// fixed key set in every environment (unavailable values become `null`),
    /// so the produced set is stable regardless of where the test runs.
    #[test]
    fn descriptor_name_set_equals_captured_runtime_key_set() {
        let descriptor_names: HashSet<&str> = CONTEXT_VARIABLE_DESCRIPTORS
            .iter()
            .map(|d| d.name)
            .collect();

        let ctx = ComposeContext::capture_for_dir(&std::env::temp_dir());
        let runtime_names: HashSet<&str> =
            ctx.values().keys().map(|k| k.as_str()).collect();

        let missing_descriptors: Vec<_> =
            runtime_names.difference(&descriptor_names).collect();
        let extra_descriptors: Vec<_> =
            descriptor_names.difference(&runtime_names).collect();

        assert!(
            missing_descriptors.is_empty(),
            "Runtime keys without descriptors: {missing_descriptors:?}"
        );
        assert!(
            extra_descriptors.is_empty(),
            "Descriptor names without runtime keys: {extra_descriptors:?}"
        );
    }

    #[test]
    fn descriptor_traversal_order_is_deterministic() {
        let names: Vec<&str> = CONTEXT_VARIABLE_DESCRIPTORS
            .iter()
            .map(|d| d.name)
            .collect();
        let names_again: Vec<&str> = CONTEXT_VARIABLE_DESCRIPTORS
            .iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(names, names_again);
    }

    #[test]
    fn descriptor_names_are_unique() {
        let mut seen = HashSet::new();
        for d in CONTEXT_VARIABLE_DESCRIPTORS {
            assert!(
                seen.insert(d.name),
                "Duplicate descriptor name: {}",
                d.name
            );
        }
    }

    #[test]
    fn catalog_access_performs_no_capture() {
        // Reading the catalog is pure static metadata access: no host probe,
        // no I/O, no runtime capture. (The parity test above is the only place
        // that intentionally captures, and it lives in the test module.)
        let _ = context_variable_descriptors();
        // If we reach here without panicking or hanging, the test passes.
    }
}

#[cfg(test)]
mod phase2_tests {
    use super::*;
    use crate::markdown::compose::ComposeContext;
    use serde_json::Value;

    fn value_shape_matches(value: &Value, shape: &ContextValueType) -> bool {
        match shape {
            ContextValueType::Nullable(inner) => {
                value.is_null() || value_shape_matches(value, inner)
            }
            ContextValueType::Date
            | ContextValueType::DateTime
            | ContextValueType::Time
            | ContextValueType::Timezone
            | ContextValueType::String
            | ContextValueType::Csv
            | ContextValueType::MarkdownList
            | ContextValueType::NestedMarkdownList => value.is_string(),
            ContextValueType::Integer | ContextValueType::Number => value.is_number(),
            ContextValueType::Boolean => value.is_boolean(),
            ContextValueType::Object => value.is_object(),
        }
    }

    fn parse_example_result(result: &str, shape: &ContextValueType) -> Value {
        match shape {
            ContextValueType::Nullable(inner) => {
                if result == "null" {
                    Value::Null
                } else {
                    parse_example_result(result, inner)
                }
            }
            ContextValueType::Integer | ContextValueType::Number => {
                if let Ok(n) = result.parse::<i64>() {
                    Value::Number(n.into())
                } else {
                    Value::Number(
                        serde_json::Number::from_f64(result.parse::<f64>().unwrap()).unwrap(),
                    )
                }
            }
            ContextValueType::Boolean => Value::Bool(result.parse::<bool>().unwrap()),
            ContextValueType::Object => serde_json::from_str(result).unwrap(),
            _ => Value::String(result.to_string()),
        }
    }

    #[test]
    fn capture_value_shape_matches_display_type() {
        let repo_root = crate::markdown::compose::find_git_root_from(std::path::Path::new("."))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let ctx = ComposeContext::capture_for_dir(&repo_root);
        let mut failures = Vec::new();
        for d in CONTEXT_VARIABLE_DESCRIPTORS {
            let value = ctx.values().get(d.name).unwrap_or(&Value::Null);
            if !value_shape_matches(value, &d.display_type) {
                failures.push((d.name, d.display_type, value.clone()));
            }
        }
        assert!(
            failures.is_empty(),
            "context value shapes do not match display_type: {failures:?}"
        );
    }

    #[test]
    fn context_example_results_are_type_consistent() {
        let mut failures = Vec::new();
        for d in CONTEXT_VARIABLE_DESCRIPTORS {
            if let Some(example) = d.example() {
                let expected = parse_example_result(example.result, &d.display_type);
                if !value_shape_matches(&expected, &d.display_type) {
                    failures.push((d.name, d.display_type, example.result, expected));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "example results are not type-consistent with display_type: {failures:?}"
        );
    }
}

