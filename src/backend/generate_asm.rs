use crate::backend::instruction::Instruction;
use crate::backend::register::RVRegister::A0;
use crate::common::environment::{AsmEnvironment, FunctionPrologueInfo, ROContext, ValueStorage};
use koopa::ir::{BinaryOp, FunctionData, Program, ValueKind};
use koopa::ir::entities::ValueData;
use crate::backend::asm::AsmBasicBlock;
use crate::backend::register::{RVRegister, RVRegisterPool};
use crate::get_func_from_env;

pub trait GenerateAsm {
    type Target;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>);
}

pub trait ValueGenerateAsm {
    type Target;

    fn generate_value<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>);
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
                },
                presence_table: std::collections::HashMap::new(),
                function_prologue_info: crate::common::environment::FunctionPrologueInfo {
                    stack_size: 0,
                },
                register_pool: RVRegisterPool::new_temp_pool(),
            });
        }

        target.sections.push(text_section);
    }
}

impl GenerateAsm for FunctionData {
    // Function will generate on sections, appending to the list of basic blocks
    type Target = crate::backend::asm::AsmSection;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) {
        let mut prologue_info = FunctionPrologueInfo { stack_size: 0 };

        // Traverse the basic blocks and corresponding instructions
        for (&bb_h, node) in self.layout().bbs() {
            let mut bb = AsmBasicBlock::new(&self.name()[1..]);

            let mut new_env = AsmEnvironment {
                context: ROContext {
                    program: env.context.program,
                    current_func: env.context.current_func,
                    current_bb: Some(bb_h),
                },
                presence_table: env.presence_table.clone(),
                function_prologue_info: env.function_prologue_info.clone(),
                register_pool: env.register_pool.clone(),
            };

            // Inside a basic block
            for &inst_h in node.insts().keys() {
                let value_data = self.dfg().value(inst_h);
                // Access the instruction, updating environment to basic block level
                println!("Generating value: {:?}", value_data);
                value_data.generate_value(&mut bb, &mut new_env);

                prologue_info = new_env.function_prologue_info.clone();
            }

            let aligned_stack_size = prologue_info.stack_size + (16 - prologue_info.stack_size % 16);
            bb.prologue.extend(vec![
                Instruction::Addi {
                    rd: RVRegister::Sp,
                    rs: RVRegister::Sp,
                    imm: -aligned_stack_size,
                },
            ]);
            bb.epilogue.extend(vec![
                Instruction::Addi {
                    rd: RVRegister::Sp,
                    rs: RVRegister::Sp,
                    imm: aligned_stack_size,
                },
                Instruction::Ret,
            ]);

            target.content.push(bb);
        }
    }
}

impl ValueGenerateAsm for ValueData {
    type Target = AsmBasicBlock;

    fn generate_value<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) {
        if env.is_present(self) {
            println!("Value already present in presence table");
            return;
        }

        let func_data = get_func_from_env!(env);

        match self.kind() {
            ValueKind::Integer(int) => {
                env.bind_data_storage(&self, ValueStorage::Immediate(int.value()));
            }
            ValueKind::Return(ret) => {
                let value_h = ret.value().expect("Return value not found");

                func_data.dfg().value(value_h).generate_value(target, env);
                let rs = env.load_data(target, func_data.dfg().value(value_h));

                target.instructions.push(Instruction::Mv {
                    rd: A0,
                    rs
                });
            }
            ValueKind::Binary(bin) => {
                // HAS return, allocate stack space
                env.alloc_stack_storage(self, 4);

                func_data.dfg().value(bin.lhs()).generate_value(target, env);
                func_data.dfg().value(bin.rhs()).generate_value(target, env);

                let rs1 = env.load_data(target, func_data.dfg().value(bin.lhs()));
                let rs2 = env.load_data(target, func_data.dfg().value(bin.rhs()));

                let rd = env.apply_register(self);
                let instructions = match bin.op() {
                    BinaryOp::NotEq => {
                        vec![
                            Instruction::Xor { rd, rs1, rs2 },
                            Instruction::Snez { rd, rs: rd },
                        ]
                    }
                    BinaryOp::Eq => {
                        vec![
                            Instruction::Xor { rd, rs1, rs2 },
                            Instruction::Seqz { rd, rs: rd },
                        ]
                    }
                    BinaryOp::Gt => { vec![Instruction::Sgt { rd, rs1, rs2 }] }
                    BinaryOp::Lt => { vec![Instruction::Slt { rd, rs1, rs2 }] }
                    BinaryOp::Ge => { vec![Instruction::Slt { rd, rs1, rs2 }, Instruction::Seqz { rd, rs: rd }] }
                    BinaryOp::Le => { vec![Instruction::Sgt { rd, rs1, rs2 }, Instruction::Seqz { rd, rs: rd }] }
                    BinaryOp::Add => { vec![Instruction::Add { rd, rs1, rs2 }] }
                    BinaryOp::Sub => { vec![Instruction::Sub { rd, rs1, rs2 }] }
                    BinaryOp::Mul => { vec![Instruction::Mul { rd, rs1, rs2 }] }
                    BinaryOp::Div => { vec![Instruction::Div { rd, rs1, rs2 }] }
                    BinaryOp::Mod => { vec![Instruction::Rem { rd, rs1, rs2 }] }
                    BinaryOp::And => { vec![Instruction::And { rd, rs1, rs2 }] }
                    BinaryOp::Or => { vec![Instruction::Or { rd, rs1, rs2 }] }
                    // BinaryOp::Xor => {}
                    // BinaryOp::Shl => {}
                    // BinaryOp::Shr => {}
                    // BinaryOp::Sar => {}
                    _ => unreachable!()
                };

                target.instructions.extend(instructions);

                env.free_register(rs1);
                env.free_register(rs2);
                env.store_data(target, self, Some(rd));
            }
            ValueKind::Alloc(alloc) => {
                unimplemented!()
            }
            _ => unreachable!(),
        }
    }
}