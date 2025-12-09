# ashlang 

A language designed to compile programs to r1cs and execute on mathematical machines.

## Language

ashlang is a scripting language for expressing mathematical relations between scalars and vectors in a finite field.

Two types of variables exist:
1. `static` variables, which are known at compile time
2. `let` variables, which are known at witness computation time

All variables are vectors. Vectors of length 1 are scalars and may be used to index other vectors using bracket syntax e.g. `let x[100]`

### Features

- all variables are vectors with static length
- automatic element-wise vector operations
- throws if vectors of mismatched size are used in an operation e.g. `val[0..10] * lav[0..5]`
- functions cannot be declared, each file is a single function
- files are not imported, function calls match the filename and tell the compiler what files are needed
- all function calls are inlined at call site
- r1cs witnesses are computed using a static ashlang script

## Language support tracking

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
- [ ] if statement
  - [ ] equality
  - [ ] block support
- [ ] general block support
- [x] precompile keywords
  - [x] `assert_eq`
  - [x] `read_input`
  - [x] `write_outpu`
- [x] vector support
  - [x] vectors of any dimension 
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

## Example

```sh
# comments start with a pound sign
# the first non-comment, non-whitespace line MAY be a declaration of function inputs
(x, y)

# if this function is called with a non-static x, this will panic
static z = x * x

# statics can be used to address vectors
# if z+1 is beyond the bounds of y, this will panic
return y[z + 1]
```

## Example ZKP

```sh
# a function is an entrypoint if it's the first function called when making a ZKP
# the first argument is a static containing the total number of inputs
(input_len)

# read the first input
# read_input is a keyword corresponding to a precompile
let x = read_input 1

# read the remaining input
let v = read_input input_len-1

# write some public signals to the zkp
# write_output is another keyword corresponding to a precompile
write_output x * v

# entrypoints should not return values
```
