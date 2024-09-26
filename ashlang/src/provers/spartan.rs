use ring_math::Polynomial;
use scalarff::Curve25519FieldElement;
use scalarff::FieldElement;
extern crate libspartan;
extern crate merlin;
use anyhow::Result;
use curve25519_dalek::scalar::Scalar;
use libspartan::Assignment;
use libspartan::ComputationCommitment;
use libspartan::InputsAssignment;
use libspartan::Instance;
use libspartan::SNARKGens;
use libspartan::VarsAssignment;
use libspartan::SNARK;
use merlin::Transcript;

use crate::cli::Config;
use crate::compiler::Compiler;
use crate::log;
use crate::provers::AshlangProver;
use crate::r1cs::constraint::R1csConstraint;
use crate::r1cs::parser::R1csParser;
use crate::rings::Curve25519PolynomialRing;

pub type SpartanConfig = (
    usize,
    usize,
    usize,
    usize,
    Instance,
    VarsAssignment,
    InputsAssignment,
);

// contains the data necessary to
// verify a proof
pub struct SpartanProof {
    pub snark: SNARK,
    pub comm: ComputationCommitment,
    pub gens: SNARKGens,

    pub inputs: Assignment,
}

/// Convert a vector into a fixed-size slice
/// panic if the input vector.len() > 32
/// if the input vector.len() < 32, fill the remainder with zeros
fn to_32(v: Vec<u8>) -> [u8; 32] {
    let mut out: [u8; 32] = [0; 32];
    if v.len() > 32 {
        panic!("too many bytes");
    }
    for i in 0..32 {
        if i < v.len() {
            out[i] = v[i];
        }
    }
    out
}

pub struct SpartanProver {}

impl AshlangProver<SpartanProof> for SpartanProver {
    fn prove(config: &Config) -> Result<SpartanProof> {
        let mut config = config.clone();
        config.extension_priorities.push("ar1cs".to_string());

        if config.field != "curve25519" {
            return log::error!(
                "unsupported curve for microsoft/spartan proof",
                "field must be \"curve25519\""
            );
        }

        let mut compiler: Compiler<Curve25519PolynomialRing> = Compiler::new(&config)?;
        let r1cs = compiler.compile(&config.entry_fn)?;

        // produce public parameters
        let spartan_config = transform_r1cs(
            &r1cs,
            config
                .secret_inputs
                .iter()
                .map(|v| Curve25519FieldElement::deserialize(v))
                .collect::<Vec<_>>(),
        )?;
        let (
            num_cons,
            num_vars,
            num_inputs,
            num_non_zero_entries,
            inst,
            assignment_vars,
            assignment_inputs,
        ) = spartan_config;
        let gens = SNARKGens::new(num_cons, num_vars, num_inputs, num_non_zero_entries);

        // create a commitment to the R1CS instance
        let (comm, decomm) = SNARK::encode(&inst, &gens);

        // produce a proof of satisfiability
        let mut prover_transcript = Transcript::new(b"ashlang-spartan");
        Ok(SpartanProof {
            snark: SNARK::prove(
                &inst,
                &comm,
                &decomm,
                assignment_vars,
                &assignment_inputs,
                &gens,
                &mut prover_transcript,
            ),
            comm,
            gens,
            inputs: assignment_inputs,
        })
    }

    fn verify(serialized_proof: SpartanProof) -> Result<bool> {
        // verify the proof of satisfiability
        let mut verifier_transcript = Transcript::new(b"ashlang-spartan");

        // TODO: deal with the return value of this function
        // instead of discarding it with is_ok
        Ok(serialized_proof
            .snark
            .verify(
                &serialized_proof.comm,
                &serialized_proof.inputs,
                &mut verifier_transcript,
                &serialized_proof.gens,
            )
            .is_ok())
    }
}

/// Take an ar1cs source file and do the following:
/// - calculate a witness given some inputs
/// - rearrange the R1CS variables such that the `one` variable and all inputs are at the end
/// - prepare a SpartanConfig structure to be used with `ashlang_spartan::prove`
pub fn transform_r1cs(r1cs: &str, inputs: Vec<Curve25519FieldElement>) -> Result<SpartanConfig> {
    let witness = crate::r1cs::witness::build::<Curve25519PolynomialRing>(
        r1cs,
        inputs
            .iter()
            .map(|v| Curve25519PolynomialRing(Polynomial::new(vec![*v])))
            .collect(),
    )?;
    let mut witness = witness.variables;

    // put the one variable at the end of the witness vector
    // all the R1csConstraint variables need to be modified similary
    // see the massive iterator body below
    let l = witness.len();
    witness[0] = witness[l - 1];
    witness[l - 1] = Curve25519FieldElement::from(1);

    // filter out the symbolic constraints
    let constraints = {
        let r1cs_parser: R1csParser<Curve25519PolynomialRing> = R1csParser::new(r1cs)?;
        r1cs_parser
            .constraints
            .into_iter()
            .filter(|c| !c.symbolic)
            .collect::<Vec<_>>()
    };

    // number of constraints
    let num_cons = constraints.len();
    // number of variables
    let num_vars = witness.len() - 1;
    let num_inputs = 0;

    // this variable is absurdly complex, it works for now
    // but if anything weird happens ask the spartan authors
    let mut num_non_zero_entries = 0;

    // in each constraint remap the one variable to the end of the
    // var vector
    // TODO: when inputs are supported by ashlang they will
    // need to be moved to the correct place as well.
    let remapped_constraints = constraints
        .iter()
        .map(|constraint| {
            let mut new_a = vec![];
            let mut new_b = vec![];
            let mut new_c = vec![];
            for (v, var_i) in constraint.a.clone() {
                if var_i == 0 {
                    new_a.push((v, witness.len() - 1));
                } else if var_i == witness.len() - 1 {
                    new_a.push((v, 0));
                } else {
                    new_a.push((v, var_i));
                }
            }
            for (v, var_i) in constraint.b.clone() {
                if var_i == 0 {
                    new_b.push((v, witness.len() - 1));
                } else if var_i == witness.len() - 1 {
                    new_b.push((v, 0));
                } else {
                    new_b.push((v, var_i));
                }
            }
            for (v, var_i) in constraint.c.clone() {
                if var_i == 0 {
                    new_c.push((v, witness.len() - 1));
                } else if var_i == witness.len() - 1 {
                    new_c.push((v, 0));
                } else {
                    new_c.push((v, var_i));
                }
            }
            R1csConstraint {
                a: new_a,
                b: new_b,
                c: new_c,
                out_i: None,
                comment: None,
                symbolic: false,
                symbolic_op: None,
            }
        })
        .collect::<Vec<_>>();

    // create a VarsAssignment
    let mut vars = vec![Scalar::ZERO.to_bytes(); num_vars];
    for i in 0..num_vars {
        vars[i] = to_32(witness[i].to_bytes_le());
    }

    // every row = constraint
    // every column = variable

    // We will encode the above constraints into three matrices, where
    // the coefficients in the matrix are in the little-endian byte order
    let mut a_mat: Vec<(usize, usize, [u8; 32])> = Vec::new();
    let mut b_mat: Vec<(usize, usize, [u8; 32])> = Vec::new();
    let mut c_mat: Vec<(usize, usize, [u8; 32])> = Vec::new();

    for (i, constraint) in remapped_constraints.iter().enumerate() {
        for (v, col_i) in &constraint.a {
            num_non_zero_entries += 1;
            a_mat.push((i, *col_i, to_32(v.to_bytes_le())));
        }
        for (v, col_i) in &constraint.b {
            b_mat.push((i, *col_i, to_32(v.to_bytes_le())));
        }
        for (v, col_i) in &constraint.c {
            c_mat.push((i, *col_i, to_32(v.to_bytes_le())));
        }
    }

    // println!("cons: {num_cons} vars: {num_vars}: non-0 entries: {num_non_zero_entries}");

    let inst = Instance::new(num_cons, num_vars, num_inputs, &a_mat, &b_mat, &c_mat);
    if let Err(e) = inst {
        panic!("error building instance: {:?}", e);
    }
    let inst = inst.unwrap();

    let assignment_vars = VarsAssignment::new(&vars).unwrap();

    // create an InputsAssignment
    let inputs = vec![Scalar::ZERO.to_bytes(); num_inputs];
    let assignment_inputs = InputsAssignment::new(&inputs).unwrap();

    // check if the instance we created is satisfiable
    let res = inst.is_sat(&assignment_vars, &assignment_inputs);
    // panic if the provided R1CS is not satisfied
    assert!(res.unwrap());

    Ok((
        num_cons,
        num_vars,
        num_inputs,
        num_non_zero_entries,
        inst,
        assignment_vars,
        assignment_inputs,
    ))
}
