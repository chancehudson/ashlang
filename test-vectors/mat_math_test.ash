const ca = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let a = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let b = [[2, 24],[2, 1],[4, 41]]

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

let z = ca[0] * b[2]

if z[0] != 856496 {
  let _ = crash()
}

if z[1] != 98974 {
  let _ = crash()
}

let _ = mem_alloc()
