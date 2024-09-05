# ashlang [![CircleCI](https://dl.circleci.com/status-badge/img/gh/chancehudson/ashlang/tree/main.svg?style=shield)](https://dl.circleci.com/status-badge/redirect/gh/chancehudson/ashlang/tree/main)

A language designed to compile and execute on mathematical virtual machines.

Simplicity is the philosophy of ashlang. The language is simple to learn and expresses relationships very close to the arithmetization. Functions are globally available to encourage the development of a single, well audited, well maintained standard library of logic that can be re-used in many proving systems.

## Targets

ashlang currently supports two targets:

- [`ar1cs`](./src/r1cs/README.md) - an extended rank 1 constraint system that includes witness calculation instructions
- [`tasm`](https://triton-vm.org/spec/instructions.html) - a novel assembly language used to express instructions for the [Triton VM](https://github.com/tritonvm/triton-vm)

## Provers

ashlang supprts proving on the following systems:

- [`TritonVM/triton-vm`](https://github.com/tritonvm/triton-vm) - using `tasm` target in this crate
- [`microsoft/spartan`](https://github.com/microsoft/spartan) - using `ar1cs` target in [chancehudson/ashlang-spartan](https://github.com/chancehudson/ashlang-spartan)

## Language

ashlang is a scripting language for expressing mathematical relations between scalars and vectors in a finite field.

The language is untyped, with each variable being one of the following:

- scalar
- vector
- matrix (of any dimension)

ashlang is designed to be written in conjunction with a lower level language. Each file is a single function, it may be invoked using its filename. Directories are recursively imported and functions become globally available.

### Features

- element-wise vector operations
- throws if vectors of mismatched size are used in an operation e.g. `val[0..10] * lav[0..5]`
- functions cannot be declared, each file is a single function
- files are not imported, function calls match the filename and tell the compiler what files are needed

## Language support tracking

### Target `tasm`

- [x] scalar math operations
- [x] tuple inputs
- [x] let variables
- [x] re-assigned variables
- [x] static variables
  - [x] define static variables
  - [x] static variables as function arguments
  - [x] static variables as loop condition
  - [x] static variables as function return values
- [x] function support
  - [x] `let` assignment
  - [x] `static` assignment (static evaluation)
  - [x] return function content directly
  - [x] arguments
- [x] function auto-import
- [x] if statement
  - [x] equality
  - [x] block support
- [ ] general block support
- [x] builtin functions
  - [x] `assert_eq`
  - [x] `crash`
- [x] vector support
  - [x] vectors of any dimension e.g. `v[2][3][4][1]`
  - [x] vector variable support
  - [x] vector constants support
  - [x] vector math support
  - [ ] vector index ranges e.g. `[0..5]`
  - [ ] vector binary operation support
  - [x] vector support in functions
  - [x] vector support as function argument
  - [x] vector support as function return
  - [x] vector index access by static e.g. `v[i]`
- [x] loops

### Target `r1cs`

- [x] scalar math operations
- [ ] tuple inputs
- [x] let variables
- [x] re-assigned variables
- [x] static variables
  - [x] define static variables
  - [x] static variables as function arguments
  - [ ] static variables as loop condition
  - [x] static variables as function return values
- [x] function support
  - [x] `let` assignment
  - [x] `static` assignment (static evaluation)
  - [x] return function content directly
  - [x] arguments
- [x] function auto-import
- [ ] if statement
  - [ ] equality
  - [ ] block support
- [ ] general block support
- [x] builtin functions
  - [x] `assert_eq`
  - [x] `crash`
- [x] vector support
  - [x] vectors of any dimension e.g. `v[2][3][4][1]`
  - [x] vector variable support
  - [x] vector static support
  - [x] vector math support
  - [ ] vector index ranges e.g. `[0..5]`
  - [ ] vector binary operation support
  - [x] vector support in functions
  - [x] vector support as function argument
  - [x] vector support as function return
  - [x] vector index access by static e.g. `v[i]`
- [x] loops
