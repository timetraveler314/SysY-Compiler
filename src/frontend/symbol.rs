use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc};
use koopa::ir::{Function, Type, Value};
use crate::frontend::FrontendError;

#[derive(Clone)]
pub enum SymbolTableEntry {
    Const(String, i32),
    Var(Value),
    Func { handle: Function, ret_type: Type, params: Vec<(String, Type)> },
}


type SymbolTable = Rc<RefCell<NestedSymbolTable>>;

pub struct NestedSymbolTable {
    entries: std::collections::HashMap<String, SymbolTableEntry>,
    parent: Option<Rc<RefCell<NestedSymbolTable>>>,
}

impl NestedSymbolTable {
    pub fn new() -> Self {
        NestedSymbolTable {
            entries: std::collections::HashMap::new(),
            parent: None,
        }
    }

    pub fn new_child(parent: Rc<RefCell<NestedSymbolTable>>) -> Self {
        NestedSymbolTable {
            entries: std::collections::HashMap::new(),
            parent: Some(parent)
        }
    }

    pub fn lookup(&self, ident: &str) -> Option<SymbolTableEntry> {
        match self.entries.get(ident) {
            Some(entry) => Some(entry.clone()),
            None => {
                match &self.parent {
                    Some(parent) => parent.borrow().lookup(ident),
                    None => None
                }
            }
        }
    }

    pub fn bind(&mut self, ident: &str, entry: SymbolTableEntry) -> Result<(), FrontendError> {
        if self.entries.contains_key(ident) {
            return Err(FrontendError::MultipleDefinitionsForIdentifier(ident.into()));
        }
        self.entries.insert(ident.into(), entry);
        Ok(())
    }
}