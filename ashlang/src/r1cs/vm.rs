use std::collections::HashMap;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;
use std::str::FromStr;

use anyhow::Result;
use lettuce::FieldScalar;
use lettuce::Matrix;
use lettuce::Vector;

use crate::compiler::CompilerState;
use crate::log;
use crate::parser::AstNode;
use crate::parser::Expr;
use crate::parser::NumOp;
use crate::r1cs::constraint::R1csConstraint;
use crate::r1cs::constraint::SymbolicOp;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Static,
    Constraint,
}

#[derive(Clone, PartialEq, Debug)]
pub enum VarType<T: FieldScalar> {
    Scalar(T),
    Vector(Vector<T>),
    Matrix(Matrix<T>),
}

impl<T: FieldScalar> VarType<T> {
    pub fn len(&self) -> usize {
        match self {
            Self::Scalar(_) => 1,
            Self::Vector(v) => v.len(),
            Self::Matrix(v) => v.height() * v.width(),
        }
    }

    pub fn iter(&self) -> Vec<T> {
        match self {
            Self::Scalar(v) => vec![*v],
            Self::Vector(v) => v.iter().copied().collect::<Vec<_>>(),
            Self::Matrix(v) => v
                .iter()
                .flat_map(|vec| vec.iter())
                .copied()
                .collect::<Vec<_>>(),
        }
    }

    pub fn inverse(&self) -> Self {
        match self {
            Self::Scalar(v) => Self::Scalar(v.inverse()),
            Self::Vector(v) => Self::Vector(v.iter().map(|v| v.inverse()).collect::<Vector<_>>()),
            Self::Matrix(v) => Self::Matrix(Matrix::from_iter(
                v.iter()
                    .map(|vec| vec.iter().map(|v| v.inverse()).collect::<Vector<_>>()),
            )),
        }
    }
}

impl<T: FieldScalar> Add for VarType<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Scalar(lhs), Self::Scalar(rhs)) => Self::Scalar(lhs + rhs),
            (Self::Scalar(lhs), Self::Vector(rhs)) => Self::Vector(rhs + lhs),
            (Self::Scalar(lhs), Self::Matrix(rhs)) => unimplemented!(),
            (Self::Vector(lhs), Self::Scalar(rhs)) => Self::Vector(lhs + rhs),
            (Self::Vector(lhs), Self::Vector(rhs)) => Self::Vector(lhs + &rhs),
            (Self::Vector(lhs), Self::Matrix(rhs)) => unimplemented!(),
            (Self::Matrix(lhs), Self::Scalar(rhs)) => unimplemented!(),
            (Self::Matrix(lhs), Self::Vector(rhs)) => {
                unimplemented!()
            }
            (Self::Matrix(lhs), Self::Matrix(rhs)) => Self::Matrix(lhs + &rhs),
        }
    }
}

impl<T: FieldScalar> Mul for VarType<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Scalar(lhs), Self::Scalar(rhs)) => Self::Scalar(lhs * rhs),
            (Self::Scalar(lhs), Self::Vector(rhs)) => Self::Vector(rhs * lhs),
            (Self::Scalar(lhs), Self::Matrix(rhs)) => unimplemented!(),
            (Self::Vector(lhs), Self::Scalar(rhs)) => Self::Vector(lhs * rhs),
            (Self::Vector(lhs), Self::Vector(rhs)) => Self::Vector(lhs * &rhs),
            (Self::Vector(lhs), Self::Matrix(rhs)) => unimplemented!(),
            (Self::Matrix(lhs), Self::Scalar(rhs)) => unimplemented!(),
            (Self::Matrix(lhs), Self::Vector(rhs)) => {
                unimplemented!()
            }
            (Self::Matrix(lhs), Self::Matrix(rhs)) => Self::Matrix(lhs * &rhs),
        }
    }
}

impl<T: FieldScalar> Sub for VarType<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self + match rhs {
            Self::Scalar(v) => Self::Scalar(T::negone() * v),
            Self::Vector(v) => Self::Vector(v * T::negone()),
            Self::Matrix(v) => Self::Matrix(v * T::negone()),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Var<T: FieldScalar> {
    pub index: Option<usize>,
    pub location: VarLocation,
    pub value: VarType<T>,
}

/// Instance of an r1cs VM. This struct is responsible for
/// taking an AST and a compiler instance and outputing
/// a set of r1cs constraints.
pub struct VM<'a, T: FieldScalar> {
    // global counter for distinct variables
    // variable at index 0 is always 1
    pub var_index: usize,
    // local scope name keyed to global variable index
    pub vars: HashMap<String, Var<T>>,
    pub compiler_state: &'a mut CompilerState<T>,
    // a, b, c
    pub constraints: Vec<R1csConstraint<T>>,
    pub args: Vec<Var<T>>,
    pub return_val: Option<Var<T>>,
    pub name: String,
}

impl<'a, T: FieldScalar> VM<'a, T> {
    pub fn new(compiler_state: &'a mut CompilerState<T>) -> Self {
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
        compiler_state.messages.push("".to_string());
        VM {
            name: "entrypoint".to_string(),
            var_index: 1,
            vars: HashMap::new(),
            compiler_state,
            constraints,
            args: Vec::new(),
            return_val: None,
        }
    }

    pub fn from(vm: &'a mut VM<T>, args: Vec<Var<T>>, name: &str) -> Self {
        VM {
            var_index: vm.var_index,
            vars: HashMap::new(),
            compiler_state: vm.compiler_state,
            constraints: Vec::new(),
            args,
            return_val: None,
            name: name.to_string(),
        }
    }

    pub fn eval_ast(&mut self, ast: Vec<AstNode>) -> Result<()> {
        for v in ast {
            match v {
                AstNode::Stmt(name, is_let, expr) => {
                    if is_let && self.vars.contains_key(&name) {
                        return log::error!(&format!("variable already defined: {name}"));
                    } else if !is_let && !self.vars.contains_key(&name) {
                        return log::error!(&format!("variable does not exist in scope: {name}"));
                    }
                    if is_let {
                        self.compiler_state
                            .messages
                            .insert(0, format!("let {name}"));
                    } else {
                        self.compiler_state
                            .messages
                            .insert(0, format!("re-assign {name}"));
                    }
                    let v = self.eval(&expr)?;
                    if v.location == VarLocation::Constraint {
                        // if we get a constrained variable from the
                        // evaluation we simply store that as a named variable
                        self.vars.insert(name, v);
                    } else {
                        // if we get a static variable from the evaluation
                        // we constraint the assigment into a new signal
                        let new_var = self.static_to_constraint(&v.value)?;
                        self.vars.insert(name, new_var);
                    }
                }
                AstNode::FnVar(names) => {
                    for (i, v) in names[0..names.len()].iter().enumerate() {
                        let name = v;
                        if self.vars.contains_key(name) {
                            return log::error!(
                                &format!("variable already defined: {name}"),
                                "attempting to define variable in function header"
                            );
                        }
                        self.vars.insert(name.clone(), self.args[i].clone());
                    }
                }
                AstNode::Rtrn(expr) => {
                    let fn_source_path = self.compiler_state.fn_to_path.get(&self.name).unwrap();
                    self.compiler_state
                        .messages
                        .insert(0, format!("return call in {}", fn_source_path));
                    if self.return_val.is_some() {
                        return log::error!(
                            "return value already set",
                            "you likely have called return more than once"
                        );
                    }
                    self.return_val = Some(self.eval(&expr)?);
                }
                AstNode::StaticDef(name, expr) => {
                    if self.vars.contains_key(&name) {
                        return log::error!("variable already defined: {name}");
                    }
                    let v = self.eval(&expr)?;
                    if v.location != VarLocation::Static {
                        return log::error!("static variable cannot be assigned from signal");
                    }
                    self.vars.insert(name, v);
                }
                AstNode::ExprUnassigned(expr) => {
                    self.compiler_state
                        .messages
                        .insert(0, "unassigned expression".to_string());
                    self.eval(&expr)?;
                }
                AstNode::Loop(expr, body) => {
                    self.compiler_state
                        .messages
                        .insert(0, "loop condition".to_string());
                    let v = self.eval(&expr)?;
                    if v.location != VarLocation::Static {
                        return log::error!("loop condition must be static variable");
                    }
                    let v = match v.value {
                        VarType::Scalar(v) => v,
                        VarType::Vector(_) | VarType::Matrix(_) => {
                            return log::error!(
                                "loop condition must be a scalar, received a vector/matrix"
                            );
                        }
                    };
                    // track the old variables, delete any variables
                    // created inside the loop body
                    let old_vars = self.vars.clone();
                    let loop_count = v.into();
                    let mut i = T::zero().into();
                    while i < loop_count {
                        self.compiler_state
                            .messages
                            .insert(0, format!("loop iteration {i}"));
                        self.eval_ast(body.clone())?;
                        i += 1;
                        let current_vars = self.vars.clone();
                        for k in current_vars.keys() {
                            if !old_vars.contains_key(k) {
                                self.vars.remove(k);
                            }
                        }
                    }
                }
                _ => {
                    return log::error!(&format!("ast node not supported for r1cs: {:?}", v));
                }
            }
        }
        Ok(())
    }

    /// Serialization to/from AST representation
    pub fn build_var_from_ast_vec(&mut self, expr: &Expr) -> Result<(Vec<usize>, Vec<T>)> {
        // first iterate all the way through to the first literal
        let mut dimensions: Vec<usize> = Vec::new();
        let mut root_expr = expr.clone();
        // first calculate the dimensions of the matrix
        loop {
            match root_expr {
                Expr::VecVec(v) => {
                    dimensions.push(v.len());
                    root_expr = v[0].clone();
                }
                Expr::VecLit(v) => {
                    dimensions.push(v.len());
                    break;
                }
                _ => {}
            }
        }
        // then pull all the literals into a 1 dimensional vec
        let mut vec_rep: Vec<T> = Vec::new();
        Self::extract_literals(expr, &mut vec_rep)?;
        Ok((dimensions, vec_rep))
    }

    fn extract_literals(expr: &Expr, out: &mut Vec<T>) -> Result<()> {
        match expr {
            Expr::VecVec(v) => {
                for a in v {
                    Self::extract_literals(a, out)?
                }
            }
            Expr::VecLit(v) => {
                let mut vv = v
                    .clone()
                    .iter()
                    .map(|v| Ok(T::from(u128::from_str(v)?)))
                    .collect::<Result<_>>()?;
                out.append(&mut vv);
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn spawn_known_variable(&mut self, val: T) {
        self.constraints.push(R1csConstraint::new(
            vec![(T::one(), self.var_index)],
            vec![(T::one(), 0)],
            vec![(val, 0)],
            &format!(
                "scalar literal ({}) to signal index ({})",
                val, self.var_index,
            ),
        ));
        self.constraints.push(R1csConstraint::symbolic(
            self.var_index,
            vec![(T::one(), 0)],
            vec![(val, 0)],
            SymbolicOp::Mul,
            format!(
                "scalar literal ({}) to signal index {} (member of vector)",
                val, self.var_index,
            ),
        ));
        self.var_index += 1;
    }

    /// Take a static variable and constrain it's current value
    /// into a signal or set of signals
    fn static_to_constraint(&mut self, var_type: &VarType<T>) -> Result<Var<T>> {
        let index = self.var_index;
        match var_type {
            VarType::Scalar(v) => {
                self.spawn_known_variable(*v);
            }
            VarType::Vector(vec) => {
                for v in vec {
                    self.spawn_known_variable(*v);
                }
            }
            VarType::Matrix(v) => {
                for vec in v.iter() {
                    for v in vec {
                        self.spawn_known_variable(*v);
                    }
                }
            }
        }
        Ok(Var {
            index: Some(index),
            location: VarLocation::Constraint,
            value: var_type.clone(),
        })
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Var<T>> {
        match &expr {
            Expr::VecVec(_) | Expr::VecLit(_) => {
                let (dimensions, values) = self.build_var_from_ast_vec(expr)?;
                if dimensions.len() > 2 {
                    anyhow::bail!("Hypercube structures are not supported.");
                }
                Ok(Var {
                    index: None,
                    location: VarLocation::Static,
                    value: if dimensions.len() == 1 {
                        VarType::Vector(values.into())
                    } else {
                        VarType::Matrix(Matrix::from_iter(
                            values.chunks(dimensions[1]).map(|v| v.to_vec().into()),
                        ))
                    },
                })
            }
            Expr::FnCall(name, vars) => {
                // TODO: break this into separate functions
                let path = self.compiler_state.fn_to_path.get(name).unwrap();
                self.compiler_state
                    .messages
                    .insert(0, format!("{}() ({})", name, path));
                let args: Vec<Var<T>> = vars.iter().map(|v| self.eval(v)).collect::<Result<_>>()?;
                // look for an ar1cs implementation first
                if let Some(v) = self.compiler_state.fn_to_r1cs_parser.get(name).cloned() {
                    let constrain_args_if_needed = args
                        .iter()
                        .map(|v| {
                            if let Some(i) = v.index {
                                return Ok(i);
                            }
                            let val = match v.value {
                                VarType::Matrix(_) | VarType::Vector(_) => {
                                    return log::error!(
                                        "cannot pass a vector static to an r1cs function"
                                    );
                                }
                                VarType::Scalar(v) => v,
                            };
                            // if we get a static variable we need to
                            // assert equality of it's current value
                            // to turn it into a signal
                            // log::error!("cannot pass a static variable to a r1cs function");
                            let index = self.var_index;
                            self.spawn_known_variable(val);
                            Ok(index)
                        })
                        .collect::<Result<_>>()?;
                    let out_constraints =
                        v.signals_as_args(self.var_index, constrain_args_if_needed)?;
                    self.constraints.append(&mut out_constraints.clone());
                    let return_index = self.var_index;
                    self.var_index += v.return_names.len();
                    return if !v.return_names.is_empty() {
                        Ok(Var {
                            index: Some(return_index),
                            location: VarLocation::Constraint,
                            // TODO: determine a value here
                            // use the symbolic constraint to determine the value
                            value: VarType::Scalar(0.into()),
                        })
                    } else {
                        Ok(Var {
                            index: None,
                            location: VarLocation::Static,
                            value: VarType::Scalar(1.into()),
                        })
                    };
                }
                let fn_ast = self.compiler_state.fn_to_ast.get(name);
                if fn_ast.is_none() {
                    return log::error!("function not found: {name}");
                }
                let fn_ast = fn_ast.unwrap().clone();
                let mut vm = VM::from(self, args, name);
                vm.eval_ast(fn_ast)?;
                let return_val = vm.return_val;
                let new_var_index = vm.var_index;
                let mut out_constraints = vm.constraints;
                self.constraints.append(&mut out_constraints);
                self.var_index = new_var_index;
                if let Some(v) = return_val {
                    Ok(v)
                } else {
                    Ok(Var {
                        index: None,
                        location: VarLocation::Static,
                        value: VarType::Scalar(1.into()),
                    })
                }
            }
            Expr::Val(name, indices) => {
                let mut new_indices: Vec<usize> = vec![];
                for index_expr in indices {
                    let v = self.eval(index_expr)?;
                    if v.value.len() != 1 || v.location != VarLocation::Static {
                        return log::error!(
                            "index notation must contain a scalar static expression in: {name}"
                        );
                    }
                    if v.value.len() != 1 {
                        return log::error!(
                            "index notation must contain a scalar static expression in: {name}"
                        );
                    }
                    new_indices.push(v.value.iter()[0].into() as usize);
                }
                let v = self.vars.get(name);
                if v.is_none() {
                    return log::error!(&format!("variable not found: {name}"));
                }
                let v = v.unwrap();
                if new_indices.len() > 2 {
                    return log::error!("only two dimensional matrices are allowed");
                }
                let (offset, val) = match &v.value {
                    VarType::Scalar(val) => {
                        assert_eq!(new_indices.len(), 0);
                        (0, VarType::Scalar(*val))
                    }
                    VarType::Vector(val) => {
                        assert!(new_indices.len() < 2);
                        if new_indices.len() == 0 {
                            (0, VarType::Vector(val.clone()))
                        } else {
                            (
                                new_indices[0] as usize,
                                VarType::Scalar(val[new_indices[0] as usize].clone()),
                            )
                        }
                    }
                    VarType::Matrix(val) => {
                        assert_ne!(new_indices.len(), 0);
                        assert!(new_indices.len() <= 2);
                        if new_indices.len() == 1 {
                            (
                                val.width() * new_indices[0] as usize,
                                VarType::Vector(val[new_indices[0] as usize].clone()),
                            )
                        } else {
                            (
                                val.width() * new_indices[0] as usize + new_indices[1] as usize,
                                VarType::Scalar(
                                    val[new_indices[0] as usize][new_indices[1] as usize],
                                ),
                            )
                        }
                    }
                };
                // let (matrix, offset) = v.value.retrieve_indices(&new_indices);
                if let Some(index) = v.index {
                    Ok(Var {
                        index: Some(index + offset),
                        location: VarLocation::Constraint,
                        value: val,
                    })
                } else {
                    Ok(Var {
                        index: None,
                        location: VarLocation::Static,
                        value: val,
                    })
                }
            }
            Expr::NumOp { lhs, op, rhs } => self.eval_numop(lhs, op, rhs),
            Expr::Lit(val) => Ok(Var {
                index: None,
                location: VarLocation::Static,
                value: VarType::Scalar(T::from(u128::from_str(val)?)),
            }),
            _ => {
                log::error!("unimplemented expression case")
            }
        }
    }

    // handle the following cases
    // 1. lhs and rhs are both constraint variables
    // 2. lhs is a constraint variable and rhs is a static (and vis-versa)
    // 3. lhs and rhs are both static variables
    fn eval_numop(&mut self, lhs: &Expr, op: &NumOp, rhs: &Expr) -> Result<Var<T>> {
        let lv = self.eval(lhs)?;
        let rv = self.eval(rhs)?;
        // check that the variables are the same type
        // lv.value.assert_eq_shape(&rv.value);

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

    fn eval_numop_static(&mut self, lv: &Var<T>, op: &NumOp, rv: &Var<T>) -> Result<Var<T>> {
        Ok(match op {
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
                value: lv.value.clone() * rv.value.clone().inverse(),
            },
        })
    }

    fn eval_numop_signal_static(&mut self, lv: &Var<T>, op: &NumOp, rv: &Var<T>) -> Result<Var<T>> {
        let static_v;
        let signal_v;
        if lv.location == VarLocation::Static {
            static_v = lv;
            signal_v = rv;
        } else {
            static_v = rv;
            signal_v = lv;
        }
        Ok(match op {
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
                    let cv = static_v.value.iter()[x].clone();
                    let ovi = new_var.index.unwrap() + x;
                    // (cv*1 + 1*svi)*(1*1) - (1*ovi) = 0
                    // cv + sv - ov = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(cv, 0), (T::one(), svi)],
                            vec![(T::one(), 0)],
                            vec![(T::one(), ovi)],
                            &format!("addition between ({cv}) and {svi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(cv, 0), (T::one(), svi)],
                            vec![(T::one(), 0)],
                            SymbolicOp::Mul,
                            self.compiler_state.messages[0].clone(),
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
                    let cv = static_v.value.iter()[x].clone();
                    let ovi = new_var.index.unwrap() + x;
                    // (cv*1)*(1*svi) - (1*ovi) = 0
                    // cv * sv - ov = 0
                    self.constraints.append(&mut vec![
                        R1csConstraint::new(
                            vec![(cv, 0)],
                            vec![(T::one(), svi)],
                            vec![(T::one(), ovi)],
                            &format!("multiplication between ({cv}) and {svi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(cv, 0)],
                            vec![(T::one(), svi)],
                            SymbolicOp::Mul,
                            self.compiler_state.messages[0].clone(),
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
                        let lvi = lv.index.unwrap() + x;
                        // the value being subtracted
                        let cv = rv.value.iter()[x].clone();
                        let ovi = new_var.index.unwrap() + x;
                        // (cv*1 + 1*ovi)*(1*1) - (1*lvi) = 0
                        // (cv + ovi)*1 - lvi = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(cv, 0), (T::one(), ovi)],
                                vec![(T::one(), 0)],
                                vec![(T::one(), lvi)],
                                &format!("subtraction between {lvi} and ({cv}) into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(T::one(), lvi), (T::negone() * cv, 0)],
                                vec![(T::one(), 0)],
                                SymbolicOp::Mul,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                    }
                } else {
                    // subtracting a signal from a static
                    for x in 0..new_var.value.len() {
                        let lv = lv.value.iter()[x].clone();
                        // the static being subtracted
                        let rvi = rv.index.unwrap() + x;
                        let ovi = new_var.index.unwrap() + x;
                        // (1*rvi + 1*ovi)*(1*1) - (lv*1) = 0
                        // (rvi + ovi)*1 - lv = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(T::one(), rvi), (T::one(), ovi)],
                                vec![(T::one(), 0)],
                                vec![(lv, 0)],
                                &format!("subtraction between ({lv}) and {rvi} into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(lv, 0), (T::negone(), rvi)],
                                vec![(T::one(), 0)],
                                SymbolicOp::Mul,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                    }
                }
                new_var
            }
            NumOp::Inv => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() * rv.value.clone().inverse(),
                };
                self.var_index += new_var.value.len();
                if lv.location == VarLocation::Constraint {
                    // can statically inv
                    // subtracting a static from a signal
                    for x in 0..new_var.value.len() {
                        let lvi = lv.index.unwrap() + x;
                        // this is a static value
                        let cv = rv.value.iter()[x].clone();
                        let icv = cv.inverse();
                        let ovi = new_var.index.unwrap() + x;
                        // (icv*lvi)*(1*1) - (1*ovi) = 0
                        // (icv*lvi)*1 - ovi = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(icv, lvi)],
                                vec![(T::one(), 0)],
                                vec![(T::one(), ovi)],
                                &format!("modinv between {lvi} and ({cv}) into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(icv, lvi)],
                                vec![(T::one(), 0)],
                                SymbolicOp::Mul,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                    }
                } else {
                    // must inv in a signal first
                    let inv_var = Var {
                        index: Some(self.var_index),
                        location: VarLocation::Constraint,
                        value: rv.value.inverse(),
                    };
                    self.var_index += inv_var.value.len();
                    for x in 0..inv_var.value.len() {
                        // first invert into a signal
                        // let lv = lv.value.values[x].clone();
                        let rvi = rv.index.unwrap() + x;
                        let ovi = inv_var.index.unwrap() + x;
                        // (1*rvi)*(1*ovi) - (1*1) = 0
                        // rvi*ovi - 1 = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(T::one(), rvi)],
                                vec![(T::one(), ovi)],
                                vec![(T::one(), 0)],
                                &format!("modinv {rvi} into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(T::one(), 0)],
                                vec![(T::one(), rvi)],
                                SymbolicOp::Inv,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                        // now constrain the new_var
                        let lv = lv.value.iter()[x].clone();
                        let rvi = inv_var.index.unwrap() + x;
                        let ovi = new_var.index.unwrap() + x;
                        // (lv*rvi)*(1*1) - (1*ovi) = 0
                        // (lv*rvi)*1 - ovi = 0
                        self.constraints.append(&mut vec![
                            R1csConstraint::new(
                                vec![(lv, rvi)],
                                vec![(T::one(), 0)],
                                vec![(T::one(), ovi)],
                                &format!("multiply {rvi} and ({lv}) into {ovi}"),
                            ),
                            R1csConstraint::symbolic(
                                ovi,
                                vec![(lv, rvi)],
                                vec![(T::one(), 0)],
                                SymbolicOp::Mul,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                    }
                }
                new_var
            }
        })
    }

    fn eval_numop_signals(&mut self, lv: &Var<T>, op: &NumOp, rv: &Var<T>) -> Result<Var<T>> {
        // take a lhs and rhs of variable size and apply
        // an operation to each element
        Ok(match op {
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
                            self.compiler_state.messages[0].clone(),
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
                            self.compiler_state.messages[0].clone(),
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Sub => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() + rv.value.clone() * VarType::Scalar(T::negone()),
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
                            vec![(T::one(), lvi), (T::negone(), rvi)],
                            vec![(T::one(), 0)],
                            vec![(T::one(), ovi)],
                            &format!("subtraction between {lvi} and {rvi} into {ovi}"),
                        ),
                        R1csConstraint::symbolic(
                            ovi,
                            vec![(T::one(), lvi), (T::negone(), rvi)],
                            vec![(T::one(), 0)],
                            SymbolicOp::Mul,
                            self.compiler_state.messages[0].clone(),
                        ),
                    ]);
                }
                new_var
            }
            NumOp::Inv => {
                let inv_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: rv.value.inverse(),
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
                            vec![(T::one(), 0)],
                            vec![(T::one(), rvi)],
                            SymbolicOp::Inv,
                            self.compiler_state.messages[0].clone(),
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
                            self.compiler_state.messages[0].clone(),
                        ),
                    ]);
                }
                new_var
            }
        })
    }
}
