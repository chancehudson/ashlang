let v = 2
let c = 99

let v5 = pow5(v)

if v5 != 32 {
  let _ = crash()
}

v = 4

v5 = pow5(v)

if v5 != 1024 {
  let _ = crash()
}

v = 19

v5 = pow5(v)

if v5 != 2476099 {
  let _ = crash()
}

let i0 = [1, 2, 3, 4, 5]
let i5 = pow5(i0)
if i5[0] != 1 {
  let _ = crash()
}
if i5[1] != 32 {
  let _ = crash()
}
if i5[2] != 243 {
  let _ = crash()
}
if i5[3] != 1024 {
  let _ = crash()
}
if i5[4] != 3125 {
    let _ = crash()
}
