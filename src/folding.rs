use crate::math::field_254::Bn254FieldElement;
use crate::math::FieldElement;
use crate::parser::AshParser;
use crate::r1cs::constraint::R1csConstraint;
use crate::r1cs::parser::R1csParser;
use crate::{compiler::Compiler, r1cs::witness};
use ark_ff::PrimeField;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::R1CSVar;
use ark_relations::r1cs::{
    ConstraintSystem, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};
use sonobe::frontend::FCircuit;
use std::collections::HashMap;

use ark_bn254::{constraints::GVar, Bn254, Fr, G1Projective as Projective};
use ark_grumpkin::{constraints::GVar as GVar2, Projective as Projective2};
use sonobe::commitment::{kzg::KZG, pedersen::Pedersen};
use sonobe::folding::nova::{Nova, PreprocessorParam};
use sonobe::transcript::poseidon::poseidon_canonical_config;
use sonobe::{Error, FoldingScheme};

pub fn prove() {
    let num_steps = 5;
    let initial_state = vec![
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
        Fr::from(1_u32),
    ];

    let F_circuit = FoldingProver::<Fr>::new(()).unwrap();

    let poseidon_config = poseidon_canonical_config::<Fr>();
    let mut rng = rand::rngs::OsRng;

    /// The idea here is that eventually we could replace the next line chunk that defines the
    /// `type N = Nova<...>` by using another folding scheme that fulfills the `FoldingScheme`
    /// trait, and the rest of our code would be working without needing to be updated.
    type N = Nova<
        Projective,
        GVar,
        Projective2,
        GVar2,
        FoldingProver<Fr>,
        KZG<'static, Bn254>,
        Pedersen<Projective2>,
        false,
    >;

    println!("Prepare Nova ProverParams & VerifierParams");
    let nova_preprocess_params = PreprocessorParam::new(poseidon_config, F_circuit.clone());
    let nova_params = N::preprocess(&mut rng, &nova_preprocess_params).unwrap();

    println!("Initialize FoldingScheme");
    let mut folding_scheme = N::init(&nova_params, F_circuit, initial_state.clone()).unwrap();

    // compute a step of the IVC
    for i in 0..num_steps {
        let start = std::time::Instant::now();
        folding_scheme.prove_step(rng, vec![], None).unwrap();
        println!("Nova::prove_step {}: {:?}", i, start.elapsed());
    }

    let (running_instance, incoming_instance, cyclefold_instance) = folding_scheme.instances();

    println!("Run the Nova's IVC verifier");
    N::verify(
        nova_params.1,
        initial_state.clone(),
        folding_scheme.state(), // latest state
        Fr::from(num_steps as u32),
        running_instance,
        incoming_instance,
        cyclefold_instance,
    )
    .unwrap();
}

#[derive(Clone, Debug)]
pub struct FoldingProver<F: PrimeField + FieldElement> {
    pub r1cs: Vec<R1csConstraint<F>>,
    pub state_len: usize,
    phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField + FieldElement> FCircuit<F> for FoldingProver<F> {
    /// FCircuit defines the trait of the circuit of the F function, which is the one being folded (ie.
    /// inside the agmented F' function).
    /// The parameter z_i denotes the current state, and z_{i+1} denotes the next state after applying
    /// the step.
    type Params = ();

    /// returns a new FCircuit instance
    fn new(params: Self::Params) -> Result<Self, sonobe::Error> {
        let () = params;
        let mut compiler: Compiler<Bn254FieldElement> = Compiler::new(vec![]);
        compiler.register_ash_fn(
            "entry",
            "
            let x = input()
            let y = input()

            # prove that y = x^5

            let z = x * x
            z = z * z * x
            assert_eq(y, z)
            ",
        );
        compiler.register_ar1cs_fn("input", "(a) -> ()\n");
        compiler.register_ar1cs_fn(
            "assert_eq",
            "(a, b) -> ()
            (1*a) * (1*one) - (1*b) # assert equality
            ",
        );

        let constraints = compiler.compile("entry", "r1cs");
        Ok(Self {
            r1cs: R1csParser::new(&constraints).constraints,
            state_len: 2,
            phantom: std::marker::PhantomData,
        })
    }

    /// returns the number of elements in the state of the FCircuit, which corresponds to the
    /// FCircuit inputs.
    fn state_len(&self) -> usize {
        let witness = witness::build(&self.r1cs, vec![F::from(0_u32), F::from(0_u32)]).unwrap();
        witness.len()
    }

    /// returns the number of elements in the external inputs used by the FCircuit. External inputs
    /// are optional, and in case no external inputs are used, this method should return 0.
    fn external_inputs_len(&self) -> usize {
        0
    }
    /// computes the next state values in place, assigning z_{i+1} into z_i, and computing the new
    /// z_{i+1}
    fn step_native(
        &self,
        i: usize,
        z_i: Vec<F>,
        _external_inputs: Vec<F>,
    ) -> Result<Vec<F>, sonobe::Error> {
        let witness = witness::build(&self.r1cs, vec![z_i[0], z_i[1]]).unwrap();
        Ok(witness)
    }

    /// generates the constraints for the step of F for the given z_i
    fn generate_step_constraints(
        &self,
        cs: ConstraintSystemRef<F>,
        _i: usize,
        z_i: Vec<FpVar<F>>,
        _external_inputs: Vec<FpVar<F>>,
    ) -> Result<Vec<FpVar<F>>, SynthesisError> {
        // let mut cs = cs.into_inner().unwrap();
        let mut out: Vec<_> = Vec::new();
        let witness = witness::build(&self.r1cs, vec![z_i[0].value()?, z_i[1].value()?]).unwrap();
        let mut vars: HashMap<usize, Variable> = HashMap::new();
        for x in 0..witness.len() {
            let v = FpVar::<F>::new_witness(cs.clone(), || Ok(witness[x]))?;
            out.push(v);
            vars.insert(x, cs.new_witness_variable(|| Ok(witness[x]))?);
        }
        for constraint in &self.r1cs {
            let mut lca = LinearCombination::<F>::zero();
            for (coef, var) in &constraint.a {
                lca.0.push((*coef, *vars.get(var).unwrap()));
            }
            let mut lcb = LinearCombination::<F>::zero();
            for (coef, var) in &constraint.a {
                lcb.0.push((*coef, *vars.get(var).unwrap()));
            }
            let mut lcc = LinearCombination::<F>::zero();
            for (coef, var) in &constraint.a {
                lcc.0.push((*coef, *vars.get(var).unwrap()));
            }
            cs.enforce_constraint(lca, lcb, lcc)?;
        }
        Ok(out)
    }
}
