use crate::compiler::CompilerState;
use crate::log;
use crate::math::matrix::Matrix;
use crate::math::FieldElement;
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
pub struct Var<T: FieldElement> {
    pub index: Option<usize>,
    pub location: VarLocation,
    pub value: Matrix<T>,
}

pub struct VM<'a, T: FieldElement> {
    // global counter for distinct variables
    // variable at index 0 is always 1
    pub var_index: usize,
    // local scope name keyed to global variable index
    pub vars: HashMap<String, Var<T>>,
    pub compiler_state: &'a mut CompilerState,
    // a, b, c
    pub constraints: Vec<R1csConstraint<T>>,
    pub args: Vec<Var<T>>,
    pub return_val: Option<Var<T>>,
}

impl<'a, T: FieldElement> VM<'a, T> {
    pub fn new(compiler_state: &'a mut CompilerState) -> Self {
        // add the field safety constraint
        // constrains -1*1 * -1*1 - 1 = 0
        // should fail in any field that is different than
        // the current one
        let constraints = vec![R1csConstraint::new(
            vec![(T::zero() - T::one(), 0)],
            vec![(T::zero() - T::one(), 0)],
            vec![(T::one(), 0)],
            "field safety constraint",
        )];
        VM {
            var_index: 1,
            vars: HashMap::new(),
            compiler_state,
            constraints,
            args: Vec::new(),
            return_val: None,
        }
    }

    pub fn from(vm: &'a mut VM<T>, args: Vec<Var<T>>) -> Self {
        VM {
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
                    if is_let && self.vars.contains_key(&name) {
                        log::error!(&format!("variable already defined: {name}"));
                    } else if !is_let && !self.vars.contains_key(&name) {
                        log::error!(&format!("variable does not exist in scope: {name}"));
                    }
                    let v = self.eval(&expr);
                    if v.location == VarLocation::Constraint {
                        // if we get a constrained variable from the
                        // evaluation we simply store that as a named variable
                        self.vars.insert(name, v);
                    } else {
                        // if we get a static variable from the evaluation
                        // we constraint the assigment into a new signal
                        let new_var = Var {
                            index: Some(self.var_index),
                            location: VarLocation::Constraint,
                            value: v.value,
                        };
                        self.var_index += new_var.value.len();
                        for v in &new_var.value.values {
                            // assigning a constant
                            self.constraints.push(R1csConstraint::new(
                                vec![(T::from(1), new_var.index.unwrap())],
                                vec![(T::from(1), 0)],
                                vec![(v.clone(), 0)],
                                &format!(
                                    "assigning literal ({v}) to signal {}",
                                    new_var.index.unwrap()
                                ),
                            ));
                            self.constraints.push(R1csConstraint::symbolic(
                                new_var.index.unwrap(),
                                vec![(v.clone(), 0)],
                                vec![(T::from(0), 0)],
                                SymbolicOp::Add,
                            ));
                        }
                        self.vars.insert(name, new_var);
                    }
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
                AstNode::StaticDef(name, expr) => {
                    if self.vars.contains_key(&name) {
                        log::error!("variable already defined: {name}");
                    }
                    let v = self.eval(&expr);
                    if v.location != VarLocation::Static {
                        log::error!("static variable cannot be assigned from signal");
                    }
                    self.vars.insert(name, v);
                }
                _ => {
                    log::error!(&format!("ast node not supported for r1cs: {:?}", v));
                }
            }
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Var<T> {
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
                let args = vars.into_iter().map(|v| self.eval(&*v)).collect::<_>();
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
                        index: None,
                        location: VarLocation::Static,
                        value: Matrix::from(T::one()),
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
            Expr::Lit(val) => Var {
                index: None,
                location: VarLocation::Static,
                value: Matrix::from(T::from(*val)),
            },
            _ => {
                log::error!("unimplemented expression case");
            }
        }
    }

    // handle the following cases
    // 1. lhs and rhs are both constraint variables
    // 2. lhs is a constraint variable and rhs is a static (and vis-versa)
    // 3. lhs and rhs are both static variables
    fn eval_numop(&mut self, lhs: &Expr, op: &NumOp, rhs: &Expr) -> Var<T> {
        let lv = self.eval(lhs);
        let rv = self.eval(rhs);
        // check that the variables are the same type
        lv.value.assert_eq_shape(&rv.value);

        if lv.location == VarLocation::Constraint && rv.location == VarLocation::Constraint {
            // both are signals
            self.eval_numop_signals(&lv, op, &rv)
        } else if lv.location != rv.location {
            // one is signal one is static
            self.eval_numop_signal_static(&lv, op, &rv)
        } else {
            // both static
            self.eval_numop_static(&lv, op, &rv)
        }
    }

    fn eval_numop_static(&mut self, lv: &Var<T>, op: &NumOp, rv: &Var<T>) -> Var<T> {
        match op {
            NumOp::Add => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() + rv.value.clone(),
            },
            NumOp::Mul => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() * rv.value.clone(),
            },
            NumOp::Sub => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() - rv.value.clone(),
            },
            NumOp::Inv => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() / rv.value.clone(),
            },
        }
    }

    fn eval_numop_signal_static(&mut self, lv: &Var<T>, op: &NumOp, rv: &Var<T>) -> Var<T> {
        let static_v;
        let signal_v;
        if lv.location == VarLocation::Static {
            static_v = lv;
            signal_v = rv;
        } else {
            static_v = rv;
            signal_v = lv;
        }
        match op {
            NumOp::Add => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() + rv.value.clone(),
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let svi = signal_v.index.unwrap() + x;
                    // the coefficient value, not a signal index
                    let cv = static_v.value.values[x].clone();
                    let ovi = new_var.index.unwrap() + x;
                    // (cv*1 + 1*svi)*(1*1) - (1*ovi) = 0
                    // cv + sv - ov = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(cv.clone(), 0), (T::one(), svi)],
                            vec![(T::one(), 0)],
                            vec![(T::one(), ovi)],
                            &format!("addition between ({cv}) and {svi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(cv, 0), (T::one(), svi)],
                            vec![(T::one(), 0)],
                            SymbolicOp::Mul,
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Mul => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() * rv.value.clone(),
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let svi = signal_v.index.unwrap() + x;
                    // the coefficient value, not a signal index
                    let cv = static_v.value.values[x].clone();
                    let ovi = new_var.index.unwrap() + x;
                    // (cv*1)*(1*svi) - (1*ovi) = 0
                    // cv * sv - ov = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(cv.clone(), 0)],
                            vec![(T::one(), svi)],
                            vec![(T::one(), ovi)],
                            &format!("multiplication between ({cv}) and {svi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(cv, 0)],
                            vec![(T::one(), svi)],
                            SymbolicOp::Mul,
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Sub => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() - rv.value.clone(),
                };
                self.var_index += new_var.value.len();
                if lv.location == VarLocation::Constraint {
                    // subtracting a static from a signal
                    for x in 0..new_var.value.len() {
                        let svi = lv.index.unwrap() + x;
                        // the value being subtracted
                        let cv = rv.value.values[x].clone();
                        let ovi = new_var.index.unwrap() + x;
                        // (1*ovi + cv*1)*(1*1) - (1*svi) = 0
                        // (ovi + cv)*1 - svi = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(cv.clone(), 0), (T::one(), ovi)],
                                vec![(T::one(), 0)],
                                vec![(T::one(), svi)],
                                &format!("subtraction between {svi} and ({cv}) into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(T::one(), svi), (-cv.clone(), 0)],
                                vec![(T::one(), 0)],
                                SymbolicOp::Mul,
                            ),
                        ]);
                    }
                } else {
                    // subtracting a signal from a static
                    for x in 0..new_var.value.len() {
                        let lv = lv.value.values[x].clone();
                        // the static being subtracted
                        let rvi = rv.index.unwrap() + x;
                        let ovi = new_var.index.unwrap() + x;
                        // (1*rvi + 1*ovi)*(1*1) - (lv*1) = 0
                        // (rvi + ovi)*1 - lv = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(T::one(), rvi), (T::one(), ovi)],
                                vec![(T::one(), 0)],
                                vec![(lv.clone(), 0)],
                                &format!("subtraction between ({lv}) and {rvi} into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(lv, 0), (-T::one(), rvi)],
                                vec![(T::one(), 0)],
                                SymbolicOp::Mul,
                            ),
                        ]);
                    }
                }
                new_var
            }
            NumOp::Inv => {
                log::error!("division by signal and static not supported");
            }
        }
    }

    fn eval_numop_signals(&mut self, lv: &Var<T>, op: &NumOp, rv: &Var<T>) -> Var<T> {
        // take a lhs and rhs of variable size and apply
        // an operation to each element
        match op {
            NumOp::Add => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() + rv.value.clone(),
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = rv.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv + 1*rv) * (1*1) - (1*new_var) = 0
                    // lv + rv - new_var = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(T::one(), lvi), (T::one(), rvi)],
                            vec![(T::one(), 0)],
                            vec![(T::one(), ovi)],
                            &format!("addition between {lvi} and {rvi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(T::one(), lvi), (T::one(), rvi)],
                            vec![(T::one(), 0)],
                            SymbolicOp::Mul,
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Mul => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() * rv.value.clone(),
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = rv.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv + 1*rv) * (1*1) - (1*new_var) = 0
                    // lv + rv - new_var = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(T::one(), lvi)],
                            vec![(T::one(), rvi)],
                            vec![(T::one(), ovi)],
                            &format!("multiplication between {lvi} and {rvi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(T::one(), lvi)],
                            vec![(T::one(), rvi)],
                            SymbolicOp::Mul,
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Sub => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() * rv.value.mul_scalar(T::zero() - T::one()),
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = rv.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv + -1*rv) * (1*1) - (1*new_var) = 0
                    // lv + -1*rv - new_var = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(T::one(), lvi), (T::one().neg(), rvi)],
                            vec![(T::one(), 0)],
                            vec![(T::one(), ovi)],
                            &format!("subtraction between {lvi} and {rvi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(T::one(), lvi), (T::one().neg(), rvi)],
                            vec![(T::one(), 0)],
                            SymbolicOp::Mul,
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Inv => {
                let inv_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: rv.value.invert(),
                };
                self.var_index += inv_var.value.len();
                for x in 0..inv_var.value.len() {
                    let rvi = rv.index.unwrap() + x;
                    let ovi = inv_var.index.unwrap() + x;
                    // first: constrain rhs_inv
                    // (1*rhs) * (1*rhs_inv) - (1*1) = 0
                    // rhs * rhs_inv - 1 = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(T::one(), rvi)],
                            vec![(T::one(), ovi)],
                            vec![(T::one(), 0)],
                            &format!("inversion of {rvi} into {ovi} (1/2)"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(T::one(), rvi)],
                            vec![],
                            SymbolicOp::Inv,
                        ),
                    ]);
                }
                // then multiply rv_inv by the lhs
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() * inv_var.value.clone(),
                };
                self.var_index += new_var.value.len();
                for x in 0..inv_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = inv_var.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv) * (1*rv) - (1*new_var) = 0
                    // lv * rv - new_var = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(T::one(), lvi)],
                            vec![(T::one(), rvi)],
                            vec![(T::one(), ovi)],
                            &format!("multiplication of {lvi} and {rvi} into {ovi} (2/2)"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(T::one(), lvi)],
                            vec![(T::one(), rvi)],
                            SymbolicOp::Mul,
                        ),
                    ]);
                }
                new_var
            }
        }
    }
}
