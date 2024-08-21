use crate::r1cs::constraint::SymbolicOp;
use crate::r1cs::inv::inv;
use crate::r1cs::parser::R1csParser;
use anyhow::Result;
use std::collections::HashMap;

// Attempt to validate the constraints
// in an r1cs
pub fn solve(r1cs: &str) -> Result<()> {
    let r1cs = R1csParser::new(&r1cs);
    let mut vars: HashMap<usize, u64> = HashMap::new();
    vars.insert(0, 1);
    // build the witness
    for c in &r1cs.constraints {
        if !c.symbolic {
            continue;
        }
        let p = 18446744069414584321_u128;
        let mut a = 0_u128;
        for (coef, index) in &c.a {
            a += u128::try_from(coef.clone())? * u128::try_from(*vars.get(&index).unwrap())?;
            a %= p;
        }
        let mut b = 0_u128;
        for (coef, index) in &c.b {
            b += u128::try_from(coef.clone())? * u128::try_from(*vars.get(&index).unwrap())?;
            b %= p;
        }
        match c.symbolic_op.as_ref().unwrap() {
            SymbolicOp::Add => {
                vars.insert(c.out_i.unwrap(), u64::try_from((a + b) % p)?);
            }
            SymbolicOp::Mul => {
                vars.insert(c.out_i.unwrap(), u64::try_from((a * b) % p)?);
            }
            SymbolicOp::Inv => {
                vars.insert(c.out_i.unwrap(), inv(u64::try_from(a)?, u64::try_from(p)?));
            }
        }
    }
    // confirm that the witness is correct
    for c in &r1cs.constraints {
        if c.symbolic {
            continue;
        }
        let p = 18446744069414584321_u128;
        let mut a_lc = 0_u128;
        for (coef, index) in &c.a {
            a_lc += u128::try_from(coef.clone())? * u128::try_from(*vars.get(&index).unwrap())?;
            a_lc %= p;
        }
        let mut b_lc = 0_u128;
        for (coef, index) in &c.b {
            b_lc += u128::try_from(coef.clone())? * u128::try_from(*vars.get(&index).unwrap())?;
            b_lc %= p;
        }
        let mut c_lc = 0_u128;
        for (coef, index) in &c.c {
            c_lc += u128::try_from(coef.clone())? * u128::try_from(*vars.get(&index).unwrap())?;
            c_lc %= p;
        }
        assert_eq!((a_lc * b_lc) % p, c_lc);
    }

    println!("");
    println!("R1CS: built and validated witness ✅");
    Ok(())
}
