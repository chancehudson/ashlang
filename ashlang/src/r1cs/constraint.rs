use std::collections::HashMap;
use std::fmt::Display;

use anyhow::Result;
use ring_math::Polynomial;
use scalarff::FieldElement;

// a b and c represent values in
// a constraint a * b - c = 0
// each factor specifies an array
// of coefficient, index pairs
// indices may be specified multiple times
// and will be summed
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct R1csConstraint<T: FieldElement> {
    // (coefficient, var_index)
    pub a: Vec<(T, usize)>,
    pub b: Vec<(T, usize)>,
    pub c: Vec<(T, usize)>,
    pub out_i: Option<usize>,
    pub comment: Option<String>,
    pub symbolic: bool,
    pub symbolic_op: Option<SymbolicOp>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SymbolicOp {
    Inv,
    Mul,
    Add,
    Sqrt,
    Input,       // mark a variable as being an input. Value will be assigned as part of witness
    PublicInput, // mark a variable as being exposed as a public value, if possible
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
            "public_input" => SymbolicOp::PublicInput,
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
            SymbolicOp::PublicInput => "public_input".to_owned(),
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

impl<T: FieldElement> Display for R1csConstraint<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut out = "".to_owned();
        if self.symbolic {
            // push the signal that should be assigned
            // and the operation that should be applied
            out.push_str(&format!(
                "{} = ",
                // self.symbolic_op.as_ref().unwrap().to_string(),
                index_to_string(&self.out_i.unwrap())
            ));

            out.push('(');
            for i in 0..self.a.len() {
                let (coef, index) = &self.a[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.a.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push_str(&format!(") {} (", self.symbolic_op.as_ref().unwrap()));
            for i in 0..self.b.len() {
                let (coef, index) = &self.b[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.b.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push(')');
            out.push_str(&comment_space(&out));
            if let Some(comment) = &self.comment {
                out.push_str(&format!("# {}", comment));
            } else {
                out.push_str("# symbolic");
            }
        } else {
            out.push_str("0 = (");
            for i in 0..self.a.len() {
                let (coef, index) = &self.a[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.a.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push_str(") * (");
            for i in 0..self.b.len() {
                let (coef, index) = &self.b[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.b.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push_str(") - (");
            for i in 0..self.c.len() {
                let (coef, index) = &self.c[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.c.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push(')');
            if let Some(comment) = &self.comment {
                out.push_str(&comment_space(&out));
                out.push_str(&format!("# {}", comment));
            }
        }
        write!(f, "{}", out)
    }
}

impl<T: FieldElement> R1csConstraint<T> {
    pub fn new(a: Vec<(T, usize)>, b: Vec<(T, usize)>, c: Vec<(T, usize)>, comment: &str) -> Self {
        Self {
            a,
            b,
            c,
            out_i: None,
            comment: Some(comment.to_string()),
            symbolic: false,
            symbolic_op: None,
        }
    }

    /// build a symbolic constraint used to solve the witness
    /// symbolic constraints are of the form `out_i = a (op) b`
    /// where operation may be any possible operation
    /// e.g. not limited by the nature of the proving system
    ///
    /// out_i: index of the signal to constrain (assign)
    /// a: left side operand
    /// b: right side operand
    /// op: a function to apply to the operands
    /// comment: a comment to include in the r1cs for debugging
    pub fn symbolic(
        out_i: usize,
        a: Vec<(T, usize)>,
        b: Vec<(T, usize)>,
        op: SymbolicOp,
        comment: String,
    ) -> Self {
        Self {
            a,
            b,
            c: vec![],
            out_i: Some(out_i),
            comment: Some(comment),
            symbolic: true,
            symbolic_op: Some(op),
        }
    }

    pub fn solve_symbolic(&self, vars: &HashMap<usize, T>) -> Result<T> {
        if !self.symbolic {
            return Err(anyhow::anyhow!("not a symbolic constraint"));
        }
        let mut a = T::zero();
        for (coef, index) in &self.a {
            a += coef.clone() * vars.get(index).unwrap().clone();
        }
        let mut b = T::zero();
        for (coef, index) in &self.b {
            b += coef.clone() * vars.get(index).unwrap().clone();
        }
        match self.symbolic_op.as_ref().unwrap() {
            SymbolicOp::Add => Ok(a + b),
            SymbolicOp::Mul => Ok(a * b),
            SymbolicOp::Inv => Ok(T::one() / b),
            SymbolicOp::Sqrt => {
                if a != (T::one() + T::one()) {
                    anyhow::bail!("Cannot calculate non-square root");
                }
                let l = b.legendre();
                if l == 0 {
                    Ok(T::zero())
                } else if l == 1 {
                    // always return the positive value
                    Ok(b.sqrt())
                } else {
                    return crate::log::error!(&format!(
                        "cannot take square root of non-residue element: {}",
                        b.serialize()
                    ));
                }
            }
            SymbolicOp::PublicInput => crate::log::error!(
                "cannot solve symbolic variable of type \"PublicInput\"",
                "witness build should prove public input values"
            ),
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
