use std::collections::{HashSet, VecDeque};
use koopa::ir::{FunctionData, Value, ValueKind};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use koopa::ir::entities::ValueData;
use crate::opt::{OptError, OptPassFunction};

pub struct DeadCodeEliminationPass {
    terminators: HashSet<Value>,
}

impl OptPassFunction for DeadCodeEliminationPass {
    fn run_on(&mut self, func_data: &mut FunctionData) -> Result<(), OptError> {
        self.mark(func_data);
        self.sweep(func_data);
        Ok(())
    }
}

impl DeadCodeEliminationPass {
    pub fn new() -> Self {
        DeadCodeEliminationPass {
            terminators: HashSet::new(),
        }
    }

    fn mark(&mut self, func_data: &mut FunctionData) {
        for (value_h, value) in func_data.dfg().values() {
            if Self::is_terminator(value) {
                self.terminators.insert(*value_h);
            }
        }
    }

    fn sweep(&mut self, func_data: &mut FunctionData) {
        let mut worklist = VecDeque::new();
        let mut bb_worklist = Vec::new();

        let mut bb_cursor = func_data.layout_mut().bbs_mut().cursor_front_mut();
        while let Some(bb) = bb_cursor.node_mut() {
            let mut inst_cursor = bb.insts_mut().cursor_front_mut();
            'inst: while let Some(inst) = inst_cursor.key() {
                if self.terminators.contains(inst) {
                    // Remove all the following instructions
                    inst_cursor.move_next();
                    while let Some((inst, _)) = inst_cursor.remove_current() {
                        worklist.push_back(inst);
                    }

                    // Check if the basic block is empty
                    drop(inst_cursor);

                    break 'inst;
                }

                inst_cursor.move_next();
            }

            if !bb.insts().back_key().is_some_and(|inst| self.terminators.contains(inst)) {
                // The basic block is not terminated by a terminator instruction
                // They are pushed into a worklist to avoid Rust's borrowing mechanism
                // Finally, we follow the C++ rule:
                // "if control reaches the end of the main function, return 0; is executed."
                bb_worklist.push(bb_cursor.key().unwrap().clone());
            }

            bb_cursor.move_next();
        }

        // Remove all the instructions in the worklist, iteratively
        while let Some(inst) = worklist.pop_front() {
            if func_data.dfg().value(inst).used_by().is_empty() {
                // Not referenced by any other instruction, safe to remove
                drop(func_data.dfg_mut().remove_value(inst));
            } else {
                worklist.push_back(inst);
            }
        }

        // remove empty basic blocks
        // for bb in bb_worklist {
        //     drop(func_data.layout_mut().bbs_mut().remove(&bb));
        // }

        if func_data.name() == "@main" {
            for bb in bb_worklist {
                let zero = func_data.dfg_mut().new_value().integer(0).clone();
                let ret_inst = func_data.dfg_mut().new_value().ret(Some(zero)).clone();
                let bb_node = func_data.layout_mut().bbs_mut().node_mut(&bb).unwrap();
                bb_node.insts_mut().push_key_back(ret_inst).unwrap();
            }
        }
    }

    fn is_terminator(inst: &ValueData) -> bool {
        matches!(
            inst.kind(),
            ValueKind::Branch(_) | ValueKind::Return(_) | ValueKind::Jump(_)
        )
    }
}