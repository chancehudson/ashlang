(am, bm, cm, dm)

# a += b; d ^= a; d <<<= 16;
# c += d; b ^= c; b <<<= 12;
# a += b; d ^= a; d <<<=  8;
# c += d; b ^= c; b <<<=  7;
let a = am
let b = bm
let c = cm
let d = dm

a = lower32(a + b)
d = shlc(xor(a, d), 16)

c = lower32(c + d)
b = shlc(xor(b, c), 12)

a = lower32(a + b)
d = shlc(xor(a, d), 8)

c = lower32(c + d)
b = shlc(xor(b, c), 7)
