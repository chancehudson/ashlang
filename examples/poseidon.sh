########################
# poseidon
# -
# Grassi, Khovratovich,
# Rechberger, Roy, Schofnegger

# name of the exported function
export poseidon(inputs)

let T = inputs[0]
let state = [[0], inputs[1..T + 1]]

# these constants are calculated
# according to the current field
let C = poseidon_C(T)
let M = poseidon_M(T)
let N_F = poseidon_N_F(T)
let N_P = poseidon_N_P(T)

let c_i
let round_i

# M[..][..] is accessing the elements
# that have been set in the second dimension
#
# accessing a dimension with no elements
# is an error

# a jump destination
: full_round N_F + N_P

    state * C[c_i..c_i + T]
    c_i = c_i + T

    if round_i < N_F / 2 || round_i >= N_F / 2 + N_P
        pow5(state[..][..])
    else
        pow(state[..][0])

    # element-wise multiplication
    # between vectors
    let mix_inner = M[..] * state[i / T]

    state = sum(mix_inner[..])

    full_round()

export state
