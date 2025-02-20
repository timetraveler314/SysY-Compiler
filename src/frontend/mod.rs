use koopa::ir::Program;
use crate::frontend::ast::CompUnit;
use crate::frontend::generate_ir::IRGenerator;
use crate::frontend::ir_context::IRContext;

pub mod ast;
mod generate_ir;
mod ir_context;

#[derive(Debug)]
pub enum FrontendError {
    ParseError(String),
}

pub fn generate_ir(comp_unit: &CompUnit) -> Result<Program, FrontendError> {
    let mut ircontext = IRContext {
        program: Program::new(),
        current_func: None,
        current_bb: None,
    };
    comp_unit.generate_ir(&mut ircontext)?;
    Ok(ircontext.program)
}