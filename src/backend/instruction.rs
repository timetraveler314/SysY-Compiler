use crate::backend::register::RVRegister;

#[derive(Debug)]
pub enum Instruction {
    Addi { rd: RVRegister, rs: RVRegister, imm: i32 },
    Li { rd: RVRegister, imm: i32 },
    Lw { rd: RVRegister, rs: RVRegister, imm: i32 },
    Sw { rs: RVRegister, rd: RVRegister, imm: i32 },
    Mv { rd: RVRegister, rs: RVRegister },
    Add { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Sub { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Mul { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Div { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Rem { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    And { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Or { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Xor { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Slt { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Sgt { rd: RVRegister, rs1: RVRegister, rs2: RVRegister },
    Seqz { rd: RVRegister, rs: RVRegister },
    Snez { rd: RVRegister, rs: RVRegister },
    // Branch instructions
    Bnez { rs: RVRegister, label: String },
    J { label: String },
    Call { label: String },
    Ret,
}

// Impl Write for Instruction
impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instruction::Addi { rd, rs, imm } => write!(f, "addi {}, {}, {}", rd, rs, imm),
            Instruction::Li { rd, imm } => write!(f, "li {}, {}", rd, imm),
            Instruction::Lw { rd, rs, imm } => write!(f, "lw {}, {}({})", rd, imm, rs),
            Instruction::Sw { rs, rd, imm } => write!(f, "sw {}, {}({})", rs, imm, rd),
            Instruction::Mv { rd, rs } => write!(f, "mv {}, {}", rd, rs),
            Instruction::Add { rd, rs1, rs2 } => write!(f, "add {}, {}, {}", rd, rs1, rs2),
            Instruction::Sub { rd, rs1, rs2 } => write!(f, "sub {}, {}, {}", rd, rs1, rs2),
            Instruction::Mul { rd, rs1, rs2 } => write!(f, "mul {}, {}, {}", rd, rs1, rs2),
            Instruction::Div { rd, rs1, rs2 } => write!(f, "div {}, {}, {}", rd, rs1, rs2),
            Instruction::Rem { rd, rs1, rs2 } => write!(f, "rem {}, {}, {}", rd, rs1, rs2),
            Instruction::And { rd, rs1, rs2 } => write!(f, "and {}, {}, {}", rd, rs1, rs2),
            Instruction::Or { rd, rs1, rs2 } => write!(f, "or {}, {}, {}", rd, rs1, rs2),
            Instruction::Xor { rd, rs1, rs2 } => write!(f, "xor {}, {}, {}", rd, rs1, rs2),
            Instruction::Slt { rd, rs1, rs2 } => write!(f, "slt {}, {}, {}", rd, rs1, rs2),
            Instruction::Sgt { rd, rs1, rs2 } => write!(f, "sgt {}, {}, {}", rd, rs1, rs2),
            Instruction::Seqz { rd, rs } => write!(f, "seqz {}, {}", rd, rs),
            Instruction::Snez { rd, rs } => write!(f, "snez {}, {}", rd, rs),
            Instruction::Bnez { rs, label } => write!(f, "bnez {}, {}", rs, label),
            Instruction::J { label } => write!(f, "j {}", label),
            Instruction::Call { label } => write!(f, "call {}", label),
            Instruction::Ret => write!(f, "ret"),
        }
    }
}