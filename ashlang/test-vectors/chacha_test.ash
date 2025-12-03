let b = 2

# let state[16][1] # test

let k[2000]

static i = 0
loop 2000 {
  k[i] = b * i
  i = i + 1
}

