use crate::log;
use crate::parser::{AshParser, AstNode};
use crate::tasm::asm_parser::AsmParser;
use crate::tasm::vm::FnCall;
use anyhow::Result;
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
    extensions: Vec<String>,
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
    pub fn new(extensions: Vec<String>) -> Self {
        Compiler {
            path_to_fn: HashMap::new(),
            fn_to_path: HashMap::new(),
            print_asm: false,
            state: CompilerState::new(),
            extensions,
        }
    }

    // include a path in the build
    //
    // if the include is a file, the function name is calculated
    // and stored in the local instance
    //
    // if the include is a directory, the directory is recursively
    // walked and passed to this function
    pub fn include(&mut self, path: Utf8PathBuf) -> Result<()> {
        // first check if it's a directory
        let metadata = fs::metadata(&path)
            .map_err(|_| anyhow::anyhow!("Failed to stat metadata for include path: {:?}", path))?;
        if metadata.is_file() {
            let ext = path.extension();
            if ext.is_none() {
                anyhow::bail!("Failed to get extension for path: {:?}", path)
            }
            let ext = ext.unwrap();
            if !self.extensions.contains(&ext.to_string()) {
                return Ok(());
            }
            let name_str = path.file_stem();
            if name_str.is_none() {
                anyhow::bail!("Failed to parse file stem for include path: {:?}", path)
            }
            let name_str = name_str.unwrap().to_string();
            if self.fn_to_path.contains_key(&name_str) {
                // check if another file exists at the same path with a different
                // extension
                //
                // if so prefer the higher index file

                let existing_path = self
                    .fn_to_path
                    .get(&name_str)
                    .unwrap()
                    .canonicalize_utf8()?;
                if existing_path.parent().is_none() {
                    anyhow::bail!(
                        "Failed to canonicalize path: {:?}",
                        self.fn_to_path.get(&name_str).unwrap()
                    )
                }
                if existing_path.parent() != path.canonicalize_utf8()?.parent() {
                    log::error!(&format!(
                        "{}\n{}\n{}",
                        format!("Duplicate file/function names detected: {name_str}"),
                        format!("Path 1: {:?}", &path),
                        format!("Path 2: {:?}", self.fn_to_path.get(&name_str).unwrap())
                    ));
                }
                let existing_extension = existing_path.extension().unwrap();
                let existing_index = self
                    .extensions
                    .iter()
                    .position(|x| *x == *existing_extension)
                    .unwrap();
                let current_index = self.extensions.iter().position(|x| *x == *ext).unwrap();
                if current_index > existing_index {
                    // we'll prefer the higher indexed impl
                    self.fn_to_path.insert(name_str.clone(), path.clone());
                    self.path_to_fn.insert(path, name_str);
                }
                return Ok(());
            }
            self.fn_to_path.insert(name_str.clone(), path.clone());
            self.path_to_fn.insert(path, name_str);
        } else if metadata.is_dir() {
            let files = fs::read_dir(&path)
                .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", &path));
            for entry in files {
                let next_path = entry
                    .unwrap_or_else(|_| panic!("Failed to read dir entry: {:?}", &path))
                    .path();
                self.include(Utf8PathBuf::from_path_buf(next_path).unwrap())?;
            }
        }
        Ok(())
    }

    // loads, parses, and returns an ashlang function by name
    // returns the function as an ast
    pub fn parse_fn(&self, fn_name: &str) -> (String, String) {
        if let Some(file_path) = self.fn_to_path.get(fn_name) {
            if let Some(ext) = file_path.extension() {
                let unparsed_file = std::fs::read_to_string(file_path)
                    .unwrap_or_else(|_| panic!("Failed to read source file: {:?}", file_path));
                (unparsed_file, ext.to_string())
            } else {
                panic!("unexpected: cannot get file extension");
            }
        } else {
            log::error!(
                &format!("function is not present in sources: {fn_name}"),
                &format!("unable to find a file {fn_name}.ash in your include paths after searching recursively\n\nmake sure you have specified an include path containing this file")
            );
        }
    }

    // start at the entry file
    // parse it and determine what other files are needed
    // repeat until all files have been parsed
    pub fn compile(&mut self, entry_fn_name: &str, target: &str) -> String {
        let parser = AshParser::parse(&self.parse_fn(entry_fn_name).0, entry_fn_name);

        // tracks total number of includes for a fn in all sources
        let mut included_fn: HashMap<String, u64> = parser.fn_names.clone();
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
                let (text, ext) = self.parse_fn(&fn_name);
                match ext.as_str() {
                    "ash" => {
                        let parser = AshParser::parse(&text, &fn_name);
                        for (fn_name, count) in parser.fn_names {
                            if let Some(x) = included_fn.get_mut(&fn_name) {
                                *x += count;
                            } else {
                                included_fn.insert(fn_name, count);
                            }
                        }
                        self.state.fn_to_ast.insert(fn_name, parser.ast);
                    }
                    "tasm" => {
                        let parser = AsmParser::parse(&text, &fn_name);
                        self.state.fn_to_ast.insert(fn_name.clone(), vec![]);
                        let mut call_no_return = parser.call_type.clone();
                        call_no_return.return_type = None;
                        self.state
                            .fn_return_types
                            .insert(call_no_return.clone(), parser.call_type.clone());
                        self.state
                            .compiled_fn
                            .insert(call_no_return, parser.asm.clone());
                    }
                    "r1cs" => {
                        log::error!(&format!("r1cs files are not yet supported: {fn_name}"));
                    }
                    _ => {
                        log::error!(&format!("unexpected file extension: {ext}"));
                    }
                }
            }
        }
        match target {
            "r1cs" => {
                log::error!("r1cs compile target is not yet supported");
            }
            "tasm" => {
                // step 1: compile the entrypoint to assembly
                let mut vm = crate::tasm::vm::VM::new(&mut self.state);
                vm.eval_ast(parser.ast, vec![]);
                let mut asm = vm.asm.clone();
                asm.push("halt".to_string());

                // step 2: add functions to file
                for (fn_call, fn_asm) in &self.state.compiled_fn {
                    asm.push("\n".to_string());
                    asm.push(format!("{}:", fn_call.typed_name()));
                    asm.append(&mut fn_asm.clone());
                }

                // step 3: add blocks to file
                for v in self.state.block_fn_asm.iter() {
                    let mut block_asm = v.clone();
                    asm.push("\n".to_string());
                    asm.append(&mut block_asm);
                }
                asm.push("\n".to_string());

                if self.print_asm {
                    // prints the assembly
                    for l in &asm {
                        println!("{}", l);
                    }
                }
                asm.clone().join("\n")
            }
            _ => {
                log::error!(&format!("unexpected target: {target}"));
            }
        }
    }
}
