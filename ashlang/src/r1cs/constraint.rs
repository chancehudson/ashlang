use std::fmt::Display;

use anyhow::Result;
use lettuce::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Constraint<E: FieldScalar> {
    Witness {
        // (coefficient, var_index)
        a: Vec<(E, usize)>,
        b: Vec<(E, usize)>,
        c: Vec<(E, usize)>,
        comment_maybe: Option<String>,
    },
    Symbolic {
        lhs: Vec<(E, usize)>,
        rhs: Vec<(E, usize)>,
        /// output wtns index if this is a symbolic constraint
        out_i: usize,
        comment_maybe: Option<String>,
        op: SymbolicOp,
    },
}

/// A mathematical operation that will be used during witness
/// calculation, but not at proving time.
///
/// These operations can be arbitrarily complex and
/// are not bound by ability to be expressed as r1cs
/// constraints.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SymbolicOp {
    Inv,
    Mul,
    Add,
    Sqrt,
    Input, // mark a variable as being an input. Value will be assigned as part of witness
    Output,
}

impl From<&str> for SymbolicOp {
    fn from(input: &str) -> Self {
        match input {
            "/" => SymbolicOp::Inv,
            "*" => SymbolicOp::Mul,
            "+" => SymbolicOp::Add,
            "radix" => SymbolicOp::Sqrt,
            "input" => SymbolicOp::Input,
            "output" => SymbolicOp::Output,
            _ => panic!("bad symbolic_op input \"{input}\""),
        }
    }
}

impl Display for SymbolicOp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let out = match self {
            SymbolicOp::Inv => "/".to_owned(),
            SymbolicOp::Mul => "*".to_owned(),
            SymbolicOp::Add => "+".to_owned(),
            SymbolicOp::Sqrt => "radix".to_owned(),
            SymbolicOp::Input => "input".to_owned(),
            SymbolicOp::Output => "output".to_owned(),
        };
        write!(f, "{}", out)
    }
}

pub fn index_to_string(i: &usize) -> String {
    if i == &0 {
        return "one".to_owned();
    }
    format!("x{i}")
}

pub fn string_to_index(s: &str) -> usize {
    if s == "one" {
        return 0;
    }
    s[1..].parse::<usize>().unwrap()
}

fn comment_space(s: &str) -> String {
    if LINE_WIDTH > s.len() {
        vec![" "; LINE_WIDTH - s.len()].join("")
    } else {
        " ".to_string()
    }
}

static LINE_WIDTH: usize = 40;

impl<E: FieldScalar> Display for Constraint<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut out = "".to_owned();
        match self {
            Self::Symbolic {
                lhs,
                rhs,
                out_i,
                comment_maybe,
                op,
            } => {
                // push the signal that should be assigned
                // and the operation that should be applied
                out.push_str(&format!(
                    "{} = ",
                    // self.symbolic_op.as_ref().unwrap().to_string(),
                    index_to_string(&out_i)
                ));

                out.push('(');
                for i in 0..lhs.len() {
                    let (coef, index) = &lhs[i];
                    out.push_str(&format!("{}*{}", coef.to_string(), index_to_string(index)));
                    if i < lhs.len() - 1 {
                        out.push_str(" + ");
                    }
                }
                out.push_str(&format!(") {} (", op));
                for i in 0..rhs.len() {
                    let (coef, index) = &rhs[i];
                    out.push_str(&format!("{}*{}", coef.to_string(), index_to_string(index)));
                    if i < rhs.len() - 1 {
                        out.push_str(" + ");
                    }
                }
                out.push(')');
                out.push_str(&comment_space(&out));
                if let Some(comment) = &comment_maybe {
                    out.push_str(&format!("# {}", comment));
                } else {
                    out.push_str("# symbolic");
                }
            }
            Self::Witness {
                a,
                b,
                c,
                comment_maybe,
            } => {
                out.push_str("0 = (");
                for i in 0..a.len() {
                    let (coef, index) = &a[i];
                    out.push_str(&format!("{}*{}", coef.to_string(), index_to_string(index)));
                    if i < a.len() - 1 {
                        out.push_str(" + ");
                    }
                }
                out.push_str(") * (");
                for i in 0..b.len() {
                    let (coef, index) = &b[i];
                    out.push_str(&format!("{}*{}", coef.to_string(), index_to_string(index)));
                    if i < b.len() - 1 {
                        out.push_str(" + ");
                    }
                }
                out.push_str(") - (");
                for i in 0..c.len() {
                    let (coef, index) = &c[i];
                    out.push_str(&format!("{}*{}", coef.to_string(), index_to_string(index)));
                    if i < c.len() - 1 {
                        out.push_str(" + ");
                    }
                }
                out.push(')');
                if let Some(comment) = &comment_maybe {
                    out.push_str(&comment_space(&out));
                    out.push_str(&format!("# {}", comment));
                }
            }
        }
        write!(f, "{}", out)
    }
}

impl<E: FieldScalar> Constraint<E> {
    pub fn new(a: Vec<(E, usize)>, b: Vec<(E, usize)>, c: Vec<(E, usize)>, comment: &str) -> Self {
        Self::Witness {
            a,
            b,
            c,
            comment_maybe: Some(comment.to_string()),
        }
    }

    /// build a symbolic constraint used to solve the witness.
    /// symbolic constraints are of the form `out_i = a (op) b`
    /// where op may be any possible operation
    /// e.g. not limited by the nature of the proving system
    ///
    /// out_i: index of the signal to constrain (assign)
    /// a: left side operand
    /// b: right side operand
    /// op: a function to apply to the operands
    /// comment: a comment to include in the r1cs for debugging
    pub fn symbolic(
        out_i: usize,
        lhs: Vec<(E, usize)>,
        rhs: Vec<(E, usize)>,
        op: SymbolicOp,
        comment: String,
    ) -> Self {
        Self::Symbolic {
            lhs,
            rhs,
            out_i,
            comment_maybe: Some(comment),
            op,
        }
    }

    /// Given a symbolic constraint a witness, solve for the c constraint value.
    /// The opposite of a constraint, an arbitrary, binding assignment to the c matrix.
    pub fn solve_symbolic(&self, wtns: &Vector<E>) -> Result<E> {
        match self {
            Self::Witness { .. } => Err(anyhow::anyhow!("not a symbolic constraint")),
            Self::Symbolic {
                lhs,
                rhs,
                out_i,
                op,
                ..
            } => {
                let mut a = E::zero();
                for (coef, index) in lhs {
                    assert!(*index < wtns.len());
                    assert!(index != out_i);
                    a += *coef * wtns[*index];
                }
                let mut b = E::zero();
                for (coef, index) in rhs {
                    assert!(*index < wtns.len());
                    assert!(index != out_i);
                    b += *coef * wtns[*index];
                }
                match op {
                    SymbolicOp::Add => Ok(a + b),
                    SymbolicOp::Mul => Ok(a * b),
                    SymbolicOp::Inv => Ok(E::one() * b.inverse()),
                    SymbolicOp::Sqrt => {
                        if a != (E::one() + E::one()) {
                            anyhow::bail!("Cannot calculate non-square root");
                        }
                        let l = b.legendre();
                        if l == 0 {
                            Ok(E::zero())
                        } else if l == 1 {
                            // always return the positive value
                            Ok(b.sqrt())
                        } else {
                            return crate::log::error!(&format!(
                                "cannot take square root of non-residue element: {}",
                                b.to_string()
                            ));
                        }
                    }
                    SymbolicOp::Input => crate::log::error!(
                        "cannot solve symbolic variable of type \"Input\"",
                        "witness builder should provide input values"
                    ),
                    SymbolicOp::Output => crate::log::error!(
                        "cannot solve symbolic variable of type \"Output\"",
                        "witness builder should mark output values"
                    ),
                }
            }
        }
    }

    pub fn is_symbolic(&self) -> bool {
        matches!(self, Self::Symbolic { .. })
    }
}
