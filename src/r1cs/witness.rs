use crate::math::FieldElement;
use crate::r1cs::constraint::SymbolicOp;
use crate::r1cs::parser::R1csParser;
use anyhow::Result;
use std::collections::HashMap;

use super::constraint::R1csConstraint;

pub fn verify<T: FieldElement>(r1cs: &str, witness: Vec<T>) -> Result<()> {
    // confirm that the witness is correct
    let r1cs: R1csParser<T> = R1csParser::new(&r1cs);
    let mut vars: HashMap<usize, T> = HashMap::new();
    for x in 0..witness.len() {
        vars.insert(x, witness[x].clone());
    }

    for c in &r1cs.constraints {
        if c.symbolic {
            continue;
        }
        let mut a_lc = T::zero();
        for (coef, index) in &c.a {
            a_lc += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut b_lc = T::zero();
        for (coef, index) in &c.b {
            b_lc += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut c_lc = T::zero();
        for (coef, index) in &c.c {
            c_lc += coef.clone() * vars.get(&index).unwrap().clone();
        }
        assert_eq!(a_lc * b_lc, c_lc);
    }
    Ok(())
}

pub fn build_str<T: FieldElement>(r1cs: &str, inputs: Vec<T>) -> Result<Vec<T>> {
    build(&R1csParser::new(r1cs).constraints, inputs)
}

// Attempt to validate the constraints
// in an r1cs
pub fn build<T: FieldElement>(r1cs: &Vec<R1csConstraint<T>>, inputs: Vec<T>) -> Result<Vec<T>> {
    // let r1cs: R1csParser<T> = R1csParser::new(&r1cs);
    let mut vars: HashMap<usize, T> = HashMap::new();
    vars.insert(0, T::one());
    for x in 0..inputs.len() {
        vars.insert(x + 1, inputs[x].clone());
    }
    // build the witness
    for c in r1cs {
        if !c.symbolic {
            continue;
        }
        let mut a = T::zero();
        for (coef, index) in &c.a {
            a += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut b = T::zero();
        for (coef, index) in &c.b {
            b += coef.clone() * vars.get(&index).unwrap().clone();
        }
        match c.symbolic_op.as_ref().unwrap() {
            SymbolicOp::Add => {
                vars.insert(c.out_i.unwrap(), a + b);
            }
            SymbolicOp::Mul => {
                vars.insert(c.out_i.unwrap(), a * b);
            }
            SymbolicOp::Inv => {
                vars.insert(c.out_i.unwrap(), T::one() / b);
            }
        }
    }
    let mut out = vars.keys().map(|k| *k).collect::<Vec<usize>>();
    out.sort();
    Ok(out
        .iter()
        .map(|k| vars.get(k).unwrap().clone())
        .collect::<Vec<_>>())
}
