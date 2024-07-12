# ashlang

A language designed to run on any computer that can exist.

## Design

ashlang is described by a regex-like grammar. This grammar is parsed into an AST. The AST is designed to be compatible with both traditional computers, as well as more restrictive executors like R1CS and PLONK.

## Targets

The included compiler supports [`tasm`](https://triton-vm.org/spec/instructions.html) assembly and executes on the [Triton VM](https://github.com/TritonVM/triton-vm?tab=readme-ov-file#triton-vm).

The compiler is designed to be multi-stage/partially re-usable.

## Language

The language is designed to efficiently express mathematical relationships between sets. Main features:
- element-wise vector operations
- throws if vectors of mismatched size are used in an operation e.g. `val[0..10] * lav[0..5]`
- functions cannot be declared, each file is a single function
- files are not imported, function calls match the filename and tell the compiler what files are needed

The language has a single type: field element. This is effectively an unsigned number with max value depending on the system executing the program. e.g. on an m3 macbook the max size is `2^64`. In triton vm the max size is `2^64 - 2^32 + 1` or approximately `2^64`.

Below is the [poseidon](https://eprint.iacr.org/2019/458.pdf) hash function implemented in a few different languages:
- [circom](https://github.com/vimwitch/poseidon-hash/blob/main/circom/poseidon.circom)
- [solidity (naive)](https://github.com/vimwitch/poseidon-solidity/blob/db5b345bc2ab542537f02ef0c07137d62e46b3cf/contracts/Poseidon.sol)
- [solidity (optimized)](https://github.com/vimwitch/poseidon-solidity/blob/main/contracts/PoseidonT3.sol)
- [javascript](https://github.com/vimwitch/poseidon-hash/blob/main/src/index.mjs)

Below is the ashlang implementation `poseidon`. See [examples/poseidon.sh](examples/poseidon.sh) for a commented version. Note that vectors can be manipulated using the `*+-/` operators directly.

```sh
########################
# poseidon
# -
# Grassi, Khovratovich,
# Rechberger, Roy, Schofnegger

(T, inputs)

let state[T]
state[0] = 0 # unnecessary, added for clarity
state[1..T] = inputs

const C = poseidon_C(T)
const M = poseidon_M(T)
const N_F = poseidon_N_F(T)
const N_P = poseidon_N_P(T)

let c_i
let round_i
let mix_i

: full_round N_F + N_P

    state = state * C[c_i]

    c_i = c_i + T

    if round_i < N_F / 2 || round_i >= N_F / 2 + N_P
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
```