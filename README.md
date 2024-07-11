# ashlang

A language designed to run on any computer that can exist.

## Design

ashlang is described by a regex-like grammar. This grammar is parsed into an AST. The AST is designed to be compatible with both traditional computers, as well as more restrictive executors like R1CS and PLONK.

## Targets

The included compiler supports [`tasm`](https://triton-vm.org/spec/instructions.html) assembly and executes on the [Triton VM](https://github.com/TritonVM/triton-vm?tab=readme-ov-file#triton-vm).

The compiler is designed to be multi-stage/partially re-usable.