use crate::parser::{Expr, Op};
use std::collections::HashMap;

/**
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
    pub vars: HashMap<String, usize>,
    pub consts: HashMap<String, u64>,
    // compiled assembly
    pub asm: Vec<String>,
    // the compiler needs this stat
    // it's not used in vm
    pub fn_calls: HashMap<String, u64>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            vars: HashMap::new(),
            stack: Vec::new(),
            asm: Vec::new(),
            consts: HashMap::new(),
            fn_calls: HashMap::new(),
        }
    }

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
            Expr::FnCall(_name) => {
                panic!("constant expression functions not implemented");
            }
        }
    }

    pub fn return_expr(&mut self, expr: Expr) {
        // we leave the returned value on the top of the stack
        self.eval(expr);
    }

    pub fn let_var(&mut self, name: String, expr: Expr) {
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("var is not unique");
        }
        self.eval(expr);
        self.vars.insert(name, self.stack.len());
    }

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

    pub fn stack_index(&self, var_name: &String) -> usize {
        if let Some(var) = self.vars.get(var_name) {
            self.stack.len() - var
        } else {
            panic!("unknown var");
        }
    }

    pub fn eval(&mut self, expr: Expr) {
        let mut asm = match &expr {
            Expr::FnCall(name) => {
                if let Some(c) = self.fn_calls.get_mut(name) {
                    *c += 1;
                } else {
                    self.fn_calls.insert(name.clone(), 1);
                }
                self.stack.push(name.clone());
                vec![format!("call {name}")]
            }
            Expr::Val(name) => {
                // if the val is a constant we push to stack
                if self.vars.contains_key(name) {
                    self.stack.push(name.clone());
                    vec![format!("dup {}", self.stack_index(name))]
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
        };
        self.asm.append(&mut asm);
    }
}
