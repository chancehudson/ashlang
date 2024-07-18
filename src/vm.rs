use crate::parser::{BoolOp, Expr, Op};
use std::collections::HashMap;

#[derive(Clone)]

pub struct Var {
    stack_index: usize,
    block_index: usize,
}
/**
 * This structure is used to track a simple model
 * of the VM being executed. An instance of the
 * top of the stack is stored with variables at fixed
 * indices
 *
 * MISC TODO ITEMS:
 *
 * Iterate over the AST to see what variables
 * are accessed the most
 *
 * Automatically move vars between memory and stack
 */
pub struct VM {
    // represents the contents of the stack
    pub stack: Vec<String>,
    // name of variable keyed to offset in the stack
    // offsets are based on zero so they stay correct
    // as items are pushed/popped on the stack
    pub vars: HashMap<String, Var>,
    // constants stored keyed to their value
    pub consts: HashMap<String, u64>,
    // compiled assembly
    pub asm: Vec<String>,
    // map of function name to number of invocations
    // the compiler needs this stat
    // it's not used in vm
    pub fn_calls: HashMap<String, u64>,
    // track whether the current vm has returned
    // this means the stack is cleared of variables
    pub has_returned: bool,
    // tracks the current logic block depth
    // the executor can see variables in higher blocks
    // but not lower blocks
    pub block_depth: usize,
}

impl VM {
    pub fn new() -> Self {
        VM {
            vars: HashMap::new(),
            stack: Vec::new(),
            asm: Vec::new(),
            consts: HashMap::new(),
            fn_calls: HashMap::new(),
            has_returned: false,
            block_depth: 0,
        }
    }

    pub fn from_vm(vm: &Self) -> Self {
        VM {
            vars: vm.vars.clone(),
            stack: vm.stack.clone(),
            asm: Vec::new(),
            consts: vm.consts.clone(),
            fn_calls: HashMap::new(),
            has_returned: false,
            block_depth: vm.block_depth,
        }
    }

    // begin execution of a block
    // we'll track any variables created during
    // execution and remove them from the stack
    // in `end_block`
    pub fn begin_block(&mut self) {
        self.block_depth += 1;
    }

    // remove variables from the stack and the
    // local vm
    pub fn end_block(&mut self) {
        if self.block_depth == 0 {
            panic!("cannot exit execution root");
        }
        // find all variables in this depth
        // and remove them from the stack
        let entries_to_remove = self
            .vars
            .iter()
            .filter(|(_k, v)| v.block_index == self.block_depth)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<(String, Var)>>();
        if entries_to_remove.is_empty() {
            self.block_depth -= 1;
            return;
        }
        self.asm.push(format!("pop {}", entries_to_remove.len()));
        // TODO: don't iterate here
        for _ in 0..entries_to_remove.len() {
            self.stack.pop();
        }
        for (k, _v) in entries_to_remove {
            self.vars.remove(&k);
        }
        self.block_depth -= 1;
    }

    // define a constant that will be available in
    // the current VM object
    pub fn const_var(&mut self, name: String, expr: Expr) {
        // check for duplicate var names
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("name is not unique");
        }
        match expr {
            Expr::Lit(v) => {
                self.consts.insert(name, v);
            }
            Expr::Val(ref_name) => {
                if self.consts.contains_key(&ref_name) {
                    self.consts
                        .insert(name, *self.consts.get(&ref_name).unwrap());
                } else if self.vars.contains_key(&ref_name) {
                    panic!("dynamically evaluated consts not supported");
                } else {
                    panic!("unknown variable");
                }
            }
            Expr::NumOp {
                lhs: _,
                op: _,
                rhs: _,
            } => {
                panic!("numerical operations in constants is not yet supported");
            }
            Expr::FnCall(_name, _vars) => {
                panic!("constant expression functions not implemented");
            }
            Expr::BoolOp {
                lhs: _,
                bool_op: _,
                rhs: _,
            } => {
                panic!("boolean operations in constants is not supported");
            }
        }
    }

    // return a value to the calling function
    //
    // this is not a way to create a public output
    // this is cross-function communication
    //
    // this function does not clean up the local variable
    // state. e.g. this VM cannot be used after returning
    pub fn return_expr(&mut self, expr: Expr) {
        // we leave the returned value on the top of the stack
        self.eval(expr);
        // when we're done executing a block we clear
        // everything on the stack so that when we return
        // to the previous position the stack is in a
        // predictable state
        if self.vars.is_empty() {
            self.has_returned = true;
            return;
        }
        self.asm.push(format!("swap {}", self.vars.len()));
        self.asm.push(format!("pop {}", self.vars.len()));
        self.has_returned = true;
    }

    // if the VM has not yet returned this function
    // pops any variables managed by the VM off the stack
    //
    // this function does not clean up the local variable
    // state. e.g. this VM cannot be used after returning
    pub fn return_if_needed(&mut self) {
        if self.has_returned || self.vars.is_empty() {
            return;
        }
        self.asm.push(format!("pop {}", self.vars.len()));
        self.asm.push("push 0".to_string());
        self.has_returned = true;
    }

    // defines a new mutable variable in the current block scope
    pub fn let_var(&mut self, name: String, expr: Expr) {
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("var is not unique");
        }
        self.eval(expr);
        self.vars.insert(
            name,
            Var {
                stack_index: self.stack.len(),
                block_index: self.block_depth,
            },
        );
    }

    // defines a new variable that is being passed to a function
    // such a variable must already exist on the top of the stack
    // relative to the local stack
    //
    // e.g. if the local stack is empty the variable must be on the
    // top of the stack. If the local stack has 1 entry the variable
    // must be index 1 in the stark stack.
    pub fn fn_var(&mut self, name: String) {
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("var is not unique");
        }
        self.stack.push(name.clone());
        self.vars.insert(
            name,
            Var {
                stack_index: self.stack.len(),
                block_index: self.block_depth,
            },
        );
    }

    // assign a variable that already exists
    //
    // do this by evaluating the expression on
    // the top of the stack, then swapping with
    // the real location on the stack, and popping
    // the swapped value
    pub fn set_var(&mut self, name: String, expr: Expr) {
        if !self.vars.contains_key(&name) {
            panic!("var does not exist {name}");
        }
        // new value is on the top of the stack
        self.eval(expr);
        self.asm.push(format!("swap {}", self.stack_index(&name)));
        self.asm.push("pop 1".to_string());
        self.stack.pop();
    }

    // get the index of a variable in the execution stack
    //
    // the local stack tracks the relative positions of
    // variables in the execution stack
    pub fn stack_index(&self, var_name: &String) -> usize {
        if let Some(var) = self.vars.get(var_name) {
            self.stack.len() - var.stack_index
        } else {
            panic!("unknown var");
        }
    }

    // blocks are inserted into the asm as functions
    // each block has a function name and is accessed
    // with a jump (call)
    pub fn call_block(&mut self, block_name: &String) {
        self.asm.push(format!("call {block_name}"));
    }

    // evaluate an AST expression
    //
    // manipulates the execution stack and tracks
    // changes in the local stack
    pub fn eval(&mut self, expr: Expr) {
        let mut asm = match &expr {
            Expr::FnCall(name, vars) => {
                if let Some(c) = self.fn_calls.get_mut(name) {
                    *c += 1;
                } else {
                    self.fn_calls.insert(name.clone(), 1);
                }
                // we push these but don't pop them here
                // the destination function will handle that
                for v in vars.iter().rev().collect::<Vec<&String>>() {
                    self.eval(Expr::Val(v.clone()));
                }
                // we pop them off the virtual stack assuming
                // that the callee will do the _actual_ popping
                for _ in 0..vars.len() {
                    self.stack.pop();
                }
                // we push 1 element onto the virtual stack
                // this element is the return value of the function
                // or 0 if the function does not explicitly
                // return a value
                self.stack.push(name.clone());
                vec![format!("call {name}")]
            }
            Expr::Val(name) => {
                // if the val is a constant we push to stack
                if self.vars.contains_key(name) {
                    let out = vec![format!("dup {}", self.stack_index(name))];
                    self.stack.push(name.clone());
                    out
                } else if self.consts.contains_key(name) {
                    self.stack.push(name.clone());
                    vec![format!("push {}", self.consts.get(name).unwrap())]
                } else {
                    panic!("unknown variable: {name}");
                }
            }
            Expr::Lit(v) => {
                self.stack.push(v.to_string());
                vec![format!("push {}", v)]
            }
            Expr::NumOp { lhs, op, rhs } => {
                let l = (*lhs).clone();
                self.eval(*l);
                let r = (*rhs).clone();
                self.eval(*r);
                match op {
                    // each one of these removes two elements and
                    // adds 1
                    // so we have a net effect of a single pop
                    Op::Add => {
                        self.stack.pop();
                        vec![format!("add")]
                    }
                    Op::Sub => {
                        self.stack.pop();
                        vec![format!("push -1"), format!("mul"), format!("add")]
                    }
                    Op::Mul => {
                        self.stack.pop();
                        vec![format!("mul")]
                    }
                    Op::Inv => {
                        self.stack.pop();
                        vec![format!("invert"), format!("mul")]
                    }
                }
            }
            Expr::BoolOp { lhs, bool_op, rhs } => {
                let l = (*lhs).clone();
                self.eval(*l);
                let r = (*rhs).clone();
                self.eval(*r);
                match bool_op {
                    BoolOp::Equal => {
                        self.stack.pop();
                        vec![format!("eq"), format!("skiz")]
                    }
                    BoolOp::NotEqual => {
                        // TODO: possibly use the eq instruction
                        self.stack.pop();
                        vec![
                            format!("push -1"),
                            format!("mul"),
                            format!("add"),
                            format!("skiz"),
                        ]
                    }
                    _ => panic!("boolean operation not supported"),
                }
            }
        };
        self.asm.append(&mut asm);
    }
}
