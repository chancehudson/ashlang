# repeatedly calculate and store the fifth
# power of the numbers [0, 1024)
#
# see stdlib/pow5

let z[1024]

let i = 0
# possible to access an invalid index here
# if loop length is too long
loop 1024 {
  z[i] = pow5(i)
  i = i + 1
}
