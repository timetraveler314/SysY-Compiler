use std::collections::{HashMap, HashSet};
use koopa::ir::{Function, ValueKind};

#[derive(Clone, Debug)]
pub struct CallGraphBody {
    pub callee: HashSet<Function>,
    pub max_args: usize,
}

#[derive(Clone)]
pub struct CallGraph {
    pub graph: HashMap<Function, CallGraphBody>,
}

impl CallGraph {
    pub fn build(program: &koopa::ir::Program) -> Self {
        let mut graph = CallGraph {
            graph: HashMap::new(),
        };

        for &func_h in program.func_layout() {
            let func = program.func(func_h);
            for (_bb_h, bb) in func.layout().bbs() {
                for (value_h, _) in bb.insts() {
                    let value_data = func.dfg().value(*value_h);
                    if let ValueKind::Call(call) = value_data.kind() {
                        let callee = call.callee();
                        let num_args = call.args().len();
                        graph.add_call(func_h, callee, num_args);
                    }
                }
            }
        }

        graph
    }

    pub fn add_call(&mut self, caller: Function, callee: Function, num_args: usize) {
        if !self.graph.contains_key(&caller) {
            self.graph.insert(caller, CallGraphBody {
                callee: HashSet::new(),
                max_args: 0,
            });
        }

        let body = self.graph.get_mut(&caller).unwrap();
        body.callee.insert(callee);
        body.max_args = body.max_args.max(num_args);
    }
}