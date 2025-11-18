# Technical Debt Analysis & Refactoring Proposals

## Executive Summary

This document analyzes the current technical debt in mustcc and proposes concrete refactoring strategies, with specific focus on the symtable/MIR architecture that you mentioned. The primary issues stem from:

1. **Tight coupling** between compilation stages
2. **Duplicate type information** across SymTable, TypeInfo, and Layout structures
3. **Complex data flow** through the MIR stage with unnecessary transformations
4. **Inconsistent abstraction levels** between stages

## Current Architecture Problems

### 1. SymTable/MIR Coupling Issues

**Problem**: SymTable mixes concerns and is tightly coupled to MIR generation.

**Current Issues**:
```rust
// symtable/mod.rs - Multiple responsibilities
pub struct SymTable {
    node_map: HashMap<NodeID, SymInfo>,    // Symbol information
    tvar_map: HashMap<TVar, TypeInfo>,     // Type information
    tvar_order: Vec<TVar>,                 // Topological ordering (for MIR)
    tvar_size: HashMap<TVar, usize>,       // Size calculation (for MIR/codegen)
}
```

**Issues**:
- Mixes **symbol resolution** (node_map, tvar_map) with **code generation concerns** (tvar_size, tvar_order)
- The `sizeof` method performs complex recursive traversal that should be cached
- Type layout calculation duplicated between SymTable::sizeof and mir::get_layout
- SymTable is both a data structure and has behavior (check_sizes, sizeof)

### 2. MIR Stage Complexity

**Problem**: MIR does too much work and creates unnecessary intermediate representations.

**Issues**:
- The `Layout` structure in MIR duplicates information from SymTable
- `get_layout` function (547 lines in mir/mod.rs) recalculates layouts that SymTable already knows
- Double transformation: TypeCheck → MIR (with layouts) → Core (transmute layouts)
- `deblock` function does AST restructuring that could be done in typecheck
- Complex var_needs_stack tracking that's computed late

### 3. Type System Fragmentation

**Problem**: Type information scattered across multiple representations.

**Type Representations**:
1. `tp::Type` - High-level type with inference (typecheck stage)
2. `symtable::TypeInfo` - Type metadata with fields/constructors
3. `mir::Layout` - Physical layout with size/offset/align
4. `mir::TypeLayout` - Simple/Array/Tuple classification
5. `mir::Type` - Concrete primitive types (Tu8, Ti32, etc.)
6. `core::Type` - Same as mir::Type (via unsafe transmute!)

**Issues**:
- Converting between representations is error-prone
- Layout calculation happens in multiple places
- No single source of truth for type properties

### 4. Unsafe Code in Core Translation

**Problem**: Core stage uses `unsafe { transmute }` excessively.

```rust
// core/mod.rs:10
let symbols = unsafe { transmute(prog.symbols) };

// core/mod.rs:37
fn tr_type(tp: in_a::Type) -> out_a::Type {
    unsafe { transmute(tp) }
}
```

**Issues**:
- Fragile - breaks if type definitions change
- No compile-time guarantees
- Suggests types are too similar and could be unified

## Proposed Refactoring Solutions

### Phase 1: Separate Concerns in SymTable (High Priority)

**Proposal**: Split SymTable into three focused components:

```rust
// src/symbols/mod.rs - NEW MODULE
pub mod resolution;
pub mod types;
pub mod layout;

// 1. Symbol Resolution - Pure symbol table
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

pub enum SymbolKind {
    Func { params: HashSet<TVar>, args: Vec<Type>, ret: Type },
    Type(TVar),
    TypeConstructor { id: usize, args: Vec<Type>, parent: NodeID },
}

// 2. Type Information - Separate from symbols
pub struct TypeRegistry {
    types: HashMap<TVar, TypeInfo>,
    type_order: Vec<TVar>,  // Topological order for recursive types
}

pub struct TypeInfo {
    pub name: String,
    pub pos: Position,
    pub kind: TypeKind,
}

pub enum TypeKind {
    Builtin { size: usize },
    Struct { params: HashSet<TVar>, fields: Vec<(String, Type)> },
    Enum { params: HashSet<TVar>, variants: Vec<(String, NodeID)> },
}

// 3. Layout Calculation - Computed once, cached
pub struct LayoutCache {
    layouts: HashMap<Type, Layout>,
    sizes: HashMap<TVar, TypeSize>,
}

impl LayoutCache {
    pub fn compute_layout(&mut self, typ: &Type, type_reg: &TypeRegistry) -> Layout {
        if let Some(cached) = self.layouts.get(typ) {
            return cached.clone();
        }
        // Compute and cache
        let layout = self.compute_layout_uncached(typ, type_reg);
        self.layouts.insert(typ.clone(), layout.clone());
        layout
    }
}
```

**Benefits**:
- Single Responsibility Principle: each component has one job
- LayoutCache can be computed once after type checking
- Easier to test and maintain
- Clear ownership and data flow

**Migration Path**:
1. Create new `src/symbols/` module
2. Move SymTable → SymbolTable (rename, strip layout code)
3. Extract TypeRegistry from tvar_map
4. Create LayoutCache, move sizeof logic
5. Update resolve/typecheck to use new APIs
6. Deprecate old SymTable

### Phase 2: Simplify MIR Stage (High Priority)

**Proposal**: Make MIR a thin translation layer, not a computation stage.

**Current MIR problems**:
- Line 446-547: `get_layout` duplicates LayoutCache work
- Line 33-76: `make_symtable` does unnecessary conversions
- Line 122-228: `deblock` restructures AST (should be in typecheck)

**Refactored MIR**:
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
    // No layout computation - just lookup!
    let mut env = Env::new();
    
    let args = func.args.into_iter().map(|(name, is_mut, tp)| {
        let id = env.add_var(name);
        let layout = layouts.get(&tp);  // O(1) lookup, not O(n) computation
        let mir_type = layout.to_mir_type();
        (id, is_mut, mir_type, layout.needs_stack())
    }).collect();
    
    let body = tr_expr(&mut env, layouts, func.body)?;
    
    Ok(out_a::Func {
        id: func.id,
        args,
        body,
        // var_needs_stack computed inline above
    })
}
```

**Benefits**:
- Faster compilation (no recomputation)
- Less code (remove 300+ lines)
- Clearer data flow
- Easier to debug

**What to Move**:
- `deblock` → **See detailed proposal below** 
- `get_layout` → Delete (use LayoutCache instead)
- Layout computation → Done once after typecheck

#### Detailed Proposal: Handling `deblock` Without Complicating Typecheck

**The Problem**: The `deblock` function (lines 122-228) transforms Block expressions into a flattened sequence of LetIn/Ignore expressions. Moving this directly to typecheck would complicate the type inference algorithm.

**Better Solution - Create a Separate Normalization Pass**:

Instead of adding `deblock` to typecheck OR keeping it in MIR, create a lightweight post-typecheck normalization stage:

```rust
// src/normalize/mod.rs - NEW MODULE (between typecheck and MIR)
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
    // Same logic as current deblock, but operates on typecheck AST
    // Converts Block { exprs: [e1, e2], last: e3 } 
    // Into:     Sequence(e1, Sequence(e2, e3))
    match expr {
        Expr::Block { exprs, last_expr, block_tp } => {
            // Flatten blocks into sequences
            let result = exprs.into_iter().rfold(*last_expr, |acc, e| {
                Expr::Sequence {
                    first: Box::new(deblock_expr(e)),
                    second: Box::new(acc),
                }
            });
            deblock_expr(result)
        }
        Expr::Let { name, tp, is_mut, expr } => {
            // Standalone Let becomes a LetIn when followed by another expr
            // This transformation happens in context of parent
            expr  // For now, pass through; parent will handle
        }
        // Recursively normalize other expressions
        other => normalize_children(other)
    }
}
```

**Why This is Better**:

1. **Separation of Concerns**: 
   - Typecheck focuses on type correctness
   - Normalize focuses on AST structure
   - MIR focuses on lowering to physical representation

2. **Simpler Each Stage**:
   - Typecheck doesn't need deblock logic
   - MIR doesn't need deblock logic
   - Normalize is <100 lines, single purpose

3. **Pipeline Becomes**:
   ```
   Parser → ModTree → Resolve → TypeCheck → Normalize → MIR → Core → Codegen
   ```

4. **Can Add Other Normalizations** without touching typecheck:
   - Desugar certain constructs
   - Optimize constant expressions
   - Eliminate dead code after type analysis

5. **Testing is Easier**: Test normalize independently

**Alternative - Keep in MIR but Separate**:

If you prefer not to add a new stage, another option is to keep `deblock` in MIR but make it operate on typecheck AST before translation:

```rust
// src/mir/mod.rs
pub fn translate(prog: in_a::Program, layouts: &LayoutCache) -> Result<out_a::Program, InternalError> {
    // Normalize typecheck AST first
    let prog = normalize_typecheck_ast(prog);
    
    // Then translate (now simpler since AST is normalized)
    let functions = prog.functions
        .into_iter()
        .map(|f| tr_func_normalized(&prog.symbols, layouts, f))
        .collect::<Result<_, _>>()?;
    
    out_a::Program {
        symbols: prog.symbols.into_mir_symbols(),
        functions,
    }
}

fn normalize_typecheck_ast(prog: in_a::Program) -> in_a::Program {
    // Operates on typecheck::ast, not mir::ast
    // Returns modified typecheck::ast
    // Simpler because types are already present
}
```

**Recommendation**: I suggest the separate normalization pass because:
- It's cleaner architecturally
- Other compilers (Rust, GHC) use similar multi-stage approaches
- Makes each stage easier to understand and test
- Only ~100 lines of code

**Migration Path**:
1. Add `Sequence` variant to typecheck AST:
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
2. Create `src/normalize/mod.rs` with deblock logic
3. Update driver.rs to call normalize between typecheck and MIR:
   ```rust
   let prog = typecheck::translate(&mut ctx, prog)?;
   let prog = normalize::normalize(prog);  // NEW
   let prog = mir::translate(prog)?;
   ```
4. Simplify MIR to expect normalized input (remove deblock function)
5. Test that output is identical

### Phase 3: Unify Type Representations (Medium Priority)

**Proposal**: Create a progressive type refinement architecture.

```rust
// src/types/mod.rs - NEW UNIFIED MODULE
pub mod untyped;     // Parser types
pub mod typed;       // After typechecking
pub mod physical;    // With layout info

// Progressive refinement
pub struct UntypedExpr { /* parser AST */ }
pub struct TypedExpr { expr: UntypedExpr, typ: Type }
pub struct PhysicalExpr { expr: TypedExpr, layout: Layout }

// Each stage adds information, doesn't transform structure
```

**Alternative - Keep separate but share via traits**:
```rust
pub trait TypeRepresentation {
    fn size_of(&self) -> Option<usize>;
    fn align_of(&self) -> Option<usize>;
    fn is_sized(&self) -> bool;
}

impl TypeRepresentation for tp::Type { /* ... */ }
impl TypeRepresentation for mir::Type { /* ... */ }
impl TypeRepresentation for core::Type { /* ... */ }
```

**Benefits**:
- Reduce conversions
- Shared behavior
- Type safety

### Phase 4: Remove Unsafe Transmutes (Medium Priority)

**Proposal**: Replace transmute with proper conversions or unify types.

**Option A - Proper conversion**:
```rust
// core/mod.rs
impl From<mir::Type> for core::Type {
    fn from(t: mir::Type) -> Self {
        match t {
            mir::Type::Tu8 => core::Type::Tu8,
            mir::Type::Ti32 => core::Type::Ti32,
            // ... explicit, safe conversion
        }
    }
}
```

**Option B - Unify types** (RECOMMENDED):
```rust
// src/backend/types.rs - Shared by MIR and Core
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    U8, U16, U32, U64, Usize,
    I8, I16, I32, I64, Isize,
}

// Both MIR and Core use this directly
```

**Benefits**:
- Type safety
- No undefined behavior risk
- Clearer relationship between stages

### Phase 5: Better Error Handling (Low Priority)

**Current Issues**:
- Many `panic!()` calls (e.g., symtable/mod.rs:98, 159)
- `unreachable!()` that might be reachable (mir/mod.rs:245)
- `.unwrap()` everywhere (mir/mod.rs:30, 466)

**Proposal**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Type {0} not found in symbol table")]
    TypeNotFound(String),
    
    #[error("Internal: {0}")]
    Internal(String),
}

// Replace panic with Result
pub fn find_type_info(&self, tvar: TVar) -> Result<&TypeInfo, CompilerError> {
    self.types.get(&tvar)
        .ok_or_else(|| CompilerError::TypeNotFound(tvar.to_string()))
}
```

### Phase 6: Improve AST Design (Low Priority)

**Issues**:
- Many unused fields (246 warnings!)
- Complex nested structures
- Pattern matching partially implemented

**Proposals**:
1. Remove unused variants or implement them
2. Use the `typed-builder` crate for complex structs
3. Separate "parsed" AST from "resolved" AST more clearly

## Implementation Priority & Timeline

### Sprint 1 (1-2 weeks): Foundation
- [ ] Create `src/symbols/` module structure
- [ ] Implement SymbolTable split (resolution, types, layout)
- [ ] Add LayoutCache with tests
- [ ] Keep old SymTable as deprecated wrapper

### Sprint 2 (1 week): MIR Simplification
- [ ] Refactor MIR to use LayoutCache
- [ ] Remove get_layout function
- [ ] Move deblock to typecheck
- [ ] Update tests

### Sprint 3 (1 week): Type Unification
- [ ] Create unified PrimitiveType in backend module
- [ ] Remove transmutes in core
- [ ] Add TypeRepresentation trait if needed

### Sprint 4 (Ongoing): Quality Improvements
- [ ] Add proper error types
- [ ] Remove panics/unwraps
- [ ] Fix 246 warnings
- [ ] Add documentation

## Quick Wins (Do First!)

These can be done independently in a few hours each:

1. **Extract SymbolAttributes struct** (30 min)
   - Group is_extern, mangle, builtin_name
   - Reduces parameter passing

2. **Cache Layout in TypeInfo** (1 hour)
   - Add `layout: OnceCell<Layout>` to TypeInfo
   - Compute once on first access

3. **Remove unused AST fields** (2 hours)
   - Fix the 246 warnings
   - Either implement features or remove dead code

4. **Add builder pattern for complex structs** (1 hour)
   - Use for SymInfo, TypeInfo construction
   - Reduces errors

5. **Document data flow** (1 hour)
   - Add module-level docs showing: Parser → ModTree → Resolve → TypeCheck → MIR → Core → Codegen
   - Include what each stage adds/transforms

## Measuring Success

After refactoring:
- [ ] Zero warnings
- [ ] Zero unsafe blocks (or documented justification for each)
- [ ] LayoutCache reduces MIR time by >30%
- [ ] Lines of code in mir/mod.rs reduced from 547 to <300
- [ ] All public APIs documented
- [ ] Test coverage >70%

## References

- Clean Architecture (Robert Martin) - Separation of concerns
- Rust API Guidelines - Error handling, builder patterns
- Crafting Interpreters (Nystrom) - Multi-pass compiler design
- Modern Compiler Implementation (Appel) - IR design patterns

## Next Steps

1. **Get feedback** on these proposals
2. **Prioritize** which phases to tackle first
3. **Create issues** for each refactoring task
4. **Write tests** before refactoring (regression prevention)
5. **Refactor incrementally** - one component at a time

Would you like me to elaborate on any specific proposal or create implementation PRs for the quick wins?
