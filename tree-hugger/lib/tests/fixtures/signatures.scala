package example

/** A simple greeting function. */
def greet(name: String): String = {
  s"Hello, $name!"
}

/** A function with multiple parameters. */
def greetWithAge(name: String, age: Int): String = {
  s"Hello, $name ($age)!"
}

/** A function with default parameter. */
def greetWithPrefix(name: String, prefix: String = "Hello"): String = {
  s"$prefix, $name!"
}

/** A void function. */
def printGreeting(message: String): Unit = {
  println(message)
}

/** A function with varargs. */
def greetMany(names: String*): Unit = {
  names.foreach(name => println(s"Hello, $name"))
}

/** A class with methods. */
class Greeter(val prefix: String = "Hello") {
  /** A simple greeting method. */
  def greet(name: String): String = {
    s"$prefix, $name!"
  }
}

/** An object with methods. */
object DefaultGreeter {
  def greet(name: String): String = s"Hello, $name!"
}
