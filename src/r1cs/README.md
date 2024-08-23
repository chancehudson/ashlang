# r1cs compile target

The r1cs compile target outputs system of constraints as well as symbolic constraints that can be used to build a witness. We define an `ar1cs` ascii file format for specifying constraint systems.

## `ar1cs` file format

The ashlang r1cs file format is designed to express constraint systems in a readable way. Each line is either a constraint, or a symbolic constraint.

### Constraints

Constraints are expressed as fixed sequence of `a*b-c` operations. Each of these operations should evaluate to 0.

Each variable in the line is the dot product of any number of signals.

### Symbolic constraints

Symbolic constraints allow a prover to calculate a witness without needing a special implementation for each circuit. They define the value of a variable relative to other known variables. Symbolic constraints are discarded by the prover once the witness is calculated.

Symbolic constraints are expressed similarly to constraints, but are written as equalities with the lhs being the signal being defined.

### Comments

Comments are preceded by the `#` character and end at the newline.

## Example

Consider the following program:

```
let x = 99
let y = 1

let v = x * y + x / y - 10

let _ = pow5(v)
```

where `pow5` is implemented as so:
```
(v)

let v2 = v * v
let v4 = v2 * v2

return v4 * v
```

This program compiles to the following `ar1cs`:

```
x1 = (99*one) + (0*one)                 # let x
x2 = (1*one) + (0*one)                  # let y
x3 = (1*x1) * (1*x2)                    # let v
x4 = (1*one) / (1*x2)                   # let v
x5 = (1*x1) * (1*x4)                    # let v
x6 = (1*x3 + 1*x5) * (1*one)            # let v
x7 = (1*x6 + 18446744069414584311*one) * (1*one) # let v
x8 = (1*x7) * (1*x7)                    # let v2
x9 = (1*x8) * (1*x8)                    # let v4
x10 = (1*x9) * (1*x7)                   # return call in pow5
(18446744069414584320*one) * (18446744069414584320*one) - (1*one) # field safety constraint
(1*x1) * (1*one) - (99*one)             # assigning literal (99) to signal 1
(1*x2) * (1*one) - (1*one)              # assigning literal (1) to signal 2
(1*x1) * (1*x2) - (1*x3)                # multiplication between 1 and 2 into 3
(1*x2) * (1*x4) - (1*one)               # inversion of 2 into 4 (1/2)
(1*x1) * (1*x4) - (1*x5)                # multiplication of 1 and 4 into 5 (2/2)
(1*x3 + 1*x5) * (1*one) - (1*x6)        # addition between 3 and 5 into 6
(10*one + 1*x7) * (1*one) - (1*x6)      # subtraction between 6 and (10) into 7
(1*x7) * (1*x7) - (1*x8)                # multiplication between 7 and 7 into 8
(1*x8) * (1*x8) - (1*x9)                # multiplication between 8 and 8 into 9
(1*x9) * (1*x7) - (1*x10)               # multiplication between 9 and 7 into 10
```

Looking through the constraints it's possible to see how each assignment is constrained and used. For more info on the field safety constraint see [here](https://github.com/chancehudson/ashlang/issues/29).
