use std::collections::HashMap;

use anyhow::Result;
use scalarff::FieldElement;

use crate::r1cs::parser::R1csParser;

use super::constraint::SymbolicOp;

/// A structure representing a witness computation
/// outputs represents a list of indices of the variables that should be publicly revealed
/// variables represents the values of the variables in the computation
pub struct Witness<T: FieldElement> {
    outputs: Vec<usize>,
    variables: Vec<T>,
}

pub fn verify<T: FieldElement>(r1cs: &str, witness: Witness<T>) -> Result<Vec<T>> {
    // confirm that the witness is correct
    let r1cs: R1csParser<T> = R1csParser::new(r1cs)?;
    let mut vars: HashMap<usize, T> = HashMap::new();
    for (i, v) in witness.variables.iter().enumerate() {
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
    Ok(witness
        .outputs
        .iter()
        .map(|i| witness.variables[*i].clone())
        .collect::<Vec<_>>())
}

// Attempt to validate the constraints
// in an r1cs
pub fn build<T: FieldElement>(r1cs: &str, inputs: Vec<T>) -> Result<Witness<T>> {
    let r1cs: R1csParser<T> = R1csParser::new(r1cs)?;
    let mut vars: HashMap<usize, T> = HashMap::new();
    let mut outputs = vec![];
    let mut input_counter = 0_usize;
    vars.insert(0, T::one());
    // build the witness
    for c in &r1cs.constraints {
        if !c.symbolic {
            continue;
        }
        match c.symbolic_op.as_ref().unwrap() {
            SymbolicOp::Input => {
                // we'll take the next input value and set it
                if input_counter >= inputs.len() {
                    return crate::log::error!(
                        "not enough inputs supplied to fulfill symbolic constraints",
                        "the number of inputs must match the number of input constraints"
                    );
                }
                vars.insert(c.out_i.unwrap(), inputs[input_counter].clone());
                input_counter += 1;
            }
            SymbolicOp::PublicInput => {
                // we'll take the relevant signal and mark it as public
                if input_counter >= inputs.len() {
                    return crate::log::error!(
                        "not enough inputs supplied to fulfill symbolic constraints",
                        "the number of inputs must match the number of input constraints"
                    );
                }
                outputs.push(c.out_i.unwrap());
                vars.insert(c.out_i.unwrap(), inputs[input_counter].clone());
                input_counter += 1;
            }
            SymbolicOp::Output => {
                outputs.push(c.out_i.unwrap());
            }
            _ => {
                let v = c.solve_symbolic(&vars)?;
                if vars.contains_key(&c.out_i.unwrap()) {
                    return crate::log::error!(
                        &format!("variable {} already set", c.out_i.unwrap()),
                        "setting a variable multiple times is considered a programming error"
                    );
                }
                vars.insert(c.out_i.unwrap(), v);
            }
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
    Ok(Witness {
        outputs,
        variables: out
            .iter()
            .map(|k| vars.get(k).unwrap().clone())
            .collect::<Vec<_>>(),
    })
}
