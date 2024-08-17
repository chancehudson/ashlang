# r1cs

Scalars and vectors with `+*/-` base operations. `/` and `-` perform multiplication with modular inverse and addition with negation respectively.

Multiplications of degree > 2 need to be reduced to degree 2 equations.

Assignment is implemented as defining a variable and constraining it's value as equal to a constant, or another variable.

Re-assignment is implemented as creating a new variable in the r1cs. Re-assignment is thus silently assignment.

if statements are evaluating statically. Conditions must be constant

loops are evaluated statically. Iteration expr must be constant
