let x = 0
let x_vec = [10, 20, 30, 40]

x = x * x * x * x
x_vec = x_vec * x_vec + x_vec

loop 2 {
  x = x + 1
  x = x * 100
}

write_output(x)
