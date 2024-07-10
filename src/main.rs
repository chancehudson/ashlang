mod parser;
use parser::*;

fn main() {
    let unparsed_file = std::fs::read_to_string("test.ash").expect("cannot read ash file");
    let astnode = parse(&unparsed_file).expect("unsuccessful parse");
    // println!("{:?}", &astnode);
    // return;
    for v in astnode {
        if let AstNode::Stmt { name, expr } = v {
            
        } else {
            panic!("asfh");
        };
        // match v {
        //     AstNode::Stmt { name, expr }
        // }
        // if let AstNode::Print(n) = v {
        //     if let AstNode::Stmt {name: x, expr: y} = *n {
        //         println!("{}", x.to_string());
        //     };
        //     // println!("{:?}", (*n));
        // };
    }
}