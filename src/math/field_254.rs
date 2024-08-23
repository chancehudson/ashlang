use super::FieldElement;
use ark_bn254::Fr;
use ark_std::str::FromStr;

pub type Bn254FieldElement = Fr;

impl FieldElement for Fr {
    fn zero() -> Self {
        Self::from_str("0").unwrap()
    }

    fn one() -> Self {
        Self::from_str("1").unwrap()
    }

    // why does arkworks serialize 0 to an empty string?
    // why would you do that?
    fn serialize(&self) -> String {
        let s = self.clone().to_string();
        if s.len() == 0 {
            "0".to_string()
        } else {
            s
        }
    }

    fn deserialize(str: &str) -> Self {
        Fr::from_str(str).unwrap()
    }
}
