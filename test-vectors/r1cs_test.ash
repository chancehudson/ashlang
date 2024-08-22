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
let z1 = x * y
let z2 = x - y
let z3 = x / y

# test static assignment
static z00 = x + y
static z01 = x * y
static z02 = x - y
static z03 = x / y

# test signal operations with statics

# test signal operations
_ = z0 + z1
_ = z0 - z1
_ = z0 * z1
_ = z0 / z1

# test function call
let a = pow5(z0)
static b = pow5_static(2)
a = b
