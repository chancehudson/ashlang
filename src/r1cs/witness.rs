use crate::math::FieldElement;
use crate::r1cs::parser::R1csParser;
use anyhow::Result;
use std::collections::HashMap;

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
        if a_lc.clone() * b_lc.clone() != c_lc {
            anyhow::bail!("Constraint failed: {:?}", c)
        }
    }
    Ok(())
}

// Attempt to validate the constraints
// in an r1cs
pub fn build<T: FieldElement>(r1cs: &str) -> Result<Vec<T>> {
    let r1cs: R1csParser<T> = R1csParser::new(&r1cs);
    let mut vars: HashMap<usize, T> = HashMap::new();
    vars.insert(0, T::one());
    // build the witness
    for c in &r1cs.constraints {
        if !c.symbolic {
            continue;
        }
        vars.insert(c.out_i.unwrap(), c.solve_symbolic(&vars));
    }
    let mut out = vars.keys().map(|k| *k).collect::<Vec<usize>>();
    out.sort();
    Ok(out
        .iter()
        .map(|k| vars.get(k).unwrap().clone())
        .collect::<Vec<_>>())
}
