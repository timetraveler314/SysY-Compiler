use crate::backend::instruction::Instruction;
use crate::backend::register::RVRegister::A0;
use crate::frontend::ir_context::{ROContext};
use koopa::ir::{FunctionData, Program, ValueKind};
use koopa::ir::entities::ValueData;
use crate::get_func_from_context;

pub trait GenerateAsm {
    type Target;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, ircontext: &mut ROContext<'b>);
}

impl GenerateAsm for Program {
    type Target = crate::backend::asm::AsmProgram;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, ircontext: &mut ROContext<'b>) {
        ircontext.program = self;
        let mut text_section = crate::backend::asm::AsmSection {
            section_type: crate::backend::asm::AsmSectionType::Text,
            label: "main".to_string(),
            content: Vec::new(),
        };

        // Traverse the functions
        for &func_h in self.func_layout() {
            self.func(func_h).generate(&mut text_section, &mut ROContext {
                current_func: Some(func_h),
                ..*ircontext
            });
        }

        target.sections.push(text_section);
    }
}

impl GenerateAsm for FunctionData {
    // Function will generate on sections, appending to the list of basic blocks
    type Target = crate::backend::asm::AsmSection;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, ircontext: &mut ROContext<'b>) {

        // Traverse the basic blocks and corresponding instructions
        for (&_bb_h, node) in self.layout().bbs() {
            let mut bb = crate::backend::asm::AsmBasicBlock {
                label: Some(self.name()[1..].to_string()),
                instructions: Vec::new(),
            };

            // Inside a basic block
            for &inst_h in node.insts().keys() {
                let value_data = self.dfg().value(inst_h);
                // Access the instruction
                value_data.generate(&mut bb, ircontext);
            }

            target.content.push(bb);
        }
    }
}

impl GenerateAsm for ValueData {
    type Target = crate::backend::asm::AsmBasicBlock;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, ircontext: &mut ROContext<'b>) {
        let func_data = get_func_from_context!(ircontext);

        match self.kind() {
            ValueKind::Integer(int) => {
                target.instructions.push(Instruction::Li {
                    rd: A0,
                    imm: int.value(),
                });
            }
            ValueKind::Return(ret) => {
                let value_h = ret.value().expect("Return value not found");
                func_data.dfg().value(value_h).generate(target, ircontext);

                target.instructions.push(Instruction::Ret);
            }
            _ => unreachable!(),
        }
    }
}
