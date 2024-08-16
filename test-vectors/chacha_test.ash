let i = 0

let state[16] # test

loop 16 {
  state[add(0, i)] = i
  i = i + 1
}

chacha(state, 0, 0, state)
