use crate::log;

use self::AstNode::*;
use anyhow::Result;
use pest::iterators::Pair;
use pest::iterators::Pairs;
use pest::Parser;
use pest::RuleType;
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
    Loop(Expr, Vec<AstNode>),
    EmptyVecDef(String, Vec<usize>),
    AssignVec(String, Vec<u64>, Expr),
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
pub struct PestParser;

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

        match PestParser::parse(Rule::program, source) {
            Ok(pairs) => {
                out.build_ast_from_lines(pairs).unwrap_or_else(|e| {
                    log::compile_error(format!("error building program ast: {e}").as_str(), None);
                    std::process::exit(1);
                });
            }
            Err(e) => {
                log::parse_error(e);
                unreachable!();
            }
        }
        out
    }

    fn mark_fn_call(&mut self, name: String) {
        let count = self.fn_names.entry(name).or_insert(0);
        *count += 1;
    }

    pub fn next_or_error<'a, T: RuleType>(pairs: &'a mut Pairs<T>) -> Result<Pair<'a, T>> {
        if let Some(n) = pairs.next() {
            Ok(n)
        } else {
            anyhow::bail!("Expected next token but found none")
        }
    }

    fn build_ast_from_lines(&mut self, pairs: Pairs<Rule>) -> Result<()> {
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
                    self.ast.push(FnVar(vars));
                }
                Rule::stmt => {
                    let mut pair = pair.into_inner();
                    let next = AshParser::next_or_error(&mut pair)?;
                    let ast = self.build_ast_from_pair(next)?;
                    self.ast.push(ast);
                }
                Rule::return_stmt => {
                    let mut pair = pair.into_inner();
                    let next = AshParser::next_or_error(&mut pair)?;
                    let expr = self.build_expr_from_pair(next)?;
                    self.ast.push(Rtrn(expr));
                }
                Rule::EOI => {}
                _ => anyhow::bail!("unexpected line pair rule: {:?}", pair.as_rule()),
            }
        }
        Ok(())
    }

    fn build_ast_from_pair(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<AstNode> {
        match pair.as_rule() {
            Rule::var_index_assign => {
                let mut pair = pair.into_inner();
                let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                let mut indices: Vec<u64> = Vec::new();
                let mut expr = None;
                while let Some(v) = pair.next() {
                    match v.as_rule() {
                        Rule::literal_dec => indices.push(v.as_str().parse::<u64>().unwrap()),
                        Rule::expr => {
                            expr = Some(self.build_expr_from_pair(v)?);
                        }
                        _ => anyhow::bail!("unexpected rule in var_index_assign"),
                    }
                }
                if expr.is_none() {
                    anyhow::bail!("no expression found in var_index_assign");
                }
                Ok(AssignVec(name, indices, expr.unwrap()))
            }
            Rule::var_vec_def => {
                let mut pair = pair.into_inner();
                let _ = AshParser::next_or_error(&mut pair)?;
                let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                let mut indices: Vec<usize> = Vec::new();
                while let Some(v) = pair.next() {
                    match v.as_rule() {
                        Rule::literal_dec => indices.push(v.as_str().parse::<usize>().unwrap()),
                        _ => anyhow::bail!("unexpected rule in var_vec_def"),
                    }
                }
                Ok(EmptyVecDef(name, indices))
            }
            Rule::loop_stmt => {
                let mut pair = pair.into_inner();
                let iter_count = AshParser::next_or_error(&mut pair)?;
                let iter_count_expr = self.build_expr_from_pair(iter_count)?;
                let block = AshParser::next_or_error(&mut pair)?;
                let block_inner = block.into_inner();
                let block_ast = block_inner
                    .map(|v| match v.as_rule() {
                        Rule::stmt => {
                            let mut pair = v.into_inner();
                            let next = AshParser::next_or_error(&mut pair)?;
                            self.build_ast_from_pair(next)
                        }
                        _ => panic!("invalid expression in block"),
                    })
                    .collect::<Result<Vec<AstNode>>>()?;
                Ok(Loop(iter_count_expr, block_ast))
            }
            Rule::function_call => Ok(ExprUnassigned(self.build_expr_from_pair(pair)?)),
            Rule::var_def => {
                // get vardef
                let mut pair = pair.into_inner();
                let next = AshParser::next_or_error(&mut pair)?;
                let mut varpair = next.into_inner();
                let name;
                let is_let;
                if varpair.len() == 2 {
                    // it's a let assignment
                    AshParser::next_or_error(&mut varpair)?;
                    name = AshParser::next_or_error(&mut varpair)?.as_str().to_string();
                    is_let = true;
                } else if varpair.len() == 1 {
                    // it's a regular assignment
                    name = AshParser::next_or_error(&mut varpair)?.as_str().to_string();
                    is_let = false;
                } else {
                    panic!("invalid varpait");
                }

                let n = AshParser::next_or_error(&mut pair)?;
                Ok(Stmt(name, is_let, self.build_expr_from_pair(n)?))
            }
            Rule::const_def => {
                let mut pair = pair.into_inner();
                let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                let expr = AshParser::next_or_error(&mut pair)?;
                Ok(Const(
                    name.as_str().to_string(),
                    self.build_expr_from_pair(expr)?,
                ))
            }
            Rule::if_stmt => {
                let mut pair = pair.into_inner();
                let bool_expr = AshParser::next_or_error(&mut pair)?;
                let mut bool_expr_pair = bool_expr.into_inner();
                let expr1 =
                    self.build_expr_from_pair(AshParser::next_or_error(&mut bool_expr_pair)?)?;
                let bool_op = match AshParser::next_or_error(&mut bool_expr_pair)?.as_rule() {
                    Rule::equal => BoolOp::Equal,
                    Rule::not_equal => BoolOp::NotEqual,
                    Rule::gt => BoolOp::GreaterThan,
                    Rule::lt => BoolOp::LessThan,
                    _ => anyhow::bail!("invalid bool op"),
                };
                let expr2 =
                    self.build_expr_from_pair(AshParser::next_or_error(&mut bool_expr_pair)?)?;
                let block = AshParser::next_or_error(&mut pair)?;
                let block_inner = block.into_inner();
                let block_ast = block_inner
                    .map(|v| match v.as_rule() {
                        Rule::stmt => {
                            let mut pair = v.into_inner();
                            let next = AshParser::next_or_error(&mut pair)?;
                            self.build_ast_from_pair(next)
                        }
                        _ => anyhow::bail!("invalid expression in block"),
                    })
                    .collect::<Result<Vec<AstNode>>>()?;
                Ok(If(
                    Expr::BoolOp {
                        lhs: Box::new(expr1),
                        bool_op,
                        rhs: Box::new(expr2),
                    },
                    block_ast,
                ))
            }
            unknown_expr => anyhow::bail!("Unexpected expression: {:?}", unknown_expr),
        }
    }

    fn build_expr_from_pair(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<Expr> {
        match pair.as_rule() {
            Rule::vec => {
                let mut pair = pair.into_inner();
                let next = AshParser::next_or_error(&mut pair)?;
                if next.as_rule() == Rule::vec {
                    let mut out: Vec<Expr> = Vec::new();
                    out.push(self.build_expr_from_pair(next.clone())?);
                    for next in pair {
                        out.push(self.build_expr_from_pair(next.clone())?);
                        // next = pair.next().unwrap();
                    }
                    Ok(Expr::VecVec(out))
                } else {
                    let mut out: Vec<u64> = Vec::new();
                    out.push(next.as_str().parse::<u64>().unwrap());
                    for next in pair {
                        out.push(next.as_str().parse::<u64>().unwrap());
                    }
                    Ok(Expr::VecLit(out))
                }
            }
            Rule::function_call => {
                let mut pair = pair.into_inner();
                let next = AshParser::next_or_error(&mut pair)?;
                let fn_name = next.as_str().to_string();
                let arg_pair = AshParser::next_or_error(&mut pair)?.into_inner();
                let mut vars: Vec<Box<Expr>> = Vec::new();
                for v in arg_pair {
                    vars.push(Box::new(self.build_expr_from_pair(v)?));
                }
                self.mark_fn_call(fn_name.clone());
                Ok(Expr::FnCall(fn_name, vars))
            }
            Rule::atom => {
                let mut pair = pair.into_inner();
                let n = AshParser::next_or_error(&mut pair)?;
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
                        Ok(Expr::Val(name, indices))
                    }
                    Rule::literal_dec => Ok(Expr::Lit(n.as_str().parse::<u64>().unwrap())),
                    _ => anyhow::bail!("invalid atom"),
                }
            }
            Rule::expr => {
                let mut pair = pair.into_inner();
                if pair.len() == 1 {
                    return self.build_expr_from_pair(AshParser::next_or_error(&mut pair)?);
                    // return Expr::Val(first_atom.as_str().to_string());
                }
                let lhs =
                    Box::new(self.build_expr_from_pair(AshParser::next_or_error(&mut pair)?)?);
                let op = pair.next().unwrap();
                let rhs =
                    Box::new(self.build_expr_from_pair(AshParser::next_or_error(&mut pair)?)?);
                Ok(Expr::NumOp {
                    lhs,
                    op: match op.as_rule() {
                        Rule::add => Op::Add,
                        Rule::sub => Op::Sub,
                        Rule::mul => Op::Mul,
                        Rule::inv => Op::Inv,
                        _ => panic!("invalid op"),
                    },
                    rhs,
                })
            }
            unknown_expr => anyhow::bail!("Unexpected expression: {:?}", unknown_expr),
        }
    }
}
