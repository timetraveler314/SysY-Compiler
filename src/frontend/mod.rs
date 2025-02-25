use koopa::ir::Program;
use crate::frontend::ast::CompUnit;
use crate::frontend::generate_ir::IRGenerator;
use crate::common::environment::{IRContext, IREnvironment};

pub mod ast;
mod generate_ir;

#[derive(Debug)]
pub enum FrontendError {
    // ParseError(String),
    NoBindingForIdentifier(String),
    BindingNonConstExpr(String),
    ConstEvalDivZero,
}

pub fn generate_ir(comp_unit: &CompUnit) -> Result<Program, FrontendError> {
    let mut program = Program::new();
    let ircontext = IRContext {
        program: &mut program,
        current_func: None,
        current_bb: None,
    };
    comp_unit.generate_ir(&mut IREnvironment {
        context: ircontext,
        symbol_table: std::collections::HashMap::new(),
    })?;
    Ok(program)
}