use std::collections::HashMap;
use koopa::ir::{BinaryOp, FunctionData, Type, Value};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use crate::frontend::ast::{Block, BlockItem, CompUnit, ConstInitVal, Decl, Expr, FuncDef, Stmt};
use crate::frontend::FrontendError;
use crate::common::environment::{IRContext, IREnvironment, SymbolTableEntry};

macro_rules! value_builder {
    ($env:expr) => {
        $env.context.func_data_mut().dfg_mut().new_value()
    };
}

pub trait IRGenerator {
    type Output;
    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError>;
}

impl IRGenerator for CompUnit {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        self.func_def.generate_ir(env)?;
        Ok(())
    }
}

impl IRGenerator for FuncDef {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        // name -> @ + name
        let ir_func_name = format!("@{}", self.ident);
        let func_data = FunctionData::new(ir_func_name, Vec::new(), Type::get_i32());

        // Add the function to the program, and set the context's current function
        let func = env.context.program.new_func(func_data);

        // Recursively generate IR for the block
        self.block.generate_ir(&mut IREnvironment {
            context: IRContext {
                program: env.context.program,
                current_func: Some(func),
                current_bb: None,
            },
            symbol_table: HashMap::new(),
        })?;

        Ok(())
    }
}

impl IRGenerator for Block {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        env.context.create_block(Some("%entry".into()));

        // Recursively generate IR for the statement
        for block_item in self.items.iter() {
            block_item.generate_ir(env)?;
        }

        // Exit the current block
        env.context.current_bb = None;
        Ok(())
    }
}

impl IRGenerator for BlockItem {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        match self {
            BlockItem::Decl(decl) => decl.generate_ir(env),
            BlockItem::Stmt(stmt) => stmt.generate_ir(env),
        }
    }
}

impl IRGenerator for Decl {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        match self {
            Decl::ConstDecl(const_decl) => {
                // TODO: Now assuming BType int
                for const_def in const_decl.defs.iter() {
                    // Try to const-evaluate the expression
                    match &const_def.init_val {
                        ConstInitVal::Expr(expr) => {
                            let eval_result = expr.try_const_eval(env)?;

                            // Eval success, add the constant to the symbol table
                            env.symbol_table.insert(const_def.ident.clone(), SymbolTableEntry::Const(const_def.ident.clone(), eval_result));
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

impl IRGenerator for Stmt {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        // TODO: Currently only support `return` statement
        let return_val = self.expr.generate_ir(env)?;
        let return_stmt = value_builder!(env).ret(Some(return_val));
        env.context.add_instruction(return_stmt);
        Ok(())
    }
}

macro_rules! generate_binary_expr {
    ($env:expr, $lhs:expr, $rhs:expr, $op:ident) => {{
        let lhs_val = $lhs.generate_ir($env)?;
        let rhs_val = $rhs.generate_ir($env)?;
        let op = value_builder!($env).binary(BinaryOp::$op, lhs_val, rhs_val);
        $env.context.add_instruction(op);
        Ok(op)
    }};
}

impl IRGenerator for Expr {
    type Output = Value;

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        match self {
            Expr::Num(num) => Ok(value_builder!(env).integer(*num)),
            Expr::LVal(lval) => {
                match env.lookup(lval) {
                    None => Err(FrontendError::NoBindingForIdentifier(lval.ident.clone())),
                    Some(entry) => {
                        match entry {
                            SymbolTableEntry::Const(_, num) => Ok(value_builder!(env).integer(num)),
                        }
                    }
                }
            }
            Expr::Pos(expr) => expr.generate_ir(env),
            Expr::Neg(expr) => {
                let zero = value_builder!(env).integer(0);
                let val = expr.generate_ir(env)?;
                let op = value_builder!(env).binary(BinaryOp::Sub, zero, val);
                env.context.add_instruction(op);
                Ok(op)
            }
            Expr::Not(expr) => {
                let zero = value_builder!(env).integer(0);
                let val = expr.generate_ir(env)?;
                let op = value_builder!(env).binary(BinaryOp::Eq, val, zero);
                env.context.add_instruction(op);
                Ok(op)
            }
            // Binary operations
            Expr::Add(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Add),
            Expr::Sub(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Sub),
            Expr::Mul(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Mul),
            Expr::Div(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Div),
            Expr::Mod(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Mod),
            // Logical operations
            Expr::Lt(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Lt),
            Expr::Gt(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Gt),
            Expr::Le(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Le),
            Expr::Ge(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Ge),
            Expr::Eq(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, Eq),
            Expr::Ne(lhs, rhs) => generate_binary_expr!(env, lhs, rhs, NotEq),
            Expr::Land(lhs, rhs) => {
                let lhs_val = lhs.generate_ir(env)?;
                let rhs_val = rhs.generate_ir(env)?;
                let zero = value_builder!(env).integer(0);
                let lhs_neq_z = value_builder!(env).binary(BinaryOp::NotEq, lhs_val, zero);
                let rhs_neq_z = value_builder!(env).binary(BinaryOp::NotEq, rhs_val, zero);
                let op = value_builder!(env).binary(BinaryOp::And, lhs_neq_z, rhs_neq_z);
                env.context.add_instruction(lhs_neq_z);
                env.context.add_instruction(rhs_neq_z);
                env.context.add_instruction(op);
                Ok(op)
            }
            Expr::Lor(lhs, rhs) => {
                let lhs_val = lhs.generate_ir(env)?;
                let rhs_val = rhs.generate_ir(env)?;
                let zero = value_builder!(env).integer(0);
                let op = value_builder!(env).binary(BinaryOp::Or, lhs_val, rhs_val);
                let snez = value_builder!(env).binary(BinaryOp::NotEq, op, zero);
                env.context.add_instruction(op);
                env.context.add_instruction(snez);
                Ok(snez)
            }
        }
    }
}