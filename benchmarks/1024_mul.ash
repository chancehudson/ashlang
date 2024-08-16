let i = 0

let z[1024]

loop 1024 {
  z[i] = i
  i = i + 1
}

let x[1024]

loop 1024 {
  z[i] = i
  i = i + 1
}

let _ = z * x
