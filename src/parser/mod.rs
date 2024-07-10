use self::AstNode::*;
use pest::{iterators::Pairs, pratt_parser::PrattParser, Parser};
use pest_derive::Parser;
use pest::error::Error;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"] // relative to project `src`
struct AshParser;

// pub enum Val {
//     Var(String),
//     Val(u64)
// }

#[derive(Debug)]
pub enum AstNode {
    Print(Box<AstNode>),
    Stmt {
        name: String,
        expr: Box<AstNode>
    },
    Expr(Expr),
    Op
}

#[derive(Debug)]
pub enum Expr {
    Val(String),
    NumOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>
    }
}

#[derive(Debug)]
pub enum Op {
    Add,
    Sub,
    Inv,
    Mul
}

pub fn parse(source: &str) -> Result<Vec<AstNode>, Error<Rule>> {
    let mut ast = vec![];

    let pairs = AshParser::parse(Rule::program, source)?;
    for pair in pairs {
        match pair.as_rule() {
            Rule::stmt=> {
                ast.push(build_ast_from_pair(pair));
            }
            _ => {}
        }
    }

    Ok(ast)
}

fn build_ast_from_pair(pair: pest::iterators::Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::stmt => {
            let mut pair = pair.into_inner();
            let name = pair.next().unwrap();
            let n = pair.next().unwrap();
            Stmt {
                name: name.as_str().to_string(),
                expr: Box::new(build_ast_from_pair(n))
            }
        },
        Rule::expr => {
            Expr(build_expr_from_pair(pair))
        }
        unknown_expr => panic!("Unexpected expression: {:?}", unknown_expr),
    }
}

fn build_expr_from_pair(pair: pest::iterators::Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::atom => {
            let mut pair = pair.into_inner();
            Expr::Val(pair.next().unwrap().as_str().to_string())
        }
        Rule::expr => {
            let mut pair = pair.into_inner();
            let first_atom = pair.next().unwrap();
            if pair.len() == 0 {
                return Expr::Val(first_atom.as_str().to_string());
            }
            let op = pair.next().unwrap();
            let rhs = pair.next().unwrap();
            Expr::NumOp {
                lhs: Box::new(Expr::Val(first_atom.as_str().to_string())),
                op: match op.as_rule() {
                    Rule::add => Op::Add,
                    Rule::sub => Op::Sub,
                    Rule::mul => Op::Mul,
                    Rule::inv => Op::Inv,
                    _ => panic!("invalid op")
                },
                rhs: Box::new(build_expr_from_pair(rhs))
            }
        }
        unknown_expr => panic!("Unexpected expression: {:?}", unknown_expr),
    }
}