use std::collections::HashSet;

use anyhow::Result;
use lettuce::*;

use crate::*;

use super::*;

/// A solvable rank 1 constraint system for a program of finite, constant, length
#[derive(Clone)]
pub struct AshlangR1CS<E: FieldScalar> {
    pub r1cs: R1CS<E>,
    /// A binary vector with 1's in witness indices that are public
    pub output_mask: Vector<E>,
    pub ar1cs_src: AR1CSSourceString,
}

impl<E: FieldScalar> AshlangR1CS<E> {
    pub fn compute_wtns(&self, input: Vector<E>) -> Result<Vector<E>> {
        let parser = AR1CSParser::new(&self.ar1cs_src)?;
        let mut wtns = Vector::new(parser.wtns_len());
        let mut written_wtns_indices = HashSet::<usize>::new();

        let mut output_indices = vec![];
        let mut input_counter = 0_usize;

        written_wtns_indices.insert(0);
        wtns[0] = E::one();

        // build the witness
        for c in parser.symbolic_constraints() {
            let symbolic_wtns_i = c.out_i.expect("symbolic wtns should exist");
            match c.symbolic_op.as_ref().unwrap() {
                SymbolicOp::Output => {
                    // we'll reveal in place rather than move to the front of the vector
                    output_indices.push(symbolic_wtns_i);
                }
                SymbolicOp::Input => {
                    assert!(
                        written_wtns_indices.insert(symbolic_wtns_i),
                        "duplicate write, input {c}"
                    );
                    // we'll take the next input value and set it
                    if input_counter >= input.len() {
                        return crate::log::error!(
                            "not enough inputs supplied to fulfill symbolic constraints",
                            "the number of inputs must match the number of input constraints"
                        );
                    }
                    wtns[symbolic_wtns_i] = input[input_counter];
                    input_counter += 1;
                }
                _ => {
                    assert!(
                        written_wtns_indices.insert(symbolic_wtns_i),
                        "duplicate write, constraint {c}"
                    );
                    wtns[symbolic_wtns_i] = c.solve_symbolic(&wtns)?;
                }
            }
        }
        assert_eq!(
            wtns.len(),
            written_wtns_indices.len(),
            "not all witness entries were written! check arrays, matrices and inputs"
        );
        if input_counter != input.len() {
            return crate::log::error!(&format!(
                "not all inputs were used in witness calculation, {} inputs unused",
                input.len() - input_counter
            ));
        }
        Ok(wtns)
    }

    pub fn new(ar1cs_src: AR1CSSourceString) -> Result<Self> {
        let parser = AR1CSParser::new(&ar1cs_src)?;
        Ok(Self {
            output_mask: parser.wtns_mask(),
            r1cs: parser.into_r1cs(),
            ar1cs_src,
        })
    }

    /// Assert that a witness is set and fulfills the provided R1CS.
    pub fn assert_wtns(&self, wtns: &Vector<E>) -> Result<()> {
        let eval = self.r1cs.eval(wtns)?;
        if !eval.is_zero() {
            let constraint_i = eval.iter().enumerate().find(|(_, v)| !v.is_zero()).unwrap();
            anyhow::bail!(
                "ashlang: constraint {} failed, resulting value: {}",
                constraint_i.0,
                constraint_i.1
            );
        }
        Ok(())
    }
}
