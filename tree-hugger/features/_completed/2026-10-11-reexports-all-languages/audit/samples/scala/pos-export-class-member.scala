class Facade(impl: Impl):
  export impl.*
  export impl.{m as n}
  private export impl.hidden
  protected export impl.p
