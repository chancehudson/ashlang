use std::collections::HashMap;
use std::fs;
use std::marker::PhantomData;

use anyhow::Result;
use camino::Utf8PathBuf;
use lettuce::*;

use crate::*;

pub type AshlangSourceString = String;

#[derive(Clone)]
pub struct AshlangProgram<E: FieldScalar> {
    pub src: AshlangSourceString,
    _phantom: PhantomData<E>,
}

impl<E: FieldScalar> AshlangProgram<E> {
    pub fn new(src: AshlangSourceString) -> Self {
        Self {
            src,
            _phantom: PhantomData::default(),
        }
    }

    /// Execute the program using statics only. Program must not have witness variables.
    pub fn exec_static(&self, static_args: Vec<Vector<E>>) -> Result<Option<Vector<E>>> {
        Compiler::<E>::default().exec_static(&self.src, static_args)
    }

    /// Get an r1cs representing this program and a witness computation program that may be
    /// executed with inputs to produce a witness vector.
    pub fn as_r1cs(
        &self,
        input_len: usize,
        static_args: Vec<Vector<E>>,
    ) -> Result<(R1CS<E>, AshlangProgram<E>)> {
        let compiler = Compiler::<E>::default();
        compiler.compile_src(&self.src, input_len, static_args)
    }
}

impl<E: FieldScalar> ZKProgram<E> for AshlangProgram<E> {
    fn id(&self) -> Vector<E> {
        vec![].into()
    }

    fn r1cs(&self, input_len: usize, static_args: Vec<Vector<E>>) -> Result<R1CS<E>> {
        Ok(self.as_r1cs(input_len, static_args)?.0)
    }

    fn compute_wtns(&self, input: Vector<E>, static_args: Vec<Vector<E>>) -> Result<Vector<E>> {
        let wtns_program = self.as_r1cs(input.len(), static_args)?.1;
        if let Some(wtns) = wtns_program.exec_static(vec![input])? {
            Ok(wtns)
        } else {
            anyhow::bail!("ashlang: Witness program did not return a value!");
        }
    }
}

/// Compiler configuration. Contains all fields necessary to compile an ashlang program.
#[derive(Clone, Debug)]
pub struct Config<E: FieldScalar> {
    pub include_paths: Vec<Utf8PathBuf>,
    pub verbosity: u8,
    pub input: Vector<E>,
    pub statics: Vec<usize>,
    pub extension_priorities: Vec<String>,
    pub entry_fn: String,
    pub arg_fn: String,
}

// things that both Compiler and VM
// need to modify
pub struct CompilerState {
    // each function gets it's own memory space
    // track where in the memory we're at
    pub memory_offset: usize,
    is_fn_ash: HashMap<String, bool>,
    fn_to_ash_parser: HashMap<String, AshParser>,
    pub block_counter: usize,
    pub block_fn_asm: Vec<Vec<String>>,
    path_to_fn: HashMap<Utf8PathBuf, String>,
    fn_to_path: HashMap<String, Utf8PathBuf>,
    pub messages: Vec<String>,
}

impl Default for CompilerState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerState {
    pub fn new() -> Self {
        CompilerState {
            memory_offset: 0,
            fn_to_ash_parser: HashMap::new(),
            is_fn_ash: HashMap::new(),
            block_counter: 0,
            block_fn_asm: vec![],
            path_to_fn: HashMap::new(),
            fn_to_path: HashMap::new(),
            messages: vec![],
        }
    }
}

impl CompilerState {
    pub fn fn_ash_maybe(&self, fn_name: &str) -> Option<AshParser> {
        self.fn_to_ash_parser.get(fn_name).cloned()
    }
}

/// The Compiler struct handles reading filepaths,
/// parsing files, recursively loading dependencies,
/// and then combining functions to form the final asm/ar1cs.
///
/// There are two primary functions:
/// 1. Create combined ashlang strings. Compiles an entrypoint and all functions into a single
///    file.
/// 2. Create ar1cs from ashlang string. Note that ashlang programs may be of variable length based
///    on static variables.
///
///
/// Compiler should output deterministically identical ar1cs from an equal ashlang ast.
pub struct Compiler<E: FieldScalar> {
    pub print_asm: bool,
    state: CompilerState,
    extensions: Vec<String>,
    _phantom: PhantomData<E>,
}

impl<E: FieldScalar> Default for Compiler<E> {
    fn default() -> Self {
        Compiler {
            print_asm: false,
            state: CompilerState::new(),
            extensions: ["ash"].map(|v| v.to_string()).into(),
            _phantom: PhantomData::default(),
        }
    }
}

impl<E: FieldScalar> Compiler<E> {
    pub fn new(config: &Config<E>) -> Result<Self> {
        let mut compiler = Compiler {
            print_asm: false,
            state: CompilerState::new(),
            extensions: config.extension_priorities.clone(),
            _phantom: PhantomData::default(),
        };
        if let Err(e) = compiler.include_many(&config.include_paths) {
            return log::error!(&format!("Failed to include path: {:?}", e));
        }
        compiler.print_asm = config.verbosity > 0;
        Ok(compiler)
    }

    pub fn include_many(&mut self, paths: &Vec<Utf8PathBuf>) -> Result<()> {
        for path in paths {
            self.include(path)?;
        }
        Ok(())
    }

    // include a path in the build
    //
    // if the include is a file, the function name is calculated
    // and stored in the local instance
    //
    // if the include is a directory, the directory is recursively
    // walked and passed to this function
    pub fn include(&mut self, path: &Utf8PathBuf) -> Result<()> {
        // first check if it's a directory
        let metadata = fs::metadata(path)
            .map_err(|_| anyhow::anyhow!("Failed to stat metadata for include path: {:?}", path))?;
        if metadata.is_file() {
            let ext = path.extension();
            if ext.is_none() {
                return Ok(());
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
            if self.state.fn_to_path.contains_key(&name_str) {
                // check if another file exists at the same path with a different
                // extension
                //
                // if so prefer the higher index file

                let existing_path = self
                    .state
                    .fn_to_path
                    .get(&name_str)
                    .unwrap()
                    .canonicalize_utf8()?;
                if existing_path.parent().is_none() {
                    anyhow::bail!(
                        "Failed to canonicalize path: {:?}",
                        self.state.fn_to_path.get(&name_str).unwrap()
                    )
                }
                if existing_path.parent() != path.canonicalize_utf8()?.parent() {
                    return log::error!(&format!(
                        "Duplicate file/function names detected: {name_str}
Path 1: {:?}
Path 2: {:?}",
                        &path,
                        self.state.fn_to_path.get(&name_str).unwrap()
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
                    self.state.fn_to_path.insert(name_str.clone(), path.clone());
                    self.state.path_to_fn.insert(path.clone(), name_str);
                }
                return Ok(());
            }
            self.state.fn_to_path.insert(name_str.clone(), path.clone());
            self.state.path_to_fn.insert(path.clone(), name_str);
        } else if metadata.is_dir() {
            let files = fs::read_dir(path)
                .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", &path));
            for entry in files {
                let next_path = entry
                    .unwrap_or_else(|_| panic!("Failed to read dir entry: {:?}", &path))
                    .path();
                self.include(&Utf8PathBuf::from_path_buf(next_path).unwrap())?;
            }
        }
        Ok(())
    }

    // loads, parses, and returns an ashlang function by name
    // returns the function as an ast
    pub fn parse_fn(&self, fn_name: &str) -> Result<(String, String)> {
        if let Some(file_path) = self.state.fn_to_path.get(fn_name) {
            if let Some(ext) = file_path.extension() {
                let unparsed_file = std::fs::read_to_string(file_path)
                    .unwrap_or_else(|_| panic!("Failed to read source file: {:?}", file_path));
                Ok((unparsed_file, ext.to_string()))
            } else {
                panic!("unexpected: cannot get file extension");
            }
        } else {
            log::error!(
                &format!("function is not present in sources: {fn_name}"),
                &format!(
                    "unable to find a file {fn_name}.ash in your include paths after searching recursively\n\nmake sure you have specified an include path containing this file"
                )
            )
        }
    }

    /// Take an entry source file and combine entry and includes into a single ashlang source
    /// string.
    ///
    /// Combine as sources separated by == name.ext ==
    pub fn combine_src(self, entry_fn: &str) -> Result<AshlangProgram<E>> {
        let (entry_src, ext) = self.parse_fn(entry_fn)?;
        assert_eq!(ext, "ash");
        self.combine_entrypoint_src(&entry_src)
    }

    /// Take an entry source file and combine entry and includes into a single ashlang source
    /// string.
    ///
    /// Combine as sources separated by == name.ext ==
    pub fn combine_entrypoint_src(mut self, ashlang_src: &str) -> Result<AshlangProgram<E>> {
        let mut out = ashlang_src.to_string();
        let parser = AshParser::parse(ashlang_src, "entrypoint")?;
        let fn_calls = self.compile_functions(&parser)?;
        for (fn_name, call_count) in fn_calls {
            if call_count == 0 {
                continue;
            }
            if let Some(ash_parser) = self.state.fn_ash_maybe(&fn_name) {
                out.push_str(&format!("\n=====\n{fn_name}.ash\n"));
                out.push_str(&ash_parser.src);
            } else {
                anyhow::bail!("ashlang::combine_entrypoint_src: unknown function: {fn_name}")
            }
        }

        Ok(AshlangProgram::new(out))
    }

    /// Try to compile a combined ashlang source file into an r1cs and witness fn.
    pub fn compile_src(
        mut self,
        src: &AshlangSourceString,
        input_len: usize,
        static_args: Vec<Vector<E>>,
    ) -> Result<(R1CS<E>, AshlangProgram<E>)> {
        let mut src_files = src.split("\n=====\n");
        let entrypoint = src_files.next().expect("should have entrypoint");
        for fn_src in src_files {
            let (filename, src) = fn_src.split_once("\n").expect("");
            let filename = Utf8PathBuf::from(filename);
            let file_ext = filename
                .extension()
                .expect("combined source name should have file extension");
            let fn_name = filename.file_stem().expect("should have filename");
            match file_ext {
                "ash" => {
                    let parser = AshParser::parse(src, fn_name)?;
                    self.state.is_fn_ash.insert(fn_name.to_string(), true);
                    self.state
                        .fn_to_ash_parser
                        .insert(fn_name.to_string(), parser);
                }
                v => anyhow::bail!("ashlang: bad source file extension: {v}"),
            }
        }
        let entrypoint_parser = AshParser::parse(&entrypoint, "entrypoint")?;
        self.compile_parser(entrypoint_parser, input_len, static_args)
    }

    // start at the entry file
    // parse it and determine what other files are needed
    // repeat until all files have been parsed
    pub fn compile(
        self,
        entry_fn_name: &str,
        input_len: usize,
        static_args: Vec<Vector<E>>,
    ) -> Result<(R1CS<E>, AshlangProgram<E>)> {
        let parsed = self.parse_fn(entry_fn_name)?;
        let parser = AshParser::parse(&parsed.0, entry_fn_name)?;
        self.compile_parser(parser, input_len, static_args)
    }

    fn compile_functions(&mut self, parser: &AshParser) -> Result<HashMap<String, u64>> {
        // tracks total number of includes for a fn in all sources
        let mut included_fn: HashMap<String, u64> = parser.fn_names.clone();
        // step 1: build ast for all functions
        // each function has a single ast, but multiple implementations
        // based on argument types it is called with
        loop {
            if included_fn.len() == self.state.fn_to_ash_parser.len() {
                break;
            }
            for (fn_name, _) in included_fn.clone() {
                if self.state.fn_to_ash_parser.contains_key(&fn_name) {
                    continue;
                }
                let (text, ext) = self.parse_fn(&fn_name)?;
                match ext.as_str() {
                    "ash" => {
                        let parser = AshParser::parse(&text, &fn_name)?;
                        for (fn_name, count) in &parser.fn_names {
                            if let Some(x) = included_fn.get_mut(fn_name) {
                                *x += count;
                            } else {
                                included_fn.insert(fn_name.clone(), *count);
                            }
                        }
                        self.state.is_fn_ash.insert(fn_name.clone(), true);
                        self.state.fn_to_ash_parser.insert(fn_name, parser);
                    }
                    _ => {
                        return log::error!(&format!("unexpected file extension: {ext}"));
                    }
                }
            }
        }
        Ok(included_fn)
    }

    /// Take an ashlang source file and statically evaluate it. Equivalent to executing with a
    /// witness of length 0. A vector static may be returned.
    fn exec_static(
        mut self,
        ashlang_src: &str,
        static_args: Vec<Vector<E>>,
    ) -> Result<Option<Vector<E>>> {
        let parser = AshParser::parse(ashlang_src, "_")?;
        let mut vm: VM<E> = VM::new(&mut self.state, 0, static_args);

        vm.eval_ast(parser.ast)?;
        let constraints = vm
            .constraints
            .iter()
            .filter(|v| !v.is_symbolic())
            .cloned()
            .collect::<Vec<Constraint<E>>>()
            .to_vec();
        assert!(
            constraints.len() == 1, // field safety constraint
            "ashlang: static execution should have no constraints!"
        );
        if let Some(return_var) = vm.return_val {
            assert_eq!(
                return_var.location(),
                VarLocation::Static,
                "ashlang: static execution may only return static variables"
            );
            Ok(Some(return_var.static_value()?.clone()))
        } else {
            Ok(None)
        }
    }

    fn compile_parser(
        mut self,
        parser: AshParser,
        input_len: usize,
        static_args: Vec<Vector<E>>,
    ) -> Result<(R1CS<E>, AshlangProgram<E>)> {
        let mut vm: VM<E> = VM::new(&mut self.state, input_len, static_args);
        // build constraints from the AST
        vm.eval_ast(parser.ast)?;
        let mut final_constraints: Vec<Constraint<E>> = Vec::new();
        final_constraints.append(
            &mut vm
                .constraints
                .iter()
                .filter(|v| v.is_symbolic())
                .cloned()
                .collect::<Vec<Constraint<E>>>()
                .to_vec(),
        );
        final_constraints.append(
            &mut vm
                .constraints
                .iter()
                .filter(|v| !v.is_symbolic())
                .cloned()
                .collect::<Vec<Constraint<E>>>()
                .to_vec(),
        );
        // separate the symbolic constraints and the non-symbolic ones into a witness computation
        // program and an r1cs
        let constraints = final_constraints
            .iter()
            .filter(|v| !v.is_symbolic())
            .collect::<Vec<_>>();

        let wtns_len = {
            let mut max_i = 0usize;
            for constraint in &final_constraints {
                match constraint {
                    Constraint::Symbolic { out_i, .. } => {
                        if out_i > &max_i {
                            max_i = *out_i;
                        }
                    }
                    Constraint::Witness { a, b, c, .. } => {
                        for (_, i) in a {
                            if i > &max_i {
                                max_i = *i;
                            }
                        }
                        for (_, i) in b {
                            if i > &max_i {
                                max_i = *i;
                            }
                        }
                        for (_, i) in c {
                            if i > &max_i {
                                max_i = *i;
                            }
                        }
                    }
                }
            }
            max_i + 1
        };

        // determine the public outputs
        let mut output_mask = Vector::new(wtns_len);
        // take the inputs as a vector, return a witness as a vector
        let mut wtns_script = String::new();
        wtns_script.push_str("(input_len, input)\n");
        // declare the output witness vector
        wtns_script.push_str(&format!("static wtns[{wtns_len}]\n"));
        // set the first entry in the vector to the multiplicative identity
        wtns_script.push_str("wtns[0] = 1\n");
        let mut input_count = 0_usize;
        let build_linear_combo_expr = |name: &str, val: &Vec<(E, usize)>| -> String {
            if val.is_empty() {
                return format!("static {name} = 0");
            }
            let mut expr = format!("static {name} = ");
            let mut is_first = true;
            for (coef, wtns_i) in val {
                if is_first {
                    is_first = false;
                } else {
                    expr.push_str(" + ");
                }
                if coef == &E::one() {
                    expr.push_str(&format!("wtns[{wtns_i}]"));
                } else {
                    expr.push_str(&format!("{coef} * wtns[{wtns_i}]"));
                }
            }
            expr
        };
        for symbolic_constraint in final_constraints.iter().filter(|v| v.is_symbolic()) {
            match symbolic_constraint {
                Constraint::Symbolic {
                    lhs,
                    rhs,
                    out_i,
                    op,
                    ..
                } => match op {
                    SymbolicOp::Inv => {
                        let lhs = build_linear_combo_expr("lhs", &lhs);
                        let rhs = build_linear_combo_expr("rhs", &rhs);
                        wtns_script.push_str(&format!("{lhs}\n{rhs}\nwtns[{out_i}] = lhs / rhs\n"));
                    }
                    SymbolicOp::Mul => {
                        let lhs = build_linear_combo_expr("lhs", &lhs);
                        let rhs = build_linear_combo_expr("rhs", &rhs);
                        wtns_script.push_str(&format!("{lhs}\n{rhs}\nwtns[{out_i}] = lhs * rhs\n"));
                    }
                    SymbolicOp::Add => {
                        unimplemented!()
                    }
                    SymbolicOp::Sqrt => {
                        unimplemented!()
                    }
                    SymbolicOp::Input => {
                        wtns_script.push_str(&format!("wtns[{out_i}] = input[{input_count}]\n"));
                        input_count += 1;
                    }
                    SymbolicOp::Output => {
                        output_mask[*out_i] = E::one();
                    }
                },
                _ => unreachable!(),
            }
        }
        wtns_script.push_str("return wtns\n");
        let mut r1cs = R1CS::<E>::identity(constraints.len(), wtns_len);
        r1cs.output_mask = output_mask;

        for (i, constraint) in constraints.iter().enumerate() {
            match constraint {
                Constraint::Witness { a, b, c, .. } => {
                    for (coef, wtns_i) in a {
                        assert!(r1cs.a[i][*wtns_i].is_zero());
                        r1cs.a[i][*wtns_i] = *coef;
                    }
                    for (coef, wtns_i) in b {
                        assert!(r1cs.b[i][*wtns_i].is_zero());
                        r1cs.b[i][*wtns_i] = *coef;
                    }
                    for (coef, wtns_i) in c {
                        assert!(r1cs.c[i][*wtns_i].is_zero());
                        r1cs.c[i][*wtns_i] = *coef;
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok((r1cs, AshlangProgram::new(wtns_script)))
    }
}
