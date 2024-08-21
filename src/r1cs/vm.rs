use crate::compiler::CompilerState;
use crate::log;
use crate::parser::AstNode;
use crate::parser::Expr;
use crate::parser::NumOp;
use crate::r1cs::constraint::R1csConstraint;
use crate::r1cs::constraint::SymbolicOp;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Static,
    Constraint,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Var {
    pub index: usize,
    pub location: VarLocation,
    pub dimensions: Vec<usize>,
    pub value: Vec<u64>,
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
    pub args: Vec<Var>,
    pub return_val: Option<Var>,
}

impl<'a> VM<'a> {
    pub fn new(compiler_state: &'a mut CompilerState) -> Self {
        VM {
            prime: 18446744069414584321, // 0xfoi
            var_index: 1,
            vars: HashMap::new(),
            compiler_state,
            constraints: Vec::new(),
            args: Vec::new(),
            return_val: None,
        }
    }

    pub fn from(vm: &'a mut VM, args: Vec<Var>) -> Self {
        VM {
            prime: vm.prime,
            var_index: vm.var_index,
            vars: HashMap::new(),
            compiler_state: vm.compiler_state,
            constraints: Vec::new(),
            args,
            return_val: None,
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
                AstNode::FnVar(names) => {
                    for x in 0..names.len() {
                        let name = &names[x];
                        if self.vars.contains_key(name) {
                            log::error!(
                                &format!("variable already defined: {name}"),
                                "attempting to define variable in function header"
                            );
                        }
                        self.vars.insert(name.clone(), self.args[x].clone());
                    }
                }
                AstNode::Rtrn(expr) => {
                    if self.return_val.is_some() {
                        log::error!(
                            "return value already set",
                            "you likely have called return more than once"
                        );
                    }
                    self.return_val = Some(self.eval(&expr));
                }
                AstNode::StaticDef(name, expr) => {}
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
                let fn_ast = self.compiler_state.fn_to_ast.get(name);
                if fn_ast.is_none() {
                    log::error!("function not found: {name}");
                }
                let fn_ast = fn_ast.unwrap().clone();
                let args = vars
                    .into_iter()
                    .map(|v| self.eval(&*v))
                    .collect::<Vec<Var>>();
                let mut vm = VM::from(self, args);
                vm.eval_ast(fn_ast);
                let return_val = vm.return_val;
                let new_var_index = vm.var_index;
                let mut out_constraints = vm.constraints;
                self.constraints.append(&mut out_constraints);
                self.var_index = new_var_index;
                if let Some(v) = return_val {
                    return v;
                } else {
                    Var {
                        index: 0,
                        location: VarLocation::Constraint,
                        dimensions: vec![],
                        value: vec![1],
                    }
                }
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
                self.constraints.push(R1csConstraint::new(
                    vec![(1, new_var.index)],
                    vec![(1, 0)],
                    vec![(val.clone(), 0)],
                    format!("assigning literal ({val}) to signal {}", new_var.index),
                ));
                self.constraints.push(R1csConstraint::symbolic(
                    new_var.index,
                    vec![(val.clone(), 0)],
                    vec![(0, 0)],
                    SymbolicOp::Add,
                ));
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
                        vec![
                            R1csConstraint::new(
                                vec![(1, ai), (1, bi)],
                                vec![(1, 0)],
                                vec![(1, oi)],
                                format!("addition between {ai} and {bi} into {oi}"),
                            ),
                            R1csConstraint::symbolic(
                                oi,
                                vec![(1, ai), (1, bi)],
                                vec![(1, 0)],
                                SymbolicOp::Mul,
                            ),
                        ],
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
                        vec![
                            R1csConstraint::new(
                                vec![(1, ai)],
                                vec![(1, bi)],
                                vec![(1, oi)],
                                format!("multiplication between {ai} and {bi} into {oi}"),
                            ),
                            R1csConstraint::symbolic(
                                oi,
                                vec![(1, ai)],
                                vec![(1, bi)],
                                SymbolicOp::Mul,
                            ),
                        ],
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
                        vec![
                            R1csConstraint::new(
                                vec![(1, ai), (self.prime - 1, bi)],
                                vec![(1, 0)],
                                vec![(1, oi)],
                                format!("subtraction between {ai} and {bi} into {oi}"),
                            ),
                            R1csConstraint::symbolic(
                                oi,
                                vec![(1, ai), (self.prime - 1, bi)],
                                vec![(1, 0)],
                                SymbolicOp::Mul,
                            ),
                        ],
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
                        vec![
                            R1csConstraint::new(
                                vec![(1, bi)],
                                vec![(1, oi)],
                                vec![(1, 0)],
                                format!("inversion of {bi} into {oi} (1/2)"),
                            ),
                            R1csConstraint::symbolic(oi, vec![(1, bi)], vec![], SymbolicOp::Inv),
                        ],
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
                        vec![
                            R1csConstraint::new(
                                vec![(1, ai)],
                                vec![(1, bi)],
                                vec![(1, oi)],
                                format!("multiplication of {ai} and {bi} into {oi} (2/2)"),
                            ),
                            R1csConstraint::symbolic(
                                oi,
                                vec![(1, ai)],
                                vec![(1, bi)],
                                SymbolicOp::Mul,
                            ),
                        ],
                        u64::try_from(x).unwrap(),
                    )
                };
                operate(lv, rv_inv, Box::new(mul))
            }
        }
    }
}
