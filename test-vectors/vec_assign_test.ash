let vec[2][3]
vec[0][0] = 99
vec[1][2] = 999

assert_eq(vec[0][0], 99)
assert_eq(vec[1][2], 999)

let vec2[2][3]
vec2[0][0] = 99
vec2[1][2] = 999

assert_eq(vec2[0][0], 99)
assert_eq(vec2[1][2], 999)

let vec_dyn[100]
let i = 0
loop 100 {
  vec_dyn[i] = i
  i = i + 1
}

i = 0
loop 100 {
  assert_eq(vec_dyn[i], i)
  i = i + 1
}

let vec_dyn2[100]
vec_dyn2[vec_dyn[5]] = 99
assert_eq(vec_dyn2[5], 99)
assert_eq(vec_dyn2[vec_dyn[5]], 99)

# test expr in function call
assert_eq(vec_dyn[5] * vec_dyn[5], 25)
# test expr in assignment
let x = vec_dyn[5] * vec_dyn[5]
assert_eq(x, 25)

vec_dyn[5] = 99
