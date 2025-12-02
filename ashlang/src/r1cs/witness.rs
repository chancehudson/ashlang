use std::collections::HashMap;

use anyhow::Result;
use lettuce::*;

use crate::r1cs::parser::R1csParser;

use super::constraint::SymbolicOp;

/// A structure representing a witness computation
///
/// `outputs`: a list of indices of variables that should be publicly revealed
/// `variables`: values of the variables in the computation
#[derive(Clone)]
pub struct Witness<E: FieldScalar> {
    pub outputs: Vec<usize>,
    pub variables: Vec<E>,
}

/// Verify that a witness satisfies the constraints of an ar1cs source string.
/// This function handles parsing the ar1cs source string.
pub fn verify<E: FieldScalar>(r1cs_parser: &R1csParser<E>, witness: Witness<E>) -> Result<Vec<E>> {
    // confirm that the witness is correct
    let mut vars: HashMap<usize, E> = HashMap::new();
    for (i, v) in witness.variables.iter().enumerate() {
        vars.insert(i, v.clone());
    }

    for c in &r1cs_parser.constraints {
        if c.symbolic {
            continue;
        }
        let mut a_lc = E::zero();
        for (coef, index) in &c.a {
            a_lc += coef.clone() * vars.get(index).unwrap().clone();
        }
        let mut b_lc = E::zero();
        for (coef, index) in &c.b {
            b_lc += coef.clone() * vars.get(index).unwrap().clone();
        }
        let mut c_lc = E::zero();
        for (coef, index) in &c.c {
            c_lc += coef.clone() * vars.get(index).unwrap().clone();
        }
        if a_lc.clone() * b_lc.clone() != c_lc {
            anyhow::bail!("Constraint failed: {:?}", /*c*/ "")
        }
    }
    Ok(witness
        .outputs
        .iter()
        .map(|i| witness.variables[*i].clone())
        .collect::<Vec<_>>())
}

/// Take an ar1cs source file and a set of inputs and build a witness.
pub fn build<E: FieldScalar>(r1cs_parser: R1csParser<E>, inputs: Vec<E>) -> Result<Witness<E>> {
    let mut vars: HashMap<usize, E> = HashMap::new();
    let mut outputs = vec![];
    let mut input_counter = 0_usize;
    vars.insert(0, E::one());
    // build the witness
    for c in &r1cs_parser.constraints {
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
                vars.insert(c.out_i.unwrap(), inputs[input_counter]);
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
