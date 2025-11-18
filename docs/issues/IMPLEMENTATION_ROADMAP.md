# Refactoring Implementation Roadmap

Quick reference guide for implementing the refactoring issues.

## Implementation Order

### Phase 0: Preparation (Week 1)
**Goal**: Set up infrastructure for safe refactoring

1. **Issue #8: Add Test Infrastructure** ⭐ DO FIRST
   - Create test directory structure
   - Add tests for parser, typecheck, MIR
   - Set up CI
   - Target: >70% coverage
   - **Why first**: Prevents regressions during refactoring

2. **Issue #6: Fix 246 Warnings** ⭐ QUICK WIN
   - Remove unused imports
   - Fix dead code warnings
   - Either implement or remove unused fields
   - Time: 2-4 hours
   - **Why second**: Clean slate, easier to spot new issues

3. **Issue #7: Extract SymbolAttributes** ⭐ QUICK WIN
   - Group related attributes
   - Reduce parameter passing
   - Time: 30 minutes
   - **Why third**: Easy win, improves readability

### Phase 1: Foundation (Week 2-3)
**Goal**: Separate concerns in SymTable

4. **Issue #1: Split SymTable**
   - Create `src/symbols/` module
   - SymbolTable (symbol resolution)
   - TypeRegistry (type definitions)
   - LayoutCache (layout computation)
   - Update all dependent code
   - Dependencies: None
   - Enables: Issues #2, #3

### Phase 2: Simplification (Week 4-5)
**Goal**: Make each stage focused and efficient

5. **Issue #3: Add Normalization Stage**
   - Create `src/normalize/mod.rs`
   - Move deblock from MIR
   - Add Sequence variant to typecheck AST
   - Update driver.rs
   - Dependencies: None
   - Time: 1 week

6. **Issue #2: Simplify MIR**
   - Remove 300-line get_layout function
   - Use LayoutCache for O(1) lookups
   - Update driver.rs
   - Dependencies: Issue #1
   - Time: 1 week
   - Benefit: 30%+ faster MIR stage

### Phase 3: Safety & Quality (Week 6-7)
**Goal**: Remove unsafe code and improve reliability

7. **Issue #4: Unify Type Representations**
   - Create `src/backend/types.rs`
   - Shared PrimitiveType for MIR and Core
   - Remove unsafe transmutes
   - Dependencies: None
   - Time: 3-4 days

8. **Issue #5: Improve Error Handling** (Ongoing)
   - Add thiserror dependency
   - Create CompilerError enum
   - Replace panics with Results
   - Dependencies: None
   - Time: Ongoing, do incrementally

## Quick Reference

### File Locations

```
src/
  symbols/           # NEW - Issue #1
    mod.rs
    resolution.rs    # SymbolTable
    types.rs         # TypeRegistry
    layout.rs        # LayoutCache
  
  normalize/         # NEW - Issue #3
    mod.rs
  
  backend/           # NEW - Issue #4
    types.rs         # Unified PrimitiveType
  
  error/
    compiler_error.rs # NEW - Issue #5
  
  symtable/          # REFACTOR - Issue #1
    mod.rs           # Keep as compatibility wrapper initially
  
  mir/               # SIMPLIFY - Issue #2
    mod.rs           # Remove get_layout, deblock
  
  typecheck/         # EXTEND - Issue #3
    ast.rs           # Add Sequence variant
```

### Key Dependencies

```
Issue #2 depends on Issue #1 (needs LayoutCache)
Issue #3 can be done independently
Issue #4 can be done independently  
Issue #5 ongoing, can be done anytime
Issue #6 should be done early (quick win)
Issue #7 should be done early (quick win)
Issue #8 MUST be done first
```

### Success Metrics

After completion:
- [ ] Zero warnings
- [ ] Zero unsafe blocks (except documented)
- [ ] >70% test coverage
- [ ] mir/mod.rs: 547 → <300 lines
- [ ] MIR stage: 30%+ faster
- [ ] All phases documented

## Weekly Checklist

### Week 1: Preparation
- [ ] Set up test infrastructure
- [ ] Add tests for all stages
- [ ] Fix 246 warnings
- [ ] Extract SymbolAttributes
- [ ] Run baseline benchmarks

### Week 2-3: Split SymTable
- [ ] Create symbols module
- [ ] Implement SymbolTable
- [ ] Implement TypeRegistry
- [ ] Implement LayoutCache
- [ ] Update dependent code
- [ ] Add tests
- [ ] Verify benchmarks

### Week 4: Add Normalization
- [ ] Add Sequence to typecheck AST
- [ ] Create normalize module
- [ ] Move deblock logic
- [ ] Update driver
- [ ] Add tests
- [ ] Verify output identical

### Week 5: Simplify MIR
- [ ] Update MIR signature for LayoutCache
- [ ] Replace get_layout calls
- [ ] Remove get_layout function
- [ ] Update driver
- [ ] Add tests
- [ ] Benchmark improvement

### Week 6-7: Safety & Quality
- [ ] Create unified PrimitiveType
- [ ] Remove transmutes
- [ ] Add CompilerError
- [ ] Replace panics
- [ ] Final documentation
- [ ] Final benchmarks

## Commands

### Run tests
```bash
cargo test
```

### Check warnings
```bash
cargo build 2>&1 | grep warning | wc -l
```

### Benchmark
```bash
cargo build --release
time cargo run --release -- examples/001
```

### Coverage
```bash
cargo tarpaulin --out Html
```

### Format
```bash
cargo fmt
```

### Lint
```bash
cargo clippy -- -W clippy::all
```

## Resources

- Main analysis: `TECHNICAL_DEBT_ANALYSIS.md`
- Detailed issues: `docs/issues/REFACTORING_ISSUES.md`
- Project summary: `PROJECT_SUMMARY.md`

## Questions?

See the detailed proposals in TECHNICAL_DEBT_ANALYSIS.md or create a discussion issue.
