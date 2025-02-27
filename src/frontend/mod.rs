use std::cell::RefCell;
use std::rc::Rc;
use koopa::ir::Program;
use crate::frontend::ast::CompUnit;
use crate::frontend::generate_ir::IRGenerator;
use crate::common::environment::{IRContext, IREnvironment};

pub mod ast;
pub mod symbol;
mod generate_ir;

#[derive(Debug)]
pub enum FrontendError {
    // ParseError(String),
    MultipleDefinitionsForIdentifier(String),
    DefinitionNotFoundForIdentifier(String),
    BindingNonConstExpr(String),
    ConstEvalDivZero,
    InvalidAssignmentToConst,
}

pub fn generate_ir(comp_unit: &CompUnit) -> Result<Rc<RefCell<Program>>, FrontendError> {
    let mut program = Rc::from(RefCell::from(Program::new()));
    comp_unit.generate_ir(&mut IREnvironment::new(&program))?;
    Ok(program)
}