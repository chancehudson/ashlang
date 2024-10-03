use ring_math::Polynomial;
use ring_math::PolynomialRingElement;
use scalarff::FieldElement;

use super::vm::Var;
use super::vm::VarLocation;
use crate::parser::NumOp;
use crate::r1cs::constraint::R1csConstraint;
use crate::r1cs::constraint::SymbolicOp;

pub fn generate_constraints<T: PolynomialRingElement>(
    lhs: &Var<T>,
    rhs: &Var<T>,
    operation: &NumOp,
    var_index: usize,
) -> (Vec<R1csConstraint<T::F>>, usize) {
    let mut constraints = vec![];
    let mut new_var_count = 0;
    let mut lhs_var_offset = 0;
    let mut rhs_var_offset = 0;
    for i in 0..lhs.value.values.len() {
        let (mut new_constraints, new_vars) = generate_constraints_poly(
            lhs.value.values[i].polynomial(),
            lhs.index.unwrap() + lhs_var_offset,
            rhs.value.values[i].polynomial(),
            rhs.index.unwrap() + rhs_var_offset,
            operation,
            var_index + new_var_count,
        );
        constraints.append(&mut new_constraints);
        new_var_count += new_vars;
        lhs_var_offset += lhs.value.values[i].polynomial().degree();
        rhs_var_offset += rhs.value.values[i].polynomial().degree();
    }
    (constraints, new_var_count)
}

/// Generate r1cs constraints for binary operations between polynomials
/// returns the constraints, and the number of new signals that were created
pub fn generate_constraints_poly<F: FieldElement>(
    lhs: &Polynomial<F>,
    lhs_index: usize,
    rhs: &Polynomial<F>,
    rhs_index: usize,
    operation: &NumOp,
    var_index: usize,
) -> (Vec<R1csConstraint<F>>, usize) {
    let mut constraints = vec![];
    let mut new_var_count = 0;
    match operation {
        NumOp::Add => {
            for i in 0..(usize::max(lhs.degree(), rhs.degree()) + 1) {
                let a = lhs.coefficients.get(i).unwrap_or(&F::zero()).clone();
                let b = rhs.coefficients.get(i).unwrap_or(&F::zero()).clone();
                let c = a.clone() + b.clone();
                let comment = format!("polynomial addition");
                constraints.push(R1csConstraint::new(
                    vec![(F::one(), lhs_index + i), (F::one(), rhs_index + i)],
                    vec![(F::one(), 0)],
                    vec![(F::one(), var_index + new_var_count)],
                    &comment,
                ));
                constraints.push(R1csConstraint::symbolic(
                    var_index + new_var_count,
                    vec![(F::one(), lhs_index + i), (F::one(), rhs_index + i)],
                    vec![(F::one(), 0)],
                    SymbolicOp::Mul,
                    "polynomial addition".to_string(),
                ));
                new_var_count += 1;
            }
        }
        NumOp::Sub => {
            panic!("subtraction is not implemented")
        }
        NumOp::Mul => {
            // we'll do this in two steps
            // 1. multiply all coefficients and store in variables
            // 2. combine the multiplied variables into a single polynomial

            let coefficient_count = usize::max(lhs.degree(), rhs.degree()) + 1;
            let out_coef_count = lhs.degree() + rhs.degree() + 1;

            // we need to store the resulting variable at the begining of the var_index
            new_var_count += out_coef_count;

            // a vector of the variables that will be combined using addition to form
            // each final coefficient
            let mut coef_vars = vec![vec![]; out_coef_count];

            for i in 0..coefficient_count {
                for j in 0..coefficient_count {
                    constraints.push(R1csConstraint::new(
                        vec![(F::one(), lhs_index + i)],
                        vec![(F::one(), rhs_index + j)],
                        vec![(F::one(), var_index + new_var_count)],
                        "saf",
                    ));
                    constraints.push(R1csConstraint::symbolic(
                        var_index + new_var_count,
                        vec![(F::one(), lhs_index + i)],
                        vec![(F::one(), rhs_index + j)],
                        SymbolicOp::Mul,
                        "sahfjkasf".to_string(),
                    ));
                    coef_vars[i + j].push(var_index + new_var_count);
                    new_var_count += 1;
                }
            }

            for i in 0..out_coef_count {
                // TODO: add symbolic constraint
                constraints.push(R1csConstraint::new(
                    coef_vars[i]
                        .iter()
                        .map(|v| (F::one(), *v))
                        .collect::<Vec<_>>(),
                    vec![(F::one(), 0)],
                    vec![(F::one(), var_index + i)],
                    "asf",
                ));
                constraints.push(R1csConstraint::symbolic(
                    var_index + i,
                    coef_vars[i]
                        .iter()
                        .map(|v| (F::one(), *v))
                        .collect::<Vec<_>>(),
                    vec![(F::one(), 0)],
                    SymbolicOp::Mul,
                    "sahfjkasf".to_string(),
                ));
            }
        }
        NumOp::Inv => {
            panic!("inversion is not implemented")
        }
    }
    (constraints, new_var_count)
}
