# RUPAIR

RUPAIR (Rust Undefined behavior Prevention And Intelligent Rectification) is a tool for detecting and fixing buffer overflows in Rust code.

## Features

- AST-based buffer overflow detection
- MIR-based buffer overflow detection (when compiled with `with-rustc` feature)
- Z3 SMT solver integration for verification
- Automatic code rectification

## Using MIR Analysis

The MIR analysis feature provides more precise detection of buffer overflows by analyzing the Mid-level Intermediate Representation of Rust code. It is used in conjunction with AST analysis when the `with-rustc` feature is enabled during compilation.

### Building with MIR Support

To enable MIR analysis alongside AST analysis, build RUPAIR with the `with-rustc` feature:

```bash
cargo build --features with-rustc
```

### Requirements for MIR Analysis

- A nightly version of Rust (recommended to match the version used by `rustc-ap-*` crates if you were to use them directly, or a recent stable/nightly for `rustc_driver` based approaches).
- Run `rustup component add rustc-dev llvm-tools-preview` to get required components for `rustc_driver`.

### Running RUPAIR

When compiled with the `with-rustc` feature, RUPAIR will automatically use both AST and MIR for its analysis phase.

```bash
# If built with --features with-rustc, MIR analysis is automatically included
cargo run --features with-rustc -- path/to/file.rs 

# If built without the feature, only AST analysis is performed
cargo run -- path/to/file.rs
```

## Usage Examples

Analyze a single file (AST only if not built with `with-rustc` feature):
```bash
cargo run -- examples/buffer_overflow_example.rs
```

Analyze a single file (AST + MIR if built with `with-rustc` feature):
```bash
cargo run --features with-rustc -- examples/buffer_overflow_example.rs
```

Analyze a directory:
```bash
# Behavior depends on how it was built (with or without with-rustc)
cargo run -- examples/
```

## How It Works

1. RUPAIR's Front-end parses Rust source files to generate an AST.
2. If compiled with `with-rustc`, it also uses `rustc_driver` to obtain MIR.
3. The Analyzer module takes both AST and MIR (if available) as input to identify potential buffer overflow candidates.
4. The Z3 SMT solver verifies if these are real overflows.
5. Confirmed overflows are fixed with appropriate bounds checks by the Rectifier.
6. Fixed code is saved to a new file.
7. The Validator (future work) could be used to verify the equivalence of the rectified programs. 