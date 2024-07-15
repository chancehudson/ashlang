use crate::parser::{Expr, Op};
use std::collections::HashMap;

/**
 * Iterate over the AST to see what variables
 * are accessed the most
 *
 * Automatically move vars between memory and stack
 */
pub struct VM {
    pub stack: Vec<String>,
    pub vars: HashMap<String, usize>,
    pub asm: Vec<String>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            vars: HashMap::new(),
            stack: Vec::new(),
            asm: Vec::new(),
        }
    }

    pub fn return_expr(&mut self, expr: Expr) {
        self.eval(expr);
        self.asm.push(format!("write_io 1"));
    }

    pub fn let_var(&mut self, name: String, expr: Expr) {
        if self.vars.contains_key(&name) {
            panic!("var is not unique");
        }
        self.eval(expr);
        self.vars.insert(name, self.stack.len());
    }

    pub fn set_var(&mut self, name: String, expr: Expr) {
        if !self.vars.contains_key(&name) {
            panic!("var does not exist");
        }
        // new value is on the top of the stack
        self.eval(expr);
        self.asm.push(format!("swap {}", self.stack_index(&name)));
        self.asm.push("pop 1".to_string());
        self.stack.pop();
    }

    pub fn stack_index(&self, var_name: &String) -> usize {
        if let Some(var) = self.vars.get(var_name) {
            (self.stack.len() - var) + 1
        } else {
            panic!("unknown var");
        }
    }

    pub fn eval(&mut self, expr: Expr) {
        let mut asm = match &expr {
            Expr::Val(name) => {
                self.stack.push(name.clone());
                vec![format!("dup {}", self.stack_index(name))]
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

    pub fn halt(&mut self) {
        self.asm.push("halt".to_string());
    }
}
