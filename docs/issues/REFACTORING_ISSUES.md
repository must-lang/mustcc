# Refactoring Issues for mustcc

This document contains detailed issue descriptions for the refactoring proposals outlined in TECHNICAL_DEBT_ANALYSIS.md. Each issue is ready to be created in GitHub.

---

## Issue 1: Split SymTable into Focused Components (Phase 1 - High Priority)

**Title**: Refactor: Split SymTable into SymbolTable, TypeRegistry, and LayoutCache

**Labels**: `refactoring`, `high-priority`, `architecture`

**Description**:

### Overview

Split the current `SymTable` into three focused components to improve separation of concerns and reduce coupling between compilation stages.

### Problem

Currently `SymTable` mixes multiple responsibilities:
- Symbol resolution (node_map, tvar_map)
- Type information management
- Layout calculation (tvar_size, tvar_order) - MIR/codegen concerns
- Runtime behavior (sizeof, check_sizes methods)

This violates the Single Responsibility Principle and creates tight coupling between typecheck and MIR stages.

### Proposed Solution

Create new `src/symbols/` module with three components:

#### 1. SymbolTable - Pure symbol resolution
```rust
pub struct SymbolTable {
    symbols: HashMap<NodeID, SymbolInfo>,
}

pub struct SymbolInfo {
    pub name: String,
    pub pos: Position,
    pub kind: SymbolKind,
    pub attributes: SymbolAttributes,
}

pub struct SymbolAttributes {
    pub builtin_name: Option<String>,
    pub is_extern: bool,
    pub mangle: bool,
}
```

#### 2. TypeRegistry - Type definitions
```rust
pub struct TypeRegistry {
    types: HashMap<TVar, TypeInfo>,
    type_order: Vec<TVar>,  // Topological order for recursive types
}

pub struct TypeInfo {
    pub name: String,
    pub pos: Position,
    pub kind: TypeKind,
}
```

#### 3. LayoutCache - Computed once, cached
```rust
pub struct LayoutCache {
    layouts: HashMap<Type, Layout>,
    sizes: HashMap<TVar, TypeSize>,
}

impl LayoutCache {
    pub fn compute_layout(&mut self, typ: &Type, type_reg: &TypeRegistry) -> Layout {
        // Compute and cache layout
    }
}
```

### Benefits

- ✅ Each component has single responsibility
- ✅ LayoutCache computed once after typecheck, not recomputed in MIR
- ✅ Easier to test each component independently
- ✅ Clear data flow and ownership
- ✅ Reduces MIR complexity by 300+ lines

### Migration Path

1. Create `src/symbols/` module structure:
   - `src/symbols/mod.rs`
   - `src/symbols/resolution.rs` (SymbolTable)
   - `src/symbols/types.rs` (TypeRegistry)
   - `src/symbols/layout.rs` (LayoutCache)

2. Move SymTable → SymbolTable:
   - Extract node_map into SymbolTable
   - Strip layout calculation code

3. Extract TypeRegistry:
   - Move tvar_map into TypeRegistry
   - Move tvar_order (topological sort)

4. Create LayoutCache:
   - Move sizeof logic
   - Move tvar_size map
   - Add caching mechanism

5. Update dependent modules:
   - `src/resolve/mod.rs` - use new SymbolTable API
   - `src/typecheck/mod.rs` - use new TypeRegistry API
   - `src/mir/mod.rs` - use LayoutCache instead of computing layouts

6. Keep old SymTable as deprecated wrapper temporarily for compatibility

7. Add tests for each component

8. Remove deprecated SymTable wrapper

### Acceptance Criteria

- [ ] New `src/symbols/` module created with three components
- [ ] SymbolTable handles only symbol resolution
- [ ] TypeRegistry handles only type definitions
- [ ] LayoutCache computes layouts once and caches them
- [ ] All existing tests pass
- [ ] New tests for each component (>80% coverage)
- [ ] Documentation for each component
- [ ] MIR no longer recomputes layouts
- [ ] Zero compiler warnings related to changes

### References

- See TECHNICAL_DEBT_ANALYSIS.md Phase 1 for detailed design
- Clean Architecture principles

---

## Issue 2: Simplify MIR Stage (Phase 2 - High Priority)

**Title**: Refactor: Simplify MIR to use LayoutCache and remove get_layout function

**Labels**: `refactoring`, `high-priority`, `performance`

**Description**:

### Overview

Simplify the MIR stage by removing the 300+ line `get_layout` function that duplicates work already done by SymTable, and make MIR a thin translation layer rather than a computation stage.

### Problem

Current MIR stage does too much:
- Lines 446-547: `get_layout` function recalculates layouts that SymTable already computed
- Lines 33-76: `make_symtable` does unnecessary conversions
- Lines 122-228: `deblock` restructures AST (should be in separate normalization stage)
- Complex var_needs_stack tracking computed late

This results in:
- Slower compilation (O(n) recomputation instead of O(1) lookup)
- Duplicated logic between SymTable::sizeof and mir::get_layout
- Harder to debug and maintain
- Unclear data flow

### Proposed Solution

#### 1. Remove get_layout function
Replace with O(1) lookups from LayoutCache:

```rust
// src/mir/mod.rs - SIMPLIFIED
pub fn translate(prog: in_a::Program, layouts: &LayoutCache) -> Result<out_a::Program, InternalError> {
    let functions = prog.functions
        .into_iter()
        .map(|f| tr_func(&prog.symbols, layouts, f))
        .collect::<Result<_, _>>()?;
    
    out_a::Program {
        symbols: prog.symbols.into_mir_symbols(),
        functions,
    }
}

fn tr_func(
    symbols: &SymbolTable,
    layouts: &LayoutCache,
    func: in_a::Func,
) -> Result<out_a::Func, InternalError> {
    let mut env = Env::new();
    
    let args = func.args.into_iter().map(|(name, is_mut, tp)| {
        let id = env.add_var(name);
        let layout = layouts.get(&tp);  // O(1) lookup!
        let mir_type = layout.to_mir_type();
        (id, is_mut, mir_type, layout.needs_stack())
    }).collect();
    
    let body = tr_expr(&mut env, layouts, func.body)?;
    
    Ok(out_a::Func {
        id: func.id,
        args,
        body,
    })
}
```

#### 2. Update driver.rs
```rust
let prog = typecheck::translate(&mut ctx, prog)?;

// NEW: Compute layouts once after typecheck
let layout_cache = LayoutCache::compute(&prog.sym_table);

// MIR uses pre-computed layouts
let prog = mir::translate(prog, &layout_cache)?;
```

### Benefits

- ✅ Faster compilation (>30% improvement expected in MIR stage)
- ✅ Remove 300+ lines of duplicate code
- ✅ Clearer data flow: typecheck → compute layouts → MIR uses layouts
- ✅ Easier to debug
- ✅ No more O(n) recursive layout calculations in MIR

### Migration Path

1. **Depends on**: Issue #1 (LayoutCache must exist first)

2. Update MIR signature to accept LayoutCache parameter

3. Replace all `get_layout(st, tp)` calls with `layouts.get(&tp)`

4. Remove the 300-line `get_layout` function

5. Simplify `make_symtable` to not do layout conversions

6. Update driver.rs to compute LayoutCache after typecheck

7. Update all tests

8. Benchmark to verify performance improvement

### Acceptance Criteria

- [ ] MIR translate function accepts LayoutCache parameter
- [ ] All calls to get_layout replaced with LayoutCache lookups
- [ ] get_layout function deleted (save 300+ lines)
- [ ] Driver.rs computes LayoutCache once after typecheck
- [ ] All existing tests pass
- [ ] MIR stage 30%+ faster (benchmark)
- [ ] mir/mod.rs reduced from 547 to <300 lines
- [ ] Zero compiler warnings

### Performance Metrics

Before:
- MIR computation: O(n) for each type
- Layout recalculated multiple times

After:
- MIR computation: O(1) lookup
- Layout calculated once, cached

### References

- See TECHNICAL_DEBT_ANALYSIS.md Phase 2
- Depends on Issue #1

---

## Issue 3: Add AST Normalization Stage (High Priority)

**Title**: Feature: Add normalization stage for AST transformations (deblock)

**Labels**: `enhancement`, `high-priority`, `architecture`

**Description**:

### Overview

Create a separate normalization stage between typecheck and MIR to handle AST restructuring (specifically the `deblock` transformation) without complicating the typechecker or MIR.

### Problem

The `deblock` function (lines 122-228 in mir/mod.rs) transforms Block expressions into flattened sequences of LetIn/Ignore expressions. Currently this is in MIR, but:

- It operates on MIR AST after layout computation (wasteful)
- It's mixed with MIR's translation logic (confusing)
- Can't easily add other normalizations without touching MIR
- Moving to typecheck would complicate type inference

### Proposed Solution

Create a lightweight normalization stage between typecheck and MIR:

```
Parser → ModTree → Resolve → TypeCheck → Normalize → MIR → Core → Codegen
```

#### Architecture

```rust
// src/normalize/mod.rs - NEW MODULE
pub fn normalize(prog: typecheck::ast::Program) -> typecheck::ast::Program {
    let functions = prog.functions
        .into_iter()
        .map(normalize_func)
        .collect();
    
    typecheck::ast::Program {
        functions,
        sym_table: prog.sym_table,
    }
}

fn normalize_func(func: typecheck::ast::Func) -> typecheck::ast::Func {
    typecheck::ast::Func {
        body: deblock_expr(func.body),
        ..func
    }
}

fn deblock_expr(expr: typecheck::ast::Expr) -> typecheck::ast::Expr {
    // Flatten Block { exprs: [e1, e2], last: e3 } 
    // Into:     Sequence(e1, Sequence(e2, e3))
    match expr {
        Expr::Block { exprs, last_expr, block_tp } => {
            let result = exprs.into_iter().rfold(*last_expr, |acc, e| {
                Expr::Sequence {
                    first: Box::new(deblock_expr(e)),
                    second: Box::new(acc),
                }
            });
            deblock_expr(result)
        }
        // ... other cases
    }
}
```

### Benefits

- ✅ **Separation of Concerns**: Typecheck for types, Normalize for structure, MIR for lowering
- ✅ **Simpler stages**: Each stage <100 lines for core logic
- ✅ **Extensible**: Can add more normalizations (desugaring, constant folding) without touching typecheck
- ✅ **Testable**: Test normalize independently
- ✅ **Standard practice**: Rust, GHC use similar multi-stage approaches

### Migration Path

1. **Add Sequence variant to typecheck AST**:
   ```rust
   // src/typecheck/ast.rs
   pub enum Expr {
       // ... existing variants ...
       Sequence {
           first: Box<Expr>,
           second: Box<Expr>,
       },
   }
   ```

2. **Create src/normalize/mod.rs**:
   - Copy deblock logic from mir/mod.rs
   - Adapt to operate on typecheck::ast::Expr
   - Add other normalizations as needed

3. **Update driver.rs**:
   ```rust
   let prog = typecheck::translate(&mut ctx, prog)?;
   let prog = normalize::normalize(prog);  // NEW
   let prog = mir::translate(prog, &layout_cache)?;
   ```

4. **Simplify MIR**:
   - Remove deblock function (save 100+ lines)
   - Expect normalized input

5. **Add tests**:
   - Test normalize independently
   - Ensure output identical to before

### Acceptance Criteria

- [ ] `src/normalize/mod.rs` created
- [ ] `Sequence` variant added to typecheck::ast::Expr
- [ ] deblock logic moved from MIR to normalize
- [ ] normalize handles typechecked AST (types present, no inference)
- [ ] Driver.rs calls normalize between typecheck and MIR
- [ ] MIR simplified (deblock function removed)
- [ ] All existing tests pass
- [ ] New tests for normalize module (>90% coverage)
- [ ] Documentation explains normalization pipeline
- [ ] Zero compiler warnings

### Future Normalizations

Once this stage exists, we can add:
- Desugar complex constructs
- Constant expression evaluation
- Dead code elimination after type analysis
- Pattern match compilation
- Closure conversion

### References

- See TECHNICAL_DEBT_ANALYSIS.md "Detailed Proposal: Handling deblock"
- Related to Issue #2
- Rust compiler's HIR → MIR lowering
- GHC's Core normalization passes

---

## Issue 4: Unify Type Representations (Medium Priority)

**Title**: Refactor: Unify mir::Type and core::Type to eliminate unsafe transmutes

**Labels**: `refactoring`, `medium-priority`, `safety`

**Description**:

### Overview

Eliminate unsafe transmutes in the Core stage by unifying type representations between MIR and Core.

### Problem

Currently core/mod.rs uses unsafe transmutes excessively:

```rust
// core/mod.rs:10
let symbols = unsafe { transmute(prog.symbols) };

// core/mod.rs:37
fn tr_type(tp: in_a::Type) -> out_a::Type {
    unsafe { transmute(tp) }
}
```

Issues:
- Fragile - breaks if type definitions change
- No compile-time guarantees
- Undefined behavior if types diverge
- Suggests types are too similar and could be unified

### Proposed Solution (Recommended)

Create a shared primitive type used by both MIR and Core:

```rust
// src/backend/types.rs - NEW MODULE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    U8, U16, U32, U64, Usize,
    I8, I16, I32, I64, Isize,
}

// Both src/mir/ast.rs and src/core/ast.rs import and use this
```

### Alternative Solution

If types need to stay separate, use proper From/Into conversions:

```rust
// core/mod.rs
impl From<mir::Type> for core::Type {
    fn from(t: mir::Type) -> Self {
        match t {
            mir::Type::Tu8 => core::Type::Tu8,
            mir::Type::Ti32 => core::Type::Ti32,
            // ... explicit, safe conversion for all variants
        }
    }
}
```

### Benefits

- ✅ Type safety - compile-time guarantees
- ✅ No undefined behavior risk
- ✅ Clearer relationship between stages
- ✅ Easier to maintain (changes caught at compile time)

### Migration Path

1. Create `src/backend/types.rs` module

2. Define shared `PrimitiveType` enum

3. Update `src/mir/ast.rs`:
   - Replace `pub enum Type` with `pub use crate::backend::types::PrimitiveType as Type;`

4. Update `src/core/ast.rs`:
   - Replace `pub enum Type` with `pub use crate::backend::types::PrimitiveType as Type;`

5. Remove all unsafe transmutes in core/mod.rs

6. Update any code that pattern matches on Type variants

7. Add tests to ensure types are compatible

### Acceptance Criteria

- [ ] `src/backend/types.rs` created with shared PrimitiveType
- [ ] mir::Type and core::Type unified
- [ ] All unsafe transmutes removed from core/mod.rs
- [ ] All existing tests pass
- [ ] New tests for type conversions
- [ ] Zero compiler warnings
- [ ] No unsafe blocks related to type conversions

### References

- See TECHNICAL_DEBT_ANALYSIS.md Phase 4
- Rust API Guidelines on type safety

---

## Issue 5: Improve Error Handling (Low Priority)

**Title**: Refactor: Replace panics with Result types for better error handling

**Labels**: `refactoring`, `low-priority`, `error-handling`

**Description**:

### Overview

Replace panic!() calls, unwrap(), and unreachable!() with proper Result types and error handling.

### Problem

Current code has many panic points:
- `panic!()` calls (e.g., symtable/mod.rs:98, 159)
- `unreachable!()` that might be reachable (mir/mod.rs:245)
- `.unwrap()` everywhere (mir/mod.rs:30, 466)

Issues:
- Compiler crashes on invalid input instead of reporting errors
- Hard to debug what went wrong
- Poor user experience
- Makes testing harder

### Proposed Solution

Use thiserror crate for typed errors:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Type {0} not found in symbol table")]
    TypeNotFound(String),
    
    #[error("Symbol {0} not found")]
    SymbolNotFound(NodeID),
    
    #[error("Internal compiler error: {0}")]
    Internal(String),
    
    #[error("Layout computation failed for type {0}")]
    LayoutError(String),
}

// Replace panic with Result
pub fn find_type_info(&self, tvar: TVar) -> Result<&TypeInfo, CompilerError> {
    self.types.get(&tvar)
        .ok_or_else(|| CompilerError::TypeNotFound(tvar.to_string()))
}
```

### Benefits

- ✅ Better error messages for users
- ✅ Compiler doesn't crash on invalid input
- ✅ Easier to test error paths
- ✅ Can add context to errors
- ✅ Follows Rust best practices

### Migration Path

1. Add `thiserror = "1.0"` to Cargo.toml

2. Create `src/error/compiler_error.rs` with CompilerError enum

3. Identify all panic points in codebase:
   ```bash
   rg "panic!" src/
   rg "unwrap\(\)" src/
   rg "unreachable!" src/
   ```

4. Replace panics in each module:
   - Start with symtable/mod.rs
   - Then mir/mod.rs
   - Then other modules

5. Update function signatures to return Result

6. Propagate errors with `?` operator

7. Add error handling tests

### Acceptance Criteria

- [ ] thiserror dependency added
- [ ] CompilerError enum created with all error variants
- [ ] All panic!() calls replaced (or documented why they're safe)
- [ ] All .unwrap() calls replaced with ? or expect with good messages
- [ ] All unreachable!() verified or replaced
- [ ] Function signatures updated to return Result
- [ ] Tests for error paths added
- [ ] Documentation explains error handling strategy

### References

- See TECHNICAL_DEBT_ANALYSIS.md Phase 5
- Rust API Guidelines on error handling
- thiserror crate documentation

---

## Issue 6: Fix Compiler Warnings (Quick Win)

**Title**: Code Quality: Fix 246 compiler warnings (unused fields, dead code)

**Labels**: `code-quality`, `quick-win`, `good-first-issue`

**Description**:

### Overview

Fix all 246 compiler warnings related to unused fields, dead code, and unused imports.

### Problem

Current codebase has 246 warnings:
- Unused fields in AST structures (e.g., parser/ast.rs)
- Dead code in enums (e.g., Slice, MutSlice variants)
- Unused imports
- Unused methods

This indicates:
- Features partially implemented
- Technical debt accumulating
- Harder to spot real issues among noise

### Proposed Solution

For each warning, choose one:

1. **Implement the feature** - If it's needed soon
2. **Remove the code** - If it's not needed
3. **Mark as TODO** - If planning to implement later:
   ```rust
   #[allow(dead_code)] // TODO: Implement slice types
   Slice(Box<RTypeNode>),
   ```

### Categories of Warnings

#### Unused Fields (100+ warnings)
- Fields in AST nodes that should be used
- Position fields that are never read

Action: Either use them or remove them

#### Dead Code (50+ warnings)
- Unused enum variants (Slice, MutSlice)
- Unused functions (unbound_method, unsolved_uvar)

Action: Either implement or remove

#### Unused Imports (30+ warnings)
Action: Remove

### Benefits

- ✅ Cleaner codebase
- ✅ Easier to spot real issues
- ✅ Shows what's implemented vs planned
- ✅ Good first issue for contributors

### Migration Path

1. Run `cargo build 2>&1 | grep warning | wc -l` to count warnings

2. Group warnings by category

3. For each category:
   - Decide: implement, remove, or allow with TODO
   - Make changes
   - Verify warnings reduced

4. Goal: Zero warnings or all remaining have #[allow] with explanation

### Acceptance Criteria

- [ ] All 246 warnings addressed
- [ ] Remaining warnings have #[allow] with TODO comments
- [ ] Documentation updated for removed features
- [ ] All tests pass
- [ ] Ideally: Zero warnings

### Time Estimate

- 2-4 hours for careful review and fixes
- Can be done incrementally by module

### References

- See TECHNICAL_DEBT_ANALYSIS.md "Quick Wins #3"
- Good first issue for new contributors

---

## Issue 7: Extract SymbolAttributes Struct (Quick Win)

**Title**: Refactor: Extract SymbolAttributes struct to reduce parameter passing

**Labels**: `refactoring`, `quick-win`, `code-quality`

**Description**:

### Overview

Group related symbol attributes (is_extern, mangle, builtin_name) into a dedicated struct to reduce parameter passing and improve code organization.

### Problem

Currently these attributes are separate fields in SymInfo:
```rust
pub struct SymInfo {
    pub name: String,
    pub pos: Position,
    pub kind: SymKind,
    pub builtin_name: Option<String>,  // Scattered
    pub is_extern: bool,                // Scattered
    pub mangle: bool,                   // Scattered
}
```

This leads to:
- Functions with many parameters
- Unclear which attributes go together
- Hard to add new attributes

### Proposed Solution

```rust
pub struct SymbolAttributes {
    pub builtin_name: Option<String>,
    pub is_extern: bool,
    pub mangle: bool,
}

pub struct SymInfo {
    pub name: String,
    pub pos: Position,
    pub kind: SymKind,
    pub attributes: SymbolAttributes,  // Grouped!
}

impl SymbolAttributes {
    pub fn from_rattributes(attrs: Vec<RAttribute>) -> Self {
        let mut result = SymbolAttributes::default();
        for attr in attrs {
            match attr.name.data.as_str() {
                "extern" => result.is_extern = true,
                "no_mangle" => result.mangle = false,
                _ => continue,
            }
        }
        result
    }
}
```

### Benefits

- ✅ Reduced parameter passing
- ✅ Clearer code organization
- ✅ Easier to add new attributes
- ✅ Can implement Default, builder pattern

### Migration Path

1. Create SymbolAttributes struct in symtable/mod.rs

2. Update SymInfo to use attributes: SymbolAttributes

3. Update all code that accesses these fields:
   - Change `sym.is_extern` to `sym.attributes.is_extern`
   - Change `sym.mangle` to `sym.attributes.mangle`
   - etc.

4. Add convenience methods to SymbolAttributes

5. Update tests

### Acceptance Criteria

- [ ] SymbolAttributes struct created
- [ ] SymInfo updated to use it
- [ ] All code accessing attributes updated
- [ ] Helper methods added (from_rattributes, etc.)
- [ ] All tests pass
- [ ] Zero compiler warnings

### Time Estimate

30 minutes - 1 hour

### References

- See TECHNICAL_DEBT_ANALYSIS.md "Quick Wins #1"
- Part of Phase 1 preparation

---

## Issue 8: Add Test Infrastructure (High Priority)

**Title**: Testing: Add comprehensive test suite for compiler stages

**Labels**: `testing`, `high-priority`, `infrastructure`

**Description**:

### Overview

Add comprehensive testing infrastructure for all compiler stages. Currently there are 0 tests.

### Problem

- No test suite exists (0 tests)
- Can't verify refactorings don't break behavior
- Hard to catch regressions
- No confidence in changes

### Proposed Solution

Add tests for each stage:

#### 1. Parser Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_function() {
        let input = "fn foo() -> i32 { 42 }";
        let result = parse_file(&mut ctx, input);
        assert!(result.is_ok());
    }
}
```

#### 2. TypeCheck Tests
```rust
#[test]
fn test_typecheck_function_call() {
    // Test that function calls type-check correctly
}

#[test]
fn test_typecheck_error_type_mismatch() {
    // Test that type mismatches are caught
}
```

#### 3. MIR Tests
```rust
#[test]
fn test_layout_computation() {
    // Test layout calculation
}
```

#### 4. Integration Tests
```rust
#[test]
fn test_compile_simple_program() {
    // End-to-end test
}
```

### Test Organization

```
tests/
  parser/
    mod.rs
    expressions.rs
    statements.rs
  typecheck/
    mod.rs
    inference.rs
    errors.rs
  mir/
    mod.rs
    layouts.rs
  integration/
    mod.rs
    examples.rs
```

### Benefits

- ✅ Catch regressions
- ✅ Enable confident refactoring
- ✅ Document expected behavior
- ✅ Reduce debugging time

### Migration Path

1. Create `tests/` directory structure

2. Add basic tests for each stage:
   - Start with happy path tests
   - Add error case tests
   - Add edge case tests

3. Add integration tests using example programs

4. Set up CI to run tests automatically

5. Target >70% coverage

### Acceptance Criteria

- [ ] Test directory structure created
- [ ] Tests for parser (>10 tests)
- [ ] Tests for typecheck (>10 tests)
- [ ] Tests for MIR (>5 tests)
- [ ] Integration tests (>5 tests)
- [ ] CI configured to run tests
- [ ] >70% code coverage
- [ ] All tests pass

### Priority

This is HIGH PRIORITY - should be done before major refactoring to prevent regressions.

### References

- See TECHNICAL_DEBT_ANALYSIS.md "High Priority #2"
- See PROJECT_SUMMARY.md (notes 0 tests)

---

## Summary of Priority Order

1. **Issue #8**: Add Test Infrastructure (FIRST - enables safe refactoring)
2. **Issue #6**: Fix 246 Warnings (Quick win, improves visibility)
3. **Issue #7**: Extract SymbolAttributes (Quick win, 30 min)
4. **Issue #1**: Split SymTable (High priority, foundation for others)
5. **Issue #3**: Add Normalization Stage (High priority, architectural improvement)
6. **Issue #2**: Simplify MIR (High priority, depends on #1)
7. **Issue #4**: Unify Type Representations (Medium priority)
8. **Issue #5**: Improve Error Handling (Low priority, ongoing)

## How to Use These Issues

1. Copy each issue description to a new GitHub issue
2. Add appropriate labels
3. Link related issues in the description
4. Assign to developers based on priority
5. Track progress in a project board
