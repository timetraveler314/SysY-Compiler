use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use koopa::ir::builder::BasicBlockBuilder;
use koopa::ir::{BasicBlock, Function, Program, Value};
use koopa::ir::entities::ValueData;
use crate::backend::asm::AsmBasicBlock;
use crate::backend::instruction::Instruction;
use crate::backend::register::{RVRegister, RVRegisterPool};
use crate::frontend::ast::{LVal};
use crate::frontend::FrontendError;
use crate::frontend::symbol::{NestedSymbolTable, SymbolTableEntry};
use crate::util::name_generator::NameGenerator;

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

pub struct IREnvironment {
    pub context: IRContext,
    pub name_generator: Rc<RefCell<NameGenerator>>,
    pub while_stack: Vec<(BasicBlock, BasicBlock)>,
    symbol_table: Rc<RefCell<NestedSymbolTable>>,
}

impl IREnvironment {
    pub fn new(program: &Rc<RefCell<Program>>) -> Self {
        IREnvironment {
            context: IRContext {
                program: program.clone(),
                current_func: None,
                current_bb: None,
            },
            name_generator: Rc::new(RefCell::from(NameGenerator::new())),
            while_stack: Vec::new(),
            symbol_table: Rc::new(RefCell::new(NestedSymbolTable::new())),
        }
    }

    pub fn enter_func(&self, func: Function) -> Self {
        IREnvironment {
            context: IRContext {
                program: self.context.program.clone(),
                current_func: Some(func),
                current_bb: None,
            },
            name_generator: self.name_generator.clone(),
            while_stack: Vec::new(),
            // A new symbol table as a child of the current symbol table
            symbol_table: Rc::new(RefCell::new(NestedSymbolTable::new_child(self.symbol_table.clone()))),
        }
    }

    pub fn switch_bb(&self, bb: BasicBlock) -> Self {
        assert!(self.context.current_func.is_some());

        IREnvironment {
            context: IRContext {
                program: self.context.program.clone(),
                current_func: self.context.current_func,
                current_bb: Some(bb),
            },
            name_generator: self.name_generator.clone(),
            while_stack: self.while_stack.clone(),
            symbol_table: self.symbol_table.clone(),
        }
    }

    pub fn enter_bb(&mut self, bb: BasicBlock) {
        self.context.current_bb = Some(bb);
    }

    pub fn enter_scope(&self) -> Self {
        IREnvironment {
            context: IRContext {
                program: self.context.program.clone(),
                current_func: self.context.current_func,
                current_bb: self.context.current_bb,
            },
            name_generator: self.name_generator.clone(),
            while_stack: self.while_stack.clone(),
            symbol_table: Rc::new(RefCell::new(NestedSymbolTable::new_child(self.symbol_table.clone()))),
        }
    }

    pub fn lookup_lval(&self, lval: &LVal) -> Option<SymbolTableEntry> {
        self.lookup_ident(lval.ident())
    }

    pub fn lookup_ident(&self, ident: &str) -> Option<SymbolTableEntry> {
        self.symbol_table.borrow().lookup(ident)
    }

    pub fn bind(&mut self, ident: &str, entry: SymbolTableEntry) -> Result<(), FrontendError> {
        self.symbol_table.borrow_mut().bind(ident, entry)
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

pub struct IRContext {
    pub program: Rc<RefCell<Program>>,
    pub current_func: Option<Function>,
    pub current_bb: Option<BasicBlock>,
}

pub struct ROContext<'a> {
    pub program: &'a Program,
    pub current_func: Option<Function>,
    pub current_bb: Option<BasicBlock>,
}

impl IRContext {
    pub fn create_block(&mut self, name: Option<String>) -> BasicBlock {
        let mut binding = self.program.borrow_mut();
        let func_data = binding.func_mut(self.current_func.unwrap());
        let bb = func_data.dfg_mut().new_bb().basic_block(name);
        // Add to the function's list of basic blocks
        func_data.layout_mut().bbs_mut().push_key_back(bb).unwrap();
        // Do not set the current block in Context
        bb
    }

    // This is created to avoid borrowing issues of disjoint fields in IRContext
    pub fn add_instruction(&mut self, inst: Value) {
        self.program.borrow_mut()
            .func_mut(self.current_func.unwrap())
            .layout_mut()
            .bb_mut(self.current_bb.unwrap().clone())
            .insts_mut()
            .push_key_back(inst)
            .unwrap();
    }
}
