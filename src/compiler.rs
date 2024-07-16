use crate::parser::{parse, AstNode, Expr};
use camino::Utf8PathBuf;
use std::{collections::HashMap, fs};

use crate::vm::VM;

pub struct Compiler {
    path_to_fn: HashMap<Utf8PathBuf, String>,
    fn_to_path: HashMap<String, Utf8PathBuf>,
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

    pub fn include(&mut self, path: Utf8PathBuf) {
        // first check if it's a directory
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to stat metadata for include path: {:?}", path));
        if metadata.is_file() {
            let ext = path
                .extension()
                .unwrap_or_else(|| panic!("Failed to get extension for path: {:?}", path));
            if ext != "ash" {
                return;
            }
            let name_str = path
                .file_stem()
                .unwrap_or_else(|| panic!("Failed to parse file stem for include path: {:?}", path))
                .to_string();
            // if self.fn_to_path.contains_key(&name_str) {
            //     // skip for now
            //     println!("Duplicate file/function names detected: {name_str}");
            //     println!("Path 1: {:?}", &path);
            //     println!("Path 2: {:?}", self.fn_to_path.get(&name_str).unwrap());
            //     std::process::exit(1);
            // } else {
            self.fn_to_path.insert(name_str.clone(), path.clone());
            self.path_to_fn.insert(path, name_str);
            // }
        } else if metadata.is_dir() {
            let files = fs::read_dir(&path)
                .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", &path));
            for entry in files {
                let next_path = entry
                    .unwrap_or_else(|_| panic!("Failed to read dir entry: {:?}", &path))
                    .path();
                self.include(Utf8PathBuf::from_path_buf(next_path).unwrap());
            }
        }
    }

    fn ast_to_asm(
        &mut self,
        ast: Vec<AstNode>,
        included_fn: &mut HashMap<String, u64>,
    ) -> Vec<String> {
        let mut vm = VM::new();
        // stage one, build the
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
        for (name, count) in vm.fn_calls {
            if let Some(c) = included_fn.get_mut(&name) {
                *c += count;
            } else {
                included_fn.insert(name, count);
            }
        }
        vm.asm
    }

    pub fn parse_fn(&self, fn_name: &String) -> Vec<AstNode> {
        if let Some(file_path) = self.fn_to_path.get(fn_name) {
            let unparsed_file = std::fs::read_to_string(file_path)
                .unwrap_or_else(|_| panic!("Failed to read source file: {:?}", file_path));
            // let the parser throw it's error to stderr/out
            parse(&unparsed_file).unwrap()
        } else {
            panic!("function is not present in sources: {fn_name}");
        }
    }

    // start at the entry file
    // parse it and determine what other files are needed
    // repeat until all files have been parsed
    pub fn compile(&mut self, entry: &Utf8PathBuf) -> String {
        let entry_fn_name = entry.file_stem().unwrap().to_string();

        let ast = self.parse_fn(&entry_fn_name);

        // tracks total number of includes for a fn in all sources
        let mut included_fn: HashMap<String, u64> = HashMap::new();
        let mut written_fn: HashMap<String, bool> = HashMap::new();

        // step 1: compile the entrypoint to assembly
        let mut asm = self.ast_to_asm(ast, &mut included_fn);
        asm.push("halt".to_string());

        // step 2: compile each dependency function
        // step 2a: calculate function dependence from each file
        loop {
            if included_fn.len() == written_fn.len() {
                break;
            }
            let mut compiled_fns: Vec<String> = Vec::new();
            let current_fn: Vec<String> = included_fn.keys().cloned().collect();
            for fn_name in current_fn {
                let parsed = self.parse_fn(&fn_name);
                let mut dep_asm = self.ast_to_asm(parsed, &mut included_fn);
                asm.push("\n".to_string());
                asm.push(format!("{fn_name}:"));
                asm.append(&mut dep_asm);
                asm.push("return".to_string());
                compiled_fns.push(fn_name.clone());
            }
            for fn_name in compiled_fns {
                written_fn.insert(fn_name.clone(), true);
            }
        }

        // prints the assembly
        for l in &asm {
            println!("{}", l);
        }
        asm.clone().join("\n")
    }
}
