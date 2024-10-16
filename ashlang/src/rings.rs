use ring_math::polynomial_ring;
use ring_math::Polynomial;
use ring_math::PolynomialRingElement;

use scalarff::alt_bn128::Bn128FieldElement;
use scalarff::foi::FoiFieldElement;
use scalarff::scalar_ring;
use scalarff::Curve25519FieldElement;
use scalarff::FieldElement;

polynomial_ring!(
    Bn128PolynomialRing,
    Bn128FieldElement,
    {
        let mut p = Polynomial::new(vec![Bn128FieldElement::one()]);
        p.term(&Bn128FieldElement::one(), 64);
        p
    },
    "alt_bn128 polynomial ring"
);

polynomial_ring!(
    OxfoiPolynomialRing,
    FoiFieldElement,
    {
        let mut p = Polynomial::new(vec![FoiFieldElement::one()]);
        p.term(&FoiFieldElement::one(), 64);
        p
    },
    "oxfoi polynomial ring"
);

polynomial_ring!(
    Curve25519PolynomialRing,
    Curve25519FieldElement,
    {
        let mut p = Polynomial::new(vec![Curve25519FieldElement::one()]);
        p.term(&Curve25519FieldElement::one(), 64);
        p
    },
    "curve25519 polynomial ring"
);

// creates a scalar ring struct DilithiumRingElement
scalar_ring!(DilithiumRingElement, 8380417, "dilithium_23_bit");

// creates a polynomial ring struct
polynomial_ring!(
    DilithiumPolynomialRingElement,
    DilithiumRingElement,
    {
        // creating the ring modulus polynomial
        // here we use x^64 + 1
        let mut p = Polynomial::identity();
        p.term(&DilithiumRingElement::one(), 64);
        p
    },
    "dilithium_x64+1"
);
