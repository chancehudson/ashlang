let _ = 0

# static variables are specified at compile time
# these are not constrained
static x = 99
static y = 1

# test static operations
#
# assignment to the let variable is constrained
# but the operation is not constrained because
# it's between two static variables
#
# it's the same as assigning a literal e.g. z0 = 100
let z0 = x + y
assert_eq(z0, 100)

let z1 = x * y
assert_eq(z1, 99)

let z2 = x - y
assert_eq(z2, 98)

let z3 = x / y
# todo: calculate literal

# test static assignment
static z00 = x + y
assert_eq(z00, 100)

static z01 = x * y
assert_eq(z01, 99)

static z02 = x - y
assert_eq(z02, 98)

static z03 = x / y
# todo: calculate literal

# test signal operations with statics
_ = z0 * x
assert_eq(_, 9900)

_ = z0 - x
assert_eq(_, 1)

_ = x - z0
assert_eq(_, x - z0)

_ = z0 + x
assert_eq(_, 199)

_ = z0 / x
# todo

_ = x / z0
# todo

# test signal operations
_ = z0 + z1
assert_eq(_, 199)

_ = z0 - z1
assert_eq(_, 1)

_ = z0 * z1
assert_eq(_, 9900)

_ = z0 / z1
# todo

# test function call
let a = pow5(z0)
assert_eq(a, 10000000000)

static b = pow5_static(2)
assert_eq(b, 32)

a = b

# test sqrt

let out = sqrt(9)
assert_eq(out, 3)
