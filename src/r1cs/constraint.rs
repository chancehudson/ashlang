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
}

impl From<&str> for SymbolicOp {
    fn from(input: &str) -> Self {
        match input {
            "inv" => SymbolicOp::Inv,
            "mul" => SymbolicOp::Mul,
            "add" => SymbolicOp::Add,
            _ => panic!("bad symbolic_op input"),
        }
    }
}

impl ToString for SymbolicOp {
    fn to_string(&self) -> String {
        match self {
            SymbolicOp::Inv => "inv".to_owned(),
            SymbolicOp::Mul => "mul".to_owned(),
            SymbolicOp::Add => "add".to_owned(),
        }
    }
}

impl<T: FieldElement> ToString for R1csConstraint<T> {
    fn to_string(&self) -> String {
        if self.symbolic {
            let mut out = "".to_owned();
            out.push_str("{");
            for (coef, index) in &self.a {
                out.push_str(&format!("({},{})", coef, index));
            }
            out.push_str("}{");
            for (coef, index) in &self.b {
                out.push_str(&format!("({},{})", coef, index));
            }
            out.push_str("}{");
            // push the signal that should be assigned
            // and the operation that should be applied
            out.push_str(&format!(
                "({},{})",
                self.symbolic_op.as_ref().unwrap().to_string(),
                self.out_i.unwrap()
            ));

            out.push_str("}");
            if let Some(comment) = &self.comment {
                out.push_str(&format!(" # {}", comment));
            } else {
                out.push_str(&format!(" # symbolic"));
            }
            out
        } else {
            let mut out = "".to_owned();
            out.push_str("[");
            for (coef, index) in &self.a {
                out.push_str(&format!("({},{})", coef, index));
            }
            out.push_str("][");
            for (coef, index) in &self.b {
                out.push_str(&format!("({},{})", coef, index));
            }
            out.push_str("][");
            for (coef, index) in &self.c {
                out.push_str(&format!("({},{})", coef, index));
            }
            out.push_str("]");
            if let Some(comment) = &self.comment {
                out.push_str(&format!(" # {}", comment));
            }
            out
        }
    }
}

impl<T: FieldElement> R1csConstraint<T> {
    pub fn new(
        a: Vec<(T, usize)>,
        b: Vec<(T, usize)>,
        c: Vec<(T, usize)>,
        comment: String,
    ) -> Self {
        Self {
            a,
            b,
            c,
            out_i: None,
            comment: Some(comment),
            symbolic: false,
            symbolic_op: None,
        }
    }

    pub fn symbolic(out_i: usize, a: Vec<(T, usize)>, b: Vec<(T, usize)>, op: SymbolicOp) -> Self {
        Self {
            a,
            b,
            c: vec![],
            out_i: Some(out_i),
            comment: None,
            symbolic: true,
            symbolic_op: Some(op),
        }
    }
}
