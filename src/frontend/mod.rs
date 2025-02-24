use koopa::ir::Program;
use crate::frontend::ast::CompUnit;
use crate::frontend::generate_ir::IRGenerator;
use crate::frontend::environment::IRContext;

pub mod ast;
mod generate_ir;
pub(crate) mod environment;

#[derive(Debug)]
pub enum FrontendError {
    ParseError(String),
}

pub fn generate_ir(comp_unit: &CompUnit) -> Result<Program, FrontendError> {
    let mut program = Program::new();
    let mut ircontext = IRContext {
        program: &mut program,
        current_func: None,
        current_bb: None,
    };
    comp_unit.generate_ir(&mut ircontext)?;
    Ok(program)
}