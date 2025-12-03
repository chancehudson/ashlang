
static INPUT_LEN = 3

# read some input data
let x = read(INPUT_LEN)

let b = 2

# let state[16][1] # test

let k[200]

static i = 0
loop 200 {
  k[i] = b * i
  i = i + 1
}

