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
    pub(crate) content: Vec<AsmFunction>,
}

#[derive(Debug)]
pub struct AsmFunction {
    pub(crate) label: String,
    pub(crate) basic_blocks: Vec<AsmBasicBlock>,
    pub(crate) prologue: Vec<Instruction>,
    pub(crate) epilogue: Vec<Instruction>,
}

#[derive(Debug)]
pub struct AsmBasicBlock {
    pub(crate) label: Option<String>,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) is_entry: bool,
    pub(crate) is_exit: bool,
}

impl AsmFunction {
    pub fn new(label: &str) -> Self {
        AsmFunction {
            label: label.to_string(),
            basic_blocks: Vec::new(),
            prologue: Vec::new(),
            epilogue: Vec::new(),
        }
    }
}

impl AsmBasicBlock {
    pub fn new(label: &str) -> Self {
        AsmBasicBlock {
            label: Option::from(label.to_string()),
            instructions: Vec::new(),
            is_entry: false,
            is_exit: false,
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
                    // Traverse the functions
                    for func in &section.content {
                        for (i, bb) in func.basic_blocks.iter().enumerate() {
                            if let Some(label) = &bb.label {
                                writeln!(out, "{}:", label)?;
                            }

                            if bb.is_entry {
                                writeln!(out, "    # --- Prologue of {} ---", func.label)?;
                                for inst in &func.prologue {
                                    writeln!(out, "    {}", inst)?;
                                }
                                writeln!(out, "    # --- Prologue of {} ---", func.label)?;
                            }

                            for inst in &bb.instructions {
                                writeln!(out, "    {}", inst)?;
                            }

                            if bb.is_exit {
                                writeln!(out, "    # --- Epilogue of {} ---", func.label)?;
                                for inst in &func.epilogue {
                                    writeln!(out, "    {}", inst)?;
                                }
                                writeln!(out, "    # --- Epilogue of {} ---", func.label)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}