use std::cmp::max;
use std::fmt::Pointer;
use crate::backend::instruction::Instruction;
use crate::backend::register::RVRegister::A0;
use crate::backend::environment::{AsmEnvironment, FunctionPrologueInfo, ROContext, ValueStorage};
use koopa::ir::{BinaryOp, FunctionData, Program, ValueKind};
use koopa::ir::entities::ValueData;
use crate::backend::asm::{AsmBasicBlock, AsmFunction, AsmGlobal, AsmVariable, AsmVariableInit};
use crate::backend::register::{RVRegister, RVRegisterPool};
use crate::get_func_from_ir_env;

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

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) {
        let mut data_section = crate::backend::asm::AsmSection {
            section_type: crate::backend::asm::AsmSectionType::Data,
            content: Vec::new(),
        };
        let mut text_section = crate::backend::asm::AsmSection {
            section_type: crate::backend::asm::AsmSectionType::Text,
            content: Vec::new(),
        };

        // Traverse the global variables
        for &global_h in self.inst_layout() {
            let global = self.borrow_value(global_h);
            match global.kind() {
                ValueKind::GlobalAlloc(alloc) => {
                    let name = &global.name().clone().unwrap()[1..];

                    // Add to presence table
                    env.presence_table.insert(&*global as *const ValueData, ValueStorage::Global(name.to_string()));

                    let initial_value_data = self.borrow_value(alloc.init());

                    let init = match initial_value_data.kind() {
                        ValueKind::Integer(int) => AsmVariableInit::Word(int.value()),
                        ValueKind::ZeroInit(_) => AsmVariableInit::Zero(initial_value_data.ty().size()),
                        _ => unreachable!(),
                    };

                    let asm_global = AsmGlobal::AsmVariable(
                        AsmVariable {
                            label: name.to_string(),
                            init,
                        }
                    );

                    data_section.content.push(asm_global);
                }
                _ => {}
            }
        }

        // Traverse the functions
        for &func_h in self.func_layout() {
            let func_data = self.func(func_h);
            if func_data.layout().entry_bb().is_none() {
                // SysY library functions, skip
                continue;
            }

            let mut asm_func = AsmFunction::new(&func_data.name()[1..]);
            func_data.generate(&mut asm_func, &mut AsmEnvironment {
                context: ROContext {
                    program: self,
                    current_func: Some(func_h),
                    current_bb: None,
                },
                presence_table: env.presence_table.clone(),
                function_prologue_info: FunctionPrologueInfo::new(),
                analysis_result: env.analysis_result.clone(),
                register_pool: RVRegisterPool::new_temp_pool(),
                name_map: std::collections::HashMap::new(),
                name_generator: env.name_generator.clone(),
                stack_frame_size: 0,
            });

            text_section.content.push(AsmGlobal::AsmFunction(asm_func));
        }

        target.sections.push(data_section);
        target.sections.push(text_section);
    }
}

impl GenerateAsm for FunctionData {
    // Function will generate on sections, appending to the list of basic blocks
    type Target = AsmFunction;

    fn generate<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) {
        let mut prologue_info = FunctionPrologueInfo::new();
        // Fill in prologue_info with analysis results
        let self_handle = env.context.current_func.unwrap();
        let call_graph = &env.analysis_result.call_graph.graph;
        if call_graph.contains_key(&self_handle) {
            let body = call_graph.get(&self_handle).unwrap();
            prologue_info.args_stack_size = max(0, body.max_args as i32 - 8) * 4;
            prologue_info.is_leaf = body.callee.is_empty();
        } else {
            prologue_info.args_stack_size = 0;
            prologue_info.is_leaf = true;
        }
        env.function_prologue_info = prologue_info.clone();

        // Estimate the stack frame size, save to the outside `prologue_info`
        let estimated_stack_size = env.context.program.func(self_handle).dfg().values().iter().fold(
            0usize, |stack_size, (&value_h, value_data)| {
                stack_size + match value_data.kind() {
                    ValueKind::FuncArgRef(_) => 0,
                    ValueKind::BlockArgRef(_) => unreachable!(),
                    ValueKind::Alloc(_) => 4,
                    ValueKind::GlobalAlloc(_) => unreachable!(),
                    ValueKind::Load(_) => 4,
                    ValueKind::GetPtr(_) => unreachable!(),
                    ValueKind::GetElemPtr(_) => unreachable!(),
                    ValueKind::Binary(_) => 4,
                    ValueKind::Jump(_) => 0,
                    ValueKind::Call(_) => 4,
                    ValueKind::Return(_) => 0,
                    _ => 0
                }
            }
        );
        prologue_info.stack_size = estimated_stack_size as i32;
        env.stack_frame_size = prologue_info.get_aligned_stack_size() as usize;

        // Traverse the basic blocks and corresponding instructions
        for (i, (&bb_h, node)) in self.layout().bbs().iter().enumerate() {
            let mut bb = AsmBasicBlock::new(env.lookup_name(&bb_h).as_str());

            // The entry basic block is the first one
            if i == 0 {
                bb.label = Some(self.name()[1..].to_string());
                bb.is_entry = true;
            }

            env.enter_bb(bb_h);

            // Inside a basic block
            for &inst_h in node.insts().keys() {
                let value_data = self.dfg().value(inst_h);
                // Access the instruction, updating environment to basic block level
                value_data.generate_value(&mut bb, env);
            }

            target.basic_blocks.push(bb);
        }

        let aligned_stack_size = prologue_info.get_aligned_stack_size();

        // Now we have the stack size that is calculated in two ways,
        // compare them to check whether the implementation is correct
        assert_eq!(prologue_info.stack_size, env.function_prologue_info.stack_size);
        assert_eq!(prologue_info.args_stack_size, env.function_prologue_info.args_stack_size);
        assert_eq!(prologue_info.is_leaf, env.function_prologue_info.is_leaf);

        // Prologue
        target.prologue.extend(env.generate_addi(RVRegister::Sp, RVRegister::Sp, -aligned_stack_size));
        // Save the `ra` register if applicable
        if !prologue_info.is_leaf {
            target.prologue.extend(env.generate_sw(RVRegister::Ra, RVRegister::Sp, prologue_info.stack_size + prologue_info.args_stack_size));
        }

        // Epilogue
        // Restore the `ra` register if applicable
        if !prologue_info.is_leaf {
            target.epilogue.extend(env.generate_lw(RVRegister::Ra, RVRegister::Sp, prologue_info.stack_size + prologue_info.args_stack_size));
        }
        target.epilogue.extend(env.generate_addi(RVRegister::Sp, RVRegister::Sp, aligned_stack_size));
        target.epilogue.push(Instruction::Ret);
    }
}

impl ValueGenerateAsm for ValueData {
    type Target = AsmBasicBlock;

    fn generate_value<'b, 'a: 'b>(&'a self, target: &mut Self::Target, env: &mut AsmEnvironment<'b>) {
        if env.is_present(self) {
            println!("Value already present in presence table");
            return;
        }

        let func_data = get_func_from_ir_env!(env);

        match self.kind() {
            ValueKind::Integer(int) => {
                env.bind_data_storage(&self, ValueStorage::Immediate(int.value()));
            }
            ValueKind::Return(ret) => {
                match ret.value() {
                    Some(value_h) => {
                        func_data.dfg().value(value_h).generate_value(target, env);
                        let rs = env.load_data(target, func_data.dfg().value(value_h));
                        target.instructions.push(Instruction::Mv {
                            rd: A0,
                            rs
                        });
                        env.free_register(rs);
                    }
                    None => {}
                }

                target.is_exit = true;
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
            ValueKind::Alloc(_) => {
                env.alloc_stack_storage(self, 4);
            }
            ValueKind::Load(load) => {
                // Trivially, load should write to another stack space
                // just as what we did in binary
                env.alloc_stack_storage(self, 4);

                let x = env.context.program.borrow_values();
                let from = x.get(&load.src()).unwrap_or_else(
                    || func_data.dfg().value(load.src())
                );
                // let from = func_data.dfg().value(load.src());
                let rs = env.load_data(target, &*from);
                env.store_data(target, self, Some(rs));
            }
            ValueKind::Store(store) => {
                let src_value_data = func_data.dfg().value(store.value());

                src_value_data.generate_value(target, env);

                let src = env.load_data(target, src_value_data);

                let x = env.context.program.borrow_values();
                let to = x.get(&store.dest()).unwrap_or_else(
                    || func_data.dfg().value(store.dest())
                );
                env.store_data(target, to, Some(src));
            }
            ValueKind::Branch(branch) => {
                let cond_value_data = func_data.dfg().value(branch.cond());
                cond_value_data.generate_value(target, env);

                let rs = env.load_data(target, cond_value_data);
                target.instructions.push(Instruction::Bnez {
                    rs,
                    label: env.lookup_name(&branch.true_bb()).to_string(),
                });
                target.instructions.push(Instruction::J {
                    label: env.lookup_name(&branch.false_bb()).to_string(),
                });

                env.free_register(rs);
            }
            ValueKind::Jump(jump) => {
                target.instructions.push(Instruction::J {
                    label: env.lookup_name(&jump.target()).to_string(),
                });
            }
            ValueKind::Call(call) => {
                // TODO: check preparation correctness
                // Prepare arguments
                for (i, &arg) in call.args().iter().enumerate() {
                    let arg_value_data = func_data.dfg().value(arg);
                    arg_value_data.generate_value(target, env);

                    if i < 8 {
                        let rs = env.load_data(target, arg_value_data);
                        target.instructions.push(Instruction::Mv {
                            rd: RVRegister::get_arg_reg(i),
                            rs,
                        });
                        env.free_register(rs);
                    } else {
                        let rs = env.load_data(target, arg_value_data);
                        target.instructions.push(Instruction::Sw {
                            rs,
                            rd: RVRegister::Sp,
                            imm: (i - 8) as i32 * 4,
                        });
                        env.free_register(rs);
                    }
                }

                // Call!
                let callee = env.context.program.func(call.callee()).name()[1..].to_string();
                target.instructions.push(Instruction::Call {
                    label: callee,
                });

                // Handle return by saving `a0`
                env.alloc_stack_storage(self, 4);
                env.store_data(target, self, Some(RVRegister::A0));
            }
            ValueKind::FuncArgRef(arg) => {
                let arg_index = arg.index() as i32;
                if arg_index < 8 {
                    env.bind_data_storage(&self, ValueStorage::Register(RVRegister::get_arg_reg(arg.index())));
                } else {
                    // Compensate for the current stack frame
                    let position = (arg.index() - 8) * 4 + env.stack_frame_size;
                    env.bind_data_storage(&self, ValueStorage::Stack(position as i32));
                }
            }
            _ => unreachable!(),
        }
    }
}