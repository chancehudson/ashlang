use self::AstNode::*;
use crate::log;
use anyhow::Result;
use log::error;
use pest::iterators::Pair;
use pest::iterators::Pairs;
use pest::pratt_parser::Assoc;
use pest::pratt_parser::Op;
use pest::pratt_parser::PrattParser;
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
    StaticDef(String, Expr),
    If(Expr, Vec<AstNode>),
    Loop(Expr, Vec<AstNode>),
    EmptyVecDef(String, Vec<usize>),
    AssignVec(String, Vec<Expr>, Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    VecVec(Vec<Expr>),
    VecLit(Vec<String>),
    Lit(String),
    Val(String, Vec<Expr>),
    FnCall(String, Vec<Expr>),
    NumOp {
        lhs: Box<Expr>,
        op: NumOp,
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
pub enum NumOp {
    Add,
    Sub,
    Inv,
    Mul,
}

#[derive(Parser)]
#[grammar = "grammar.pest"] // relative to project `src`
pub struct AshPestParser;

pub struct AshParser {
    pub ast: Vec<AstNode>,
    pub fn_names: HashMap<String, u64>,
}

impl AshParser {
    pub fn parse(source: &str, name: &str) -> Self {
        let mut out = Self {
            ast: Vec::new(),
            fn_names: HashMap::new(),
        };

        match AshPestParser::parse(Rule::program, source) {
            Ok(pairs) => {
                out.build_ast_from_lines(pairs).unwrap_or_else(|e| {
                    error!(&format!("error building program ast: {e}"));
                });
            }
            Err(e) => {
                log::parse_error(e, name);
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
                let next = AshParser::next_or_error(&mut pair)?;
                let v = self.build_expr_from_pair(next)?;
                let name;
                let indices;
                match v {
                    Expr::Val(n, i) => {
                        name = n;
                        indices = i;
                    }
                    _ => {
                        anyhow::bail!("unexpected expr in var_index_assign: {:?}, expected Val", v)
                    }
                }
                let next = AshParser::next_or_error(&mut pair)?;
                let expr = self.build_expr_from_pair(next)?;
                Ok(AssignVec(name, indices, expr))
            }
            Rule::var_vec_def => {
                let mut pair = pair.into_inner();
                let _ = AshParser::next_or_error(&mut pair)?;
                let next = AshParser::next_or_error(&mut pair)?;
                let expr = self.build_expr_from_pair(next)?;
                match expr {
                    Expr::Val(name, indices) => {
                        let mut indices_static: Vec<usize> = Vec::new();
                        for i in indices {
                            match i {
                                Expr::Lit(v) => {
                                    indices_static.push(v.parse::<usize>().unwrap());
                                }
                                _ => {
                                    anyhow::bail!(
                                        "unexpected expr in var_vec_def: {:?}, expected Lit",
                                        i
                                    )
                                }
                            }
                        }
                        Ok(EmptyVecDef(name, indices_static))
                    }
                    _ => {
                        anyhow::bail!("unexpected expr in var_vec_def: {:?}, expected Val", expr)
                    }
                }
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
            Rule::static_def => {
                let mut pair = pair.into_inner();
                let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                let expr = AshParser::next_or_error(&mut pair)?;
                Ok(StaticDef(
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
            unknown_expr => anyhow::bail!(
                "Unable to build ast node, unexpected expression: {:?}",
                unknown_expr
            ),
        }
    }

    fn build_expr_from_pair(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<Expr> {
        match pair.as_rule() {
            Rule::var_indexed => {
                let mut pair = pair.into_inner();
                let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                let mut indices: Vec<Expr> = Vec::new();
                for v in pair {
                    indices.push(self.build_expr_from_pair(v)?);
                }
                Ok(Expr::Val(name, indices))
            }
            Rule::literal_dec => Ok(Expr::Lit(pair.as_str().to_string())),
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
                    let mut out: Vec<String> = Vec::new();
                    out.push(next.as_str().to_string());
                    for next in pair {
                        out.push(next.as_str().to_string());
                    }
                    Ok(Expr::VecLit(out))
                }
            }
            Rule::function_call => {
                let mut pair = pair.into_inner();
                let next = AshParser::next_or_error(&mut pair)?;
                let fn_name = next.as_str().to_string();
                let arg_pair = AshParser::next_or_error(&mut pair)?.into_inner();
                let mut vars: Vec<Expr> = Vec::new();
                for v in arg_pair {
                    vars.push(self.build_expr_from_pair(v)?);
                }
                self.mark_fn_call(fn_name.clone());
                Ok(Expr::FnCall(fn_name, vars))
            }
            Rule::atom => {
                let mut pair = pair.into_inner();
                let n = AshParser::next_or_error(&mut pair)?;
                match n.as_rule() {
                    Rule::function_call => Ok(self.build_expr_from_pair(n)?),
                    Rule::varname => Ok(Expr::Val(n.as_str().to_string(), vec![])),
                    Rule::var_indexed => {
                        let mut pair = n.into_inner();
                        let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                        let mut indices: Vec<Expr> = Vec::new();
                        for v in pair {
                            indices.push(self.build_expr_from_pair(v)?);
                        }
                        Ok(Expr::Val(name, indices))
                    }
                    Rule::literal_dec => Ok(Expr::Lit(n.as_str().to_string())),
                    _ => anyhow::bail!("invalid atom"),
                }
            }
            Rule::expr => {
                let mut pair = pair.into_inner();
                if pair.len() == 1 {
                    return self.build_expr_from_pair(AshParser::next_or_error(&mut pair)?);
                }
                let pratt = PrattParser::new()
                    .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
                    .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::inv, Assoc::Left));
                pratt
                    .map_primary(|primary| match primary.as_rule() {
                        Rule::atom => self.build_expr_from_pair(primary),
                        Rule::expr => self.build_expr_from_pair(primary),
                        _ => panic!("unexpected rule in pratt parser"),
                    })
                    .map_infix(|lhs, op, rhs| match op.as_rule() {
                        Rule::add => Ok(Expr::NumOp {
                            lhs: Box::new(lhs?),
                            op: NumOp::Add,
                            rhs: Box::new(rhs?),
                        }),
                        Rule::sub => Ok(Expr::NumOp {
                            lhs: Box::new(lhs?),
                            op: NumOp::Sub,
                            rhs: Box::new(rhs?),
                        }),
                        Rule::mul => Ok(Expr::NumOp {
                            lhs: Box::new(lhs?),
                            op: NumOp::Mul,
                            rhs: Box::new(rhs?),
                        }),
                        Rule::inv => Ok(Expr::NumOp {
                            lhs: Box::new(lhs?),
                            op: NumOp::Inv,
                            rhs: Box::new(rhs?),
                        }),
                        _ => unreachable!(),
                    })
                    .parse(pair)
            }
            unknown_expr => anyhow::bail!(
                "Unable to build expression, unexpected rule: {:?}",
                unknown_expr
            ),
        }
    }
}
