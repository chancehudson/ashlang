let a = 9000
let b = 9001

if a == b - 1 {
    let z = 99
    let zz = 999
    z = zz
    zz = b
    a = b
}

let z = 88
let zz = 8888
a = add(z, zz)

assert_eq(a, z + zz)
assert_eq(a, 8976)
