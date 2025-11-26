# JIT/AOT Stdlib Parity with Interpreter

## Goal
The interpreter stdlib is the gold standard. Any valid Consair program that runs in interpreted mode must also compile and run correctly with JIT (`cons --jit`) and AOT (`cadr`).

## Root Cause
JIT and AOT were written with separate `compile_value()` implementations that diverged from the interpreter stdlib:
1. **Naming bugs** - AOT uses `eq?` but interpreter uses `eq`
2. **Missing functions** - Stdlib functions not implemented in JIT/AOT
3. **Extra functions** - JIT/AOT have functions that don't exist in interpreter

## Implementation Phases

### Phase 1: Fix Naming Bugs (Critical)
**File:** `consair-core/src/aot/compiler.rs`
- Line 405: `"eq?"` ’ `"eq"`
- Line 425: `"atom?"` ’ `"atom"`

### Phase 2: Bidirectional Parity
**Add to interpreter** (`stdlib.rs`): `length`, `append`, `reverse`, `list`, `nth`, `nil?`, `cons?`, `number?`, `not`, `vector-length`, `vector-ref`

**Add to JIT** (`jit/engine.rs`): `cons?`, `number?`

### Phase 3: Add I/O to JIT/AOT
Implement `print`, `println` with runtime IR support.

### Phase 4: Add `if` to JIT
JIT only has `cond`, but `if` is common.

### Phase 5 (Future): Collection Abstractions
The 19 `%` sequence functions - implement as library or native.

## Files to Modify
- `consair-core/src/aot/compiler.rs`
- `consair-core/src/aot/runtime_ir.rs`
- `consair-core/src/stdlib.rs`
- `consair-core/src/jit/engine.rs`

## Verification
1. `cargo test` passes
2. Test file runs identically in interpreter, JIT, and AOT
