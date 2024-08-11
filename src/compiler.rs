use crate::parser::{AshParser, AstNode};
use crate::vm::{ArgType, FnCall, VarLocation, VM};
use camino::Utf8PathBuf;
use std::{collections::HashMap, fs};

// things that both Compiler and VM
// need to modify
pub struct CompilerState {
    // each function gets it's own memory space
    // track where in the memory we're at
    pub memory_offset: usize,
    pub called_fn: HashMap<FnCall, u64>,
    pub fn_return_types: HashMap<FnCall, FnCall>,
    pub compiled_fn: HashMap<FnCall, Vec<String>>,
    pub fn_to_ast: HashMap<String, Vec<AstNode>>,
    pub block_counter: usize,
    pub block_fn_asm: Vec<Vec<String>>,
}

impl CompilerState {
    pub fn new() -> Self {
        CompilerState {
            memory_offset: 0,
            called_fn: HashMap::new(),
            fn_return_types: HashMap::new(),
            compiled_fn: HashMap::new(),
            fn_to_ast: HashMap::new(),
            block_counter: 0,
            block_fn_asm: vec![],
        }
    }
}

pub struct Compiler {
    path_to_fn: HashMap<Utf8PathBuf, String>,
    fn_to_path: HashMap<String, Utf8PathBuf>,
    pub print_asm: bool,
    state: CompilerState,
}

/**
 * The Compiler struct handles reading filepaths,
 * parsing files, recursively loading dependencies,
 * and then combining functions to form the final asm.
 *
 * Compiler uses many VM instances to compile individual functions.
 * Compiler is responsible for structuring each function asm into
 * a full output file.
 */
impl Compiler {
    pub fn new() -> Self {
        Compiler {
            path_to_fn: HashMap::new(),
            fn_to_path: HashMap::new(),
            print_asm: false,
            state: CompilerState::new(),
        }
    }

    // builtin functions that are globally available
    //
    // files may not use these strings as names
    pub fn builtins() -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();

        // cause execution to fall off the bottom without halting
        out.insert("crash".to_string(), vec!["crash:".to_string()]);

        out
    }

    // include a path in the build
    //
    // if the include is a file, the function name is calculated
    // and stored in the local instance
    //
    // if the include is a directory, the directory is recursively
    // walked and passed to this function
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

    // loads, parses, and returns an ashlang function by name
    // returns the function as an ast
    pub fn parse_fn(&self, fn_name: &String) -> AshParser {
        if let Some(file_path) = self.fn_to_path.get(fn_name) {
            let unparsed_file = std::fs::read_to_string(file_path)
                .unwrap_or_else(|_| panic!("Failed to read source file: {:?}", file_path));
            // let the parser throw it's error to stderr/out
            AshParser::parse(&unparsed_file)
        } else {
            panic!("function is not present in sources: {fn_name}");
        }
    }

    // start at the entry file
    // parse it and determine what other files are needed
    // repeat until all files have been parsed
    pub fn compile(&mut self, entry: &Utf8PathBuf) -> String {
        let entry_fn_name = entry.file_stem().unwrap().to_string();

        let parser = self.parse_fn(&entry_fn_name);

        // tracks total number of includes for a fn in all sources
        let mut included_fn: HashMap<String, u64> = parser.fn_names.clone();
        let builtins = Compiler::builtins();
        for (name, _v) in builtins.iter() {
            included_fn.insert(name.clone(), 0);
            self.state.fn_to_ast.insert(name.clone(), vec![]);
            self.state.fn_return_types.insert(
                FnCall {
                    name: name.clone(),
                    arg_types: vec![],
                    return_type: None,
                },
                FnCall {
                    name: name.clone(),
                    arg_types: vec![],
                    return_type: Some(ArgType {
                        location: VarLocation::Stack,
                        dimensions: vec![],
                    }),
                },
            );
            self.state.compiled_fn.insert(
                FnCall {
                    name: name.clone(),
                    arg_types: vec![],
                    return_type: None,
                },
                vec![],
            );
        }
        // step 1: build ast for all functions
        // each function has a single ast, but multiple implementations
        // based on argument types it is called with
        loop {
            if included_fn.len() == self.state.fn_to_ast.len() {
                break;
            }
            for (fn_name, _) in included_fn.clone() {
                if self.state.fn_to_ast.contains_key(&fn_name) {
                    continue;
                }
                let parser = self.parse_fn(&fn_name);
                for (fn_name, count) in parser.fn_names {
                    if let Some(x) = included_fn.get_mut(&fn_name) {
                        *x += count;
                    } else {
                        included_fn.insert(fn_name, count);
                    }
                }
                self.state.fn_to_ast.insert(fn_name, parser.ast);
            }
        }

        // step 1: compile the entrypoint to assembly
        let mut vm = VM::new(&mut self.state);
        vm.eval_ast(parser.ast, vec![]);
        let mut asm = vm.asm.clone();
        asm.push("halt".to_string());

        // step 2: add functions to file
        for (fn_call, fn_asm) in &self.state.compiled_fn {
            asm.push("\n".to_string());
            asm.push(format!("{}:", fn_call.typed_name()));
            asm.append(&mut fn_asm.clone());
            asm.push("return".to_string());
        }

        // step 3: add blocks to file
        for v in self.state.block_fn_asm.iter() {
            let mut block_asm = v.clone();
            asm.push("\n".to_string());
            asm.append(&mut block_asm);
        }

        if self.print_asm {
            // prints the assembly
            for l in &asm {
                println!("{}", l);
            }
        }
        asm.clone().join("\n")
    }
}
