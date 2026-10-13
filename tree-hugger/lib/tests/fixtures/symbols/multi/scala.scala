// Multi-file fixture: Scala.

package multi

trait Named {
  def name: String
}

class Tag(val name: String) extends Named {
  def upper(): String = name.toUpperCase
}

object Tag {
  def empty(): Tag = new Tag("")
}
