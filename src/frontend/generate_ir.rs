use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use koopa::ir::{BinaryOp, FunctionData, Type, Value};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use crate::frontend::ast::{Block, BlockItem, CompUnit, ConstInitVal, Decl, Expr, FuncDef, LVal, Stmt, VarDef};
use crate::frontend::FrontendError;
use crate::common::environment::{IRContext, IREnvironment};
use crate::frontend::symbol::{NestedSymbolTable, SymbolTableEntry};

macro_rules! value_builder {
    ($env:expr) => {
        $env.context.program.borrow_mut().func_mut($env.context.current_func.unwrap()).dfg_mut().new_value()
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
        let func = env.context.program.borrow_mut().new_func(func_data);

        // Recursively generate IR for the block

        let mut new_env = env.enter_func(func);
        // TODO: Currently only 1 bb, just mutate the env for the bb
        let entry_bb = new_env.context.create_block(Some("%entry".into()));
        new_env.enter_bb(entry_bb);
        self.block.generate_ir(&mut new_env)?;

        Ok(())
    }
}

impl IRGenerator for Block {
    type Output = ();

    fn generate_ir(&self, env: &mut IREnvironment) -> Result<Self::Output, FrontendError> {
        // Recursively generate IR for the statement
        for block_item in self.items.iter() {
            block_item.generate_ir(env)?;
        }

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
                            env.bind(&const_def.ident, SymbolTableEntry::Const(const_def.ident.clone(), eval_result))?;
                        }
                    }
                }
                Ok(())
            }
            Decl::VarDecl(var_decl) => {
                // TODO: Now assuming BType int
                for var_def in var_decl.defs.iter() {
                    match var_def {
                        VarDef::Ident(ident) => {
                            // Alloc for the variable
                            // TODO: Any way to assign a name to the value in the IR?
                            let var = value_builder!(env).alloc(Type::get_i32());
                            env.context.add_instruction(var);
                            env.bind(ident, SymbolTableEntry::Var(var))?;
                        }
                        VarDef::Init(ident, expr) => {
                            // Alloc for the variable
                            let var = value_builder!(env).alloc(Type::get_i32());
                            env.context.add_instruction(var);

                            // Assign the value
                            let val = expr.generate_ir(env)?;
                            let store = value_builder!(env).store(val, var);
                            env.context.add_instruction(store);

                            env.bind(ident, SymbolTableEntry::Var(var))?;
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
        match self {
            Stmt::Return(expr) => {
                let return_val = expr.generate_ir(env)?;
                let return_stmt = value_builder!(env).ret(Some(return_val));
                env.context.add_instruction(return_stmt);
                Ok(())
            }
            Stmt::Assign(lval, expr) => {
                match lval {
                    LVal::Ident(ident) => {
                        // Assign the value
                        let val = expr.generate_ir(env)?;
                        if let Some(entry) = env.lookup(lval) {
                            match entry {
                                SymbolTableEntry::Var(var) => {
                                    let store = value_builder!(env).store(val, var);
                                    env.context.add_instruction(store);
                                    Ok(())
                                }
                                _ => Err(FrontendError::InvalidAssignmentToConst)
                            }
                        } else {
                            Err(FrontendError::DefinitionNotFoundForIdentifier(ident.clone()))
                        }
                    }
                }
            }
            Stmt::Expr(expr) => {
                // TODO: validate the correctness here
                expr.generate_ir(env)?;
                Ok(())
            }
            Stmt::Empty => { Ok(()) }
            Stmt::Block(block) => {
                // Enter a new scope
                let mut new_env = env.enter_scope();
                let result = block.generate_ir(&mut new_env);

                result
            }
            Stmt::If(cond, then_stmt) => {
                let cond_val = cond.generate_ir(env)?;

                let group = env.name_generator.borrow_mut().generate_group(&["%then", "%merge"]);
                let then_bb = env.context.create_block(Some(group[0].clone()));
                let merge_bb = env.context.create_block(Some(group[1].clone()));

                let branch = value_builder!(env).branch(cond_val, then_bb, merge_bb);
                env.context.add_instruction(branch);

                // Generate IR for then block
                let mut then_env = env.switch_bb(then_bb);
                then_stmt.generate_ir(&mut then_env)?;
                let then_jump = value_builder!(then_env).jump(merge_bb);
                then_env.context.add_instruction(then_jump);

                // Enter the merge block
                env.enter_bb(merge_bb);

                Ok(())
            }
            Stmt::IfElse(cond, then_stmt, else_stmt) => {
                let cond_val = cond.generate_ir(env)?;

                let group = env.name_generator.borrow_mut().generate_group(&["%then", "%else", "%merge"]);
                let then_bb = env.context.create_block(Some(group[0].clone()));
                let else_bb = env.context.create_block(Some(group[1].clone()));
                let merge_bb = env.context.create_block(Some(group[2].clone()));

                let branch = value_builder!(env).branch(cond_val, then_bb, else_bb);
                env.context.add_instruction(branch);

                // Generate IR for then block
                let mut then_env = env.switch_bb(then_bb);
                then_stmt.generate_ir(&mut then_env)?;
                let then_jump = value_builder!(then_env).jump(merge_bb);
                then_env.context.add_instruction(then_jump);

                // Generate IR for else block
                let mut else_env = env.switch_bb(else_bb);
                else_stmt.generate_ir(&mut else_env)?;
                let else_jump = value_builder!(then_env).jump(merge_bb);
                else_env.context.add_instruction(else_jump);

                // Enter the merge block
                env.enter_bb(merge_bb);

                Ok(())
            }
        }
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
                    None => Err(FrontendError::DefinitionNotFoundForIdentifier(lval.ident().into())),
                    Some(entry) => {
                        match entry {
                            SymbolTableEntry::Const(_, num) => Ok(value_builder!(env).integer(num)),
                            SymbolTableEntry::Var(var) => {
                                let load = value_builder!(env).load(var);
                                env.context.add_instruction(load);
                                Ok(load)
                            }
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