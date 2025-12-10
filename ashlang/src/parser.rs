use std::collections::HashMap;

use anyhow::Result;
use pest::Parser;
use pest::RuleType;
use pest::iterators::Pair;
use pest::iterators::Pairs;
use pest::pratt_parser::Assoc;
use pest::pratt_parser::Op;
use pest::pratt_parser::PrattParser;
use pest_derive::Parser;

use crate::*;
use log::error;

/// A top level AST node. Each of these generally corresponds to
/// a single line of source code.
#[derive(Debug, Clone)]
pub enum AstNode {
    // a variable argument to a function call
    FnVar(Vec<String>),
    // a let definition, static definition, or a variable assignment
    Stmt(String, Option<VarLocation>, Expr),
    ExprUnassigned(Expr),
    Rtrn(Expr),
    If(Expr, Vec<AstNode>),
    // name of the precompile, tuple inputs, optional body
    Precompile(String, Vec<Expr>, Option<Vec<AstNode>>),
    EmptyVecDef(VarLocation, String, Box<Expr>),

    // direct variable assignment without index access
    AssignVar(String, Expr),
    // name, index being assigned, Expr
    AssignVarIndex(String, Box<Expr>, Expr),
}

/// An expression in the AST. Many expressions may appear on a single
/// line.
#[derive(Debug, Clone)]
pub enum Expr {
    VecLit(Vec<String>),
    Lit(String),
    ValVar(String),
    ValVarIndex(String, Box<Expr>),
    FnCall(String, Vec<Expr>),
    // a precompile that returns a value
    Precompile(String, Vec<Expr>, Option<Vec<AstNode>>),
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

/// Operations that output a boolean result.
#[derive(Debug, Clone)]
pub enum BoolOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
}

/// Operations that output a numerical result.
#[derive(Debug, Clone)]
pub enum NumOp {
    Add,
    Sub,
    Inv,
    Mul,
}

pub use internal::AshParser;

// pest living up to its name
// jk this is a totally reasonable thing
mod internal {
    use super::*;
    #[derive(Parser)]
    #[grammar = "grammar.pest"] // relative to project `src`
    struct AshPestParser;

    /// Parses an ashlang source file into an AST and
    /// map of function names called by the source file.
    #[derive(Clone)]
    pub struct AshParser {
        pub src: String,
        pub ast: Vec<AstNode>,
        pub fn_names: HashMap<String, u64>,
        pub entry_fn_name: String,
    }

    impl AshParser {
        /// Take a source file and a function name and output
        /// an instance of the parser.
        pub fn parse(source: &str, name: &str) -> Result<Self> {
            // append a new line to all source strings because
            // they aren't necessarily unix compatible files
            let source = format!("{source}\n");
            let mut out = Self {
                src: source.clone(),
                ast: Vec::new(),
                fn_names: HashMap::new(),
                entry_fn_name: name.to_string(),
            };

            match AshPestParser::parse(Rule::program, &source) {
                Ok(pairs) => {
                    let ast = out.build_ast_from_lines(pairs);
                    if let Err(e) = ast {
                        return error!(&format!("error building program ast: {e}"));
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(log::parse_error(e, name)));
                }
            }
            Ok(out)
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
                        self.ast.push(AstNode::FnVar(vars));
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
                        self.ast.push(AstNode::Rtrn(expr));
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
                    let index_maybe;
                    match v {
                        Expr::ValVar(n) => {
                            name = n;
                            index_maybe = None;
                        }
                        Expr::ValVarIndex(n, i) => {
                            name = n;
                            index_maybe = Some(i);
                        }
                        _ => {
                            anyhow::bail!(
                                "unexpected expr in var_index_assign: {:?}, expected Val",
                                v
                            )
                        }
                    }
                    let next = AshParser::next_or_error(&mut pair)?;
                    let expr = self.build_expr_from_pair(next)?;
                    match index_maybe {
                        Some(index) => Ok(AstNode::AssignVarIndex(name, index, expr)),
                        None => Ok(AstNode::AssignVar(name, expr)),
                    }
                }
                Rule::var_vec_def => {
                    let mut pair = pair.into_inner();
                    let var_location = AshParser::next_or_error(&mut pair)?;
                    let location = match var_location.as_rule() {
                        Rule::let_r => VarLocation::Witness,
                        Rule::static_r => VarLocation::Static,
                        _ => {
                            anyhow::bail!(
                                "unexpected rule in vector definition: {:?}",
                                var_location.as_rule()
                            )
                        }
                    };
                    let next = AshParser::next_or_error(&mut pair)?;
                    let expr = self.build_expr_from_pair(next)?;
                    match expr {
                        Expr::ValVarIndex(name, index) => {
                            Ok(AstNode::EmptyVecDef(location, name, index))
                        }
                        _ => {
                            anyhow::bail!(
                                "unexpected expr in var_vec_def: {:?}, expected Val",
                                expr
                            )
                        }
                    }
                }
                Rule::precompile_stmt => {
                    let mut pair = pair.into_inner();
                    let precompile_name = AshParser::next_or_error(&mut pair)?;
                    let precompile_name = precompile_name.as_str().to_string();

                    let mut args = Vec::default();
                    let mut block_maybe = None;
                    while let Some(next) = pair.next() {
                        match next.as_rule() {
                            Rule::expr => {
                                args.push(self.build_expr_from_pair(next)?);
                            }
                            Rule::block => {
                                let block_inner = next.into_inner();
                                let block_ast = block_inner
                                    .map(|v| match v.as_rule() {
                                        Rule::stmt => {
                                            let mut pair = v.into_inner();
                                            let next = AshParser::next_or_error(&mut pair)?;
                                            self.build_ast_from_pair(next)
                                        }
                                        _ => Err(anyhow::anyhow!(
                                            "non-statement in precompile block not allowed"
                                        )),
                                    })
                                    .collect::<Result<Vec<AstNode>>>()?;
                                block_maybe = Some(block_ast);
                            }
                            _ => anyhow::bail!(
                                "ashlang: unexpected rule in precompile statement: {:?}",
                                next.as_rule()
                            ),
                        }
                    }
                    Ok(AstNode::Precompile(precompile_name, args, block_maybe))
                }
                Rule::function_call => {
                    Ok(AstNode::ExprUnassigned(self.build_expr_from_pair(pair)?))
                }
                Rule::var_def => {
                    // get vardef
                    let mut pair = pair.into_inner();
                    let next = AshParser::next_or_error(&mut pair)?;
                    let mut varpair = next.into_inner();
                    let name;
                    let mut location_maybe = None;
                    if varpair.len() == 2 {
                        // it's a definition assignment
                        let let_or_static = AshParser::next_or_error(&mut varpair)?;
                        match let_or_static.as_rule() {
                            Rule::let_r => {
                                location_maybe = Some(VarLocation::Witness);
                            }
                            Rule::static_r => {
                                location_maybe = Some(VarLocation::Static);
                            }
                            _ => anyhow::bail!(
                                "ashlang: unsupported rule in var_def: {:?}",
                                let_or_static.as_rule()
                            ),
                        }
                        name = AshParser::next_or_error(&mut varpair)?.as_str().to_string();
                    } else if varpair.len() == 1 {
                        // it's a regular assignment
                        name = AshParser::next_or_error(&mut varpair)?.as_str().to_string();
                    } else {
                        return Err(anyhow::anyhow!("invalid varpait"));
                    }

                    let n = AshParser::next_or_error(&mut pair)?;
                    Ok(AstNode::Stmt(
                        name,
                        location_maybe,
                        self.build_expr_from_pair(n)?,
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
                    Ok(AstNode::If(
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
                    if let Some(next) = pair.next() {
                        let index = self.build_expr_from_pair(next)?;
                        Ok(Expr::ValVarIndex(name, Box::new(index)))
                    } else {
                        Ok(Expr::ValVar(name))
                    }
                }
                Rule::literal_dec => Ok(Expr::Lit(pair.as_str().to_string())),
                Rule::vec => {
                    let mut pair = pair.into_inner();
                    let next = AshParser::next_or_error(&mut pair)?;
                    if next.as_rule() == Rule::vec {
                        unimplemented!()
                        // let mut out: Vec<Expr> = Vec::new();
                        // out.push(self.build_expr_from_pair(next.clone())?);
                        // for next in pair {
                        //     out.push(self.build_expr_from_pair(next.clone())?);
                        //     // next = pair.next().unwrap();
                        // }
                        // Ok(Expr::VecVec(out))
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
                        Rule::varname => Ok(Expr::ValVar(n.as_str().to_string())),
                        Rule::var_indexed => {
                            let mut pair = n.into_inner();
                            let name = AshParser::next_or_error(&mut pair)?.as_str().to_string();
                            if let Some(next) = pair.next() {
                                let index = self.build_expr_from_pair(next)?;
                                Ok(Expr::ValVarIndex(name, Box::new(index)))
                            } else {
                                Ok(Expr::ValVar(name))
                            }
                        }
                        Rule::precompile_expr => {
                            let mut pair = n.into_inner();
                            let precompile_name = AshParser::next_or_error(&mut pair)?;
                            let precompile_name = precompile_name.as_str().to_string();

                            let mut args = Vec::default();
                            let mut block_maybe = None;
                            while let Some(next) = pair.next() {
                                match next.as_rule() {
                                    Rule::expr => {
                                        args.push(self.build_expr_from_pair(next)?);
                                    }
                                    Rule::block => {
                                        let block_inner = next.into_inner();
                                        let block_ast = block_inner
                                            .map(|v| match v.as_rule() {
                                                Rule::stmt => {
                                                    let mut pair = v.into_inner();
                                                    let next = AshParser::next_or_error(&mut pair)?;
                                                    self.build_ast_from_pair(next)
                                                }
                                                _ => Err(anyhow::anyhow!(
                                                    "invalid expression in block"
                                                )),
                                            })
                                            .collect::<Result<Vec<AstNode>>>()?;
                                        block_maybe = Some(block_ast);
                                    }
                                    _ => anyhow::bail!(
                                        "ashlang: unexpected rule in precompile expression: {:?}",
                                        next.as_rule()
                                    ),
                                }
                            }
                            Ok(Expr::Precompile(precompile_name, args, block_maybe))
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
                            _ => Err(anyhow::anyhow!("unexpected rule in pratt parser")),
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
}
