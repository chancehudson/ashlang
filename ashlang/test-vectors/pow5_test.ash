let v = 2
let c = 99

let v5 = pow5(v)

assert_eq(v5, 32)

v = 4

v5 = pow5(v)

assert_eq(v5, 1024)

v = 19

v5 = pow5(v)

assert_eq(v5, 2476099)

let d_d = [[1, 2, 3], [4, 5, 6]]

pow5(d_d[0])
pow5(d_d[0])
pow5(d_d[0])
pow5(d_d[0])
pow5(d_d[0])
let aaa = pow5(d_d[1])

assert_eq(aaa[0], 1024)
assert_eq(aaa[1], 3125)
assert_eq(aaa[2], 7776)

let az = pow5(5 * 2)
assert_eq(az, 100000)

let aaz = pow5(pow5(2))
assert_eq(aaz, 33554432)

let i0 = [1, 2, 3, 4, 5]

let i55 = pow5(pow5(i0))

let i5 = pow5(i0)
assert_eq(i5[0], 1)
assert_eq(i5[1], 32)
assert_eq(i5[2], 243)
assert_eq(i5[3], 1024)
assert_eq(i5[4], 3125)

let i5_5 = pow5(i5)
assert_eq(i5_5[0], i55[0])
assert_eq(i5_5[1], i55[1])
assert_eq(i5_5[2], i55[2])
assert_eq(i5_5[3], i55[3])
assert_eq(i5_5[4], i55[4])
