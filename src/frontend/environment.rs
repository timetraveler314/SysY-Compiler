use koopa::ir::builder::BasicBlockBuilder;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Value};
use koopa::ir::entities::ValueData;
use crate::backend::register::{RVRegister, RVRegisterIterator};

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

pub struct AsmEnvironment<'a> {
    pub context: ROContext<'a>,
    // Map from Value to its result register
    pub register_table: std::collections::HashMap<*const ValueData, RVRegister>,
}

impl<'a> AsmEnvironment<'a> {
    pub fn new(program: &'a Program) -> Self {
        AsmEnvironment {
            context: ROContext {
                program,
                current_func: None,
                current_bb: None,
                it: RVRegisterIterator::new(),
            },
            register_table: std::collections::HashMap::new(),
        }
    }

    pub fn apply_register(&mut self, value: &ValueData) -> RVRegister {
        println!("Applying register for {:?}", value);
        let register = self.context.it.next().unwrap();
        self.register_table.insert(value as *const ValueData, register);
        register
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

    // TODO: Improve this
    pub it: RVRegisterIterator
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
