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
