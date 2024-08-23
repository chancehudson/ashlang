use std::str::FromStr;

use super::FieldElement;
use twenty_first::math::b_field_element::BFieldElement;

pub type FoiFieldElement = BFieldElement;

impl FieldElement for BFieldElement {
    fn zero() -> Self {
        BFieldElement::from(0)
    }

    fn one() -> Self {
        BFieldElement::from(1)
    }

    fn serialize(&self) -> String {
        self.value().to_string()
    }

    fn deserialize(str: &str) -> Self {
        BFieldElement::from_str(str).unwrap()
    }
}
