(am, bm, cm, dm)

# a += b; d ^= a; d <<<= 16;
# c += d; b ^= c; b <<<= 12;
# a += b; d ^= a; d <<<=  8;
# c += d; b ^= c; b <<<=  7;
let a = am[0]
let b = bm[0]
let c = cm[0]
let d = dm[0]

a = lower32(a + b)
d = shlc(xor(a, d), 16)

c = lower32(c + d)
b = shlc(xor(b, c), 12)

a = lower32(a + b)
d = shlc(xor(a, d), 8)

c = lower32(c + d)
b = shlc(xor(b, c), 7)

am[0] = a
bm[0] = b
cm[0] = c
dm[0] = d
