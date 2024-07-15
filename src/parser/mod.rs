use self::AstNode::*;
use pest::error::Error;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"] // relative to project `src`
struct AshParser;

#[derive(Debug, Clone)]
pub enum AstNode {
    FnVar(Vec<String>),
    Stmt(String, bool, Expr),
    Rtrn(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(u64),
    Val(String),
    NumOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Sub,
    Inv,
    Mul,
}

pub fn parse(source: &str) -> Result<Vec<AstNode>, Error<Rule>> {
    let mut ast = vec![];

    let pairs = AshParser::parse(Rule::program, source)?;
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
                ast.push(FnVar(vars));
            }
            Rule::stmt => {
                ast.push(build_ast_from_pair(pair));
            }
            Rule::return_stmt => {
                let mut pair = pair.into_inner();
                let next = pair.next().unwrap();
                ast.push(Rtrn(build_expr_from_pair(next)))
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
            // get vardef
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
            Stmt(name.as_str().to_string(), is_let, build_expr_from_pair(n))
        }
        unknown_expr => panic!("Unexpected expression: {:?}", unknown_expr),
    }
}

fn build_expr_from_pair(pair: pest::iterators::Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::atom => {
            let mut pair = pair.into_inner();
            let n = pair.next().unwrap();
            match n.as_rule() {
                Rule::varname => Expr::Val(n.as_str().to_string()),
                Rule::literal_dec => Expr::Lit(n.as_str().parse::<u64>().unwrap()),
                _ => panic!("invalid atom"),
            }
            // Expr::Val(pair.next().unwrap().as_str().to_string())
        }
        Rule::expr => {
            let mut pair = pair.into_inner();
            let first_atom = pair.next().unwrap();
            if pair.len() == 0 {
                return build_expr_from_pair(first_atom);
                // return Expr::Val(first_atom.as_str().to_string());
            }
            let op = pair.next().unwrap();
            let rhs = pair.next().unwrap();
            Expr::NumOp {
                lhs: Box::new(build_expr_from_pair(first_atom)),
                op: match op.as_rule() {
                    Rule::add => Op::Add,
                    Rule::sub => Op::Sub,
                    Rule::mul => Op::Mul,
                    Rule::inv => Op::Inv,
                    _ => panic!("invalid op"),
                },
                rhs: Box::new(build_expr_from_pair(rhs)),
            }
        }
        unknown_expr => panic!("Unexpected expression: {:?}", unknown_expr),
    }
}
