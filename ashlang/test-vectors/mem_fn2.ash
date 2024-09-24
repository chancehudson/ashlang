(in0, in1)
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
