use std::cell::RefCell;
use std::rc::Rc;
use koopa::ir::Program;
use crate::frontend::ast::CompUnit;
use crate::frontend::environment::IREnvironment;
use crate::frontend::generate_ir::IRGenerator;

pub mod ast;
pub mod symbol;
mod generate_ir;
mod environment;

#[derive(Debug)]
pub enum FrontendError {
    // ParseError(String),
    MultipleDefinitionsForIdentifier(String),
    DefinitionNotFoundForIdentifier(String),
    BindingNonConstExpr(String),
    ConstEvalDivZero,
    InvalidAssignmentToConst,
    BreakOutsideOfLoop,
    ContinueOutsideOfLoop,
    InvalidFunctionCall,
}

pub fn generate_ir(comp_unit: &CompUnit) -> Result<Rc<RefCell<Program>>, FrontendError> {
    let mut program = Rc::from(RefCell::from(Program::new()));
    comp_unit.generate_ir(&mut IREnvironment::new(&program))?;
    Ok(program)
}