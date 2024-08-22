use crate::compiler::CompilerState;
use crate::log;
use crate::parser::AstNode;
use crate::parser::BoolOp;
use crate::parser::Expr;
use crate::parser::NumOp;
use std::collections::HashMap;
use std::fmt::format;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Stack,
    Memory,
    Static,
}

#[derive(Clone)]
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
        if self.arg_types.len() > 0 {
            out.push_str("____");
        }
        for arg in &self.arg_types {
            out.push_str("_");
            if arg.dimensions.len() == 0 {
                out.push_str("s");
            }
            for d_index in 0..arg.dimensions.len() {
                if d_index > 0 {
                    out.push_str("x");
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
pub struct VM<'a> {
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

    pub compiler_state: &'a mut CompilerState,
}

impl<'a> VM<'a> {
    pub fn new(compiler_state: &'a mut CompilerState) -> Self {
        let memory_start = compiler_state.memory_offset.clone();
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
    pub fn end_block(&mut self, keep_var: Option<String>) {
        if self.block_depth == 0 {
            panic!("cannot exit execution root");
        }
        // find all variables in this depth
        // and remove them from the stack
        let entries_to_remove = self
            .vars
            .iter()
            .filter(|(_k, v)| v.block_index == self.block_depth && v.location == VarLocation::Stack)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<(String, Var)>>();
        if entries_to_remove.is_empty() {
            self.block_depth -= 1;
            return;
        }
        // TODO: don't iterate here
        let keep_name;
        if let Some(name) = &keep_var {
            if let Some(_) = self.eval(Expr::Val(name.clone(), vec![]), false) {
                panic!("non-stack variable cannot be kept on the stack");
            } else {
                let target_stack_depth = entries_to_remove.len();
                self.asm.push(format!("swap {}", target_stack_depth));
                self.asm.push("pop 1".to_string());
                self.stack.pop();
            }
            keep_name = name.clone();
        } else {
            keep_name = String::new();
        }
        for (k, _) in &entries_to_remove {
            // swap with the bottom of the stack
            if &keep_name == k {
                continue;
            }
            self.stack.pop();
            self.asm.push(format!("pop 1"));
            self.vars.remove(k);
        }
        self.block_depth -= 1;
    }

    // define a static that will be available in
    // the current VM object
    pub fn static_var(&mut self, name: String, expr: Expr) {
        // check for duplicate var names
        if self.vars.contains_key(&name) {
            log::error!(
                &format!("variable name \"{name}\" is already in use"),
                &format!("you're attempting to define a static variable with the same name as another variable")
            );
        }
        match expr {
            Expr::Lit(v) => {
                self.vars.insert(
                    name,
                    Var {
                        stack_index: None,
                        block_index: self.block_depth,
                        location: VarLocation::Static,
                        memory_index: None,
                        dimensions: vec![],
                        value: Some(vec![v]),
                    },
                );
            }
            Expr::Val(ref_name, indices) => {
                if indices.len() > 0 {
                    log::error!("static var index assignment not supported");
                }
                if let Some(v) = self.vars.get(&ref_name) {
                    match v.location {
                        VarLocation::Static => {
                            self.vars.insert(name, v.clone());
                        }
                        _ => {
                            log::error!("dynamically evaluated statics not supported");
                        }
                    }
                } else {
                    log::error!(&format!("unknown variable {ref_name}"));
                }
            }
            Expr::NumOp {
                lhs: _,
                op: _,
                rhs: _,
            } => {
                log::error!("numerical operations in statics is not yet supported");
            }
            Expr::FnCall(_name, _vars) => {
                log::error!("static expression functions not implemented");
            }
            Expr::BoolOp {
                lhs: _,
                bool_op: _,
                rhs: _,
            } => {
                log::error!("boolean operations in statics is not supported");
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
    }

    // return a value to the calling function
    //
    // this is not a way to create a public output
    // this is cross-function communication
    //
    // this function does not clean up the local variable
    // state. e.g. this VM cannot be used after returning
    pub fn return_expr(&mut self, expr: Expr) {
        // we leave the returned value on the top of the stack
        // without registering is in self.vars
        let out = self.eval(expr, true);
        if let Some(v) = out {
            self.return_type = Some(ArgType {
                location: v.location.clone(),
                dimensions: v.dimensions.clone(),
            });
        } else {
            // put the top of the stack at the bottom
            self.asm.push(format!("swap {}", self.vars.len()));
            self.return_type = Some(ArgType {
                location: VarLocation::Stack,
                dimensions: vec![],
            });
        }
        // when we're done executing a block we clear
        // everything on the stack so that when we return
        // to the previous position the stack is in a
        // predictable state
        for _ in 0..self
            .vars
            .iter()
            .filter(|(_k, v)| v.stack_index.is_some())
            .collect::<Vec<(&String, &Var)>>()
            .len()
        {
            self.asm.push(format!("pop 1"));
        }
        self.has_returned = true;
    }

    // if the VM has not yet returned this function
    // pops any variables managed by the VM off the stack
    //
    // this function does not clean up the local variable
    // state. e.g. this VM cannot be used after returning
    pub fn return_if_needed(&mut self) {
        if self.has_returned || self.vars.is_empty() {
            return;
        }
        self.return_type = Some(ArgType {
            location: VarLocation::Memory,
            dimensions: vec![],
        });
        for _ in 0..self
            .vars
            .iter()
            .filter(|(_k, v)| v.stack_index.is_some())
            .collect::<Vec<(&String, &Var)>>()
            .len()
        {
            self.asm.push(format!("pop 1"));
        }
        self.has_returned = true;
    }

    // defines a new mutable variable in the current block scope
    pub fn let_var(&mut self, name: String, expr: Expr) {
        if self.vars.contains_key(&name) {
            log::error!(&format!("var is not unique {name}"));
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
                    self.asm.push(format!("push {vv}"))
                }
                self.asm
                    .push(format!("push {}", self.memory_start + self.memory_index));
                self.memory_index += vec.len();
                // TODO: batch insertion
                for _ in vec.iter() {
                    self.asm.push(format!("write_mem 1"))
                }
                // pop the updated ram pointer
                // track memory index in this VM instead
                self.asm.push("pop 1".to_string());
            }
            _ => {
                let out = self.eval(expr, false);
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
                } else {
                    // memory based variable
                    self.vars.insert(name, out.unwrap());
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
    pub fn fn_var(&mut self, name: String, t: ArgType) {
        if self.vars.contains_key(&name) {
            log::error!(&format!(
                "function argument variable \"{name}\" is not unique"
            ));
        }
        match t.location {
            VarLocation::Static => {
                log::error!(
                    &format!("function argument variable \"{name}\" is static"),
                    "calling functions with static arguments is not supported yet"
                );
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
            }
        }
    }

    // assign a variable that already exists
    //
    // do this by evaluating the expression on
    // the top of the stack, then swapping with
    // the real location on the stack, and popping
    // the swapped value
    pub fn set_var(&mut self, name: String, expr: Expr) {
        if !self.vars.contains_key(&name) {
            log::error!(
                &format!("var does not exist \"{name}\""),
                "you're attempting to assign a value to a variable that is not in scope"
            );
        }
        let v = self.vars.get(&name).unwrap();
        if v.location == VarLocation::Static {
            log::error!(
                &format!("cannot assign static var \"{name}\""),
                "you're attempting to assign a value to variable that is a static"
            );
        }
        if v.location == VarLocation::Memory {
            // TODO: allow assigning memory based variable
            // partially or entirely
            // e.g. v[0] = [1, 2, 3]
            // or v = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            log::error!(&format!("cannot assign memory var \"{name}\""), "you're attempting to re-assign a vector or matrix variable directly. This is not yet supported.");
        }
        // new value is on the top of the stack
        let v = self.eval(expr, false);
        if v.is_some() {
            log::error!(
                &format!("cannot assign memory value to stack var \"{name}\""),
                "you're attempting to assign a vector to a scalar variable"
            );
        }
        self.asm.push(format!("swap {}", self.stack_index(&name)));
        self.asm.push("pop 1".to_string());
        self.stack.pop();
    }

    // get the index of a variable in the execution stack
    //
    // the local stack tracks the relative positions of
    // variables in the execution stack
    pub fn stack_index(&self, var_name: &String) -> usize {
        if let Some(var) = self.vars.get(var_name) {
            if var.location == VarLocation::Memory {
                log::error!(&format!(
                    "cannot get stack index of memory variable \"{var_name}\""
                ));
            }
            if var.location == VarLocation::Static {
                log::error!(&format!(
                    "cannot get stack index of static variable \"{var_name}\""
                ));
            }
            if let Some(stack_index) = var.stack_index {
                self.stack.len() - stack_index
            } else {
                panic!("var does not have a stack index");
            }
        } else {
            log::error!(&format!("unknown variable \"{var_name}\""));
        }
    }

    // blocks are inserted into the asm as functions
    // each block has a function name and is accessed
    // with a jump (call)
    pub fn call_block(&mut self, block_name: &String) {
        self.asm.push(format!("call {block_name}"));
    }

    pub fn extract_literals(&mut self, expr: Expr, out: &mut Vec<u64>) {
        match expr {
            Expr::VecVec(v) => {
                for a in v {
                    self.extract_literals(a, out);
                }
            }
            Expr::VecLit(v) => {
                let mut vv = v.clone();
                out.append(&mut vv);
            }
            _ => panic!("asf"),
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
        self.extract_literals(expr, &mut vec_rep);
        (dimensions, vec_rep)
    }

    // output a single stack element
    pub fn calc_vec_offset(&mut self, dimensions: &Vec<usize>, indices: &Vec<Expr>) {
        let sum = |vec: &Vec<usize>, start: usize| -> usize {
            let mut out = 1;
            for x in start..vec.len() {
                out *= vec[x];
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
            let offset = Self::calc_vec_offset_static(dimensions, indices);
            self.stack.push("vec_offset".to_string());
            self.asm.push(format!("push {offset}"));
            return;
        }
        self.asm.push("push 0".to_string());
        self.stack.push("offset".to_string());
        for x in 0..indices.len() {
            let o = self.eval(indices[x].clone(), false);
            if o.is_some() {
                log::error!("memory variables are not allowed as indices");
            }
            if x == indices.len() - 1 && indices.len() == dimensions.len() {
                self.asm.push("add".to_string());
                self.stack.pop();
            } else {
                let dim_sum = sum(dimensions, x + 1);
                self.asm.push(format!("push {dim_sum}"));
                self.asm.push("mul".to_string());
                self.asm.push("add".to_string());
                self.stack.pop();
            }
        }
    }

    pub fn calc_vec_offset_static(dimensions: &Vec<usize>, indices: &Vec<Expr>) -> usize {
        let sum = |vec: &Vec<usize>, start: usize| -> usize {
            let mut out = 1;
            for x in start..vec.len() {
                out *= vec[x];
            }
            out
        };
        let indices = indices
            .iter()
            .map(|v| match v {
                Expr::Lit(v) => usize::try_from(*v).unwrap(),
                _ => {
                    log::error!("only literals are allowed as static indices");
                }
            })
            .collect::<Vec<usize>>();
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
        offset
    }

    // evaluate an AST expression
    //
    // manipulates the execution stack and tracks
    // changes in the local stack
    //
    // Optionally returns a variable reference to be used by the caller
    // if the return value is non-null the asm has not been mutated
    pub fn eval(&mut self, expr: Expr, is_returning: bool) -> Option<Var> {
        match &expr {
            Expr::VecLit(_v) => {
                panic!("vector literals must be assigned before operation");
            }
            Expr::VecVec(_v) => {
                panic!("matrix literals must be assigned before operation");
            }
            Expr::FnCall(name, vars) => {
                let mut arg_types: Vec<ArgType> = Vec::new();
                // we push these but don't pop them here
                // the destination function will handle that
                for v in vars {
                    // if it's a stack variable the asm will be modified as needed
                    let o = self.eval(*(*v).clone(), false);
                    // let o = self.eval(Expr::Val(v.clone(), vec![]));
                    // if it's not a stack variable we'll get a return from self.eval
                    // and can add it to the arg_types. We'll then push the absolute
                    // position of the memory variable onto the stack
                    if let Some(v) = o {
                        if v.location == VarLocation::Static {
                            panic!("cannot pass static variable as argument");
                        }
                        arg_types.push(ArgType {
                            location: v.location,
                            dimensions: v.dimensions.clone(),
                        });
                        // as long as each argument is exactly
                        // 1 stack element we don't need to mutate
                        // the virtual stack
                        if let Some(mem_index) = v.memory_index {
                            self.stack.push(name.clone());
                            self.asm.push(format!("push {mem_index}"));
                        } else if let Some(stack_index) = v.stack_index {
                            // give a copy of the stack memory index or value
                            // to the function
                            // the function will pop the value off the stack
                            println!("{stack_index}, {}", self.stack.len());
                            self.asm
                                .push(format!("dup {}", self.stack.len() - stack_index));
                            self.stack.push(name.clone());
                        } else {
                            panic!("unexpected: variable has no memory or stack index")
                        }
                    } else {
                        arg_types.push(ArgType {
                            location: VarLocation::Stack,
                            dimensions: vec![],
                        });
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
                    let mut vm = VM::new(&mut self.compiler_state);
                    vm.eval_ast(fn_ast, arg_types.clone());
                    vm.return_if_needed();
                    let mut asm = vm.asm.clone();
                    asm.push("return".to_string());
                    let no_return_call = call.clone();
                    if let Some(return_type) = vm.return_type {
                        call.return_type = Some(return_type);
                    } else {
                        log::error!(&format!(
                            "unable to determine return type for function \"{}\"",
                            name
                        ), &format!("you may be calling a tasm function with the wrong number or type of arguments"));
                    }
                    self.compiler_state.compiled_fn.insert(call.clone(), asm);
                    self.compiler_state
                        .fn_return_types
                        .insert(no_return_call.clone(), call.clone());
                }
                self.compiler_state
                    .called_fn
                    .entry(call.clone())
                    .and_modify(|call_count| {
                        *call_count += 1;
                    })
                    .or_insert_with(|| 1);
                // we push the return memory index for every call
                // it's not used if the return type is stack
                if is_returning {
                    if let Some(v) = self.vars.get(RETURN_VAR) {
                        self.asm
                            .push(format!("dup {}", self.stack.len() - v.stack_index.unwrap()));
                    } else {
                        panic!("no return memory address");
                    }
                } else {
                    self.asm
                        .push(format!("push {}", self.memory_start + self.memory_index));
                }
                // the function pops all arguments off the stack before it returns
                for _ in 0..vars.len() {
                    self.stack.pop();
                }
                match call.return_type.clone().unwrap().location {
                    VarLocation::Static => {
                        panic!("cannot return static variable from function");
                    }
                    VarLocation::Stack => {
                        // if the return value is a stack variable
                        // we need to increment the virtual stack
                        self.stack.push(name.clone());
                        self.asm.push(format!("call {}", call.typed_name()));
                        return None;
                    }
                    VarLocation::Memory => {
                        self.asm.push(format!("call {}", call.typed_name()));
                        if is_returning {
                            // TODO: test this code, it's currently only used
                            // in the return function above
                            // the stack index calculated below is likely wrong
                            if let Some(v) = self.vars.get(RETURN_VAR) {
                                return Some(Var {
                                    stack_index: Some(self.stack.len() - v.stack_index.unwrap()),
                                    location: VarLocation::Memory,
                                    dimensions: call.return_type.unwrap().dimensions.clone(),
                                    memory_index: None,
                                    block_index: self.block_depth,
                                    value: None,
                                });
                            } else {
                                panic!("no return memory address");
                            }
                        } else {
                            let len =
                                VM::dimensions_to_len(call.clone().return_type.unwrap().dimensions);
                            let memory_index = self.memory_start + self.memory_index;
                            self.memory_index += len;
                            return Some(Var {
                                stack_index: None,
                                location: VarLocation::Memory,
                                dimensions: call.return_type.unwrap().dimensions.clone(),
                                memory_index: Some(memory_index),
                                block_index: self.block_depth,
                                value: None,
                            });
                        }
                    }
                }
            }
            Expr::Val(name, indices) => {
                // if the val is a static we push to stack
                if !self.vars.contains_key(name) {
                    log::error!(&format!("unknown variable: {name}"));
                }
                let v = self.vars.get(name).unwrap().clone();
                self.load_variable(&v, indices)
            }
            Expr::Lit(v) => {
                self.stack.push(v.to_string());
                self.asm.push(format!("push {}", v));
                return None;
            }
            Expr::NumOp { lhs, op, rhs } => {
                // only allow variables of same dimensions
                // for now
                let lv = self.eval(*lhs.clone(), false);
                let rv = self.eval(*rhs.clone(), false);
                if lv.is_none() != rv.is_none() {
                    log::error!("type mismatch in numeric operation");
                }
                if lv.is_some() {
                    let lvu = lv.unwrap();
                    let rvu = rv.unwrap();
                    if lvu.dimensions.len() != rvu.dimensions.len() {
                        log::error!("type mismatch in numeric operation, vector width mismatch");
                    }
                    for x in 0..lvu.dimensions.len() {
                        if lvu.dimensions[x] != rvu.dimensions[x] {
                            log::error!(
                                "type mismatch in numeric operation, vector height mismatch"
                            );
                        }
                    }
                    let total_len = VM::dimensions_to_len(lvu.dimensions.clone());
                    let out_v;
                    if is_returning {
                        let return_var = self.vars.get(RETURN_VAR).unwrap();
                        out_v = Var {
                            stack_index: Some(return_var.stack_index.unwrap()),
                            block_index: self.block_depth,
                            location: VarLocation::Memory,
                            memory_index: None,
                            dimensions: lvu.dimensions.clone(),
                            value: None,
                        };
                    } else {
                        out_v = Var {
                            stack_index: None,
                            block_index: self.block_depth,
                            location: VarLocation::Memory,
                            memory_index: Some(self.memory_start + self.memory_index),
                            dimensions: lvu.dimensions.clone(),
                            value: None,
                        };
                        self.memory_index += total_len;
                    }
                    // operate on elements in a vector stored in memory
                    // store the result in memory
                    // TODO: batch memory read/write operations
                    match op {
                        NumOp::Add => {
                            self.op_elements(&lvu, &rvu, &out_v, &mut vec![format!("add")]);
                        }
                        NumOp::Mul => {
                            self.op_elements(&lvu, &rvu, &out_v, &mut vec![format!("mul")]);
                        }
                        NumOp::Sub => {
                            self.op_elements(
                                &lvu,
                                &rvu,
                                &out_v,
                                &mut vec![format!("push -1"), format!("mul"), format!("add")],
                            );
                        }
                        NumOp::Inv => {
                            self.op_elements(
                                &lvu,
                                &rvu,
                                &out_v,
                                &mut vec![format!("invert"), format!("mul")],
                            );
                        }
                    }

                    return Some(out_v);
                }
                match op {
                    // each one of these removes two elements and
                    // adds 1
                    // so we have a net effect of a single pop
                    NumOp::Add => {
                        self.stack.pop();
                        self.asm.push(format!("add"));
                    }
                    NumOp::Sub => {
                        self.stack.pop();
                        self.asm.append(&mut vec![
                            format!("push -1"),
                            format!("mul"),
                            format!("add"),
                        ]);
                    }
                    NumOp::Mul => {
                        self.stack.pop();
                        self.asm.push(format!("mul"));
                    }
                    NumOp::Inv => {
                        self.stack.pop();
                        self.asm
                            .append(&mut vec![format!("invert"), format!("mul")]);
                    }
                }
                return None;
            }
            Expr::BoolOp { lhs, bool_op, rhs } => {
                let l = (*lhs).clone();
                let lv = self.eval(*l, false);
                let r = (*rhs).clone();
                let rv = self.eval(*r, false);
                if lv.is_none() != rv.is_none() {
                    log::error!("cannot apply boolean operation to stack and memory vars");
                }
                if !lv.is_none() {
                    log::error!("cannot apply boolean operation to memory vars");
                }
                match bool_op {
                    BoolOp::Equal => {
                        // we're popping the two inputs off the stack
                        // eq removes 1, and skiz removes 1
                        self.stack.pop();
                        self.stack.pop();
                        self.asm.append(&mut vec![format!("eq"), format!("skiz")]);
                    }
                    BoolOp::NotEqual => {
                        self.stack.pop();
                        self.stack.pop();
                        self.asm.append(&mut vec![
                            format!("eq"),
                            format!("push -1"),
                            format!("add"),
                            format!("skiz"),
                        ]);
                    }
                    _ => panic!("boolean operation not supported"),
                }
                return None;
            }
        }
    }

    pub fn eval_ast(&mut self, ast: Vec<AstNode>, arg_types: Vec<ArgType>) {
        for v in ast {
            match v {
                AstNode::AssignVec(name, indices, expr) => {
                    if !self.vars.contains_key(&name) {
                        log::error!(&format!(
                            "attempting to assign to undeclared variable \"{name}\""
                        ));
                    }
                    // value being assigned
                    let o = self.eval(expr, false);
                    let v = self.vars.get(&name).unwrap().clone();
                    // offset is pushed onto the stack
                    self.calc_vec_offset(&v.dimensions, &indices);
                    if indices.len() == v.dimensions.len() {
                        // assigning a scalar into a specific index in a vec
                        if o.is_some() {
                            log::error!(&format!(
                                "attempting to assign memory value to scalar \"{name}\""
                            ));
                        }
                        // push the expr to the stack then into memory
                        // we're accessing a scalar, move it to the stack
                        if let Some(mem_index) = v.memory_index {
                            self.asm.push(format!("push {mem_index}"));
                            self.asm.push(format!("add"));
                            self.asm.push(format!("write_mem 1"));
                            self.asm.push(format!("pop 1"));
                            self.stack.pop();
                            self.stack.pop();
                        } else if let Some(stack_index) = v.stack_index {
                            self.asm
                                .push(format!("dup {}", self.stack.len() - stack_index));
                            self.asm.push("add".to_string());
                            self.asm.push(format!("write_mem 1"));
                            self.asm.push(format!("pop 1"));
                            self.stack.pop();
                            self.stack.pop();
                        } else {
                            log::error!("unexpected: variable has no memory or stack index");
                        }
                    } else if indices.len() > v.dimensions.len() {
                        log::error!("var dimension is too low for assignment", "you're accessing an index on a scalar, or an n+1 dimension on a vector of n dimensions");
                    } else {
                        panic!("cannot assign vec");
                    }
                }
                AstNode::EmptyVecDef(name, dimensions) => {
                    if self.vars.contains_key(&name) {
                        log::error!(&format!(
                            "attempting to define a variable that already exists \"{name}\""
                        ));
                    }
                    let len = VM::dimensions_to_len(dimensions.clone());
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
                        self.let_var(name, expr);
                    } else {
                        self.set_var(name, expr)
                    }
                }
                AstNode::ExprUnassigned(expr) => {
                    let o = self.eval(expr, false);
                    if let Some(v) = o {
                        if let Some(_) = v.stack_index {
                            self.stack.pop();
                            self.asm.push("pop 1".to_string());
                        }
                    } else {
                        self.stack.pop();
                        self.asm.push("pop 1".to_string());
                    }
                }
                AstNode::FnVar(vars) => {
                    if arg_types.len() != vars.len() {
                        log::error!(&format!(
                            "function argument count mismatch: expected {}, got {}",
                            arg_types.len(),
                            vars.len()
                        ));
                    }
                    for x in 0..vars.len() {
                        self.fn_var(vars[x].clone(), arg_types[x].clone());
                    }
                    self.fn_var(
                        RETURN_VAR.to_string(),
                        ArgType {
                            location: VarLocation::Stack,
                            dimensions: vec![],
                        },
                    );
                }
                AstNode::Rtrn(expr) => {
                    self.return_expr(expr);
                }
                AstNode::StaticDef(name, expr) => {
                    // we must be able to fully evaluate
                    // the static at compile time
                    // e.g. the expr must contain only
                    // Expr::Lit and Expr::Val containing other statics
                    self.static_var(name, expr);
                }
                AstNode::If(expr, block_ast) => {
                    self.eval(expr, false);
                    let block_name = format!("block_____{}", self.compiler_state.block_counter);
                    self.compiler_state.block_counter += 1;
                    self.call_block(&block_name);
                    // vm.eval(expr1);
                    // vm.eval(expr2);
                    // push 0 to the stack based on the bool_op
                    let start_asm_len = self.asm.len();
                    self.begin_block();
                    // blocks can't take args
                    self.eval_ast(block_ast, vec![]);
                    self.end_block(None);
                    // pull the resulting asm as the block asm
                    let mut block_asm = self.asm.drain(start_asm_len..).collect::<Vec<String>>();
                    block_asm.insert(0, format!("{block_name}:"));
                    block_asm.push("return".to_string());
                    self.compiler_state.block_fn_asm.push(block_asm);
                }
                AstNode::Loop(expr, block_ast) => {
                    if let Some(_) = self.eval(expr, false) {
                        log::error!("loop condition must be a stack variable");
                    }
                    let block_counter_name =
                        format!("block_____{}_counter", self.compiler_state.block_counter);
                    let block_name = format!("loop_____{}", self.compiler_state.block_counter);
                    self.compiler_state.block_counter += 1;
                    self.call_block(&block_name);
                    // push 0 to the stack based on the bool_op
                    let start_asm_len = self.asm.len();
                    self.begin_block();
                    self.vars.insert(
                        block_counter_name.clone(),
                        Var {
                            stack_index: Some(self.stack.len()),
                            block_index: self.block_depth,
                            location: VarLocation::Stack,
                            memory_index: None,
                            dimensions: vec![],
                            value: None,
                        },
                    );
                    // blocks can't take args
                    self.asm.push(format!("{block_name}:"));
                    self.eval_ast(block_ast, vec![]);
                    if let Some(_) = self.eval(Expr::Val(block_counter_name.clone(), vec![]), false)
                    {
                        panic!("unexpected: loop counter is not scalar");
                    }
                    // pull the resulting asm as the block asm
                    self.asm.push("push -1".to_string());
                    self.asm.push("add".to_string());

                    self.asm.push("dup 0".to_string());
                    self.stack.push("dup".to_string());
                    self.asm
                        .push(format!("swap {}", self.stack_index(&block_counter_name)));
                    self.asm.push("pop 1".to_string());
                    self.stack.pop();
                    self.end_block(Some(block_counter_name.clone()));
                    self.asm.push("skiz".to_string());
                    self.stack.pop();
                    self.asm.push("recurse".to_string());
                    self.stack.pop();
                    self.vars.remove(&block_counter_name);
                    self.asm.push("pop 1".to_string()); // pop the loop counter
                    self.asm.push("return".to_string());
                    let block_asm = self.asm.drain(start_asm_len..).collect::<Vec<String>>();
                    self.compiler_state.block_fn_asm.push(block_asm);
                }
            }
        }
    }

    // call this with an offset on the stack
    fn load_scalar(&mut self, v: &Var, offset: Option<usize>) {
        match v.location {
            VarLocation::Stack => {
                if offset.is_some() {
                    log::error!(&format!("attempting to access stack variable by index"));
                }
                let mut out = vec![format!("dup {}", v.stack_index.unwrap())];
                self.stack.push("sfaf".to_string());
                self.asm.append(&mut out);
            }
            VarLocation::Memory => {
                if offset.is_some() {
                    self.asm.push(format!("push {}", offset.unwrap()));
                    self.stack.push("".to_string());
                } else {
                    self.calc_vec_offset(&v.dimensions, &vec![]);
                }
                // we're accessing a scalar, move it to the stack
                if let Some(mem_index) = v.memory_index {
                    self.asm.push(format!("push {}", mem_index));
                    self.asm.push("add".to_string());
                    self.asm.push(format!("read_mem 1"));
                    self.asm.push(format!("pop 1"));
                } else if let Some(stack_index) = v.stack_index {
                    self.asm
                        .push(format!("dup {}", self.stack.len() - stack_index));
                    self.asm.push("add".to_string());
                    self.asm.push(format!("read_mem 1"));
                    self.asm.push(format!("pop 1"));
                } else {
                    panic!("unexpected: variable has no memory or stack index");
                }
            }
            VarLocation::Static => {
                // should not have an offset on the stack
                if v.value.is_none() {
                    panic!("static variable does not have values defined");
                }
                if offset.is_none() {
                    panic!("static variable access must have an offset");
                }
                let value = v.value.as_ref().unwrap();
                self.asm.push(format!("push {}", value[offset.unwrap()]));
                self.stack.push("unknown".to_string());
            }
        }
    }

    // load a stack, memory or static variable and return it
    fn load_variable(&mut self, v: &Var, indices: &Vec<Expr>) -> Option<Var> {
        match v.location {
            VarLocation::Stack => {
                if indices.len() > 0 {
                    log::error!(&format!(
                        "attempting to access stack variable \"unknown\" by index"
                    ));
                }
                let mut out = vec![format!("dup {}", self.stack.len() - v.stack_index.unwrap())];
                self.stack.push("sfaf".to_string());
                self.asm.append(&mut out);
                return None;
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
                    self.calc_vec_offset(&v.dimensions, indices);
                    // we're accessing a scalar, move it to the stack
                    if let Some(mem_index) = v.memory_index {
                        self.asm.push(format!("push {}", mem_index));
                        self.asm.push("add".to_string());
                        self.asm.push(format!("read_mem 1"));
                        self.asm.push(format!("pop 1"));
                    } else if let Some(stack_index) = v.stack_index {
                        self.asm
                            .push(format!("dup {}", self.stack.len() - stack_index));
                        self.asm.push("add".to_string());
                        self.asm.push(format!("read_mem 1"));
                        self.asm.push(format!("pop 1"));
                    } else {
                        panic!("unexpected: variable has no memory or stack index");
                    }
                    return None;
                } else {
                    let offset = VM::calc_vec_offset_static(&v.dimensions, indices);
                    // we're accessing a vec/mat, leave it in memory
                    if let Some(mem_index) = v.memory_index {
                        if v.stack_index.is_some() {
                            panic!(
                                "memory variable should not have a stack and memory index defined"
                            );
                        }
                        return Some(Var {
                            stack_index: None,
                            block_index: v.block_index,
                            location: v.location.clone(),
                            memory_index: Some(mem_index + offset),
                            dimensions: v.dimensions[indices.len()..].to_vec(),
                            value: None,
                        });
                    } else if offset == 0 {
                        if v.stack_index.is_none() {
                            panic!("memory variable has neither stack nor memory index defined");
                        }
                        return Some(Var {
                            stack_index: v.stack_index,
                            block_index: v.block_index,
                            location: v.location.clone(),
                            memory_index: None,
                            dimensions: v.dimensions[indices.len()..].to_vec(),
                            value: None,
                        });
                    } else {
                        panic!("cannot access stack addressed memory variable by index");
                    }
                }
            }
            VarLocation::Static => {
                if v.value.is_none() {
                    panic!("static variable does not have values defined");
                }
                let value = v.value.as_ref().unwrap();
                if value.len() == 1 {
                    // a static that can be represented on the stack
                    self.stack.push("unknown".to_string());
                    self.asm.push(format!("push {}", value[0]));
                    return None;
                }
                let offset = VM::calc_vec_offset_static(&v.dimensions, indices);
                if indices.len() == v.dimensions.len() {
                    // we're accessing a scalar, move it to the stack
                    self.stack.push("unknown".to_string());
                    self.asm.push(format!("push {}", value[offset]));
                    return None;
                }
                // we're accessing a vec/mat, leave it in memory
                // or in this case (static) in the VM value array
                return Some(Var {
                    stack_index: v.stack_index,
                    block_index: v.block_index,
                    location: v.location.clone(),
                    memory_index: v.memory_index,
                    dimensions: v.dimensions[indices.len()..].to_vec(),
                    value: Some(value[offset..].to_vec()),
                });
            }
        }
    }

    // load two variables onto the stack from wherever they are
    // then apply `ops` to them
    fn op_elements(&mut self, v1: &Var, v2: &Var, out: &Var, ops: &mut Vec<String>) {
        let total_len = VM::dimensions_to_len(v1.dimensions.clone());
        // TODO: assert equal shape
        for x in 0..total_len {
            self.load_scalar(v1, Some(x));
            // make sure the RHS is read second so the inv operation
            // is applied to the correct operand
            self.load_scalar(v2, Some(x));
            // v1 and v2 are operated on and a single output
            // remains
            self.stack.pop();

            self.asm.append(&mut ops.clone());

            if let Some(memory_index) = out.memory_index {
                self.asm.push(format!("push {}", memory_index + x));
            } else if let Some(stack_index) = out.stack_index {
                self.asm
                    .push(format!("dup {}", self.stack.len() - stack_index));
                self.asm.push(format!("push {x}"));
                self.asm.push("add".to_string());
            }
            // the final output is written to memory below and
            // removed from the stack
            self.stack.pop();
            self.asm.push(format!("write_mem 1"));
            self.asm.push(format!("pop 1"));
        }
    }
}
