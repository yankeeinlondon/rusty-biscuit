package example

/** A 2D point class. */
class Point(val x: Int, val y: Int)

/** A greeter trait. */
trait Greeter {
  def greet(name: String): String
}

/** A singleton greeter object. */
object DefaultGreeter extends Greeter {
  def greet(name: String): String = s"Hello, $name!"
}

/** Status codes (Scala 3 enum). */
enum Status {
  case Success
  case Error
  case Pending
}
