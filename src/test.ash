# entry files may not accept arguments
# instead argv() should be provided
# by the implementation for the target
# system
()

let a = [[214124, 2414],[241948, 1241],[49499, 41942814]]
let b = [[2, 24],[2, 1],[4, 41]]

let c = a * b
let z = a + b
let zz = a / b
let zzz = a - b

let d = 190124
let e = 2140124
let f = d * e

if 406888935376 != f {
    # TODO: support unassigned function calls
    let _ = crash()
}
