use crate::parser::{parse, AstNode, Expr};
use std::path::PathBuf;
use std::{collections::HashMap, fs};

use crate::vm::VM;

pub struct Compiler {
    path_to_fn: HashMap<PathBuf, String>,
    fn_to_path: HashMap<String, PathBuf>,
    #[allow(unused)]
    fn_to_ast: HashMap<String, Vec<AstNode>>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            path_to_fn: HashMap::new(),
            fn_to_path: HashMap::new(),
            fn_to_ast: HashMap::new(),
        }
    }

    // TODO: read the functions in the AST to determine
    // what files to parse next
    //
    // DEPENDS: function parsing
    pub fn parse_entry(file_path: &PathBuf) -> Vec<AstNode> {
        let unparsed_file = std::fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read source file: {:?}", file_path));
        // let the parser throw it's error to stderr/out
        parse(&unparsed_file).unwrap()
    }

    pub fn include(&mut self, path: PathBuf) {
        // first check if it's a directory
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to stat metadata for include path: {:?}", path));
        let ext = path
            .extension()
            .unwrap_or_else(|| panic!("Failed to get extension for path: {:?}", path));
        if metadata.is_file() && ext == "ash" {
            let name = path.file_stem().unwrap_or_else(|| {
                panic!("Failed to parse file stem for include path: {:?}", path)
            });
            let name_str = name
                .to_str()
                .unwrap_or_else(|| panic!("Failed to unwrap filename for path: {:?}", &path))
                .to_string();
            if self.fn_to_path.contains_key(&name_str) {
                println!("Duplicate file/function names detected: {name_str}");
                println!("Path 1: {:?}", &path);
                println!("Path 2: {:?}", self.fn_to_path.get(&name_str).unwrap());
                std::process::exit(1);
            } else {
                self.fn_to_path.insert(name_str.clone(), path.clone());
                self.path_to_fn.insert(path, name_str);
            }
        } else if metadata.is_dir() {
            let files = fs::read_dir(&path)
                .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", &path));
            for entry in files {
                let next_path = entry
                    .unwrap_or_else(|_| panic!("Failed to read dir entry: {:?}", &path))
                    .path();
                self.include(next_path);
            }
        }
    }

    // start at the entry file
    // parse it and determine what other files are needed
    // repeat until all files have been parsed
    pub fn compile(&mut self, entry: &PathBuf) -> String {
        let ast = Compiler::parse_entry(entry);

        let mut vm = VM::new();
        for v in ast {
            match v {
                AstNode::Stmt(name, is_let, expr) => {
                    if is_let {
                        vm.let_var(name, expr);
                    } else {
                        vm.set_var(name, expr)
                    }
                }
                AstNode::FnVar(vars) => {
                    for v in vars {
                        vm.let_var(v, Expr::Lit(0));
                    }
                }
                AstNode::Rtrn(expr) => {
                    vm.return_expr(expr);
                }
                AstNode::Const(name, expr) => {
                    // we must be able to fully evaluate
                    // the constant at compile time
                    // e.g. the expr must contain only
                    // Expr::Lit and Expr::Val containing other consts
                    vm.const_var(name, expr);
                }
            }
        }
        vm.halt();
        // prints the assembly
        for l in &vm.asm {
            println!("{}", l);
        }
        vm.asm.clone().join("\n")
    }
}
