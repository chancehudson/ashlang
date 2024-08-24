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
