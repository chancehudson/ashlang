use std::{collections::HashSet, panic};

use anyhow::Result;
use lettuce::*;

use super::*;

/// R1CS arithmetizer for ashlang. Converts ashlang programs
/// to `Matrix<E>` and `Vector<E>`.
#[derive(Clone)]
pub struct Arithmetizer<E: FieldScalar> {
    pub wtns: Option<Vector<E>>,
    pub r1cs: R1CS<E>,
    pub parser: R1csParser<E>,
    /// Witness indices that are public. Used to construct a vector mask.
    pub output_indices: Vec<usize>,
}
impl<E: FieldScalar> Arithmetizer<E> {
    pub fn new(ar1cs_src: &str) -> Result<Self> {
        let parser = R1csParser::new(ar1cs_src)?;
        Ok(Self {
            wtns: None,
            r1cs: parser.clone().into_r1cs(),
            parser,
            output_indices: Vec::default(),
        })
    }

    /// Get an iterator over the public signals in the circuit.
    pub fn outputs(&self) -> Result<impl Iterator<Item = E>> {
        Ok(self
            .output_indices
            .iter()
            .map(|v| self.wtns.as_ref().unwrap()[*v]))
    }

    /// Assert that a witness is set and fulfills the provided R1CS.
    pub fn assert_wtns(&self) -> Result<&Vector<E>> {
        let wtns = self.wtns.as_ref().ok_or(anyhow::anyhow!("no wtns"))?;
        let eval = self.r1cs.eval(wtns)?;
        if !eval.is_zero() {
            let constraint_i = eval.iter().enumerate().find(|(_, v)| !v.is_zero()).unwrap();
            anyhow::bail!(
                "ashlang: constraint {} failed, resulting value: {}",
                constraint_i.0,
                constraint_i.1
            );
        }
        Ok(wtns)
    }

    /// Computes the witness for self. Returns the number of public outputs.
    pub fn compute_wtns(&mut self, input: Vector<E>) -> Result<usize> {
        let mut wtns = Vector::new(self.parser.wtns_len());
        let mut written_wtns_indices = HashSet::<usize>::new();

        let mut output_indices = vec![];
        let mut input_counter = 0_usize;

        written_wtns_indices.insert(0);
        wtns[0] = E::one();

        // build the witness
        let mut output_done = false;
        for c in &self.parser.constraints {
            if !c.symbolic {
                continue;
            }
            let symbolic_wtns_i = c.out_i.expect("symbolic wtns should exist");
            assert!(
                written_wtns_indices.insert(symbolic_wtns_i),
                "duplicate write, output {c}"
            );
            written_wtns_indices.insert(symbolic_wtns_i);
            match c.symbolic_op.as_ref().unwrap() {
                SymbolicOp::Output => {
                    assert!(
                        !output_done,
                        "cannot generate public output after private witness"
                    );
                    output_indices.push(symbolic_wtns_i);
                    // dont use output for now
                    unreachable!()
                }
                SymbolicOp::Input => {
                    output_done = true;
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
                    output_done = true;
                    wtns[symbolic_wtns_i] = c.solve_symbolic(&wtns)?;
                }
            }
        }
        assert_eq!(
            wtns.len(),
            written_wtns_indices.len(),
            "not all witness entries were written!"
        );
        if input_counter != input.len() {
            return crate::log::error!(&format!(
                "not all inputs were used in witness calculation, {} inputs unused",
                input.len() - input_counter
            ));
        }
        self.wtns = Some(wtns);
        self.output_indices = output_indices;
        Ok(self.output_indices.len())
    }
}
