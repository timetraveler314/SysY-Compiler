use koopa::ir::{BinaryOp, FunctionData, Type, Value};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use crate::frontend::ast::{Block, CompUnit, Expr, FuncDef, Stmt};
use crate::frontend::FrontendError;
use crate::common::environment::IRContext;

macro_rules! value_builder {
    ($ircontext:expr) => {
        $ircontext.func_data_mut().dfg_mut().new_value()
    };
}

pub trait IRGenerator {
    type Output;
    fn generate_ir(&self, ircontext: &mut IRContext) -> Result<Self::Output, FrontendError>;
}

impl IRGenerator for CompUnit {
    type Output = ();

    fn generate_ir(&self, ircontext: &mut IRContext) -> Result<Self::Output, FrontendError> {
        self.func_def.generate_ir(ircontext)?;
        Ok(())
    }
}

impl IRGenerator for FuncDef {
    type Output = ();

    fn generate_ir(&self, ircontext: &mut IRContext) -> Result<Self::Output, FrontendError> {
        // name -> @ + name
        let ir_func_name = format!("@{}", self.ident);
        let func_data = FunctionData::new(ir_func_name, Vec::new(), Type::get_i32());

        // Add the function to the program, and set the context's current function
        let func = ircontext.program.new_func(func_data);

        // Recursively generate IR for the block
        self.block.generate_ir(&mut IRContext {
            program: ircontext.program,
            current_func: Some(func),
            current_bb: None,
        })?;

        Ok(())
    }
}

impl IRGenerator for Block {
    type Output = ();

    fn generate_ir(&self, ircontext: &mut IRContext) -> Result<Self::Output, FrontendError> {
        ircontext.create_block(Some("%entry".into()));

        // Recursively generate IR for the statement, currently only one
        self.stmt.generate_ir(ircontext)?;

        // Exit the current block
        ircontext.current_bb = None;
        Ok(())
    }
}

impl IRGenerator for Stmt {
    type Output = ();

    fn generate_ir(&self, ircontext: &mut IRContext) -> Result<Self::Output, FrontendError> {
        // TODO: Currently only support `return` statement
        let return_val = self.expr.generate_ir(ircontext)?;
        let return_stmt = value_builder!(ircontext).ret(Some(return_val));
        ircontext.add_instruction(return_stmt);
        Ok(())
    }
}

macro_rules! generate_binary_expr {
    ($ircontext:expr, $lhs:expr, $rhs:expr, $op:ident) => {{
        let lhs_val = $lhs.generate_ir($ircontext)?;
        let rhs_val = $rhs.generate_ir($ircontext)?;
        let op = value_builder!($ircontext).binary(BinaryOp::$op, lhs_val, rhs_val);
        $ircontext.add_instruction(op);
        Ok(op)
    }};
}

impl IRGenerator for Expr {
    type Output = Value;

    fn generate_ir(&self, ircontext: &mut IRContext) -> Result<Self::Output, FrontendError> {
        match self {
            Expr::Num(num) => Ok(value_builder!(ircontext).integer(*num)),
            Expr::Pos(expr) => expr.generate_ir(ircontext),
            Expr::Neg(expr) => {
                let zero = value_builder!(ircontext).integer(0);
                let val = expr.generate_ir(ircontext)?;
                let op = value_builder!(ircontext).binary(BinaryOp::Sub, zero, val);
                ircontext.add_instruction(op);
                Ok(op)
            }
            Expr::Not(expr) => {
                let zero = value_builder!(ircontext).integer(0);
                let val = expr.generate_ir(ircontext)?;
                let op = value_builder!(ircontext).binary(BinaryOp::Eq, val, zero);
                ircontext.add_instruction(op);
                Ok(op)
            }
            // Binary operations
            Expr::Add(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Add),
            Expr::Sub(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Sub),
            Expr::Mul(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Mul),
            Expr::Div(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Div),
            Expr::Mod(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Mod),
            // Logical operations
            Expr::Lt(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Lt),
            Expr::Gt(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Gt),
            Expr::Le(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Le),
            Expr::Ge(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Ge),
            Expr::Eq(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, Eq),
            Expr::Ne(lhs, rhs) => generate_binary_expr!(ircontext, lhs, rhs, NotEq),
            Expr::Land(lhs, rhs) => {
                let lhs_val = lhs.generate_ir(ircontext)?;
                let rhs_val = rhs.generate_ir(ircontext)?;
                let zero = value_builder!(ircontext).integer(0);
                let lhs_neq_z = value_builder!(ircontext).binary(BinaryOp::NotEq, lhs_val, zero);
                let rhs_neq_z = value_builder!(ircontext).binary(BinaryOp::NotEq, rhs_val, zero);
                let op = value_builder!(ircontext).binary(BinaryOp::And, lhs_neq_z, rhs_neq_z);
                ircontext.add_instruction(lhs_neq_z);
                ircontext.add_instruction(rhs_neq_z);
                ircontext.add_instruction(op);
                Ok(op)
            }
            Expr::Lor(lhs, rhs) => {
                let lhs_val = lhs.generate_ir(ircontext)?;
                let rhs_val = rhs.generate_ir(ircontext)?;
                let zero = value_builder!(ircontext).integer(0);
                let op = value_builder!(ircontext).binary(BinaryOp::Or, lhs_val, rhs_val);
                let snez = value_builder!(ircontext).binary(BinaryOp::NotEq, op, zero);
                ircontext.add_instruction(op);
                ircontext.add_instruction(snez);
                Ok(snez)
            }
        }
    }
}