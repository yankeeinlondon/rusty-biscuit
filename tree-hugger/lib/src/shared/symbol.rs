
pub enum MethodScope {
    Public,
    Protected,
    Private,
    Other(String)
}

pub enum VariableKind {
    Constant,
    Let,
    Var,
    Other(String)
}

pub enum TypeKind {
    Interface,
    Enumeration,
    Scalar,
    Other(String)
}


pub enum SymbolKind {
    Function,
    Class,
    ClassMethod(MethodScope),
    Variable(VariableKind),
    Type(TypeKind)
}


pub trait Symbol {
  // TODO
}
