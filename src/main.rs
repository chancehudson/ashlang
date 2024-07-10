mod parser;
mod compiler;

use parser::*;
use compiler::*;

fn main() {
    let unparsed_file = std::fs::read_to_string("test.ash").expect("cannot read ash file");
    let ast = parse(&unparsed_file).expect("unsuccessful parse");
    println!("{:?}", ast);
    compile(ast);
    // return;
}