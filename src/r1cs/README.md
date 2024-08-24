# r1cs compile target

The r1cs compile target outputs system of constraints as well as symbolic constraints that can be used to build a witness. We define an `ar1cs` ascii file format for specifying constraint systems.

## `ar1cs` file format

The ashlang r1cs file format is designed to express constraint systems in a readable way. Each line is either a constraint, or a symbolic constraint.

### Constraints

Constraints are expressed as fixed sequence of `a*b-c` operations. Each of these operations should evaluate to 0.

Each variable in the line is the dot product of any number of signals.

Example: `0 = (1*x7) * (1*one) - (1*x8)`

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

```bash
let x = 99
let y = 43

let v = x * y + x / y - 10

# constant only works in foi field
assert_eq(v, 9008875010644336127)
assert_eq(pow5(v), v * v * v * v * v)

let square = 21941893
let root = sqrt(square)

# this is the low root
assert_eq(root, 899715509682497048)
assert_eq(square, root * root)

# this is the high (negative) root
let high_root = 0 - root
assert_eq(high_root, 17547028559732087273)
assert_eq(square, high_root * high_root)

# -square = root * -root
assert_eq(0 - square, root * high_root)
```

where [`pow5`](../../stdlib/pow5.ash) is implemented as so:

```bash
(v)

let v2 = v * v
let v4 = v2 * v2

return v4 * v
```

and [`assert_eq`](../../stdlib/assert_eq.ar1cs) is implemented as so:

```bash
(a, b) -> ()

(1*a) * (1*one) - (1*b) # assert equality
```

and [`sqrt`](../../stdlib/sqrt.ar1cs) is implemented as so:

```bash
(a) -> (b)

# radix = √
# square (2) root
b = (2*one) radix (1*a) # b is the square root of a

0 = (1*b) * (1*b) - (1*a) # assert that a = b*b
```

This program compiles to the following `ar1cs`:

```bash
x1 = (99*one) + (0*one)                 # let x
x2 = (43*one) + (0*one)                 # let y
x3 = (1*x1) * (1*x2)                    # let v
x4 = (1*one) / (1*x2)                   # let v
x5 = (1*x1) * (1*x4)                    # let v
x6 = (1*x3 + 1*x5) * (1*one)            # let v
x7 = (1*x6 + 18446744069414584311*one) * (1*one) # let v
x8 = (9008875010644336127*one) + (0*one) # assert_eq()
x9 = (1*x7) * (1*x7)                    # let v2
x10 = (1*x9) * (1*x9)                   # let v4
x11 = (1*x10) * (1*x7)                  # return call in pow5
x12 = (1*x7) * (1*x7)                   # return call in pow5
x13 = (1*x12) * (1*x7)                  # return call in pow5
x14 = (1*x13) * (1*x7)                  # return call in pow5
x15 = (1*x14) * (1*x7)                  # return call in pow5
x16 = (21941893*one) + (0*one)          # let square
x17 = (2*one) radix (1*x16)             # b is the square root of a
x18 = (899715509682497048*one) + (0*one) # assert_eq()
x19 = (1*x17) * (1*x17)                 # assert_eq()
x20 = (0*one + 18446744069414584320*x17) * (1*one) # let high_root
x21 = (17547028559732087273*one) + (0*one) # assert_eq()
x22 = (1*x20) * (1*x20)                 # assert_eq()
x23 = (0*one + 18446744069414584320*x16) * (1*one) # assert_eq()
x24 = (1*x17) * (1*x20)                 # assert_eq()
0 = (18446744069414584320*one) * (18446744069414584320*one) - (1*one) # field safety constraint
0 = (1*x1) * (1*one) - (99*one)         # assigning literal (99) to signal 1
0 = (1*x2) * (1*one) - (43*one)         # assigning literal (43) to signal 2
0 = (1*x1) * (1*x2) - (1*x3)            # multiplication between 1 and 2 into 3
0 = (1*x2) * (1*x4) - (1*one)           # inversion of 2 into 4 (1/2)
0 = (1*x1) * (1*x4) - (1*x5)            # multiplication of 1 and 4 into 5 (2/2)
0 = (1*x3 + 1*x5) * (1*one) - (1*x6)    # addition between 3 and 5 into 6
0 = (10*one + 1*x7) * (1*one) - (1*x6)  # subtraction between 6 and (10) into 7
0 = (1*x8) * (1*one) - (9008875010644336127*one) # assigning literal (09008875010644336127) to signal 8
0 = (1*x7 + 0*one) * (1*one) - (1*x8)   # assert equality
0 = (1*x7) * (1*x7) - (1*x9)            # multiplication between 7 and 7 into 9
0 = (1*x9) * (1*x9) - (1*x10)           # multiplication between 9 and 9 into 10
0 = (1*x10) * (1*x7) - (1*x11)          # multiplication between 10 and 7 into 11
0 = (1*x7) * (1*x7) - (1*x12)           # multiplication between 7 and 7 into 12
0 = (1*x12) * (1*x7) - (1*x13)          # multiplication between 12 and 7 into 13
0 = (1*x13) * (1*x7) - (1*x14)          # multiplication between 13 and 7 into 14
0 = (1*x14) * (1*x7) - (1*x15)          # multiplication between 14 and 7 into 15
0 = (1*x11 + 0*one) * (1*one) - (1*x15) # assert equality
0 = (1*x16) * (1*one) - (21941893*one)  # assigning literal (00000000000021941893) to signal 16
0 = (1*x17) * (1*x17) - (1*x16)         # assert that a = b*b
0 = (1*x18) * (1*one) - (899715509682497048*one) # assigning literal (00899715509682497048) to signal 18
0 = (1*x17 + 0*one) * (1*one) - (1*x18) # assert equality
0 = (1*x17) * (1*x17) - (1*x19)         # multiplication between 17 and 17 into 19
0 = (1*x16 + 0*one) * (1*one) - (1*x19) # assert equality
0 = (1*x17 + 1*x20) * (1*one) - (0*one) # subtraction between (0) and 17 into 20
0 = (1*x21) * (1*one) - (17547028559732087273*one) # assigning literal (17547028559732087273) to signal 21
0 = (1*x20 + 0*one) * (1*one) - (1*x21) # assert equality
0 = (1*x20) * (1*x20) - (1*x22)         # multiplication between 20 and 20 into 22
0 = (1*x16 + 0*one) * (1*one) - (1*x22) # assert equality
0 = (1*x16 + 1*x23) * (1*one) - (0*one) # subtraction between (0) and 16 into 23
0 = (1*x17) * (1*x20) - (1*x24)         # multiplication between 17 and 20 into 24
0 = (1*x23 + 0*one) * (1*one) - (1*x24) # assert equality
```

Looking through the constraints it's possible to see how each assignment is constrained and used. For more info on the field safety constraint see [here](https://github.com/chancehudson/ashlang/issues/29).

Run this program by cloning and running:

`cargo run -- r1cs_readme -t r1cs -i ./stdlib -i ./test-vectors -v -f foi`

## Other curves

You can compile this example for other curves by changing the `-f` argument. e.g.

`cargo run -- r1cs_readme -t r1cs -i ./stdlib -i ./test-vectors -v -f alt_bn128`

Changing the field and compiling the above example outputs the following:

```
Compile error
cannot take square root of non-residue element: 21941893
```

Can you figure out why?
