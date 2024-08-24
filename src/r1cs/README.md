# r1cs compile target

The r1cs compile target outputs system of constraints as well as symbolic constraints that can be used to build a witness. We define an `ar1cs` ascii file format for specifying constraint systems.

## `ar1cs` file format

The ashlang r1cs file format is designed to express constraint systems in a readable way. Each line is either a constraint, or a symbolic constraint.

### Constraints

Constraints are expressed as fixed sequence of `a*b-c` operations. Each of these operations should evaluate to 0.

Each variable in the line is the dot product of any number of signals.

Example: `(1*x7) * (1*one) - (1*x8)`

Explanation: A signal `x7` is being constrained as equal to `x8`

### Symbolic constraints

Symbolic constraints allow a prover to calculate a witness without needing a special implementation for each circuit. They define the value of a variable relative to other known variables. Symbolic constraints are discarded by the prover once the witness is calculated.

Symbolic constraints are expressed similarly to constraints, but are written as equalities where the lhs is the signal being defined.

Example: `x3 = (1*x1) * (1*x2)`

Explanation: A signal `x3` is being assigned the value `x1 * x2`

### Comments

Comments are preceded by the `#` character and end at the newline.

## Example

Consider the following program:

```
let x = 99
let y = 43

let v = x * y + x / y - 10

assert_eq(v, 9008875010644336127)
assert_eq(pow5(v), v * v * v * v * v)
```

where [`pow5`](../../stdlib/pow5.ash) is implemented as so:

```
(v)

let v2 = v * v
let v4 = v2 * v2

return v4 * v
```

and [`assert_eq`](../../stdlib/assert_eq.ar1cs) is implemented as so:

```
(a, b) -> ()

(1*a) * (1*one) - (1*b) # assert equality
```

This program compiles to the following `ar1cs`:

```
x1 = (99*one) + (0*one)                 # let x
x2 = (43*one) + (0*one)                 # let y
x3 = (1*x1) * (1*x2)                    # let v
x4 = (1*one) / (1*x2)                   # let v
x5 = (1*x1) * (1*x4)                    # let v
x6 = (1*x3 + 1*x5) * (1*one)            # let v
x7 = (1*x6 + 7237005577332262213973186563042994240857116359379907606001950938285454250979*one) * (1*one) # let v
x8 = (1*x7) * (1*x7)                    # let v2
x9 = (1*x8) * (1*x8)                    # let v4
x10 = (1*x9) * (1*x7)                   # return call in pow5
x11 = (1*x7) * (1*x7)                   # return call in pow5
x12 = (1*x11) * (1*x7)                  # return call in pow5
x13 = (1*x12) * (1*x7)                  # return call in pow5
x14 = (1*x13) * (1*x7)                  # return call in pow5
0 = (7237005577332262213973186563042994240857116359379907606001950938285454250988*one) * (7237005577332262213973186563042994240857116359379907606001950938285454250988*one) - (1*one) # field safety constraint
0 = (1*x1) * (1*one) - (99*one)         # assigning literal (99) to signal 1
0 = (1*x2) * (1*one) - (43*one)         # assigning literal (43) to signal 2
0 = (1*x1) * (1*x2) - (1*x3)            # multiplication between 1 and 2 into 3
0 = (1*x2) * (1*x4) - (1*one)           # inversion of 2 into 4 (1/2)
0 = (1*x1) * (1*x4) - (1*x5)            # multiplication of 1 and 4 into 5 (2/2)
0 = (1*x3 + 1*x5) * (1*one) - (1*x6)    # addition between 3 and 5 into 6
0 = (10*one + 1*x7) * (1*one) - (1*x6)  # subtraction between 6 and (10) into 7
0 = (1*x7) * (1*x7) - (1*x8)            # multiplication between 7 and 7 into 8
0 = (1*x8) * (1*x8) - (1*x9)            # multiplication between 8 and 8 into 9
0 = (1*x9) * (1*x7) - (1*x10)           # multiplication between 9 and 7 into 10
0 = (1*x7) * (1*x7) - (1*x11)           # multiplication between 7 and 7 into 11
0 = (1*x11) * (1*x7) - (1*x12)          # multiplication between 11 and 7 into 12
0 = (1*x12) * (1*x7) - (1*x13)          # multiplication between 12 and 7 into 13
0 = (1*x13) * (1*x7) - (1*x14)          # multiplication between 13 and 7 into 14
0 = (1*x10 + 0*one) * (1*one) - (1*x14) # assert equality
```

Looking through the constraints it's possible to see how each assignment is constrained and used. For more info on the field safety constraint see [here](https://github.com/chancehudson/ashlang/issues/29).
