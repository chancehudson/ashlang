(state, MDS_MATRIX, T)

let out[T]

static x = 0
loop T {
  let v = 0
  let y = 0
  loop T {
    v = v + mat_index(MDS_MATRIX, x, y, T) * state[y]
    y = y + 1
  }
  x = x + 1
}

return out
