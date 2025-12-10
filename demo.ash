(input_len)

let x = 0x2
let x_vec = [10, 20, 30, 40]

x = x * x * x * x
let x_vec = x_vec * x_vec + x_vec

#let v = read_input input_len

loop input_len * input_len {
  x = x + 1
  x = x * 100
}

# asserting equality between the same witness index
# fails
# e.g.
# let y = x
# assert_eq x y

# constrain into new var
let y = x + 29
assert_eq x+29 y

let x = 10
write_output x
let o = div_floor 22 2
write_output o
#write_output v
