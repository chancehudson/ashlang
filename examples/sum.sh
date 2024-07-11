# sum a vector
# input may be a vector of scalars
# or a vector of vectors
(input)

let total
let i

: sum_loop len(input)
  total = total + input[i]
  i += 1
  sum_loop()

return total
