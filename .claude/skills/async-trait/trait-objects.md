# Trait Object Patterns with async-trait

Practical patterns for using async traits with dynamic dispatch.

## The Registry Pattern

A common pattern for plugin systems and dependency injection:

```rust
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Scanner: Send + Sync {
    /// Unique identifier for this scanner
    fn name(&self) -> &'static str;

    /// Check if the scanner's backend is available
    async fn is_available(&self) -> bool;

    /// Scan for items
    async fn scan(&self) -> Result<Vec<Item>, ScanError>;
}

pub struct Registry {
    scanners: Vec<Arc<dyn Scanner>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { scanners: Vec::new() }
    }

    pub fn register(&mut self, scanner: impl Scanner + 'static) {
        self.scanners.push(Arc::new(scanner));
    }

    pub async fn scan_all(&self) -> Vec<Item> {
        let futures: Vec<_> = self.scanners
            .iter()
            .map(|s| s.scan())
            .collect();

        futures::future::join_all(futures)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .flatten()
            .collect()
    }

    pub async fn scan_available(&self) -> Vec<Item> {
        let mut results = Vec::new();

        for scanner in &self.scanners {
            if scanner.is_available().await {
                if let Ok(items) = scanner.scan().await {
                    results.extend(items);
                }
            }
        }

        results
    }
}
```

## Box vs Arc

Choose based on ownership needs:

### Box<dyn Trait> - Exclusive Ownership

```rust
struct Processor {
    handler: Box<dyn Handler>,  // Single owner
}

impl Processor {
    pub fn new(handler: impl Handler + 'static) -> Self {
        Self { handler: Box::new(handler) }
    }

    pub async fn process(&self, data: &[u8]) {
        self.handler.handle(data).await;
    }
}
```

### Arc<dyn Trait> - Shared Ownership

```rust
struct Server {
    handlers: Vec<Arc<dyn Handler>>,
}

impl Server {
    pub async fn broadcast(&self, data: &[u8]) {
        let futures: Vec<_> = self.handlers
            .iter()
            .cloned()  // Arc clone is cheap
            .map(|h| async move { h.handle(data).await })
            .collect();

        futures::future::join_all(futures).await;
    }
}
```

## Factory Pattern

Creating trait objects from configuration:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn fetch(&self, id: &str) -> Result<Data, Error>;
}

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(config: &Config) -> Box<dyn Provider> {
        match config.provider_type.as_str() {
            "ollama" => Box::new(OllamaProvider::new(&config.ollama)),
            "lmstudio" => Box::new(LmStudioProvider::new(&config.lmstudio)),
            "llamacpp" => Box::new(LlamaCppProvider::new(&config.llamacpp)),
            _ => Box::new(NullProvider),
        }
    }

    pub fn create_all(config: &Config) -> Vec<Box<dyn Provider>> {
        let mut providers: Vec<Box<dyn Provider>> = Vec::new();

        if config.ollama.enabled {
            providers.push(Box::new(OllamaProvider::new(&config.ollama)));
        }
        if config.lmstudio.enabled {
            providers.push(Box::new(LmStudioProvider::new(&config.lmstudio)));
        }

        providers
    }
}
```

## Dependency Injection

Using trait objects for testability:

```rust
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: &str) -> Result<Response, Error>;
    async fn post(&self, url: &str, body: &[u8]) -> Result<Response, Error>;
}

pub struct ApiClient {
    http: Arc<dyn HttpClient>,
    base_url: String,
}

impl ApiClient {
    pub fn new(http: Arc<dyn HttpClient>, base_url: String) -> Self {
        Self { http, base_url }
    }

    pub async fn fetch_models(&self) -> Result<Vec<Model>, Error> {
        let url = format!("{}/models", self.base_url);
        let response = self.http.get(&url).await?;
        Ok(serde_json::from_slice(&response.body)?)
    }
}

// Production implementation
pub struct ReqwestClient {
    client: reqwest::Client,
}

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn get(&self, url: &str) -> Result<Response, Error> {
        let resp = self.client.get(url).send().await?;
        Ok(Response {
            status: resp.status().as_u16(),
            body: resp.bytes().await?.to_vec(),
        })
    }

    async fn post(&self, url: &str, body: &[u8]) -> Result<Response, Error> {
        let resp = self.client.post(url).body(body.to_vec()).send().await?;
        Ok(Response {
            status: resp.status().as_u16(),
            body: resp.bytes().await?.to_vec(),
        })
    }
}

// Test mock
#[cfg(test)]
pub struct MockHttpClient {
    responses: std::collections::HashMap<String, Response>,
}

#[cfg(test)]
#[async_trait]
impl HttpClient for MockHttpClient {
    async fn get(&self, url: &str) -> Result<Response, Error> {
        self.responses
            .get(url)
            .cloned()
            .ok_or(Error::NotFound)
    }

    async fn post(&self, _url: &str, _body: &[u8]) -> Result<Response, Error> {
        Ok(Response { status: 200, body: vec![] })
    }
}
```

## Concurrent Execution with Timeout

Handling slow or failing implementations:

```rust
use tokio::time::{timeout, Duration};

impl Registry {
    pub async fn scan_with_timeout(&self, dur: Duration) -> Vec<ScanResult> {
        let futures: Vec<_> = self.scanners
            .iter()
            .map(|scanner| {
                let scanner = scanner.clone();
                async move {
                    let name = scanner.name();
                    match timeout(dur, scanner.scan()).await {
                        Ok(Ok(items)) => ScanResult::Success { name, items },
                        Ok(Err(e)) => ScanResult::Error { name, error: e.to_string() },
                        Err(_) => ScanResult::Timeout { name },
                    }
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }
}

pub enum ScanResult {
    Success { name: &'static str, items: Vec<Item> },
    Error { name: &'static str, error: String },
    Timeout { name: &'static str },
}
```

## Type-Erased Builders

Using trait objects in builder patterns:

```rust
#[async_trait]
pub trait Step: Send + Sync {
    async fn execute(&self, context: &mut Context) -> Result<(), Error>;
    fn name(&self) -> &str;
}

pub struct Pipeline {
    steps: Vec<Box<dyn Step>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(mut self, step: impl Step + 'static) -> Self {
        self.steps.push(Box::new(step));
        self
    }

    pub async fn execute(&self, context: &mut Context) -> Result<(), Error> {
        for step in &self.steps {
            tracing::info!(step = step.name(), "Executing step");
            step.execute(context).await?;
        }
        Ok(())
    }
}

// Usage
let pipeline = Pipeline::new()
    .add_step(ValidateStep::new())
    .add_step(TransformStep::new())
    .add_step(SaveStep::new());

pipeline.execute(&mut ctx).await?;
```

## Trait Object Collections

Different storage patterns:

```rust
// Heterogeneous collection
pub struct HandlerChain {
    handlers: Vec<Box<dyn Handler>>,
}

// Named lookup
pub struct HandlerRegistry {
    handlers: HashMap<String, Arc<dyn Handler>>,
}

// Priority queue
pub struct PriorityHandlers {
    handlers: BinaryHeap<PrioritizedHandler>,
}

struct PrioritizedHandler {
    priority: i32,
    handler: Arc<dyn Handler>,
}

impl Ord for PrioritizedHandler {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}
```

## Error Handling

Returning trait objects from fallible operations:

```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    async fn query(&self, id: &str) -> Result<Data, Error>;
}

pub async fn get_best_source(
    config: &Config
) -> Result<Box<dyn DataSource>, Error> {
    // Try sources in priority order
    if let Ok(source) = PrimarySource::connect(&config.primary).await {
        return Ok(Box::new(source));
    }

    if let Ok(source) = BackupSource::connect(&config.backup).await {
        return Ok(Box::new(source));
    }

    Err(Error::NoSourceAvailable)
}
```

## Testing Patterns

Mocking async trait objects:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockScanner {
        items: Vec<Item>,
        available: bool,
    }

    #[async_trait]
    impl Scanner for MockScanner {
        fn name(&self) -> &'static str { "mock" }

        async fn is_available(&self) -> bool {
            self.available
        }

        async fn scan(&self) -> Result<Vec<Item>, ScanError> {
            Ok(self.items.clone())
        }
    }

    #[tokio::test]
    async fn test_registry_aggregates_results() {
        let mut registry = Registry::new();

        registry.register(MockScanner {
            items: vec![Item::new("a")],
            available: true,
        });

        registry.register(MockScanner {
            items: vec![Item::new("b"), Item::new("c")],
            available: true,
        });

        let results = registry.scan_all().await;
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_registry_skips_unavailable() {
        let mut registry = Registry::new();

        registry.register(MockScanner {
            items: vec![Item::new("available")],
            available: true,
        });

        registry.register(MockScanner {
            items: vec![Item::new("unavailable")],
            available: false,
        });

        let results = registry.scan_available().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "available");
    }
}
```

## Best Practices Summary

1. **Use `Arc<dyn Trait>`** when sharing across tasks/threads
2. **Use `Box<dyn Trait>`** for single ownership
3. **Add `Send + Sync`** bounds to trait definition for multi-threaded use
4. **Implement timeouts** for unreliable backends
5. **Use factory pattern** for configuration-driven instantiation
6. **Prefer trait objects for hot-swappable components**
7. **Use generics for performance-critical paths** when type is known
