(a, b)

# a % b

# statics should be evaluated at compile time allowing
# non-field operations like floored division to be used
#
# for tasm execution an assembly implementation would be provided
static quotient = in \ b
static remainder = a - quotient * b

# todo: implement lt in r1cs
assert_eq(lt(remainder, quotient), 1)
assert_eq(lt(quotient, quotient), 1)

assert_eq(divisor * quotient + remainder, dividend)

return remainder
