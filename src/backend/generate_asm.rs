use crate::backend::instruction::Instruction;
use crate::backend::register::RVRegister::A0;
use crate::frontend::environment::{AsmEnvironment, ROContext};
use koopa::ir::{FunctionData, Program, Value, ValueKind};
use koopa::ir::entities::ValueData;
use crate::backend::asm::AsmBasicBlock;
use crate::backend::register::{RVRegister, RVRegisterIterator};
use crate::{get_func_from_context, get_func_from_env};

pub trait GenerateAsm {
    type Target;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>);
}

pub trait ValueGenerateAsm {
    type Target;

    fn generate_value<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) -> Option<RVRegister>;
}

impl GenerateAsm for Program {
    type Target = crate::backend::asm::AsmProgram;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, _env: &mut AsmEnvironment<'b>) {
        let mut text_section = crate::backend::asm::AsmSection {
            section_type: crate::backend::asm::AsmSectionType::Text,
            label: "main".to_string(),
            content: Vec::new(),
        };

        // Traverse the functions
        for &func_h in self.func_layout() {
            self.func(func_h).generate(&mut text_section, &mut AsmEnvironment {
                context: ROContext {
                    program: self,
                    current_func: Some(func_h),
                    current_bb: None,
                    it: RVRegisterIterator::new(),
                },
                register_table: std::collections::HashMap::new(),
            });
        }

        target.sections.push(text_section);
    }
}

impl GenerateAsm for FunctionData {
    // Function will generate on sections, appending to the list of basic blocks
    type Target = crate::backend::asm::AsmSection;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) {

        // Traverse the basic blocks and corresponding instructions
        for (&_bb_h, node) in self.layout().bbs() {
            let mut bb = AsmBasicBlock {
                label: Some(self.name()[1..].to_string()),
                instructions: Vec::new(),
            };

            // Inside a basic block
            for &inst_h in node.insts().keys() {
                let value_data = self.dfg().value(inst_h);
                // Access the instruction
                value_data.generate_value(&mut bb, env);
            }

            target.content.push(bb);
        }
    }
}

impl ValueGenerateAsm for ValueData {
    type Target = AsmBasicBlock;

    fn generate_value<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) -> Option<RVRegister> {
        let func_data = get_func_from_env!(env);

        if let Some(register) = env.register_table.get(&(self as *const ValueData)) {
            return Some(*register);
        }

        match self.kind() {
            ValueKind::Integer(int) => {
                let rd = env.apply_register(self);
                target.instructions.push(Instruction::Li {
                    rd,
                    imm: int.value(),
                });
                Some(rd)
            }
            ValueKind::Return(ret) => {
                let value_h = ret.value().expect("Return value not found");
                let rs = func_data.dfg().value(value_h).generate_value(target, env).unwrap();

                target.instructions.push(Instruction::Mv {
                    rd: A0,
                    rs
                });
                target.instructions.push(Instruction::Ret);
                None
            }
            _ => unreachable!(),
        }
    }
}