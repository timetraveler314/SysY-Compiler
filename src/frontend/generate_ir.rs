use koopa::ir::{FunctionData, Type};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use crate::frontend::ast::{Block, CompUnit, FuncDef, Stmt};
use crate::frontend::FrontendError;
use crate::frontend::ir_context::IRContext;

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
        let func_data_mut = ircontext.func_data_mut();
        let return_val = func_data_mut.dfg_mut().new_value().integer(self.num);
        let return_stmt = func_data_mut.dfg_mut().new_value().ret(Some(return_val));
        ircontext.add_instruction(return_stmt);
        Ok(())
    }
}