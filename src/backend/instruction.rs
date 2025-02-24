use crate::backend::register::RVRegister;

#[derive(Debug)]
pub enum Instruction {
    Li { rd: RVRegister, imm: i32 },
    Ret,
}

// Impl Write for Instruction
impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instruction::Li { rd, imm } => write!(f, "li {}, {}", rd, imm),
            Instruction::Ret => write!(f, "ret"),
        }
    }
}