use super::*;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum VarLocation {
    Static,
    Witness,
}

/// All variables are of static length. No heap memory is available.
#[derive(Clone, PartialEq, Debug)]
pub enum Var<E: FieldScalar> {
    // a variable in the witness vector. Known at at witness computation time only.
    // dimension of variable is always known.
    Witness { indices: Vec<usize> },
    // a static variable. Always known. Parameter of program.
    Static { value: Vector<E> },
}

impl<E: FieldScalar> Var<E> {
    pub fn new_wtns(index: usize, len: usize) -> Self {
        Self::Witness {
            indices: (0..len).map(|i| index + i).collect(),
        }
    }

    /// Where doth the variable reside?
    pub fn location(&self) -> VarLocation {
        match self {
            Self::Witness { .. } => VarLocation::Witness,
            Self::Static { .. } => VarLocation::Static,
        }
    }

    /// Variables are statically sized
    pub fn len(&self) -> usize {
        match self {
            Self::Witness { indices } => indices.len(),
            Self::Static { value } => value.len(),
        }
    }

    /// Get a reference to the value, if the variable is a static.
    /// If the variable is not a static, error.
    pub fn static_value(&self) -> Result<&Vector<E>> {
        match self {
            Self::Witness { .. } => {
                anyhow::bail!("ashlang::Var::static_value: attempted witness value access")
            }
            Self::Static { value } => Ok(value),
        }
    }

    /// Get the index of the variable in the witness, if the variable is a witness variable.
    /// If the variable is static, error.
    pub fn wtns_index(&self) -> Result<usize> {
        match self {
            Self::Witness { indices } => Ok(indices[0]),
            Self::Static { .. } => anyhow::bail!(
                "ashlang::Var::wtns_index: attempted to access witness index of static variable"
            ),
        }
    }

    /// Return a scalar, if the variable is a scalar, and the value is known.
    /// Error otherwise
    pub fn scalar_static_value(&self) -> Result<E> {
        match (self, self.is_scalar()) {
            (Self::Static { value }, true) => Ok(value[0]),
            _ => anyhow::bail!(
                "ashlang::Var::scalar_static_value: Attempted to access value that is either not scalar or not static"
            ),
        }
    }

    /// Return a scalar, if the variable is a scalar and static.
    /// Return None otherwise
    pub fn scalar_maybe(&self) -> Option<E> {
        match (self, self.is_scalar()) {
            (Self::Static { value }, true) => Some(value[0]),
            _ => None,
        }
    }

    /// Is the variable of dimension one?
    pub fn is_scalar(&self) -> bool {
        match self {
            Self::Static { value } => value.len() == 1,
            Self::Witness { indices } => indices.len() == 1,
        }
    }
}
