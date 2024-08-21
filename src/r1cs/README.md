# r1cs compile target

The r1cs compile target outputs system of constraints as well as symbolic constraints that can be used to build a witness. We define an `ar1cs` file format for specifying constraint systems.

## `ar1cs` file format

The ashlang r1cs file format is designed to express constraint systems in a readable way. Each line is either a constraint, or a symbolic constraint.

### Constraints

Constraints are expressed as a sequence of three bracket statements `[]`. Each bracket may contain an arbitrary number of tuples specifying a coefficient and a variable index. Each bracket is evaluated as a dot product of each coefficient and variable. Any variable not specified is implied to have a 0 coefficient.

For example: `[(1, 4)(99, 1)]` is equivalent to `1*vars[4] + 99*vars[1]`.

A full line consists of 3 bracket statements corresponding to `a`, `b`, and `c` in an equation `a*b - c = 0`. For example `[(1,6)(-1,7)][(1,0)][(1,8)(4,3)]` is equivalent to `(1*vars[6] + -1*vars[7]) * (1*vars[0]) - (1*vars[8] + 4*vars[3]) = 0`.

### Symbolic constraints

Symbolic constraints allow a prover to calculate a witness without needing a special implementation for each circuit. They define the value of a variable relative to other known variables. Symbolic constraints are discarded by the prover once the witness is calculated.

Symbolic constraints are expressed similarly to constraints, but using curly brackets (`{}`) instead of regular brackets (`[]`).

The first two brackets specify a dot product of the internal tuples, just like in a normal constraint. The final bracket may contain only a single tuple specifying `(operation, var_out)`. That is, the type of operation to be applied to the first two brackets (`mul`, `inv`, `add`, etc), and a variable that should take the value of the result (`var_out`).

For example: `{(99,0)(10, 2)}{(0,0)}{(add,3)}` is equivalent to `vars[3] = (99*vars[0] + 10*vars[2]) + (0*vars[0])`.

Note that because this constraint is not used in proving the operator is not limited to that which is provable. e.g. it's possible to take the floored division of `a` and `b`, or a bitwise `AND` or any other type of operation.

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
[(1,1)][(1,0)][(99,0)] # assigning literal (99) to signal 1
{(99,0)}{(0,0)}{(add,1)} # symbolic
[(1,2)][(1,0)][(1,0)] # assigning literal (1) to signal 2
{(1,0)}{(0,0)}{(add,2)} # symbolic
[(1,1)][(1,2)][(1,3)] # multiplication between 1 and 2 into 3
{(1,1)}{(1,2)}{(mul,3)} # symbolic
[(1,2)][(1,4)][(1,0)] # inversion of 2 into 4 (1/2)
{(1,2)}{}{(inv,4)} # symbolic
[(1,1)][(1,4)][(1,5)] # multiplication of 1 and 4 into 5 (2/2)
{(1,1)}{(1,4)}{(mul,5)} # symbolic
[(1,3)(1,5)][(1,0)][(1,6)] # addition between 3 and 5 into 6
{(1,3)(1,5)}{(1,0)}{(mul,6)} # symbolic
[(1,7)][(1,0)][(10,0)] # assigning literal (10) to signal 7
{(10,0)}{(0,0)}{(add,7)} # symbolic
[(1,6)(-1,7)][(1,0)][(1,8)] # subtraction between 6 and 7 into 8
{(1,6)(-1,7)}{(1,0)}{(mul,8)} # symbolic
[(1,8)][(1,8)][(1,9)] # multiplication between 8 and 8 into 9
{(1,8)}{(1,8)}{(mul,9)} # symbolic
[(1,9)][(1,9)][(1,10)] # multiplication between 9 and 9 into 10
{(1,9)}{(1,9)}{(mul,10)} # symbolic
[(1,10)][(1,8)][(1,11)] # multiplication between 10 and 8 into 11
{(1,10)}{(1,8)}{(mul,11)} # symbolic
```

Looking through the constraints it's possible to see how each assignment is constrained and used.
