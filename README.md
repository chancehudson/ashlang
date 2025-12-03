# ashlang 

A language designed to compile programs to r1cs and execute on mathematical machines.

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
- r1cs witnesses can be computed without specialized code

## Language support tracking

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
