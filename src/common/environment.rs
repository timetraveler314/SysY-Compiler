use koopa::ir::builder::BasicBlockBuilder;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Value};
use koopa::ir::entities::ValueData;
use crate::backend::asm::AsmBasicBlock;
use crate::backend::instruction::Instruction;
use crate::backend::register::{RVRegister, RVRegisterPool};
use crate::frontend::ast::{LVal};
use crate::frontend::FrontendError;
use crate::frontend::symbol::SymbolTableEntry;

#[macro_export]
macro_rules! get_func_from_context {
    ($context:expr) => {
        $context.program.func($context.current_func.unwrap())
    };
}

#[macro_export]
macro_rules! get_func_from_env {
    ($env:expr) => {
        $env.context.program.func($env.context.current_func.unwrap())
    };
}

pub struct IREnvironment<'a> {
    pub context: IRContext<'a>,
    pub symbol_table: std::collections::HashMap<String, SymbolTableEntry>,
}

impl<'a> IREnvironment<'a> {
    pub fn lookup(&self, lval: &LVal) -> Option<SymbolTableEntry> {
        self.symbol_table.get(lval.ident().into()).cloned()
    }

    pub fn bind(&mut self, ident: &str, entry: SymbolTableEntry) -> Result<(), FrontendError> {
        if self.symbol_table.contains_key(ident) {
            return Err(FrontendError::MultipleDefinitionsForIdentifier(ident.into()));
        }
        self.symbol_table.insert(ident.into(), entry);
        Ok(())
    }
}

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

pub struct AsmEnvironment<'a> {
    pub context: ROContext<'a>,
    // Map from Value to its result register
    pub presence_table: std::collections::HashMap<*const ValueData, ValueStorage>,
    pub function_prologue_info: FunctionPrologueInfo,
    pub(crate) register_pool: RVRegisterPool,
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
            register_pool: RVRegisterPool::new_temp_pool()
        }
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
                ValueStorage::Register(reg_prev) => unimplemented!(),
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
}

pub struct IRContext<'a> {
    pub program: &'a mut Program,
    pub current_func: Option<Function>,
    pub current_bb: Option<BasicBlock>,
}

pub struct ROContext<'a> {
    pub program: &'a Program,
    pub current_func: Option<Function>,
    pub current_bb: Option<BasicBlock>,
}

impl<'a> IRContext<'a> {
    pub fn func_data_mut(&mut self) -> &mut FunctionData {
        self.program.func_mut(self.current_func.unwrap())
    }

    pub fn create_block(&mut self, name: Option<String>) {
        let func_data = self.func_data_mut();
        let bb = func_data.dfg_mut().new_bb().basic_block(name);
        // Add to the function's list of basic blocks
        func_data.layout_mut().bbs_mut().push_key_back(bb).unwrap();
        // Set the current block in Context
        self.current_bb = Some(bb);
    }

    // This is created to avoid borrowing issues of disjoint fields in IRContext
    pub fn add_instruction(&mut self, inst: Value) {
        self.program
            .func_mut(self.current_func.unwrap())
            .layout_mut()
            .bb_mut(self.current_bb.unwrap().clone())
            .insts_mut()
            .push_key_back(inst)
            .unwrap();
    }
}
