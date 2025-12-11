use super::*;

/// Use gaussian reduction to take a set of constraints and reduce them
/// using backsubstitution and linear combinations.
pub struct ConstraintOptimizer<E: FieldScalar> {
    constraints: Vec<Constraint<E>>,
    /// Symbolic constraints keyed to the witness index they write/refer to
    symbolic_constraints: HashMap<usize, Vec<Constraint<E>>>,
}

impl<E: FieldScalar> ConstraintOptimizer<E> {
    pub fn new(constraints: Vec<Constraint<E>>) -> Self {
        let mut symbolic_constraints = HashMap::<usize, Vec<_>>::default();
        let constraints = constraints
            .into_iter()
            .filter_map(|v| match &v {
                Constraint::Witness { .. } => Some(v),
                Constraint::Symbolic { out_i, .. } => {
                    symbolic_constraints.entry(*out_i).or_default().push(v);
                    None
                }
            })
            .collect();
        Self {
            constraints,
            symbolic_constraints,
        }
    }

    pub fn optimize(self) -> Vec<Constraint<E>> {
        // iterate over witness indices ?

        vec![
            self.symbolic_constraints.into_values().flatten().collect(),
            self.constraints,
        ]
        .concat()
    }
}
