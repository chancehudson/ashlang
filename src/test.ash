# entry files may not accept arguments
# instead argv() should be provided
# by the implementation for the target
# system
()

let a = 10
let b = 20

let c = addv(a, b)

if c != 30 {
    let j = crash()
}
