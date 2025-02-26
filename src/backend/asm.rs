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
    pub(crate) prologue: Vec<Instruction>,
    pub(crate) epilogue: Vec<Instruction>,
}

impl AsmBasicBlock {
    pub fn new(label: &str) -> Self {
        AsmBasicBlock {
            label: Option::from(label.to_string()),
            instructions: Vec::new(),
            prologue: Vec::new(),
            epilogue: Vec::new(),
        }
    }

    pub fn add_instruction(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }
}

impl AsmProgram {
    // write to an output stream
    pub fn emit(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        for section in &self.sections {
            match section.section_type {
                AsmSectionType::Text => {
                    writeln!(out, "   .text")?;
                    writeln!(out, "   .globl {}", section.label)?;
                    for (i, bb) in section.content.iter().enumerate() {
                        if let Some(label) = &bb.label {
                            writeln!(out, "{}:", label)?;
                        }

                        writeln!(out, "    # --- Prologue Begin ---")?;
                        if i == 0 { // First basic block
                            for inst in &bb.prologue {
                                writeln!(out, "    {}", inst)?;
                            }
                        }
                        writeln!(out, "    # --- Prologue End ---")?;

                        for inst in &bb.instructions {
                            writeln!(out, "    {}", inst)?;
                        }

                        writeln!(out, "    # --- Epilogue Begin ---")?;
                        if i == 0 { // First basic block
                            for inst in &bb.epilogue {
                                writeln!(out, "    {}", inst)?;
                            }
                        }
                        writeln!(out, "    # --- Epilogue End ---")?;
                    }
                }
            }
        }
        Ok(())
    }
}