# sum a vector of length T of dimension >= 1
(T, input)

let i
let out = 0
loop T {
  out = out + input[i]
  i = i + 1
}

return out
