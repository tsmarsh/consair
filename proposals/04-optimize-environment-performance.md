# Optimize Environment Performance

## Problem

The `Environment::define()` method clones the entire HashMap on every variable definition, resulting in O(n) cost per definition. This becomes a significant bottleneck in programs with many definitions.

**Location:** `consair-core/src/interpreter.rs:42-46`

**Problematic code:**
```rust
pub fn define(&mut self, name: String, value: Value) {
    let mut new_bindings = (*self.bindings).clone();  // Clones entire map!
    new_bindings.insert(name, value);
    self.bindings = Arc::new(new_bindings);
}
```

## Impact

- O(n) cost per variable definition
- Performance degrades with number of definitions
- Unnecessary memory churn

## Prompt for Implementation

```
Optimize the Environment implementation to avoid cloning the entire HashMap on every define():

1. Current code in consair-core/src/interpreter.rs:42-46 clones entire HashMap per definition
2. This is O(n) cost which is unnecessary

Please evaluate and implement one of these approaches:

**Option A: Persistent Data Structure**
- Replace Arc<HashMap> with im::HashMap or rpds::HashTrieMap
- These provide O(log n) structural sharing for updates
- Add dependency: im = "15.1" or rpds = "1.1"
- Benchmark to verify improvement

**Option B: Copy-on-Write Optimization**
- Only clone when Arc has multiple strong references
- Use Arc::make_mut() pattern
- This optimizes the common case (single environment)

**Option C: Hybrid Approach**
- Use regular HashMap for leaf scopes (most definitions)
- Only use Arc for parent scopes that need sharing

Please:
- Implement the chosen approach
- Add benchmarks comparing old vs new performance:
  * 10, 100, 1000, 10000 definitions in single scope
  * Nested scopes with varying depths
  * Lambda closures capturing environments
- Ensure all existing tests pass
- Measure memory usage as well as speed
- Document the chosen approach and tradeoffs

Recommend Option B for simplicity unless benchmarks show Option A is significantly better.
```

## Success Criteria

- [ ] define() performance improved (measured via benchmarks)
- [ ] All existing tests pass
- [ ] Benchmarks show improvement for 100+ definitions
- [ ] Memory usage is same or better
- [ ] Code complexity is reasonable
