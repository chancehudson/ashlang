//! Core logic for the r1cs target.
mod constraint;
mod optimizer;
mod var;
mod vm;

pub use constraint::*;
pub use optimizer::*;
pub use var::*;
pub use vm::*;

use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::str::FromStr;

use anyhow::Result;
use lettuce::*;

use super::*;
