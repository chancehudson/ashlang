// temporary implementation until we switch
// to a proper number library
pub fn inv(v: u64, m: u64) -> u64 {
    let v = v % m;
    if v == 0 {
        panic!("divide by zero");
    }
    let mut y = 0_u128;
    let mut x = 1_u128;
    let mut f = u128::try_from(m).unwrap();
    let mut v = u128::try_from(v).unwrap();
    let m = u128::try_from(m).unwrap();
    while v > 1 {
        let q = v / f;
        let mut t = f;
        f = v % f;
        v = t;
        t = y;
        y = x - q * y;
        x = t;
    }
    return u64::try_from(x % m).unwrap();
}
