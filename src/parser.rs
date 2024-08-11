use self::AstNode::*;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum AstNode {
    // a variable argument to a function call
    FnVar(Vec<String>),
    // a let defintion, const definition, or if statement
    Stmt(String, bool, Expr),
    ExprUnassigned(Expr),
    Rtrn(Expr),
    Const(String, Expr),
    If(Expr, Vec<AstNode>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    VecVec(Vec<Expr>),
    VecLit(Vec<u64>),
    Lit(u64),
    Val(String, Vec<u64>),
    FnCall(String, Vec<Box<Expr>>),
    NumOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>,
    },
    BoolOp {
        lhs: Box<Expr>,
        bool_op: BoolOp,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum BoolOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Sub,
    Inv,
    Mul,
}

#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to project `src`
struct PestParser;

pub struct AshParser {
    pub ast: Vec<AstNode>,
    pub fn_names: HashMap<String, u64>,
}

impl AshParser {
    pub fn parse(source: &str) -> Self {
        let mut out = Self {
            ast: Vec::new(),
            fn_names: HashMap::new(),
        };

        let pairs = PestParser::parse(Rule::program, source).unwrap();
        for pair in pairs {
            match pair.as_rule() {
                Rule::fn_header => {
                    // parse the function header which includes argument
                    // if invocation started in the file no arguments
                    // should be accepted. Instead the argv function should
                    // be used
                    let pair = pair.into_inner();
                    let mut vars: Vec<String> = Vec::new();
                    for v in pair {
                        vars.push(v.as_str().to_string());
                    }
                    // let pair.next().unwrap()
                    out.ast.push(FnVar(vars));
                }
                Rule::stmt => {
                    let mut pair = pair.into_inner();
                    let next = pair.next().unwrap();
                    let ast = out.build_ast_from_pair(next);
                    out.ast.push(ast);
                }
                Rule::return_stmt => {
                    let mut pair = pair.into_inner();
                    let next = pair.next().unwrap();
                    let expr = out.build_expr_from_pair(next);
                    out.ast.push(Rtrn(expr))
                }
                _ => {}
            }
        }
        out
    }

    fn mark_fn_call(&mut self, name: String) {
        let count = self.fn_names.entry(name).or_insert(0);
        *count += 1;
    }

    fn build_ast_from_pair(&mut self, pair: pest::iterators::Pair<Rule>) -> AstNode {
        match pair.as_rule() {
            Rule::function_call => ExprUnassigned(self.build_expr_from_pair(pair)),
            Rule::var_def => {
                // get vardef
                let mut pair = pair.into_inner();
                let next = pair.next().unwrap();
                let mut varpair = next.into_inner();
                let name;
                let is_let;
                if varpair.len() == 2 {
                    // it's a let assignment
                    varpair.next();
                    name = varpair.next().unwrap();
                    is_let = true;
                } else if varpair.len() == 1 {
                    // it's a regular assignment
                    name = varpair.next().unwrap();
                    is_let = false;
                } else {
                    panic!("invalid varpait");
                }

                let n = pair.next().unwrap();
                Stmt(
                    name.as_str().to_string(),
                    is_let,
                    self.build_expr_from_pair(n),
                )
            }
            Rule::const_def => {
                let mut pair = pair.into_inner();
                let name = pair.next().unwrap();
                let expr = pair.next().unwrap();
                Const(name.as_str().to_string(), self.build_expr_from_pair(expr))
            }
            Rule::if_stmt => {
                let mut pair = pair.into_inner();
                let bool_expr = pair.next().unwrap();
                let mut bool_expr_pair = bool_expr.into_inner();
                let expr1 = self.build_expr_from_pair(bool_expr_pair.next().unwrap());
                let bool_op = match bool_expr_pair.next().unwrap().as_rule() {
                    Rule::equal => BoolOp::Equal,
                    Rule::not_equal => BoolOp::NotEqual,
                    Rule::gt => BoolOp::GreaterThan,
                    Rule::lt => BoolOp::LessThan,
                    _ => panic!("invalid bool op"),
                };
                let expr2 = self.build_expr_from_pair(bool_expr_pair.next().unwrap());
                let block = pair.next().unwrap();
                let block_inner = block.into_inner();
                let block_ast = block_inner
                    .map(|v| match v.as_rule() {
                        Rule::stmt => {
                            let mut pair = v.into_inner();
                            let next = pair.next().unwrap();
                            self.build_ast_from_pair(next)
                        }
                        _ => panic!("invalid expression in block"),
                    })
                    .collect();
                If(
                    Expr::BoolOp {
                        lhs: Box::new(expr1),
                        bool_op,
                        rhs: Box::new(expr2),
                    },
                    block_ast,
                )
            }
            unknown_expr => panic!("Unexpected expression: {:?}", unknown_expr),
        }
    }

    fn build_expr_from_pair(&mut self, pair: pest::iterators::Pair<Rule>) -> Expr {
        match pair.as_rule() {
            Rule::vec => {
                let mut pair = pair.into_inner();
                let next = pair.next().unwrap();
                if next.as_rule() == Rule::vec {
                    let mut out: Vec<Expr> = Vec::new();
                    out.push(self.build_expr_from_pair(next.clone()));
                    for next in pair {
                        out.push(self.build_expr_from_pair(next.clone()));
                        // next = pair.next().unwrap();
                    }
                    return Expr::VecVec(out);
                } else {
                    let mut out: Vec<u64> = Vec::new();
                    out.push(next.as_str().parse::<u64>().unwrap());
                    for next in pair {
                        out.push(next.as_str().parse::<u64>().unwrap());
                    }
                    return Expr::VecLit(out);
                }
            }
            Rule::function_call => {
                let mut pair = pair.into_inner();
                let next = pair.next().unwrap();
                let arg_pair = pair.next().unwrap().into_inner();
                let mut vars: Vec<Box<Expr>> = Vec::new();
                for v in arg_pair {
                    vars.push(Box::new(self.build_expr_from_pair(v)));
                }
                let fn_name = next.as_str().to_string();
                self.mark_fn_call(fn_name.clone());
                Expr::FnCall(fn_name, vars)
            }
            Rule::atom => {
                let mut pair = pair.into_inner();
                let n = pair.next().unwrap();
                match n.as_rule() {
                    Rule::varname => {
                        let name = n.as_str().to_string();
                        let mut indices: Vec<u64> = Vec::new();
                        while let Some(v) = pair.next() {
                            match v.as_rule() {
                                Rule::literal_dec => {
                                    indices.push(v.as_str().parse::<u64>().unwrap())
                                }
                                _ => panic!("unexpected rule in atom"),
                            }
                        }
                        Expr::Val(name, indices)
                    }
                    Rule::literal_dec => Expr::Lit(n.as_str().parse::<u64>().unwrap()),
                    _ => panic!("invalid atom"),
                }
                // Expr::Val(pair.next().unwrap().as_str().to_string())
            }
            Rule::expr => {
                let mut pair = pair.into_inner();
                let first_atom = pair.next().unwrap();
                if pair.len() == 0 {
                    return self.build_expr_from_pair(first_atom);
                    // return Expr::Val(first_atom.as_str().to_string());
                }
                let op = pair.next().unwrap();
                let rhs = pair.next().unwrap();
                Expr::NumOp {
                    lhs: Box::new(self.build_expr_from_pair(first_atom)),
                    op: match op.as_rule() {
                        Rule::add => Op::Add,
                        Rule::sub => Op::Sub,
                        Rule::mul => Op::Mul,
                        Rule::inv => Op::Inv,
                        _ => panic!("invalid op"),
                    },
                    rhs: Box::new(self.build_expr_from_pair(rhs)),
                }
            }
            unknown_expr => panic!("Unexpected expression: {:?}", unknown_expr),
        }
    }
}
