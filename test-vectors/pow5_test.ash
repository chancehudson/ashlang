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

let d_d = [[1, 2, 3], [4, 5, 6]]

let aaa = pow5(d_d[1])

if aaa[0] != 1024 {
    let _ = crash()
}
if aaa[1] != 3125 {
    let _ = crash()
}
if aaa[2] != 7776 {
    let _ = crash()
}

let az = pow5(5 * 2)
if az != 100000 {
  let _ = crash()
}
let aaz = pow5(pow5(2))
if aaz != 33554432 {
  let _ = crash()
}

let i0 = [1, 2, 3, 4, 5]

let i55 = pow5(pow5(i0))

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

let i5_5 = pow5(i5)
if i5_5[0] != i55[0] {
  let _ = crash()
}
if i5_5[1] != i55[1] {
  let _ = crash()
}
if i5_5[2] != i55[2] {
  let _ = crash()
}
if i5_5[3] != i55[3] {
  let _ = crash()
}
if i5_5[4] != i55[4] {
    let _ = crash()
}
