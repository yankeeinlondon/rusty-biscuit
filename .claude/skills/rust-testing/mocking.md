# Mocking with Mockall

Mockall generates mock implementations of traits for testing code with external dependencies.

## Setup

```toml
# Cargo.toml
[dev-dependencies]
mockall = "0.13"
```

## Basic Mocking

```rust
use mockall::automock;

#[automock]
trait Database {
    fn get_user(&self, id: u32) -> Option<User>;
    fn save_user(&self, user: &User) -> Result<(), Error>;
}

// Production code uses the trait
pub struct UserService<D: Database> {
    db: D,
}

impl<D: Database> UserService<D> {
    pub fn new(db: D) -> Self {
        Self { db }
    }

    pub fn find_user(&self, id: u32) -> Option<User> {
        self.db.get_user(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_existing_user() {
        let mut mock_db = MockDatabase::new();

        mock_db
            .expect_get_user()
            .with(mockall::predicate::eq(42))
            .times(1)
            .returning(|_| Some(User { id: 42, name: "Test".into() }));

        let service = UserService::new(mock_db);
        let user = service.find_user(42);

        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Test");
    }
}
```

## Expectation Methods

```rust
#[test]
fn mock_expectations() {
    let mut mock = MockDatabase::new();

    // Expect specific argument
    mock.expect_get_user()
        .with(mockall::predicate::eq(1))
        .returning(|_| Some(User::default()));

    // Expect any argument
    mock.expect_get_user()
        .withf(|id| *id > 0)
        .returning(|id| Some(User { id, ..Default::default() }));

    // Expect exact number of calls
    mock.expect_save_user()
        .times(2)
        .returning(|_| Ok(()));

    // Expect range of calls
    mock.expect_get_user()
        .times(1..=3)
        .returning(|_| None);
}
```

## Returning Values

```rust
#[test]
fn return_patterns() {
    let mut mock = MockDatabase::new();

    // Return owned value
    mock.expect_get_user()
        .returning(|_| Some(User::default()));

    // Return different values on successive calls
    mock.expect_get_user()
        .times(1)
        .returning(|_| Some(User { id: 1, ..Default::default() }));
    mock.expect_get_user()
        .times(1)
        .returning(|_| None);

    // Return error
    mock.expect_save_user()
        .returning(|_| Err(Error::NotFound));
}
```

## Argument Matching

```rust
use mockall::predicate::*;

#[test]
fn argument_predicates() {
    let mut mock = MockDatabase::new();

    // Exact match
    mock.expect_get_user()
        .with(eq(42))
        .returning(|_| None);

    // Custom predicate
    mock.expect_get_user()
        .withf(|id| *id > 100)
        .returning(|_| None);

    // Any argument
    mock.expect_get_user()
        .with(always())
        .returning(|_| None);

    // Multiple arguments
    mock.expect_complex_method()
        .with(eq(1), str::starts_with("test"))
        .returning(|_, _| Ok(()));
}
```

## Sequences

Enforce call order:

```rust
use mockall::Sequence;

#[test]
fn ordered_calls() {
    let mut mock = MockDatabase::new();
    let mut seq = Sequence::new();

    mock.expect_get_user()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| None);

    mock.expect_save_user()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| Ok(()));

    // get_user must be called before save_user
}
```

## Mocking Structs (not just traits)

```rust
use mockall::automock;

pub struct HttpClient {
    base_url: String,
}

#[automock]
impl HttpClient {
    pub fn get(&self, path: &str) -> Result<String, Error> {
        // Real implementation
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_mock_client() {
        let mut mock = MockHttpClient::new();
        mock.expect_get()
            .returning(|_| Ok("response".into()));

        // Use mock in tests
    }
}
```

## Design for Testability

```rust
// Define behavior as trait
pub trait Fetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<String, Error>;
}

// Production implementation
pub struct HttpFetcher;

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str) -> Result<String, Error> {
        // Real HTTP call
        todo!()
    }
}

// Service depends on trait, not concrete type
pub struct DataService<F: Fetcher> {
    fetcher: F,
}

impl<F: Fetcher> DataService<F> {
    pub fn new(fetcher: F) -> Self {
        Self { fetcher }
    }

    pub fn get_data(&self) -> Result<Data, Error> {
        let raw = self.fetcher.fetch("https://api.example.com/data")?;
        // Process raw data
        Ok(Data::parse(&raw)?)
    }
}
```

## Alternative: Manual Test Doubles

For simple cases, a manual fake may be clearer:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        response: String,
    }

    impl Fetcher for FakeFetcher {
        fn fetch(&self, _url: &str) -> Result<String, Error> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn processes_data_correctly() {
        let fetcher = FakeFetcher {
            response: r#"{"key": "value"}"#.into(),
        };
        let service = DataService::new(fetcher);

        let data = service.get_data().unwrap();
        assert_eq!(data.key, "value");
    }
}
```

## Related

- [Unit Tests](./unit-tests.md) - Basic testing patterns
- [Integration Tests](./integration-tests.md) - Testing with real dependencies
