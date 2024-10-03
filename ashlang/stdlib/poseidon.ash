########################
# poseidon
# -
# Grassi, Khovratovich,
# Rechberger, Roy, Schofnegger

(T, inputs)

# we don't need this assertion.
# the program will throw if vectors
# of mismatched size are operated on
#
# assert_eq(T, len(inputs) + 1)

let state[T]
state[0] = 0 # vectors are not zeroed by default
let i = 1
loop T {
  state[i] = inputs[i]
  i = i + 1
}

# these constants are pre-calculated
# according to the current field
#
# constants are included in the
# program digest and do not require
# memory
#
# TODO: switch to statics
let C = poseidon_C(T)
let M = poseidon_M(T)
let N_F = poseidon_N_F(T)
let N_P = poseidon_N_P(T)

let c_i = 0

loop N_F / 2 {
  state = state * C[c_i]
  c_i = c_i + T

  state = pow5(state)

  let mix_i = 0
  loop T {
    state[mix_i] = sum(T, M[mix_i] * state)
    mix_i = mix_i + 1
  }
}

loop N_P {
  state = state * C[c_i]
  c_i = c_i + T
  state[0] = pow5(state[0])

  let mix_i = 0
  loop T {
    state[mix_i] = sum(T, M[mix_i] * state)
    mix_i = mix_i + 1
  }
}

loop N_F / 2 {
  state = state * C[c_i]
  c_i = c_i + T

  state = pow5(state)

  let mix_i = 0
  loop T {
    state[mix_i] = sum(T, M[mix_i] * state)
    mix_i = mix_i + 1
  }
}

return state
