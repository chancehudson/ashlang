(in0, in1, in2)

let o = in0 * in1

if o[0][0] != 428248 {
  let _ = crash()
}
if o[0][1] != 57936 {
  let _ = crash()
}
if o[1][1] != 1241 {
  let _ = crash()
}

let o2 = in0 * in2
if o2[0][0] != 1927116 {
  let _ = crash()
}
if o2[2][1] != 1174398792 {
  let _ = crash()
}

let _ = mem_fn2(in0, in1)
