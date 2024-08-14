# ashlang

A language designed to run on any computer that can exist.

## What is the name?

I didn't choose the name ashlang completely randomly.

If I did would the file extension stand for "arithmetic symbol hierarchy"?

## Approach

ashlang is a scripting language for expressing mathematical relations between scalars and vectors.

The language is untyped, with each variable being one of the following:
- scalar
- vector
- matrix (of any dimension)
- function

ashlang is designed to be written in conjunction with a lower level language, such as assembly or r1cs.
- each file is a function
  - files may be written in any language
- functions can only be called from `.ash` files

For example, when targetting the triton vm `.tasm` files will be loaded from the include paths as functions. Each of these can be called from `.ash` files, but not from `.tasm` files.

## Targets

The included assembler targets [`tasm`](https://triton-vm.org/spec/instructions.html) assembly and executes on the [Triton VM](https://github.com/TritonVM/triton-vm?tab=readme-ov-file#triton-vm).

The assembler is designed to be multi-stage/partially re-usable.

## Language

The language is designed to efficiently express mathematical relationships between scalars and vectors in a finite field. Main features:
- element-wise vector operations
- throws if vectors of mismatched size are used in an operation e.g. `val[0..10] * lav[0..5]`
- functions cannot be declared, each file is a single function
- files are not imported, function calls match the filename and tell the compiler what files are needed

Below is the [poseidon](https://eprint.iacr.org/2019/458.pdf) hash function implemented in a few different languages:
- [circom](https://github.com/vimwitch/poseidon-hash/blob/main/circom/poseidon.circom)
- [solidity (naive)](https://github.com/vimwitch/poseidon-solidity/blob/db5b345bc2ab542537f02ef0c07137d62e46b3cf/contracts/Poseidon.sol)
- [solidity (optimized)](https://github.com/vimwitch/poseidon-solidity/blob/main/contracts/PoseidonT3.sol)
- [javascript](https://github.com/vimwitch/poseidon-hash/blob/main/src/index.mjs)

Below is the ashlang implementation `poseidon`. Note that vectors can be manipulated using the `*+-/` operators directly.

```sh
########################
# poseidon
# -
# Grassi, Khovratovich,
# Rechberger, Roy, Schofnegger

(T, inputs)

let state[T]
state[0] = 0 # vectors are not zeroed by default
let i = 1
loop T {
  state[i] = inputs[i]
  i = i + 1
}

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
```
