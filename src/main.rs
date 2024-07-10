mod compiler;
mod parser;

use compiler::*;
use parser::*;

use triton_vm::prelude::*;

fn main() {
    let unparsed_file = std::fs::read_to_string("test.ash").expect("cannot read ash file");
    let ast = parse(&unparsed_file).expect("unsuccessful parse");
    let asm = compile(ast);
    // return;
    let instructions = triton_vm::parser::parse(&asm).unwrap();
    let l_instructions = triton_vm::parser::to_labelled_instructions(instructions.as_slice());
    let program = triton_vm::program::Program::new(l_instructions.as_slice());

    let public_input = PublicInput::from([]);
    let non_determinism = NonDeterminism::default();
    let (_stark, _claim, _proof) =
        triton_vm::prove_program(&program, public_input, non_determinism).unwrap();
    println!("{:?}", _stark);
    println!("{:?}", _claim);
}
