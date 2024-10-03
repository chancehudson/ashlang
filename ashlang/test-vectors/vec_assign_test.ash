
let high_dim[10][10][10][10][10]


let i = 0
let j = 0
let k = 0
let l = 0
let m = 0
loop 4 {
  j = 0
  loop 4 {
    k = 0
    loop 4 {
      l = 0
      loop 4 {
        m = 0
        loop 4 {
          high_dim[i][j][k][l][m] = i * j * k * l * m
          m = m + 1
        }
        l = l + 1
      }
      k = k + 1
    }
    j = j + 1
  }
  i = i + 1
}

assert_eq(high_dim[0][0][0][0][0], 0)
assert_eq(high_dim[1][1][1][1][1], 1)
assert_eq(high_dim[2][2][2][2][2], 32)
assert_eq(high_dim[3][3][3][3][3], 243)
assert_eq(high_dim[3][3][1][3][3], 81)

i = 3
j = 3
k = 2
l = 3
m = 1
assert_eq(high_dim[i][j][k][l][m], 54)
assert_eq(high_dim[i][j][k][l][m], i * j * k * l * m)

i = 1
j = 2
k = 2
l = 3
m = 1
assert_eq(high_dim[i][j][k][l][m], 12)
assert_eq(high_dim[i][j][k][l][m], i * j * k * l * m)
