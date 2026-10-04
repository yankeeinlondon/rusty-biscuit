// Symbol-kind, visibility, and static fixture for Scala 3.
package example.symbols

def topLevelFunction(parameterValue: Int): Int =
  val localValue = parameterValue + 1
  localValue

private def privateTopLevelFunction(): Unit = ()

class PlainWidget(val constructorField: Int):
  val publicField: Int = 1
  private val privateField: Int = 2
  protected val protectedField: Int = 3
  private[symbols] val packageQualifiedField: Int = 4
  private[this] val thisQualifiedField: Int = 5

  def publicMethod(): Int = privateField
  private def privateMethod(): Unit = ()
  protected def protectedMethod(): Unit = ()
  private[symbols] def packageQualifiedMethod(): Unit = ()
  private[PlainWidget] def classQualifiedMethod(): Unit = ()

object PlainWidget:
  val companionStaticField: Int = 6
  def companionStaticMethod(): PlainWidget = new PlainWidget(0)

trait PlainTrait:
  def abstractTraitMethod(): Int
  def concreteTraitMethod(): Int = 7

object StandaloneObject:
  def objectMethod(): Unit = ()
  private def privateObjectMethod(): Unit = ()

case class CasePoint(xCoordinate: Int, yCoordinate: Int):
  def sum(): Int = xCoordinate + yCoordinate

enum PlainColor:
  case Red, Green

private class PrivateClass:
  def methodInPrivateClass(): Unit = ()
