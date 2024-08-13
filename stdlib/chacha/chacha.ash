# the chacha state contains 4 32 bit constant words
# 8 32 bit key words
# 2 32 bit counter words indicating the position in the stream
# 2 32 bit nonce words that must never be repeated
# the block input will be overwritten
(key, counter, nonce, block)

# the state is stored as a 16x1 matrix
# so that individual entries can be passed
# by reference (pointer) instead of by value
let state[16][1]
let i

# nothing up my sleeve numbers
state[0][0] = 1634760805
state[1][0] = 857760878
state[2][0] = 2036477234
state[3][0] = 1797285236

# key values
i = 0
loop 8 {
  state[4 + i][0] = key[i]
  i = i + 1
}
# counter
state[12][0] = lower32(counter)
state[13][0] = upper32(counter)
# nonce
state[14][0] = lower32(nonce)
state[15][0] = upper32(nonce)

loop 10 {
  # odd rounds
  chacha_quarter(state[0], state[4], state[8], state[12])
  chacha_quarter(state[1], state[5], state[9], state[13])
  chacha_quarter(state[2], state[6], state[10], state[14])
  chacha_quarter(state[3], state[7], state[11], state[15])

  # even rounds
  chacha_quarter(state[0], state[5], state[10], state[15])
  chacha_quarter(state[1], state[6], state[11], state[12])
  chacha_quarter(state[2], state[7], state[8], state[13])
  chacha_quarter(state[3], state[4], state[9], state[14])
}

i = 0
loop 16 {
  block[i] = state[i][0] + block[i]
  i = i + 1
}
