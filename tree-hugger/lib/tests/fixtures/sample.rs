/// Greets a person by name.
///
/// ## Arguments
/// * `name` - The name of the person to greet
///
/// ## Returns
/// A formatted greeting string.
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Greets multiple people.
///
/// ## Arguments
/// * `names` - A slice of names to greet
pub fn greet_many(names: &[String]) {
    for name in names {
        println!("{}", greet(name));
    }
}

/// A struct that can generate greetings.
pub struct Greeter {
    /// The prefix to use for greetings.
    prefix: String,
}

impl Greeter {
    /// Creates a new Greeter with the given prefix.
    pub fn new(prefix: String) -> Self {
        Self { prefix }
    }

    /// Greets a person using this greeter's prefix.
    pub fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.prefix, name)
    }
}
