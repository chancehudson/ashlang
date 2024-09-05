# use values small enough that we can operate in
# any field

static ca = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let a = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let b = [[2, 24],[2, 1],[4, 41]]

# pass a matrix to a function

let z = pow5(a)
assert_eq(z[0][0], a[0][0] * a[0][0] * a[0][0] * a[0][0] * a[0][0])
assert_eq(z[0][1], a[0][1] * a[0][1] * a[0][1] * a[0][1] * a[0][1])
assert_eq(z[1][0], a[1][0] * a[1][0] * a[1][0] * a[1][0] * a[1][0])
assert_eq(z[1][1], a[1][1] * a[1][1] * a[1][1] * a[1][1] * a[1][1])
assert_eq(z[2][0], a[2][0] * a[2][0] * a[2][0] * a[2][0] * a[2][0])
assert_eq(z[2][1], a[2][1] * a[2][1] * a[2][1] * a[2][1] * a[2][1])

# used for index access by a static variable
static one = 1

# matrix math
let cmul = a[one] * b[0]
let cadd = a[one] + b[0]
let csub = a[one] - b[0]
let cdiv = a[one] / b[0]

assert_eq(cmul[0], 483896)
assert_eq(cmul[1], 29784)

assert_eq(cadd[0], 241950)
assert_eq(cadd[1], 1265)

assert_eq(csub[0], 241946)
assert_eq(csub[1], 1217)

assert_eq(cdiv[0], 120974)

# constrain relatively to avoid
# depending on a specific field
assert_eq(cdiv[1] * b[0][1], a[1][1])

# inner index access
let dd = a[2][1] * b[0][1]

assert_eq(dd, 1006627536)

assert_eq(a[0][0], 214124)
assert_eq(a[0][1], 2414)
assert_eq(a[1][0], 241948)
assert_eq(a[1][1], 1241)
assert_eq(a[2][0], 49499)
assert_eq(a[2][1], 41942814)
assert_eq(b[0][0], 2)
assert_eq(b[0][1], 24)
assert_eq(b[1][0], 2)
assert_eq(b[1][1], 1)
assert_eq(b[2][0], 4)
assert_eq(b[2][1], 41)
assert_eq(ca[0][0], 214124)
assert_eq(ca[0][1], 2414)
assert_eq(ca[1][0], 241948)
assert_eq(ca[1][1], 1241)
assert_eq(ca[2][0], 49499)
assert_eq(ca[2][1], 41942814)
