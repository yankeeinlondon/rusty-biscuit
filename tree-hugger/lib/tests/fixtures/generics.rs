use std::fmt::Display;

/// Generic identity function.
pub fn identity<T>(value: T) -> T {
    value
}

/// Maps a value using a function.
pub fn map_value<T, U, F>(value: T, f: F) -> U
where
    F: FnOnce(T) -> U,
{
    f(value)
}

/// Generic struct with lifetime.
pub struct Container<'a, T> {
    value: &'a T,
}

impl<'a, T> Container<'a, T> {
    /// Creates a new container.
    pub fn new(value: &'a T) -> Self {
        Self { value }
    }

    /// Gets the value with a bound.
    pub fn get_display(&self) -> String
    where
        T: Display,
    {
        self.value.to_string()
    }
}

/// Function returning Result with generic error.
pub fn try_parse<T, E>(input: &str) -> Result<T, E>
where
    T: std::str::FromStr<Err = E>,
{
    input.parse()
}

/// Function with impl Trait return.
pub fn make_iter(n: usize) -> impl Iterator<Item = usize> {
    0..n
}
