use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use koopa::ir::{BasicBlock, Function, Program};
use koopa::ir::entities::ValueData;
use crate::backend::asm::AsmBasicBlock;
use crate::backend::instruction::Instruction;
use crate::backend::register::{RVRegister, RVRegisterPool};
use crate::util::name_generator::NameGenerator;

#[derive(Debug, Clone)]
pub struct FunctionPrologueInfo {
    pub stack_size: i32,
}

#[derive(Debug, Clone)]
pub enum ValueStorage {
    Immediate(i32),
    Register(RVRegister),
    Stack(i32),
}

pub struct ROContext<'a> {
    pub program: &'a Program,
    pub current_func: Option<Function>,
    pub current_bb: Option<BasicBlock>,
}

pub struct AsmEnvironment<'a> {
    pub context: ROContext<'a>,
    // Map from Value to its result register
    pub presence_table: std::collections::HashMap<*const ValueData, ValueStorage>,
    pub function_prologue_info: FunctionPrologueInfo,
    pub(crate) register_pool: RVRegisterPool,
    pub(crate) name_generator: Rc<RefCell<NameGenerator>>,
    pub(crate) name_map: HashMap<BasicBlock, String>,
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
            function_prologue_info: FunctionPrologueInfo { stack_size: 0 },
            register_pool: RVRegisterPool::new_temp_pool(),
            name_generator: Rc::new(RefCell::from(NameGenerator::new())),
            name_map: HashMap::new(),
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
                    target.add_instruction(Instruction::Lw {
                        rd: register.clone(),
                        rs: RVRegister::Sp,
                        imm: *offset,
                    });
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
            },
            None => panic!("Value {:?} not present in presence table", value),
        }
    }

    pub fn store_data(&mut self, target: &mut AsmBasicBlock, value: &ValueData, register: Option<RVRegister>) {
        match self.presence_table.get_mut(&(value as *const ValueData)) {
            Some(storage) => match storage {
                ValueStorage::Register(_reg_prev) => unimplemented!(),
                ValueStorage::Stack(offset) => {
                    // Store from register to stack
                    let register = register.unwrap();
                    target.add_instruction(Instruction::Sw {
                        rs: register,
                        rd: RVRegister::Sp,
                        imm: *offset,
                    });

                    // Free the register
                    self.register_pool.release(register);
                }
                ValueStorage::Immediate(_) => unimplemented!(),
            },
            None => panic!("Value not present in presence table"),
        }
    }

    pub fn bind_data_storage(&mut self, value: &ValueData, storage: ValueStorage) {
        self.presence_table.insert(value as *const ValueData, storage);
    }

    pub fn alloc_stack_storage(&mut self, value_data: &ValueData, size: i32) {
        // Save to the storage mapping
        self.presence_table.insert(value_data as *const ValueData, ValueStorage::Stack(self.function_prologue_info.stack_size));
        // Update the stack size
        self.function_prologue_info.stack_size += size;
    }

    pub fn apply_register(&mut self, value: &ValueData) -> RVRegister {
        println!("Applying register for {:?}", value);
        let register = self.register_pool.next().unwrap();
        register
    }

    pub fn free_register(&mut self, register: RVRegister) {
        println!("Freeing register {:?}", register);
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
}
