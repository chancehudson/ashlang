use std::collections::HashMap;

use super::constraint::string_to_index;
use super::constraint::R1csConstraint;
use super::constraint::SymbolicOp;
use crate::log;
use anyhow::anyhow;
use anyhow::Result;
use pest::Parser;
use pest_derive::Parser;
use scalarff::FieldElement;

#[derive(Parser)]
#[grammar = "r1cs/r1cs_grammar.pest"] // relative to project `src`
pub struct R1csPestParser;

pub struct R1csParser<T: FieldElement> {
    pub constraints: Vec<R1csConstraint<T>>,
    pub arg_name_index: HashMap<String, usize>,
    pub arg_names: Vec<String>,
    pub return_name_index: HashMap<String, usize>,
    pub return_names: Vec<String>,
    pub is_function: bool,
}

impl<T: FieldElement> R1csParser<T> {
    pub fn new(source: &str) -> Self {
        let mut out = R1csParser {
            constraints: Vec::new(),
            arg_name_index: HashMap::new(),
            arg_names: vec![],
            is_function: false,
            return_names: vec![],
            return_name_index: HashMap::new(),
        };
        out.arg_name_index.insert("one".to_string(), 0);
        out.arg_names.push("one".to_string());
        let parsed = R1csPestParser::parse(Rule::program, source);
        if let Err(e) = parsed {
            panic!("Failed to parse r1cs: {}", e);
        }
        let parsed = parsed.unwrap();
        for pair in parsed {
            match pair.as_rule() {
                Rule::type_header => {
                    out.is_function = true;
                    let mut pair = pair.into_inner();
                    let args = pair.next().unwrap();
                    let args_tuple = args.into_inner();
                    for v in args_tuple {
                        let varname = v.as_str().to_string();
                        if out.arg_name_index.contains_key(&varname) {
                            println!("ar1cs parse error: duplicate arg name: {}", varname);
                            panic!();
                        }
                        out.arg_name_index
                            .insert(varname.clone(), out.arg_name_index.len());
                        out.arg_names.push(varname);
                    }
                    let returns = pair.next().unwrap();
                    let returns_tuple = returns.into_inner();
                    for v in returns_tuple {
                        // varname
                        let varname = v.as_str();
                        if out.arg_name_index.contains_key(varname) {
                            println!(
                                "ar1cs parse error: return arg name is not unique: {}",
                                varname
                            );
                            panic!();
                        }
                        if out.return_name_index.contains_key(varname) {
                            println!(
                                "ar1cs parse error: return arg name is not unique: {}",
                                varname
                            );
                            panic!();
                        }
                        out.return_names.push(varname.to_string());
                        out.return_name_index.insert(
                            varname.to_string(),
                            out.arg_name_index.len() + out.return_name_index.len(),
                        );
                    }
                }
                Rule::constraint_line => {
                    let mut pair = pair.into_inner();
                    let a = pair.next().unwrap();
                    let a = out.parse_inner(a);
                    let b = pair.next().unwrap();
                    let b = out.parse_inner(b);
                    let c = pair.next().unwrap();
                    let c = out.parse_inner(c);
                    out.constraints.push(R1csConstraint::new(a, b, c, ""));
                }
                Rule::symbolic_line => {
                    let mut pair = pair.into_inner();
                    // println!("{:?}", pair);
                    let o = pair.next().unwrap();
                    let a = pair.next().unwrap();
                    let a = out.parse_inner(a);
                    let op = pair.next().unwrap();
                    let op = SymbolicOp::from(op.as_str());
                    let b = pair.next().unwrap();
                    let b = out.parse_inner(b);
                    let out_index;
                    if out.is_function {
                        if let Some(i) = out.return_name_index.get(o.as_str()) {
                            out_index = *i;
                        } else {
                            panic!("constraints can only be assigned to return values");
                        }
                    } else {
                        out_index = string_to_index(o.as_str());
                    }
                    out.constraints.push(R1csConstraint::symbolic(
                        out_index,
                        a,
                        b,
                        op,
                        "".to_string(),
                    ));
                }
                Rule::comment => {
                    let mut pair = pair.into_inner();
                    let text = pair.next();
                    if text.is_none() {
                        continue;
                    }
                    let text = text.unwrap().as_str().to_string();
                    if !out.constraints.is_empty() {
                        out.constraints.last_mut().unwrap().comment = Some(text);
                    }
                }
                Rule::comment_line => {}
                Rule::EOI => {}
                _ => panic!("{:?}", pair.as_rule()),
            }
        }
        out
    }

    pub fn parse_inner(&mut self, p: pest::iterators::Pair<Rule>) -> Vec<(T, usize)> {
        let mut pair = p.into_inner();
        let mut out_terms = Vec::new();
        while let Some(v) = pair.next() {
            let coef = v.as_str();
            let var_index = pair.next().unwrap().as_str();
            if self.is_function {
                // restrict the signals that may be accessed by literal
                if !self.arg_name_index.contains_key(var_index)
                    && !self.return_name_index.contains_key(var_index)
                    && var_index != "0"
                {
                    panic!("cannot access signals by literal in ar1cs source");
                }
                if let Some(v) = self.arg_name_index.get(var_index) {
                    // if signal is a variable
                    out_terms.push((T::deserialize(coef), *v));
                } else if let Some(v) = self.return_name_index.get(var_index) {
                    out_terms.push((T::deserialize(coef), *v));
                } else {
                    // if coef is a literal
                    out_terms.push((T::deserialize(coef), string_to_index(var_index)));
                }
            } else {
                out_terms.push((T::deserialize(coef), string_to_index(var_index)));
            }
        }
        out_terms
    }

    pub fn signals_as_args(
        &self,
        index_start: usize,
        mut args: Vec<usize>,
    ) -> Result<Vec<R1csConstraint<T>>> {
        // map local signal indices to caller indices
        let mut signal_map: HashMap<usize, usize> = HashMap::new();
        // push the 1 signal to the front of the arg list
        args.insert(0, 0);
        if args.len() != self.arg_names.len() {
            return log::error!(&format!(
                "error calling function, incorrect number of arguments, got {} expected {}",
                args.len(),
                self.arg_names.len()
            ));
        }
        for (i, v) in args.iter().enumerate() {
            let local_index = self.arg_name_index.get(&self.arg_names[i]);
            if let Some(local_index) = local_index {
                signal_map.insert(*local_index, *v);
            } else {
                return Err(anyhow!("error mapping signal as arg"));
            }
        }
        // map return values to calculated offsets
        for x in 0..self.return_names.len() {
            let local_index = self.return_name_index.get(&self.return_names[x]);
            if let Some(local_index) = local_index {
                signal_map.insert(*local_index, index_start + x);
            } else {
                return Err(anyhow!("error mapping signal as arg"));
            }
        }
        self.constraints
            .iter()
            .map(|constraint| {
                let mut new_constraint = constraint.clone();
                if let Some(i) = constraint.out_i {
                    new_constraint.out_i = Some(*signal_map.get(&i).unwrap());
                }
                new_constraint.a = constraint
                    .a
                    .clone()
                    .iter()
                    .map(|v| (v.0.clone(), *signal_map.get(&v.1).unwrap()))
                    .collect::<Vec<_>>();
                new_constraint.b = constraint
                    .b
                    .clone()
                    .iter()
                    .map(|v| (v.0.clone(), *signal_map.get(&v.1).unwrap()))
                    .collect::<Vec<_>>();
                new_constraint.c = constraint
                    .c
                    .clone()
                    .iter()
                    .map(|v| (v.0.clone(), *signal_map.get(&v.1).unwrap()))
                    .collect::<Vec<_>>();
                Ok(new_constraint)
            })
            .collect::<Result<Vec<_>>>()
    }
}
