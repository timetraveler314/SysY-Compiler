use crate::backend::instruction::Instruction;

#[derive(Debug)]
pub struct AsmProgram {
    pub(crate) sections: Vec<AsmSection>,
}

#[derive(Debug)]
pub enum AsmSectionType {
    Text,
}

#[derive(Debug)]
pub struct AsmSection {
    pub(crate) section_type: AsmSectionType,
    pub(crate) label: String,
    pub(crate) content: Vec<AsmBasicBlock>,
}

#[derive(Debug)]
pub struct AsmBasicBlock {
    pub(crate) label: Option<String>,
    pub(crate) instructions: Vec<Instruction>,
}

impl AsmProgram {
    // write to an output stream
    pub fn emit(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        for section in &self.sections {
            match section.section_type {
                AsmSectionType::Text => {
                    writeln!(out, "   .text")?;
                    writeln!(out, "   .globl {}", section.label)?;
                    for bb in &section.content {
                        if let Some(label) = &bb.label {
                            writeln!(out, "{}:", label)?;
                        }
                        for inst in &bb.instructions {
                            writeln!(out, "    {}", inst)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}