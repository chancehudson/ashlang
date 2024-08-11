# entry files may not accept arguments
# instead argv() should be provided
# by the implementation for the target
# system
()

#const aa = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let a = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let b = [[2, 24],[2, 1],[4, 41]]
let c = [[9, 744],[1838, 819],[219, 28]]

let zz = mul(a, b)
if zz[0][0] != 428248 {
  let _ = crash()
}

let cc = a[1] * b[0]

if cc[0] != 483896 {
  let _ = crash()
}

if cc[1] != 29784 {
  let _ = crash()
}

let dd = a[2][1] * b[0][1]

if dd != 1006627536 {
  let _ = crash()
}
