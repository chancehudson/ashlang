use crate::math::FieldElement;
use crate::r1cs::constraint::R1csConstraint;
use crate::r1cs::constraint::SymbolicOp;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "r1cs/r1cs_grammar.pest"] // relative to project `src`
pub struct R1csPestParser;

pub struct R1csParser<T: FieldElement> {
    pub constraints: Vec<R1csConstraint<T>>,
}

impl<T: FieldElement> R1csParser<T> {
    pub fn new(source: &str) -> Self {
        let mut out = R1csParser {
            constraints: Vec::new(),
        };
        let parsed = R1csPestParser::parse(Rule::program, source);
        if let Err(e) = parsed {
            panic!("Failed to parse r1cs: {}", e);
        }
        let parsed = parsed.unwrap();
        let parse_constraint_inner = |p: pest::iterators::Pair<Rule>| -> Vec<(T, usize)> {
            let mut pair = p.into_inner();
            let mut out = vec![];
            while let Some(v) = pair.next() {
                let coef = T::from(v.as_str().parse::<u64>().unwrap());
                let var_index = pair.next().unwrap().as_str().parse::<usize>().unwrap();
                out.push((coef, var_index));
            }
            out
        };
        for pair in parsed {
            match pair.as_rule() {
                Rule::constraint_line => {
                    let mut pair = pair.into_inner();
                    let a = pair.next().unwrap();
                    let a = parse_constraint_inner(a);
                    let b = pair.next().unwrap();
                    let b = parse_constraint_inner(b);
                    let c = pair.next().unwrap();
                    let c = parse_constraint_inner(c);
                    out.constraints
                        .push(R1csConstraint::new(a, b, c, "".to_owned()));
                }
                Rule::symbolic_line => {
                    let mut pair = pair.into_inner();
                    let a = pair.next().unwrap();
                    let a = parse_constraint_inner(a);
                    let b = pair.next().unwrap();
                    let b = parse_constraint_inner(b);
                    let c = pair.next().unwrap();
                    let mut pair = c.into_inner();
                    let op = pair.next().unwrap();
                    let op = SymbolicOp::from(op.as_str());
                    let out_index = pair.next().unwrap().as_str().parse::<usize>().unwrap();
                    out.constraints
                        .push(R1csConstraint::symbolic(out_index, a, b, op));
                }
                Rule::EOI => {}
                _ => panic!("{:?}", pair.as_rule()),
            }
        }
        out
    }
}
