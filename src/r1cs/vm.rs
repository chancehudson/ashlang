use crate::{
    compiler::CompilerState,
    log,
    parser::{AstNode, Expr, Op},
};
use std::collections::HashMap;

// a b and c represent values in
// a constraint a * b - c = 0
// each factor specifies an array
// of coefficient, index pairs
// indices may be specified multiple times
// and will be summed
pub struct R1csConstraint {
    // (coefficient, var_index)
    pub a: Vec<(u64, u64)>,
    pub b: Vec<(u64, u64)>,
    pub c: Vec<(u64, u64)>,
}

impl ToString for R1csConstraint {
    fn to_string(&self) -> String {
        let mut out = "".to_owned();
        out.push_str("[");
        for (coef, index) in &self.a {
            out.push_str(&format!("({},{})", coef, index));
        }
        out.push_str("][");
        for (coef, index) in &self.b {
            out.push_str(&format!("({},{})", coef, index));
        }
        out.push_str("][");
        for (coef, index) in &self.c {
            out.push_str(&format!("({},{})", coef, index));
        }
        out.push_str("]");
        out
    }
}

pub struct VM<'a> {
    pub prime: u64,
    // global counter for distinct variables
    // variable at index 0 is always 1
    pub var_index: u64,
    // local scope name keyed to global variable index
    pub vars: HashMap<String, u64>,
    pub compiler_state: &'a mut CompilerState,
    // a, b, c
    pub constraints: Vec<R1csConstraint>,
}

impl<'a> VM<'a> {
    pub fn new(compiler_state: &'a mut CompilerState) -> Self {
        VM {
            prime: 18446744069414584321, // 0xfoi
            var_index: 1,
            vars: HashMap::new(),
            compiler_state,
            constraints: Vec::new(),
        }
    }

    pub fn eval(&mut self, expr: Expr) -> u64 {
        match &expr {
            Expr::VecLit(_v) => {
                panic!("vector literals must be assigned before operation");
            }
            Expr::VecVec(_v) => {
                panic!("matrix literals must be assigned before operation");
            }
            Expr::FnCall(name, vars) => {
                log::error!(&format!("function calls not supported in r1cs: {name}"));
            }
            Expr::Val(name, indices) => {
                if indices.len() > 0 {
                    log::error!(
                        "indices not supported in r1cs, accessing indices on variable: {name}"
                    );
                }
                if let Some(v) = self.vars.get(name) {
                    return v.clone();
                } else {
                    log::error!("variable not found: {name}");
                }
            }
            Expr::NumOp { lhs, op, rhs } => {
                let lv = self.eval(*lhs.clone());
                let rv = self.eval(*rhs.clone());
                match op {
                    Op::Add => {
                        let new_var = self.var_index;
                        self.var_index += 1;
                        // (1*lv + 1*rv) * (1*1) - (1*new_var) = 0
                        // lv + rv - new_var = 0
                        self.constraints.push(R1csConstraint {
                            a: vec![(1, lv), (1, rv)],
                            b: vec![(1, 0)],
                            c: vec![(1, new_var)],
                        });
                        return new_var;
                    }
                    Op::Mul => {
                        let new_var = self.var_index;
                        self.var_index += 1;
                        // (1*lv) * (1*rv) - (1*new_var) = 0
                        // lv * rv - new_var = 0
                        self.constraints.push(R1csConstraint {
                            a: vec![(1, lv)],
                            b: vec![(1, rv)],
                            c: vec![(1, new_var)],
                        });
                        return new_var;
                    }
                    Op::Sub => {
                        let new_var = self.var_index;
                        self.var_index += 1;
                        // (1*lv + -1*rv) * (1*1) - (1*new_var) = 0
                        // lv + -1*rv - new_var = 0
                        self.constraints.push(R1csConstraint {
                            a: vec![(1, lv), (self.prime - 1, rv)],
                            b: vec![(1, 0)],
                            c: vec![(1, new_var)],
                        });
                        return new_var;
                    }
                    Op::Inv => {
                        // (1/rhs) * lhs
                        //
                        let rhs_inv = self.var_index;
                        self.var_index += 1;
                        // first: constrain rhs_inv
                        // (1*rhs) * (1*rhs_inv) - (1*1) = 0
                        // rhs * rhs_inv - 1 = 0
                        self.constraints.push(R1csConstraint {
                            a: vec![(1, lv)],
                            b: vec![(1, rhs_inv)],
                            c: vec![(1, 0)],
                        });
                        let new_var = self.var_index;
                        self.var_index += 1;
                        // second: constrain the inv multiplication
                        // (1*lhs) * (1*rhs_inv) - (1*new_var) = 0
                        self.constraints.push(R1csConstraint {
                            a: vec![(1, lv)],
                            b: vec![(1, rhs_inv)],
                            c: vec![(1, new_var)],
                        });
                        return new_var;
                    }
                }
            }
            Expr::Lit(val) => {
                let new_var = self.var_index;
                self.var_index += 1;
                self.constraints.push(R1csConstraint {
                    a: vec![(1, new_var)],
                    b: vec![(1, 0)],
                    c: vec![(val.clone(), 0)],
                });
                new_var
            }
            _ => {
                log::error!("unimplemented expression case");
            }
        }
    }

    pub fn eval_ast(&mut self, ast: Vec<AstNode>) {
        for v in ast {
            match v {
                AstNode::Stmt(name, is_let, expr) => {
                    if !is_let {
                        log::error!("re-assignment not supported");
                    }
                    if self.vars.contains_key(&name) {
                        log::error!("variable already defined: {name}");
                    }
                    // returns a variable index
                    let v = self.eval(expr);
                    self.vars.insert(name, v);
                }
                AstNode::Const(name, expr) => {}
                _ => {
                    log::error!(&format!("ast node not supported for r1cs: {:?}", v));
                }
            }
        }
    }
}
