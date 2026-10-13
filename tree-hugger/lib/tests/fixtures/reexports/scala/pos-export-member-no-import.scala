object O:
  val impl = new Impl
  export impl.run
  export impl.{stop, start as begin}
