use koopa::ir::Value;

#[derive(Clone)]
pub enum SymbolTableEntry {
    Const(String, i32),
    Var(Value),
}