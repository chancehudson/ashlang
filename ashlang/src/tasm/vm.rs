use std::cmp;
use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;
use ring_math::PolynomialRingElement;

use crate::compiler::CompilerState;
use crate::log;
use crate::parser::AstNode;
use crate::parser::BoolOp;
use crate::parser::Expr;
use crate::parser::NumOp;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Stack,
    Memory,
    Static,
}

#[derive(Clone, Debug)]
pub struct Var {
    stack_index: Option<usize>,
    block_index: usize,
    location: VarLocation,
    memory_index: Option<usize>,
    // e.g. 2x3x4
    // [
    //   [
    //     [[], [], [], []],
    //     [[], [], [], []],
    //     [[], [], [], []],
    //   ],
    //   [
    //     [[], [], [], []],
    //     [[], [], [], []],
    //     [[], [], [], []],
    //   ]
    // ]
    dimensions: Vec<usize>,
    value: Option<Vec<u64>>,
}

// represents the type of an argument to a function
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArgType {
    pub location: VarLocation,
    pub dimensions: Vec<usize>,
    pub value: Option<Vec<u64>>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FnCall {
    pub name: String,
    pub arg_types: Vec<ArgType>,
    pub return_type: Option<ArgType>,
}

impl FnCall {
    pub fn typed_name(&self) -> String {
        let mut out = self.name.to_owned();
        if !self.arg_types.is_empty() {
            out.push_str("____");
        }
        for arg in &self.arg_types {
            out.push('_');
            if arg.dimensions.is_empty() && arg.value.is_none() {
                out.push('s');
            } else if arg.dimensions.is_empty() && arg.value.is_some() {
                out.push_str(
                    &arg.value
                        .clone()
                        .unwrap()
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            for d_index in 0..arg.dimensions.len() {
                if d_index > 0 {
                    out.push('x');
                }
                out.push_str(arg.dimensions[d_index].to_string().as_str());
            }
        }
        out
    }
}

static RETURN_VAR: &str = "_____return_____";

/**
 * This structure is used to track a simple model
 * of the VM being executed. An instance of the
 * top of the stack is stored with variables at fixed
 * indices
 *
 * MISC TODO ITEMS:
 *
 * Iterate over the AST to see what variables
 * are accessed the most
 *
 * Automatically move vars between memory and stack
 */
pub struct VM<'a, T: PolynomialRingElement> {
    // represents the contents of the stack
    pub stack: Vec<String>,

    // name of variable keyed to offset in the stack
    // offsets are based on zero so they stay correct
    // as items are pushed/popped on the stack
    pub vars: HashMap<String, Var>,

    // compiled assembly
    pub asm: Vec<String>,

    // track whether the current vm has returned
    // this means the stack is cleared of variables
    pub has_returned: bool,

    // tracks the current logic block depth
    // the executor can see variables in higher blocks
    // but not lower blocks
    pub block_depth: usize,

    // the absolute position where our memory region begins
    pub memory_start: usize,

    // the current free memory index, relative to memory_start
    pub memory_index: usize,

    pub return_type: Option<ArgType>,

    pub compiler_state: &'a mut CompilerState<T>,
}

impl<'a, T: PolynomialRingElement> VM<'a, T> {
    pub fn new(compiler_state: &'a mut CompilerState<T>) -> Self {
        let memory_start = compiler_state.memory_offset;
        compiler_state.memory_offset += 2_usize.pow(32);
        VM {
            vars: HashMap::new(),
            stack: Vec::new(),
            asm: Vec::new(),
            has_returned: false,
            block_depth: 0,
            memory_index: 0,
            memory_start,
            compiler_state,
            return_type: None,
        }
    }

    // begin execution of a block
    // we'll track any variables created during
    // execution and remove them from the stack
    // in `end_block`
    pub fn begin_block(&mut self) {
        self.block_depth += 1;
    }

    // remove variables from the stack and the
    // local vm
    pub fn end_block(&mut self) -> Result<()> {
        if self.block_depth == 0 {
            return Err(anyhow::anyhow!("cannot exit execution root"));
        }
        // find all variables in this depth
        // and remove them from the stack
        let entries_to_remove = self
            .vars
            .iter()
            .filter(|(_k, v)| v.block_index >= self.block_depth)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<(String, Var)>>();
        if entries_to_remove.is_empty() {
            self.block_depth -= 1;
            return Ok(());
        }
        let mut pop_count = 0;
        for (k, v) in &entries_to_remove {
            // swap with the bottom of the stack
            if v.stack_index.is_some() {
                pop_count += 1;
            }
            self.vars.remove(k);
        }
        self.stack_pop(pop_count);
        self.block_depth -= 1;
        Ok(())
    }

    /**********
    Stack operations
    **/

    fn stack_pop(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        // at most 5 elements can be popped at once
        let iters = count / 5;
        let remainder = count % 5;
        for _ in 0..iters {
            self.asm.push("pop 5".to_string());
        }
        if remainder > 0 {
            self.asm.push(format!("pop {remainder}"));
        }
        self.stack.truncate(self.stack.len() - count);
    }

    fn stack_push(&mut self, v: u64) {
        self.asm.push(format!("push {v}"));
        self.stack.push("".to_string());
    }

    fn stack_dup(&mut self, i: usize) -> Result<()> {
        if i > self.stack.len() {
            return Err(anyhow::anyhow!(
                "cannot dup more elements than are on the stack {} {}",
                i,
                self.stack.len()
            ));
        }
        self.asm.push(format!("dup {}", self.stack.len() - i));
        self.stack.push("".to_string());
        Ok(())
    }

    fn stack_swap(&mut self, i: usize) -> Result<()> {
        if i == 0 {
            return Ok(());
        }
        if i > 16 {
            return Err(anyhow::anyhow!("cannot swap more than 16 elements deep"));
        }
        self.asm.push(format!("swap {}", i));
        Ok(())
    }

    fn stack_read_mem(&mut self, count: usize) {
        self.asm.push(format!("read_mem {count}"));
        for _ in 0..count {
            self.stack.push("".to_string());
        }
    }

    fn stack_write_mem(&mut self, count: usize) {
        self.asm.push(format!("write_mem {count}"));
        self.stack.truncate(self.stack.len() - count);
    }

    // define a static that will be available in
    // the current VM object
    pub fn static_var(&mut self, name: String, expr: Expr) -> Result<()> {
        // check for duplicate var names
        if self.vars.contains_key(&name) {
            return log::error!(
                &format!("variable name \"{name}\" is already in use"),
                "you're attempting to define a static variable with the same name as another variable"
            );
        }
        match &expr {
            Expr::Lit(v) => {
                self.vars.insert(
                    name,
                    Var {
                        stack_index: None,
                        block_index: self.block_depth,
                        location: VarLocation::Static,
                        memory_index: None,
                        dimensions: vec![],
                        value: Some(vec![(*v).parse::<u64>().unwrap()]),
                    },
                );
            }
            Expr::Val(ref_name, indices) => {
                if !indices.is_empty() {
                    return log::error!("static var index assignment not supported");
                }
                if let Some(v) = self.vars.get(&ref_name.clone()) {
                    match v.location {
                        VarLocation::Static => {
                            self.vars.insert(name, v.clone());
                        }
                        _ => {
                            return log::error!("dynamically evaluated statics not supported");
                        }
                    }
                } else {
                    return log::error!(&format!("unknown variable {ref_name}"));
                }
            }
            Expr::NumOp {
                lhs: _,
                op: _,
                rhs: _,
            } => {
                let out = self.eval(expr, false)?;
                if out.is_none() {
                    return log::error!("static expression evaluated to stack variable");
                }
                let out = out.unwrap();
                if out.location != VarLocation::Static {
                    return log::error!("static expression evaluated to memory variable");
                }
                self.vars.insert(name, out);
            }
            Expr::FnCall(_, _) => {
                if let Some(v) = self.eval(expr.clone(), false)? {
                    if v.location != VarLocation::Static {
                        return log::error!(
                            "static expression evaluated to memory variable in FnCall"
                        );
                    }
                    self.vars.insert(name.clone(), v);
                } else {
                    return log::error!("static expression evaluated to stack variable in FnCall");
                }
            }
            Expr::BoolOp {
                lhs: _,
                bool_op: _,
                rhs: _,
            } => {
                return log::error!("boolean operations in statics is not supported");
            }
            Expr::VecVec(_) | Expr::VecLit(_) => {
                let (dimensions, vec) = self.build_var_from_ast_vec(expr);
                self.vars.insert(
                    name,
                    Var {
                        stack_index: None,
                        block_index: self.block_depth,
                        location: VarLocation::Static,
                        memory_index: None,
                        dimensions,
                        value: Some(vec),
                    },
                );
            }
        }
        Ok(())
    }

    // return a value to the calling function
    //
    // this is not a way to create a public output
    // this is cross-function communication
    //
    // this function does not clean up the local variable
    // state. e.g. this VM cannot be used after returning
    pub fn return_expr(&mut self, expr: Expr) -> Result<()> {
        // we leave the returned value on the top of the stack
        // without registering is in self.vars
        let out = self.eval(expr, true)?;
        if let Some(v) = out {
            self.return_type = Some(ArgType {
                location: v.location.clone(),
                dimensions: v.dimensions.clone(),
                value: v.value,
            });
            self.stack_pop(self.stack.len());
        } else {
            // put the top of the stack at the bottom
            self.stack_swap(self.stack.len() - 1)?;
            self.return_type = Some(ArgType {
                location: VarLocation::Stack,
                dimensions: vec![],
                value: None,
            });
            self.stack_pop(self.stack.len() - 1);
        }
        // when we're done executing a block we clear
        // everything on the stack so that when we return
        // to the previous position the stack is in a
        // predictable state
        self.has_returned = true;
        Ok(())
    }

    // if the VM has not yet returned this function
    // pops any variables managed by the VM off the stack
    //
    // this function does not clean up the local variable
    // state. e.g. this VM cannot be used after returning
    pub fn return_if_needed(&mut self) {
        if self.has_returned {
            return;
        }
        self.return_type = Some(ArgType {
            location: VarLocation::Memory,
            dimensions: vec![],
            value: None,
        });
        self.stack_pop(self.stack.len());
        self.has_returned = true;
    }

    // defines a new mutable variable in the current block scope
    pub fn let_var(&mut self, name: String, expr: Expr) -> Result<()> {
        if self.vars.contains_key(&name) {
            return log::error!(&format!("var is not unique {name}"));
        }
        match &expr {
            Expr::VecLit(_) | Expr::VecVec(_) => {
                let (dimensions, vec) = self.build_var_from_ast_vec(expr);
                let v = Var {
                    stack_index: None,
                    block_index: self.block_depth,
                    location: VarLocation::Memory,
                    memory_index: Some(self.memory_start + self.memory_index),
                    dimensions,
                    value: None,
                };
                self.vars.insert(name, v);
                // put the variable in memory
                for vv in vec.clone().iter().rev() {
                    // self.asm.push(format!("push {vv}"))
                    self.stack_push(*vv);
                }
                self.stack_push(u64::try_from(self.memory_start + self.memory_index).unwrap());
                self.memory_index += vec.len();
                // TODO: batch insertion
                for _ in vec.iter() {
                    self.stack_write_mem(1);
                }
                // pop the updated ram pointer
                // track memory index in this VM instead
                self.stack_pop(1);
                Ok(())
            }
            _ => {
                let out = self.eval(expr, false)?;
                if out.is_none() {
                    // stack based variable
                    self.vars.insert(
                        name,
                        Var {
                            stack_index: Some(self.stack.len()),
                            block_index: self.block_depth,
                            location: VarLocation::Stack,
                            memory_index: None,
                            dimensions: vec![],
                            value: None,
                        },
                    );
                    return Ok(());
                }
                let out = out.unwrap();
                match out.location {
                    VarLocation::Memory => {
                        // memory based variable
                        self.vars.insert(name, out);
                        Ok(())
                    }
                    VarLocation::Static => {
                        // if static is a scalar write to stack
                        if out.value.clone().unwrap().len() == 1 {
                            self.stack_push(out.value.clone().unwrap()[0]);
                            self.vars.insert(
                                name,
                                Var {
                                    stack_index: Some(self.stack.len()),
                                    block_index: self.block_depth,
                                    location: VarLocation::Stack,
                                    memory_index: None,
                                    dimensions: vec![],
                                    value: None,
                                },
                            );
                            return Ok(());
                        }
                        // write to memory
                        let v = Var {
                            stack_index: None,
                            block_index: self.block_depth,
                            location: VarLocation::Memory,
                            memory_index: Some(self.memory_start + self.memory_index),
                            dimensions: out.dimensions,
                            value: out.value.clone(),
                        };
                        self.memory_index += out.value.unwrap().len();
                        // copy values into memory
                        let mut offset = v.memory_index.unwrap();
                        for v in v.value.as_ref().unwrap() {
                            self.stack_push(*v);
                            self.stack_push(u64::try_from(offset).unwrap());
                            self.stack_write_mem(1);
                            self.stack_pop(1);
                            offset += 1;
                        }
                        self.vars.insert(name, v);
                        Ok(())
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    // defines a new variable that is being passed to a function
    // such a variable must already exist on the top of the stack
    // relative to the local stack
    //
    // e.g. if the local stack is empty the variable must be on the
    // top of the stack. If the local stack has 1 entry the variable
    // must be index 1 in the stark stack.
    pub fn fn_var(&mut self, name: String, t: ArgType) -> Result<()> {
        if self.vars.contains_key(&name) {
            return log::error!(&format!(
                "function argument variable \"{name}\" is not unique"
            ));
        }
        match t.location {
            VarLocation::Static => {
                self.vars.insert(
                    name.clone(),
                    Var {
                        stack_index: None,
                        block_index: self.block_depth,
                        dimensions: t.dimensions,
                        location: VarLocation::Static,
                        memory_index: None,
                        value: t.value,
                    },
                );
                Ok(())
            }
            VarLocation::Stack => {
                self.stack.push(name.clone());
                self.vars.insert(
                    name.clone(),
                    Var {
                        stack_index: Some(self.stack.len()),
                        block_index: self.block_depth,
                        dimensions: vec![],
                        location: VarLocation::Stack,
                        memory_index: None,
                        value: None,
                    },
                );
                Ok(())
            }
            VarLocation::Memory => {
                self.stack.push(name.clone());
                self.vars.insert(
                    name.clone(),
                    Var {
                        stack_index: Some(self.stack.len()),
                        block_index: self.block_depth,
                        location: VarLocation::Memory,
                        memory_index: None,
                        dimensions: t.dimensions.clone(),
                        value: None,
                    },
                );
                Ok(())
            }
        }
    }

    // assign a variable that already exists
    //
    // do this by evaluating the expression on
    // the top of the stack, then swapping with
    // the real location on the stack, and popping
    // the swapped value
    pub fn set_var(&mut self, name: String, expr: Expr) -> Result<()> {
        if !self.vars.contains_key(&name) {
            return log::error!(
                &format!("var does not exist \"{name}\""),
                "you're attempting to assign a value to a variable that is not in scope"
            );
        }
        let v = self.vars.get(&name).unwrap();
        if v.location == VarLocation::Static {
            return log::error!(
                &format!("cannot assign static var \"{name}\""),
                "you're attempting to assign a value to variable that is a static"
            );
        }
        if v.location == VarLocation::Memory {
            // TODO: allow assigning memory based variable
            // partially or entirely
            // e.g. v[0] = [1, 2, 3]
            // or v = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            return log::error!(&format!("cannot assign memory var \"{name}\""), "you're attempting to re-assign a vector or matrix variable directly. This is not yet supported.");
        }
        // new value is on the top of the stack
        let v = self.eval_to_stack(expr, false)?;
        if let Some(v) = v {
            if v.location == VarLocation::Memory {
                return log::error!(
                    &format!("cannot assign memory value to stack var \"{name}\""),
                    "you're attempting to assign a vector to a scalar variable"
                );
            }
            if v.location == VarLocation::Static {
                return log::error!(
                    &format!("cannot assign vector static value to stack var \"{name}\""),
                    "you're attempting to assign a vector to a scalar variable"
                );
            }
        }
        self.stack_swap(self.stack_index(&name)?)?;
        self.stack_pop(1);
        Ok(())
    }

    // get the index of a variable in the execution stack
    //
    // the local stack tracks the relative positions of
    // variables in the execution stack
    pub fn stack_index(&self, var_name: &String) -> Result<usize> {
        if let Some(var) = self.vars.get(var_name) {
            if var.location == VarLocation::Memory {
                return log::error!(&format!(
                    "cannot get stack index of memory variable \"{var_name}\""
                ));
            }
            if var.location == VarLocation::Static {
                return log::error!(&format!(
                    "cannot get stack index of static variable \"{var_name}\""
                ));
            }
            if let Some(stack_index) = var.stack_index {
                Ok(self.stack.len() - stack_index)
            } else {
                Err(anyhow::anyhow!("var does not have a stack index"))
            }
        } else {
            log::error!(&format!("unknown variable \"{var_name}\""))
        }
    }

    // blocks are inserted into the asm as functions
    // each block has a function name and is accessed
    // with a jump (call)
    pub fn call_block(&mut self, block_name: &String) {
        self.asm.push(format!("call {block_name}"));
    }

    pub fn extract_literals(expr: Expr, out: &mut Vec<u64>) {
        match expr {
            Expr::VecVec(v) => {
                for a in v {
                    Self::extract_literals(a, out);
                }
            }
            Expr::VecLit(v) => {
                let mut vv = v
                    .clone()
                    .iter()
                    .map(|v| v.parse::<u64>().unwrap())
                    .collect();
                out.append(&mut vv);
            }
            _ => unreachable!(),
        }
    }

    pub fn dimensions_to_len(dimensions: Vec<usize>) -> usize {
        let mut len = 1;
        for d in &dimensions {
            len *= d;
        }
        len
    }

    pub fn build_var_from_ast_vec(&mut self, expr: Expr) -> (Vec<usize>, Vec<u64>) {
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
        let mut vec_rep: Vec<u64> = Vec::new();
        Self::extract_literals(expr, &mut vec_rep);
        (dimensions, vec_rep)
    }

    // output a single stack element
    pub fn calc_vec_offset(&mut self, dimensions: &Vec<usize>, indices: &[Expr]) -> Result<()> {
        let sum = |vec: &Vec<usize>, start: usize| -> usize {
            let mut out = 1;
            for v in &vec[start..] {
                out *= v;
            }
            out
        };
        // if all values are literals we can calculate statically and push that to the stack
        let mut is_static = true;
        for i in indices {
            match i {
                Expr::Lit(_) => {}
                _ => {
                    is_static = false;
                    break;
                }
            }
        }
        if is_static {
            let offset = self.calc_vec_offset_static(dimensions, indices)?;
            self.stack_push(offset.try_into().unwrap());
            return Ok(());
        }
        self.stack_push(0); // offset
        for x in 0..indices.len() {
            let o = self.eval_to_stack(indices[x].clone(), false)?;
            if o.is_some() {
                return log::error!("vector variables are not allowed as indices");
            }
            if x == indices.len() - 1 && indices.len() == dimensions.len() {
                self.asm.push("add".to_string());
                self.stack.pop();
            } else {
                let dim_sum = sum(dimensions, x + 1);
                self.stack_push(dim_sum.try_into().unwrap());
                self.asm.push("mul".to_string());
                self.stack.pop();
                self.asm.push("add".to_string());
                self.stack.pop();
            }
        }
        Ok(())
    }

    pub fn calc_vec_offset_static(
        &mut self,
        dimensions: &Vec<usize>,
        indices: &[Expr],
    ) -> Result<usize> {
        let sum = |vec: &Vec<usize>, start: usize| -> usize {
            let mut out = 1;
            for v in &vec[start..] {
                out *= v;
            }
            out
        };
        let indices = indices
            .iter()
            .map(|v| match v {
                Expr::Lit(v) => Ok((*v).parse::<usize>().unwrap()),
                Expr::Val(_, _) => {
                    let out = self.eval(v.clone(), false)?;
                    if out.is_none() {
                        return log::error!("vector cannot be indexed by stack variable");
                    }
                    let out = out.unwrap();
                    if out.location != VarLocation::Static {
                        return log::error!("non-static variables are not allowed as indices");
                    }
                    if !out.dimensions.is_empty() {
                        return log::error!("vector cannot be indexed by non-scalar");
                    }
                    Ok(out.value.unwrap()[0].try_into().unwrap())
                }
                // we should probably do some additional checks here
                _ => unreachable!(),
            })
            .collect::<Result<Vec<usize>>>()?;
        let mut offset = 0;
        for x in 0..indices.len() {
            // for each index we sum the deeper dimensions
            // to determine how far to move in the array storage
            if x == indices.len() - 1 && indices.len() == dimensions.len() {
                offset += indices[x];
            } else {
                offset += indices[x] * sum(dimensions, x + 1);
            }
        }
        Ok(offset)
    }

    pub fn static_to_stack(&mut self, v: &Var) -> Result<()> {
        if v.location == VarLocation::Static && v.value.clone().unwrap().len() == 1 {
            // static we can put on stack
            self.stack_push(v.value.clone().unwrap()[0]);
            return Ok(());
        }
        anyhow::bail!("static variable is not a scalar");
    }

    pub fn eval_to_stack(&mut self, expr: Expr, is_returning: bool) -> Result<Option<Var>> {
        if let Some(v) = self.eval(expr, is_returning)? {
            if self.static_to_stack(&v).is_ok() {
                return Ok(None);
            }
            return Ok(Some(v));
        }
        Ok(None)
    }

    // evaluate an AST expression
    //
    // manipulates the execution stack and tracks
    // changes in the local stack
    //
    // Optionally returns a variable reference to be used by the caller
    // if the return value is non-null the asm has not been mutated
    pub fn eval(&mut self, expr: Expr, is_returning: bool) -> Result<Option<Var>> {
        match &expr {
            Expr::VecLit(_v) => Err(anyhow::anyhow!(
                "vector literals must be assigned before operation"
            )),
            Expr::VecVec(_v) => Err(anyhow::anyhow!(
                "matrix literals must be assigned before operation"
            )),
            Expr::FnCall(name, vars) => {
                let mut arg_types: Vec<ArgType> = Vec::new();
                // we push these but don't pop them here
                // the destination function will handle that
                let mut stack_arg_count = 0;
                for v in vars {
                    // if it's a stack variable the asm will be modified as needed
                    let o = if self.compiler_state.is_fn_ash.contains_key(name) {
                        // if it's an ash function we can statically evaluate
                        // as needed
                        self.eval((*v).clone(), false)?
                    } else {
                        // if it's an assembly function we have to push the variable to the stack
                        self.eval_to_stack((*v).clone(), false)?
                    };
                    // let o = self.eval(Expr::Val(v.clone(), vec![]));
                    // if it's not a stack variable we'll get a return from self.eval
                    // and can add it to the arg_types. We'll then push the absolute
                    // position of the memory variable onto the stack
                    if let Some(v) = o {
                        arg_types.push(ArgType {
                            location: v.location.clone(),
                            dimensions: v.dimensions.clone(),
                            value: v.value.clone(),
                        });
                        // as long as each argument is exactly
                        // 1 stack element we don't need to mutate
                        // the virtual stack
                        if let Some(mem_index) = v.memory_index {
                            self.stack_push(mem_index.try_into().unwrap());
                            stack_arg_count += 1;
                        } else if let Some(stack_index) = v.stack_index {
                            // give a copy of the stack memory index or value
                            // to the function
                            // the function will pop the value off the stack
                            self.stack_dup(stack_index)?;
                            stack_arg_count += 1;
                        } else if v.location == VarLocation::Static {
                            //
                        } else {
                            return Err(anyhow::anyhow!("unexpected: variable has no memory or stack index and is not static"));
                        }
                    } else {
                        arg_types.push(ArgType {
                            location: VarLocation::Stack,
                            dimensions: vec![],
                            value: None,
                        });
                        stack_arg_count += 1;
                    }
                }
                // build functions as needed
                let mut call = FnCall {
                    name: name.clone(),
                    arg_types: arg_types.clone(),
                    return_type: None,
                };
                if let Some(call_type) = self.compiler_state.fn_return_types.get(&call) {
                    call.return_type = Some(call_type.return_type.as_ref().unwrap().clone());
                } else {
                    let fn_ast = self.compiler_state.fn_to_ast.get(name).unwrap().clone();
                    let mut vm = VM::new(self.compiler_state);
                    vm.eval_ast(fn_ast.clone(), arg_types.clone(), None)?;
                    vm.return_if_needed();
                    if let Some(return_type) = vm.return_type {
                        call.return_type = Some(return_type);
                    } else {
                        return log::error!(&format!(
                            "unable to determine return type for function \"{}\"",
                            name
                        ), "you may be calling a tasm function with the wrong number or type of arguments");
                    }
                    // re-evaluate the function with the return type
                    // set from the beginning
                    let mut vm = VM::new(self.compiler_state);
                    vm.eval_ast(fn_ast, arg_types.clone(), call.return_type.clone())?;
                    vm.return_if_needed();
                    let mut asm = vm.asm.clone();
                    asm.push("return".to_string());
                    // let no_return_call = call.clone();
                    self.compiler_state.compiled_fn.insert(call.clone(), asm);
                    // self.compiler_state
                    //     .fn_return_types
                    //     .insert(no_return_call.clone(), call.clone());
                }
                self.compiler_state
                    .called_fn
                    .entry(call.clone())
                    .and_modify(|call_count| {
                        *call_count += 1;
                    })
                    .or_insert_with(|| 1);
                // the function pops all arguments off the stack before it returns
                match call.return_type.clone().unwrap().location {
                    VarLocation::Static => {
                        for _ in 0..stack_arg_count {
                            self.stack.pop();
                        }
                        Ok(Some(Var {
                            stack_index: None,
                            location: VarLocation::Static,
                            dimensions: call.return_type.clone().unwrap().dimensions,
                            memory_index: None,
                            block_index: self.block_depth,
                            value: call.return_type.unwrap().value,
                        }))
                    }
                    VarLocation::Stack => {
                        for _ in 0..stack_arg_count {
                            self.stack.pop();
                        }
                        self.stack.push("".to_string());
                        // if the return value is a stack variable
                        // we need to increment the virtual stack
                        self.asm.push(format!("call {}", call.typed_name()));
                        Ok(None)
                    }
                    VarLocation::Memory => {
                        if is_returning {
                            if let Some(v) = self.vars.get(RETURN_VAR) {
                                self.stack_dup(v.stack_index.unwrap())?;
                            } else {
                                return Err(anyhow!("no return memory address"));
                            }
                        } else {
                            self.stack_push(
                                (self.memory_start + self.memory_index).try_into().unwrap(),
                            );
                        }
                        for _ in 0..stack_arg_count {
                            self.stack.pop();
                        }
                        self.stack.pop();
                        self.asm.push(format!("call {}", call.typed_name()));
                        if is_returning {
                            if let Some(v) = self.vars.get(RETURN_VAR) {
                                Ok(Some(Var {
                                    stack_index: Some(self.stack.len() - v.stack_index.unwrap()),
                                    location: VarLocation::Memory,
                                    dimensions: call.return_type.unwrap().dimensions.clone(),
                                    memory_index: None,
                                    block_index: self.block_depth,
                                    value: None,
                                }))
                            } else {
                                Err(anyhow!("no return memory address"))
                            }
                        } else {
                            let len = VM::<T>::dimensions_to_len(
                                call.clone().return_type.unwrap().dimensions,
                            );
                            let memory_index = self.memory_start + self.memory_index;
                            self.memory_index += len;
                            Ok(Some(Var {
                                stack_index: None,
                                location: VarLocation::Memory,
                                dimensions: call.return_type.unwrap().dimensions.clone(),
                                memory_index: Some(memory_index),
                                block_index: self.block_depth,
                                value: None,
                            }))
                        }
                    }
                }
            }
            Expr::Val(name, indices) => {
                // if the val is a static we push to stack
                if !self.vars.contains_key(name) {
                    return log::error!(&format!("unknown variable: {name}"));
                }
                let v = self.vars.get(name).unwrap().clone();
                self.load_variable(&v, indices)
            }
            Expr::Lit(v) => Ok(Some(Var {
                stack_index: None,
                location: VarLocation::Static,
                dimensions: vec![],
                memory_index: None,
                block_index: self.block_depth,
                value: Some(vec![v.parse::<u64>().unwrap()]),
            })),
            Expr::NumOp { lhs, op, rhs } => {
                // only allow variables of same dimensions
                // for now
                let mut lv = self.eval(*lhs.clone(), false)?;
                let mut rv = self.eval(*rhs.clone(), false)?;
                if lv.is_some()
                    && rv.is_none()
                    && self.static_to_stack(&lv.clone().unwrap()).is_ok()
                {
                    lv = None;
                    self.asm.push("swap 1".to_string());
                }
                if rv.is_some()
                    && lv.is_none()
                    && self.static_to_stack(&rv.clone().unwrap()).is_ok()
                {
                    rv = None;
                }
                if lv.is_none() != rv.is_none() {
                    return log::error!("type mismatch in numeric operation");
                }
                if lv.is_some() {
                    let lvu = lv.unwrap();
                    let rvu = rv.unwrap();
                    if lvu.dimensions.len() != rvu.dimensions.len() {
                        return log::error!(
                            "type mismatch in numeric operation, vector width mismatch"
                        );
                    }
                    for x in 0..lvu.dimensions.len() {
                        if lvu.dimensions[x] != rvu.dimensions[x] {
                            return log::error!(
                                "type mismatch in numeric operation, vector height mismatch"
                            );
                        }
                    }
                    let out_v = if is_returning
                        && lvu.location != VarLocation::Static
                        && rvu.location != VarLocation::Static
                    {
                        let return_var = self.vars.get(RETURN_VAR).unwrap();
                        Some(Var {
                            stack_index: Some(return_var.stack_index.unwrap()),
                            block_index: self.block_depth,
                            location: VarLocation::Memory,
                            memory_index: None,
                            dimensions: lvu.dimensions.clone(),
                            value: None,
                        })
                    } else {
                        None
                    };
                    // operate on elements in a vector stored in memory
                    // store the result in memory
                    // TODO: batch memory read/write operations
                    return match op {
                        NumOp::Add => self
                            .op_elements(&lvu, &rvu, out_v, |a, b| (a + b, vec![format!("add")])),
                        NumOp::Mul => self
                            .op_elements(&lvu, &rvu, out_v, |a, b| (a * b, vec![format!("mul")])),
                        NumOp::Sub => self.op_elements(&lvu, &rvu, out_v, |a, b| {
                            (
                                a - b,
                                vec![format!("push -1"), format!("mul"), format!("add")],
                            )
                        }),
                        NumOp::Inv => self.op_elements(&lvu, &rvu, out_v, |a, b| {
                            (a / b, vec![format!("invert"), format!("mul")])
                        }),
                    };
                }
                match op {
                    // each one of these removes two elements and
                    // adds 1
                    // so we have a net effect of a single pop
                    NumOp::Add => {
                        self.asm.push("add".to_string());
                        self.stack.pop();
                    }
                    NumOp::Sub => {
                        self.asm.append(&mut vec![
                            format!("push -1"),
                            format!("mul"),
                            format!("add"),
                        ]);
                        self.stack.pop();
                    }
                    NumOp::Mul => {
                        self.asm.push("mul".to_string());
                        self.stack.pop();
                    }
                    NumOp::Inv => {
                        self.asm
                            .append(&mut vec![format!("invert"), format!("mul")]);
                        self.stack.pop();
                    }
                }
                Ok(None)
            }
            Expr::BoolOp { lhs, bool_op, rhs } => {
                let lv = self.eval_to_stack(*lhs.clone(), false)?;
                let rv = self.eval_to_stack(*rhs.clone(), false)?;
                if lv.is_none() != rv.is_none() {
                    return log::error!("cannot apply boolean operation to stack and memory vars");
                }
                if lv.is_some() {
                    return log::error!("cannot apply boolean operation to memory vars");
                }
                match bool_op {
                    BoolOp::Equal => {
                        // we're popping the two inputs off the stack
                        // eq removes 1, and skiz removes 1
                        self.asm.append(&mut vec![format!("eq"), format!("skiz")]);
                        self.stack.pop();
                        self.stack.pop();
                    }
                    BoolOp::NotEqual => {
                        self.asm.append(&mut vec![
                            format!("eq"),
                            format!("push -1"),
                            format!("add"),
                            format!("skiz"),
                        ]);
                        self.stack.pop();
                        self.stack.pop();
                    }
                    _ => {
                        return Err(anyhow!("boolean operation not supported"));
                    }
                }
                Ok(None)
            }
        }
    }

    pub fn eval_ast(
        &mut self,
        ast: Vec<AstNode>,
        arg_types: Vec<ArgType>,
        return_type: Option<ArgType>,
    ) -> Result<()> {
        for v in ast {
            match v {
                AstNode::AssignVec(name, indices, expr) => {
                    if !self.vars.contains_key(&name) {
                        return log::error!(&format!(
                            "attempting to assign to undeclared variable \"{name}\""
                        ));
                    }
                    // value being assigned
                    let o = self.eval_to_stack(expr, false)?;
                    let v = self.vars.get(&name).unwrap().clone();
                    // offset is pushed onto the stack
                    self.calc_vec_offset(&v.dimensions, &indices)?;
                    match indices.len().cmp(&v.dimensions.len()) {
                        cmp::Ordering::Less => {
                            return Err(anyhow::anyhow!("cannot assign vec"));
                        }
                        cmp::Ordering::Greater => {
                            return log::error!("var dimension is too low for assignment", "you're accessing an index on a scalar, or an n+1 dimension on a vector of n dimensions");
                        }
                        cmp::Ordering::Equal => {
                            // assigning a scalar into a specific index in a vec
                            if o.is_some() {
                                return log::error!(&format!(
                                    "attempting to assign memory value to scalar \"{name}\""
                                ));
                            }
                            // push the expr to the stack then into memory
                            // we're accessing a scalar, move it to the stack
                            if let Some(mem_index) = v.memory_index {
                                self.stack_push(mem_index.try_into().unwrap());
                                self.asm.push("add".to_string());
                                self.stack.pop();
                                self.stack_write_mem(1);
                                self.stack_pop(1);
                            } else if let Some(stack_index) = v.stack_index {
                                self.stack_dup(stack_index)?;
                                self.asm.push("add".to_string());
                                self.stack.pop();
                                self.stack_write_mem(1);
                                self.stack_pop(1);
                            } else {
                                return log::error!(
                                    "unexpected: variable has no memory or stack index"
                                );
                            }
                        }
                    }
                }
                AstNode::EmptyVecDef(name, dimensions) => {
                    if self.vars.contains_key(&name) {
                        return log::error!(&format!(
                            "attempting to define a variable that already exists \"{name}\""
                        ));
                    }
                    let len = VM::<T>::dimensions_to_len(dimensions.clone());
                    self.vars.insert(
                        name.clone(),
                        Var {
                            stack_index: None,
                            block_index: self.block_depth,
                            location: VarLocation::Memory,
                            memory_index: Some(self.memory_start + self.memory_index),
                            dimensions,
                            value: None,
                        },
                    );
                    self.memory_index += len;
                }
                AstNode::Stmt(name, is_let, expr) => {
                    if is_let {
                        self.let_var(name, expr)?;
                    } else {
                        self.set_var(name, expr)?
                    }
                }
                AstNode::ExprUnassigned(expr) => {
                    let o = self.eval(expr, false)?;
                    if o.is_none() {
                        self.stack_pop(1);
                    }
                }
                AstNode::FnVar(vars) => {
                    if arg_types.len() != vars.len() {
                        return log::error!(&format!(
                            "function argument count mismatch: expected {}, got {}",
                            arg_types.len(),
                            vars.len()
                        ));
                    }
                    for x in 0..vars.len() {
                        self.fn_var(vars[x].clone(), arg_types[x].clone())?;
                    }
                    if let Some(t) = return_type.clone() {
                        if t.location == VarLocation::Memory {
                            self.fn_var(
                                RETURN_VAR.to_string(),
                                ArgType {
                                    location: VarLocation::Stack,
                                    dimensions: vec![],
                                    value: None,
                                },
                            )?;
                        }
                    } else {
                        self.fn_var(
                            RETURN_VAR.to_string(),
                            ArgType {
                                location: VarLocation::Stack,
                                dimensions: vec![],
                                value: None,
                            },
                        )?;
                    }
                }
                AstNode::Rtrn(expr) => {
                    self.return_expr(expr)?;
                }
                AstNode::StaticDef(name, expr) => {
                    // we must be able to fully evaluate
                    // the static at compile time
                    // e.g. the expr must contain only
                    // Expr::Lit and Expr::Val containing other statics
                    self.static_var(name, expr)?;
                }
                AstNode::If(expr, block_ast) => {
                    let v = self.eval_to_stack(expr, false)?;
                    if v.is_some() {
                        return Err(anyhow::anyhow!("if node returned memory value"));
                    }
                    let block_name = format!("block_____{}", self.compiler_state.block_counter);
                    self.compiler_state.block_counter += 1;
                    self.call_block(&block_name);
                    // vm.eval(expr1);
                    // vm.eval(expr2);
                    // push 0 to the stack based on the bool_op
                    let start_asm_len = self.asm.len();
                    self.begin_block();
                    // blocks can't take args
                    self.eval_ast(block_ast, vec![], None)?;
                    self.end_block()?;
                    // pull the resulting asm as the block asm
                    let mut block_asm = self.asm.drain(start_asm_len..).collect::<Vec<String>>();
                    block_asm.insert(0, format!("{block_name}:"));
                    block_asm.push("return".to_string());
                    self.compiler_state.block_fn_asm.push(block_asm);
                }
                AstNode::Loop(expr, block_ast) => {
                    let o = self.eval(expr, false)?;
                    if o.is_none() {
                        return log::error!("loop condition must be static");
                    }
                    let o = o.unwrap();
                    if o.location != VarLocation::Static {
                        return log::error!("loop condition must be static");
                    }

                    for _ in 0..o.value.clone().unwrap()[0] {
                        self.begin_block();
                        self.eval_ast(block_ast.clone(), vec![], None)?;
                        self.end_block()?;
                    }
                }
            }
        }
        Ok(())
    }

    // call this with an offset on the stack
    fn load_scalar(&mut self, v: &Var, offset: Option<usize>) -> Result<()> {
        match v.location {
            VarLocation::Stack => {
                if offset.is_some() {
                    return log::error!("attempting to access stack variable by index");
                }
                self.stack_dup(v.stack_index.unwrap())?;
            }
            VarLocation::Memory => {
                if let Some(offset) = offset {
                    self.stack_push(offset.try_into().unwrap());
                } else {
                    self.calc_vec_offset(&v.dimensions, &[])?;
                }
                // we're accessing a scalar, move it to the stack
                if let Some(mem_index) = v.memory_index {
                    self.stack_push(mem_index.try_into().unwrap());
                    self.asm.push("add".to_string());
                    self.stack.pop();
                    self.stack_read_mem(1);
                    self.stack_pop(1);
                } else if let Some(stack_index) = v.stack_index {
                    self.stack_dup(stack_index)?;
                    self.asm.push("add".to_string());
                    self.stack.pop();
                    self.stack_read_mem(1);
                    self.stack_pop(1);
                } else {
                    return Err(anyhow!("unexpected: variable has no memory or stack index"));
                }
            }
            VarLocation::Static => {
                // should not have an offset on the stack
                if v.value.is_none() {
                    return Err(anyhow!("static variable does not have values defined"));
                }
                if offset.is_none() {
                    return Err(anyhow!("static variable access must have an offset"));
                }
                let value = v.value.as_ref().unwrap();
                self.stack_push(value[offset.unwrap()]);
            }
        }
        Ok(())
    }

    // load a stack, memory or static variable and return it
    fn load_variable(&mut self, v: &Var, indices: &[Expr]) -> Result<Option<Var>> {
        match v.location {
            VarLocation::Stack => {
                if !indices.is_empty() {
                    return log::error!("attempting to access stack variable \"unknown\" by index");
                }
                self.stack_dup(v.stack_index.unwrap())?;
            }
            VarLocation::Memory => {
                // return a subset of the original variable based on the
                // requested indices
                //
                // let a
                // accessing as a[1]
                // dimensions: [3, 2]
                // dimensions_fin: [2]
                //
                // if we're operating on two scalars (length 1 vector)
                // we should move the value to the stack
                if indices.len() == v.dimensions.len() {
                    self.calc_vec_offset(&v.dimensions, indices)?;
                    // we're accessing a scalar, move it to the stack
                    if let Some(mem_index) = v.memory_index {
                        self.stack_push(mem_index.try_into().unwrap());
                        self.asm.push("add".to_string());
                        self.stack.pop();
                        self.stack_read_mem(1);
                        self.stack_pop(1);
                    } else if let Some(stack_index) = v.stack_index {
                        self.stack_dup(stack_index)?;
                        self.asm.push("add".to_string());
                        self.stack.pop();
                        self.stack_read_mem(1);
                        self.stack_pop(1);
                    } else {
                        return Err(anyhow!("unexpected: variable has no memory or stack index"));
                    }
                } else {
                    let offset = self.calc_vec_offset_static(&v.dimensions, indices)?;
                    // we're accessing a vec/mat, leave it in memory
                    if let Some(mem_index) = v.memory_index {
                        if v.stack_index.is_some() {
                            return Err(anyhow!(
                                "memory variable should not have a stack and memory index defined"
                            ));
                        }
                        return Ok(Some(Var {
                            stack_index: None,
                            block_index: v.block_index,
                            location: v.location.clone(),
                            memory_index: Some(mem_index + offset),
                            dimensions: v.dimensions[indices.len()..].to_vec(),
                            value: None,
                        }));
                    } else if offset == 0 {
                        if v.stack_index.is_none() {
                            return Err(anyhow!(
                                "memory variable has neither stack nor memory index defined"
                            ));
                        }
                        return Ok(Some(Var {
                            stack_index: v.stack_index,
                            block_index: v.block_index,
                            location: v.location.clone(),
                            memory_index: None,
                            dimensions: v.dimensions[indices.len()..].to_vec(),
                            value: None,
                        }));
                    } else {
                        return Err(anyhow::anyhow!(
                            "cannot access stack addressed memory variable by index"
                        ));
                    }
                }
            }
            VarLocation::Static => {
                if v.value.is_none() {
                    return Err(anyhow!("static variable does not have values defined"));
                }
                let value = v.value.as_ref().unwrap();
                let offset = self.calc_vec_offset_static(&v.dimensions, indices)?;
                if indices.len() == v.dimensions.len() {
                    // returning a scalar
                    return Ok(Some(Var {
                        stack_index: v.stack_index,
                        block_index: v.block_index,
                        location: v.location.clone(),
                        memory_index: v.memory_index,
                        dimensions: vec![],
                        value: Some(vec![value[offset]]),
                    }));
                } else {
                    // returning a vector
                    anyhow::bail!("static vector slices not supported");
                    // return Ok(Some(Var {
                    //     stack_index: v.stack_index,
                    //     block_index: v.block_index,
                    //     location: v.location.clone(),
                    //     memory_index: v.memory_index,
                    //     dimensions: v.dimensions[indices.len()..].to_vec(),
                    //     value: Some(value[offset..].to_vec()),
                    // }));
                }
            }
        }
        Ok(None)
    }

    // load two variables onto the stack from wherever they are
    // then apply `ops` to them
    fn op_elements(
        &mut self,
        v1: &Var,
        v2: &Var,
        out_: Option<Var>,
        ops: fn(T, T) -> (T, Vec<String>),
    ) -> Result<Option<Var>> {
        let total_len = VM::<T>::dimensions_to_len(v1.dimensions.clone());
        if v1.location == VarLocation::Static
            && v2.location == VarLocation::Static
            && out_.is_none()
        {
            let mut out = Var {
                stack_index: None,
                block_index: self.block_depth,
                location: VarLocation::Static,
                memory_index: None,
                dimensions: v1.dimensions.clone(),
                value: Some(Vec::new()),
            };
            for x in 0..total_len {
                let (out_v, _) = ops(
                    T::from(v1.value.as_ref().unwrap()[x]),
                    T::from(v2.value.as_ref().unwrap()[x]),
                );
                let out_v = out_v.to_string().parse::<u64>().unwrap();
                out.value.as_mut().unwrap().push(out_v);
            }
            return Ok(Some(out));
        }
        let out;
        if let Some(v) = out_ {
            out = v.clone();
        } else {
            out = Var {
                stack_index: None,
                block_index: self.block_depth,
                location: VarLocation::Memory,
                memory_index: Some(self.memory_start + self.memory_index),
                dimensions: v1.dimensions.clone(),
                value: None,
            };
            self.memory_index += total_len;
        }
        // TODO: assert equal shape
        for x in 0..total_len {
            self.load_scalar(v1, Some(x))?;
            // make sure the RHS is read second so the inv operation
            // is applied to the correct operand
            self.load_scalar(v2, Some(x))?;
            // v1 and v2 are operated on and a single output
            // remains
            self.asm.append(&mut ops(T::zero(), T::one()).1);
            self.stack.pop();

            if let Some(memory_index) = out.memory_index {
                self.stack_push((memory_index + x).try_into().unwrap());
            } else if let Some(stack_index) = out.stack_index {
                self.stack_dup(stack_index)?;
                self.stack_push(x.try_into().unwrap());
                self.asm.push("add".to_string());
                self.stack.pop();
            }
            // the final output is written to memory below and
            // removed from the stack
            self.stack_write_mem(1);
            self.stack_pop(1);
        }
        Ok(Some(out))
    }
}
