use crate::{
    compiler::CompilerState,
    log,
    parser::{AstNode, Expr, NumOp},
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
    pub a: Vec<(u64, usize)>,
    pub b: Vec<(u64, usize)>,
    pub c: Vec<(u64, usize)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Static,
    Constraint,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Var {
    index: usize,
    location: VarLocation,
    dimensions: Vec<usize>,
    value: Vec<u64>,
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
    pub var_index: usize,
    // local scope name keyed to global variable index
    pub vars: HashMap<String, Var>,
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
                    let v = self.eval(&expr);
                    self.vars.insert(name, v);
                }
                AstNode::Const(name, expr) => {}
                _ => {
                    log::error!(&format!("ast node not supported for r1cs: {:?}", v));
                }
            }
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Var {
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
            Expr::NumOp { lhs, op, rhs } => self.eval_numop(&*lhs, op, &*rhs),
            Expr::Lit(val) => {
                let new_var = Var {
                    index: self.var_index,
                    location: VarLocation::Constraint,
                    dimensions: vec![],
                    value: vec![*val],
                };
                self.var_index += 1;
                self.constraints.push(R1csConstraint {
                    a: vec![(1, new_var.index)],
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

    fn eval_numop(&mut self, lhs: &Expr, op: &NumOp, rhs: &Expr) -> Var {
        let lv = self.eval(lhs);
        let rv = self.eval(rhs);
        if lv.location != VarLocation::Constraint {
            log::error!(&format!("lhs is not a constraint variable: {:?}", lhs));
        }
        if rv.location != VarLocation::Constraint {
            log::error!(&format!("rhs is not a constraint variable: {:?}", rhs));
        }
        if rv.dimensions.len() != lv.dimensions.len() {
            log::error!(&format!(
                "lhs and rhs dimensions are not equal: {:?} {:?}",
                lhs, rhs
            ));
        }
        for x in 0..rv.dimensions.len() {
            if rv.dimensions[x] != lv.dimensions[x] {
                log::error!(&format!(
                    "lhs and rhs inner dimensions are not equal: {:?} {:?}",
                    lhs, rhs
                ));
            }
        }
        // take a lhs and rhs of variable size and apply
        // an operation to each element
        let mut operate =
            |lhs: Var,
             rhs: Var,
             op: Box<dyn Fn(u64, u64, usize, usize, usize) -> (Vec<R1csConstraint>, u64)>|
             -> Var {
                let mut new_var = Var {
                    index: self.var_index,
                    location: VarLocation::Constraint,
                    dimensions: lhs.dimensions.clone(),
                    value: vec![],
                };
                self.var_index += lhs.value.len();
                for x in 0..lhs.value.len() {
                    // will generate constraints and output value
                    let (constraints, val) = (*op)(
                        lhs.value[x],
                        rhs.value[x],
                        lhs.index + x,
                        rhs.index + x,
                        new_var.index + x,
                    );
                    new_var.value.push(val);
                    for c in constraints {
                        self.constraints.push(c);
                    }
                }
                new_var
            };
        // TODO: better field math
        match op {
            NumOp::Add => {
                let add = |a: u64,
                           b: u64,
                           ai: usize,
                           bi: usize,
                           oi: usize|
                 -> (Vec<R1csConstraint>, u64) {
                    let x = (u128::try_from(a).unwrap() + u128::try_from(b).unwrap())
                        % u128::try_from(self.prime).unwrap();
                    // (1*lv + 1*rv) * (1*1) - (1*new_var) = 0
                    // lv + rv - new_var = 0
                    (
                        vec![R1csConstraint {
                            a: vec![(1, ai), (1, bi)],
                            b: vec![(1, 0)],
                            c: vec![(1, oi)],
                        }],
                        u64::try_from(x).unwrap(),
                    )
                };
                operate(lv, rv, Box::new(add))
            }
            NumOp::Mul => {
                let mul = |a: u64,
                           b: u64,
                           ai: usize,
                           bi: usize,
                           oi: usize|
                 -> (Vec<R1csConstraint>, u64) {
                    let x = (u128::try_from(a).unwrap() * u128::try_from(b).unwrap())
                        % u128::try_from(self.prime).unwrap();
                    // (1*lv) * (1*rv) - (1*new_var) = 0
                    // lv * rv - new_var = 0
                    (
                        vec![R1csConstraint {
                            a: vec![(1, ai)],
                            b: vec![(1, bi)],
                            c: vec![(1, oi)],
                        }],
                        u64::try_from(x).unwrap(),
                    )
                };
                operate(lv, rv, Box::new(mul))
            }
            NumOp::Sub => {
                let sub = |a: u64,
                           b: u64,
                           ai: usize,
                           bi: usize,
                           oi: usize|
                 -> (Vec<R1csConstraint>, u64) {
                    let x = (u128::try_from(self.prime - 1).unwrap() + u128::try_from(a).unwrap()
                        - u128::try_from(b).unwrap())
                        % u128::try_from(self.prime).unwrap();
                    // (1*lv + -1*rv) * (1*1) - (1*new_var) = 0
                    // lv + -1*rv - new_var = 0
                    (
                        vec![R1csConstraint {
                            a: vec![(1, ai), (self.prime - 1, bi)],
                            b: vec![(1, 0)],
                            c: vec![(1, oi)],
                        }],
                        u64::try_from(x).unwrap(),
                    )
                };
                operate(lv, rv, Box::new(sub))
            }
            NumOp::Inv => {
                // (1/rhs) * lhs
                // first invert the rhs and store in a variable
                let inv = |_: u64,
                           b: u64,
                           _: usize,
                           bi: usize,
                           oi: usize|
                 -> (Vec<R1csConstraint>, u64) {
                    let b_inv = crate::r1cs::inv::inv(b, self.prime);
                    // first: constrain rhs_inv
                    // (1*rhs) * (1*rhs_inv) - (1*1) = 0
                    // rhs * rhs_inv - 1 = 0
                    (
                        vec![R1csConstraint {
                            a: vec![(1, bi)],
                            b: vec![(1, oi)],
                            c: vec![(1, 0)],
                        }],
                        u64::try_from(b_inv).unwrap(),
                    )
                };
                let rv_inv = operate(rv.clone(), rv.clone(), Box::new(inv));
                // then multiple rv_inv by the lhs
                let mul = |a: u64,
                           b: u64,
                           ai: usize,
                           bi: usize,
                           oi: usize|
                 -> (Vec<R1csConstraint>, u64) {
                    let x = (u128::try_from(a).unwrap() * u128::try_from(b).unwrap())
                        % u128::try_from(self.prime).unwrap();
                    // (1*lv) * (1*rv) - (1*new_var) = 0
                    // lv * rv - new_var = 0
                    (
                        vec![R1csConstraint {
                            a: vec![(1, ai)],
                            b: vec![(1, bi)],
                            c: vec![(1, oi)],
                        }],
                        u64::try_from(x).unwrap(),
                    )
                };
                operate(lv, rv_inv, Box::new(mul))
            }
        }
    }
}
