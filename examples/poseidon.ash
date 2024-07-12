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
state[0] = 0 # unnecessary, added for clarity
state[1..T] = inputs

# these constants are pre-calculated
# according to the current field
#
# constants are included in the
# program digest and do not require
# memory
const C = poseidon_C(T)
const M = poseidon_M(T)
const N_F = poseidon_N_F(T)
const N_P = poseidon_N_P(T)

let c_i
let round_i
let mix_i

# a jump point
# this creates a function `full_round`
# that moves execution to this point
#
# this function must be invoked at least
# (N_F + N_P) times, subsequent invocations
# are a no-op
: full_round N_F + N_P

    # this is muliplying state by the range
    # of C starting at c_i e.g.
    #
    # for let x in 0..c_i
    #   state[x] = state[x] * C[c_i + x]
    state = state * C[c_i]

    c_i = c_i + T

    if round_i < N_F / 2 || round_i >= N_F / 2 + N_P
        # for x in 0..state.len()
        #   state[x] = pow5(state[x])
        #
        # below is doing the above
        state = pow5(state)
    else
        state[0] = pow5(state[0])

    mix_i = 0

    : mix_inner T
        state[mix_i] = sum(M[mix_i] * state)
        mix_i = mix_i + 1
        mix_inner()
      
    round_i = round_i + 1

    full_round()

return state
