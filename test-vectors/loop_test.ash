let v = 0

loop 10 {
  let z = 1000
  v = v + 1
}

assert_eq(v, 10)

let c = 5
loop add(c, 1) {
  v = v + 1
}
assert_eq(v, 16)

let z = 0
loop 2 {
  loop 5 {
    loop 7 {
        z = z + 1
    }
  }
}

assert_eq(z, 70)
