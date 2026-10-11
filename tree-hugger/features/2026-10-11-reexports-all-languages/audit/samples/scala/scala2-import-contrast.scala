package object p extends q.Trait {
  val x = q.x
  type T = q.T
  def f = q.f
}
object O { import a.b; export2 }
