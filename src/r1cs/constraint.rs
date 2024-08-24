use std::collections::HashMap;

use crate::math::FieldElement;

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
}

impl From<&str> for SymbolicOp {
    fn from(input: &str) -> Self {
        match input {
            "/" => SymbolicOp::Inv,
            "*" => SymbolicOp::Mul,
            "+" => SymbolicOp::Add,
            "radix" => SymbolicOp::Sqrt,
            _ => panic!("bad symbolic_op input \"{input}\""),
        }
    }
}

impl ToString for SymbolicOp {
    fn to_string(&self) -> String {
        match self {
            SymbolicOp::Inv => "/".to_owned(),
            SymbolicOp::Mul => "*".to_owned(),
            SymbolicOp::Add => "+".to_owned(),
            SymbolicOp::Sqrt => "radix".to_owned(),
        }
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

impl<T: FieldElement> ToString for R1csConstraint<T> {
    fn to_string(&self) -> String {
        if self.symbolic {
            let mut out = "".to_owned();
            // push the signal that should be assigned
            // and the operation that should be applied
            out.push_str(&format!(
                "{} = ",
                // self.symbolic_op.as_ref().unwrap().to_string(),
                index_to_string(&self.out_i.unwrap())
            ));

            out.push_str("(");
            for i in 0..self.a.len() {
                let (coef, index) = &self.a[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.a.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push_str(&format!(
                ") {} (",
                self.symbolic_op.as_ref().unwrap().to_string()
            ));
            for i in 0..self.b.len() {
                let (coef, index) = &self.b[i];
                out.push_str(&format!("{}*{}", coef.serialize(), index_to_string(index)));
                if i < self.b.len() - 1 {
                    out.push_str(" + ");
                }
            }
            out.push_str(")");
            out.push_str(&comment_space(&out));
            if let Some(comment) = &self.comment {
                out.push_str(&format!("# {}", comment));
            } else {
                out.push_str(&format!("# symbolic"));
            }
            out
        } else {
            let mut out = "".to_owned();
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
            out.push_str(")");
            if let Some(comment) = &self.comment {
                out.push_str(&comment_space(&out));
                out.push_str(&format!("# {}", comment));
            }
            out
        }
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

    pub fn solve_symbolic(&self, vars: &HashMap<usize, T>) -> T {
        if !self.symbolic {
            panic!("not a symbolic constraint");
        }
        let mut a = T::zero();
        for (coef, index) in &self.a {
            a += coef.clone() * vars.get(&index).unwrap().clone();
        }
        let mut b = T::zero();
        for (coef, index) in &self.b {
            b += coef.clone() * vars.get(&index).unwrap().clone();
        }
        match self.symbolic_op.as_ref().unwrap() {
            SymbolicOp::Add => a + b,
            SymbolicOp::Mul => a * b,
            SymbolicOp::Inv => T::one() / b,
            SymbolicOp::Sqrt => {
                if a != (T::one() + T::one()) {
                    panic!("Cannot calculate non-square root");
                }
                b.sqrt()
            }
        }
    }
}
