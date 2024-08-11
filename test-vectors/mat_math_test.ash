const ca = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let a = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let b = [[2, 24],[2, 1],[4, 41]]

let cc = a[1] * b[0]

assert_eq(cc[0], 483896)
assert_eq(cc[1], 29784)

let dd = a[2][1] * b[0][1]

assert_eq(dd, 1006627536)

let z = ca[0] * b[2]

assert_eq(z[0], 856496)
assert_eq(z[1], 98974)

mem_alloc()
