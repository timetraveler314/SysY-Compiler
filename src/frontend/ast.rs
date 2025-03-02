use crate::common::environment::{IREnvironment};
use crate::frontend::FrontendError;
use crate::frontend::FrontendError::{BindingNonConstExpr, ConstEvalDivZero};
use crate::frontend::symbol::SymbolTableEntry;

#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub ident: String,
    pub block: Block,
}

#[derive(Debug)]
pub enum FuncType {
    Int,
}

#[derive(Debug)]
pub enum BType {
    Int,
}

#[derive(Debug)]
pub struct Block {
    pub items: Vec<BlockItem>
}

#[derive(Debug)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug)]
pub enum Decl {
    ConstDecl(ConstDecl),
    VarDecl(VarDecl),
}

#[derive(Debug)]
pub struct ConstDecl {
    pub btype: BType,
    pub defs: Vec<ConstDef>,
}

#[derive(Debug)]
pub struct ConstDef {
    pub ident: String,
    pub init_val: ConstInitVal,
}

#[derive(Debug)]
pub enum ConstInitVal {
    Expr(Expr),
}

#[derive(Debug)]
pub struct VarDecl {
    pub btype: BType,
    pub defs: Vec<VarDef>,
}

#[derive(Debug)]
pub enum VarDef {
    Ident(String),
    Init(String, Expr),
}

#[derive(Debug)]
pub enum Stmt {
    Return(Expr),
    Assign(LVal, Expr),
    Expr(Expr),
    Empty,
    Block(Block),
    If(Expr, Box<Stmt>),
    IfElse(Expr, Box<Stmt>, Box<Stmt>),
    While(Expr, Box<Stmt>),
    Break,
    Continue,
}

#[derive(Debug)]
pub enum InitVal {
    Expr(Expr),
}

#[derive(Debug)]
pub enum LVal {
    Ident(String),
}

impl LVal {
    pub fn ident(&self) -> &str {
        match self {
            LVal::Ident(ident) => ident,
        }
    }
}

#[derive(Debug)]
pub enum Expr {
    Num(i32),
    LVal(LVal),
    Pos(Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    // Lv 3.3 Comparison and logic
    Lt(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Land(Box<Expr>, Box<Expr>),
    Lor(Box<Expr>, Box<Expr>),
}

// macro rule for binary
macro_rules! binary_expr_eval_rule {
    ($env:expr, $lhs:expr, $rhs:expr, $op:expr) => {{
        let lhs_val = $lhs.try_const_eval($env)?;
        let rhs_val = $rhs.try_const_eval($env)?;
        Ok($op(lhs_val, rhs_val))
    }};
}

impl Expr {
    pub fn has_side_effect(&self) -> bool {
        match self {
            Expr::Num(_) => false,
            Expr::LVal(_) => false,
            Expr::Pos(sub) => sub.has_side_effect(),
            Expr::Neg(sub) => sub.has_side_effect(),
            Expr::Not(sub) => sub.has_side_effect(),
            Expr::Add(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Sub(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Mul(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Div(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Mod(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Lt(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Gt(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Le(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Ge(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Eq(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Ne(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Land(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
            Expr::Lor(lhs, rhs) => lhs.has_side_effect() || rhs.has_side_effect(),
        }    
    }
    
    pub fn try_const_eval(&self, env: &IREnvironment) -> Result<i32, FrontendError> {
        match self {
            Expr::Num(num) => Ok(*num),
            Expr::LVal(lval) => {
                match env.lookup(lval) {
                    None => Err(BindingNonConstExpr(lval.ident().into())),
                    Some(entry) => {
                        match entry {
                            SymbolTableEntry::Const(_, num) => Ok(num),
                            SymbolTableEntry::Var(_) => Err(BindingNonConstExpr(lval.ident().into())),
                        }
                    }
                }
            },
            Expr::Pos(expr) => expr.try_const_eval(env),
            Expr::Neg(expr) => expr.try_const_eval(env).map(|val| -val),
            Expr::Not(expr) => expr.try_const_eval(env).map(|val| if val == 0 { 1 } else { 0 }),
            Expr::Add(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| lhs + rhs),
            Expr::Sub(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| lhs - rhs),
            Expr::Mul(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| lhs * rhs),
            Expr::Div(lhs, rhs) => {
                let lhs_val = lhs.try_const_eval(env)?;
                let rhs_val = rhs.try_const_eval(env)?;
                if rhs_val == 0 {
                    return Err(ConstEvalDivZero);
                }
                Ok(lhs_val / rhs_val)
            }
            Expr::Mod(lhs, rhs) => {
                let lhs_val = lhs.try_const_eval(env)?;
                let rhs_val = rhs.try_const_eval(env)?;
                if rhs_val == 0 {
                    return Err(ConstEvalDivZero);
                }
                Ok(lhs_val % rhs_val)
            }
            Expr::Lt(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs < rhs { 1 } else { 0 }),
            Expr::Gt(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs > rhs { 1 } else { 0 }),
            Expr::Le(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs <= rhs { 1 } else { 0 }),
            Expr::Ge(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs >= rhs { 1 } else { 0 }),
            Expr::Eq(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs == rhs { 1 } else { 0 }),
            Expr::Ne(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs != rhs { 1 } else { 0 }),
            Expr::Land(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs != 0 && rhs != 0 { 1 } else { 0 }),
            Expr::Lor(lhs, rhs) => binary_expr_eval_rule!(env, lhs, rhs, |lhs, rhs| if lhs != 0 || rhs != 0 { 1 } else { 0 }),
        }
    }
}