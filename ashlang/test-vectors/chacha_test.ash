let i = 9
let b = 2

# let state[16][1] # test

loop 1600 {
  i = i + 1
  let v = 2 + i
  i = i * v
}

#chacha(state, 0, 0, state)
