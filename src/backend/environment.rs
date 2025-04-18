use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use koopa::ir::{BasicBlock, Function, Program};
use koopa::ir::entities::ValueData;
use crate::backend::asm::AsmBasicBlock;
use crate::backend::call_graph::CallGraph;
use crate::backend::instruction::Instruction;
use crate::backend::register::{RVRegister, RVRegisterPool};
use crate::util::name_generator::NameGenerator;

#[derive(Debug, Clone)]
pub struct FunctionPrologueInfo {
    pub stack_size: i32,
    // Whether the function needs to save `ra`
    pub is_leaf: bool,
    pub args_stack_size: i32,
}

impl FunctionPrologueInfo {
    pub fn new() -> Self {
        FunctionPrologueInfo {
            stack_size: 0,
            is_leaf: false,
            args_stack_size: 0,
        }
    }

    pub fn get_aligned_stack_size(&self) -> i32 {
        let stack_size = self.stack_size + self.args_stack_size + (self.is_leaf as i32) * 4;
        // Align to 16 bytes
        let remainder = stack_size % 16;
        if remainder == 0 {
            stack_size
        } else {
            stack_size + 16 - remainder
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueStorage {
    Immediate(i32),
    Register(RVRegister),
    Stack(i32),
    Global(String),
}

pub struct ROContext<'a> {
    pub program: &'a Program,
    pub current_func: Option<Function>,
    pub current_bb: Option<BasicBlock>,
}

#[derive(Clone)]
pub struct IRAnalysisResult {
    pub call_graph: CallGraph
}

pub struct AsmEnvironment<'a> {
    pub context: ROContext<'a>,
    // Map from Value to its result register
    pub presence_table: HashMap<*const ValueData, ValueStorage>,
    pub function_prologue_info: FunctionPrologueInfo,
    pub analysis_result: IRAnalysisResult,
    pub(crate) register_pool: RVRegisterPool,
    pub(crate) name_generator: Rc<RefCell<NameGenerator>>,
    pub(crate) name_map: HashMap<BasicBlock, String>,
    pub(crate) stack_frame_size: usize,
}

impl<'a> AsmEnvironment<'a> {
    pub fn new(program: &'a Program) -> Self {
        AsmEnvironment {
            context: ROContext {
                program,
                current_func: None,
                current_bb: None,
            },
            presence_table: std::collections::HashMap::new(),
            function_prologue_info: FunctionPrologueInfo::new(),
            analysis_result: IRAnalysisResult {
                call_graph: CallGraph::build(program),
            },
            register_pool: RVRegisterPool::new_temp_pool(),
            name_generator: Rc::new(RefCell::from(NameGenerator::new())),
            name_map: HashMap::new(),
            stack_frame_size: 0,
        }
    }

    pub fn enter_bb(&mut self, bb: BasicBlock) {
        self.context.current_bb = Some(bb);
    }

    pub fn is_present(&self, value: &ValueData) -> bool {
        self.presence_table.contains_key(&(value as *const ValueData))
    }

    pub fn load_data(&mut self, target: &mut AsmBasicBlock, value: &ValueData) -> RVRegister {
        match self.presence_table.get(&(value as *const ValueData)) {
            Some(storage) => match storage {
                ValueStorage::Register(register) => register.clone(),
                ValueStorage::Stack(offset) => {
                    // Try to apply a register
                    let register = self.register_pool.next().unwrap();
                    // Load from stack to register
                    target.instructions.extend(self.generate_lw(register.clone(), RVRegister::Sp, *offset));
                    register
                }
                ValueStorage::Immediate(imm) => {
                    if *imm == 0 {
                        RVRegister::Zero
                    } else {
                        let register = self.register_pool.next().unwrap();
                        target.add_instruction(Instruction::Li {
                            rd: register.clone(),
                            imm: *imm,
                        });
                        register
                    }
                }
                ValueStorage::Global(ident) => {
                    let global_addr_register = self.register_pool.next().unwrap();
                    let register = self.register_pool.next().unwrap();
                    target.add_instruction(Instruction::La {
                        rd: global_addr_register.clone(),
                        label: ident.clone(),
                    });
                    target.add_instruction(Instruction::Lw {
                        rd: register.clone(),
                        rs: global_addr_register.clone(),
                        imm: 0,
                    });
                    // Free the global address register
                    self.register_pool.release(global_addr_register);
                    register
                }
            },
            None => panic!("Value {:?} not present in presence table", value),
        }
    }

    pub fn store_data(&mut self, target: &mut AsmBasicBlock, value: &ValueData, register: Option<RVRegister>) {
        match self.presence_table.get(&(value as *const ValueData)) {
            Some(storage) => match storage {
                ValueStorage::Register(_reg_prev) => unimplemented!(),
                ValueStorage::Stack(_offset) => {
                    // Store from register to stack
                    let register = register.unwrap();
                    let offset = *_offset;
                    target.instructions.extend(self.generate_sw(register.clone(), RVRegister::Sp, offset));

                    // Free the register
                    self.register_pool.release(register);
                }
                ValueStorage::Immediate(_) => unimplemented!(),
                ValueStorage::Global(label) => {
                    let global_addr_register = self.register_pool.next().unwrap();
                    target.add_instruction(Instruction::La {
                        rd: global_addr_register.clone(),
                        label: label.clone(),
                    });
                    let register = register.unwrap();
                    target.add_instruction(Instruction::Sw {
                        rs: register,
                        rd: global_addr_register.clone(),
                        imm: 0,
                    });
                    // Free the global address register
                    self.register_pool.release(global_addr_register);
                    // Free the register
                    self.register_pool.release(register);
                }
            },
            None => panic!("Value not present in presence table"),
        }
    }

    pub fn bind_data_storage(&mut self, value: &ValueData, storage: ValueStorage) {
        self.presence_table.insert(value as *const ValueData, storage);
    }

    pub fn alloc_stack_storage(&mut self, value_data: &ValueData, size: i32) {
        // Save to the storage mapping
        let position = self.function_prologue_info.stack_size + self.function_prologue_info.args_stack_size;
        self.presence_table.insert(value_data as *const ValueData, ValueStorage::Stack(position));
        // Update the stack size
        self.function_prologue_info.stack_size += size;
    }

    pub fn apply_register(&mut self, _value: &ValueData) -> RVRegister {
        // println!("Applying register for {:?}", value);
        let register = self.register_pool.next().unwrap();
        register
    }

    pub fn free_register(&mut self, register: RVRegister) {
        // println!("Freeing register {:?}", register);
        self.register_pool.release(register);
    }

    pub fn lookup_name(&mut self, bb: &BasicBlock) -> String {
        match self.name_map.get(bb) {
            Some(name) => name.clone(),
            None => {
                // Generate a new name
                let name = self.name_generator.borrow_mut().generate("func_");
                self.bind_name(bb, name.clone());
                name
            }
        }
    }

    pub fn bind_name(&mut self, bb: &BasicBlock, name: String) {
        self.name_map.insert(bb.clone(), name);
    }

    pub fn generate_sw(&mut self, rs: RVRegister, rd: RVRegister, imm: i32) -> Vec<Instruction> {
        // Immediate is always 12-bit, meaning we need to check if it fits in 12-bit
        if imm >= -(1 << 11) && imm < (1 << 11) {
            vec![ Instruction::Sw { rs, rd, imm } ]
        } else {
            // If it doesn't fit, we need to use a temporary register to store the immediate
            let temp = self.register_pool.next().unwrap();
            let instructions = vec![
                Instruction::Li { rd: temp.clone(), imm },
                Instruction::Add { rd: temp.clone(), rs1: temp.clone(), rs2: rd },
                Instruction::Sw { rs, rd: temp.clone(), imm: 0 },
            ];
            self.free_register(temp);
            instructions
        }
    }

    pub fn generate_lw(&mut self, rd: RVRegister, rs: RVRegister, imm: i32) -> Vec<Instruction> {
        // Immediate is always 12-bit, meaning we need to check if it fits in 12-bit
        if imm >= -(1 << 11) && imm < (1 << 11) {
            vec![ Instruction::Lw { rd, rs, imm } ]
        } else {
            // If it doesn't fit, we need to use a temporary register to store the immediate
            let temp = self.register_pool.next().unwrap();
            let instructions = vec![
                Instruction::Li { rd: temp.clone(), imm },
                Instruction::Add { rd: temp.clone(), rs1: temp.clone(), rs2: rs },
                Instruction::Lw { rd, rs: temp.clone(), imm: 0 },
            ];
            self.free_register(temp);
            instructions
        }
    }

    pub fn generate_addi(&mut self, rd: RVRegister, rs: RVRegister, imm: i32) -> Vec<Instruction> {
        // Immediate is always 12-bit, meaning we need to check if it fits in 12-bit
        if imm >= -(1 << 11) && imm < (1 << 11) {
            vec![ Instruction::Addi { rd, rs, imm } ]
        } else {
            // If it doesn't fit, we need to use a temporary register to store the immediate
            let temp = self.register_pool.next().unwrap();
            let instructions = vec![
                Instruction::Li { rd: temp.clone(), imm },
                Instruction::Add { rd, rs1: rs, rs2: temp.clone() },
            ];
            self.free_register(temp);
            instructions
        }
    }
}
