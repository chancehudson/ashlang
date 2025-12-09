(input_len)

let x = 2
let x_vec = [10, 20, 30, 40]

x = x * x * x * x
let x_vec = x_vec * x_vec + x_vec

#let v = read_input 1

loop input_len * input_len {
  x = x + 1
  x = x * 100
}

let y = x
assert_eq x y

write_output x
#write_output v
