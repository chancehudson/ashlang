use crate::math::field_64::FoiFieldElement;
use crate::math::FieldElement;
use crate::r1cs::constraint::SymbolicOp;
use crate::r1cs::parser::R1csParser;
use anyhow::Result;
use std::collections::HashMap;

// Attempt to validate the constraints
// in an r1cs
pub fn solve(r1cs: &str) -> Result<()> {
    let r1cs: R1csParser<FoiFieldElement> = R1csParser::new(&r1cs);
    let mut vars: HashMap<usize, FoiFieldElement> = HashMap::new();
    vars.insert(0, FoiFieldElement::one());
    // build the witness
    for c in &r1cs.constraints {
        if !c.symbolic {
            continue;
        }
        let mut a = FoiFieldElement::zero();
        for (coef, index) in &c.a {
            a += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut b = FoiFieldElement::zero();
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
                vars.insert(c.out_i.unwrap(), FoiFieldElement::one() / b);
            }
        }
    }
    // confirm that the witness is correct
    for c in &r1cs.constraints {
        if c.symbolic {
            continue;
        }
        let mut a_lc = FoiFieldElement::zero();
        for (coef, index) in &c.a {
            a_lc += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut b_lc = FoiFieldElement::zero();
        for (coef, index) in &c.b {
            b_lc += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut c_lc = FoiFieldElement::zero();
        for (coef, index) in &c.c {
            c_lc += coef.clone() * vars.get(&index).unwrap().clone();
        }
        assert_eq!(a_lc * b_lc, c_lc);
    }

    println!("");
    println!("R1CS: built and validated witness ✅");
    Ok(())
}
