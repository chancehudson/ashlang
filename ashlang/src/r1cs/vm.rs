use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;

use anyhow::Result;
use lettuce::FieldScalar;
use lettuce::Vector;

use crate::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Static,
    Constraint,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Var<E: FieldScalar> {
    pub index: Option<usize>,
    pub location: VarLocation,
    pub value: Vector<E>,
}

impl<E: FieldScalar> Var<E> {
    pub fn scalar_maybe(&self) -> Option<E> {
        if self.is_scalar() {
            Some(self.value[0])
        } else {
            None
        }
    }

    pub fn is_scalar(&self) -> bool {
        self.value.len() == 1
    }
}

/// Instance of an r1cs VM. This struct is responsible for
/// taking an AST and a compiler instance and outputing
/// a set of r1cs constraints.
pub struct VM<'a, E: FieldScalar> {
    // global counter for distinct variables
    // variable at index 0 is always 1
    pub var_index: usize,
    // local scope name keyed to global variable index
    pub vars: HashMap<String, Var<E>>,
    pub compiler_state: &'a mut CompilerState,
    // a, b, c
    pub constraints: Vec<Constraint<E>>,
    pub args: Vec<Var<E>>,
    pub return_val: Option<Var<E>>,
    pub name: String,
    pub input_len: usize,
    pub static_args: Vec<Vector<E>>,
    pub named_static_args: usize,
    pub is_entrypoint: bool,
}

impl<'a, E: FieldScalar> VM<'a, E> {
    pub fn new(
        compiler_state: &'a mut CompilerState,
        input_len: usize,
        static_args: Vec<Vector<E>>,
    ) -> Self {
        // add the field safety constraint
        // constrains -1*1 * -1*1 - 1 = 0
        // should fail in any field that is different than
        // the current one
        let constraints = vec![Constraint::new(
            vec![(E::negone(), 0)],
            vec![(E::negone(), 0)],
            vec![(E::one(), 0)],
            "field cardinality sanity constraint",
        )];
        compiler_state.messages.push("".to_string());
        let vars = HashMap::default();
        VM {
            name: "entrypoint".to_string(),
            var_index: 1, // the one variable is always at index 0
            vars,
            compiler_state,
            constraints,
            args: Vec::new(),
            return_val: None,
            is_entrypoint: true,
            input_len,
            static_args,
            named_static_args: 0,
        }
    }

    pub fn from(vm: &'a mut VM<E>, args: Vec<Var<E>>, name: &str) -> Self {
        VM {
            var_index: vm.var_index,
            vars: HashMap::new(),
            compiler_state: vm.compiler_state,
            constraints: Vec::new(),
            args,
            return_val: None,
            name: name.to_string(),
            input_len: vm.input_len,
            is_entrypoint: false,
            static_args: vec![],
            named_static_args: 0,
        }
    }

    pub fn eval_ast(&mut self, ast: Vec<AstNode>) -> Result<()> {
        for v in ast {
            match v {
                AstNode::Stmt(name, location_maybe, expr) => {
                    if location_maybe.is_none() && !self.vars.contains_key(&name) {
                        return log::error!(&format!("variable does not exist in scope: {name}"));
                    }
                    if matches!(location_maybe, Some(VarLocation::Static)) {
                        self.compiler_state
                            .messages
                            .insert(0, format!("static {name}"));
                    } else if matches!(location_maybe, Some(VarLocation::Constraint)) {
                        self.compiler_state
                            .messages
                            .insert(0, format!("let {name}"));
                    } else {
                        self.compiler_state
                            .messages
                            .insert(0, format!("re-assign {name}"));
                    }
                    let rhs = self.eval(&expr)?;

                    match (location_maybe, &rhs.location) {
                        (Some(VarLocation::Static), VarLocation::Static) => {
                            // new static var
                            let var = Var {
                                index: None,
                                location: VarLocation::Static,
                                value: rhs.value,
                            };
                            self.vars.insert(name, var.clone());
                        }
                        (Some(VarLocation::Constraint), VarLocation::Constraint) => {
                            // if we get a constrained variable from the
                            // evaluation we simply store that as a named variable
                            self.vars.insert(name, rhs);
                        }
                        (Some(VarLocation::Constraint), VarLocation::Static) => {
                            // if we get a static variable from the evaluation
                            // we constrain the assigment into a new signal
                            let new_var = self.static_to_constraint(&rhs)?;
                            self.vars.insert(name, new_var);
                        }
                        (None, VarLocation::Static) => {
                            // assignment to existing variable from a static value
                            let lhs_mut = self.vars.get_mut(&name).unwrap();
                            if lhs_mut.value.len() != rhs.value.len() {
                                anyhow::bail!(
                                    "ashlang: dimension mismatch in static re-assignment to {name}"
                                )
                            }
                            match lhs_mut.location {
                                VarLocation::Constraint => {
                                    // overwrite the existing witness variable with a new one
                                    let new_var = self.static_to_constraint(&rhs)?;
                                    self.vars.insert(name, new_var);
                                }
                                VarLocation::Static => {
                                    lhs_mut.value = rhs.value;
                                }
                            }
                        }
                        (None, VarLocation::Constraint) => {
                            // assignment to existing variable from a witness value
                            let lhs_mut = self.vars.get_mut(&name).unwrap();
                            if lhs_mut.value.len() != rhs.value.len() {
                                anyhow::bail!(
                                    "ashlang: dimension mismatch in witness re-assignment to {name}"
                                )
                            }
                            match lhs_mut.location {
                                VarLocation::Constraint => {
                                    // overwrite the existing witness variable with a new one
                                    self.vars.insert(name, rhs);
                                }
                                VarLocation::Static => {
                                    anyhow::bail!(
                                        "ashlang: cannot assign to static variable from witness variable"
                                    )
                                }
                            }
                        }
                        _ => anyhow::bail!("ashlang: unsupported variable assignment"),
                    }

                    // if let Some(var) = self.vars.get_mut(&name)
                    //     && var.location == VarLocation::Static
                    // {
                    //     if v.location == VarLocation::Constraint {
                    //         anyhow::bail!(
                    //             "ashlang: constraint variable may not be assigned to static"
                    //         );
                    //     } else {
                    //         assert!(var.value.len() == v.value.len());
                    //         var.value = v.value;
                    //         return Ok(());
                    //     }
                    // }
                    // if v.location == VarLocation::Constraint {
                    //     // if we get a constrained variable from the
                    //     // evaluation we simply store that as a named variable
                    //     self.vars.insert(name, v);
                    // } else {
                    //     // if we get a static variable from the evaluation
                    //     // we constrain the assigment into a new signal
                    //     let new_var = self.static_to_constraint(&v)?;
                    //     self.vars.insert(name, new_var);/
                    // }
                }
                AstNode::FnVar(names) => {
                    for (i, name) in names.iter().enumerate() {
                        if self.vars.contains_key(name) {
                            return log::error!(
                                &format!("variable already defined: {name}"),
                                "attempting to define variable in function header"
                            );
                        }
                        if self.is_entrypoint {
                            if i == 0 {
                                self.vars.insert(
                                    name.clone(),
                                    Var {
                                        index: None,
                                        location: VarLocation::Static,
                                        value: vec![E::from(self.input_len as u128)].into(),
                                    },
                                );
                            } else {
                                if i - 1 >= self.static_args.len() {
                                    log::error!(&format!(
                                        "Expected {} static args, got {}",
                                        names.len() - 1,
                                        self.static_args.len()
                                    ))?;
                                }
                                self.named_static_args += 1;
                                self.vars.insert(
                                    name.clone(),
                                    Var {
                                        index: None,
                                        location: VarLocation::Static,
                                        value: self.static_args[i - 1].clone(),
                                    },
                                );
                            }
                        } else {
                            self.vars.insert(name.clone(), self.args[i].clone());
                        }
                    }
                }
                AstNode::Rtrn(expr) => {
                    self.compiler_state
                        .messages
                        .insert(0, format!("return call in {}()", self.name));
                    if self.return_val.is_some() {
                        return log::error!(
                            "return value already set",
                            "you likely have called return more than once"
                        );
                    }
                    self.return_val = Some(self.eval(&expr)?);
                }
                AstNode::ExprUnassigned(expr) => {
                    self.compiler_state
                        .messages
                        .insert(0, "unassigned expression".to_string());
                    self.eval(&expr)?;
                }
                AstNode::Precompile(name, args, body_maybe) => {
                    match name.as_str() {
                        "loop" => {
                            if args.len() != 1 {
                                anyhow::bail!(
                                    "ashlang: loop precompile expects exactly 1 argument. Got {}",
                                    args.len()
                                )
                            }
                            let expr = &args[0];
                            let body = body_maybe.ok_or(anyhow::anyhow!(
                                "ashlang: loop precompile expects a body"
                            ))?;
                            self.compiler_state
                                .messages
                                .insert(0, "loop condition".to_string());
                            let var = self.eval(&expr)?;
                            if var.location != VarLocation::Static {
                                return log::error!("loop condition must be static variable");
                            }
                            let loop_count = if let Some(v) = var.scalar_maybe() {
                                v.into() as usize
                            } else {
                                return log::error!(&format!(
                                    "loop condition must be a scalar, received a vector of length {}",
                                    var.value.len()
                                ));
                            };
                            // track the old variables, delete any variables
                            // created inside the loop body
                            let old_vars = self.vars.clone();
                            let mut i = E::zero().into() as usize;
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
                        "write_output" => {
                            if args.len() != 1 {
                                anyhow::bail!(
                                    "ashlang: write_output precompile expects exactly 1 argument. Got {}",
                                    args.len()
                                )
                            }
                            let expr = &args[0];
                            if body_maybe.is_some() {
                                anyhow::bail!(
                                    "ashlang: write_output precompile may not contain a body"
                                );
                            }
                            let var = self.eval(expr)?;
                            if var.location == VarLocation::Static {
                                anyhow::bail!(
                                    "ashlang: write_output precompile refusing to write a static variable to output"
                                )
                            }
                            for (i, _v) in var.value.iter().enumerate() {
                                self.constraints.push(Constraint::symbolic(
                                    var.index.unwrap() + i,
                                    vec![],
                                    vec![],
                                    SymbolicOp::Output,
                                    "write_output precompile invocation".to_string(),
                                ));
                            }
                        }
                        "assert_eq" => {
                            // assert equality of 2 witness variables
                            if args.len() != 2 {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile expects exactly 2 arguments. Got {}",
                                    args.len()
                                )
                            }
                            if body_maybe.is_some() {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile may not contain a body"
                                );
                            }
                            let lhs = self.eval(&args[0])?;
                            let rhs = self.eval(&args[1])?;
                            if lhs.location == VarLocation::Static {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile refusing to operate on static lhs"
                                )
                            }
                            if rhs.location == VarLocation::Static {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile refusing to operate on static rhs"
                                )
                            }
                            if lhs.value.len() != rhs.value.len() {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile failed, lhs and rhs are of different dimension: {}, {}",
                                    lhs.value.len(),
                                    rhs.value.len()
                                )
                            }
                            let left_i = lhs.index.unwrap();
                            let right_i = rhs.index.unwrap();
                            for (i, (_lv, _rv)) in
                                lhs.value.iter().zip(rhs.value.iter()).enumerate()
                            {
                                self.constraints.push(Constraint::new(
                                    vec![(1.into(), left_i + i)],
                                    vec![(1.into(), 0)],
                                    vec![(1.into(), right_i + i)],
                                    "assert_eq constraint",
                                ));
                            }
                        }
                        _ => anyhow::bail!("ashlang: unsupported precompile: {}", name),
                    }
                }
                AstNode::EmptyVecDef(location, name, index) => {
                    // initialize a vector of witness elements to zero
                    self.compiler_state
                        .messages
                        .insert(0, "vector init".to_string());
                    let len = self.eval(&index)?;
                    match len.location {
                        VarLocation::Constraint => {
                            anyhow::bail!("ashlang: cannot index vector with witness value")
                        }
                        _ => {}
                    }
                    let len = match len.scalar_maybe() {
                        Some(l) => l.into() as usize,
                        None => anyhow::bail!("ashlang: empty vec def non-scalar"),
                    };
                    match location {
                        VarLocation::Constraint => {
                            let var = Var {
                                index: Some(self.var_index),
                                location,
                                value: Vector::new(len),
                            };
                            self.var_index += len;
                            self.vars.insert(name, var);
                        }
                        VarLocation::Static => {
                            let var = Var {
                                index: None,
                                location,
                                value: Vector::new(len),
                            };
                            self.vars.insert(name, var);
                        }
                    }
                }
                AstNode::AssignVec(name, index_maybe, expr) => {
                    // this may be a matrix or a vector
                    let rhs = self.eval(&expr)?;
                    let index_maybe = if let Some(index_expr) = index_maybe {
                        let index = self.eval(&index_expr)?;
                        match (&index.location, index.is_scalar()) {
                            (VarLocation::Static, true) => {
                                Some(index.scalar_maybe().unwrap().into() as usize)
                            }
                            _ => panic!(),
                        }
                    } else {
                        None
                    };

                    let lhs = self.vars.get(&name).unwrap().clone();
                    match (&lhs.location, index_maybe, &rhs.location) {
                        (VarLocation::Static, Some(lhs_index), VarLocation::Static) => {
                            // assigning a specific index of the lhs, which is a static
                            if let Some(rhs_scalar) = rhs.scalar_maybe() {
                                let lhs_mut = self.vars.get_mut(&name).unwrap();
                                lhs_mut.value[lhs_index] = rhs_scalar;
                            } else {
                                anyhow::bail!("ashlang: cannot assign non-scalar to vector index!")
                            }
                        }
                        (VarLocation::Static, None, VarLocation::Static) => {
                            // assigning a vector to the lhs
                            if lhs.value.len() != rhs.value.len() {
                                anyhow::bail!(
                                    "ashlang: dimension mismatch in vector assignment. Trying to assign vector of len {} to {} which is of len {}",
                                    rhs.value.len(),
                                    name,
                                    lhs.value.len()
                                )
                            }
                            let lhs_mut = self.vars.get_mut(&name).unwrap();
                            lhs_mut.value = rhs.value;
                        }
                        (VarLocation::Constraint, Some(index), VarLocation::Constraint) => {
                            if let Some(_rhs_scalar) = rhs.scalar_maybe() {
                                self.assignment_constraint(
                                    lhs.index.unwrap() + index as usize,
                                    rhs.index.unwrap(),
                                );
                            } else {
                                anyhow::bail!(
                                    "ashlang: cannot assign non-scalar to constraint vector index!"
                                )
                            }
                        }
                        (VarLocation::Constraint, None, VarLocation::Constraint) => {
                            // vector assignment
                            assert_eq!(
                                lhs.value.len(),
                                rhs.value.len(),
                                "ashlang: vector assignment failed, dimension mismatch"
                            );
                            let lhs_index = lhs.index.unwrap();
                            let rhs_index = rhs.index.unwrap();
                            for i in 0..lhs.value.len() {
                                self.assignment_constraint(lhs_index + i, rhs_index + i);
                            }
                        }

                        _ => anyhow::bail!(
                            "ashlang: Unsupported combination of assignment: ({:?}, {:?}, {:?}): {:?}",
                            lhs.location,
                            index_maybe,
                            rhs.location,
                            expr
                        ),
                    }
                }
                _ => {
                    return log::error!(&format!("ast node not supported for r1cs: {:?}", v));
                }
            }
        }
        if self.is_entrypoint && self.named_static_args != self.static_args.len() {
            // TODO: show way more information
            // what statics were defined in src, what values were provided
            // same the symmetric logic above
            anyhow::bail!(
                "ashlang: not all static args were used. Received {} statics but only named {} (not including input_len). Silence this using {} as your entrypoint signature",
                self.static_args.len(),
                self.named_static_args,
                vec![
                    "(input_len, ",
                    &vec!["_, "; self.static_args.len() - 1].join(""),
                    "_)"
                ]
                .join("")
            )
        }
        Ok(())
    }

    /// Serialization to/from AST representation
    pub fn build_var_from_ast_vec(&mut self, expr: &Expr) -> Result<(Vec<usize>, Vec<E>)> {
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
        let mut vec_rep: Vec<E> = Vec::new();
        Self::extract_literals(expr, &mut vec_rep)?;
        Ok((dimensions, vec_rep))
    }

    fn extract_literals(expr: &Expr, out: &mut Vec<E>) -> Result<()> {
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
                    .map(|v| Ok(E::from(u128::from_str(v)?)))
                    .collect::<Result<_>>()?;
                out.append(&mut vv);
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn assignment_constraint(&mut self, lhs_i: usize, rhs_i: usize) {
        self.constraints.push(Constraint::new(
            vec![(E::one(), lhs_i)],
            vec![(E::one(), 0)],
            vec![(E::one(), rhs_i)],
            &format!("equality constraint {} = {}", lhs_i, rhs_i),
        ));
        self.constraints.push(Constraint::symbolic(
            lhs_i,
            vec![(E::one(), 0)],
            vec![(E::one(), rhs_i)],
            SymbolicOp::Mul,
            format!("scalar equality assignment ({}) <- {}", lhs_i, rhs_i),
        ));
    }

    fn static_assignment_constraint(&mut self, lhs_i: usize, rhs_v: E) {
        self.constraints.push(Constraint::new(
            vec![(rhs_v, 0)],
            vec![(E::one(), 0)],
            vec![(E::one(), lhs_i)],
            &format!("static assignment ({}) <- {}", lhs_i, rhs_v),
        ));
        self.constraints.push(Constraint::symbolic(
            lhs_i,
            vec![(rhs_v, 0)],
            vec![(E::one(), 0)],
            SymbolicOp::Mul,
            format!("static assignment ({}) <- {}", lhs_i, rhs_v),
        ));
    }

    /// Used for constants and statics.
    fn spawn_known_variable(&mut self, val: E) {
        self.constraints.push(Constraint::new(
            vec![(E::one(), self.var_index)],
            vec![(E::one(), 0)],
            vec![(val, 0)],
            &format!(
                "scalar literal ({}) to signal index ({})",
                val, self.var_index,
            ),
        ));
        self.constraints.push(Constraint::symbolic(
            self.var_index,
            vec![(E::one(), 0)],
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
    fn static_to_constraint(&mut self, var: &Var<E>) -> Result<Var<E>> {
        let index = self.var_index;
        match var.scalar_maybe() {
            Some(v) => {
                self.spawn_known_variable(v);
            }
            None => {
                for v in &var.value {
                    self.spawn_known_variable(*v);
                }
            }
        }
        Ok(Var {
            index: Some(index),
            location: VarLocation::Constraint,
            value: var.value.clone(),
        })
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Var<E>> {
        match &expr {
            Expr::VecVec(_) | Expr::VecLit(_) => {
                let (dimensions, values) = self.build_var_from_ast_vec(expr)?;
                if dimensions.len() > 2 {
                    anyhow::bail!("Hypercube structures are not supported.");
                }
                Ok(Var {
                    index: None,
                    location: VarLocation::Static,
                    value: values.into(),
                })
            }
            Expr::FnCall(name, vars) => {
                let args: Vec<Var<E>> = vars.iter().map(|v| self.eval(v)).collect::<Result<_>>()?;
                if let Some(ash_parser) = self.compiler_state.fn_ash_maybe(name) {
                    let mut vm = VM::from(self, args, name);
                    vm.eval_ast(ash_parser.ast)?;
                    let return_val = vm.return_val;
                    let new_var_index = vm.var_index;
                    let mut out_constraints = vm.constraints;
                    self.constraints.append(&mut out_constraints);
                    self.var_index = new_var_index;
                    return if let Some(v) = return_val {
                        Ok(v)
                    } else {
                        Ok(Var {
                            index: None,
                            location: VarLocation::Static,
                            value: vec![1.into()].into(),
                        })
                    };
                }
                anyhow::bail!("ashlang: unknown function: {name}");
            }
            Expr::Val(name, index_maybe) => {
                let index_maybe = match index_maybe {
                    Some(index_expr) => {
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
                        Some(v.value[0].into() as usize)
                    }
                    None => None,
                };
                let v = self.vars.get(name);
                if v.is_none() {
                    return log::error!(&format!("variable not found: {name}"));
                }
                let v = v.unwrap();
                match (&v.location, index_maybe) {
                    (VarLocation::Static, Some(index)) => {
                        if index >= v.value.len() {
                            anyhow::bail!(
                                "ashlang: Attempting to access index {} of static \"{}\" which has {} elements",
                                index,
                                name,
                                v.value.len()
                            )
                        }
                        Ok(Var {
                            index: None,
                            location: VarLocation::Static,
                            value: vec![v.value[index]].into(),
                        })
                    }
                    (VarLocation::Static, None) => Ok(Var {
                        index: None,
                        location: VarLocation::Static,
                        value: v.value.clone(),
                    }),
                    (VarLocation::Constraint, Some(index)) => {
                        if index >= v.value.len() {
                            anyhow::bail!(
                                "ashlang: Attempting to access index {} of variable \"{}\" which has {} elements",
                                index,
                                name,
                                v.value.len()
                            )
                        }
                        Ok(Var {
                            index: Some(v.index.unwrap() + index),
                            location: VarLocation::Constraint,
                            value: vec![v.value[index]].into(),
                        })
                    }
                    (VarLocation::Constraint, None) => Ok(Var {
                        index: Some(v.index.unwrap()),
                        location: VarLocation::Constraint,
                        value: v.value.clone(),
                    }),
                }
            }
            Expr::NumOp { lhs, op, rhs } => self.eval_numop(lhs, op, rhs),
            Expr::Lit(val) => Ok(Var {
                index: None,
                location: VarLocation::Static,
                value: vec![E::from(u128::from_str(val)?)].into(),
            }),
            Expr::Precompile(name, args, block_maybe) => match name.as_str() {
                "read_input" => {
                    if args.len() != 1 {
                        anyhow::bail!(
                            "ashlang: read_input precompile expects exactly 1 argument. Got {}",
                            args.len()
                        )
                    }
                    let expr = &args[0];
                    if block_maybe.is_some() {
                        anyhow::bail!("ashlang: read_input precompile may not contain a body");
                    }
                    let var = self.eval(expr)?;
                    if var.location != VarLocation::Static {
                        anyhow::bail!("ashlang: read_input precompile argument must be a static")
                    }
                    let read_count = if let Some(v) = var.scalar_maybe() {
                        v.into() as usize
                    } else {
                        anyhow::bail!(
                            "ashlang: read_input precompile argument must be a vector of length 1, received vector of len: {}",
                            var.value.len()
                        )
                    };
                    let out = Var {
                        index: Some(self.var_index),
                        location: VarLocation::Constraint,
                        value: Vector::new(read_count),
                    };
                    self.var_index += read_count;
                    for i in 0..read_count {
                        self.constraints.push(Constraint::symbolic(
                            out.index.unwrap() + i,
                            vec![],
                            vec![],
                            SymbolicOp::Input,
                            "read_input recompile invocation".to_string(),
                        ));
                    }
                    Ok(out)
                }
                _ => anyhow::bail!("ashlang: Unknown precompile in expression: {}", name),
            },
            _ => {
                log::error!("unimplemented expression case")
            }
        }
    }

    // handle the following cases
    // 1. lhs and rhs are both constraint variables
    // 2. lhs is a constraint variable and rhs is a static (and vis-versa)
    // 3. lhs and rhs are both static variables
    fn eval_numop(&mut self, lhs: &Expr, op: &NumOp, rhs: &Expr) -> Result<Var<E>> {
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

    fn eval_numop_static(&mut self, lv: &Var<E>, op: &NumOp, rv: &Var<E>) -> Result<Var<E>> {
        Ok(match op {
            NumOp::Add => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() + &rv.value,
            },
            NumOp::Mul => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() * &rv.value,
            },
            NumOp::Sub => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone() - &rv.value,
            },
            NumOp::Inv => Var {
                index: None,
                location: VarLocation::Static,
                value: lv.value.clone()
                    * &rv
                        .value
                        .clone()
                        .into_iter()
                        .map(|v| v.inverse())
                        .collect::<Vector<_>>(),
            },
        })
    }

    fn eval_numop_signal_static(&mut self, lv: &Var<E>, op: &NumOp, rv: &Var<E>) -> Result<Var<E>> {
        assert_ne!(
            matches!(lv.location, VarLocation::Constraint),
            matches!(rv.location, VarLocation::Constraint),
            "ashlang: numop should be mixed"
        );
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
                    value: lv.value.clone() + &rv.value,
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let svi = signal_v.index.unwrap() + x;
                    // the coefficient value, not a signal index
                    let cv = static_v.value[x].clone();
                    let ovi = new_var.index.unwrap() + x;
                    // (cv*1 + 1*svi)*(1*1) - (1*ovi) = 0
                    // cv + sv - ov = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(cv, 0), (E::one(), svi)],
                            vec![(E::one(), 0)],
                            vec![(E::one(), ovi)],
                            &format!("addition between ({cv}) and {svi} into {ovi}"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(cv, 0), (E::one(), svi)],
                            vec![(E::one(), 0)],
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
                    value: lv.value.clone() * &rv.value,
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let svi = signal_v.index.unwrap() + x;
                    // the coefficient value, not a signal index
                    let cv = static_v.value[x].clone();
                    let ovi = new_var.index.unwrap() + x;
                    // (cv*1)*(1*svi) - (1*ovi) = 0
                    // cv * sv - ov = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(cv, 0)],
                            vec![(E::one(), svi)],
                            vec![(E::one(), ovi)],
                            &format!("multiplication between ({cv}) and {svi} into {ovi}"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(cv, 0)],
                            vec![(E::one(), svi)],
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
                        let cv = rv.value[x].clone();
                        let ovi = new_var.index.unwrap() + x;
                        // (cv*1 + 1*ovi)*(1*1) - (1*lvi) = 0
                        // (cv + ovi)*1 - lvi = 0
                        self.constraints.append(&mut vec![
                            Constraint::new(
                                vec![(cv, 0), (E::one(), ovi)],
                                vec![(E::one(), 0)],
                                vec![(E::one(), lvi)],
                                &format!("subtraction between {lvi} and ({cv}) into {ovi}"),
                            ),
                            Constraint::symbolic(
                                ovi,
                                vec![(E::one(), lvi), (E::negone() * cv, 0)],
                                vec![(E::one(), 0)],
                                SymbolicOp::Mul,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                    }
                } else {
                    // subtracting a signal from a static
                    for x in 0..new_var.value.len() {
                        let lv = lv.value[x].clone();
                        // the static being subtracted
                        let rvi = rv.index.unwrap() + x;
                        let ovi = new_var.index.unwrap() + x;
                        // (1*rvi + 1*ovi)*(1*1) - (lv*1) = 0
                        // (rvi + ovi)*1 - lv = 0
                        self.constraints.append(&mut vec![
                            Constraint::new(
                                vec![(E::one(), rvi), (E::one(), ovi)],
                                vec![(E::one(), 0)],
                                vec![(lv, 0)],
                                &format!("subtraction between ({lv}) and {rvi} into {ovi}"),
                            ),
                            Constraint::symbolic(
                                ovi,
                                vec![(lv, 0), (E::negone(), rvi)],
                                vec![(E::one(), 0)],
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
                    value: lv.value.clone()
                        * &rv.value.iter().map(|v| v.inverse()).collect::<Vector<_>>(),
                };
                self.var_index += new_var.value.len();
                if lv.location == VarLocation::Constraint {
                    // can statically inv
                    // subtracting a static from a signal
                    for x in 0..new_var.value.len() {
                        let lvi = lv.index.unwrap() + x;
                        // this is a static value
                        let cv = rv.value[x].clone();
                        let icv = cv.inverse();
                        let ovi = new_var.index.unwrap() + x;
                        // (icv*lvi)*(1*1) - (1*ovi) = 0
                        // (icv*lvi)*1 - ovi = 0
                        self.constraints.append(&mut vec![
                            Constraint::new(
                                vec![(icv, lvi)],
                                vec![(E::one(), 0)],
                                vec![(E::one(), ovi)],
                                &format!("modinv between {lvi} and ({cv}) into {ovi}"),
                            ),
                            Constraint::symbolic(
                                ovi,
                                vec![(icv, lvi)],
                                vec![(E::one(), 0)],
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
                        value: rv.value.iter().map(|v| v.inverse()).collect(),
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
                            Constraint::new(
                                vec![(E::one(), rvi)],
                                vec![(E::one(), ovi)],
                                vec![(E::one(), 0)],
                                &format!("modinv {rvi} into {ovi}"),
                            ),
                            Constraint::symbolic(
                                ovi,
                                vec![(E::one(), 0)],
                                vec![(E::one(), rvi)],
                                SymbolicOp::Inv,
                                self.compiler_state.messages[0].clone(),
                            ),
                        ]);
                        // now constrain the new_var
                        let lv = lv.value[x].clone();
                        let rvi = inv_var.index.unwrap() + x;
                        let ovi = new_var.index.unwrap() + x;
                        // (lv*rvi)*(1*1) - (1*ovi) = 0
                        // (lv*rvi)*1 - ovi = 0
                        self.constraints.append(&mut vec![
                            Constraint::new(
                                vec![(lv, rvi)],
                                vec![(E::one(), 0)],
                                vec![(E::one(), ovi)],
                                &format!("multiply {rvi} and ({lv}) into {ovi}"),
                            ),
                            Constraint::symbolic(
                                ovi,
                                vec![(lv, rvi)],
                                vec![(E::one(), 0)],
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

    fn eval_numop_signals(&mut self, lv: &Var<E>, op: &NumOp, rv: &Var<E>) -> Result<Var<E>> {
        assert!(
            matches!(lv.location, VarLocation::Constraint),
            "non-constraint lv"
        );
        assert!(
            matches!(rv.location, VarLocation::Constraint),
            "non-constraint rv"
        );

        // take a lhs and rhs of variable size and apply
        // an operation to each element
        Ok(match op {
            NumOp::Add => {
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() + &rv.value,
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = rv.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv + 1*rv) * (1*1) - (1*new_var) = 0
                    // lv + rv - new_var = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(E::one(), lvi), (E::one(), rvi)],
                            vec![(E::one(), 0)],
                            vec![(E::one(), ovi)],
                            &format!("addition between {lvi} and {rvi} into {ovi}"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(E::one(), lvi), (E::one(), rvi)],
                            vec![(E::one(), 0)],
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
                    value: lv.value.clone() * &rv.value,
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = rv.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv + 1*rv) * (1*1) - (1*new_var) = 0
                    // lv + rv - new_var = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(E::one(), lvi)],
                            vec![(E::one(), rvi)],
                            vec![(E::one(), ovi)],
                            &format!("multiplication between {lvi} and {rvi} into {ovi}"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(E::one(), lvi)],
                            vec![(E::one(), rvi)],
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
                    value: lv.value.clone() + &(rv.value.clone() * &vec![E::negone()].into()),
                };
                self.var_index += new_var.value.len();
                for x in 0..new_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = rv.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv + -1*rv) * (1*1) - (1*new_var) = 0
                    // lv + -1*rv - new_var = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(E::one(), lvi), (E::negone(), rvi)],
                            vec![(E::one(), 0)],
                            vec![(E::one(), ovi)],
                            &format!("subtraction between {lvi} and {rvi} into {ovi}"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(E::one(), lvi), (E::negone(), rvi)],
                            vec![(E::one(), 0)],
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
                    value: rv.value.iter().map(|v| v.inverse()).collect(),
                };
                self.var_index += inv_var.value.len();
                for x in 0..inv_var.value.len() {
                    let rvi = rv.index.unwrap() + x;
                    let ovi = inv_var.index.unwrap() + x;
                    // first: constrain rhs_inv
                    // (1*rhs) * (1*rhs_inv) - (1*1) = 0
                    // rhs * rhs_inv - 1 = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(E::one(), rvi)],
                            vec![(E::one(), ovi)],
                            vec![(E::one(), 0)],
                            &format!("inversion of {rvi} into {ovi} (1/2)"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(E::one(), 0)],
                            vec![(E::one(), rvi)],
                            SymbolicOp::Inv,
                            self.compiler_state.messages[0].clone(),
                        ),
                    ]);
                }
                // then multiply rv_inv by the lhs
                let new_var = Var {
                    index: Some(self.var_index),
                    location: VarLocation::Constraint,
                    value: lv.value.clone() * &inv_var.value,
                };
                self.var_index += new_var.value.len();
                for x in 0..inv_var.value.len() {
                    let lvi = lv.index.unwrap() + x;
                    let rvi = inv_var.index.unwrap() + x;
                    let ovi = new_var.index.unwrap() + x;
                    // (1*lv) * (1*rv) - (1*new_var) = 0
                    // lv * rv - new_var = 0
                    self.constraints.append(&mut vec![
                        Constraint::new(
                            vec![(E::one(), lvi)],
                            vec![(E::one(), rvi)],
                            vec![(E::one(), ovi)],
                            &format!("multiplication of {lvi} and {rvi} into {ovi} (2/2)"),
                        ),
                        Constraint::symbolic(
                            ovi,
                            vec![(E::one(), lvi)],
                            vec![(E::one(), rvi)],
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
