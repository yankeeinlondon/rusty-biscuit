import Foundation

/// A 2D point structure.
struct Point {
    var x: Int
    var y: Int
}

/// A greeter class.
class Greeter {
    var prefix: String = "Hello"

    func greet(name: String) -> String {
        return "\(prefix), \(name)!"
    }
}

/// Status codes for responses.
enum Status {
    case success
    case error
    case pending
}

/// A greeting service protocol.
protocol GreetingService {
    func greet(name: String) -> String
}
