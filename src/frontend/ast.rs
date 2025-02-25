use crate::common::environment::{IREnvironment, SymbolTableEntry};
use crate::frontend::FrontendError;
use crate::frontend::FrontendError::{BindingNonConstExpr, ConstEvalDivZero};

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

// Only support `return` statement right now
#[derive(Debug)]
pub struct Stmt {
    pub expr: Expr,
}

#[derive(Debug)]
pub struct LVal {
    pub ident: String,
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
    pub fn try_const_eval(&self, env: &IREnvironment) -> Result<i32, FrontendError> {
        match self {
            Expr::Num(num) => Ok(*num),
            Expr::LVal(lval) => {
                match env.lookup(lval) {
                    None => Err(BindingNonConstExpr(lval.ident.clone())),
                    Some(entry) => {
                        match entry {
                            SymbolTableEntry::Const(_, num) => Ok(num),
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