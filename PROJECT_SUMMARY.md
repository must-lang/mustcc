# Must Compiler (mustcc) - Project Summary

## Overview

**mustcc** is a compiler for the Must programming language, written in Rust. It is a multi-stage compiler that transforms Must source code (`.mst` files) into native object files through several intermediate representations (IRs). The project is in active development with approximately 6,628 lines of Rust code across 39 source files.

**License**: MIT License (Copyright 2025 Dominik Muc)

**Version**: 0.1.0

**Edition**: Rust 2024

## Project Architecture

### Compiler Pipeline Stages

The compiler follows a traditional multi-pass architecture with the following stages:

```
Source Code (.mst files)
    ↓
1. Parser → Parser AST
    ↓
2. Module Tree → Module-resolved AST
    ↓
3. Resolver → Resolved AST (name resolution)
    ↓
4. Type Checker → Type-checked AST
    ↓
5. MIR → Mid-level IR (with type layouts)
    ↓
6. Core IR → Low-level IR (simplified expressions)
    ↓
7. Code Generator → Object File (via Cranelift)
```

### Module Structure

The codebase is organized into the following modules:

- **`parser/`** - LALRPOP-based parser for `.mst` files
- **`mod_tree/`** - Module tree construction and scope resolution
- **`resolve/`** - Name resolution and symbol binding
- **`typecheck/`** - Type checking and type inference with unification
- **`symtable/`** - Symbol table management
- **`mir/`** - Mid-level intermediate representation
- **`core/`** - Core/low-level intermediate representation
- **`codegen/`** - Code generation using Cranelift (targets x86_64)
- **`tp/`** - Type system implementation (type variables, unification)
- **`common/`** - Common utilities (Node IDs, positions, sources)
- **`error/`** - Error reporting with Ariadne for pretty diagnostics
- **`driver.rs`** - Main compiler driver orchestrating all stages

## Current Language Features

### Implemented Features

Based on code analysis and examples, the Must language currently supports:

1. **Basic Types**
   - Integers: `i32`, `i64`, `u32`, `u64`
   - Booleans: `bool`
   - Tuples: `(T1, T2, ...)`
   - Arrays (partial support)
   - User-defined structures

2. **Functions**
   - Function declarations with parameters and return types
   - Function calls
   - Recursive functions
   - Extern functions (`@extern` attribute)
   - No-mangle functions (`@no_mangle` attribute)

3. **Expressions**
   - Number literals
   - Boolean literals (`true`, `false`)
   - Variable references
   - Struct initialization with named fields
   - Struct field access (`.` operator)
   - Function calls
   - Let bindings (`let x = expr`)
   - Block expressions
   - Built-in operations (e.g., `@iadd` for integer addition)

4. **Control Flow**
   - If expressions (partial)
   - Block expressions

5. **Declarations**
   - Struct definitions
   - Function definitions with visibility modifiers
   - Module system with imports

6. **Attributes**
   - `@extern` - Mark functions as external
   - `@no_mangle` - Prevent name mangling
   - `@` prefix for built-in operations

### Partially Implemented / Not Yet Complete

From compiler warnings and code inspection:

1. **Pattern Matching** - AST structures exist but not fully implemented in backend
2. **String Literals** - Parser support exists but MIR/Core translation incomplete
3. **Character Literals** - AST exists but backend incomplete
4. **Arrays** - Type system support exists but operations incomplete
5. **Slices** - Type variants exist but not constructed
6. **Methods** - Recently removed from type declarations
7. **Generics/Type Parameters** - Framework exists but not fully functional
8. **Match Expressions** - AST exists but not translated in later stages

## Key Dependencies

- **LALRPOP** (0.22.2) - Parser generator
- **Cranelift** (0.125.3) - Code generation backend
  - Targets: x86_64, RISC-V 64
- **Ariadne** (0.5.1) - Pretty error reporting
- **Clap** (4.5.48) - Command-line argument parsing
- **Colored** (3.0.0) - Terminal color output

## Command-Line Interface

```bash
mustcc [OPTIONS] [PATH]

Options:
  -p, --print-input-ast  Only print parsed AST and exit
  -t, --typecheck-only   Only check types and exit
  -c, --core-dump        Print program in core IR
  -h, --help             Print help
  -V, --version          Print version
```

### Typical Usage

```bash
# Compile a Must project
$ mustcc examples/001

# This generates output.o which can be linked with a system linker
$ cc examples/001/output.o -o program
$ ./program
```

## Current Development Status

### What Works

✅ **Lexer and Parser** - Full LALRPOP grammar implementation
✅ **Module System** - Can parse multi-file projects with proper module paths
✅ **Name Resolution** - Symbol binding and scope resolution
✅ **Type Checker** - Hindley-Milner style type inference with unification
✅ **Basic Code Generation** - Can compile simple functions to native code
✅ **Error Reporting** - Pretty diagnostics with Ariadne

### What's In Progress

⚠️ **Backend Implementation** - Many language features parsed but not codegen'd
⚠️ **Type System Features** - Generics/type parameters framework exists
⚠️ **String/Array Operations** - Basic support but incomplete
⚠️ **Pattern Matching** - AST exists but not in backend

### Known Limitations

⚠️ From README: "Most of the language features are not yet implemented in the backend"
⚠️ 246 compiler warnings (mostly unused fields in AST structures)
⚠️ No test suite - 0 tests currently
⚠️ No CI/CD configuration
⚠️ Minimal documentation beyond README

## Project Statistics

- **Total Lines of Code**: ~6,628 lines
- **Number of Source Files**: 39 Rust files
- **Number of Modules**: 16 main modules
- **Build Time**: ~45 seconds (clean build)
- **Warning Count**: 246 warnings (mostly dead code in AST structures)
- **Test Coverage**: 0 tests

## Example Code

From `examples/001/src/mod.mst`:

```must
pub fn add(lhs: i32, rhs: i32) -> i32 {
    @iadd(lhs, rhs)
}

struct S {
    a: i32,
    b: i32,
}

@extern
@no_mangle
fn main() -> i32 {
    let x = 42;
    let y = f(x, 13, g(s(), 69));
    add(x, y)
}

fn g(x: i32, z: i32) -> i32 {
    x
}

fn f(x: i32, y: i32, z: i32) -> i32 {
    z
}

fn s() -> i32 {
    let s = S {
        a = 75,
        b = 313,
    };
    s.a
}
```

## Possible Improvements

### High Priority - Core Functionality

1. **Complete Backend Implementation**
   - Implement codegen for all parsed language features
   - Complete string and array operations
   - Implement pattern matching in MIR/Core stages
   - Add support for control flow (if/else, loops, match)

2. **Test Infrastructure**
   - Add comprehensive unit tests for each compiler stage
   - Integration tests with example programs
   - Regression test suite
   - Property-based testing for type checker

3. **Error Handling & Diagnostics**
   - Better error messages with suggestions
   - Error recovery in parser for better multi-error reporting
   - Warning system for unused code, type mismatches, etc.

### Medium Priority - Developer Experience

4. **Documentation**
   - API documentation for all public modules
   - Language specification document
   - Tutorial for writing Must programs
   - Compiler architecture documentation
   - Contributing guidelines

5. **Code Quality**
   - Address 246 compiler warnings
   - Remove dead code or implement pending features
   - Add clippy lints for code quality
   - Set up rustfmt configuration

6. **Build & Development Tools**
   - CI/CD pipeline (GitHub Actions)
   - Automated testing on commits
   - Code coverage reporting
   - Pre-commit hooks for formatting and linting

7. **Standard Library**
   - Basic I/O operations
   - String manipulation
   - Collection types (Vec, HashMap)
   - Math operations beyond @iadd

### Low Priority - Advanced Features

8. **Language Features**
   - Generics/parametric polymorphism
   - Traits/interfaces
   - Closures and lambdas
   - Enums with data
   - Operator overloading
   - Async/await support

9. **Optimization**
   - Enable Cranelift optimizations
   - Dead code elimination
   - Inline expansion
   - LLVM backend option (alternative to Cranelift)

10. **Tooling**
    - Language Server Protocol (LSP) implementation
    - Syntax highlighting for editors
    - Debugger integration (DWARF debug info)
    - Package manager for Must libraries
    - REPL for interactive development

11. **Multi-Platform Support**
    - Support more architectures (ARM, RISC-V beyond current)
    - Windows support
    - macOS support
    - WebAssembly target

12. **Compiler Features**
    - Incremental compilation
    - Parallel compilation of modules
    - Better compile-time performance
    - Memory usage optimization

### Technical Debt

13. **Code Organization**
    - Reduce coupling between compiler stages
    - Better separation of concerns in AST types
    - Consider using a single unified IR instead of MIR + Core
    - Improve symbol table design

14. **Type System**
    - Complete type inference implementation
    - Better handling of type errors
    - Subtyping support if needed
    - Lifetime/ownership system (if going for memory safety)

## Development Guidelines

### Building the Project

```bash
# Build
$ cargo build

# Run on example
$ cargo run -- examples/001

# Install globally
$ cargo install --path .
```

### Project Conventions

- Source files use `.mst` extension
- Project structure expects `src/` directory with `mod.mst` as module root
- Module paths: `bar.mst` and `bar/mod.mst` both create module `bar`
- Built-in operations use `@` prefix (e.g., `@iadd`, `@extern`)

### Contributing Areas

For new contributors, good starting points:

1. **Easy**: Fix compiler warnings, add documentation
2. **Medium**: Add tests, implement missing built-in operations
3. **Hard**: Complete backend features, implement pattern matching
4. **Expert**: Type system improvements, optimization passes

## Future Vision

The Must language aims to be a modern systems programming language with:

- Strong static typing with inference
- Zero-cost abstractions
- Memory safety (direction TBD)
- Pattern matching and algebraic data types
- Module system with clear visibility rules
- Fast compilation times
- Good error messages

The project is currently in the foundation-building phase, with the core compiler infrastructure in place and ready for feature expansion.

## Contact & Resources

- **Repository**: https://github.com/must-lang/mustcc
- **License**: MIT
- **Main Developer**: Dominik Muc

---

*This summary was generated by analyzing the codebase as of November 2025. For the most up-to-date information, refer to the repository and commit history.*
