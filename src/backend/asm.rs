use crate::backend::instruction::Instruction;
use std::io::Write;

#[derive(Debug)]
pub struct AsmProgram {
    pub(crate) sections: Vec<AsmSection>,
}

#[derive(Debug)]
pub enum AsmSectionType {
    Text,
    Data,
}

#[derive(Debug)]
pub struct AsmSection {
    pub(crate) section_type: AsmSectionType,
    // pub(crate) label: String,
    pub(crate) content: Vec<AsmGlobal>,
}

#[derive(Debug)]
pub enum AsmGlobal {
    AsmVariable(AsmVariable),
    AsmFunction(AsmFunction),
}

impl AsmGlobal {
    pub fn label(&self) -> &str {
        match self {
            AsmGlobal::AsmVariable(v) => &v.label,
            AsmGlobal::AsmFunction(f) => &f.label,
        }
    }
}

#[derive(Debug)]
pub struct AsmVariable {
    pub(crate) label: String,
    pub(crate) init: AsmVariableInit,
}

#[derive(Debug)]
pub enum AsmVariableInit {
    Word(i32),
    Zero(usize),
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

pub trait AsmEmitter {
    fn emit(&self, out: &mut impl std::io::Write) -> std::io::Result<()>;
}

impl AsmEmitter for AsmProgram {
    // write to an output stream
    fn emit(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        for section in &self.sections {
            // Write the section

            section.emit(out)?;
        }
        Ok(())
    }
}

impl AsmEmitter for AsmSection {
    fn emit(&self, out: &mut impl Write) -> std::io::Result<()> {
        // Header
        match self.section_type {
            AsmSectionType::Text => {
                writeln!(out, "   .text")?;
            }
            AsmSectionType::Data => {
                writeln!(out, "   .data")?;
            }
        }

        // Globals
        for global in &self.content {
            writeln!(out, "   .globl {}", global.label())?;
        }

        // Body
        for global in &self.content {
            global.emit(out)?;
        }

        writeln!(out, "")?;

        Ok(())
    }
}

impl AsmEmitter for AsmGlobal {
    fn emit(&self, out: &mut impl Write) -> std::io::Result<()> {
        match self {
            AsmGlobal::AsmVariable(var) => {
                writeln!(out, "{}:", var.label)?;
                match &var.init {
                    AsmVariableInit::Word(value) => {
                        writeln!(out, "   .word {}", value)?;
                    }
                    AsmVariableInit::Zero(size) => {
                        writeln!(out, "   .zero {}", size)?;
                    }
                }
            }
            AsmGlobal::AsmFunction(func) => {
                for bb in func.basic_blocks.iter() {
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

        Ok(())
    }
}
