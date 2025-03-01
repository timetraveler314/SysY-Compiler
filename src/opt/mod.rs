use koopa::ir::FunctionData;

pub mod dead_code_elimination;

#[derive(Debug)]
pub enum OptError {
    Unimplemented,
}

pub trait OptPassFunction {
    fn run_on(&mut self, func_data: &mut FunctionData) -> Result<(), OptError>;
}