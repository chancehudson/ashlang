use std::collections::HashMap;

use crate::parser::{AstNode, Expr, Op};


/**
 * Iterate over the AST to see what variables
 * are accessed the most
 * 
 * Automatically move vars between memory and stack
 */
struct VM {
    stack: Vec<String>,
    vars: HashMap<String, usize>,
    asm: Vec<String>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            vars: HashMap::new(),
            stack: Vec::new(),
            asm: Vec::new(),
        }
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
                        vec![format!("sub")]
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

pub fn compile(ast: Vec<AstNode>) -> String {
    let mut vm = VM::new();

    for v in ast {
        match v {
            AstNode::Stmt(name, is_let, expr) => {
                if is_let {
                    vm.let_var(name, expr);
                } else {
                    vm.set_var(name, expr)
                }
            }
        }
    }
    vm.halt();
    for l in &vm.asm {
        println!("{}", l);
    }
    vm.asm.clone().join("\n")
}
