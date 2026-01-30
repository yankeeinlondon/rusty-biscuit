# Extending the Research Package

Guide to adding new research types, prompts, and providers.

## Adding New Research Types

Research types are defined in the `ResearchDetails` enum.

### Step 1: Add Detail Struct

In `/research/lib/src/metadata/types.rs`:

```rust
/// Details for widget research.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WidgetDetails {
    /// The widget platform (e.g., "Qt", "GTK")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// Widget category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}
```

### Step 2: Add Enum Variant

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ResearchDetails {
    // ... existing variants
    /// Details for widget research
    Widget(WidgetDetails),
}
```

### Step 3: Update type_name()

```rust
impl ResearchDetails {
    pub fn type_name(&self) -> &'static str {
        match self {
            // ... existing matches
            Self::Widget(_) => "Widget",
        }
    }
}
```

### Step 4: Export from mod.rs

In `/research/lib/src/metadata/mod.rs`:

```rust
pub use types::{
    // ... existing exports
    WidgetDetails,
};
```

### Step 5: Add Tests

```rust
#[test]
fn test_widget_details_serialization() {
    let details = WidgetDetails {
        platform: Some("Qt".to_string()),
        category: Some("Input".to_string()),
    };

    let json = serde_json::to_string(&details).unwrap();
    assert!(json.contains("Qt"));

    let roundtrip: WidgetDetails = serde_json::from_str(&json).unwrap();
    assert_eq!(details, roundtrip);
}
```

## Adding New Prompt Templates

Prompt templates live in `/research/lib/prompts/`.

### Step 1: Create Template File

Create `/research/lib/prompts/widget_overview.md`:

```markdown
You are researching the {{topic}} widget.

Provide a comprehensive overview including:
- Widget purpose and use cases
- Platform compatibility
- Customization options
- Performance characteristics

Focus on practical usage patterns and common configurations.
```

### Step 2: Add to Embedded Prompts

In `/research/lib/src/lib.rs`:

```rust
mod prompts {
    // ... existing prompts
    pub const WIDGET_OVERVIEW: &str = include_str!("../prompts/widget_overview.md");
}
```

### Step 3: Add to STANDARD_PROMPTS (if applicable)

```rust
const WIDGET_PROMPTS: [(&str, &str, &str); 2] = [
    ("widget_overview", "widget_overview.md", prompts::WIDGET_OVERVIEW),
    ("widget_integration", "widget_integration.md", prompts::WIDGET_INTEGRATION),
];
```

## Adding New CLI Commands

Commands are defined in `/research/cli/src/main.rs`.

### Step 1: Add Command Variant

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands

    /// Research a UI widget
    Widget {
        /// The widget name
        #[arg(required = true, value_name = "WIDGET_NAME")]
        widget_name: String,

        /// Widget platform (e.g., "qt", "gtk")
        #[arg(short, long)]
        platform: Option<String>,

        /// Additional research questions
        #[arg(value_name = "QUESTIONS")]
        questions: Vec<String>,

        /// Output directory
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,
    },
}
```

### Step 2: Handle Command

```rust
Commands::Widget {
    widget_name,
    platform,
    questions,
    output,
} => {
    match research_lib::research_widget(
        &widget_name,
        platform.as_deref(),
        output,
        &questions,
    ).await {
        Ok(result) => {
            println!("Widget research complete: {:?}", result.output_dir);
        }
        Err(e) => {
            eprintln!("Widget research failed: {}", e);
            std::process::exit(1);
        }
    }
}
```

### Step 3: Implement Library Function

In `/research/lib/src/lib.rs`:

```rust
/// Research a UI widget.
pub async fn research_widget(
    name: &str,
    platform: Option<&str>,
    output: Option<PathBuf>,
    questions: &[String],
) -> Result<ResearchResult, ResearchError> {
    // Implementation
}
```

## Adding New LLM Providers

Providers are configured in `/research/lib/src/lib.rs`.

### Using Existing rig Providers

```rust
use rig::providers::anthropic;

let client = anthropic::Client::from_env();
let model = client.agent("claude-3-opus-20240229").build();
```

### Custom OpenAI-Compatible Provider

```rust
use rig::providers::openai;

let client = openai::Client::new("https://custom-api.example.com/v1")
    .api_key("your-api-key");
let model = client.agent("custom-model").build();
```

### Adding Provider to Pipeline

In the research function:

```rust
// Configure which model to use for which task
let overview_model = match std::env::var("USE_CUSTOM_OVERVIEW") {
    Ok(_) => custom_client.agent("custom-model").build(),
    Err(_) => zai_client.agent("glm-4.7").build(),
};
```

## Adding New Tools

Tools extend agent capabilities during Phase 1 research.

### Implementing a Tool

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct WidgetSearchArgs {
    pub query: String,
    pub platform: Option<String>,
}

pub struct WidgetSearchTool {
    http_client: reqwest::Client,
}

impl Tool for WidgetSearchTool {
    const NAME: &'static str = "widget_search";
    type Error = reqwest::Error;
    type Args = WidgetSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search for widget documentation and examples".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WidgetSearchArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Implementation
    }
}
```

### Registering Tool with Agent

```rust
let agent = client
    .agent("gpt-4o")
    .tool(WidgetSearchTool::new())
    .tool(BraveSearchTool::new())
    .build();
```

## Best Practices

1. **Maintain backward compatibility**: Use `#[non_exhaustive]` on enums
2. **Skip None in serialization**: Use `#[serde(skip_serializing_if = "Option::is_none")]`
3. **Provide defaults**: Implement `Default` for all detail structs
4. **Test roundtrips**: Verify serialization/deserialization cycles
5. **Document templates**: Include variable placeholders in template docs
6. **Use feature flags**: Gate optional providers behind Cargo features
