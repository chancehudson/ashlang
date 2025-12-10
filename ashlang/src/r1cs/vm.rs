use anyhow::Context;

use super::*;

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
                // an assigment like
                // # ("x", Some(VarLocation::Witness), Expr::Lit(100))
                // let x = 100
                // # ("y", Some(VarLocation::Static), Expr::Lit(50))
                // static y = 50
                AstNode::Stmt(name, location_maybe, expr) => {
                    // log stuff needs fixing zzzz
                    if matches!(location_maybe, Some(VarLocation::Static)) {
                        self.compiler_state
                            .messages
                            .insert(0, format!("static {name}"));
                    } else if matches!(location_maybe, Some(VarLocation::Witness)) {
                        self.compiler_state
                            .messages
                            .insert(0, format!("let {name}"));
                    } else {
                        self.compiler_state
                            .messages
                            .insert(0, format!("re-assign {name}"));
                    }

                    // evaluate the expression rhs to a Var
                    let rhs = self.eval(&expr)?;

                    if let Some(location) = location_maybe {
                        // variable creation
                        match (location, &rhs) {
                            (VarLocation::Static, Var::Static { value }) => {
                                self.vars.insert(
                                    name,
                                    Var::Static {
                                        value: value.clone(),
                                    },
                                );
                            }
                            (VarLocation::Witness, Var::Static { .. }) => {
                                // assigning a static to a witness variable. Constrain equality
                                // using coefficients in the r1cs
                                let new_var = self.static_to_constraint(&rhs)?;
                                self.vars.insert(name, new_var);
                            }
                            (VarLocation::Witness, Var::Witness { index, len }) => {
                                // # assignment of witness vars
                                // let x = 0
                                // let y = x
                                self.vars.insert(
                                    name,
                                    Var::Witness {
                                        index: *index,
                                        len: *len,
                                    },
                                );
                            }
                            (VarLocation::Static, Var::Witness { .. }) => {
                                anyhow::bail!(
                                    "ashlang: Attempting to create static variable {name} from witness expr: {:?}",
                                    expr
                                );
                            }
                        }
                    } else {
                        // variable re-assignment
                        // if location is None the variable must be in scope
                        if !self.vars.contains_key(&name) {}
                        let lhs = self
                            .vars
                            .get_mut(&name)
                            .ok_or(anyhow::anyhow!("variable does not exist in scope: {name}"))?;
                        if rhs.len() != lhs.len() {
                            anyhow::bail!(
                                "ashlang: Attempting to assign vectors of mismatched dimension: lhs: {} rhs: {} ",
                                lhs.len(),
                                rhs.len()
                            )
                        }

                        match (lhs, &rhs) {
                            (
                                Var::Static { value: lhs_value },
                                Var::Static { value: rhs_value },
                            ) => {
                                *lhs_value = rhs_value.clone();
                            }
                            (Var::Witness { .. }, Var::Static { .. }) => {
                                // assigning a static to a witness variable. Constrain equality
                                // using coefficients in the r1cs
                                let new_var = self.static_to_constraint(&rhs)?;
                                self.vars.insert(name, new_var);
                            }
                            (
                                Var::Witness {
                                    index: lhs_index, ..
                                },
                                Var::Witness {
                                    index: rhs_index, ..
                                },
                            ) => {
                                // copy by value+pointer. Constraints are implicitly copied and
                                // equality is implied
                                *lhs_index = *rhs_index;
                            }
                            (Var::Static { .. }, Var::Witness { .. }) => {
                                anyhow::bail!(
                                    "ashlang: Attempting to assign to static var from witness var: {name}"
                                )
                            }
                        }
                    }
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
                                    Var::Static {
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
                                    Var::Static {
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
                    let v = self.eval(&expr)?;
                    self.return_val = Some(v);
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
                            if var.location() != VarLocation::Static {
                                return log::error!("loop condition must be static variable");
                            }
                            let loop_count = var
                                .scalar_static_value()
                                .with_context(|| "In invocation of loop keyword argument")?
                                .into() as usize;
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
                            match var {
                                Var::Witness { index, .. } => {
                                    for i in 0..var.len() {
                                        self.constraints.push(Constraint::symbolic(
                                            index + i,
                                            vec![],
                                            vec![],
                                            SymbolicOp::Output,
                                            "write_output precompile invocation".to_string(),
                                        ));
                                    }
                                }
                                Var::Static { .. } => {
                                    anyhow::bail!(
                                        "ashlang: write_output precompile refusing to write a static variable to output"
                                    )
                                }
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
                            if lhs.location() == VarLocation::Static {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile refusing to operate on static lhs"
                                )
                            }
                            if rhs.location() == VarLocation::Static {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile refusing to operate on static rhs"
                                )
                            }
                            if lhs.len() != rhs.len() {
                                anyhow::bail!(
                                    "ashlang: assert_eq precompile failed, lhs and rhs are of different dimension: {}, {}",
                                    lhs.len(),
                                    rhs.len()
                                )
                            }
                            let left_i = lhs.wtns_index()?;
                            let right_i = rhs.wtns_index()?;
                            if left_i == right_i {
                                anyhow::bail!(
                                    "ashlang::assert_eq: Refusing to assert equality between a witness variable and itself:\nlhs: {:?}\nrhs: {:?}",
                                    args[0],
                                    args[1]
                                )
                            }
                            for i in 0..lhs.len() {
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
                    if len.location() == VarLocation::Witness {
                        anyhow::bail!("ashlang: cannot index vector with witness value")
                    }
                    let len = len
                        .scalar_static_value()
                        .with_context(|| "vm::AstNode::EmptyVecDef")?
                        .into() as usize;
                    match location {
                        VarLocation::Witness => {
                            let var = Var::Witness {
                                index: self.var_index,
                                len,
                            };
                            self.var_index += len;
                            self.vars.insert(name, var);
                        }
                        VarLocation::Static => {
                            let var = Var::Static {
                                value: Vector::new(len),
                            };
                            self.vars.insert(name, var);
                        }
                    }
                }
                AstNode::AssignVar(_name, _expr) => {
                    unimplemented!()
                }
                AstNode::AssignVarIndex(name, index_expr, expr) => {
                    let index = self.eval(&index_expr)?;
                    if index.location() != VarLocation::Static {
                        anyhow::bail!("ashlang: Vector index assignment, index is not static!")
                    }
                    if !index.is_scalar() {
                        anyhow::bail!("ashlang: Vector index assignment, index is not scalar!")
                    }
                    let index = index.scalar_static_value()?.into() as usize;

                    let lhs = self.vars.get(&name).unwrap().clone();

                    if lhs.len() <= index {
                        anyhow::bail!(
                            "ashlang: Attempting to assign index {} in vector of len {}",
                            index,
                            lhs.len()
                        )
                    }
                    // lhs is consistent, get our rhs
                    let rhs = self.eval(&expr)?;
                    if !rhs.is_scalar() {
                        anyhow::bail!(
                            "ashlang: Attempting to assign non-scalar expression to index {index} of vector {name}"
                        );
                    }
                    match (lhs, rhs) {
                        (Var::Static { .. }, Var::Static { value: rhs_value }) => {
                            let lhs_mut = self.vars.get_mut(&name).unwrap();
                            match lhs_mut {
                                Var::Static { value } => {
                                    value[index] = rhs_value[0];
                                }
                                Var::Witness { .. } => unreachable!(),
                            }
                        }
                        (Var::Witness { index: wtns_i, .. }, Var::Static { value }) => {
                            self.static_assignment_constraint(wtns_i + index, value[0]);
                        }
                        (Var::Witness { .. }, Var::Witness { .. }) => {
                            // need sparse vectors for this
                            anyhow::bail!(
                                "ashlang: witness index assignment from witness unsupported"
                            )
                        }
                        _ => unreachable!(),
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
        let root_expr = expr.clone();
        // first calculate the dimensions of the matrix
        loop {
            match root_expr {
                // Expr::VecVec(v) => {
                //     dimensions.push(v.len());
                //     root_expr = v[0].clone();
                // }
                Expr::VecLit(v) => {
                    dimensions.push(v.len());
                    break;
                }
                _ => {}
            }
        }
        // then pull all the literals into a 1 dimensional vec
        let mut vec_rep: Vec<E> = Vec::new();
        self.extract_literals(expr, &mut vec_rep)?;
        Ok((dimensions, vec_rep))
    }

    fn extract_literals(&mut self, expr: &Expr, out: &mut Vec<E>) -> Result<()> {
        match expr {
            // Expr::VecVec(v) => {
            //     for a in v {
            //         Self::extract_literals(a, out)?
            //     }
            // }
            Expr::VecLit(v) => {
                for expr in v {
                    println!("{:?}", expr);
                    let var = self.eval(&expr)?;
                    if var.location() != VarLocation::Static {
                        anyhow::bail!("ashlang: Vector literal must have only static expressions")
                    }
                    if !var.is_scalar() {
                        anyhow::bail!("ashlang: Vector literal must have only scalar expressions")
                    }
                    out.push(var.scalar_static_value()?);
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Constraint equality between two indices in the witness
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
                for v in var.static_value()? {
                    self.spawn_known_variable(*v);
                }
            }
        }
        Ok(Var::Witness {
            index,
            len: var.len(),
        })
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Var<E>> {
        match &expr {
            /*Expr::VecVec(_) |*/
            Expr::VecLit(exprs) => {
                let (dimensions, values) = self.build_var_from_ast_vec(expr)?;
                println!("{}", Into::<Vector<E>>::into(values.clone()));
                if dimensions.len() > 1 {
                    anyhow::bail!("Only vectors are supported.");
                }
                Ok(Var::Static {
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
                        Ok(Var::Static {
                            value: vec![1.into()].into(),
                        })
                    };
                }
                anyhow::bail!("ashlang: unknown function: {name}");
            }
            Expr::ValVar(name) => {
                // getting a variable by name
                match self.vars.get(name) {
                    Some(var) => Ok(var.clone()),
                    None => anyhow::bail!(
                        "ashlang: Attempting to access variable {} which is not in scope",
                        name
                    ),
                }
            }
            Expr::ValVarIndex(name, index) => {
                let index_var = self.eval(&index)?;
                if index_var.location() != VarLocation::Static {
                    anyhow::bail!(
                        "ashlang: Attempting to index a variable {name} with a non-static"
                    )
                }
                if !index_var.is_scalar() {
                    anyhow::bail!(
                        "ashlang: Attempting to index a variable {name} with a non-scalar"
                    )
                }
                let index = index_var.static_value()?[0].into() as usize;
                // getting a value by name and index
                let v = self.vars.get(name);
                if v.is_none() {
                    return log::error!(&format!("variable not found: {name}"));
                }
                let var = v.unwrap();
                if var.len() <= index {
                    anyhow::bail!(
                        "ashlang: Attempting to access index {index} of variable {name} which is of len: {}",
                        var.len()
                    )
                }
                match var {
                    Var::Static { value } => Ok(Var::Static {
                        value: vec![value[index]].into(),
                    }),
                    Var::Witness {
                        index: var_index,
                        len: _,
                    } => {
                        // constrain an equality to a new witness scalar
                        self.assignment_constraint(self.var_index, var_index + index);
                        let var = Var::Witness {
                            index: self.var_index,
                            len: 1,
                        };
                        self.var_index += 1;
                        Ok(var)
                    }
                }
            }
            Expr::NumOp { lhs, op, rhs } => self.eval_numop(lhs, op, rhs),
            Expr::DecLit(val) => Ok(Var::Static {
                value: vec![E::from(u128::from_str(val)?)].into(),
            }),
            Expr::HexLit(val) => Ok(Var::Static {
                value: vec![E::from(
                    u128::from_str_radix(val, 16)
                        .with_context(|| format!("ashlang: HexLit: \"{val}\""))?,
                )]
                .into(),
            }),
            Expr::Precompile(name, args, block_maybe) => match name.as_str() {
                "len" => {
                    if args.len() != 1 {
                        anyhow::bail!(
                            "ashlang: len precompile expects exactly 1 argument. Got {}",
                            args.len()
                        )
                    }
                    let v = self.eval(&args[0])?;
                    Ok(Var::Static {
                        value: vec![(v.len() as u128).into()].into(),
                    })
                }
                "div_floor" => {
                    if args.len() != 2 {
                        anyhow::bail!(
                            "ashlang: div_floor precompile expects exactly 2 static argument. Got {}",
                            args.len()
                        )
                    }
                    let lhs = self.eval(&args[0])?;
                    let rhs = self.eval(&args[1])?;
                    if lhs.location() != VarLocation::Static
                        || rhs.location() != VarLocation::Static
                    {
                        anyhow::bail!("ashlang: div_floor precompile inputs must be static")
                    }
                    if !rhs.is_scalar() || !lhs.is_scalar() {
                        anyhow::bail!("ashlang: div_floor precompile inputs must be scalar")
                    }

                    let lhs = lhs.scalar_static_value()?;
                    let rhs = rhs.scalar_static_value()?;
                    let out = lhs.into() / rhs.into();

                    Ok(Var::Static {
                        value: vec![out.into()].into(),
                    })
                }
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
                    if var.location() != VarLocation::Static {
                        anyhow::bail!("ashlang: read_input precompile argument must be a static")
                    }
                    let read_count = if let Some(v) = var.scalar_maybe() {
                        v.into() as usize
                    } else {
                        anyhow::bail!(
                            "ashlang: read_input precompile argument must be a vector of length 1, received vector of len: {}",
                            var.len()
                        )
                    };
                    let out = Var::Witness {
                        index: self.var_index,
                        len: read_count,
                    };
                    self.var_index += read_count;
                    for i in 0..read_count {
                        self.constraints.push(Constraint::symbolic(
                            out.wtns_index()? + i,
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

        if lv.location() == VarLocation::Witness && rv.location() == VarLocation::Witness {
            // both are signals
            self.eval_numop_signals(&lv, op, &rv)
        } else if lv.location() != rv.location() {
            // one is signal one is static
            self.eval_numop_signal_static(&lv, op, &rv)
        } else {
            // both static
            self.eval_numop_static(&lv, op, &rv)
        }
    }

    fn eval_numop_static(&mut self, lv: &Var<E>, op: &NumOp, rv: &Var<E>) -> Result<Var<E>> {
        Ok(match op {
            NumOp::Add => Var::Static {
                value: lv.static_value()?.clone() + rv.static_value()?,
            },
            NumOp::Mul => Var::Static {
                value: lv.static_value()?.clone() * rv.static_value()?,
            },
            NumOp::Sub => Var::Static {
                value: lv.static_value()?.clone() - rv.static_value()?,
            },
            NumOp::Inv => Var::Static {
                value: lv.static_value()?.clone()
                    * &rv
                        .static_value()?
                        .clone()
                        .into_iter()
                        .map(|v| v.inverse())
                        .collect::<Vector<_>>(),
            },
        })
    }

    fn eval_numop_signal_static(&mut self, lv: &Var<E>, op: &NumOp, rv: &Var<E>) -> Result<Var<E>> {
        assert_ne!(
            matches!(lv.location(), VarLocation::Witness),
            matches!(rv.location(), VarLocation::Witness),
            "ashlang: numop should be mixed"
        );
        let static_v;
        let signal_v;
        if lv.location() == VarLocation::Static {
            static_v = lv;
            signal_v = rv;
        } else {
            static_v = rv;
            signal_v = lv;
        }
        if lv.len() != rv.len() {
            anyhow::bail!(
                "ashlang: Cannot do signal/static operation on vectors of different len: lhs: {}, rhs: {}",
                lv.len(),
                rv.len()
            );
        }
        Ok(match op {
            NumOp::Add => {
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: lv.len(),
                };
                self.var_index += new_var.len();
                for x in 0..new_var.len() {
                    let svi = signal_v.wtns_index()? + x;
                    // the coefficient value, not a signal index
                    let cv = static_v.static_value()?[x].clone();
                    let ovi = new_var.wtns_index()? + x;
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
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: lv.len(),
                };
                self.var_index += new_var.len();
                for x in 0..new_var.len() {
                    let svi = signal_v.wtns_index()? + x;
                    // the coefficient value, not a signal index
                    let cv = static_v.static_value()?[x].clone();
                    let ovi = new_var.wtns_index()? + x;
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
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: lv.len(),
                };
                self.var_index += new_var.len();
                if lv.location() == VarLocation::Witness {
                    // subtracting a static from a signal
                    for x in 0..new_var.len() {
                        let lvi = lv.wtns_index()? + x;
                        // the value being subtracted
                        let cv = rv.static_value()?[x].clone();
                        let ovi = new_var.wtns_index()? + x;
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
                    for x in 0..new_var.len() {
                        let lv = lv.static_value()?[x].clone();
                        // the static being subtracted
                        let rvi = rv.wtns_index()? + x;
                        let ovi = new_var.wtns_index()? + x;
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
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: lv.len(),
                };
                self.var_index += new_var.len();
                if lv.location() == VarLocation::Witness {
                    // can statically inv
                    // subtracting a static from a signal
                    for x in 0..new_var.len() {
                        let lvi = lv.wtns_index()? + x;
                        // this is a static value
                        let cv = rv.static_value()?[x].clone();
                        let icv = cv.inverse();
                        let ovi = new_var.wtns_index()? + x;
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
                    let inv_var = Var::<E>::Witness {
                        index: self.var_index,
                        len: lv.len(),
                    };
                    self.var_index += inv_var.len();
                    for x in 0..inv_var.len() {
                        // first invert into a signal
                        // let lv = lv.value.values[x].clone();
                        let rvi = rv.wtns_index()? + x;
                        let ovi = inv_var.wtns_index()? + x;
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
                        let lv = lv.static_value()?[x].clone();
                        let rvi = inv_var.wtns_index()? + x;
                        let ovi = new_var.wtns_index()? + x;
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
            matches!(lv.location(), VarLocation::Witness),
            "non-witness lv"
        );
        assert!(
            matches!(rv.location(), VarLocation::Witness),
            "non-witness rv"
        );
        if lv.len() != rv.len() {
            anyhow::bail!(
                "ashlang: Attempting to operate on witness variables of different dimension"
            )
        }

        // take a lhs and rhs of variable size and apply
        // an operation to each element
        Ok(match op {
            NumOp::Add => {
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: lv.len(),
                };
                self.var_index += new_var.len();
                for x in 0..new_var.len() {
                    let lvi = lv.wtns_index()? + x;
                    let rvi = rv.wtns_index()? + x;
                    let ovi = new_var.wtns_index()? + x;
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
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: rv.len(),
                };
                self.var_index += new_var.len();
                for x in 0..new_var.len() {
                    let lvi = lv.wtns_index()? + x;
                    let rvi = rv.wtns_index()? + x;
                    let ovi = new_var.wtns_index()? + x;
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
                let new_var = Var::Witness {
                    index: self.var_index,
                    len: rv.len(),
                };
                self.var_index += new_var.len();
                for x in 0..new_var.len() {
                    let lvi = lv.wtns_index()? + x;
                    let rvi = rv.wtns_index()? + x;
                    let ovi = new_var.wtns_index()? + x;
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
                let inv_var = Var::<E>::Witness {
                    index: self.var_index,
                    len: rv.len(),
                };
                self.var_index += inv_var.len();
                for x in 0..inv_var.len() {
                    let rvi = rv.wtns_index()? + x;
                    let ovi = inv_var.wtns_index()? + x;
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
                let new_var: Var<E> = Var::Witness {
                    index: self.var_index,
                    len: inv_var.len(),
                };
                self.var_index += new_var.len();
                for x in 0..inv_var.len() {
                    let lvi = lv.wtns_index()? + x;
                    let rvi = inv_var.wtns_index()? + x;
                    let ovi = new_var.wtns_index()? + x;
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
