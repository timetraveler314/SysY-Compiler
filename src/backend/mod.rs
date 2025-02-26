pub(crate) mod generate_asm;
pub(crate) mod asm;
pub(crate) mod register;
pub(crate) mod instruction;

pub enum BackendError {
    Unimplemented,
}