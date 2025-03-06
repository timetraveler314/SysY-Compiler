use std::cell::RefCell;
use std::rc::Rc;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, Value};
use koopa::ir::builder::BasicBlockBuilder;
use crate::frontend::ast::LVal;
use crate::frontend::FrontendError;
use crate::frontend::symbol::{NestedSymbolTable, SymbolTableEntry};
use crate::util::name_generator::NameGenerator;

#[macro_export]
macro_rules! get_func_from_ir_env {
    ($env:expr) => {
        $env.context.program.func($env.context.current_func.unwrap())
    };
}

#[macro_export]
macro_rules! local_value_builder {
    ($env:expr) => {
        $env.context.program.borrow_mut().func_mut($env.context.current_func.unwrap()).dfg_mut().new_value()
    };
}

#[macro_export]
macro_rules! global_value_builder {
    ($env:expr) => {
        $env.context.program.borrow_mut().new_value()
    };
}

pub struct IRContext {
    pub program: Rc<RefCell<Program>>,
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

    pub fn generate_decl(&mut self, name: &str, params_ty: Vec<Type>, ret_ty: Type) -> Result<(), FrontendError> {
        let function = self.context.program.borrow_mut().new_func(FunctionData::new_decl(name.to_string(), params_ty.clone(), ret_ty.clone()));
        // Add to symbol table
        self.bind(&*name[1..].to_string(), SymbolTableEntry::Func {
            handle: function,
            params: params_ty.iter().zip(0..).map(|(ty, i)| (format!("_arg{}", i), ty.clone())).collect(),
            ret_type: ret_ty
        })?;
        Ok(())
    }

    pub fn is_global(&self) -> bool {
        self.context.current_func.is_none() && self.context.current_bb.is_none()
    }
}