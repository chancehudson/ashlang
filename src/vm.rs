use crate::parser::{BoolOp, Expr, Op};
use std::collections::HashMap;

#[derive(Clone)]
enum VarLocation {
    Stack,
    Memory,
}
#[derive(Clone)]

pub struct Var {
    stack_index: usize,
    block_index: usize,
    location: VarLocation,
    memory_index: usize,
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
}
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
pub struct VM {
    // represents the contents of the stack
    pub stack: Vec<String>,
    // name of variable keyed to offset in the stack
    // offsets are based on zero so they stay correct
    // as items are pushed/popped on the stack
    pub vars: HashMap<String, Var>,
    // constants stored keyed to their value
    pub consts: HashMap<String, u64>,
    // compiled assembly
    pub asm: Vec<String>,
    // map of function name to number of invocations
    // the compiler needs this stat
    // it's not used in vm
    pub fn_calls: HashMap<String, u64>,
    // track whether the current vm has returned
    // this means the stack is cleared of variables
    pub has_returned: bool,
    // tracks the current logic block depth
    // the executor can see variables in higher blocks
    // but not lower blocks
    pub block_depth: usize,
    pub memory_index: usize,
}

impl VM {
    pub fn new() -> Self {
        VM {
            vars: HashMap::new(),
            stack: Vec::new(),
            asm: Vec::new(),
            consts: HashMap::new(),
            fn_calls: HashMap::new(),
            has_returned: false,
            block_depth: 0,
            memory_index: 0,
        }
    }

    pub fn from_vm(vm: &Self) -> Self {
        VM {
            vars: vm.vars.clone(),
            stack: vm.stack.clone(),
            asm: Vec::new(),
            consts: vm.consts.clone(),
            fn_calls: HashMap::new(),
            has_returned: false,
            block_depth: vm.block_depth,
            memory_index: vm.memory_index,
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
    pub fn end_block(&mut self) {
        if self.block_depth == 0 {
            panic!("cannot exit execution root");
        }
        // find all variables in this depth
        // and remove them from the stack
        let entries_to_remove = self
            .vars
            .iter()
            .filter(|(_k, v)| v.block_index == self.block_depth)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<(String, Var)>>();
        if entries_to_remove.is_empty() {
            self.block_depth -= 1;
            return;
        }
        self.asm.push(format!("pop {}", entries_to_remove.len()));
        // TODO: don't iterate here
        for _ in 0..entries_to_remove.len() {
            self.stack.pop();
        }
        for (k, _v) in entries_to_remove {
            self.vars.remove(&k);
        }
        self.block_depth -= 1;
    }

    // define a constant that will be available in
    // the current VM object
    pub fn const_var(&mut self, name: String, expr: Expr) {
        // check for duplicate var names
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("name is not unique");
        }
        match expr {
            Expr::Lit(v) => {
                self.consts.insert(name, v);
            }
            Expr::Val(ref_name) => {
                if self.consts.contains_key(&ref_name) {
                    self.consts
                        .insert(name, *self.consts.get(&ref_name).unwrap());
                } else if self.vars.contains_key(&ref_name) {
                    panic!("dynamically evaluated consts not supported");
                } else {
                    panic!("unknown variable");
                }
            }
            Expr::NumOp {
                lhs: _,
                op: _,
                rhs: _,
            } => {
                panic!("numerical operations in constants is not yet supported");
            }
            Expr::FnCall(_name, _vars) => {
                panic!("constant expression functions not implemented");
            }
            Expr::BoolOp {
                lhs: _,
                bool_op: _,
                rhs: _,
            } => {
                panic!("boolean operations in constants is not supported");
            }
            Expr::VecLit(_v) => panic!("vector literals are not supported in consts"),
            Expr::VecVec(_v) => panic!("vector literals are not supported in consts"),
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
        self.eval(expr);
        // when we're done executing a block we clear
        // everything on the stack so that when we return
        // to the previous position the stack is in a
        // predictable state
        if self.vars.is_empty() {
            self.has_returned = true;
            return;
        }
        self.asm.push(format!("swap {}", self.vars.len()));
        self.asm.push(format!("pop {}", self.vars.len()));
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
        self.asm.push(format!("pop {}", self.vars.len()));
        self.asm.push("push 0".to_string());
        self.has_returned = true;
    }

    // defines a new mutable variable in the current block scope
    pub fn let_var(&mut self, name: String, expr: Expr) {
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("var is not unique");
        }
        match &expr {
            Expr::VecLit(_) | Expr::VecVec(_) => {
                let (dimensions, vec) = self.build_var_from_ast_vec(expr);
                let v = Var {
                    stack_index: 0,
                    block_index: self.block_depth,
                    location: VarLocation::Memory,
                    memory_index: self.memory_index,
                    dimensions,
                };
                // println!("{}", vec);
                self.vars.insert(name, v);
                // put the variable in memory
                for vv in vec.clone() {
                    self.asm.push(format!("push {vv}"))
                }
                self.asm.push(format!("push {}", self.memory_index));
                self.memory_index += vec.len();
                // TODO: batch insertion
                for _ in vec.iter().rev() {
                    self.asm.push(format!("write_mem 1"))
                }
                // pop the updated ram pointer
                // track memory index in this VM instead
                self.asm.push("pop 1".to_string());
            }
            _ => {
                let out = self.eval(expr);
                if out.is_none() {
                    // stack based variable
                    self.vars.insert(
                        name,
                        Var {
                            stack_index: self.stack.len(),
                            block_index: self.block_depth,
                            location: VarLocation::Stack,
                            memory_index: 0,
                            dimensions: vec![],
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
    pub fn fn_var(&mut self, name: String) {
        if self.vars.contains_key(&name) || self.consts.contains_key(&name) {
            panic!("var is not unique");
        }
        self.stack.push(name.clone());
        self.vars.insert(
            name,
            Var {
                stack_index: self.stack.len(),
                block_index: self.block_depth,
                dimensions: vec![1],
                location: VarLocation::Stack,
                memory_index: 0,
            },
        );
    }

    // assign a variable that already exists
    //
    // do this by evaluating the expression on
    // the top of the stack, then swapping with
    // the real location on the stack, and popping
    // the swapped value
    pub fn set_var(&mut self, name: String, expr: Expr) {
        if !self.vars.contains_key(&name) {
            panic!("var does not exist {name}");
        }
        // new value is on the top of the stack
        self.eval(expr);
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
            self.stack.len() - var.stack_index
        } else {
            panic!("unknown var");
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

    // evaluate an AST expression
    //
    // manipulates the execution stack and tracks
    // changes in the local stack
    //
    // Optionally returns a variable reference to be used by the caller
    // if the return value is non-null the asm has not been mutated
    pub fn eval(&mut self, expr: Expr) -> Option<Var> {
        match &expr {
            Expr::VecLit(_v) => {
                panic!("vector literals must be assigned before operation");
            }
            Expr::VecVec(_v) => {
                panic!("matrix literals must be assigned before operation");
            }
            Expr::FnCall(name, vars) => {
                if let Some(c) = self.fn_calls.get_mut(name) {
                    *c += 1;
                } else {
                    self.fn_calls.insert(name.clone(), 1);
                }
                // we push these but don't pop them here
                // the destination function will handle that
                for v in vars.iter().rev().collect::<Vec<&String>>() {
                    self.eval(Expr::Val(v.clone()));
                }
                // we pop them off the virtual stack assuming
                // that the callee will do the _actual_ popping
                for _ in 0..vars.len() {
                    self.stack.pop();
                }
                // we push 1 element onto the virtual stack
                // this element is the return value of the function
                // or 0 if the function does not explicitly
                // return a value
                self.stack.push(name.clone());
                self.asm.push(format!("call {name}"));
                return None;
            }
            Expr::Val(name) => {
                // if the val is a constant we push to stack
                if self.vars.contains_key(name) {
                    let v = self.vars.get(name).unwrap();
                    match v.location {
                        VarLocation::Stack => {
                            let mut out = vec![format!("dup {}", self.stack_index(name))];
                            self.stack.push(name.clone());
                            self.asm.append(&mut out);
                            return None;
                        }
                        VarLocation::Memory => {
                            let v = self.vars.get(name).unwrap();
                            return Some(v.clone());
                        }
                    }
                } else if self.consts.contains_key(name) {
                    self.stack.push(name.clone());
                    self.asm
                        .push(format!("push {}", self.consts.get(name).unwrap()));
                    return None;
                } else {
                    panic!("unknown variable: {name}");
                }
            }
            Expr::Lit(v) => {
                self.stack.push(v.to_string());
                self.asm.push(format!("push {}", v));
                return None;
            }
            Expr::NumOp { lhs, op, rhs } => {
                // only allow variables of same dimensions
                // for now
                let l = (*lhs).clone();
                let lv = self.eval(*l);
                let r = (*rhs).clone();
                let rv = self.eval(*r);
                if lv.is_none() != rv.is_none() {
                    panic!("cannot operate on stack types and memory types");
                }
                if !lv.is_none() {
                    let lvu = lv.unwrap();
                    let rvu = rv.unwrap();
                    if lvu.dimensions.len() != rvu.dimensions.len() {
                        panic!("attempting to operate on variables of mismatched width");
                    }
                    let mut total_len = 0;
                    for x in 0..lvu.dimensions.len() {
                        if lvu.dimensions[x] != rvu.dimensions[x] {
                            panic!("attempting to operate on variables of mismatched height");
                        }
                        total_len += lvu.dimensions[x];
                    }
                    let out_v = Var {
                        stack_index: 0,
                        block_index: self.block_depth,
                        location: VarLocation::Memory,
                        memory_index: self.memory_index,
                        dimensions: lvu.dimensions.clone(),
                    };
                    self.memory_index += total_len;
                    // operate on elements in a vector stored in memory
                    // store the result in memory
                    // TODO: batch memory read/write operations
                    let mut op_elements = |v1: &Var, v2: &Var, out: &Var, ops: &mut Vec<String>| {
                        let mut v1_mem_offset = v1.memory_index;
                        let mut v2_mem_offset = v2.memory_index;
                        let mut out_mem_offset = out.memory_index;
                        for x in 0..lvu.dimensions.len() {
                            for _ in 0..lvu.dimensions[x] {
                                self.asm.push(format!("push {}", v1_mem_offset));
                                self.asm.push(format!("read_mem 1"));
                                self.asm.push(format!("pop 1"));
                                // make sure the RHS is read second so the inv operation
                                // is applied to the correct operand
                                self.asm.push(format!("push {}", v2_mem_offset));
                                self.asm.push(format!("read_mem 1"));
                                self.asm.push(format!("pop 1"));

                                self.asm.append(&mut ops.clone());

                                self.asm.push(format!("push {}", out_mem_offset));
                                self.asm.push(format!("write_mem 1"));
                                self.asm.push(format!("pop 1"));
                                v1_mem_offset += 1;
                                v2_mem_offset += 1;
                                out_mem_offset += 1;
                            }
                        }
                    };
                    match op {
                        Op::Add => {
                            op_elements(&lvu, &rvu, &out_v, &mut vec![format!("add")]);
                        }
                        Op::Mul => {
                            op_elements(&lvu, &rvu, &out_v, &mut vec![format!("mul")]);
                        }
                        Op::Sub => {
                            op_elements(
                                &lvu,
                                &rvu,
                                &out_v,
                                &mut vec![format!("push -1"), format!("mul"), format!("add")],
                            );
                        }
                        Op::Inv => {
                            op_elements(
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
                    Op::Add => {
                        self.stack.pop();
                        self.asm.push(format!("add"));
                    }
                    Op::Sub => {
                        self.stack.pop();
                        self.asm.append(&mut vec![
                            format!("push -1"),
                            format!("mul"),
                            format!("add"),
                        ]);
                    }
                    Op::Mul => {
                        self.stack.pop();
                        self.asm.push(format!("mul"));
                    }
                    Op::Inv => {
                        self.stack.pop();
                        self.asm
                            .append(&mut vec![format!("invert"), format!("mul")]);
                    }
                }
                return None;
            }
            Expr::BoolOp { lhs, bool_op, rhs } => {
                let l = (*lhs).clone();
                self.eval(*l);
                let r = (*rhs).clone();
                self.eval(*r);
                match bool_op {
                    BoolOp::Equal => {
                        self.stack.pop();
                        self.asm.append(&mut vec![format!("eq"), format!("skiz")]);
                    }
                    BoolOp::NotEqual => {
                        // TODO: possibly use the eq instruction
                        self.stack.pop();
                        self.asm.append(&mut vec![
                            format!("push -1"),
                            format!("mul"),
                            format!("add"),
                            format!("skiz"),
                        ]);
                    }
                    _ => panic!("boolean operation not supported"),
                }
                return None;
            }
            _ => panic!("unexpect expression in vm eval"),
        };
    }

    fn stack_num_op() -> Option<Var> {
        None
    }
}
