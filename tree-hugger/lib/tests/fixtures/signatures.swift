import Foundation

/// A simple greeting function.
func greet(name: String) -> String {
    return "Hello, \(name)!"
}

/// A function with multiple parameters.
func greetWithAge(name: String, age: Int) -> String {
    return "Hello, \(name) (\(age))!"
}

/// A function with default parameter.
func greetWithPrefix(name: String, prefix: String = "Hello") -> String {
    return "\(prefix), \(name)!"
}

/// A void function.
func printGreeting(message: String) {
    print(message)
}

/// A function with variadic parameter.
func greetMany(names: String...) {
    for name in names {
        print("Hello, \(name)")
    }
}

/// A class with methods.
class Greeter {
    var prefix: String

    /// Initializer with parameter.
    init(prefix: String = "Hello") {
        self.prefix = prefix
    }

    /// A simple greeting method.
    func greet(name: String) -> String {
        return "\(prefix), \(name)!"
    }
}
