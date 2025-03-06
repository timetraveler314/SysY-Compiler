pub(crate) mod generate_asm;
pub(crate) mod asm;
pub(crate) mod register;
pub(crate) mod instruction;
pub(crate) mod environment;

pub enum BackendError {
    Unimplemented,
}