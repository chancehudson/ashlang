(state, MDS_MATRIX, T)

let out[T]

static x = 0
loop T {
  let v = 0
  static y = 0
  loop T {
    v = v + mat_index(MDS_MATRIX, x, y, T) * state[y]
    y = y + 1
  }
  out[x] = v
  x = x + 1
}

return out
