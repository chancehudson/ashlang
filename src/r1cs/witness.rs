use std::collections::HashMap;

use anyhow::Result;
use scalarff::FieldElement;

use crate::r1cs::parser::R1csParser;

use super::constraint::SymbolicOp;

pub fn verify<T: FieldElement>(r1cs: &str, witness: Vec<T>) -> Result<()> {
    // confirm that the witness is correct
    let r1cs: R1csParser<T> = R1csParser::new(r1cs)?;
    let mut vars: HashMap<usize, T> = HashMap::new();
    for (i, v) in witness.iter().enumerate() {
        vars.insert(i, v.clone());
    }

    for c in &r1cs.constraints {
        if c.symbolic {
            continue;
        }
        let mut a_lc = T::zero();
        for (coef, index) in &c.a {
            a_lc += coef.clone() * vars.get(index).unwrap().clone();
        }
        let mut b_lc = T::zero();
        for (coef, index) in &c.b {
            b_lc += coef.clone() * vars.get(index).unwrap().clone();
        }
        let mut c_lc = T::zero();
        for (coef, index) in &c.c {
            c_lc += coef.clone() * vars.get(index).unwrap().clone();
        }
        if a_lc.clone() * b_lc.clone() != c_lc {
            anyhow::bail!("Constraint failed: {:?}", c)
        }
    }
    Ok(())
}

// Attempt to validate the constraints
// in an r1cs
pub fn build<T: FieldElement>(r1cs: &str, inputs: Vec<T>) -> Result<Vec<T>> {
    let r1cs: R1csParser<T> = R1csParser::new(r1cs)?;
    let mut vars: HashMap<usize, T> = HashMap::new();
    let mut input_counter = 0_usize;
    vars.insert(0, T::one());
    // build the witness
    for c in &r1cs.constraints {
        if !c.symbolic {
            continue;
        }
        if c.symbolic_op.as_ref().unwrap() == &SymbolicOp::Input {
            // we'll take the next input value and set it
            // vars.insert(inputs., v)
            vars.insert(c.out_i.unwrap(), inputs[input_counter].clone());
            input_counter += 1;
        } else {
            vars.insert(c.out_i.unwrap(), c.solve_symbolic(&vars)?);
        }
    }
    if input_counter != inputs.len() {
        return crate::log::error!(&format!(
            "not all inputs were used in witness calculation, {} inputs unused",
            inputs.len() - input_counter
        ));
    }
    let mut out = vars.keys().copied().collect::<Vec<usize>>();
    out.sort();
    Ok(out
        .iter()
        .map(|k| vars.get(k).unwrap().clone())
        .collect::<Vec<_>>())
}
