let x = 1367144358592436

# js to test this, using above statement
# let x = 1367144358592436n
# let lower = x & (2n**32n - 1n) // lower bits
# let upper = x >> 32n // upper bits
# let and = lower & upper // and combo
# let xor = lower ^ upper // xor combo

let lower = lower32(x)
let upper = upper32(x)

assert_eq(lower, 433700788)
assert_eq(upper, 318313)

assert_eq(and(lower, upper), 39712)
assert_eq(xor(lower, upper), 433939677)

assert_eq(lte(1, 1), 1)
assert_eq(lte(1, 2), 1)
assert_eq(lte(2, 1), 0)

assert_eq(lt(1, 2), 1)
assert_eq(lt(1, 1), 0)
assert_eq(lt(1, 0), 0)

assert_eq(gt(2, 1), 1)
assert_eq(gt(1, 1), 0)
assert_eq(gt(1, 2), 0)

assert_eq(gte(2, 1), 1)
assert_eq(gte(1, 1), 1)
assert_eq(gte(1, 2), 0)

assert_eq(shl(2, 1), 4)
assert_eq(shl(2, 5), 64)
assert_eq(shl(1, 32), 0)

assert_eq(shr(4, 1), 2)
assert_eq(shr(8, 1), 4)
assert_eq(shr(8, 2), 2)
assert_eq(shr(4, 2), 1)
assert_eq(shr(4, 3), 0)

assert_eq(shlc(4, 32), 4)
assert_eq(shlc(8, 32), 8)
assert_eq(shlc(0, 32), 0)

# https://onlinetoolz.net/bitshift#base=10
assert_eq(shlc(lower, 16), 3216251353)
assert_eq(shlc(upper, 16), 3681091588)
assert_eq(shlc(0, 16), 0)
assert_eq(shlc(1, 16), shl(1, 16))
